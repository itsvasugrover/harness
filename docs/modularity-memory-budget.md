# Modularity + Memory Budget (the 200–300 line rule)

## The contract

- No code file exceeds **300 lines**; **200** is the soft target.
- One file = one job (one loop stage, one tool, one filter, one screen).
- If a file approaches the cap, split by responsibility (never by
  arbitrary halves): `transform.rs` → `cache.rs + params.rs + variants.rs`;
  `git.rs` → `git_status.rs + git_diff.rs + git_log.rs`.
- Barrel files re-export only; no logic in barrels.
- New shared helpers require ≥2 real call sites (no speculative utils).

## Memory efficiency rules

- No async runtime in `shell-trim` or `context-press` hot paths;
  single-threaded, streaming, `<10ms` trim overhead target.
- No per-call compiled patterns (we carry no regex engine; heuristics
  are plain string scans).
- Estimate tokens as `bytes/4` where no tokenizer is vendored; label it
  `estimate` everywhere in UI and logs.
- Cap every list at the source (tool output capped at 2000 lines /
  51200 bytes with `recall` for the rest); never load-then-trim in
  memory beyond a 1MB read/grep guard.
- SQLite indexes rebuildable; markdown memory files are truth, indexes
  are cache (delete-and-rebuild is a supported operation).
- Phone app: paginated lists, cached board, SSE resume cursors, no
  images, no polling <15s.

## Enforcement (CI, not honor system)

- `size-guard` job fails the PR if any code file >300 lines (docs and
  generated files excluded via allowlist).
- `lints`: Rust `clippy -D warnings`, Dart `flutter analyze`, TS strict.
- Pre-commit: `fmt + lint + affected tests`. Full suite on PR.
- Reviewer agent checklist includes: file sizes, barrel-only barrels,
  no raw tokens in logs, no stored display status, idempotent writes.

## Splitting recipes

| Smell | Split into |
|---|---|
| `match` on 20 commands | one file per command group |
| giant `transform()` | `cache.rs`, `params.rs`, `variants.rs` |
| screen with 5 concerns | screen + 4 widget files |
| shared "util" grab-bag | delete; inline at call sites until 2nd use |
| test file >300 lines | `*_test.rs` per unit + fixtures dir |
