//! Skill frontmatter: parse + version gate.
//! Full SKILL.md bodies load on trigger match only (see docs).
use anyhow::{Context, Result};
use serde::Deserialize;

/// Engine version this binary reports to skill gates.
pub const ENGINE_VERSION: &str = "0.1.0";

#[derive(Debug, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub min_engine: Option<String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

impl SkillMeta {
    /// Parse `---` frontmatter block from a SKILL.md file.
    pub fn parse(text: &str) -> Result<Self> {
        let block = text
            .split("---")
            .nth(1)
            .context("SKILL.md missing frontmatter block")?;
        Ok(noyalib::from_str(block)?)
    }

    /// Reject skills that need a newer engine (fail loud, never half-run).
    pub fn check_engine(&self) -> Result<()> {
        if let Some(min) = &self.min_engine {
            anyhow::ensure!(
                version_gte(ENGINE_VERSION, min),
                "skill '{}' needs engine >= {} (have {})",
                self.name,
                min,
                ENGINE_VERSION
            );
        }
        Ok(())
    }
}

/// Minimal semver `a >= b` over `major.minor.patch` (no pre-release).
fn version_gte(a: &str, b: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    }
    let (x, y) = (parts(a), parts(b));
    for i in 0..3 {
        let (m, n) = (*x.get(i).unwrap_or(&0), *y.get(i).unwrap_or(&0));
        if m != n {
            return m > n;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nname: demo\nmin_engine: 0.1.0\n---\nbody";

    #[test]
    fn parses_and_gates() {
        let m = SkillMeta::parse(SAMPLE).unwrap();
        assert_eq!(m.name, "demo");
        assert!(m.check_engine().is_ok());
    }
}
