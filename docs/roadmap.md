# Roadmap (build order, MVP → v1)

## Phase 0 — Skeleton (week 1)

Scaffold monorepo, CI (`ci`, `size-guard`, `secret-scan`), `AGENTS.md`,
`config.example.yaml`, OpenAPI stub. Acceptance: fresh clone builds all
members; size-guard green.

## Phase 1 — Model Switchboard + Work Engine core (weeks 2–3)

Split for reviewability:

- [x] **1a (done):** catalog fetch + overlay, `ModelRef` parse, gateway
  resolve, tool trait + registry, loop-turn dispatch, 8 bet stubs.
- [x] **1b (done):** workdir-jailed `read/write/edit/glob/grep`,
  `bash` with timeout + trim caps, axum gateway serving `/v1/*` in `harnessd`.
- [x] **1c (done):** sqlx SQLite sessions/messages, transport-agnostic
  stream fold, session usage ledger, overflow check + compaction select.
  (Live SSE parsing + summarizer ride with Phase 2 adapters.)

## Phase 2 — Context economy + metering (weeks 4–5)

- [x] **2a (done):** Press route + JSON/text crushers, shared-cache stub,
  budget stub, Spend ledger stub, retrieval scorer stub.
- [x] **2b (done):** Trim filters (git/tests/files) + `trim` CLI with
  per-invocation meter, Press `pipeline.rs` (route+crush+stats).
- [x] **2c (done):** Code crusher (signatures-first), `trim rewrite`
  hook helper + `run`/`gain` CLI with JSONL ledger, gateway
  `route_model` decision + `UsageSink` (SQLite recording → Phase 3).
- [x] **2d (done):** `trim discover` (passthrough ranking by command
  head), cache-align pass in Press pipeline, `serve --dry-run`,
  file-backed recall store + `recall` agent tool (24h TTL).

Context Press (align+route+JSON/code/text crushers+recall store) and
Shell Trim (git, read/ls/grep, one test runner) + hook + `gain`
dashboard. Acceptance: measured input-byte reduction on a scripted
log-heavy task; full outputs recallable; overhead <10ms.

## Phase 3 — Isolation + supervision daemon (done, weeks 5–7)

- [x] **3a:** worker branch+worktree lifecycle (dirty refuses archive),
  checkpoint snapshot/revert, planner assignments, derived board
  columns (`+ Done`), `/api/v1/identity` live.
- [x] **3b:** run loop (`run.rs` sync + async, per-turn usage),
  `harnessd run` end to end with **parallel** units (JoinSet, tested),
  keychain fallback, models.dev base + overlay (224 providers live),
  LAN bearer + served board (verified live), boot resume + `board.json`
  facts, MCP spawn/supervise/stop + call attribution, `doctor`.

Acceptance (daemon-side): parallel workers run isolated units,
Kanban derives correctly (incl. Done), dirty worktrees never
force-deleted, providers fully config-driven.
Deck UI (Tauri board/worker detail/terminal, Flutter screens) is
Phase 5 — the daemon API it needs is already live.

## Phase 4 — Forge Bridge + Sentinel (done, weeks 7–8)

GitHub + Gitea adapters, issue/PR/checks observer, merge gate, audit
log + `GET /api/v1/audit`, CI triage/requirements workflows.
Acceptance: open→review→green→approve→merge flow on both forges from
the deck, every step in the ledger. Shipped as PRs #14–#17; run-path
lease wiring and deck screens ride with Phase 5.

## Phase 5 — Decks: Command Deck UI + Field Deck (weeks 9–10)

Tauri desktop (board, worker detail with chat/terminal/diff, planner)
over the live daemon API, plus the Flutter phone supervisor:
`harness_ui` kit + 5-tab app, LAN pairing, cached board, queued
approve/retry/comment with idempotency. Acceptance: phone pairs via
identity probe + bearer, approves a PR from cellular-off LAN, action
appears in ledger, no execution code ships in the app.

## Bet staging (docs/state-of-art-bets.md — tracked as stubs from Phase 1a)

| Bet | Stub location | Full build |
|---|---|---|
| Budget-aware routing | `model-switchboard/src/budget.rs` | Phase 2 (metering exists) |
| Shared context cache | `context-press/src/shared.rs` | Phase 2 |
| Session replay journal | `work-engine/src/replay.rs` | v1 hardening |
| Skill golden evals | `skills/*/eval.md` + `skill-evals.yml` | v1 hardening |
| Skill golden evals | `skills/*/eval.md` + `skill-evals.yml` | v1 hardening |
| Sentinel jury | `ledger-sentinel/src/jury.rs` | v1 hardening |
| Worktree checkpoint | `harnessd/src/checkpoint.rs` | Phase 3 (isolation exists) |
| Semantic tool retrieval | `work-engine/src/retrieve.rs` | Phase 2 |
| Compaction QA probe | `work-engine/src/compaction_qa.rs` | v1 hardening |
| Repo fingerprint sync | `recall-ledger/src/fingerprint.rs` | Phase 3 |
| Cost attribution | `model-switchboard/src/ledger.rs` | Phase 2 |
| Offline intent queue | `forge-bridge/src/intents.rs` + Field Deck model | Phase 4 |

## Phase 6 — Hardening to v1 (weeks 11–12)

Relay `wrap` for third-party CLIs, failure-note learning, updater +
signing, docs site, store/APK release. Acceptance: zero-code wrap of an
external coding CLI through the relay with savings visible; tagged
release publishes all three artifacts from one commit.
