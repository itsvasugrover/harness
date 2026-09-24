//! MCP server ownership: the daemon spawns, supervises, and stops
//! every MCP process. Decks and workers never spawn servers directly.
//! Tool-call attribution logging rides on top (Phase 4).
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Child;

pub struct McpRegistry {
    servers: HashMap<String, Child>,
    calls: Vec<CallRecord>,
}

/// One attributed tool call: which server, which tool, which receipt.
#[derive(Debug, Clone)]
pub struct CallRecord {
    pub server: String,
    pub tool: String,
    pub receipt: String,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            calls: Vec::new(),
        }
    }

    /// Spawn `cmd args...` as a named server. Env passes through;
    /// secrets arrive via the parent environment, never arguments.
    pub fn start(&mut self, name: &str, cmd: &str, args: &[&str]) -> Result<()> {
        let child = std::process::Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .spawn()
            .with_context(|| format!("mcp spawn failed: {cmd}"))?;
        self.servers.insert(name.into(), child);
        Ok(())
    }

    pub fn stop(&mut self, name: &str) -> Result<()> {
        if let Some(mut child) = self.servers.remove(name) {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }

    pub fn stop_all(&mut self) {
        for (_, mut child) in self.servers.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn running(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }

    /// Attribute a tool call to its server (receipt links the output).
    pub fn record_call(&mut self, server: &str, tool: &str, receipt: &str) {
        self.calls.push(CallRecord {
            server: server.into(),
            tool: tool.into(),
            receipt: receipt.into(),
        });
    }

    pub fn calls(&self) -> &[CallRecord] {
        &self.calls
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpRegistry {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_and_kills() {
        let mut reg = McpRegistry::new();
        reg.start("sleeper", "sleep", &["60"]).unwrap();
        assert_eq!(reg.running(), vec!["sleeper".to_string()]);
        reg.stop("sleeper").unwrap();
        assert!(reg.running().is_empty());
    }
}
