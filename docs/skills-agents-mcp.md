# Skills, Agents, MCPs, Commands (global vs local)

Status: layering model + frontmatter schema + the five hardening rules
are specced and partly live (`SkillMeta` parse/gate, `CapabilityLease`,
`NoteProposal`, audit kinds). The runtime loader — skill index build,
trigger matching, `mcp.json` parsing/validation, command registry,
`help` aggregation — is live in `harnessd/src/skills.rs` and tested;
per-session run-path wiring (which lease each worker gets) rides with
the Phase 5 loop. Anything below describing runtime behavior beyond
the loader is the target shape.

One extensibility model across three primitives. Same shape in every
layer; local shadows global on name collision; session flags win.

## Layers (low → high precedence)

| Layer | Path | Owns |
|---|---|---|
| Built-in | shipped with binary | `build/plan/explore` agents, core tools, review-gate skill |
| Global (user) | `~/.harness/{agents,skills,mcp.json,commands,config.yaml}` | personal workflow, keys, preferred models |
| Local (project) | `<repo>/.harness/{agents,skills,mcp.json,commands,config.yaml}` + `<repo>/AGENTS.md` | repo conventions, forge defaults, required gates |
| Session | worker spawn args + CLI flags + env | one-off overrides, never persisted |

Merge rule: concat lists, dedupe by `name` with higher layer winning;
`tools.allow/deny` intersect down the stack (deny always wins);
secrets resolve at use time from keychain/env, never stored in files.

## Skills (progressive disclosure)

File: `<layer>/skills/<name>/SKILL.md` with frontmatter:

```yaml
---
name: forge-ops
version: 0.1.0
scope: global|local
triggers: [issues, pr, review, gitea, github]
needs_tools: [forge]
permission: read|write
---
```

Runtime loads only the index (`name + triggers + one-line`) into the
system prompt (~10 lines per skill). Full body loads on trigger match
or explicit `skill <name>` call. Bodies stay small (<150 lines each).

- Global skill example: `my-triage` (personal labeling habits).
- Local skill example: `repo-release` (this repo's changelog + gate steps).
- `failure-notes` skill writes corrections to the right layer:
  personal habit → `~/.harness/notes/`, repo rule → `.harness/notes/`
  (proposed, human approves; bot never silently rewrites policy).

## Agents

File: `<layer>/agents/<name>.yaml`:

```yaml
name: explore
kind: subagent
model: null            # null = inherit session model
tools: { mode: allowlist, list: [read, glob, grep, recall] }
max_steps: 25
permission: read-only
prompt_file: ./prompts/explore.md
```

Built-ins: `build` (default coder), `plan` (no writes), `explore`
(read-only fan-out), micro `title/summary/compaction`. Custom global
agents (e.g. `my-planner`) vs local agents (e.g. `repo-migrator` with
repo-specific paths). Subagent depth cap = 2; child gets scoped prompt
+ budget; only its report returns.

## MCP servers

File: `<layer>/mcp.json`:

```json
{
  "servers": {
    "forge": { "transport": "stdio", "command": "forge-mcp",
                "env_ref": ["FORGE_TOKEN"], "agents": ["build", "plan"],
                "tools": {"mode": "allowlist", "list": ["issue_*", "pr_*"] } },
    "recall": { "transport": "http", "url": "http://127.0.0.1:4317/mcp",
                "agents": ["*"] }
  }
}
```

- Daemon owns MCP processes (spawn, supervise, kill with session).
  Decks and workers never spawn servers directly.
- `env_ref` names resolve from keychain/env at spawn; literal secrets
  in `mcp.json` fail validation.
- Tool modes per agent: `allowlist|denylist|ask`. Write tools default
  to `ask` unless the skill declares `permission: write` + human
  approved the session scope.
- Sentinel logs every MCP tool call (`server.tool`, actor, verdict).

## Slash commands

File: `<layer>/commands/<name>.md` (prompt template + arg schema).
Global: `/triage`, `/summarize`; local: `/release`, `/migrate`.
Local shadows global; `help` lists merged set with layer tags
(`[global]`/`[local]`). Phone shows the same list read-only v1.

## Instructions layering (AGENTS.md)

System prompt assembles: built-in prompt → `~/.harness/AGENTS.md` →
`<repo>/AGENTS.md` → `.harness/instructions/*.md` → session extras.
Each source tagged so the model can cite which rule it followed.
Repo rules can tighten (deny tools, require gates) but never widen
global denies or exfiltrate keys.

## Adopted hardening (Capabilities, Attribution, Versions)

These five rules are load-bearing, not nice-to-haves:

1. **Capability leases (forge tokens never touch workers).** The daemon
   holds PATs in the OS keychain and mints a `CapabilityLease`
   (`forge-bridge/src/port.rs`): scoped (`pr.comment`, never raw
   scopes), 4-hour expiry, revocable. Workers present the lease;
   the daemon resolves credentials per call. Phone-queued actions use
   the same leases with idempotency ids.
2. **Attribution in every audit event.** `Attribution{actor, agent,
   skill?, mcp_server?}` rides on each `AuditEvent`
   (`ledger-sentinel/src/port.rs`). A bad merge is traceable to the
   exact skill + MCP server that advised it.
3. **Version-pinned skills.** Frontmatter carries `version` +
   `min_engine`; `SkillMeta::check_engine` (`work-engine/src/skill.rs`)
   refuses newer-than-engine skills loudly instead of half-running.
4. **Layer-aware failure notes.** `NoteProposal{target: global|local}`
   (`recall-ledger/src/port.rs`): personal habits → `~/.harness/notes/`,
   repo rules → `.harness/notes/`. Proposed by the skill, written only
   after human approve.
5. **Phone skill viewer (read-only).** Field Deck lists the merged
   global/local skill + command set with layer tags. No execution —
   the phone shows what the fleet knows, and which layer taught it.
