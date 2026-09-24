//! Session handover export: when context hits the early line and
//! compaction cannot free enough, the engine writes this file so a
//! fresh session (or another agent) resumes with zero re-discovery.
//! Mirrors docs/handover.md at runtime.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Handover {
    pub session_id: String,
    pub agent: String,
    pub model: String,
    pub goal: String,
    pub decisions: Vec<String>,
    pub touched_files: Vec<String>,
    pub next_steps: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Fresh agent receiving this task after the 40% soft cap.
    pub successor: String,
    /// Why the task moved (e.g. "context at 52% of usable").
    pub delegation_reason: String,
}

impl Handover {
    pub fn render(&self) -> String {
        let list = |items: &[String]| {
            if items.is_empty() {
                "  (none recorded)".into()
            } else {
                items
                    .iter()
                    .map(|i| format!("  - {i}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };
        format!(
            "# Handover from {sid}\n\n\
             agent: {agent}\nmodel: {model}\n\
             usage: {inp} in / {out} out\n\
             handing to: {successor} ({reason})\n\n\
             ## Goal\n{goal}\n\n\
             ## Decisions\n{dec}\n\n\
             ## Touched files\n{touched}\n\n\
             ## Next steps\n{next}\n",
            sid = self.session_id,
            agent = self.agent,
            model = self.model,
            inp = self.input_tokens,
            out = self.output_tokens,
            successor = self.successor,
            reason = self.delegation_reason,
            goal = self.goal,
            dec = list(&self.decisions),
            touched = list(&self.touched_files),
            next = list(&self.next_steps),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_sections() {
        let h = Handover {
            session_id: "s1".into(),
            goal: "fix login".into(),
            next_steps: vec!["run tests".into()],
            ..Default::default()
        };
        let md = h.render();
        assert!(md.contains("# Handover from s1"));
        assert!(md.contains("fix login"));
        assert!(md.contains("run tests"));
    }
}
