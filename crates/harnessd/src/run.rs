//! Run loop: planner -> spawn -> gate -> dispatch -> record.
//! The `Driver` decides each turn's tool call; `ScriptDriver` proves
//! the wiring in tests, `model::OpenAiDriver` drives live turns.
use anyhow::{Context, Result};
use forge_bridge::port::CapabilityLease;
use std::sync::Arc;
use work_engine::tools::forge::ForgeExec;
use work_engine::{loop_turn, registry, store::Store, task};

/// One turn's instruction from the driver. `None` = unit complete.
pub type Turn = Option<(String, String)>;

pub trait Driver {
    fn next_turn(&mut self, steps_used: u32) -> Turn;
    /// Feed a finished tool result back (script drivers ignore this).
    fn observe(&mut self, _title: &str, _output: &str) {}
}

/// Fixed script for tests and demos (no model needed).
pub struct ScriptDriver {
    turns: Vec<(String, String)>,
    pos: usize,
}

impl ScriptDriver {
    pub fn new(turns: Vec<(String, String)>) -> Self {
        Self { turns, pos: 0 }
    }
}

impl Driver for ScriptDriver {
    fn next_turn(&mut self, _steps: u32) -> Turn {
        if self.pos >= self.turns.len() {
            return None;
        }
        let t = self.turns[self.pos].clone();
        self.pos += 1;
        Some(t)
    }
}

pub struct RunConfig {
    pub repo: String,
    pub wt_root: String,
    pub base: String,
    pub session: String,
    pub agent: String,
    pub model: String,
    pub input_limit: u64,
    /// Lease-scoped forge access. `None` = agents get no `forge` tool.
    pub forge: Option<ForgeScope>,
}

/// Session forge scope: the daemon minted this lease for the run's
/// repo and owns the executor behind it.
#[derive(Clone)]
pub struct ForgeScope {
    pub lease: CapabilityLease,
    pub exec: Arc<dyn ForgeExec>,
}

pub(crate) fn tools_for(cfg: &RunConfig) -> registry::Registry {
    match &cfg.forge {
        Some(scope) => registry::builtins_with_forge(scope.lease.clone(), scope.exec.clone()),
        None => registry::builtins(),
    }
}

pub struct UnitReport {
    pub worker_id: String,
    pub steps: u32,
    pub delegated: bool,
}

/// Run one unit to completion: gate each turn, dispatch tools, record
/// messages. On soft-cap delegation the loop stops with
/// `delegated: true` so the caller can export a handover + spawn a
/// successor; this function never writes the handover itself.
pub async fn run_unit(
    store: &Store,
    cfg: &RunConfig,
    worker: &super::workers::Worker,
    _task_title: &str,
    driver: &mut dyn Driver,
) -> Result<UnitReport> {
    let tools = tools_for(cfg);
    let ctx = work_engine::tool::ToolCtx {
        session_id: cfg.session.clone(),
        workdir: worker.workdir.clone(),
    };
    let loop_cfg = loop_turn::LoopConfig::default();
    let mut steps = 0u32;
    let mut delegated = false;
    store
        .create_session(&cfg.session, &cfg.agent, &cfg.model)
        .await
        .ok();
    loop {
        let session = store
            .session(&cfg.session)
            .await?
            .with_context(|| format!("session {} vanished mid-run", cfg.session))?;
        let used = (session.input_tokens + session.output_tokens) as u64;
        match loop_turn::gate(used, cfg.input_limit, 20_000, 8_000) {
            loop_turn::Gate::Delegate { .. } => {
                delegated = true;
                break;
            }
            loop_turn::Gate::Proceed => {}
        }
        let Some((tool, input)) = driver.next_turn(steps) else {
            break;
        };
        let out = loop_turn::run_turn(&tools, &ctx, &tool, &input, steps, &loop_cfg)?;
        steps += 1;
        driver.observe(&out.title, &out.output);
        store
            .add_message(&work_engine::session::Message {
                id: format!("{}-m{steps}", cfg.session),
                session_id: cfg.session.clone(),
                role: "assistant".into(),
                body: format!("{}: {}", out.title, out.output),
                input_tokens: 0,
                output_tokens: 0,
            })
            .await?;
    }
    Ok(UnitReport {
        worker_id: worker.workdir.clone(),
        steps,
        delegated,
    })
}
/// Plan a goal into worker assignments (planner owns order).
pub fn plan_goal(goal: &str) -> Vec<super::planner::Assignment> {
    let units = task::split_goal(goal, 8, 25);
    let pairs: Vec<(String, String)> = units.into_iter().map(|u| (u.title, u.scope)).collect();
    super::planner::plan(&pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_units() {
        let a = plan_goal("do x\ndo y");
        assert_eq!(a.len(), 2);
    }

    #[tokio::test]
    async fn script_unit_reads_file() {
        let dir = std::env::temp_dir().join("harness-run-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hello").unwrap();
        let store = Store::open("sqlite::memory:").await.unwrap();
        let cfg = RunConfig {
            repo: String::new(),
            wt_root: String::new(),
            base: String::new(),
            session: "rs1".into(),
            agent: "build".into(),
            model: "demo/m".into(),
            input_limit: 200_000,
            forge: None,
        };
        let worker = super::super::workers::Worker {
            id: "w1".into(),
            branch: "b".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        let mut driver = ScriptDriver::new(vec![("read".into(), "a.txt".into())]);
        let rep = run_unit(&store, &cfg, &worker, "t", &mut driver)
            .await
            .unwrap();
        assert_eq!(rep.steps, 1);
        assert!(!rep.delegated);
        assert_eq!(store.messages("rs1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn forged_unit_runs_forge_tool() {
        struct Fake;
        impl ForgeExec for Fake {
            fn exec(
                &self,
                _lease: &CapabilityLease,
                op: &str,
                _input: &str,
            ) -> anyhow::Result<String> {
                Ok(format!("{op} ok"))
            }
        }
        let dir = std::env::temp_dir().join("harness-run-forge");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open("sqlite::memory:").await.unwrap();
        let cfg = RunConfig {
            repo: "o/r".into(),
            wt_root: String::new(),
            base: String::new(),
            session: "rs3".into(),
            agent: "build".into(),
            model: "demo/m".into(),
            input_limit: 200_000,
            forge: Some(ForgeScope {
                lease: CapabilityLease::mint("fake", "o/r", vec!["forge.read".into()]),
                exec: Arc::new(Fake),
            }),
        };
        let worker = super::super::workers::Worker {
            id: "w1".into(),
            branch: "b".into(),
            workdir: dir.to_str().unwrap().into(),
        };
        let mut driver = ScriptDriver::new(vec![(
            "forge".into(),
            r#"{"op":"issues","repo":"o/r"}"#.into(),
        )]);
        let rep = run_unit(&store, &cfg, &worker, "t", &mut driver)
            .await
            .unwrap();
        assert_eq!(rep.steps, 1);
        let msgs = store.messages("rs3").await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("issues ok"));
    }
}
