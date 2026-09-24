//! Merge policy: editable defaults mirroring `config.example.yaml`.
//! The daemon maps its `sentinel:` section onto this struct; the gate
//! itself stays pure and unit-tested.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub require_checks: bool,
    pub require_human_approve: bool,
    pub block_direct_main_push: bool,
    /// Warn above this diff size (default 1000 lines).
    pub warn_large_diff_lines: u64,
    /// Block blobs above this size (default 1 MiB).
    pub block_blob_bytes: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            require_checks: true,
            require_human_approve: true,
            block_direct_main_push: true,
            warn_large_diff_lines: 1000,
            block_blob_bytes: 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_example_config() {
        let p = Policy::default();
        assert!(p.require_checks && p.require_human_approve && p.block_direct_main_push);
        assert_eq!(p.warn_large_diff_lines, 1000);
    }
}
