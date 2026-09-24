# Handover (new-agent onboarding — read this first)

## What we are building

Local-first agentic coding harness: Rust daemon (`harnessd`) + Tauri
desktop supervisor (Command Deck) + Flutter Android supervisor
(Field Deck). Fully agentic, multi-model (OpenAI-compatible gateway +
relay proxy), token-frugal (Context Press + Shell Trim), GitHub+Gitea
day-1, audit bot (Ledger Sentinel). Our names for everything live in
`docs/README.md` under "Terminology map" — never reuse source-project
names (opencode, headroom, rtk, AO) for components.

## Locked decisions (do not relitigate without an ADR)

- Gateway + relay both ship; BYO keys, keys never leave the machine.
- Desktop = Tauri v2 stable (v3 alpha rejected). Mobile = Flutter,
  supervisor-only v1. Phone never executes agents.
- Kanban status derived at read time, never stored. All state under
  `~/.harness`. Secrets via `env_ref` only.
- Global (`~/.harness`) vs local (`.harness/`) layering: local shadows
  global, deny always wins (`docs/skills-agents-mcp.md`).
- Every code file ≤ 300 lines (CI `size-guard`); one job per file;
  cross-crate imports via `port.rs`/`lib.rs` only.

## Repo state (as of Phase 4 forge + sentinel done)

Built and green (`build`, `test`, `clippy -D warnings`, `fmt --check`,
`size-guard`, `secret-scan`, `api-drift`), CI enforced per PR
(requirements + triage), history via reviewed PRs #1–#17:

- `crates/work-engine`: `Tool` trait, `Registry`, `loop_turn` dispatch
  + `GuardedTurn` (permission gate + doom-loop guard, tested),
  `skill.rs` (frontmatter + `min_engine` gate), bet stubs (`replay`,
  `retrieve`, `compaction_qa`), `tools/` one-file-per-tool (jailed,
  tested: + `task`, `skill`, `todo`, `webfetch`), `session.rs`/`store.rs`
  (sqlx SQLite), `processor.rs` (event fold), `overflow.rs`
  (Ok/CompactSoon/Overflow + 40% soft cap), `compaction.rs` (select),
  `task.rs` (fan-out split), `approvals.rs`, `handover.rs` (successor
  delegation), `loop_turn.rs` (`gate()`).
- `crates/model-switchboard`: `ModelRef` parse, `catalog.rs` (fetch +
  overlay), `gateway.rs` (resolve + `route_model` + `UsageSink`),
  `budget.rs`, `ledger.rs` (Spend) stubs.
- `crates/context-press`: `PressStats`, `shared.rs` stub, `route.rs`,
  `align.rs` (cache-align), `crush_json.rs`, `crush_text.rs`,
  `crush_code.rs`, `pipeline.rs`, `store.rs` (recall, file-backed).
  Phase 2a–2d done.
- `crates/shell-trim`: `trim` CLI (`run` filters + meter, `gain`
  ledger, `rewrite` hook helper, `discover` ranking).
- `crates/forge-bridge`: `Forge` trait (+ `pulls` listing),
  `CapabilityLease` (scoped, expiring; workers never hold PATs),
  GitHub (REST + GraphQL threads) + Gitea adapters over shared
  `http.rs`/`parse.rs`, secret masking, offline `Intent` queue with
  replay + conflict surfacing.
- `crates/ledger-sentinel`: `AuditEvent` + `Attribution` + refs,
  `jury.rs`, JSONL + SQLite audit `ledger.rs`, merge `policy.rs` +
  `review_gate.rs` (pass/warn/block).
- `crates/harnessd`: previous daemon plus `forges:` config, observer
  (`observer.rs` + `forge_facts.rs`: PR/check/thread facts,
  follow-ups), audit ledger + `GET /api/v1/audit`, daemon forge
  execution (`forge_exec.rs` + `forge_ops.rs`), extension loader
  (`skills.rs`: skill index, triggers, `mcp.json` validation,
  commands, agents, `help`; MCP config checks in `mcp.rs`).
- `crates/recall-ledger`: `NoteProposal` + `NoteTarget` (global/local).
- `crates/harnessd`: clap CLI (`version`, `serve`, `--dry-run`,
  `doctor`, `run`), `workers.rs` (branch+worktree, dirty refuses
  archive), `checkpoint.rs` (snapshot/revert), `planner.rs`,
  `board.rs` (derived columns incl. `column_for_pr`), `api.rs`
  (persistent host_id, bearer gate, unified `{workers, prs}` board,
  contract v2, RwLock facts), `forge_gate.rs` (live merge gate over
  PR facts + `approved` flag), `observer.rs` (bounded concurrent repo
  polls), `run.rs` (sync + async loops, per-turn usage, guarded
  dispatch), `model.rs`, `keys.rs`, `mcp.rs`
  (spawn/supervise/stop + call attribution), `config.rs` (layers +
  models.dev base, zero hardcoded URLs), `doctor.rs`, `resume.rs`.
- `skills/`: `forge-ops`, `failure-notes`, `review-gate` + eval stub.
- `apps/`: Tauri sidecar/UI stubs, Flutter `harness_ui` + `field_deck`
  stubs (SDKs not resolved in this checkout — run `flutter pub get`).
- `docs/`: 14 files (index in `docs/README.md`), `config.example.yaml`,
  `mcp.example.json`, CI (`ci.yml`, `secret-scan.yml`, `skill-evals.yml`),
  `scripts/size-guard.sh`, `openapi/openapi.yaml` v0.2.0 (unified
  board + identity + audit schemas).

## Next work (in order)

1. **Phase 5 (now):** session write scopes (writes stay ungranted
    past `forge.read` until approval scopes land), Command Deck
    screens + Field Deck app over the live daemon API (unified board
    + persistent identity + merge gate with `approved` flag are live;
    still missing: per-repo tags on PR cards, SSE push, generated
    TS/Dart clients, all screens).
2. Then roadmap Phase 6 (hardening); bets attach to their staging
   phase. `work-engine` `forge` tool and `DaemonForge` are ready and
   waiting on the lease wiring.

## How to work here

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Narrow (`-p <crate>`) first, workspace before handover. Zero warnings
tolerance. One issue per branch/PR per `docs/gitflow-conventions.md`.
Update this file's "Repo state" + "Next work" when phases land.
