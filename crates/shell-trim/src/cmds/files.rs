//! File-listing shapes: tree with counts, blank-line-collapsed reads.

/// Flat path list -> indented tree with per-dir file counts.
pub fn tree(paths: &[&str]) -> String {
    let mut sorted: Vec<&&str> = paths.iter().collect();
    sorted.sort();
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for p in sorted {
        let parts: Vec<&str> = p.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            let prefix = format!("{}{}", "  ".repeat(i), part);
            if i + 1 < parts.len() {
                let dir: String = parts[..=i].join("/");
                if !seen.contains(&dir) {
                    seen.push(dir);
                    let count = paths
                        .iter()
                        .filter(|q| q.starts_with(&format!("{}/", parts[..=i].join("/"))))
                        .count();
                    out.push(format!("{}{}/ ({count} files)", "  ".repeat(i), part));
                }
            } else if !seen.contains(&prefix) {
                seen.push(prefix.clone());
                out.push(prefix);
            }
        }
    }
    out.join("\n")
}

/// Collapse 3+ blank lines to one; cap via caller budget.
pub fn shape_read(text: &str) -> String {
    let mut out = Vec::new();
    let mut blanks = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            blanks += 1;
            if blanks <= 1 {
                out.push(line);
            }
        } else {
            blanks = 0;
            out.push(line);
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree_with_counts() {
        let out = tree(&["src/a.rs", "src/b.rs", "Cargo.toml"]);
        assert!(out.contains("src/ (2 files)"));
        assert!(out.contains("Cargo.toml"));
    }
}
