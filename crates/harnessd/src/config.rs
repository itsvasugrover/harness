//! Provider config: everything model-related comes from YAML layers
//! (global `~/.harness/config.yaml`, local `.harness/config.yaml`).
//! No provider URL, key name, or price is hardcoded in Rust — adding
//! z.ai GLM, DeepSeek, Muse Spark, or anything OpenAI-compatible is a
//! config edit, never a code change.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCfg {
    #[serde(default)]
    pub context: u64,
    #[serde(default)]
    pub output: u64,
    /// USD per 1k tokens in / out. Missing = 0.0 (metered, unpriced).
    #[serde(default)]
    pub price_in: f64,
    #[serde(default)]
    pub price_out: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCfg {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub models: HashMap<String, ModelCfg>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HarnessConfig {
    #[serde(default)]
    pub default_model: String,
    /// Upstream model listing (models.dev-compatible). Empty = skip.
    #[serde(default)]
    pub catalog_url: String,
    #[serde(default)]
    pub providers: HashMap<String, ProviderCfg>,
    /// Watched forges for the PR/CI observer. Empty = observer idle.
    #[serde(default)]
    pub forges: Vec<ForgeCfg>,
}

/// One watched forge. Tokens never live here: `env` names the vars the
/// daemon resolves at use time (env, then OS keychain). Self-host base
/// URLs (Gitea) come from config; nothing is hardcoded.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForgeCfg {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default)]
    pub poll_secs: u64,
}

impl ForgeCfg {
    /// Poll cadence, defaulting to the documented 30s when unset.
    pub fn interval(&self) -> u64 {
        if self.poll_secs == 0 {
            30
        } else {
            self.poll_secs
        }
    }
}

/// Load one layer file; missing file = empty layer (not an error).
pub fn load_layer(path: &str) -> Result<HarnessConfig> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(noyalib::from_str(&text).context("config parse failed")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HarnessConfig::default()),
        Err(e) => Err(e.into()),
    }
}

/// Merge local over global: providers/models merge, scalars win.
pub fn merge(mut base: HarnessConfig, over: HarnessConfig) -> HarnessConfig {
    if !over.default_model.is_empty() {
        base.default_model = over.default_model;
    }
    if !over.catalog_url.is_empty() {
        base.catalog_url = over.catalog_url;
    }
    for (name, prov) in over.providers {
        let entry = base.providers.entry(name).or_default();
        if !prov.base_url.is_empty() {
            entry.base_url = prov.base_url;
        }
        entry.env.extend(prov.env);
        entry.models.extend(prov.models);
    }
    base.forges.extend(over.forges);
    base
}

/// Convert to the runtime catalog (base URLs + limits, no secrets).
pub fn to_catalog(cfg: &HarnessConfig) -> model_switchboard::catalog::Catalog {
    let mut catalog = model_switchboard::catalog::Catalog::default();
    for (name, prov) in &cfg.providers {
        let models = prov
            .models
            .iter()
            .map(|(m, c)| {
                (
                    m.clone(),
                    model_switchboard::catalog::ModelInfo {
                        context_limit: c.context,
                        output_limit: c.output,
                    },
                )
            })
            .collect();
        catalog.providers.insert(
            name.clone(),
            model_switchboard::catalog::ProviderInfo {
                base_url: prov.base_url.clone(),
                models,
            },
        );
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_merge_providerwise() {
        let dir = std::env::temp_dir().join("harness-config-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let g = dir.join("global.yaml");
        let l = dir.join("local.yaml");
        std::fs::write(
            &g,
            "providers:\n  acme:\n    base_url: https://a.example.com\n",
        )
        .unwrap();
        std::fs::write(
            &l,
            "default_model: acme/m1\nproviders:\n  acme:\n    env: [ACME_KEY]\n",
        )
        .unwrap();
        let merged = merge(
            load_layer(g.to_str().unwrap()).unwrap(),
            load_layer(l.to_str().unwrap()).unwrap(),
        );
        let acme = &merged.providers["acme"];
        assert_eq!(acme.base_url, "https://a.example.com");
        assert_eq!(merged.default_model, "acme/m1");
        let catalog = to_catalog(&merged);
        assert!(catalog.providers.contains_key("acme"));
    }

    #[test]
    fn forges_extend_and_default_poll() {
        let dir = std::env::temp_dir().join("harness-forge-config-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let g = dir.join("global.yaml");
        std::fs::write(
            &g,
            "forges:\n  - kind: github\n    env: [GH_TOKEN]\n    repos: [o/r]\n",
        )
        .unwrap();
        let merged = merge(
            HarnessConfig::default(),
            load_layer(g.to_str().unwrap()).unwrap(),
        );
        assert_eq!(merged.forges.len(), 1);
        assert_eq!(merged.forges[0].interval(), 30);
    }
}
