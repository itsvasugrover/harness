# State-of-the-Art Bets (beyond parity)

Parity with the references is the floor. These eleven bets are what
make the final application non-obvious. Each is small, testable, and
lands after its parent phase — none block v1.

1. **Budget-aware model routing.** Each task class (`explore`, `build`,
   `review`) gets a cost/latency SLO in config. The Switchboard picks
   the cheapest model that meets it and escalates on failure, with
   routing decisions ledger-logged so bills stay explainable.
2. **Content-addressed shared context.** Repo files hashed once per
   commit; all workers on the same commit share one cached Press
   output instead of each paying to read the same files.
3. **Deterministic session replay.** Every turn (prompt, tool I/O,
   model deltas) is journaled; `harness replay <session>` re-runs it
   offline for debugging. Time-travel beats log-diving.
4. **Skill golden evals.** Each skill ships 3–5 scripted scenarios run
   in CI. A skill that regresses fails the PR like a unit test —
   prompt engineering with a test suite.
5. **Sentinel jury.** High-risk merges (`migrate`, `auth`, `release`)
   get two cheap-model review votes before the human sees them. Split
   vote = auto-block with both rationales attached.
6. **One-key worktree checkpoint.** Snapshot before the first write
   tool; phone and desktop both get a Revert button that restores the
   snapshot and logs the rollback as an audit event.
7. **Semantic tool retrieval.** Tools/skills/MCP ops are embedded and
   top-k retrieved per turn instead of dumping 100 definitions into
   every prompt. Cuts input tokens and wrong-tool calls together.
8. **Compaction QA probe.** After every auto-compaction, a micro-model
   answers three questions from the summary (goal, blockers, next
   step). Fail → keep a wider tail and retry. Summaries that lose the
   plot get caught, not shipped.

Staging: 6 + 2 land with Phase 3 (isolation exists), 1 + 7 with
Phase 2 (metering exists), 3 + 4 + 5 + 8 harden toward v1.

9. **Repo fingerprint sync.** The daemon hashes `.harness/` +
   `AGENTS.md` per watched repo on every worker spawn and stores the
   fingerprint with the session. If the config changes mid-run, both
   decks show a "repo config changed" banner with the exact diff, and
   the worker pauses before its next tool call until the human accepts
   (re-prompt with new instructions) or pins the old fingerprint.
   Kills stale-instruction bugs where agents run on outdated rules.
   Stub: `recall-ledger/src/fingerprint.rs`. Full build: Phase 3.
10. **Cost attribution per worker.** Every Kanban card shows live spend
    (`input / output / cache_read / cache_write` + computed cost) from
    the session ledger, plus a per-worker budget. Either deck can flip
    a kill-switch that pauses the worker mid-stream; the pause and the
    spend that triggered it land in the audit log. Stub:
    `model-switchboard/src/ledger.rs`. Full build: Phase 2.
11. **Offline intent queue with conflict surfacing.** Phone actions
    (approve, retry, comment) queue locally with client-generated
    idempotency ids and replay on reconnect. If the world moved first
    (PR merged, session closed, config changed), the replay returns a
    conflict object and the phone surfaces an explicit choice —
    rebase the intent, drop it, or escalate — never a silent overwrite.
    Stub: `apps/field-deck` intent model (Phase 5); server replay +
    conflict types: `forge-bridge/src/intents.rs`. Full build: Phase 4.
