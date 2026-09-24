# Harness — Documentation Index

Fully agentic, multi-model coding harness with a native desktop
supervisor and an Android supervisor companion.

## Locked decisions (from review)

| Decision | Choice |
|---|---|
| Model access | Both: local gateway server + zero-code local relay proxy, OpenAI-compatible |
| Desktop | Rust + Tauri (`apps/command-deck`) |
| Mobile | Flutter (`apps/field-deck`), supervisor-only v1, built on the flutter-kit patterns |
| Forge | GitHub + Gitea day-1 behind one Forge Bridge port |
| Mobile role v1 | Supervise, approve, retry, view audit. No on-phone agent execution |
| Naming rule | Everything is renamed and explained in our own words. Source inspirations are credited once here, never reused as component names |

## Terminology map (read once, then we use our names)

| Our name | What it is | Ported from (concept only) |
|---|---|---|
| Work Engine | opencode-style agent loop, tools, sessions, skills | opencode agent/session/tool loop |
| Model Switchboard | provider catalog + key router + `/v1/*` gateway | opencode provider catalog + transform |
| Local Relay | localhost proxy that compresses any OpenAI-compatible traffic | headroom proxy/wrap pattern |
| Context Press | content-type-aware prompt compressor with reversible cache | headroom pipeline (router + JSON/code/text compressors + cache aligner + reversible store) |
| Shell Trim | shell-output trimmer binary + auto-rewrite hook | rtk filter + hook + gain analytics |
| Command Deck | Tauri Rust desktop supervisor (Kanban, workers, PR/CI) | agent-orchestrator desktop shape |
| Field Deck | Flutter Android supervisor (same API, approve/retry) | agent-orchestrator mobile + flutter-kit UI kit |
| Forge Bridge | unified GitHub/Gitea port (repos, issues, PRs, CI) | gitea-mcp + tea CLI + orchestrator SCM ports |
| Ledger Sentinel | audit bot: review gate + append-only audit log + CI rules | AgentTeams audit + orchestrator PR observer + bug-triage skill |
| Recall Ledger | file-native memory (markdown + rebuildable index) | ReMe + repository-harness repo-as-truth |

## Docs

| File | Answers |
|---|---|
| `architecture-overview.md` | Whole system in one page, data flow, process map |
| `agent-runtime.md` | Work Engine loop, tools, sessions, skills, subagents |
| `model-switchboard.md` | Providers, OpenAI-compatible gateway + relay, caching |
| `context-economy.md` | Context Press + Shell Trim, savings math, recall |
| `command-deck-desktop.md` | Tauri desktop: daemon, isolation, Kanban, browser |
| `field-deck-mobile.md` | Flutter supervisor: kit reuse, screens, offline, LAN pairing |
| `forge-integration.md` | Forge Bridge: GitHub/Gitea repos, issues, PRs, runners |
| `ledger-sentinel-audit.md` | Audit bot rules, audit log API, CI gates |
| `modularity-memory-budget.md` | 200–300 line rule, module budgets, enforcement |
| `repo-layout.md` | Monorepo tree, what lives where, file-size contract |
| `gitflow-conventions.md` | Branches, commits, PRs, releases |
| `skills-agents-mcp.md` | Skills, agents, MCPs, commands, global vs local |
| `state-of-art-bets.md` | Eleven bets beyond reference parity + staging |
| `handover.md` | New-agent onboarding: state, next work, how to verify |
| `roadmap.md` | Build order MVP → v1, acceptance checks |

## Non-goals for v1

- On-phone agent execution (Field Deck is supervisor-only).
- Hosted cloud backend (everything runs on the user's machine; LAN sync only).
- Kubernetes multi-tenant orchestration (single-user local daemon first).
- Automatic model fine-tuning (only failure-note learning into repo docs).
