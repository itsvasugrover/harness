//! bash: jailed `sh -c` with timeout + caps. Trim routing: Phase 2.
use super::super::jail;
use super::super::tool::{truncate, Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};
use std::process::Command;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

pub struct Bash;

impl Tool for Bash {
    fn name(&self) -> &'static str {
        "bash"
    }
    fn description(&self) -> &'static str {
        "Run a shell command (cwd = workdir, 30s timeout). Output capped."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let cwd = jail::resolve(&ctx.workdir, ".")?;
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(input.trim())
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        // Poll so a timeout KILLS the child instead of orphaning it.
        let start = Instant::now();
        let status = loop {
            match child.try_wait()? {
                Some(s) => break s,
                None if start.elapsed() >= TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!("command timed out after 30s (killed)");
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        };
        let mut text = String::new();
        if let Some(mut out) = child.stdout.take() {
            use std::io::Read as _;
            let _ = out.read_to_string(&mut text);
        }
        if let Some(mut err) = child.stderr.take() {
            use std::io::Read as _;
            let mut err_text = String::new();
            let _ = err.read_to_string(&mut err_text);
            text.push_str(&err_text);
        }
        Ok(ToolOutput {
            title: format!("bash exit={}", status.code().unwrap_or(-1)),
            output: truncate(text.trim()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_echo() {
        let dir = std::env::temp_dir().join("harness-tool-bash");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        let out = Bash.run(&ctx, "echo hi").unwrap();
        assert!(out.output.contains("hi"));
    }
}
