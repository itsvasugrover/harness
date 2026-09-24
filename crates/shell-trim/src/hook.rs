//! Hook auto-rewrite: shell command -> trim equivalent.
//! Pure mapping with an exclude list; the per-agent hook installers
//! (Phase 3) call this. Never rewrites what it does not understand.
const RULES: [(&str, &str); 6] = [
    ("git status", "git-status"),
    ("git diff", "git-diff"),
    ("git log", "git-log"),
    ("cargo test", "test"),
    ("npm test", "test"),
    ("pytest", "test"),
];

/// Rewrite `cmd` to `trim <filter>`, or return it unchanged.
/// Excludes match command prefixes (e.g. `curl` stays raw).
pub fn rewrite(cmd: &str, exclude: &[&str]) -> String {
    let cmd = cmd.trim();
    if exclude.iter().any(|x| cmd.starts_with(x)) {
        return cmd.to_string();
    }
    for (prefix, filter) in RULES {
        if cmd == prefix || cmd.starts_with(&format!("{prefix} ")) {
            return format!("trim {filter}");
        }
    }
    cmd.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_known_passthrough_unknown() {
        assert_eq!(rewrite("git status", &[]), "trim git-status");
        assert_eq!(rewrite("git status --short", &[]), "trim git-status");
        assert_eq!(rewrite("curl example.com", &[]), "curl example.com");
        assert_eq!(rewrite("git status", &["git"]), "git status");
    }
}
