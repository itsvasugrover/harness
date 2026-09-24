//! edit: exact-match replace, exactly one match or fail.
use super::super::jail;
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Context, Result};

pub struct Edit;

impl Tool for Edit {
    fn name(&self) -> &'static str {
        "edit"
    }
    fn description(&self) -> &'static str {
        "Replace OLD with NEW. Input: 'path: <p>' then ---OLD---/---NEW--- blocks."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let (head, rest) = input.split_once("---OLD---").context("missing ---OLD---")?;
        let (old, new) = rest.split_once("---NEW---").context("missing ---NEW---")?;
        let rel = head
            .strip_prefix("path:")
            .context("first line must be 'path: <p>'")?
            .trim();
        let path = jail::resolve(&ctx.workdir, rel)?;
        let text = std::fs::read_to_string(&path)?;
        let old = old.trim_matches('\n');
        if text.matches(old).count() != 1 {
            bail!("old block must match exactly once in {}", path.display());
        }
        std::fs::write(&path, text.replacen(old, new.trim_matches('\n'), 1))?;
        Ok(ToolOutput {
            title: format!("edited {}", path.display()),
            output: "ok".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_once() {
        let dir = std::env::temp_dir().join("harness-tool-edit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "foo bar").unwrap();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        Edit.run(&ctx, "path: a.txt\n---OLD---\nbar\n---NEW---\nbaz\n")
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "foo baz"
        );
    }
}
