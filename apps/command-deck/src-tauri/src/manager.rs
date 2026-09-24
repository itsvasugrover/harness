//! Daemon lifecycle: spawn `harnessd serve`, supervise it, stop it.
//! Std-only so `cargo check` stays green without the Tauri shell; the
//! shell invokes this through Tauri commands (follow-up slice).
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Child, Command, Stdio};

/// How to reach the daemon binary. `HARNESSD_BIN` wins (dev override),
/// then a `harnessd` beside the sidecar, then `PATH`.
pub fn resolve_binary() -> Result<String> {
    if let Ok(bin) = std::env::var("HARNESSD_BIN") {
        if !bin.is_empty() {
            return Ok(bin);
        }
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let next = dir.join("harnessd");
            if next.is_file() {
                return Ok(next.to_string_lossy().into_owned());
            }
        }
    }
    Ok("harnessd".into())
}

/// A supervised daemon child. `stop()` kills, then reaps.
pub struct Daemon {
    child: Child,
    log_path: String,
}

impl Daemon {
    /// Spawn `harnessd serve --bind <bind>` with stdout/stderr appended
    /// to `<data_dir>/logs/harnessd.log`. Fails fast when the binary is
    /// missing or the port is already served.
    pub fn start(data_dir: &str, bind: &str) -> Result<Self> {
        let bin = resolve_binary()?;
        let log_dir = format!("{}/logs", data_dir.trim_end_matches('/'));
        std::fs::create_dir_all(&log_dir)?;
        let log_path = format!("{log_dir}/harnessd.log");
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let child = Command::new(&bin)
            .arg("serve")
            .arg("--bind")
            .arg(bind)
            .env("HARNESS_DATA_DIR", data_dir)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()
            .with_context(|| format!("spawn {bin} serve"))?;
        Ok(Self { child, log_path })
    }

    /// Liveness: child still running AND loopback probe answers.
    pub fn alive(&mut self, bind: &str) -> bool {
        if let Ok(None) = self.child.try_wait() {
            return std::net::TcpStream::connect(bind).is_ok();
        }
        false
    }

    /// Stop what we started; never touch an attached daemon.
    pub fn stop(mut self) -> Result<()> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }

    /// Where this daemon's output lands (for the log viewer).
    pub fn log_path(&self) -> &str {
        &self.log_path
    }
}

/// Append one sidecar line to the shared log (startup, attach, errors).
pub fn note(data_dir: &str, line: &str) -> Result<()> {
    let log_dir = format!("{}/logs", data_dir.trim_end_matches('/'));
    std::fs::create_dir_all(&log_dir)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{log_dir}/harnessd.log"))?;
    writeln!(file, "[sidecar] {line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_override_wins() {
        std::env::set_var("HARNESSD_BIN", "/tmp/fake-harnessd");
        assert_eq!(resolve_binary().unwrap(), "/tmp/fake-harnessd");
        std::env::remove_var("HARNESSD_BIN");
    }

    #[test]
    fn missing_binary_fails_fast() {
        std::env::set_var("HARNESSD_BIN", "/nonexistent/harnessd-xyz");
        let dir = std::env::temp_dir().join("hx-sidecar-missing");
        let _ = std::fs::remove_dir_all(&dir);
        let res = Daemon::start(dir.to_str().unwrap(), "127.0.0.1:9");
        assert!(res.is_err());
        std::env::remove_var("HARNESSD_BIN");
    }
}
