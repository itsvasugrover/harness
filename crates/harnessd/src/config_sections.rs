//! Config sections: every non-provider table in `config.yaml`.
//! Split from `config.rs` for the file-size contract; merge logic
//! stays there. Unknown keys fail loud (`deny_unknown_fields`) so the
//! example file can never promise a knob the code drops.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PressCfg {
    pub enabled: Option<bool>,
    pub min_block_words: u64,
}

impl Default for PressCfg {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            min_block_words: 40,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TrimCfg {
    pub enabled: Option<bool>,
    pub level: String,
}

impl Default for TrimCfg {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            level: "standard".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RelayCfg {
    pub port: u16,
}

impl Default for RelayCfg {
    fn default() -> Self {
        Self { port: 8787 }
    }
}

/// Mirrors `ledger_sentinel::policy::Policy`; the merge gate reads
/// this section once it is live-wired (until then: parsed, not enforced).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SentinelCfg {
    pub require_checks: bool,
    pub require_human_approve: bool,
    pub block_direct_main_push: bool,
    pub warn_large_diff_lines: u64,
    pub block_blob_bytes: u64,
}

impl Default for SentinelCfg {
    fn default() -> Self {
        Self {
            require_checks: true,
            require_human_approve: true,
            block_direct_main_push: true,
            warn_large_diff_lines: 1000,
            block_blob_bytes: 1024 * 1024,
        }
    }
}

/// One watched forge. Tokens never live here: `env` names the vars the
/// daemon resolves at use time (env, then OS keychain). Self-host base
/// URLs (Gitea) come from config; nothing is hardcoded.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[cfg(test)]
mod tests {
    use super::super::config::{merge, HarnessConfig};
    use super::*;

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
            super::super::config::load_layer(g.to_str().unwrap()).unwrap(),
        );
        assert_eq!(merged.forges.len(), 1);
        assert_eq!(merged.forges[0].interval(), 30);
    }

    #[test]
    fn forges_dedupe_by_kind_and_url() {
        let mut base = HarnessConfig::default();
        base.forges.push(ForgeCfg {
            kind: "github".into(),
            base_url: "https://api.github.com".into(),
            repos: vec!["o/old".into()],
            ..Default::default()
        });
        let mut over = HarnessConfig::default();
        over.forges.push(ForgeCfg {
            kind: "github".into(),
            base_url: "https://api.github.com".into(),
            repos: vec!["o/new".into()],
            ..Default::default()
        });
        let merged = merge(base, over);
        assert_eq!(merged.forges.len(), 1);
        assert_eq!(merged.forges[0].repos, vec!["o/new"]);
    }
}
