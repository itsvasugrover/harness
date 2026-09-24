//! Planner: project-level sequencing. Breaks a goal into units and
//! assigns one worker per unit; workers own implementation, the
//! planner owns order and follow-ups.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub worker_id: String,
    pub unit_title: String,
    pub unit_scope: String,
}

/// Assign units round-robin to fresh worker ids (`w1`, `w2`, ...).
pub fn plan(goal_units: &[(String, String)]) -> Vec<Assignment> {
    goal_units
        .iter()
        .enumerate()
        .map(|(i, (title, scope))| Assignment {
            worker_id: format!("w{}", i + 1),
            unit_title: title.clone(),
            unit_scope: scope.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_one_worker_per_unit() {
        let a = plan(&[("u1".into(), "s1".into()), ("u2".into(), "s2".into())]);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].worker_id, "w1");
        assert_ne!(a[0].worker_id, a[1].worker_id);
    }
}
