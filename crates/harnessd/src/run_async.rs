//! Async run loop: model-driven turns with per-turn usage.
//! Split from `run.rs` for the file-size contract.
use super::run::{tools_for, RunConfig, UnitReport};
use anyhow::{Context, Result};
use work_engine::{loop_turn, store::Store};

/// Async twin of `run_unit`: model drivers decide turns over the
/// network and report provider usage, recorded per turn into SQLite
/// (feeds Bet 10 cost attribution).
pub async fn run_unit_async(
    store: &Store,
    cfg: &RunConfig,
    worker: &super::workers::Worker,
    driver: &mut (dyn super::model::AsyncDriver + Send),
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
        let Some((tool, input)) = driver.next_turn(steps).await? else {
            break;
        };
        let out = loop_turn::run_turn(&tools, &ctx, &tool, &input, steps, &loop_cfg)?;
        steps += 1;
        driver.observe(&out.title, &out.output);
        let (input_tokens, output_tokens) = driver.take_usage();
        store
            .record_usage(&cfg.session, input_tokens, output_tokens, 0.0)
            .await?;
        store
            .add_message(&work_engine::session::Message {
                id: format!("{}-m{steps}", cfg.session),
                session_id: cfg.session.clone(),
                role: "assistant".into(),
                body: format!("{}: {}", out.title, out.output),
                input_tokens,
                output_tokens,
            })
            .await?;
    }
    Ok(UnitReport {
        worker_id: worker.workdir.clone(),
        steps,
        delegated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn model_unit_records_usage() {
        use model_switchboard::adapters::openai::ChatEvent;
        let dir = std::env::temp_dir().join("harness-run-model");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "hello").unwrap();
        let store = Store::open("sqlite::memory:").await.unwrap();
        let cfg = RunConfig {
            repo: String::new(),
            wt_root: String::new(),
            base: String::new(),
            session: "rs2".into(),
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
        let mut driver = super::super::model::FakeModel::new(vec![vec![ChatEvent::ToolCall {
            name: "read".into(),
            input: "a.txt".into(),
        }]]);
        let rep = run_unit_async(&store, &cfg, &worker, &mut driver)
            .await
            .unwrap();
        assert_eq!(rep.steps, 1);
        let session = store.session("rs2").await.unwrap().unwrap();
        assert_eq!((session.input_tokens, session.output_tokens), (10, 5));
        let msgs = store.messages("rs2").await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!((msgs[0].input_tokens, msgs[0].output_tokens), (10, 5));
    }
}
