//! Public surface of context-press.
use serde::{Deserialize, Serialize};

/// Honest savings math: input-byte delta per call. Bill share of the
/// session comes from provider-reported usage, never from this struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressStats {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub recall_id: Option<String>,
}

impl PressStats {
    /// Estimated tokens via bytes/4. Labeled estimate everywhere in UI.
    pub fn tokens_saved_estimate(&self) -> u64 {
        self.bytes_in.saturating_sub(self.bytes_out) / 4
    }

    pub fn ratio(&self) -> f64 {
        if self.bytes_in == 0 {
            return 0.0;
        }
        1.0 - (self.bytes_out as f64 / self.bytes_in as f64)
    }
}
