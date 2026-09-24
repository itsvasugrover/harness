//! Overflow policy: act early, never clip, never a hard wall.
//! At 40% of the usable window the engine stops GROWING this context:
//! it finishes the current unit, then hands over to a successor agent.
//! Pure arithmetic; provider usage wins over estimates when known.

/// Fraction of the usable window that triggers early compaction.
pub const COMPACT_RATIO: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextStatus {
    /// Below the early line: keep working.
    Ok,
    /// At/above 40%: run a compaction turn (or export a handover).
    CompactSoon,
    /// At/above usable: hard compaction before the next model call.
    Overflow,
}

pub fn usable_tokens(input_limit: u64, reserved: u64, output_max: u64) -> u64 {
    input_limit.saturating_sub(reserved.max(output_max))
}

/// Classify current usage. `used` should be provider-reported totals;
/// estimates only when the provider reports nothing.
pub fn status(used: u64, input_limit: u64, reserved: u64, output_max: u64) -> ContextStatus {
    if is_overflow(used, input_limit, reserved, output_max) {
        return ContextStatus::Overflow;
    }
    if input_limit == 0 {
        return ContextStatus::Ok;
    }
    let line = (usable_tokens(input_limit, reserved, output_max) as f64 * COMPACT_RATIO) as u64;
    if used >= line {
        ContextStatus::CompactSoon
    } else {
        ContextStatus::Ok
    }
}

pub fn is_overflow(used: u64, input_limit: u64, reserved: u64, output_max: u64) -> bool {
    input_limit != 0 && used >= usable_tokens(input_limit, reserved, output_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trips_at_usable() {
        assert!(!is_overflow(100, 200, 20, 20));
        assert!(is_overflow(180, 200, 20, 20));
        assert!(!is_overflow(u64::MAX, 0, 0, 0));
    }

    #[test]
    fn early_line_at_forty_percent() {
        // usable = 200 - 20 = 180; 40% line = 72.
        assert_eq!(status(10, 200, 20, 20), ContextStatus::Ok);
        assert_eq!(status(72, 200, 20, 20), ContextStatus::CompactSoon);
        assert_eq!(status(180, 200, 20, 20), ContextStatus::Overflow);
    }
}
