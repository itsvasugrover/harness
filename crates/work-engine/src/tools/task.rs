//! task: goal fan-out via `split_goal` (one non-empty scope per unit).
//! Input: {goal, max_units?}. Output: numbered units with scopes.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::Result;

pub struct Task;

impl Tool for Task {
    fn name(&self) -> &'static str {
        "task"
    }

    fn description(&self) -> &'static str {
        "Split a goal: {goal, max_units?} -> numbered units."
    }

    fn run(&self, _ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let v: serde_json::Value =
            serde_json::from_str(input).map_err(|_| anyhow::anyhow!("task: input must be JSON"))?;
        let goal = v.get("goal").and_then(|x| x.as_str()).unwrap_or("").trim();
        if goal.is_empty() {
            anyhow::bail!("task: need {{goal}}");
        }
        let max = v
            .get("max_units")
            .and_then(|x| x.as_u64())
            .unwrap_or(8)
            .clamp(1, 25) as usize;
        let units = super::super::task::split_goal(goal, max, 25);
        let out = units
            .iter()
            .enumerate()
            .map(|(i, u)| format!("{}. {} — {}", i + 1, u.title, u.scope))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutput {
            title: format!("task split ({} units)", units.len()),
            output: super::super::tool::truncate(&out),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_goal_into_units() {
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        let out = Task.run(&ctx, r#"{"goal":"do x\ndo y"}"#).unwrap();
        assert!(out.output.contains("do x"));
    }
}
