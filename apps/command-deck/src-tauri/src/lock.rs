//! Single-instance lock: one Command Deck owns the daemon per data dir.
//! Claim is atomic (`create_new`); a live loopback port means another
//! owner is up, so we attach instead of spawning a second daemon.
use anyhow::{Context, Result};
use std::io::Write;

/// Claim `<data_dir>/sidecar.lock`, or attach when the port answers.
pub fn claim(data_dir: &str, bind: &str) -> Result<Owner> {
    let path = lock_path(data_dir);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            writeln!(file, "{}", std::process::id())?;
            Ok(Owner::Ours { path })
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if port_answers(bind) {
                Ok(Owner::Attached { path })
            } else {
                // Stale lock: previous owner died without cleanup.
                std::fs::remove_file(&path)?;
                claim(data_dir, bind)
            }
        }
        Err(e) => Err(e).with_context(|| format!("lock {path}")),
    }
}

/// Loopback liveness probe: what we serve, nothing more.
fn port_answers(bind: &str) -> bool {
    std::net::TcpStream::connect(bind).is_ok()
}

fn lock_path(data_dir: &str) -> String {
    format!("{}/sidecar.lock", data_dir.trim_end_matches('/'))
}

/// Our relationship to a running daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// We hold the lock; stop the daemon on exit.
    Ours { path: String },
    /// Another owner serves; never stop what we didn't start.
    Attached { path: String },
}

impl Owner {
    /// Release our claim. Attached owners release nothing.
    pub fn release(self) -> Result<()> {
        if let Owner::Ours { path } = self {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_attach_and_release() {
        let dir = std::env::temp_dir().join(format!("hx-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.to_string_lossy().into_owned();
        // Closed port + no lock: we own it.
        let owner = claim(&root, "127.0.0.1:9").unwrap();
        assert!(matches!(owner, Owner::Ours { .. }));
        // Second claim sees a dead port (discard) so it takes over.
        let owner2 = claim(&root, "127.0.0.1:9").unwrap();
        assert!(matches!(owner2, Owner::Ours { .. }));
        owner2.release().unwrap();
        assert!(!std::path::Path::new(&lock_path(&root)).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn attaches_when_port_answers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let dir = std::env::temp_dir().join(format!("hx-lock-a{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.to_string_lossy().into_owned();
        // Pre-seed a lock as if another owner held it, then claim: the
        // live listener proves someone serves, so we attach.
        std::fs::write(lock_path(&root), "1").unwrap();
        let _listener = listener;
        let owner = claim(&root, &format!("127.0.0.1:{port}")).unwrap();
        assert!(matches!(owner, Owner::Attached { .. }));
        owner.release().unwrap();
        // Attached release must not delete the lock.
        assert!(std::path::Path::new(&lock_path(&root)).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
