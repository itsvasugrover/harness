# Ledger Sentinel (audit bot + review gate)

Status: event shape + attribution + stable kinds + jury tally are
live and tested. The JSONL writer, `/api/v1/audit` endpoint, policy
engine, and CI triage/requirements workflows are Phase 4 — anything
below describing them is the target shape.

Every consequential action is checked before it runs and recorded
after it runs. No silent merges, no phantom pushes.

## Two jobs

1. **Gate (before):** policy check on `merge`, `push`, `release`,
   `approve`, `write-outside-worktree`. Verdict: `pass|warn|block`
   with reasons.
2. **Ledger (after):** append-only JSONL audit event + SQLite index,
   served at `GET /api/v1/audit` with filters (actor, repo, kind, time).

Event = `{seq, time, actor(human|worker-id|bot), repo, kind,
summary, refs(pr/sha/issue), verdict, hash_prev}` — hash-chained so
tampering is detectable.

## Audited engine actions (stable `kind` strings, `port.rs::kinds`)

Reserved for the Phase 4 ledger writer — emitters don't exist yet,
so no event is dropped silently, none is written either:

| Kind | Will log when |

| Kind | Logged when |
|---|---|
| `gate.decision` | Every `gate()` verdict (`Proceed`/`Delegate` + usage) |
| `handover.export` | Handover file written (includes successor + reason) |
| `subagent.spawn` | Parent → child spawn with task scope + budget |
| `approval.decision` | Every approve/deny/once with tool + summary |
| `tool.exec` | Every tool execution (actor, agent, skill, MCP server) |

Attribution (`actor, agent, skill?, mcp_server?`) rides on each event,
so a bad merge traces to the exact skill + server that advised it.

## Default policies (v1, all editable YAML)

- No direct push to `main`/`develop`; changes via PR only.
- Merge needs: green checks + 1 human approve + Sentinel `pass`.
- Block on: failing checks, unresolved `block` review thread, secret
  pattern in diff (gitleaks-style), unknown binary blob >1MB.
- Warn on: large diff (>1000 lines), migration without rollback note,
  version bump without changelog entry.
- Bot accounts never file issues under a human's name; drafts require
  human submit (attribution rule).

## CI shape (repo's own workflows mirror the policy)

`.github/workflows/`: `ci` (fmt+lint+test+build per member),
`api-drift` (OpenAPI spec + generated clients fresh),
`size-guard` (code files ≤300 lines, see modularity doc),
`secret-scan`, `issue-triage` (label + dupe check), `pr-requirements`
(title/body/checklist/branch naming). Self-host mirror for Gitea
Actions with the same job names.

## Human loop

`Needs you` cards carry the exact blocker (failed check name, review
quote, Sentinel reason) plus a one-tap `Send back to worker` that posts
the failure as a structured tool result. Phone approvals are
first-class: same bearer auth, same idempotency ids, same ledger entry.
