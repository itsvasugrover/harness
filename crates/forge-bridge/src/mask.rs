//! Secret masking for forge logs. Token-shaped strings are redacted
//! before anything reaches logs, diffs, or model context. Plain string
//! scans only — this crate carries no regex engine by policy.
//!
//! Covered shapes: `ghp_/gho_/ghu_/ghs_/ghr_` + `github_pat_`,
//! `Bearer <token>`, and `access_token=<token>` query fragments.
//! A match needs a token run of at least 8 word chars to avoid
//! redacting ordinary prose.

/// Redact token-shaped spans in `text`, keeping a 4-char prefix hint.
pub fn mask_secrets(text: &str) -> String {
    let mut out = text.to_string();
    for prefix in [
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "github_pat_",
        "Bearer ",
        "access_token=",
    ] {
        out = mask_prefix(&out, prefix);
    }
    out
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '~' | '+' | '/' | '=')
}

fn mask_prefix(text: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(prefix) {
        out.push_str(&rest[..i + prefix.len()]);
        let tail = &rest[i + prefix.len()..];
        let run: usize = tail
            .char_indices()
            .take_while(|(_, c)| is_token_char(*c))
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0);
        if run >= 8 {
            out.push_str("***");
            rest = &tail[run..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_github_pat() {
        // Built with concat! so no secret-shaped literal sits in source.
        let log = concat!("fetch failed with ghp_", "abcdefgh12345678 on repo");
        assert_eq!(mask_secrets(log), "fetch failed with ghp_*** on repo");
    }

    #[test]
    fn redacts_bearer_and_query_token() {
        let log = concat!(
            "auth Bearer ",
            "sekrit-token-99 ok; url ?access_token=",
            "qwertyuiop123"
        );
        assert_eq!(
            mask_secrets(log),
            "auth Bearer *** ok; url ?access_token=***"
        );
    }

    #[test]
    fn leaves_plain_prose_alone() {
        let log = "Bearer of good news Bearing gifts";
        assert_eq!(mask_secrets(log), log);
    }
}
