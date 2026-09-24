//! glob: minimal recursive matcher (`**`, `*`, `?`). No dependency.
use super::super::jail;
use super::super::tool::{truncate, Tool, ToolCtx, ToolOutput};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct Glob;

fn component_match(pat: &str, name: &str) -> bool {
    let (mut p, mut n) = (pat.as_bytes(), name.as_bytes());
    while !p.is_empty() {
        match p[0] {
            b'*' => {
                p = &p[1..];
                if p.is_empty() {
                    return true;
                }
                for i in 0..=n.len() {
                    if component_match(
                        std::str::from_utf8(p).unwrap_or(""),
                        std::str::from_utf8(&n[i..]).unwrap_or(""),
                    ) {
                        return true;
                    }
                }
                return false;
            }
            b'?' => {
                if n.is_empty() {
                    return false;
                }
                p = &p[1..];
                n = &n[1..];
            }
            c => {
                if n.is_empty() || n[0] != c {
                    return false;
                }
                p = &p[1..];
                n = &n[1..];
            }
        }
    }
    n.is_empty()
}

fn walk(dir: &Path, parts: &[&str], out: &mut Vec<PathBuf>) {
    if parts.is_empty() {
        return;
    }
    if parts[0] == "**" {
        walk(dir, &parts[1..], out);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    walk(&e.path(), parts, out);
                    walk(&e.path(), &parts[1..], out);
                } else if parts.len() == 1
                    || parts[1..].iter().all(|p| *p == "**")
                    || (parts.len() == 2
                        && component_match(parts[1], &e.file_name().to_string_lossy()))
                {
                    out.push(e.path());
                }
            }
        }
        return;
    }
    if parts.len() == 1 {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if component_match(parts[0], &e.file_name().to_string_lossy()) {
                    out.push(e.path());
                }
            }
        }
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.path().is_dir() && component_match(parts[0], &e.file_name().to_string_lossy()) {
                walk(&e.path(), &parts[1..], out);
            }
        }
    }
}

impl Tool for Glob {
    fn name(&self) -> &'static str {
        "glob"
    }
    fn description(&self) -> &'static str {
        "Find files by pattern ('**/*.rs'). Input: pattern."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let base = jail::resolve(&ctx.workdir, ".")?;
        let parts: Vec<&str> = input.trim().split('/').collect();
        let mut hits = Vec::new();
        walk(&base, &parts, &mut hits);
        hits.sort();
        let list: Vec<String> = hits
            .iter()
            .map(|p| p.strip_prefix(&base).unwrap().display().to_string())
            .collect();
        Ok(ToolOutput {
            title: format!("glob {} ({} hits)", input.trim(), list.len()),
            output: truncate(&list.join("\n")),
        })
    }
}
