//! Bet 6 — one-key worktree checkpoint (Phase 3: live).
//! Snapshot = HEAD sha + dirty flag. Revert restores tracked files and
//! drops untracked ones; the rollback itself is an audit event upstream.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Checkpoint {
    // Phase 5 Revert button reads identity + path; snapshot() uses session.
    #[allow(dead_code)]
    pub id: Uuid,
    #[allow(dead_code)]
    pub session: String,
    #[allow(dead_code)]
    pub path: String,
    #[allow(dead_code)]
    pub taken_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub sha: String,
    // Reported at snapshot time; the Revert button confirms it on click.
    #[allow(dead_code)]
    pub dirty: bool,
}

fn git(workdir: &str, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(args)
        .output()
        .context("git spawn failed")?;
    anyhow::ensure!(out.status.success(), "git {} failed", args.join(" "));
    Ok(String::from_utf8_lossy(&out.stdout).trim().into())
}

impl Checkpoint {
    pub fn new(session: &str, data_dir: &str) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            session: session.into(),
            path: format!("{data_dir}/checkpoints/{session}/{id}"),
            taken_at: Utc::now(),
        }
    }

    /// Record HEAD + dirtiness before the first write tool.
    pub fn snapshot(&self, workdir: &str) -> Result<Snapshot> {
        Ok(Snapshot {
            sha: git(workdir, &["rev-parse", "HEAD"])?,
            dirty: !git(workdir, &["status", "--porcelain"])?.is_empty(),
        })
    }

    /// Restore tracked files, drop untracked. Returns the restored sha.
    /// Wired to the Phase 5 Revert button (daemon side is done + tested
    /// through snapshot(); revert() is exercised in 3b tests).
    #[allow(dead_code)]
    pub fn revert(&self, workdir: &str) -> Result<String> {
        git(workdir, &["checkout", "--", "."])?;
        git(workdir, &["clean", "-fd"])?;
        git(workdir, &["rev-parse", "HEAD"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_nests_session() {
        let c = Checkpoint::new("s1", "/tmp/h");
        assert!(c.path.contains("s1"));
    }

    #[test]
    fn snapshot_and_revert() {
        let dir = std::env::temp_dir().join("harness-checkpoint-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = dir.to_str().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t"],
            vec!["config", "user.name", "t"],
        ] {
            assert!(std::process::Command::new("git")
                .arg("-C")
                .arg(r)
                .args(&args)
                .output()
                .unwrap()
                .status
                .success());
        }
        std::fs::write(dir.join("f.txt"), "v1").unwrap();
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(r)
            .args(["add", "."])
            .output()
            .unwrap()
            .status
            .success());
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(r)
            .args(["commit", "-m", "init"])
            .output()
            .unwrap()
            .status
            .success());
        let c = Checkpoint::new("s1", "/tmp/h");
        let snap = c.snapshot(r).unwrap();
        assert!(!snap.sha.is_empty() && !snap.dirty);
        std::fs::write(dir.join("f.txt"), "v2").unwrap();
        assert!(c.snapshot(r).unwrap().dirty);
        c.revert(r).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), "v1");
    }
}
