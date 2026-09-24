//! Tool registry: name -> implementation. Per-tool files in tools/.
use super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};
use std::collections::HashMap;

pub struct Registry {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name(), Box::new(tool));
    }

    pub fn run(&self, ctx: &ToolCtx, name: &str, input: &str) -> Result<ToolOutput> {
        match self.tools.get(name) {
            Some(t) => t.run(ctx, input),
            None => bail!(
                "unknown tool '{name}' (available: {})",
                self.names().join(", ")
            ),
        }
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.tools.keys().copied().collect();
        names.sort();
        names
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// The Phase-1 tools every agent receives.
pub fn builtins() -> Registry {
    use super::tools::{
        bash::Bash, edit::Edit, glob::Glob, grep::Grep, read::Read, recall::Recall, skill::Skill,
        task::Task, todo::Todo, webfetch::Webfetch, write::Write,
    };
    let mut r = Registry::new();
    r.register(Read);
    r.register(Write);
    r.register(Edit);
    r.register(Bash);
    r.register(Glob);
    r.register(Grep);
    r.register(Recall);
    r.register(Skill);
    r.register(Task);
    r.register(Todo);
    r.register(Webfetch);
    r
}

/// Builtins plus the lease-scoped `forge` tool. The daemon builds the
/// executor (it owns clients and credentials) and mints one lease per
/// session; run-path wiring passes both in when forges are configured.
pub fn builtins_with_forge(
    lease: forge_bridge::port::CapabilityLease,
    exec: std::sync::Arc<dyn super::tools::forge::ForgeExec>,
) -> Registry {
    use super::tools::forge::ForgeTool;
    let mut r = builtins();
    r.register(ForgeTool::new(lease, exec));
    r
}
