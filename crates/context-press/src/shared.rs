//! Bet 2 — content-addressed shared context (stub; full build Phase 2).
//! One hash per file+commit: workers on the same commit share Press output.
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Stable hex key for `(commit, path, bytes)`.
pub fn content_key(commit: &str, path: &str, bytes: &[u8]) -> String {
    let mut h = DefaultHasher::new();
    commit.hash(&mut h);
    path.hash(&mut h);
    bytes.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[derive(Debug, Default)]
pub struct SharedCache {
    inner: HashMap<String, String>,
}

impl SharedCache {
    pub fn put(&mut self, key: String, pressed: String) {
        self.inner.insert(key, pressed);
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_by_key() {
        let mut c = SharedCache::default();
        let k = content_key("abc", "a.rs", b"fn main(){}");
        c.put(k.clone(), "pressed".into());
        assert_eq!(c.get(&k), Some("pressed"));
    }
}
