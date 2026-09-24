//! Work Engine: agent loop + tools + sessions.
//! Split per stage: loop_turn.rs processor.rs compaction.rs overflow.rs
//! store.rs session.rs skill.rs task.rs approvals.rs handover.rs tools/.
pub mod approvals;
pub mod compaction;
pub mod compaction_qa;
pub mod handover;
pub mod jail;
pub mod loop_turn;
pub mod overflow;
pub mod port;
pub mod processor;
pub mod receipts;
pub mod registry;
pub mod replay;
pub mod retrieve;
pub mod session;
pub mod skill;
pub mod store;
pub mod task;
pub mod tool;
pub mod tools;
