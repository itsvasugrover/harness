# Command Deck (Tauri Rust desktop supervisor)

Status: design spec for Phase 5. Live today: sidecar + UI stubs and
the daemon API they will consume (board, identity, gateway). No
screens, PTY, or updater code yet.

Native, small, offline-first. The desktop never runs agent logic; it
renders daemon facts and sends commands.

## Stack

- **Shell:** Tauri **v2 stable** (v3 is alpha as of 2026-09 — rejected
  for v1; re-evaluate yearly) + Rust sidecar manager (owns `harnessd`
  lifecycle: start/stop, `~/.harness` data dir, single-instance lock,
  log tail).
- **UI:** Vite + React + TypeScript, generated API client from the
  daemon's OpenAPI spec (drift-checked in CI). No daemon logic in UI code.
- **Terminal:** xterm + per-worker PTY multiplexer over WebSocket.
- **Updates:** Tauri updater with exactly one release publisher (rule
  learned from reference apps: divergent artifacts are undebuggable).

Why Tauri over a web wrapper: 5–15 MB bundle, OS webview, Rust sidecar
for worktree/PTY/Docker flows that Flutter desktop handles poorly.

## Daemon boundary (hard rules)

- Loopback API binds `127.0.0.1` only, unauthenticated (local user owns
  the machine). No change without an ADR.
- Optional LAN listener binds `0.0.0.0` only while the user enables
  "Connect Phone", behind a bearer token, serving app routes but never
  control routes (`/shutdown`, telemetry). One exempt route:
  `GET /api/v1/identity` (opaque host id + contract version, GET only).
- CLI (`harness` command) is a thin HTTP client; it never opens SQLite
  or spawns runtimes directly.
- All app state under `~/.harness` only (`AO_DATA_DIR`-style override
  `HARNESS_DATA_DIR` for labs). Never OS app-data paths.

## Workers and isolation

Worker = 1 task + 1 agent + 1 isolated workspace.

- Git-backed work → own branch + linked worktree
  (`worker/<id>-<slug>`), registered, never force-deleted while dirty.
- Scratch work → branchless dir under `~/.harness/work/`.
- Planner (project-level agent) breaks plans into workers, passes scoped
  context, follows progress, coordinates follow-ups. Workers own
  implementation/tests/commits/PRs; planner owns sequencing.

## Kanban (derived, never stored)

Columns derive at read time: `Working` (alive session, no blocker),
`Needs you` (blocked/failed CI/requested-changes/lost heartbeat),
`In review` (open PR awaiting checks/review), `Ready to merge`
(approved + mergeable). Cards show task, agent, branch, activity, PR,
checks. Opening a card reveals conversation, terminal, diff, reviews,
and an isolated browser preview (per-worker profile, CDP).

## Screens (v1)

Board · Worker detail (chat|terminal|files|PR|checks) · Planner ·
Forge (repos/issues) · Audit (Sentinel verdicts + log) · Settings
(keys, models, relay, trim levels, phone pairing).
