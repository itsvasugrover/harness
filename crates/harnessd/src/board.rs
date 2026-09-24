//! Derived board: columns computed at read time from durable facts.
//! Nothing here is stored — liveness + blockers decide the column.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardFacts {
    pub worker_id: String,
    pub alive: bool,
    pub blocked: Option<String>,
    pub pr_open: bool,
    pub checks_green: bool,
    pub approved: bool,
    /// Finished units rest here regardless of liveness.
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    Working,
    NeedsYou,
    InReview,
    ReadyToMerge,
    Done,
}

/// Done first: finished units stay visible until archived. Then the
/// usual derivation — Working, NeedsYou, InReview, ReadyToMerge.
pub fn column(f: &CardFacts) -> Column {
    if f.completed {
        return Column::Done;
    }
    if !f.alive || f.blocked.is_some() {
        return Column::NeedsYou;
    }
    if f.pr_open {
        if f.approved && f.checks_green {
            return Column::ReadyToMerge;
        }
        return Column::InReview;
    }
    if !f.checks_green {
        return Column::NeedsYou;
    }
    Column::Working
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> CardFacts {
        CardFacts {
            worker_id: "w1".into(),
            alive: true,
            blocked: None,
            pr_open: false,
            checks_green: true,
            approved: false,
            completed: false,
        }
    }

    #[test]
    fn derives_all_columns() {
        assert_eq!(column(&facts()), Column::Working);
        assert_eq!(
            column(&CardFacts {
                pr_open: true,
                ..facts()
            }),
            Column::InReview
        );
        assert_eq!(
            column(&CardFacts {
                pr_open: true,
                approved: true,
                ..facts()
            }),
            Column::ReadyToMerge
        );
        assert_eq!(
            column(&CardFacts {
                blocked: Some("ci".into()),
                ..facts()
            }),
            Column::NeedsYou
        );
        assert_eq!(
            column(&CardFacts {
                alive: false,
                completed: true,
                ..facts()
            }),
            Column::Done
        );
    }
}
