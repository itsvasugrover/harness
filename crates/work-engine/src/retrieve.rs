//! Bet 7 — semantic tool retrieval (stub; full build Phase 2).
//! Top-k tools per turn instead of dumping all definitions every prompt.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTool {
    pub name: String,
    pub score: f32,
}

/// Substring-overlap score over name+description tokens. Zero-cost
/// stand-in until the embedding index lands.
pub fn score(query: &str, name: &str, description: &str) -> f32 {
    let q: Vec<&str> = query.split_whitespace().collect();
    if q.is_empty() {
        return 0.0;
    }
    let hay = format!("{name} {description}").to_lowercase();
    let hits = q.iter().filter(|w| hay.contains(&w.to_lowercase())).count();
    hits as f32 / q.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_matching_tool_first() {
        assert!(
            score("read file", "read", "read a file") > score("read file", "bash", "run shell")
        );
    }
}
