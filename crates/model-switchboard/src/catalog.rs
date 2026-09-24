//! Model catalog: nightly fetch + local cache + config overlay.
//! Overlay wins; missing adapter falls back to acme-compatible.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelInfo {
    pub context_limit: u64,
    pub output_limit: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub base_url: String,
    #[serde(default)]
    pub models: HashMap<String, ModelInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    #[serde(default)]
    pub providers: HashMap<String, ProviderInfo>,
}

impl Catalog {
    /// Overlay a config-layer catalog: providers merge, models merge,
    /// overlay fields win on conflict.
    pub fn apply_overlay(&mut self, overlay: Catalog) {
        for (name, prov) in overlay.providers {
            let entry = self.providers.entry(name).or_default();
            if !prov.base_url.is_empty() {
                entry.base_url = prov.base_url;
            }
            entry.models.extend(prov.models);
        }
    }

    pub fn get(&self, provider: &str, model: &str) -> Option<(&ProviderInfo, &ModelInfo)> {
        let p = self.providers.get(provider)?;
        Some((p, p.models.get(model)?))
    }
}

/// Fetch a remote catalog JSON (models.dev shape subset). Cache + ETag
/// handling lands with the gateway server in Phase 1b.
pub async fn fetch(url: &str) -> Result<Catalog> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let catalog = client.get(url).send().await?.json::<Catalog>().await?;
    Ok(catalog)
}

/// Default upstream listing (opencode-style). Base, never authority:
/// local config overlays and wins on every field it sets.
pub const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// Parse a models.dev document into our catalog. models.dev carries no
/// base URLs — those come from local config, which is also how your
/// GLM/DeepSeek/Muse-Spark endpoints attach.
pub fn from_models_dev(text: &str) -> Result<Catalog> {
    let doc: serde_json::Value = serde_json::from_str(text)?;
    let mut catalog = Catalog::default();
    let map = doc
        .as_object()
        .context("models.dev root must be an object")?;
    for (provider_id, prov) in map {
        let mut info = ProviderInfo::default();
        if let Some(models) = prov.get("models").and_then(|m| m.as_object()) {
            for (model_id, m) in models {
                let context = m
                    .pointer("/limit/context")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let output = m
                    .pointer("/limit/output")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if context > 0 {
                    info.models.insert(
                        model_id.clone(),
                        ModelInfo {
                            context_limit: context,
                            output_limit: output,
                        },
                    );
                }
            }
        }
        if !info.models.is_empty() {
            catalog.providers.insert(provider_id.clone(), info);
        }
    }
    Ok(catalog)
}

/// Prices from models.dev (`cost.input/output`, documented per-1M
/// tokens — verify against your bill; config prices always win).
pub fn prices_from_models_dev(text: &str) -> super::ledger::PriceTable {
    let mut table = super::ledger::PriceTable::default();
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(text) else {
        return table;
    };
    if let Some(map) = doc.as_object() {
        for (provider_id, prov) in map {
            if let Some(models) = prov.get("models").and_then(|m| m.as_object()) {
                for (model_id, m) in models {
                    let key = format!("{provider_id}/{model_id}");
                    if let Some(v) = m.pointer("/cost/input").and_then(|v| v.as_f64()) {
                        table.per_1k_in.insert(key.clone(), v / 1000.0);
                    }
                    if let Some(v) = m.pointer("/cost/output").and_then(|v| v.as_f64()) {
                        table.per_1k_out.insert(key, v / 1000.0);
                    }
                }
            }
        }
    }
    table
}

/// Fetch with a file cache: fresh cache wins, else network, else the
/// stale cache, else empty (boot never fails on network).
pub async fn fetch_cached(url: &str, cache_path: &str, ttl_secs: u64) -> Catalog {
    let fresh = std::fs::metadata(cache_path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|d| d.as_secs() < ttl_secs).unwrap_or(false))
        .unwrap_or(false);
    if fresh {
        if let Ok(text) = std::fs::read_to_string(cache_path) {
            if let Ok(catalog) = from_models_dev(&text) {
                return catalog;
            }
        }
    }
    if let Ok(text) = fetch_text(url).await {
        let _ = std::fs::create_dir_all(
            std::path::Path::new(cache_path)
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        );
        let _ = std::fs::write(cache_path, &text);
        if let Ok(catalog) = from_models_dev(&text) {
            return catalog;
        }
    }
    std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|text| from_models_dev(&text).ok())
        .unwrap_or_default()
}

async fn fetch_text(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    Ok(client.get(url).send().await?.text().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_wins() {
        let mut base = Catalog::default();
        base.providers.insert(
            "acme".into(),
            ProviderInfo {
                base_url: "https://a.example.com/v1".into(),
                models: HashMap::new(),
            },
        );
        let mut over = Catalog::default();
        over.providers.insert(
            "acme".into(),
            ProviderInfo {
                base_url: String::new(),
                models: [(
                    "m1".into(),
                    ModelInfo {
                        context_limit: 200_000,
                        output_limit: 8_000,
                    },
                )]
                .into(),
            },
        );
        base.apply_overlay(over);
        let (p, m) = base.get("acme", "m1").unwrap();
        assert_eq!(p.base_url, "https://a.example.com/v1");
        assert_eq!(m.context_limit, 200_000);
    }

    const MODELS_DEV_SAMPLE: &str = r#"{
        "acme": {"models": {
            "m1": {"limit": {"context": 200000, "output": 8000},
                   "cost": {"input": 1.0, "output": 3.0}},
            "tiny": {"limit": {"context": 0, "output": 0}}
        }}
    }"#;

    #[test]
    fn parses_models_dev_shape() {
        let catalog = from_models_dev(MODELS_DEV_SAMPLE).unwrap();
        let (_, m) = catalog.get("acme", "m1").unwrap();
        assert_eq!(m.context_limit, 200_000);
        assert!(catalog.get("acme", "tiny").is_none());
    }

    #[test]
    fn prices_assume_per_million() {
        let table = prices_from_models_dev(MODELS_DEV_SAMPLE);
        assert!((table.cost_for("acme/m1", 1000, 1000) - 0.004).abs() < 1e-9);
    }

    #[tokio::test]
    async fn stale_cache_survives_no_network() {
        let dir = std::env::temp_dir().join("harness-catalog-cache");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("api.json");
        std::fs::write(&path, MODELS_DEV_SAMPLE).unwrap();
        let catalog = fetch_cached(
            "http://127.0.0.1:9/unreachable",
            path.to_str().unwrap(),
            3600,
        )
        .await;
        assert!(catalog.get("acme", "m1").is_some());
    }
}
