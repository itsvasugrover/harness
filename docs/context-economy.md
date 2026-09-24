# Context Economy (Context Press + Shell Trim)

Two layers, one rule: cut what the model reads, keep what it needs
retrievable, never break the provider cache.

Status: Press route + JSON/text/code crushers + align + file-backed
recall store + `recall` tool are live and tested; Trim ships 6 filters
+ rewrite/discover/gain CLI. Live-zone incremental crushing, image
routing, output steering, and the relay proxy are roadmap items, not
code — anything below describing them is the target shape.

## Layer 1 — Context Press (prompt compressor)

Pipeline per payload: `Align → Route → Crush → Store`.

- **Align (cache guard).** Flags volatile spans (timestamps, UUIDs,
  rapidly-changing counters) so they move out of the cached prefix. Never
  rewrites the user's words; only reorders volatile tails.
- **Route (type detector).** JSON arrays → structure crusher; source
  code → signatures crusher; prose/logs → text crusher. (Image routing
  is future work.)
- **Crushers.**
  - JSON: keep head + tail boundaries and error items; collapse the
    repetitive middle with counts.
  - Code: line-heuristic signatures-first with line numbers; small
    files pass through untouched. (AST-aware parsing is future work.)
  - Text/logs: collapse repeat runs with `×N` counts, keep
    ERROR/FATAL lines byte-identical.
- **Cache discipline.** Volatile lines (timestamps, UUIDs, hashes)
  ride the tail so the stable prefix stays cache-hot. Full live-zone
  incremental crushing (frozen prefix guaranteed byte-identical across
  turns) is future work.
- **Reversible store.** Originals persist file-backed with TTL;
  `recall <id>` restores them and the `recall` agent tool reads them.
  Crushers don't yet auto-insert placeholders — callers store + link
  explicitly (wiring is Phase 4).

Interfaces today: `press_block(text)` library call (classify → crush
→ stats), `trim run|gain|rewrite|discover` CLI, `recall` agent tool.
Relay proxy mode, MCP tools, and output shaping are roadmap items.

Expected effect (our targets, measured as input-token delta):
repetitive JSON/logs 50–90%, code exploration 30–45%, issue triage
25–35%, dense prose ~0–10% (returned byte-identical under a size floor).

## Layer 2 — Shell Trim (command-output trimmer)

Six filters today (more driven by `trim discover` data):

| Command | Trimmed shape |
|---|---|
| `tree` | Indented tree with per-dir counts |
| `read` | Blank-line-collapsed |
| `grep` | Grouped by file, 20/file cap, 300-char lines |
| `git status/diff/log` | Grouped stat, de-noised diff, one-line log |
| `test` (cargo/npm/pytest) | Failures only + collapsed-pass count |

Rules: filter-or-passthrough fallback (never break the command),
`<10ms` trim overhead target, no async runtime. Truncated tool output
carries a recall marker; the file-backed recall store holds originals
— no re-execution needed.

Hook: `trim rewrite` maps known commands to filters
(`git status` → `trim git-status`); unknown commands and excludes pass
through unchanged. Per-agent hook installers land in Phase 4.

## Savings math (honest)

We report **input-byte reduction per command/prompt** (`bytes/4`
estimate, labeled as such) plus provider-reported token usage per
session. We never claim "bill −90%": shell output is one input
contributor among system prompt + history, and input is one half of
(input + output) cost. `trim gain` reports totals from the JSONL
ledger today; per-command history graphs and session rollups are
roadmap items.
Telemetry (if any) is opt-in, aggregate counts only, no code/paths/keys.
