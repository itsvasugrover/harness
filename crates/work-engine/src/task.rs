//! Task fan-out: split a goal into smallest units, one subagent each.
//! Agents call agents through this plan — never by dumping context.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub title: String,
    pub scope: String,
    pub budget_steps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spawn {
    pub parent: String,
    pub child: String,
    pub task: SubTask,
    pub depth: u32,
}

/// Max nesting: parent(0) -> child(1) -> grandchild(2). Deeper spawns
/// are rejected so fan-out cannot recurse forever.
pub const MAX_DEPTH: u32 = 2;

impl Spawn {
    pub fn child(parent: &Spawn, child: &str, task: SubTask) -> anyhow::Result<Self> {
        if parent.depth >= MAX_DEPTH {
            anyhow::bail!("depth cap {MAX_DEPTH} reached; child must finish, not spawn");
        }
        Ok(Self {
            parent: parent.child.clone(),
            child: child.into(),
            task,
            depth: parent.depth + 1,
        })
    }

    pub fn root(owner: &str, task: SubTask) -> Self {
        Self {
            parent: owner.into(),
            child: owner.into(),
            task,
            depth: 0,
        }
    }
}

/// Split a goal into at most `max_units` units (one non-empty line =
/// one unit; overflow lines fold into the last unit). Each unit is
/// sized for a single subagent with a bounded step budget.
pub fn split_goal(goal: &str, max_units: usize, budget_steps: u32) -> Vec<SubTask> {
    let max = max_units.max(1);
    let lines: Vec<&str> = goal
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return vec![SubTask {
            title: goal.trim().into(),
            scope: "single unit".into(),
            budget_steps,
        }];
    }
    let mut units: Vec<String> = lines.iter().take(max).map(|s| s.to_string()).collect();
    for extra in lines.iter().skip(max) {
        let last = units.len() - 1;
        units[last].push_str("; ");
        units[last].push_str(extra);
    }
    units
        .into_iter()
        .enumerate()
        .map(|(i, title)| SubTask {
            scope: format!("unit {i} of goal"),
            title,
            budget_steps,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_folds_overflow() {
        let units = split_goal("a\nb\nc", 2, 25);
        assert_eq!(units.len(), 2);
        assert!(units[1].title.contains('c'));
        assert_eq!(units[0].budget_steps, 25);
    }

    #[test]
    fn depth_cap_rejects_grandchildren() {
        let task = || SubTask {
            title: "t".into(),
            scope: "s".into(),
            budget_steps: 1,
        };
        let root = Spawn::root("a", task());
        let child = Spawn::child(&root, "b", task()).unwrap();
        let grand = Spawn::child(&child, "c", task()).unwrap();
        assert!(Spawn::child(&grand, "d", task()).is_err());
    }
}
