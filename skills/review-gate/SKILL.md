---
name: review-gate
version: 0.1.0
scope: built-in
triggers: [merge, release, push main, approve]
needs_tools: [forge]
permission: read
---

# Review Gate (built-in skill)

Before merge/release: confirm green checks, resolved `block` threads,
Sentinel `pass`, and a human approve. Quote the exact blocker when
failing; never merge around a red gate.
