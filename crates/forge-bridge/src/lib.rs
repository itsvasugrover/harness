//! Forge Bridge: one trait, GitHub + Gitea adapters.
//! Split: port.rs http.rs parse.rs github.rs gitea.rs observer.rs intents.rs mask.rs.
pub mod gitea;
pub mod github;
pub mod http;
pub mod intents;
pub mod mask;
pub mod parse;
pub mod port;
