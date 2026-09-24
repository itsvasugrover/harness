//! recall: restore a full output behind a Press/Trim placeholder.
//! Store dir is `<workdir>/.recall`; TTL enforced by the store.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::Result;

pub struct Recall;

impl Tool for Recall {
    fn name(&self) -> &'static str {
        "recall"
    }
    fn description(&self) -> &'static str {
        "Restore full text for a recall id. Input: id."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let dir = format!("{}/.recall", ctx.workdir.trim_end_matches('/'));
        let id = input.trim();
        match context_press::store::recall(&dir, id, 24 * 3600) {
            Some(text) => Ok(ToolOutput {
                title: format!("recall {id}"),
                output: super::super::tool::truncate(&text),
            }),
            None => anyhow::bail!("recall {id}: missing or expired"),
        }
    }
}
