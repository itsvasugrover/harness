# Harness

Local-first agentic coding harness. One Rust daemon (`harnessd`) owns
all durable state, runs the agent loop, and serves one versioned HTTP
API. A Tauri desktop supervisor (Command Deck) and a Flutter phone
supervisor (Field Deck) watch and approve over that API. Your model
keys never leave your machine.

| Piece | What it is |
|---|---|
| `harnessd` | Daemon: agent loop, model gateway, board, forge sync, audit |
| Command Deck | Desktop supervisor: board, workers, audit, pairing settings |
| Field Deck | Phone supervisor: board, review inbox, audit, offline intents |
| Context Press | Prompt compressor in front of every model call |
| Shell Trim (`trim`) | Shell-output trimmer binary plus savings ledger |
| Forge Bridge | One GitHub/Gitea port behind capability leases |
| Ledger Sentinel | Audit log plus merge review gate |

Docs live in `docs/` (index below). Agents start at `AGENTS.md`;
humans start here and continue in `DEVELOPMENT.md`.

## Prerequisites

- Rust stable (`cargo build --workspace` must pass)
- Bun (Command Deck UI; `npm`/`npx` are not used)
- Dart + Flutter (Field Deck; Dart alone covers models and queue)

## Quickstart

```bash
# 1. Build everything (Rust workspace includes the Tauri sidecar)
cargo build --workspace

# 2. Configure (keys resolve from env, then the OS keychain — never files)
mkdir -p ~/.harness
cp config.example.yaml ~/.harness/config.yaml
$EDITOR ~/.harness/config.yaml   # providers, forges, model, writes

# 3. Preflight
./target/debug/harnessd doctor

# 4. Dry-run the model routing (no bind, no keys, no network)
./target/debug/harnessd serve --dry-run --model glm/glm-4

# 5. Serve the loopback API + gateway
./target/debug/harnessd serve
# API: http://127.0.0.1:4317  (board, audit, intents, events, /v1/*)

# 6. Run a goal (plans, spawns isolated workers, loops, archives)
./target/debug/harnessd run --repo your-org/your-repo "fix the login retry bug"
```

Trim a shell command and see the savings:

```bash
git log --oneline -50 | ./target/debug/trim run git-log
./target/debug/trim gain
```

## Supervise

**Desktop** (needs Bun):

```bash
cd apps/command-deck/ui
bun install
bun run dev        # http://localhost:1420, talks to 127.0.0.1:4317
```

Open Settings in the UI only if you changed the daemon bind address.
Loopback needs no token.

**Phone** (needs Flutter):

```bash
cd apps/field-deck/apps/field_deck
flutter pub get
flutter run
```

Pairing: start the daemon with a LAN listener,
`harnessd serve --lan-bind 0.0.0.0:4318 --lan-bearer <secret>`,
then paste host + bearer into the phone's Settings and use Test
(the identity probe shows the host id before any token is sent).
Approve a PR from the Review tab; it queues offline with an
idempotency id and syncs on reconnect. Every action lands in the
audit log with actor `phone`.

## API surface

All daemon state hangs off one versioned API (contract in
`openapi/openapi.yaml`, currently v2):

- `GET /api/v1/identity` — stable host id + contract (only open route)
- `GET /api/v1/board` — derived Kanban (workers + PRs, never stored)
- `GET /api/v1/audit` — append-only log (`actor`, `repo`, `kind`, `limit`)
- `POST /api/v1/intents` — idempotent approve/comment/retry intents
- `GET /api/v1/events` — live stream with `?cursor=` replay
- `/v1/*` — OpenAI-compatible gateway (BYO keys, local routing)

Display status is always derived at read time, never stored.

## Repo map

```text
crates/harnessd        daemon: CLI, API, loop, isolation, scheduler
crates/work-engine     agent loop, tools, sessions, guards, skills
crates/model-switchboard  providers, routing, gateway
crates/context-press   prompt compression + recall store
crates/shell-trim      trim CLI (binary: trim)
crates/forge-bridge    GitHub/Gitea adapters, leases, intents
crates/ledger-sentinel audit ledger, merge policy + gate
crates/recall-ledger   file memory, fingerprints
apps/command-deck      Tauri sidecar (Rust) + Vite React UI (bun)
apps/field-deck        Flutter supervisor + harness_ui kit
skills/                agent skills (forge-ops, failure-notes, review-gate)
```

## Docs

`docs/README.md` is the index: architecture, runtime, gateway,
economy, both decks, forge, audit, roadmap, and the new-agent
handover (`docs/handover.md`, kept current on every landing).
`docs/gitflow-conventions.md` governs branches and releases.

## Status

Built and green through Phase 5e (decks, intent replay, SSE, retry
recheck, session write scopes). Left: Tauri window bindings, phone
stream wiring, generated clients — each tracked as an issue.
