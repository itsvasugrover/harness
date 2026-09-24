//! Recall store (reversible compression): originals persist as files
//! under a dir with a TTL; `recall <id>` restores them. File-backed
//! (no DB) so crushers can be aggressive without risk.
use anyhow::Result;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// Mint an id from time + size. Content-addressed dedup is Phase 4.
pub fn new_id(text: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h = DefaultHasher::new();
    nanos.hash(&mut h);
    text.len().hash(&mut h);
    text.bytes().take(64).collect::<Vec<_>>().hash(&mut h);
    format!("{:016x}", h.finish())
}

fn path_for(dir: &str, id: &str) -> std::path::PathBuf {
    std::path::Path::new(dir).join(format!("{id}.txt"))
}

/// Save the original; returns its recall id.
pub fn save(dir: &str, text: &str) -> Result<String> {
    std::fs::create_dir_all(dir)?;
    let id = new_id(text);
    std::fs::write(path_for(dir, &id), text)?;
    Ok(id)
}

/// Restore an original, or `None` when missing/expired.
pub fn recall(dir: &str, id: &str, ttl_secs: u64) -> Option<String> {
    let path = path_for(dir, id);
    let meta = std::fs::metadata(&path).ok()?;
    let age = SystemTime::now()
        .duration_since(meta.modified().ok()?)
        .ok()?
        .as_secs();
    if age > ttl_secs {
        return None;
    }
    std::fs::read_to_string(&path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_missing() {
        let dir = std::env::temp_dir().join("harness-recall-test");
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.to_str().unwrap();
        let id = save(d, "original text").unwrap();
        assert_eq!(recall(d, &id, 60).unwrap(), "original text");
        assert!(recall(d, "missing", 60).is_none());
    }
}
