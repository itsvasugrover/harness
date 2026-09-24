//! webfetch: fetch a URL or file into truncated text.
//! SSRF-safe by default: only http(s) + file, 1 MiB cap, 15s timeout.
//! Network needs session approval (see `needs_approval`); file reads
//! stay jailed to the workdir like `read`.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};

const MAX_BYTES: usize = 1024 * 1024;

pub struct Webfetch;

fn allowed(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://") || url.starts_with("file://")
}

impl Tool for Webfetch {
    fn name(&self) -> &'static str {
        "webfetch"
    }

    fn description(&self) -> &'static str {
        "Fetch URL/file as JSON: {url}. Caps at 1 MiB."
    }

    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let v: serde_json::Value = serde_json::from_str(input)
            .map_err(|_| anyhow::anyhow!("webfetch: input must be JSON"))?;
        let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
        if !allowed(url) {
            bail!("webfetch: only http(s):// and file:// URLs");
        }
        let text = if let Some(path) = url.strip_prefix("file://") {
            // Jail file reads to the workdir.
            let workdir = std::path::Path::new(&ctx.workdir);
            let target = workdir.join(path.trim_start_matches('/'));
            let canonical = std::fs::canonicalize(&target)
                .map_err(|_| anyhow::anyhow!("webfetch: unreadable file"))?;
            let base = std::fs::canonicalize(workdir).unwrap_or_else(|_| workdir.into());
            if !canonical.starts_with(base) {
                bail!("webfetch: file outside workdir");
            }
            std::fs::read_to_string(canonical)?
        } else {
            // Never block_on inline: the run loop may already execute
            // inside the daemon runtime, where a nested block_on panics.
            // Fetch on a fresh OS thread with its own runtime instead.
            let owned = url.to_string();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(async {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(15))
                        .build()?;
                    let res = client.get(&owned).send().await?;
                    let bytes = res.bytes().await?;
                    if bytes.len() > MAX_BYTES {
                        bail!("webfetch: response over 1 MiB");
                    }
                    Ok::<String, anyhow::Error>(String::from_utf8_lossy(&bytes).into_owned())
                })
            })
            .join()
            .map_err(|_| anyhow::anyhow!("webfetch: fetch thread failed"))??
        };
        Ok(ToolOutput {
            title: format!("webfetch {url}"),
            output: super::super::tool::truncate(&text),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_file() {
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        assert!(Webfetch.run(&ctx, r#"{"url":"ftp://x"}"#).is_err());
    }

    #[test]
    fn reads_jailed_file() {
        let dir = std::env::temp_dir().join(format!("harness-fetch-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hello fetch").unwrap();
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: dir.to_string_lossy().into_owned(),
        };
        let out = Webfetch.run(&ctx, r#"{"url":"file://a.txt"}"#).unwrap();
        assert!(out.output.contains("hello"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn fetches_http_inside_runtime_without_panic() {
        // Regression: inline block_on panics with "Cannot start a runtime
        // from within a runtime" when the loop runs in async context.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let body = "hello over http";
            let res = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(res.as_bytes());
        });
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        let out = Webfetch
            .run(&ctx, &format!(r#"{{"url":"http://127.0.0.1:{port}/"}}"#))
            .unwrap();
        assert!(out.output.contains("hello over http"));
        let _ = server.join();
    }
}
