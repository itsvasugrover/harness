# AGENTS.md — harness repo entrypoint

## What this repo is

Local-first agentic coding harness: Rust daemon + Tauri desktop +
Flutter phone supervisor. See `docs/README.md` for the doc index and
`docs/skills-agents-mcp.md` for the extensibility model.

## Commands (run from repo root)

```bash
cargo build --workspace        # all Rust members
cargo test --workspace         # all Rust tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
./scripts/size-guard.sh        # code files must be <= 300 lines
dart format apps/field-deck   # flutter members (when present)
flutter analyze --no-pub        # from apps/field-deck/apps/field_deck
```

Narrow first (`-p <crate>`), then workspace before handing over a PR.

## Boundaries (hard rules)

- Daemon logic never moves into `apps/*` UI code; UIs are thin API clients.
- Display status is derived at read time, never stored.
- No raw tokens/secrets in logs, issues, or committed files (`env_ref` only).
- One issue per branch/PR; branches from `develop` per `docs/gitflow-conventions.md`.
- Every code file ≤ 300 lines (CI `size-guard` enforces; split by responsibility).
- Global (`~/.harness`) vs local (`.harness/`) scoping per `docs/skills-agents-mcp.md`;
  local shadows global, deny always wins, session flags are ephemeral.

## Where to look

- `docs/architecture-overview.md` — process map + truth table
- `docs/repo-layout.md` — ownership per crate/app
- `docs/agent-runtime.md` — Work Engine loop/tools/sessions
- `docs/skills-agents-mcp.md` — skills/agents/MCP/command layering
- `docs/state-of-art-bets.md` — differentiating bets + staging
- `docs/handover.md` — current build state + next work (update on landing)
- `crates/*/` — one lib per docs page; cross-crate imports use absolute
  crate paths through `lib.rs`-declared modules only (never `super::`
  across crates, never file-relative hops).
