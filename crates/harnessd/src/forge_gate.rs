//! Forge merge gate helper: live facts + approvals into `evaluate()`.
//! Split from `forge_ops.rs` per the 300-line rule; one job per file.
use forge_bridge::port::PullFull;
use ledger_sentinel::review_gate::MergeInput;

/// Build live merge inputs from observer-grade facts + caller approvals.
/// Diff-content signals (secrets, blobs, migrations) default off until
/// full diff fetch lands; checks/threads/approval are live today.
pub(crate) fn merge_input_from_pull(pull: &PullFull, approved: bool) -> MergeInput {
    let failing = pull.checks.iter().any(|c| {
        matches!(
            c.conclusion.as_str(),
            "failure" | "cancelled" | "timed_out" | "action_required"
        )
    });
    let unresolved = pull.threads.iter().filter(|th| !th.resolved).count() as u64;
    MergeInput {
        checks_green: !failing,
        human_approved: approved,
        unresolved_block_threads: unresolved,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use forge_bridge::port::{Check, PullFull, ReviewThread};
    use ledger_sentinel::{policy::Policy, review_gate::Verdict};

    #[test]
    fn gate_blocks_unapproved_or_red_checks() {
        let pull = PullFull {
            number: 1,
            head_sha: "abc".into(),
            checks: vec![Check {
                name: "ci".into(),
                status: "completed".into(),
                conclusion: "failure".into(),
            }],
            threads: vec![ReviewThread {
                id: "t1".into(),
                resolved: false,
                ..Default::default()
            }],
            ..Default::default()
        };
        let (v, reasons) = ledger_sentinel::review_gate::evaluate(
            &Policy::default(),
            &super::merge_input_from_pull(&pull, false),
        );
        assert_eq!(v, Verdict::Block);
        assert!(reasons.len() >= 2);
        let green = PullFull {
            number: 1,
            head_sha: "abc".into(),
            ..Default::default()
        };
        let (v2, _) = ledger_sentinel::review_gate::evaluate(
            &Policy::default(),
            &super::merge_input_from_pull(&green, true),
        );
        assert_eq!(v2, Verdict::Pass);
    }
}
