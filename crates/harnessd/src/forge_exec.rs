//! Daemon forge clients: builds and holds the per-forge clients from
//! config, resolving tokens (env, then keychain) and skipping forges
//! without credentials. Op dispatch lives in `forge_ops.rs`.
use forge_bridge::port::Forge;
use std::collections::HashMap;
use std::sync::Arc;
use work_engine::tools::forge::ForgeExec;

pub struct DaemonForge {
    clients: HashMap<String, Arc<dyn Forge + Send + Sync>>,
}

impl DaemonForge {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, client: Arc<dyn Forge + Send + Sync>) {
        self.clients.insert(name.into(), client);
    }

    /// Build from config, resolving tokens (env, then keychain) and
    /// skipping forges without credentials. Never logs secrets.
    pub fn build(cfg: &super::config::HarnessConfig) -> Self {
        let mut forge = Self::new();
        for c in &cfg.forges {
            let env_refs: Vec<&str> = c.env.iter().map(String::as_str).collect();
            let account = format!("forge-{}", c.kind);
            let Some(token) = super::keys::resolve_all(None, &account, &env_refs) else {
                continue;
            };
            let base = if c.base_url.is_empty() {
                "https://api.github.com"
            } else {
                &c.base_url
            };
            match c.kind.as_str() {
                "github" => {
                    if let Ok(g) = forge_bridge::github::GitHub::new(base, &token) {
                        forge.add("github", Arc::new(g));
                    }
                }
                "gitea" if !c.base_url.is_empty() => {
                    if let Ok(g) = forge_bridge::gitea::Gitea::new(&c.base_url, &token) {
                        forge.add(&c.kind, Arc::new(g));
                    }
                }
                "gitea" => {}
                _ => {}
            }
        }
        forge
    }
}

impl Default for DaemonForge {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeExec for DaemonForge {
    fn exec(
        &self,
        lease: &forge_bridge::port::CapabilityLease,
        op: &str,
        input: &str,
    ) -> anyhow::Result<String> {
        match tokio::runtime::Handle::try_current() {
            Ok(h) => tokio::task::block_in_place(|| {
                h.block_on(super::forge_ops::run_op(&self.clients, lease, op, input))
            }),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(super::forge_ops::run_op(&self.clients, lease, op, input)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_skips_credentialless_forges() {
        let cfg = super::super::config::HarnessConfig::default();
        assert!(DaemonForge::build(&cfg).clients.is_empty());
    }
}
