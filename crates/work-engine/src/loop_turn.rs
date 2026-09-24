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
pub fn run_turn(
    registry: &Registry,
    ctx: &ToolCtx,
    tool: &str,
    input: &str,
    steps_used: u32,
    cfg: &LoopConfig,
) -> Result<ToolOutput> {
    if steps_used >= cfg.max_steps {
        bail!("max_steps ({}) reached — stopping", cfg.max_steps);
    }
    registry.run(ctx, tool, input)
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
    fn rejects_unknown_tool() {
        let r = Registry::new();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        assert!(run_turn(&r, &ctx, "nope", "", 0, &LoopConfig::default()).is_err());
    }
}
