# Architecture Overview

## Implementation status

Target shape — daemon core (loop, tools, gateway, board, isolation)
is live per `handover.md`; lifecycles below describe the full design,
not just today's code. Unbuilt pieces (relay proxy, SSE push, forge
tools, audit JSONL writer, cache-marker injection) are labeled in
`roadmap.md` with owner phases. Nothing here promises what code lacks.

## One-paragraph picture

One local daemon (`harnessd`, Rust) owns all durable state (SQLite),
runs the Work Engine agent loop, serves the Model Switchboard gateway,
and exposes one versioned HTTP API. The Command Deck (Tauri desktop)
and Field Deck (Flutter Android) are thin supervisors over that API.
Context Press and Shell Trim sit in front of every model call and every
shell call to cut token spend. Forge Bridge syncs GitHub/Gitea.
Ledger Sentinel audits every consequential action.

## Process map

```
┌─────────────┐   ┌──────────────┐   ┌──────────────┐
│ Command Deck │   │  Field Deck  │   │  Any OpenAI  │
│ Tauri desktop│   │ Flutter phone│   │  -compat app │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │ REST+SSE (loopback) │ LAN bearer   │ /v1/*
       └──────────┬──────────┴───────┬──────┘
                  ▼                  ▼
        ┌─────────────────────────────────┐
        │            harnessd             │
        │ Switchboard │ Work Engine │ API │
        │ Press │ Trim │ Bridge │ Sentinel│
        │ SQLite (sessions, audit, cache) │
        └──────────────┬──────────────────┘
                       ▼
        ┌──────────────────────────────┐
        │ Providers (BYO keys, local — config only)  │
        │ GLM / DeepSeek / Muse Spark / any /v1       │
        └──────────────────────────────┘
                       ▼
        ┌──────────────────────────────┐
        │ GitHub + Gitea (Forge Bridge)│
        │ repos/issues/PRs/CI/runners  │
        └──────────────────────────────┘
```

## Request lifecycles

### A. Agent turn (Work Engine)

1. User prompt → session row + user message row (SQLite).
2. Work Engine loads recent messages, drops compacted outputs.
3. Context Press compresses tool outputs/logs/files (live zone only,
   frozen prefix byte-identical so provider KV-cache survives).
4. Model Switchboard picks provider/model, injects cache markers +
   session-scoped cache key, streams via AI-SDK-style adapter.
5. Tool calls execute (read/edit/bash/forge/etc), Shell Trim compacts
   shell output, full output saved to recall store on failure.
6. Finish → token/cost row, SSE event to both supervisors.
7. Overflow check → structured compaction turn, never silent drop.

### B. Third-party app via gateway

Any OpenAI-compatible client points at
`http://127.0.0.1:4317/v1` → Switchboard routes by `model` prefix
(`provider/model`), Press compresses, keys stay local.

### C. Zero-code existing CLI via relay

`harness relay wrap <tool>` starts Local Relay on a port and launches
the tool with `BASE_URL` env pointed at it. No code change in the tool.

### D. Supervision (desktop + phone)

Kanban never stores display status. It derives it at read time from
durable facts: session liveness, PR state, CI checks, review threads,
Sentinel verdicts. Phone sees the same facts over LAN with a bearer
token; exactly one unauthenticated route exists (`GET /api/v1/identity`
for host discovery).

## Crate / package map (each file ≤ 300 lines)

| Binary/lib | Owns | Never owns |
|---|---|---|
| `harnessd` | API, scheduling, worker spawn, worktree isolation | UI rendering |
| `work-engine` | agent loop, tool registry, compaction | provider HTTP |
| `model-switchboard` | catalog, routing, cache markers, gateway | prompt content |
| `context-press` | type router + compressors + reversible store | transport |
| `shell-trim` | command filters + hook + savings DB | agent policy |
| `forge-bridge` | GitHub/Gitea adapters behind one trait | UI |
| `ledger-sentinel` | audit log, review gates, CI rules | execution |
| `recall-ledger` | file memory + index + failure notes | model calls |
| `command-deck` | Tauri shell + web UI | daemon logic |
| `field-deck` | Flutter supervisor | execution |

## State truth table

| Fact | Source of truth |
|---|---|
| Sessions/messages/parts | SQLite in `~/.harness/` |
| Repo files | Git worktree per worker (daemon-managed) |
| Plans/decisions | Repo `docs/plans/active/` + `AGENTS.md` (repo is authority) |
| Audit trail | Append-only `audit/*.jsonl` + SQLite index |
| Memory | Markdown files; SQLite/FTS index rebuildable |
| Display status | Derived at read time, never stored |
