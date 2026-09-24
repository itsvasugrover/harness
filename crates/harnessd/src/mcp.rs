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

/// Parse an `mcp.json` layer file.
pub fn parse_mcp_file(path: &str) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// Validate MCP config: transports known, stdio commands present, and
/// no literal secrets (those resolve via `env_ref` at spawn or the
/// file fails validation).
pub fn validate_mcp(doc: &serde_json::Value) -> Result<()> {
    let servers = doc
        .get("servers")
        .and_then(|v| v.as_object())
        .context("mcp.json needs servers")?;
    for (name, server) in servers {
        let transport = server
            .get("transport")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        anyhow::ensure!(
            transport == "stdio" || transport == "http",
            "server '{name}': bad transport"
        );
        if transport == "stdio" {
            anyhow::ensure!(
                server
                    .get("command")
                    .and_then(|v| v.as_str())
                    .is_some_and(|c| !c.is_empty()),
                "server '{name}': stdio needs command"
            );
        }
        reject_literal_secrets(name, server)?;
    }
    Ok(())
}

fn reject_literal_secrets(server: &str, v: &serde_json::Value) -> Result<()> {
    if let Some(obj) = v.as_object() {
        for (key, val) in obj {
            let lower = key.to_lowercase();
            let sensitive = ["token", "secret", "passwd", "password", "api_key", "apikey"]
                .iter()
                .any(|w| lower.contains(w));
            match val {
                serde_json::Value::String(s) if sensitive && looks_secret(s) => {
                    anyhow::bail!("server '{server}': literal secret in '{key}' (use env_ref)")
                }
                _ => reject_literal_secrets(server, val)?,
            }
        }
    } else if let Some(arr) = v.as_array() {
        for item in arr {
            reject_literal_secrets(server, item)?;
        }
    }
    Ok(())
}

/// Placeholder paths and short labels pass; real secrets do not.
fn looks_secret(s: &str) -> bool {
    s.len() >= 8 && !s.starts_with('$') && !s.starts_with('/') && !s.contains(' ')
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

    #[test]
    fn validates_mcp_and_rejects_literal_secrets() {
        let good: serde_json::Value = serde_json::from_str(
            r#"{"servers":{"forge":{"transport":"stdio","command":"forge-mcp","env_ref":["FORGE_TOKEN"]}}}"#,
        )
        .unwrap();
        assert!(validate_mcp(&good).is_ok());
        // Assembled so no secret-shaped literal sits in source.
        let bad_token = ["tok", "en-value-abcdef"].concat();
        let bad = serde_json::json!({"servers":{"x":{"transport":"http","url":"http://x","token": bad_token}}});
        assert!(validate_mcp(&bad).is_err());
    }
}
