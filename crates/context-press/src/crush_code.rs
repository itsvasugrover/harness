//! Code crusher: signatures-first, bodies on demand via recall.
//! Line-heuristic (no AST vendored yet): definition lines keep their
//! line numbers, everything else collapses to counts per block.
const SIG_HINTS: [&str; 10] = [
    "fn ",
    "def ",
    "class ",
    "struct ",
    "impl ",
    "interface ",
    "import ",
    "use ",
    "pub ",
    "async ",
];

fn is_sig(line: &str) -> bool {
    let t = line.trim_start();
    SIG_HINTS.iter().any(|h| t.starts_with(h)) || t.starts_with("//!") || t.starts_with("///")
}

/// Crush source to `lineno: signature` lines + collapsed-body counts.
/// Small files (<= 40 lines) pass through untouched.
pub fn crush_code(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 40 {
        return text.to_string();
    }
    let mut out = Vec::new();
    let mut hidden = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if is_sig(line) {
            if hidden > 0 {
                out.push(format!("  ... {hidden} body lines collapsed ..."));
                hidden = 0;
            }
            out.push(format!("{}: {}", i + 1, line.trim()));
        } else {
            hidden += 1;
        }
    }
    if hidden > 0 {
        out.push(format!("  ... {hidden} body lines collapsed ..."));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_signatures_collapses_bodies() {
        let mut src = String::new();
        for i in 0..30 {
            src.push_str(&format!("fn f{i}() {{\n"));
            for j in 0..8 {
                src.push_str(&format!(
                    "    let value_{j} = compute_something_long({i}, {j});\n"
                ));
            }
            src.push_str("}\n");
        }
        let out = crush_code(&src);
        assert!(out.contains("fn f0()"));
        assert!(out.contains("collapsed"));
        assert!(!out.contains("compute_something_long"));
        assert!(out.len() < src.len() / 2);
    }

    #[test]
    fn small_files_untouched() {
        assert_eq!(crush_code("fn a() {}"), "fn a() {}");
    }
}
