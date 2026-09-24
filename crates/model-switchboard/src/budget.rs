//! Bet 1 — budget-aware model routing (stub; full build Phase 2).
//! Cheapest model that meets the task-class SLO; decision logged.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskClass {
    Explore,
    Build,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slo {
    pub max_cost_per_1k: f64,
    pub max_latency_ms: u64,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub name: String,
    pub cost_per_1k: f64,
    pub latency_ms: u64,
}

/// Cheapest candidate meeting the SLO; None = escalate to default model.
pub fn pick<'a>(candidates: &'a [Candidate], slo: &Slo) -> Option<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| c.cost_per_1k <= slo.max_cost_per_1k && c.latency_ms <= slo.max_latency_ms)
        .min_by(|a, b| a.cost_per_1k.partial_cmp(&b.cost_per_1k).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_cheapest_within_slo() {
        let cs = vec![
            Candidate {
                name: "big".into(),
                cost_per_1k: 1.0,
                latency_ms: 100,
            },
            Candidate {
                name: "small".into(),
                cost_per_1k: 0.1,
                latency_ms: 200,
            },
        ];
        let slo = Slo {
            max_cost_per_1k: 0.5,
            max_latency_ms: 500,
        };
        assert_eq!(pick(&cs, &slo).unwrap().name, "small");
    }
}
