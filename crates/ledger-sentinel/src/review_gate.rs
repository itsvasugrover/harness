//! Merge gate (before): policy check on consequential actions.
//! Verdict is `pass|warn|block` with reasons; `block` must resolve
//! before merge. Pure function — the daemon feeds it facts from the
//! observer store, human approvals, and CI scan results.
use super::policy::Policy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Block,
}

/// Everything the gate reads. Detectors (CI, scanners, reviewers)
/// feed this in; the gate only judges.
#[derive(Debug, Clone, Default)]
pub struct MergeInput {
    pub checks_green: bool,
    pub human_approved: bool,
    pub unresolved_block_threads: u64,
    pub secret_hit: bool,
    pub large_blob: bool,
    pub diff_lines: u64,
    pub migration_without_rollback: bool,
    pub version_bump_without_changelog: bool,
}

/// Judge a merge. Blocks dominate; warns attach reasons but pass.
pub fn evaluate(policy: &Policy, input: &MergeInput) -> (Verdict, Vec<String>) {
    let mut blocks = vec![];
    let mut warns = vec![];
    if policy.require_checks && !input.checks_green {
        blocks.push("failing checks".into());
    }
    if policy.require_human_approve && !input.human_approved {
        blocks.push("missing human approval".into());
    }
    if input.unresolved_block_threads > 0 {
        blocks.push(format!(
            "{} unresolved block threads",
            input.unresolved_block_threads
        ));
    }
    if input.secret_hit {
        blocks.push("secret pattern in diff".into());
    }
    if input.large_blob {
        blocks.push("binary blob over limit".into());
    }
    if input.diff_lines > policy.warn_large_diff_lines {
        warns.push(format!("large diff ({} lines)", input.diff_lines));
    }
    if input.migration_without_rollback {
        warns.push("migration without rollback note".into());
    }
    if input.version_bump_without_changelog {
        warns.push("version bump without changelog entry".into());
    }
    if !blocks.is_empty() {
        (Verdict::Block, [blocks, warns].concat())
    } else if !warns.is_empty() {
        (Verdict::Warn, warns)
    } else {
        (Verdict::Pass, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn green() -> MergeInput {
        MergeInput {
            checks_green: true,
            human_approved: true,
            ..Default::default()
        }
    }

    #[test]
    fn green_merge_passes() {
        let (v, reasons) = evaluate(&Policy::default(), &green());
        assert_eq!(v, Verdict::Pass);
        assert!(reasons.is_empty());
    }

    #[test]
    fn red_checks_and_missing_approval_block() {
        let (v, reasons) = evaluate(&Policy::default(), &MergeInput::default());
        assert_eq!(v, Verdict::Block);
        assert_eq!(reasons.len(), 2);
    }

    #[test]
    fn large_diff_warns_but_passes_gate() {
        let input = MergeInput {
            diff_lines: 5000,
            ..green()
        };
        let (v, reasons) = evaluate(&Policy::default(), &input);
        assert_eq!(v, Verdict::Warn);
        assert_eq!(reasons.len(), 1);
    }

    #[test]
    fn secrets_and_threads_block() {
        let input = MergeInput {
            secret_hit: true,
            unresolved_block_threads: 1,
            ..green()
        };
        let (v, _) = evaluate(&Policy::default(), &input);
        assert_eq!(v, Verdict::Block);
    }
}
