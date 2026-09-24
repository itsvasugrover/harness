//! Loop turn: one tool dispatch with a step budget (Phase 1a).
//! Streaming + permissions + sessions land across Phase 1b/c.
use super::registry::Registry;
use super::tool::{ToolCtx, ToolOutput};
use anyhow::{bail, Result};

pub struct LoopConfig {
    pub max_steps: u32,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self { max_steps: 50 }
    }
}

/// Doom-loop guard: same tool+args 3x in a row stops with an
/// explanation instead of burning budget. One per unit/run.
#[derive(Debug, Default)]
pub struct DoomGuard {
    last_key: Option<String>,
    repeats: u32,
}

impl DoomGuard {
    pub fn check(&mut self, tool: &str, input: &str) -> anyhow::Result<()> {
        let key = format!("{tool}\0{input}");
        if self.last_key.as_deref() == Some(&key) {
            self.repeats += 1;
        } else {
            self.last_key = Some(key);
            self.repeats = 1;
        }
        if self.repeats >= 3 {
            anyhow::bail!(
                "doom-loop: '{tool}' with identical input 3x — stopping with explanation"
            );
        }
        Ok(())
    }
}

/// Writes and out-of-scope actions pause for approval. Read-only
/// tools (read/glob/grep/recall + forge reads) never block.
pub fn needs_approval(tool: &str, input: &str) -> bool {
    match tool {
        "write" | "edit" | "bash" | "webfetch" => true,
        "forge" => {
            let op = serde_json::from_str::<serde_json::Value>(input)
                .ok()
                .and_then(|v| v.get("op").and_then(|x| x.as_str()).map(String::from))
                .unwrap_or_default();
            !matches!(
                op.as_str(),
                "issues" | "issue" | "pull" | "pulls" | "checks"
            )
        }
        _ => false,
    }
}

/// Pre-turn gate: the 40% rule is a SOFT cap, not a wall. Past the
/// early line the agent finishes its current unit of work; the next
/// time it would need fresh input, the caller exports a handover and
/// delegates to a successor agent instead of growing this context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Proceed,
    Delegate {
        status: super::overflow::ContextStatus,
    },
}

pub fn gate(used: u64, input_limit: u64, reserved: u64, output_max: u64) -> Gate {
    match super::overflow::status(used, input_limit, reserved, output_max) {
        super::overflow::ContextStatus::Ok => Gate::Proceed,
        s => Gate::Delegate { status: s },
    }
}

/// Execute a single turn: exactly one tool call. Pure dispatch so the
/// streaming loop can reuse it unchanged; always call `gate()` first.
/// Single-turn dispatch with a fresh guard (no cross-turn doom tracking).
/// Prefer run_turn_guarded with a persistent DoomGuard for real loops.
pub fn run_turn(
    registry: &Registry,
    ctx: &ToolCtx,
    tool: &str,
    input: &str,
    steps_used: u32,
    cfg: &LoopConfig,
) -> Result<ToolOutput> {
    let mut guard = DoomGuard::default();
    run_turn_guarded(GuardedTurn {
        registry,
        ctx,
        tool,
        input,
        steps_used,
        cfg,
        guard: &mut guard,
        preapproved: &[],
    })
}

/// Guarded dispatch: max-steps + doom-loop + approval gate.
/// `preapproved` lists tools the session scope already approved
/// (e.g. read-only set or a human-approved once-token).
pub struct GuardedTurn<'a> {
    pub registry: &'a Registry,
    pub ctx: &'a ToolCtx,
    pub tool: &'a str,
    pub input: &'a str,
    pub steps_used: u32,
    pub cfg: &'a LoopConfig,
    pub guard: &'a mut DoomGuard,
    pub preapproved: &'a [String],
}

pub fn run_turn_guarded(g: GuardedTurn<'_>) -> Result<ToolOutput> {
    if g.steps_used >= g.cfg.max_steps {
        bail!("max_steps ({}) reached — stopping", g.cfg.max_steps);
    }
    g.guard.check(g.tool, g.input)?;
    if needs_approval(g.tool, g.input) && !g.preapproved.iter().any(|a| a == g.tool || a == "*") {
        bail!(
            "approval required for '{}' — pending human approve/deny (deny wins)",
            g.tool
        );
    }
    g.registry.run(g.ctx, g.tool, g.input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gates_early_context() {
        assert_eq!(gate(10, 200, 20, 20), Gate::Proceed);
        assert!(matches!(gate(72, 200, 20, 20), Gate::Delegate { .. }));
        assert!(matches!(gate(180, 200, 20, 20), Gate::Delegate { .. }));
    }

    #[test]
    fn doom_guard_trips_on_triple_repeat() {
        let mut g = DoomGuard::default();
        assert!(g.check("read", "a.txt").is_ok());
        assert!(g.check("read", "a.txt").is_ok());
        assert!(g.check("read", "a.txt").is_err());
        // Different input resets the streak.
        let mut g2 = DoomGuard::default();
        assert!(g2.check("read", "a").is_ok());
        assert!(g2.check("read", "b").is_ok());
        assert!(g2.check("read", "a").is_ok());
    }

    #[test]
    fn approval_gate_blocks_writes_but_not_reads() {
        assert!(!needs_approval("read", "a.txt"));
        assert!(needs_approval("write", "{}"));
        assert!(needs_approval("bash", "ls"));
        assert!(!needs_approval("forge", r#"{"op":"issues","repo":"o/r"}"#));
        assert!(needs_approval("forge", r#"{"op":"merge","repo":"o/r"}"#));
        let r = Registry::new();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        let mut guard = DoomGuard::default();
        assert!(run_turn_guarded(GuardedTurn {
            registry: &r,
            ctx: &ctx,
            tool: "write",
            input: "{}",
            steps_used: 0,
            cfg: &LoopConfig::default(),
            guard: &mut guard,
            preapproved: &[]
        })
        .is_err());
    }

    #[test]
    fn rejects_unknown_tool() {
        let r = Registry::new();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        assert!(run_turn(&r, &ctx, "nope", "", 0, &LoopConfig::default()).is_err());
    }
}
