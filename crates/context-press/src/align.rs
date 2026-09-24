//! Cache align: volatile lines (timestamps, UUIDs, hashes) move to
//! the tail so the stable prefix keeps hitting the provider KV-cache.
//! Never rewrites words — only reorders lines.
fn is_hex(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn is_uuid_token(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 36
        && b[8] == b'-'
        && b[13] == b'-'
        && b[18] == b'-'
        && b[23] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| [8, 13, 18, 23].contains(&i) || is_hex(*c))
}

fn is_hash_token(t: &str) -> bool {
    (t.len() == 40 || t.len() == 64) && t.bytes().all(is_hex)
}

fn has_timestamp(line: &str) -> bool {
    let b = line.as_bytes();
    b.windows(8)
        .any(|w| w[2] == b':' && w[5] == b':' && w.iter().all(|c| *c == b':' || c.is_ascii_digit()))
        || b.windows(10).any(|w| {
            w[4] == b'-'
                && w[7] == b'-'
                && w.iter()
                    .enumerate()
                    .all(|(i, c)| [4, 7].contains(&i) || c.is_ascii_digit())
        })
}

fn is_volatile(line: &str) -> bool {
    has_timestamp(line)
        || line
            .split_whitespace()
            .any(|t| is_uuid_token(t) || is_hash_token(t))
}

/// Split lines into (stable, volatile). No volatile lines = stable
/// returned as-is, so dense prose is never touched.
pub fn align(text: &str) -> (String, String) {
    let mut stable = Vec::new();
    let mut volatile = Vec::new();
    for line in text.lines() {
        if is_volatile(line) {
            volatile.push(line);
        } else {
            stable.push(line);
        }
    }
    (stable.join("\n"), volatile.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_volatile_tail() {
        let text = "2026-09-23 build ok\nid 550e8400-e29b-41d4-a716-446655440000\ndense prose here";
        let (stable, vol) = align(text);
        assert_eq!(stable, "dense prose here");
        assert!(vol.contains("2026-09-23"));
        assert!(vol.contains("550e8400"));
    }

    #[test]
    fn prose_untouched() {
        let (stable, vol) = align("hello world, dense prose here");
        assert_eq!(stable, "hello world, dense prose here");
        assert!(vol.is_empty());
    }
}
