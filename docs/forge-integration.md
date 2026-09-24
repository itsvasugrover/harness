# Forge Bridge (GitHub + Gitea, day-1 both)

Status: design spec for Phase 4. Live today: `Forge` trait,
`CapabilityLease`, offline `Intent` queue. Adapters, observer,
merge gate, and secret masking are not code yet — anything below
describing them is the target shape.

One trait, two adapters. The agent and both decks never touch
provider SDKs directly.

## Trait (Rust, ~80 lines)

```rust
// crates/forge-bridge/src/port.rs (sketch)
trait Forge {
  fn repos(&self, q: Search) -> Pages<Repo>;
  fn issues(&self, repo, filter) -> Pages<Issue>;
  fn issue_detail(&self, id) -> IssueFull;   // + comments, labels
  fn open_issue(&self, repo, NewIssue) -> Issue;
  fn comment(&self, id, body) -> Comment;
  fn pull(&self, id) -> PullFull;            // + checks + threads
  fn open_pull(&self, NewPull) -> Pull;
  fn merge(&self, id, method) -> MergeReport;
  fn checks(&self, sha: &str) -> Vec<Check>;
  fn request_review(&self, id, reviewers) -> Review;
}
```

Pagination is cursor-based; all writes are idempotent (client ids) so
phone-queued actions replay safely.

## Adapters

- **GitHub:** REST + GraphQL hybrid (search via REST, review threads via
  GraphQL), PAT from keychain/env, rate-limit backoff, webhook-optional
  (poll observer loop v1: 30s PR/checks/threads).
- **Gitea:** OpenAPI-driven client (self-host URL + token), same poll
  loop, `tea`-style CLI parity (`clone, issues, pulls, labels,
  releases, branches`) for scripts.
- Self-host runners: Gitea Actions-compatible queue supported; GitHub
  Actions observed read-only v1 (no custom runner binary v1).

## Observer (PR/CI/review facts for the Kanban)

A daemon task per watched repo polls PR metadata, check runs, review
threads, mergeability. Facts land in SQLite (`pr_facts`, `check_facts`,
`review_facts`); the Kanban derives columns from them. Failed CI /
requested-changes route back to the owning worker as a structured
follow-up (logs trimmed by Shell Trim, full log recallable).

## Permissions and safety

- Tokens: per-forge PAT in OS keychain, never logged, never sent to
  models. Workers receive scoped operation rights (e.g. `comment`,
  `open-pr`), never raw tokens.
- Merge requires: green checks + Sentinel pass + human approve (phone
  or desktop). Direct `main` push blocked by convention + hook.
- Secret masking in all logs (token-shaped strings redacted).
