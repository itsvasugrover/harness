# Development guide (for humans)

How we build Harness without breaking it. Agents follow `AGENTS.md`;
this file is the same contract in human language, plus the workflow
around it.

## Principles (non-negotiable)

1. **Thin clients.** Daemon logic never moves into `apps/*` UI code.
   UIs render daemon facts and send commands. Nothing else.
2. **Derived display.** Kanban status is computed at read time from
   durable facts, never stored. If you want a new column, derive it.
3. **No secrets in repo.** Tokens resolve from env, then the OS
   keychain, at use time. `env_ref` in config, never values. No
   tokens in logs, issues, or committed files — CI scans for them.
4. **Small files.** Every code file stays at or under 300 lines
   (CI `size-guard` fails the PR otherwise). Split by responsibility:
   one job per file, one widget per file in Flutter (`Hx` prefix).
5. **Defense in depth.** Scope (leases) and verdicts (gates) stay
   independent: a capability never implies approval, and an approval
   never widens scope.

## Ownership

Each `crates/*` library owns one docs page. Cross-crate imports use
absolute crate paths through `lib.rs`-declared modules only — never
`super::` across crates, never file-relative hops. `harnessd` owns
scheduling and HTTP, never tool logic or UI rendering.

## Git flow

Short-lived branches from `develop` (`main` stays releasable):

| Prefix | Into | Example |
|---|---|---|
| `feature/` | `develop` | `feature/intent-replay` |
| `fix/` | `develop` | `fix/trim-git-log-overflow` |
| `docs/` | `develop` | `docs/core-readme-development` |
| `agent/` | `develop` | `agent/worker-42-retry-logic` |
| `chore/` | `develop` | `chore/rotate-ci-cache` |
| `release/` / `hotfix/` | `main` (+ back-merge) | `release/0.3.0` |

One issue per branch and PR. Rebase onto the target before review.
Squash-merge into `develop`.

Commits follow Conventional Commits with a crate scope:

```text
feat(harnessd): live event bus with SSE and cursor replay

<what + why, not how>
Refs: #32
```

Scopes: `work-engine, switchboard, press, trim, harnessd, deck,
field, bridge, sentinel, recall, api, ci, docs`. The PR title follows
the same convention **with a scope** (`feat(harnessd): ...`), and the
PR body uses plain `Problem:`, `Change:`, `Verification:`, `Risk:`
lines — the requirements check rejects scopeless titles and markdown
headers alike.

Required checks on every PR: `build`, `api-drift`, `size-guard`,
`secret-scan`, `pr-requirements`. A `block` verdict from Sentinel must
resolve before merge.

## Verification matrix

Run narrow first (`-p <crate>`), then the workspace before handing
over. Zero warnings tolerance.

| Area | Command | Proves |
|---|---|---|
| Rust | `cargo build --workspace` | everything compiles, incl. sidecar |
| Rust | `cargo test --workspace` | unit suites green |
| Rust | `cargo clippy --workspace --all-targets -- -D warnings` | zero lints |
| Rust | `cargo fmt --all -- --check` | formatting |
| Sizes | `./scripts/size-guard.sh` | every code file ≤ 300 lines |
| Web UI | `bun install && bun run build && bun run typecheck` (in `apps/command-deck/ui`) | bundle + types |
| Dart | `dart analyze <pure-dart files>` and `dart format` (in `apps/field-deck`) | analysis + format |
| Live | boot `harnessd serve` on a test port, curl identity/board/events, POST an intent | the HTTP contract |

Notes from the trenches: `npm`/`npx` are broken in some sandboxes
(bun runs everything: `bunx --bun` executes tools under bun).
`EventSource` sends no `Authorization` header, so bearer remotes poll
while loopback streams. Flutter widget files cannot be analyzed
without the Flutter SDK — keep widget code to long-stable Material
APIs and say so in the PR.

## Adding code

- **New agent tool** (`crates/work-engine/src/tools/`): one file,
  jail paths to the workdir, machine-readable errors for bad input,
  unit tests. Register in `tools.rs` + `registry.rs`. Network or
  writes need the approval gate (`needs_approval`).
- **New daemon endpoint**: types + handler live with their logic
  module (not all in `api.rs`), idempotency for anything the phone
  can retry, audit every settlement, OpenAPI schema updated,
  `curl`-proven in the PR body.
- **New skill**: `skills/<name>/SKILL.md` with frontmatter (`name`,
  `version`, `scope`, `triggers`, `needs_tools`, `permission`),
  body under 150 lines.
- **New UI screen**: thin fetch of a versioned route, loading +
  empty + error states, Lucide icons only, `prefers-reduced-motion`
  respected, light/dark contrast at 4.5:1.

## UI standards

Bun first, Tailwind plus shadcn-style primitives, one accent color
(≤10% of any screen), Inter for UI text with Space Grotesk display
and JetBrains Mono for ids and hashes. Product surfaces stay
restrained; the `ui-ux-pro-max` and aesthetic-web skills carry the
full system (palettes, motion, checklists) — load them before
designing.

## Config and data

Copy `config.example.yaml` to `~/.harness/config.yaml` (global) and
optionally `.harness/config.yaml` (local, shadows global; deny always
wins). All state lives under `~/.harness` (`HARNESS_DATA_DIR`
overrides for labs). New config keys must be `#[serde(default)]`
(safe upgrades), documented in the example, and covered by the
example-parity test. Unknown keys fail loud — that is intentional.

## Releases

SemVer with tags on `main` only. The phone pairing contract
(`GET /api/v1/identity`) bumps only on breaking API change. Desktop
and phone artifacts publish from the same tag through exactly one
publisher each.
