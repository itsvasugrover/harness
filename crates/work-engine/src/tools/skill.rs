//! skill: progressive-disclosure lookup (list/match/load).
//! Reads `<workdir>/.harness/skills/*/SKILL.md` plus repo `skills/*/SKILL.md`
//! when present; falls back to bundled names so decks never go empty.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};

const BUNDLED: &[&str] = &["forge-ops", "failure-notes", "review-gate"];

fn skill_dirs(ctx: &ToolCtx) -> Vec<String> {
    vec![
        format!("{}/.harness/skills", ctx.workdir.trim_end_matches('/')),
        "skills".into(),
    ]
}

fn indexed(ctx: &ToolCtx) -> Vec<(String, String)> {
    let mut out = vec![];
    for dir in skill_dirs(ctx) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let file = entry.path().join("SKILL.md");
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            if let Ok(meta) = super::super::skill::SkillMeta::parse(&text) {
                if meta.check_engine().is_ok() {
                    out.push((meta.name, file.to_string_lossy().into_owned()));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

pub struct Skill;

impl Tool for Skill {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn description(&self) -> &'static str {
        "Skills as JSON: {op:list|match|load, text?, name?}."
    }

    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let v: serde_json::Value = serde_json::from_str(input)
            .map_err(|_| anyhow::anyhow!("skill: input must be JSON"))?;
        let op = v.get("op").and_then(|x| x.as_str()).unwrap_or("list");
        let mut entries = indexed(ctx);
        if entries.is_empty() {
            entries = BUNDLED
                .iter()
                .map(|n| ((*n).into(), String::new()))
                .collect();
        }
        let out = match op {
            "list" => entries
                .iter()
                .map(|(n, _)| n.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            "match" => {
                let text = v
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                entries
                    .iter()
                    .filter(|(n, _)| text.contains(&n.to_lowercase()))
                    .map(|(n, _)| n.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "load" => {
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let Some((_, path)) = entries.iter().find(|(n, _)| n == name) else {
                    bail!("skill: unknown skill '{name}'");
                };
                if path.is_empty() {
                    format!("bundled skill '{name}' (body ships with daemon)")
                } else {
                    std::fs::read_to_string(path)?
                }
            }
            other => bail!("skill: unknown op '{other}' (list|match|load)"),
        };
        Ok(ToolOutput {
            title: format!("skill {op}"),
            output: super::super::tool::truncate(&out),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_bundled_when_no_dir() {
        let ctx = ToolCtx {
            session_id: "s".into(),
            workdir: "/tmp".into(),
        };
        let out = Skill.run(&ctx, r#"{"op":"list"}"#).unwrap();
        assert!(out.output.contains("forge-ops"));
    }
}
