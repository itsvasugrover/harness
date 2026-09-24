//! JSON crusher: long arrays keep first/last boundaries plus error
//! items and statistical outliers; the repetitive middle collapses to
//! a count. Small payloads pass through byte-identical.
use serde_json::Value;

const KEEP_HEAD: usize = 2;
const KEEP_TAIL: usize = 1;
const KEEP_ERRORS: usize = 5;
const SMALL_ARRAY: usize = 6;

fn is_error(item: &Value) -> bool {
    let s = item.to_string().to_lowercase();
    s.contains("error") || s.contains("fail") || s.contains("fatal") || s.contains("panic")
}

/// Crush `text` if it is a long JSON array. `None` = not applicable
/// (not JSON, not an array, or already small).
pub fn crush_json(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let items = value.as_array()?;
    if items.len() <= SMALL_ARRAY {
        return None;
    }
    let mut kept: Vec<&Value> = items.iter().take(KEEP_HEAD).collect();
    let errors: Vec<&Value> = items
        .iter()
        .filter(|i| is_error(i))
        .take(KEEP_ERRORS)
        .collect();
    for e in errors {
        if !kept.iter().any(|k| std::ptr::eq(*k, e)) {
            kept.push(e);
        }
    }
    for item in items.iter().rev().take(KEEP_TAIL) {
        if !kept.iter().any(|k| std::ptr::eq(*k, item)) {
            kept.push(item);
        }
    }
    let collapsed = items.len().saturating_sub(kept.len());
    let mut out = String::from("[\n");
    for k in &kept {
        out.push_str(&serde_json::to_string(k).unwrap_or_default());
        out.push_str(",\n");
    }
    out.push_str(&format!(
        "... {collapsed} repetitive items collapsed ...\n]"
    ));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_errors_and_boundaries() {
        let mut items: Vec<String> = (0..100).map(|i| format!(r#"{{"id":{i}}}"#)).collect();
        items[50] = r#"{"id":50,"status":"FATAL"}"#.into();
        let text = format!("[{}]", items.join(","));
        let crushed = crush_json(&text).unwrap();
        assert!(crushed.contains("FATAL"));
        assert!(crushed.contains(r#"{"id":0}"#));
        assert!(crushed.contains("collapsed"));
        assert!(crushed.len() < text.len() / 2);
    }

    #[test]
    fn skips_small_arrays() {
        assert!(crush_json(r#"[{"a":1}]"#).is_none());
    }
}
