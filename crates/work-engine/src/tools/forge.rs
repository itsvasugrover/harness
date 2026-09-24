//! forge: repos/issues/PRs/checks through the daemon's Forge clients.
//! Workers never hold tokens: this tool takes a session
//! [`CapabilityLease`](forge_bridge::port::CapabilityLease) plus a
//! daemon-provided executor that resolves credentials per call. The
//! lease gates every op by capability *and* binds the repo scope —
//! denied or out-of-scope calls fail as tool errors, never panics.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};
use forge_bridge::port::CapabilityLease;
use std::sync::Arc;

/// Daemon-side execution. Implemented by `harnessd`, which owns the
/// clients, the keychain reads, and the audit write for each call.
pub trait ForgeExec: Send + Sync {
    fn exec(&self, lease: &CapabilityLease, op: &str, input: &str) -> Result<String>;
}

pub struct ForgeTool {
    lease: CapabilityLease,
    exec: Arc<dyn ForgeExec>,
}

impl ForgeTool {
    pub fn new(lease: CapabilityLease, exec: Arc<dyn ForgeExec>) -> Self {
        Self { lease, exec }
    }

    /// Lease capability an op requires. Never raw forge scopes.
    pub fn cap_for(op: &str) -> &'static str {
        match op {
            "issues" | "issue" | "pull" | "pulls" | "checks" => "forge.read",
            "open_issue" => "issue.write",
            "comment" => "pr.comment",
            "approve" => "pr.approve",
            "open_pull" => "pr.open",
            "merge" => "pr.merge",
            "review" => "review.request",
            _ => "forge.read",
        }
    }
}

impl Tool for ForgeTool {
    fn name(&self) -> &'static str {
        "forge"
    }

    fn description(&self) -> &'static str {
        "Forge ops as JSON: {op, repo, ...}. Reads need forge.read; writes need the matching lease cap."
    }

    fn run(&self, _ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let v: serde_json::Value = serde_json::from_str(input)
            .map_err(|_| anyhow::anyhow!("forge: input must be JSON"))?;
        let op = v.get("op").and_then(|x| x.as_str()).unwrap_or("");
        let repo = v.get("repo").and_then(|x| x.as_str()).unwrap_or("");
        if op.is_empty() || repo.is_empty() {
            bail!("forge: need {{op, repo}} (got {input})");
        }
        if self.lease.repo != repo {
            bail!("forge: lease covers '{}', not '{repo}'", self.lease.repo);
        }
        let cap = Self::cap_for(op);
        if !self.lease.allows(cap) {
            bail!("forge: lease lacks '{cap}' (expired, revoked, or ungranted)");
        }
        let out = self.exec.exec(&self.lease, op, input)?;
        Ok(ToolOutput {
            title: format!("forge {op}"),
            output: super::super::tool::truncate(&out),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeExec;
    impl ForgeExec for FakeExec {
        fn exec(&self, _l: &CapabilityLease, op: &str, _i: &str) -> Result<String> {
            Ok(format!("{op} ok"))
        }
    }

    fn tool(caps: Vec<String>) -> ForgeTool {
        ForgeTool::new(
            CapabilityLease::mint("github", "o/r", caps),
            Arc::new(FakeExec),
        )
    }

    fn ctx() -> ToolCtx {
        ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        }
    }

    #[test]
    fn allows_scoped_read() {
        let t = tool(vec!["forge.read".into()]);
        let out = t.run(&ctx(), r#"{"op":"issues","repo":"o/r"}"#).unwrap();
        assert_eq!(out.output, "issues ok");
    }

    #[test]
    fn denies_wrong_repo_and_missing_cap() {
        let t = tool(vec!["forge.read".into()]);
        assert!(t
            .run(&ctx(), r#"{"op":"issues","repo":"o/other"}"#)
            .is_err());
        assert!(t.run(&ctx(), r#"{"op":"merge","repo":"o/r"}"#).is_err());
        assert!(t.run(&ctx(), "not json").is_err());
    }
}
