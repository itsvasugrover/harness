//! Bet 9 — repo fingerprint sync (stub; full build Phase 3).
//! Hash of `.harness/` + `AGENTS.md` pinned per session; mid-run drift
//! pauses the worker until a human accepts or pins.
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub repo: String,
    pub hash: String,
}

pub fn hash_files(names: &[&str], bodies: &[&str]) -> String {
    let mut h = DefaultHasher::new();
    for (n, b) in names.iter().zip(bodies.iter()) {
        n.hash(&mut h);
        b.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_sensitive() {
        let a = hash_files(&["AGENTS.md"], &["x"]);
        assert_eq!(a, hash_files(&["AGENTS.md"], &["x"]));
        assert_ne!(a, hash_files(&["AGENTS.md"], &["y"]));
    }
}
