//! Provider config: everything model-related comes from YAML layers
//! (global `~/.harness/config.yaml`, local `.harness/config.yaml`).
//! No provider URL, key name, or price is hardcoded in Rust — adding
//! z.ai GLM, DeepSeek, Muse Spark, or anything OpenAI-compatible is a
//! config edit, never a code change.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ProviderCfg {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub models: HashMap<String, ModelCfg>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HarnessConfig {
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub small_model: String,
    /// Upstream model listing (models.dev-compatible). Empty = skip.
    #[serde(default)]
    pub catalog_url: String,
    #[serde(default)]
    pub providers: HashMap<String, ProviderCfg>,
    /// Watched forges for the PR/CI observer. Empty = observer idle.
    #[serde(default)]
    pub forges: Vec<ForgeCfg>,
    #[serde(default)]
    pub agents_dir: String,
    #[serde(default)]
    pub skills_dir: String,
    #[serde(default)]
    pub commands_dir: String,
    #[serde(default)]
    pub mcp_file: String,
    #[serde(default)]
    pub context_press: PressCfg,
    #[serde(default)]
    pub shell_trim: TrimCfg,
    #[serde(default)]
    pub relay: RelayCfg,
    #[serde(default)]
    pub sentinel: SentinelCfg,
}

pub use super::config_sections::{ForgeCfg, PressCfg, RelayCfg, SentinelCfg, TrimCfg};

/// Load one layer file; missing file = empty layer (not an error).
pub fn load_layer(path: &str) -> Result<HarnessConfig> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(noyalib::from_str(&text).context("config parse failed")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HarnessConfig::default()),
        Err(e) => Err(e.into()),
    }
}

/// Merge local over global: providers/models merge, scalars win when
/// set, nested tables merge field-wise, forges dedupe by kind+URL
/// (local entry replaces the global one it shadows).
pub fn merge(mut base: HarnessConfig, over: HarnessConfig) -> HarnessConfig {
    for (field, value) in [
        (&mut base.default_model, over.default_model),
        (&mut base.small_model, over.small_model),
        (&mut base.catalog_url, over.catalog_url),
        (&mut base.agents_dir, over.agents_dir),
        (&mut base.skills_dir, over.skills_dir),
        (&mut base.commands_dir, over.commands_dir),
        (&mut base.mcp_file, over.mcp_file),
    ] {
        if !value.is_empty() {
            *field = value;
        }
    }
    for (name, prov) in over.providers {
        let entry = base.providers.entry(name).or_default();
        if !prov.base_url.is_empty() {
            entry.base_url = prov.base_url;
        }
        entry.env.extend(prov.env);
        entry.models.extend(prov.models);
    }
    base.forges.retain(|f| {
        !over
            .forges
            .iter()
            .any(|o| o.kind == f.kind && o.base_url == f.base_url)
    });
    base.forges.extend(over.forges);
    if over.context_press.enabled.is_some() {
        base.context_press.enabled = over.context_press.enabled;
    }
    if over.context_press.min_block_words != 0 {
        base.context_press.min_block_words = over.context_press.min_block_words;
    }
    if over.shell_trim.enabled.is_some() {
        base.shell_trim.enabled = over.shell_trim.enabled;
    }
    if !over.shell_trim.level.is_empty() {
        base.shell_trim.level = over.shell_trim.level;
    }
    if over.relay.port != 0 {
        base.relay.port = over.relay.port;
    }
    // Plain bools cannot distinguish unset from false: a fully
    // default local table inherits, anything else wins wholesale.
    if over.sentinel != SentinelCfg::default() {
        base.sentinel = over.sentinel;
    }
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
    fn example_parses_with_every_key_honored() {
        let cfg: HarnessConfig =
            noyalib::from_str(include_str!("../../../config.example.yaml")).unwrap();
        assert_eq!(cfg.small_model, "deepseek/deepseek-chat");
        assert_eq!(cfg.relay.port, 8787);
        assert_eq!(cfg.context_press.enabled, Some(true));
        assert_eq!(cfg.shell_trim.level, "standard");
        assert!(cfg.sentinel.require_checks);
        assert_eq!(cfg.forges.len(), 1);
        assert_eq!(cfg.skills_dir, "./skills");
    }

    #[test]
    fn unknown_keys_fail_loud() {
        assert!(noyalib::from_str::<HarnessConfig>("bogus_key: 1").is_err());
    }
}
