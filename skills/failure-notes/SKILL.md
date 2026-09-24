---
name: failure-notes
version: 0.1.0
scope: built-in
triggers: [failed, mistake, correction, learn, never again]
needs_tools: [read, write]
permission: write
---

# Failure Notes (built-in skill)

After a failed-then-fixed turn, propose one short note: what failed,
the correction, and the target layer (`~/.harness/notes/` for personal
habits, `.harness/notes/` for repo rules). Never write policy silently;
show the note and wait for human approve.
