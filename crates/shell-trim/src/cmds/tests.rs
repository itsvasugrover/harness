//! Test-runner filter: failures + summary survive, passing
//! test chatter collapses to a count. Generic across runners.
const KEEP: [&str; 6] = ["fail", "error", "panic", "assert", "FAILED", "Traceback"];

/// Keep signal lines; replace passing-test lines with one count line.
pub fn failures_only(text: &str) -> String {
    let mut kept = Vec::new();
    let mut passed = 0usize;
    for line in text.lines() {
        let low = line.to_lowercase();
        if KEEP.iter().any(|k| low.contains(&k.to_lowercase())) {
            kept.push(line.to_string());
        } else if looks_passing(&low) {
            passed += 1;
        } else {
            kept.push(line.to_string());
        }
    }
    if passed > 0 {
        kept.push(format!("... {passed} passing lines collapsed ..."));
    }
    kept.join("\n")
}

fn looks_passing(low: &str) -> bool {
    (low.contains("ok") || low.contains("pass")) && !(low.contains("fail") || low.contains("error"))
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn keeps_failures_collapses_passes() {
        let text = "test a ... ok\ntest b ... ok\ntest c ... FAILED\nassertion failed";
        let out = failures_only(text);
        assert!(out.contains("FAILED"));
        assert!(out.contains("assertion failed"));
        assert!(out.contains("2 passing lines collapsed"));
        assert!(!out.contains("test a"));
    }
}
