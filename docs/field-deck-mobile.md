# Field Deck (Flutter Android supervisor, v1)

Status: design spec for Phase 5. Live today: `harness_ui` kit barrel
+ `field_deck` app skeleton with pinned dependencies. No screens,
pairing flow, or offline queue yet.

Supervisor-only companion: watch the board, read diffs, approve/retry,
view audit. It never executes agents or tools.

## Stack (ported from flutter-kit, renamed)

- Pub workspace: `packages/harness_ui` (theme + widgets, zero
  third-party deps) + `apps/field_deck` (the phone app).
- Theme engine: seed-color `ThemeData` factories (light/dark),
  semantic tokens via `context.hx` (`success/warning/error/card/border/
  muted`), role-based text widget (`HxText`), persistent controller
  (`HxThemeController` + storage SPI). Hardcoded hex only in theme
  token files.
- Registry pattern for settings/support screens: each section is a
  self-registering entry; the shell never changes when screens are added.
- Widget prefix `Hx`, one file per widget (`hx_card.dart`, …).
- Verification contract per change (from `apps/field-deck`):
  `dart format packages/harness_ui apps/field_deck && flutter analyze && flutter test`.

Reused kit pieces (our names): theme controller, card/button/spacing/
shimmer/skeleton set, app shell with bottom nav, preferences storage
adapter (shared_preferences), CI shape (format+analyze+test+build).

## Screens (v1, five tabs max)

1. **Board** — same derived columns as desktop, pull-to-refresh + SSE.
2. **Worker** — status, last activity, checks, approve/retry buttons.
3. **Issues** — forge issue list, labels, assign, comment.
4. **Audit** — Sentinel verdicts + append-only log viewer.
5. **Settings** — host pairing, theme, text scale, bearer rotation.

No terminal emulator, no diff editor, no code execution on phone.
Diffs render read-only with a line cap + `recall` link to full text.

## Connectivity and offline

- LAN: bearer-paired WebSocket/SSE to `harnessd`; host discovery via
  `GET /api/v1/identity` (shows host id before pasting a token).
- Offline: last-known board/issues/audit cached in local SQLite/Hive;
  actions queue as intents and sync on reconnect (approve/retry/comment
  are idempotent with client-generated ids).
- Auth: per-phone bearer token, rotatable + revocable from desktop;
  failed-auth lockout with backoff; tokens in secure storage only.
- Battery/memory: SSE with resume cursor, paginated lists (20/page),
  image-free UI, no background polling faster than 15s, killed-app
  state restores from cache in <1s.
