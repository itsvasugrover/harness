//! write: create-only file write. Snapshot-first arrives with Bet 6.
use super::super::jail;
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Context, Result};

pub struct Write;

impl Tool for Write {
    fn name(&self) -> &'static str {
        "write"
    }
    fn description(&self) -> &'static str {
        "Create a new file. Input: 'path: <p>' line then content. No overwrite."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let (first, body) = input
            .split_once('\n')
            .context("input needs 'path: <p>' line + body")?;
        let rel = first
            .strip_prefix("path:")
            .context("first line must be 'path: <p>'")?
            .trim();
        let path = jail::resolve(&ctx.workdir, rel)?;
        if path.exists() {
            bail!("refusing to overwrite {}", path.display());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body)?;
        Ok(ToolOutput {
            title: format!("wrote {}", path.display()),
            output: "ok".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_refuses_overwrite() {
        let dir = std::env::temp_dir().join("harness-tool-write");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        assert!(Write.run(&ctx, "path: n.txt\nbody").is_ok());
        assert!(Write.run(&ctx, "path: n.txt\nbody").is_err());
    }
}
