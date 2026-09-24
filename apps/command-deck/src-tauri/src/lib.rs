//! Tauri sidecar placeholder: daemon lifecycle + single-instance lock.
//! Real impl: spawn harnessd, manage ~/.harness, updater (see docs).
pub fn version() -> &'static str {
    "0.1.0"
}
