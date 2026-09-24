//! Goal execution: plan -> spawn workers in parallel -> model loop
//! -> archive. Dirty worktrees are left in place with a loud message,
//! never deleted. Facts append to `board.json` for the served board.
use anyhow::{Context, Result};

/// One goal end to end, units running concurrently.
pub async fn run_goal(
    cfg: &super::config::HarnessConfig,
    goal: &str,
    repo: &str,
    base: &str,
    data_dir: &str,
) -> Result<()> {
    use work_engine::store::Store;
    let store = Store::open(&format!(
        "sqlite://{data_dir}/db/harness.db?create_if_missing=true"
    ))
    .await?;
    let wt_root = format!("{data_dir}/work");
    let model_ref = model_switchboard::port::ModelRef::parse(&cfg.default_model)?;
    let prov = cfg
        .providers
        .get(&model_ref.provider)
        .with_context(|| format!("unknown provider {}", model_ref.provider))?;
    let envs: Vec<&str> = prov.env.iter().map(String::as_str).collect();
    let key = super::keys::resolve_all(None, &model_ref.provider, &envs)
        .context("no key: set a provider env var or keychain entry")?;
    let mut set = tokio::task::JoinSet::new();
    for assignment in super::run::plan_goal(goal) {
        let ctx = Ctx {
            store: store.clone(),
            repo: repo.into(),
            wt_root: wt_root.clone(),
            base: base.into(),
            model: cfg.default_model.clone(),
            base_url: prov.base_url.clone(),
            key: key.clone(),
            model_name: model_ref.model.clone(),
            assignment,
        };
        set.spawn(async move { run_assignment(ctx).await });
    }
    let mut facts = load_facts(data_dir);
    while let Some(res) = set.join_next().await {
        let (worker_id, steps, delegated, note) = res??;
        println!("{worker_id}: {steps} steps delegated={delegated} {note}");
        let mut blockers = Vec::new();
        if delegated {
            blockers.push("soft cap hit: successor handover required".to_string());
        }
        if !note.is_empty() {
            blockers.push(note);
        }
        let blocked = (!blockers.is_empty()).then(|| blockers.join("; "));
        facts.retain(|f: &super::board::CardFacts| f.worker_id != worker_id);
        facts.push(super::board::CardFacts {
            worker_id,
            alive: false,
            blocked,
            pr_open: false,
            checks_green: true,
            approved: false,
            completed: !delegated,
        });
    }
    save_facts(data_dir, &facts)?;
    Ok(())
}

struct Ctx {
    store: work_engine::store::Store,
    repo: String,
    wt_root: String,
    base: String,
    model: String,
    base_url: String,
    key: String,
    model_name: String,
    assignment: super::planner::Assignment,
}

async fn run_assignment(ctx: Ctx) -> Result<(String, u32, bool, String)> {
    let worker = super::workers::spawn(
        &ctx.repo,
        &ctx.wt_root,
        &ctx.assignment.worker_id,
        &ctx.assignment.unit_title,
        &ctx.base,
    )?;
    let checkpoint = super::checkpoint::Checkpoint::new(&ctx.assignment.worker_id, &ctx.wt_root);
    let snapshot = checkpoint.snapshot(&worker.workdir)?;
    println!(
        "{} on {} @ {}",
        ctx.assignment.worker_id, worker.branch, snapshot.sha
    );
    let run_cfg = super::run::RunConfig {
        repo: ctx.repo.clone(),
        wt_root: ctx.wt_root.clone(),
        base: ctx.base.clone(),
        session: ctx.assignment.worker_id.clone(),
        agent: "build".into(),
        model: ctx.model.clone(),
        input_limit: 200_000,
    };
    let mut driver = super::model::OpenAiDriver::new(
        &ctx.base_url,
        &ctx.key,
        &ctx.model_name,
        &format!(
            "Unit: {}\nScope: {}",
            ctx.assignment.unit_title, ctx.assignment.unit_scope
        ),
    );
    let report = super::run::run_unit_async(&ctx.store, &run_cfg, &worker, &mut driver).await?;
    let note = match super::workers::archive(&ctx.repo, &worker) {
        Ok(()) => String::new(),
        Err(e) => format!("dirty worktree kept at {}: {e}", worker.workdir),
    };
    Ok((
        ctx.assignment.worker_id,
        report.steps,
        report.delegated,
        note,
    ))
}

fn facts_path(data_dir: &str) -> String {
    format!("{data_dir}/board.json")
}

pub fn load_facts(data_dir: &str) -> Vec<super::board::CardFacts> {
    std::fs::read_to_string(facts_path(data_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_facts(data_dir: &str, facts: &[super::board::CardFacts]) -> Result<()> {
    std::fs::write(facts_path(data_dir), serde_json::to_string_pretty(facts)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn two_units_run_concurrently() {
        use model_switchboard::adapters::openai::ChatEvent;
        use work_engine::store::Store;
        let dir = std::env::temp_dir().join("harness-goal-parallel");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("w1")).unwrap();
        std::fs::create_dir_all(dir.join("w2")).unwrap();
        std::fs::write(dir.join("w1/a.txt"), "one").unwrap();
        std::fs::write(dir.join("w2/a.txt"), "two").unwrap();
        let store = Store::open("sqlite::memory:").await.unwrap();
        let mk = |id: &str, wd: &str| {
            let worker = crate::workers::Worker {
                id: id.into(),
                branch: "b".into(),
                workdir: wd.into(),
            };
            let cfg = crate::run::RunConfig {
                repo: String::new(),
                wt_root: String::new(),
                base: String::new(),
                session: id.into(),
                agent: "build".into(),
                model: "demo/m".into(),
                input_limit: 200_000,
            };
            let driver = super::super::model::FakeModel::new(vec![vec![ChatEvent::ToolCall {
                name: "read".into(),
                input: "a.txt".into(),
            }]]);
            (store.clone(), cfg, worker, driver)
        };
        let (s1, c1, w1, mut d1) = mk("p1", dir.join("w1").to_str().unwrap());
        let (s2, c2, w2, mut d2) = mk("p2", dir.join("w2").to_str().unwrap());
        let mut set = tokio::task::JoinSet::new();
        set.spawn(async move { crate::run::run_unit_async(&s1, &c1, &w1, &mut d1).await });
        set.spawn(async move { crate::run::run_unit_async(&s2, &c2, &w2, &mut d2).await });
        let mut done = 0;
        while let Some(res) = set.join_next().await {
            assert_eq!(res.unwrap().unwrap().steps, 1);
            done += 1;
        }
        assert_eq!(done, 2);
    }
}
