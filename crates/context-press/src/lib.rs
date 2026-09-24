//! Context Press: align -> route -> crush -> store.
//! Split: align.rs route.rs crush_json.rs crush_text.rs crush_code.rs
//! pipeline.rs shared.rs store.rs.
pub mod align;
pub mod crush_code;
pub mod crush_json;
pub mod crush_text;
pub mod pipeline;
pub mod port;
pub mod route;
pub mod shared;
pub mod store;
