#!/usr/bin/env bash
# Fail if any code file exceeds 300 lines (docs + generated excluded).
set -euo pipefail
LIMIT=300
EXCLUDE='(docs/|\.github/|target/|build/|\.dart_tool/|generated/|openapi/.*generated)'
fail=0
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  FILES=$(git ls-files | grep -Ev "$EXCLUDE" | grep -E '\.(rs|dart|ts|tsx|py|go)$' || true)
else
  echo "size-guard: not a git repo, scanning filesystem" >&2
  FILES=$(find . -path ./target -prune -o -path ./build -prune -o -path ./.dart_tool -prune -o -type f \( -name '*.rs' -o -name '*.dart' -o -name '*.ts' -o -name '*.tsx' -o -name '*.py' -o -name '*.go' \) -print | grep -Ev "$EXCLUDE" || true)
fi
while IFS= read -r f; do
  [ -z "$f" ] && continue
  lines=$(wc -l < "$f")
  if [ "$lines" -gt "$LIMIT" ]; then
    echo "SIZE-GUARD FAIL: $f has $lines lines (limit $LIMIT)"
    fail=1
  fi
done <<< "$FILES"
[ "$fail" -eq 0 ] && echo "size-guard OK (all code files <= $LIMIT lines)"
exit "$fail"
