//! Command Deck sidecar: harnessd lifecycle for the Tauri shell.
//! Owns starting, supervising, and stopping the loopback daemon plus
//! the single-instance lock and the log tail the UI viewer reads.
//! Thin by rule: no agent logic, no board derivation, no forge calls.
pub mod lock;
pub mod logs;
pub mod manager;

pub use lock::{claim, Owner};
pub use logs::tail;
pub use manager::{note, resolve_binary, Daemon};

pub fn version() -> &'static str {
    "0.1.0"
}
