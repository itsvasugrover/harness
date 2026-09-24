//! Per-invocation savings meter + JSONL gain ledger. Bytes/4 is an
//! estimate — labeled as such everywhere; bills come from
//! provider-reported usage.
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct Stats {
    pub bytes_in: u64,
    pub bytes_out: u64,
}

impl Stats {
    pub fn ratio(&self) -> f64 {
        if self.bytes_in == 0 {
            return 0.0;
        }
        1.0 - (self.bytes_out as f64 / self.bytes_in as f64)
    }

    pub fn tokens_saved_estimate(&self) -> u64 {
        self.bytes_in.saturating_sub(self.bytes_out) / 4
    }

    pub fn report(&self) -> String {
        format!(
            "trim: {} -> {} bytes ({:.0}% smaller, ~{} tokens saved*) *estimate",
            self.bytes_in,
            self.bytes_out,
            self.ratio() * 100.0,
            self.tokens_saved_estimate()
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub filter: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// Command text for rewrite entries; `rewritten:<filter>` or
    /// `passthrough` verdict for discover ranking.
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Default)]
pub struct Gain {
    pub invocations: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

/// Append one invocation to the JSONL ledger (created on first write).
pub fn append_ledger(path: &str, entry: &Entry) -> Result<()> {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

/// Summarize a ledger file; missing file = empty gain (not an error).
pub fn summarize(path: &str) -> Gain {
    let mut gain = Gain::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return gain;
    };
    for line in text.lines() {
        if let Ok(e) = serde_json::from_str::<Entry>(line) {
            gain.invocations += 1;
            gain.bytes_in += e.bytes_in;
            gain.bytes_out += e.bytes_out;
        }
    }
    gain
}

/// Rank unrewritten commands: what the hook saw but no filter covers.
/// Commands group by their two-word head (`kubectl logs ...` counts
/// together). Drives which filter to write next — data, not guessing.
pub fn discover(path: &str, top: usize) -> Vec<(String, u64)> {
    use std::collections::HashMap;
    fn head(cmd: &str) -> String {
        cmd.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
    }
    let mut counts: HashMap<String, u64> = HashMap::new();
    if let Ok(text) = std::fs::read_to_string(path) {
        for line in text.lines() {
            if let Ok(e) = serde_json::from_str::<Entry>(line) {
                if e.note == "passthrough" && !e.filter.is_empty() {
                    *counts.entry(head(&e.filter)).or_default() += 1;
                }
            }
        }
    }
    let mut ranked: Vec<(String, u64)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(top.max(1));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_passthrough_heads() {
        let dir = std::env::temp_dir().join("harness-trim-discover");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log.jsonl");
        let p = path.to_str().unwrap();
        for cmd in [
            "kubectl logs pod-a",
            "kubectl logs pod-b",
            "kubectl logs pod-c",
        ] {
            append_ledger(
                p,
                &Entry {
                    filter: cmd.into(),
                    bytes_in: 1,
                    bytes_out: 1,
                    note: "passthrough".into(),
                },
            )
            .unwrap();
        }
        append_ledger(
            p,
            &Entry {
                filter: "git status".into(),
                bytes_in: 1,
                bytes_out: 1,
                note: "rewritten:git-status".into(),
            },
        )
        .unwrap();
        let ranked = discover(p, 5);
        assert_eq!(ranked, vec![("kubectl logs".to_string(), 3)]);
    }
}
