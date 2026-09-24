# Repo Layout (monorepo, modular by construction)

Target tree — skeleton files exist today, full modules land per
`roadmap.md` phase. If a path below is missing, its phase hasn't
landed; `handover.md` is the current-state authority.

```
harness/
├── docs/                        # this folder (product specs, ADRs)
├── AGENTS.md                    # agent entrypoint (commands, boundaries)
├── config.example.yaml          # models, providers, relay, trim, sentinel
│
├── crates/                      # Rust workspace (daemon + libs)
│   ├── harnessd/                # binary: CLI, API, loop, isolation
│   │   └── src/main,api,board,checkpoint,config,doctor,goal,keys,
│   │       mcp,model,planner,resume,run,workers.rs
│   ├── work-engine/             # loop_turn,processor,registry,tool,
│   │                           # tools/,session,store,skill,task,
│   │                           # approvals,handover,overflow,compaction,
│   │                           # replay,retrieve,receipts,jail,port
│   ├── model-switchboard/       # catalog,adapters/,gateway,budget,
│   │                           # ledger,port
│   ├── context-press/           # align,route,crush_{json,text,code},
│   │                           # pipeline,shared,store,port
│   ├── shell-trim/              # binary + cmds/{git,files,tests,
│   │                           # tracking} + hook.rs
│   ├── forge-bridge/            # port.rs (trait+lease) + intents.rs
│   ├── ledger-sentinel/         # port.rs (events+kinds) + jury.rs
│   └── recall-ledger/           # port.rs (notes) + fingerprint.rs
│
├── apps/
│   ├── command-deck/            # Tauri desktop
│   │   ├── src-tauri/           # sidecar: daemon lifecycle, PTY, updater
│   │   └── ui/                  # Vite React: board,worker,planner,forge,audit
│   └── field-deck/              # Flutter phone supervisor
│       ├── packages/harness_ui/ # theme (hx_*), widgets (one file each)
│       └── apps/field_deck/     # screens: board,worker,issues,audit,settings
│
├── skills/                      # progressive-disclosure agent skills
│   ├── forge-ops/SKILL.md
│   ├── failure-notes/SKILL.md
│   └── review-gate/SKILL.md
│
├── .github/workflows/           # ci, api-drift, size-guard,
│                                # secret-scan, skill-evals
└── openapi/                     # openapi.yaml stub (clients: Phase 5)
```

## Ownership rules

- `harnessd` owns scheduling + HTTP; never tool logic or UI.
- Each `crates/*` lib owns one docs page; cross-crate imports use
  absolute crate paths through `lib.rs`-declared modules only (never
  `super::` across crates, never file-relative hops).
- `command-deck/ui` and `field-deck` own rendering only; shared shapes
  come from `openapi/` generated clients.
- `skills/` own agent behavior text; code owns enforcement.
- Generated code is committed; generators run in CI with drift
  failure. (SQLite schema is inline `CREATE TABLE IF NOT EXISTS`
  today; file migrations arrive with the audit tables in Phase 4.)

## File-size contract

CI `size-guard` enforces ≤300 lines per code file. Approved splits are
already sketched in each doc (e.g. one file per shell command group,
one file per agent-loop stage, one widget per file with `Hx`/`hx_`
prefix in Flutter, `Fk`-style tokens renamed to `Hx`).
