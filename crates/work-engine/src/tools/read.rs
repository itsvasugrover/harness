//! read: workdir-jailed file read with output caps.
use super::super::jail;
use super::super::tool::{truncate, Tool, ToolCtx, ToolOutput};
use anyhow::Result;

pub struct Read;

impl Tool for Read {
    fn name(&self) -> &'static str {
        "read"
    }
    fn description(&self) -> &'static str {
        "Read a file inside the workdir. Input: path."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let path = jail::resolve(&ctx.workdir, input.trim())?;
        let size = std::fs::metadata(&path)?.len();
        if size > 1_048_576 {
            anyhow::bail!("{size} bytes: too large to read whole; use grep/glob first");
        }
        let bytes = std::fs::read(&path)?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        Ok(ToolOutput {
            title: format!("read {}", path.display()),
            output: truncate(&text),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_blocks_escape() {
        let dir = std::env::temp_dir().join("harness-tool-read");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hello").unwrap();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        assert_eq!(Read.run(&ctx, "a.txt").unwrap().output, "hello");
        assert!(Read.run(&ctx, "../../x").is_err());
    }
}
