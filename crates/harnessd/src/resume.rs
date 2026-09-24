//! Crash-safe resume: on boot, walk every stored session and check
//! whether its worktree still exists. The daemon owns all durable
//! facts — resume is a startup scan, not reconstruction.
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ResumeItem {
    pub session_id: String,
    pub agent: String,
    pub workdir_exists: bool,
}

/// Sessions whose worktree dir is present (re-attachable) vs gone.
/// Workdir convention: `<wt_root>/<worker-id>`; the session id doubles
/// as the worker id for daemon-spawned runs.
pub async fn plan(store: &work_engine::store::Store, wt_root: &str) -> Result<Vec<ResumeItem>> {
    let mut items = Vec::new();
    for s in store.all_sessions().await? {
        let workdir = format!("{wt_root}/{}", s.id);
        items.push(ResumeItem {
            session_id: s.id,
            agent: s.agent,
            workdir_exists: std::path::Path::new(&workdir).is_dir(),
        });
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn flags_missing_worktrees() {
        let store = work_engine::store::Store::open("sqlite::memory:")
            .await
            .unwrap();
        store
            .create_session("rs-resume", "build", "demo/m")
            .await
            .unwrap();
        let dir = std::env::temp_dir().join("harness-resume-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("other")).unwrap();
        let items = plan(&store, dir.to_str().unwrap()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].workdir_exists);
    }
}
