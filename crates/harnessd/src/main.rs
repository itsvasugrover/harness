//! harnessd: loopback API + scheduler. No agent/tool logic here.
mod api;
mod board;
mod checkpoint;
mod config;
mod config_sections;
mod doctor;
#[allow(dead_code)] // run-path lease wiring consumes this in Phase 5.
mod forge_exec;
mod forge_facts;
mod forge_ops;
mod goal;
mod keys;
#[allow(dead_code)] // run loop passes MCP tools to workers next.
mod mcp;
mod model;
mod observer;
#[allow(dead_code)] // 3b wires planner + workers into the run loop.
mod planner;
#[allow(dead_code)] // resume scan runs at serve boot in 3b-iii.
mod resume;
#[allow(dead_code)] // run.rs drives the loop; serve wires it in 3b-ii.
mod run;
mod run_async;
#[allow(dead_code)] // help/loader serving lands with the deck API in Phase 5.
mod skills;
#[allow(dead_code)] // 3b run loop spawns workers per assignment.
mod workers;

use anyhow::Result;
use axum::{
    http::Request,
    middleware::{self, Next},
    response::Response,
};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "harnessd",
    version,
    about = "Local-first agentic harness daemon"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Print version and exit.
    Version,
    /// Preflight: git, dirs, ports in one readable report.
    Doctor,
    /// Serve the loopback API + gateway. `--dry-run` prints the routing
    /// table without binding, keys, or network.
    Serve {
        #[arg(long, default_value = "127.0.0.1:4317")]
        bind: String,
        #[arg(long)]
        dry_run: bool,
        /// Model ref to trial-route in dry-run mode.
        #[arg(long)]
        model: Option<String>,
        /// Optional LAN listener for the phone (requires --lan-bearer).
        #[arg(long)]
        lan_bind: Option<String>,
        /// Bearer token the phone presents; never logged.
        #[arg(long)]
        lan_bearer: Option<String>,
    },
    /// Run a goal: plan -> spawn workers -> model loop -> archive.
    Run {
        goal: String,
        #[arg(long)]
        repo: String,
        #[arg(long, default_value = "main")]
        base: String,
    },
}

/// LAN gate: everything except the exact identity probe needs the bearer.
async fn lan_gate(
    axum::extract::State(bearer): axum::extract::State<String>,
    req: Request<axum::body::Body>,
    next: Next,
) -> std::result::Result<Response, axum::http::StatusCode> {
    if api::authorized(req.headers(), &bearer, req.uri().path()) {
        Ok(next.run(req).await)
    } else {
        Err(axum::http::StatusCode::UNAUTHORIZED)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Version => println!("harnessd {}", env!("CARGO_PKG_VERSION")),
        Cmd::Doctor => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let data_dir =
                std::env::var("HARNESS_DATA_DIR").unwrap_or_else(|_| format!("{home}/.harness"));
            let checks = vec![
                doctor::check_git(),
                doctor::check_writable_dir(&format!("{data_dir}/work")),
                doctor::check_port_free(4317),
            ];
            println!("{}", doctor::Check::report(&checks));
        }
        Cmd::Serve {
            bind,
            dry_run,
            model,
            lan_bind,
            lan_bearer,
        } => {
            // Providers come from config layers only — nothing hardcoded.
            // models.dev is the base listing; local config overlays + wins.
            // Data dirs exist before anything touches sqlite, cache, or facts.
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let data_dir =
                std::env::var("HARNESS_DATA_DIR").unwrap_or_else(|_| format!("{home}/.harness"));
            let global = config::load_layer(&format!("{home}/.harness/config.yaml"))?;
            let local = config::load_layer(".harness/config.yaml")?;
            let merged = config::merge(global, local);
            for sub in ["work", "db", "cache"] {
                std::fs::create_dir_all(format!("{data_dir}/{sub}"))?;
            }
            let cache_path = format!("{data_dir}/cache/models-dev.json");
            let mut catalog = if merged.catalog_url.is_empty() {
                model_switchboard::catalog::Catalog::default()
            } else {
                model_switchboard::catalog::fetch_cached(
                    &merged.catalog_url,
                    &cache_path,
                    24 * 3600,
                )
                .await
            };
            catalog.apply_overlay(config::to_catalog(&merged));
            if dry_run {
                println!("dry-run: no bind, no keys, no network");
                println!("paths: {}", model_switchboard::gateway::paths().join(" "));
                println!("providers: {}", catalog.providers.len());
                if let Some(m) = model {
                    match model_switchboard::port::ModelRef::parse(&m) {
                        Ok(r) => match model_switchboard::gateway::resolve_base_url(&catalog, &r) {
                            Some(url) => println!("route {m} -> {url}"),
                            None => {
                                println!("route {m} -> unknown provider (no keys touched)")
                            }
                        },
                        Err(e) => println!("route {m} -> invalid ref: {e}"),
                    }
                }
                return Ok(());
            }
            let mut state = api::AppState::default();
            // Audit ledger serves GET /api/v1/audit; a failed open
            // serves [] rather than failing boot.
            if let Ok(ledger) = ledger_sentinel::ledger::Ledger::open(&data_dir).await {
                state.audit = Some(std::sync::Arc::new(tokio::sync::Mutex::new(ledger)));
            }
            // Boot: board.json from finished runs wins; else derive
            // live facts from the resume scan of stored sessions.
            let boot_facts = goal::load_facts(&data_dir);
            if !boot_facts.is_empty() {
                *state.facts.lock().unwrap() = boot_facts;
            } else if let Ok(store) = work_engine::store::Store::open(&format!(
                "sqlite://{data_dir}/db/harness.db?mode=rwc"
            ))
            .await
            {
                if let Ok(items) = resume::plan(&store, &format!("{data_dir}/work")).await {
                    let mut facts = state.facts.lock().unwrap();
                    for item in &items {
                        facts.push(board::CardFacts {
                            worker_id: item.session_id.clone(),
                            alive: item.workdir_exists,
                            blocked: if item.workdir_exists {
                                None
                            } else {
                                Some("worktree missing".into())
                            },
                            pr_open: false,
                            checks_green: true,
                            approved: false,
                            completed: false,
                        });
                    }
                    println!("resume: {} sessions", items.len());
                }
            }
            let app = model_switchboard::gateway::router(catalog).merge(api::router(state.clone()));
            // Forge observer: background tasks mirror PR facts the Kanban
            // derives from. Idle without configured forges, never fails boot.
            observer::spawn_forges(&merged.forges, &data_dir).await;
            let listener = tokio::net::TcpListener::bind(&bind).await?;
            match (lan_bind, lan_bearer) {
                (Some(lan), Some(bearer)) if !bearer.is_empty() => {
                    let lan_app =
                        api::router(state).layer(middleware::from_fn_with_state(bearer, lan_gate));
                    let lan_listener = tokio::net::TcpListener::bind(&lan).await?;
                    let (a, b) = tokio::join!(
                        axum::serve(listener, app),
                        axum::serve(lan_listener, lan_app)
                    );
                    a?;
                    b?;
                }
                (Some(_), _) => {
                    anyhow::bail!("--lan-bind requires a non-empty --lan-bearer");
                }
                (None, _) => {
                    axum::serve(listener, app).await?;
                }
            }
        }
        Cmd::Run { goal, repo, base } => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let data_dir =
                std::env::var("HARNESS_DATA_DIR").unwrap_or_else(|_| format!("{home}/.harness"));
            std::fs::create_dir_all(format!("{data_dir}/work"))?;
            std::fs::create_dir_all(format!("{data_dir}/db"))?;
            let global = config::load_layer(&format!("{home}/.harness/config.yaml"))?;
            let local = config::load_layer(".harness/config.yaml")?;
            let merged = config::merge(global, local);
            if merged.default_model.is_empty() {
                anyhow::bail!("no default_model in config (global or local)");
            }
            goal::run_goal(&merged, &goal, &repo, &base, &data_dir).await?;
        }
    }
    Ok(())
}
