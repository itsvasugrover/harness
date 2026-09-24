//! Content router: JSON arrays -> structure crusher, source code ->
//! syntax crusher (Phase 2b), prose/logs -> text crusher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Json,
    Code,
    Text,
}

const CODE_HINTS: [&str; 6] = ["fn ", "def ", "class ", "import ", "#include", "package "];

/// Classify a block. JSON only when it parses; code on strong hints;
// prose and logs fall through to text.
pub fn classify(text: &str) -> BlockKind {
    let t = text.trim_start();
    if (t.starts_with('{') || t.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(text).is_ok()
    {
        return BlockKind::Json;
    }
    if CODE_HINTS.iter().any(|h| text.contains(h)) {
        return BlockKind::Code;
    }
    BlockKind::Text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_each_kind() {
        assert_eq!(classify(r#"[{"a":1}]"#), BlockKind::Json);
        assert_eq!(classify("fn main() {}"), BlockKind::Code);
        assert_eq!(classify("just a log line"), BlockKind::Text);
        assert_eq!(classify("{not json"), BlockKind::Text);
    }
}
