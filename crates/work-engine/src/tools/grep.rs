//! grep: substring search grouped by file, capped per file + total.
use super::super::jail;
use super::super::tool::{truncate, Tool, ToolCtx, ToolOutput};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const PER_FILE: usize = 20;

pub struct Grep;

fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                visit(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

impl Tool for Grep {
    fn name(&self) -> &'static str {
        "grep"
    }
    fn description(&self) -> &'static str {
        "Search contents. Input: 'pattern: <s>' + optional 'path: <subdir>'."
    }
    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let mut pattern: Option<&str> = None;
        let mut sub = ".";
        for line in input.lines() {
            if let Some(p) = line.strip_prefix("pattern:") {
                pattern = Some(p.trim());
            } else if let Some(p) = line.strip_prefix("path:") {
                sub = p.trim();
            }
        }
        let needle = pattern.context("missing 'pattern: <s>'")?;
        let base = jail::resolve(&ctx.workdir, ".")?;
        let root = jail::resolve(&ctx.workdir, sub)?;
        let mut files = Vec::new();
        visit(&root, &mut files);
        files.sort();
        let mut groups = Vec::new();
        for f in files {
            if std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0) > 1_048_576 {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            let hits: Vec<String> = text
                .lines()
                .enumerate()
                .filter(|(_, l)| l.contains(needle))
                .take(PER_FILE)
                .map(|(i, l)| format!("{}:{}", i + 1, l.chars().take(300).collect::<String>()))
                .collect();
            if !hits.is_empty() {
                groups.push(format!(
                    "{}:\n  {}",
                    f.strip_prefix(&base).unwrap().display(),
                    hits.join("\n  ")
                ));
            }
        }
        Ok(ToolOutput {
            title: format!("grep '{needle}' ({} files)", groups.len()),
            output: truncate(&groups.join("\n")),
        })
    }
}
