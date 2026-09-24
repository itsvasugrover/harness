//! Pipeline: align -> classify -> crush -> stats. One entry point so
//! the gateway, relay, and tools all meter identically.
use super::align::align;
use super::crush_code::crush_code;
use super::crush_json::crush_json;
use super::crush_text::crush_text;
use super::port::PressStats;
use super::route::{classify, BlockKind};

/// Press one block. Small/dense input returns byte-identical with a
/// zero-delta stat (honest floor: prose compresses ~nothing).
/// Volatile lines ride the tail so the stable prefix stays cache-hot.
pub fn press_block(text: &str) -> (String, PressStats) {
    let bytes_in = text.len() as u64;
    let (stable, volatile) = align(text);
    let crushed = match classify(&stable) {
        BlockKind::Json => crush_json(&stable).unwrap_or_else(|| stable.clone()),
        BlockKind::Code => crush_code(&stable),
        BlockKind::Text => crush_text(&stable),
    };
    let crushed = if volatile.is_empty() {
        crushed
    } else {
        format!("{crushed}\n--- volatile (cache-busting, read last) ---\n{volatile}")
    };
    let stats = PressStats {
        bytes_in,
        bytes_out: crushed.len() as u64,
        recall_id: None,
    };
    (crushed, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_prose_passes_through() {
        let (out, stats) = press_block("hello world, dense prose here");
        assert_eq!(out, "hello world, dense prose here");
        assert_eq!(stats.tokens_saved_estimate(), 0);
    }

    #[test]
    fn repetitive_logs_shrink() {
        let text = (0..50).map(|_| "ok line").collect::<Vec<_>>().join("\n");
        let (out, stats) = press_block(&text);
        assert!(stats.ratio() > 0.5);
        assert!(out.contains("[x50]"));
    }
}
