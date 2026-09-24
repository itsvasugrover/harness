//! Trim command filters. One file per group; unknown input passes
//! through byte-identical (filter-or-passthrough, never break).
pub mod files;
pub mod git;
pub mod tests;
pub mod tracking;
