# Git Flow Conventions (branches, commits, PRs, releases)

Git Flow with short-lived branches. `main` always releasable,
`develop` integration, work happens on prefixed branches.

## Branches

| Prefix | From | Into | Example |
|---|---|---|---|
| `feature/` | `develop` | `develop` | `feature/forge-gitea-observer` |
| `fix/` | `develop` (or `main` for hotfix via `hotfix/`) | `develop` | `fix/trim-git-log-overflow` |
| `agent/` | `develop` | `develop` | `agent/worker-42-retry-logic` |
| `release/` | `develop` | `main` + back-merge `develop` | `release/0.3.0` |
| `hotfix/` | `main` | `main` + back-merge `develop` | `hotfix/relay-token-leak` |

Rules: one issue per branch/PR, rebase onto target before review,
worker branches named `worker/<id>-<slug>` and deleted after merge
(daemon refuses to force-delete dirty worktrees).

## Commits (Conventional Commits, enforced by review; commit-lint
CI lands in Phase 6)

```
<type>(<scope>): <short imperative summary>

<body: what + why, not how>
Refs: #<issue>
```

Types: `feat|fix|docs|refactor|test|chore|perf|ci|build`.
Scopes are crate/app names: `work-engine|switchboard|press|trim|
harnessd|deck|field|bridge|sentinel|recall|api|ci|docs`.

Good: `feat(bridge): add gitea check polling with backoff`
Bad: `fixed stuff`, `WIP`, `update`.

## PRs

- Title follows commit convention; body uses the repo template
  (problem, change, verification, risk, refs).
- Required checks: `ci`, `api-drift`, `size-guard`, `secret-scan`.
- Sentinel adds a `pass|warn|block` verdict comment; `block` must be
  resolved before merge.
- Squash-merge to `develop`; `release/*` merged with `--no-ff` + tag.

## Releases and versioning

SemVer. Tags `vX.Y.Z` on `main` only, changelog generated from
conventional commits. Desktop (Tauri updater) and phone (store/APK)
publish from the same tag; exactly one publisher per artifact runs the
publish job. Phone pairing contract version bumps only on breaking API
change (surfaced via `GET /api/v1/identity`).
