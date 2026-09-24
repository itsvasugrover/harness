//! Git output filters: compact status, condensed diff, one-line log.

/// `git status --porcelain` grouped by state code.
pub fn compact_status(text: &str) -> String {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        groups
            .entry(line[..2].trim())
            .or_default()
            .push(line[3..].trim());
    }
    groups
        .into_iter()
        .map(|(code, files)| format!("{code} ({}): {}", files.len(), files.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Diff: keep file headers + change lines, drop index/blob noise.
pub fn condense_diff(text: &str) -> String {
    text.lines()
        .filter(|l| {
            !(l.starts_with("index ")
                || l.starts_with("new file mode")
                || l.starts_with("old mode"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse multi-line log blocks to hash + subject lines.
pub fn oneline_log(text: &str) -> String {
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("Author:") && !t.starts_with("Date:")
        })
        .map(|l| {
            let t = l.trim();
            if t.starts_with("commit ") {
                format!("\n{t}")
            } else {
                t.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_status() {
        let out = compact_status(" M a.rs\n M b.rs\n?? c.rs\n");
        assert!(out.contains("M (2): a.rs, b.rs"));
        assert!(out.contains("?? (1): c.rs"));
    }

    #[test]
    fn strips_diff_noise() {
        let out = condense_diff("diff --git a/a b/a\nindex 123..456\n+new\n");
        assert!(!out.contains("index "));
        assert!(out.contains("+new"));
    }
}
