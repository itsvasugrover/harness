//! Compaction select: newest-first within a protected tail budget.
//! The summarizer (Phase 2) condenses the dropped middle; pruned parts
//! are marked compacted, never deleted.
use super::session::estimate_tokens;

/// Ids to keep, newest first, while estimated size fits `budget`.
pub fn select_keep(newest_first: &[String], bodies: &[&str], budget: u64) -> Vec<String> {
    let mut keep = Vec::new();
    let mut used = 0u64;
    for (id, body) in newest_first.iter().zip(bodies.iter()) {
        let size = estimate_tokens(body);
        if used + size > budget {
            break;
        }
        used += size;
        keep.push(id.clone());
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_newest_within_budget() {
        let ids = vec!["n3".into(), "n2".into(), "n1".into()];
        let bodies = vec!["aaaa", "aaaa", "aaaa"];
        assert_eq!(
            select_keep(&ids, &bodies, 2),
            vec!["n3".to_string(), "n2".to_string()]
        );
    }
}
