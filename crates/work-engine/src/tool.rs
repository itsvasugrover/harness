//! Tool trait: every tool is name + description + schema + execute.
//! Invalid args return machine-readable errors for model self-correction.
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ToolCtx {
    pub session_id: String,
    /// Tools are jailed to this dir; writes outside fail.
    pub workdir: String,
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub title: String,
    pub output: String,
}

/// Implemented by every tool. `Send + Sync` so units run concurrently.
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput>;
}

/// Tool-output budget (mirrors opencode `tool_output` defaults).
pub const MAX_LINES: usize = 2000;
pub const MAX_BYTES: usize = 51200;

/// Cap output; full text is recallable, never silently dropped.
pub fn truncate(text: &str) -> String {
    let lines: Vec<&str> = text.lines().take(MAX_LINES + 1).collect();
    let line_capped = lines.len() > MAX_LINES;
    let mut out = lines[..lines.len().min(MAX_LINES)].join("\n");
    if out.len() > MAX_BYTES {
        let mut end = MAX_BYTES;
        while !out.is_char_boundary(end) {
            end -= 1;
        }
        out.truncate(end);
        return format!("{out}\n[truncated — recall full output via recall tool]");
    }
    if line_capped {
        format!("{out}\n[truncated — recall full output via recall tool]")
    } else {
        out
    }
}
