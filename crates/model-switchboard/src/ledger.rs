//! Bet 10 — cost attribution per worker (stub; full build Phase 2).
//! Live spend per session from provider-reported usage; kill-switch
//! pauses over-budget workers from either deck.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Spend {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cost_usd: f64,
}

impl Spend {
    pub fn add(&mut self, other: &Spend) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_write += other.cache_write;
        self.cost_usd += other.cost_usd;
    }

    pub fn over_budget(&self, budget_usd: f64) -> bool {
        self.cost_usd > budget_usd
    }
}

/// Dollar pricing per `provider/model`, from config (never hardcoded).
/// Missing entries meter tokens at $0.00 — visible, never billed blind.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriceTable {
    #[serde(default)]
    pub per_1k_in: HashMap<String, f64>,
    #[serde(default)]
    pub per_1k_out: HashMap<String, f64>,
}

impl PriceTable {
    pub fn cost_for(&self, model: &str, input: u64, output: u64) -> f64 {
        let pin = self.per_1k_in.get(model).copied().unwrap_or(0.0);
        let pout = self.per_1k_out.get(model).copied().unwrap_or(0.0);
        input as f64 / 1000.0 * pin + output as f64 / 1000.0 * pout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_and_trips() {
        let mut s = Spend::default();
        s.add(&Spend {
            cost_usd: 1.5,
            ..Default::default()
        });
        assert!(s.over_budget(1.0));
        assert!(!s.over_budget(2.0));
    }

    #[test]
    fn prices_from_table() {
        let t = PriceTable {
            per_1k_in: [("acme/m1".into(), 1.0)].into(),
            per_1k_out: [("acme/m1".into(), 3.0)].into(),
        };
        assert!((t.cost_for("acme/m1", 1000, 1000) - 4.0).abs() < 1e-9);
        assert_eq!(t.cost_for("unknown/m", 999_999, 999_999), 0.0);
    }
}
