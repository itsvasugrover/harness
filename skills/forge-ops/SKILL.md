---
name: forge-ops
version: 0.1.0
scope: built-in
triggers: [issues, pr, review, github, gitea]
needs_tools: [forge]
permission: write
---

# Forge Ops (built-in skill)

Use the `forge` tool for repos, issues, PRs, checks. Never raw curl.
Reads default to allow; writes (comment, open PR, merge) require the
session permission scope, and merges additionally need a human approve
plus a Sentinel `pass`. Prefer local `.harness` forge defaults over
guessing URLs.
