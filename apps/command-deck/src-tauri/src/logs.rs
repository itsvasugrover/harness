//! Log tail: last N lines of the daemon log for the UI viewer.
//! Reads from the end in 8 KiB chunks; never loads the whole file.
use anyhow::Result;

const CHUNK: usize = 8192;

/// Last `n` lines of `<data_dir>/logs/harnessd.log` (empty when absent).
pub fn tail(data_dir: &str, n: usize) -> Result<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};
    let path = format!("{}/logs/harnessd.log", data_dir.trim_end_matches('/'));
    let Ok(mut file) = std::fs::File::open(&path) else {
        return Ok(vec![]);
    };
    let size = file.metadata()?.len();
    let mut pos = size;
    let mut buf = Vec::new();
    let mut lines = 0usize;
    while pos > 0 && lines <= n {
        let take = (pos.min(CHUNK as u64)) as usize;
        pos -= take as u64;
        file.seek(SeekFrom::Start(pos))?;
        let mut chunk = vec![0u8; take];
        file.read_exact(&mut chunk)?;
        lines += chunk.iter().filter(|&&b| b == b'\n').count();
        buf.splice(0..0, chunk);
    }
    let text = String::from_utf8_lossy(&buf);
    let mut all: Vec<String> = text.lines().map(String::from).collect();
    if all.len() > n {
        all.drain(0..all.len() - n);
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tails_last_lines_and_empty_when_absent() {
        let dir = std::env::temp_dir().join(format!("hx-logs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.to_string_lossy().into_owned();
        assert!(tail(&root, 10).unwrap().is_empty());
        std::fs::create_dir_all(format!("{root}/logs")).unwrap();
        std::fs::write(
            format!("{root}/logs/harnessd.log"),
            "one\ntwo\nthree\nfour\n",
        )
        .unwrap();
        assert_eq!(tail(&root, 2).unwrap(), vec!["three", "four"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
