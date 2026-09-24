# Work Engine (opencode-style agent runtime, our own code)

The autonomous coder. One loop, many tools, many models. Behavior
mirrors the proven opencode shape (prompt → loop → stream → tool →
repeat with compaction), reimplemented in Rust with our module limits.

## Implementation status (Phase 2 done — honest snapshot)

Real and tested: `Tool` trait + `Registry` + 7 tools (`read`, `write`,
`edit`, `bash`, `glob`, `grep`, `recall`), workdir jail, `gate()`
soft-cap delegation, handover export, task fan-out with depth cap,
approvals ledger-shape, SQLite sessions/messages/usage, event fold,
overflow levels, compaction select + QA probe, skill frontmatter gate,
`recall` store.
Deferred with owner phase: live model streaming + permission prompts +
doom-loop guard (Phase 3 loop), `task/skill/todo/webfetch` tools
(Phase 5), summarizer that writes compacted notes (Phase 3),
MCP-owned tool servers (Phase 4 loader live, run wiring pending).
The `forge` tool is live (lease-checked, read-only leases in the run
loop). Anything below describing the rest as live is the target
shape, not today's code.

## Loop (one file per stage, each < 300 lines)

```
prompt() → create user message → run_loop():
  while true:
    visible = load_messages().hide_compacted()
    last    = latest(visible)
    if assistant finished without tool calls → break
    if overflow(tokens, model) → compaction_turn(auto) → continue
    agent   = agents.get(last.agent)
    tools   = resolve_tools(agent, model)   # registry + MCP + permissions
    system  = [agent.prompt, env_info, repo_instructions, mcp_notes]
    stream  = switchboard.stream(system, messages, tools, model)
    outcome = processor.consume(stream)     # text|reasoning|tool|finish|error
    if outcome == stop    → break
    if outcome == compact → compaction_turn() → continue
```

Files: `loop.rs`, `prompt.rs`, `processor.rs`, `compaction.rs`,
`overflow.rs`, `messages.rs`, `system.rs`. No file owns two stages.

## Processor rules (target shape — Phase 3 loop)

- Permission check before every tool (`ask` → allow/deny/allow-once).
- First tool call snapshots the worktree (for revert).
- Doom-loop guard: same tool+args 3× → stop with explanation.
- Summary mode: no tools while writing the final summary.
- Tool errors return as tool results (model self-corrects), not panics.
- Live today: the last rule only. Approvals pause shape exists
  (`approvals.rs`); permission prompts, snapshots-per-tool,
  doom-loop, and summary mode arrive with the Phase 3 loop.

## Tool registry

Tool = `{ id, description, schema, execute(ctx) → {title, output, files?} }`.

| Tool | Notes |
|---|---|
| `read`, `write`, `edit` | Path-jail to worktree; edit is exact-match replace |
| `bash` | Cwd jail + 30s kill-timeout; Trim routing is Phase 4 |
| `glob`, `grep` | Grouped, truncated; full hits recallable |
| `recall` | File-backed restore with TTL |
| `task`, `skill`, `todo`, `webfetch` | Phase 5 tools (spec in `skills-agents-mcp.md`) |
| `forge` | Live: lease-checked reads via `builtins_with_forge` (writes await session scopes) |

Invalid args → machine-readable error fed back to the model, never a crash.

## Sessions (SQLite via sqlx)

`sessions(id, agent, model, input_tokens, output_tokens, cost_usd)`,
`messages(id, session_id, role, body, input_tokens, output_tokens)`.
Project/dir scoping, permission modes, parts tables, and compacted
markers arrive with the Phase 3 loop + Phase 4 audit tables.

- `hide_compacted()` drops compacted tool outputs from the model view.
- `latest()` returns `{user, assistant, finished, tasks}` cursors.
- Cursor pagination uses opaque base64 `{id,time}` tokens.

## Agents and skills

Agent = `{ name, kind: primary|subagent, model?, tools: allow|deny list, max_steps?, prompt?, permission? }`.
Ship four built-ins: `build` (default coder), `plan` (no writes),
`explore` (read-only fan-out), `title/summary/compaction` (micro-tasks).

Skills are progressive disclosure: `SKILL.md` frontmatter + body loaded
only when the task matches. Bundled: `forge-ops` (repos/issues/PRs),
`failure-notes` (write corrections to repo-local notes), `review-gate`
(Ledger Sentinel checklist).

## Subagents, approvals, and the 40% soft cap

- **Agents call agents.** Big goals split via `split_goal()` (`task.rs`)
  into smallest units — one non-empty scope per unit, each with a
  bounded step budget. A parent spawns one subagent per unit (`Spawn`
  record: parent → child + task); only the final report returns. Depth
  cap 2; children never spawn grandchildren.
- **Approvals.** Writes and out-of-scope tools pause on an `Approval`
  (`approvals.rs`): `Pending → Approved|Denied` (`Once` for
  single-use). Deny always wins. Every decision is audit-logged; the
  loop never executes past a pending approval.
- **40% is a soft cap, not a wall.** Every turn calls `gate()`
  (`loop_turn.rs`) over provider-reported totals against
  `usable = input_limit − max(reserved, output_max)` (`overflow.rs`):
  below 40% → `Proceed`; at/above → `Delegate`. Past the line the
  agent finishes its current unit, and the next time it would need
  fresh input the caller exports a handover (`handover.rs`: goal,
  decisions, touched files, next steps, usage, successor, reason) and
  the successor agent continues. Context never grows past usefulness,
  and no work is ever cut mid-unit.
- Compaction keeps a protected tail budget (newest turns byte-identical),
  summarizes the middle into one assistant note, marks pruned parts
  `compacted_at`. Prune only after the summary is durable.
