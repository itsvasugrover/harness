//! Worker isolation: one task = one branch + one linked worktree.
//! Dirty worktrees are never force-deleted; removal refuses when dirty.
use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct Worker {
    pub id: String,
    pub branch: String,
    pub workdir: String,
}

fn git(repo: &str, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("git spawn failed")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().into())
}

fn slug(task: &str) -> String {
    task.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-")
}

/// Create branch `worker/<id>-<slug>` + linked worktree under `wt_root`.
pub fn spawn(repo: &str, wt_root: &str, id: &str, task: &str, base: &str) -> Result<Worker> {
    let branch = format!("worker/{id}-{}", slug(task));
    let workdir = format!("{wt_root}/{id}");
    git(repo, &["branch", &branch, base])?;
    git(repo, &["worktree", "add", &workdir, &branch])?;
    Ok(Worker {
        id: id.into(),
        branch,
        workdir,
    })
}

/// True when the worktree has uncommitted changes.
pub fn is_dirty(workdir: &str) -> Result<bool> {
    Ok(!git(workdir, &["status", "--porcelain"])?.is_empty())
}

/// Remove the worktree + branch. Refuses when dirty unless `force`
/// and even then never force-removes — caller must clean first.
pub fn archive(repo: &str, worker: &Worker) -> Result<()> {
    if is_dirty(&worker.workdir)? {
        bail!("refusing to archive dirty worktree {}", worker.workdir);
    }
    git(repo, &["worktree", "remove", &worker.workdir])?;
    git(repo, &["branch", "-D", &worker.branch])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("harness-worker-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = dir.to_str().unwrap().to_string();
        git(&r, &["init", "-b", "main"]).unwrap();
        git(&r, &["config", "user.email", "t@t"]).unwrap();
        git(&r, &["config", "user.name", "t"]).unwrap();
        std::fs::write(dir.join("f.txt"), "x").unwrap();
        git(&r, &["add", "."]).unwrap();
        git(&r, &["commit", "-m", "init"]).unwrap();
        r
    }

    #[test]
    fn spawn_and_archive() {
        let r = repo("spawn");
        let wt = std::env::temp_dir().join("harness-wt-spawn");
        let _ = std::fs::remove_dir_all(&wt);
        let w = spawn(&r, wt.to_str().unwrap(), "w1", "Fix login bug", "main").unwrap();
        assert!(w.branch.starts_with("worker/w1-fix-login-bug"));
        assert!(std::path::Path::new(&w.workdir).exists());
        archive(&r, &w).unwrap();
    }

    #[test]
    fn dirty_blocks_archive() {
        let r = repo("dirty");
        let wt = std::env::temp_dir().join("harness-wt-dirty2");
        let _ = std::fs::remove_dir_all(&wt);
        let w = spawn(&r, wt.to_str().unwrap(), "w2", "task", "main").unwrap();
        std::fs::write(format!("{}/new.txt", w.workdir), "dirty").unwrap();
        assert!(archive(&r, &w).is_err());
        std::fs::remove_file(format!("{}/new.txt", w.workdir)).unwrap();
        let _ = git(&r, &["worktree", "remove", "--force", &w.workdir]);
    }
}
