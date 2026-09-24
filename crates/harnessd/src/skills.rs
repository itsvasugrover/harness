//! Skill/command/MCP loader: builds the runtime indices the daemon
//! serves to agents and decks. Progressive disclosure holds — the
//! system prompt carries one line per skill; bodies load on trigger
//! match or explicit call. Local layers shadow global on collision.
use anyhow::{Context, Result};

/// One indexed skill: frontmatter plus a one-line summary.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub triggers: Vec<String>,
    pub summary: String,
    pub path: String,
    pub layer: String,
}

#[derive(Debug, Clone, Default)]
pub struct SkillIndex {
    pub entries: Vec<SkillEntry>,
}

impl SkillIndex {
    /// Index `<dir>/*/SKILL.md`. Missing dir = empty index, not error.
    pub fn build(dir: &str, layer: &str) -> Self {
        let mut entries = vec![];
        let Ok(read) = std::fs::read_dir(dir) else {
            return Self::default();
        };
        let mut dirs: Vec<_> = read.filter_map(|e| e.ok()).collect();
        dirs.sort_by_key(|e| e.file_name());
        for entry in dirs {
            let skill_file = entry.path().join("SKILL.md");
            let Ok(text) = std::fs::read_to_string(&skill_file) else {
                continue;
            };
            let Ok(meta) = work_engine::skill::SkillMeta::parse(&text) else {
                continue;
            };
            if meta.check_engine().is_err() {
                continue;
            }
            entries.push(SkillEntry {
                name: meta.name,
                triggers: meta.triggers,
                summary: summary_of(&text),
                path: skill_file.to_string_lossy().into(),
                layer: layer.into(),
            });
        }
        Self { entries }
    }

    /// Skills whose trigger appears in the task text (case-insensitive).
    pub fn matches(&self, text: &str) -> Vec<&SkillEntry> {
        let lower = text.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.triggers.iter().any(|t| lower.contains(&t.to_lowercase())))
            .collect()
    }

    /// Full body for an indexed skill (loads on match or explicit call).
    pub fn load_body(&self, name: &str) -> Result<String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.name == name)
            .context("unknown skill")?;
        Ok(std::fs::read_to_string(&entry.path)?)
    }
}

/// Merge two layers: `over` wins on name collision.
pub fn merge_index(base: SkillIndex, over: SkillIndex) -> SkillIndex {
    let mut entries = base.entries;
    for entry in over.entries {
        entries.retain(|e| e.name != entry.name);
        entries.push(entry);
    }
    SkillIndex { entries }
}

/// First body line after frontmatter, capped — the prompt-sized view.
fn summary_of(text: &str) -> String {
    let mut parts = text.splitn(3, "---");
    parts.next();
    parts.next();
    let body = parts.next().unwrap_or("");
    let line = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    const CAP: usize = 120;
    if line.len() > CAP {
        format!("{}…", &line[..CAP])
    } else {
        line.into()
    }
}

/// One slash command: prompt template plus a one-line summary.
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub name: String,
    pub summary: String,
    pub layer: String,
}

/// Index `<dir>/*.md`. Name is the stem; summary is the first heading
/// or first non-empty line.
pub fn load_commands(dir: &str, layer: &str) -> Vec<CommandEntry> {
    let mut out = vec![];
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut files: Vec<_> = read.filter_map(|e| e.ok()).collect();
    files.sort_by_key(|e| e.file_name());
    for file in files {
        let path = file.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .file_stem()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .into();
        out.push(CommandEntry {
            name,
            summary: summary_of(&format!("---\n---\n{text}")),
            layer: layer.into(),
        });
    }
    out
}

/// Agent names from `<dir>/*.yaml` (stem, or the `name:` field).
pub fn load_agents(dir: &str) -> Vec<String> {
    let mut out = vec![];
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut files: Vec<_> = read.filter_map(|e| e.ok()).collect();
    files.sort_by_key(|e| e.file_name());
    for file in files {
        let path = file.path();
        if path.extension().and_then(|x| x.to_str()) != Some("yaml") {
            continue;
        }
        let stem: String = path
            .file_stem()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .into();
        out.push(stem);
    }
    out
}

/// Merged `help` listing with layer tags for agents and the phone.
pub fn help(
    skills: &SkillIndex,
    commands: &[CommandEntry],
    agents: &[String],
    layer: &str,
) -> String {
    let mut out = String::from("skills:\n");
    for s in &skills.entries {
        out.push_str(&format!("  {} [{}] — {}\n", s.name, s.layer, s.summary));
    }
    out.push_str("commands:\n");
    for c in commands {
        out.push_str(&format!("  /{} [{}] — {}\n", c.name, c.layer, c.summary));
    }
    out.push_str("agents:\n");
    for a in agents {
        out.push_str(&format!("  {a} [{layer}]\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_dir(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("harness-skills-test-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("skills").join("demo")).unwrap();
        std::fs::create_dir_all(dir.join("commands")).unwrap();
        std::fs::create_dir_all(dir.join("agents")).unwrap();
        std::fs::write(
            dir.join("skills").join("demo").join("SKILL.md"),
            "---\nname: demo\ntriggers: [issues, triage]\n---\nDemo body line.\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("commands").join("go.md"),
            "# /go — ship it\nRun the thing.\n",
        )
        .unwrap();
        std::fs::write(dir.join("agents").join("scout.yaml"), "name: scout\n").unwrap();
        dir.to_string_lossy().into()
    }

    #[test]
    fn indexes_matches_and_merges() {
        let g = layer_dir("g");
        let l = layer_dir("l");
        let global = SkillIndex::build(&format!("{g}/skills"), "global");
        assert_eq!(global.entries.len(), 1);
        assert_eq!(global.matches("triage the issues").len(), 1);
        assert!(global.matches("write code").is_empty());
        let body = global.load_body("demo").unwrap();
        assert!(body.contains("Demo body"));
        let local = SkillIndex::build(&format!("{l}/skills"), "local");
        let merged = merge_index(global, local);
        assert_eq!(merged.entries.len(), 1);
        assert_eq!(merged.entries[0].layer, "local");
        let cmds = load_commands(&format!("{g}/commands"), "global");
        assert_eq!(cmds.len(), 1);
        assert_eq!(load_agents(&format!("{g}/agents")), vec!["scout"]);
    }
}
