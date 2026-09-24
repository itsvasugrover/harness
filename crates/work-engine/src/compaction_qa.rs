//! Bet 8 — compaction QA probe (stub; full build v1).
//! A micro-model must answer 3 questions from the summary or we widen.
pub fn questions() -> [&'static str; 3] {
    [
        "What is the current goal?",
        "What is blocked?",
        "What is the next step?",
    ]
}

/// Pass = every answer non-empty. Real grader arrives with micro-models.
pub fn grade(answers: &[&str]) -> bool {
    answers.len() == 3 && answers.iter().all(|a| !a.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_answers() {
        assert!(!grade(&["goal", "", "next"]));
        assert!(grade(&["goal", "nothing", "next"]));
    }
}
