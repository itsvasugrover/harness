//! Gateway: resolve + serve. Chat streaming lands in Phase 1c.
use super::catalog::Catalog;
use super::port::ModelRef;
use anyhow::Result;
use axum::{http::StatusCode, routing::get, Json, Router};
use std::sync::Arc;

/// Resolve the upstream base URL for a model reference.
/// Empty base URLs (e.g. models.dev entries without local config)
/// resolve to `None` — never route into an empty string.
pub fn resolve_base_url<'a>(catalog: &'a Catalog, r: &ModelRef) -> Option<&'a str> {
    let base = catalog
        .providers
        .get(&r.provider)
        .map(|p| p.base_url.as_str())?;
    if base.is_empty() {
        return None;
    }
    Some(base)
}

/// Gateway request paths served in Phase 1b.
pub fn paths() -> [&'static str; 4] {
    [
        "/v1/chat/completions",
        "/v1/responses",
        "/v1/messages",
        "/v1/models",
    ]
}

/// Live routing decision: parse the model ref, resolve the upstream,
/// and apply the task-class SLO. The HTTP hop lands with provider
/// adapters (Phase 3); the decision is already real and testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routing {
    pub provider: String,
    pub model: String,
    pub base_url: String,
}

pub fn route_model(
    catalog: &Catalog,
    model_str: &str,
    candidates: &[super::budget::Candidate],
    slo: &super::budget::Slo,
) -> Option<Routing> {
    let r = ModelRef::parse(model_str).ok()?;
    let base = resolve_base_url(catalog, &r)?.to_string();
    if super::budget::pick(candidates, slo).is_none() {
        return None;
    }
    Some(Routing {
        provider: r.provider,
        model: r.model,
        base_url: base,
    })
}

/// Where finished-turn usage goes. SQLite recording lands with the
/// daemon session store (Phase 3); tests use the memory sink.
pub trait UsageSink {
    fn record(&mut self, model: &str, input: i64, output: i64);
}

#[derive(Debug, Default)]
pub struct MemorySink {
    pub records: Vec<(String, i64, i64)>,
}

impl UsageSink for MemorySink {
    fn record(&mut self, model: &str, input: i64, output: i64) {
        self.records.push((model.into(), input, output));
    }
}

/// Failover chain: ordered routings, cheapest-first. On adapter error
/// the loop advances to the next entry and logs the switch as a
/// `gate.decision` audit event. Pure selection — transport-agnostic.
#[derive(Debug, Clone, Default)]
pub struct FailoverChain {
    pub routings: Vec<Routing>,
}

impl FailoverChain {
    /// Routing after `failed` consecutive failures from the head.
    /// `None` = chain exhausted, surface the error to the human.
    pub fn after_failures(&self, failed: usize) -> Option<&Routing> {
        self.routings.get(failed)
    }
}

async fn models(
    axum::extract::State(catalog): axum::extract::State<Arc<Catalog>>,
) -> Json<Catalog> {
    Json((*catalog).clone())
}

async fn unimplemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

pub fn router(catalog: Catalog) -> Router {
    let shared = Arc::new(catalog);
    Router::new()
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", axum::routing::post(unimplemented))
        .route("/v1/responses", axum::routing::post(unimplemented))
        .route("/v1/messages", axum::routing::post(unimplemented))
        .with_state(shared)
}

/// Bind the loopback gateway. Keys stay local; only 127.0.0.1 in Phase 1.
pub async fn serve(catalog: Catalog, bind: &str) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, router(catalog)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn resolves_known_provider() {
        let catalog = Catalog {
            providers: [(
                "acme".into(),
                super::super::catalog::ProviderInfo {
                    base_url: "https://a.example.com/v1".into(),
                    models: HashMap::new(),
                },
            )]
            .into(),
        };
        let r = ModelRef::parse("acme/m1").unwrap();
        assert_eq!(
            resolve_base_url(&catalog, &r),
            Some("https://a.example.com/v1")
        );
    }

    #[test]
    fn routes_with_slo_and_sinks_usage() {
        use super::super::budget::{Candidate, Slo};
        let catalog = Catalog {
            providers: [(
                "acme".into(),
                super::super::catalog::ProviderInfo {
                    base_url: "https://a.example.com/v1".into(),
                    models: HashMap::new(),
                },
            )]
            .into(),
        };
        let cs = vec![Candidate {
            name: "m1".into(),
            cost_per_1k: 0.1,
            latency_ms: 200,
        }];
        let slo = Slo {
            max_cost_per_1k: 0.5,
            max_latency_ms: 500,
        };
        let routing = route_model(&catalog, "acme/m1", &cs, &slo).unwrap();
        assert_eq!(routing.base_url, "https://a.example.com/v1");
        let mut sink = super::MemorySink::default();
        super::UsageSink::record(&mut sink, "acme/m1", 10, 5);
        assert_eq!(sink.records.len(), 1);
    }

    #[test]
    fn failover_advances_then_exhausts() {
        let chain = super::FailoverChain {
            routings: vec![
                super::Routing {
                    provider: "a".into(),
                    model: "m".into(),
                    base_url: "https://a.example.com".into(),
                },
                super::Routing {
                    provider: "b".into(),
                    model: "m".into(),
                    base_url: "https://b.example.com".into(),
                },
            ],
        };
        assert_eq!(chain.after_failures(0).unwrap().provider, "a");
        assert_eq!(chain.after_failures(1).unwrap().provider, "b");
        assert!(chain.after_failures(2).is_none());
    }
}
