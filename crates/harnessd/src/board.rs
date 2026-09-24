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

/// Column for a forge PR card (observer facts, no worker involved).
/// Closed states rest in Done; anything needing a human lands in
/// NeedsYou; the rest await review. Human approval still gates the
/// merge itself via the Sentinel review gate, not this column.
#[allow(dead_code)] // board API exposure consumes this in Phase 5.
pub fn column_for_pr(card: &super::forge_facts::PrCard) -> Column {
    if card.state != "open" {
        return Column::Done;
    }
    if !card.checks_green || card.unresolved > 0 || !card.mergeable {
        return Column::NeedsYou;
    }
    Column::InReview
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

    fn pr(
        number: i64,
        state: &str,
        mergeable: bool,
        green: bool,
        unresolved: i64,
    ) -> crate::forge_facts::PrCard {
        crate::forge_facts::PrCard {
            number,
            title: "t".into(),
            state: state.into(),
            mergeable,
            checks_green: green,
            unresolved,
        }
    }

    #[test]
    fn derives_pr_columns() {
        assert_eq!(
            column_for_pr(&pr(1, "open", true, true, 0)),
            Column::InReview
        );
        assert_eq!(
            column_for_pr(&pr(1, "open", true, false, 0)),
            Column::NeedsYou
        );
        assert_eq!(
            column_for_pr(&pr(1, "open", true, true, 2)),
            Column::NeedsYou
        );
        assert_eq!(
            column_for_pr(&pr(1, "open", false, true, 0)),
            Column::NeedsYou
        );
        assert_eq!(column_for_pr(&pr(1, "merged", true, true, 0)), Column::Done);
    }
}
