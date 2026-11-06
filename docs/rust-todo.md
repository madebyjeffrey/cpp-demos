# Rust TODO

Items deliberately deferred from the current M8 release close-out.

## M5.5 — Interactive VoiceOver and IME audit

**Status:** incomplete; deferred by user request on 2026-11-06. Do not count as
passed. The M8 close-out may finish independently.

- Repeat the `m5_semantics` tab announcement check with the latest build. The
  earlier session passed settings focus restoration, but reported “selected,
  collapsed tab 1 of 2”; the protocol fix is not proof of actual speech.
- Audit navigation, editing/validation/password reveal, popup and sheet focus
  restoration, and actual IME candidate placement while moving, resizing and
  scrolling.
- Record macOS version, input source and actual backing scale for each session;
  cover both 1× and 2×. The prior assisted session was macOS 27.2 on a PA278CV at
  1×; the composing input method and 2× display were not identified or tested.
- Update the M5.5 row in `rust-completion-checklist.md` and the release audit only
  after recording actual observations. Hidden protocol checks and synthetic raster
  scales do not close this item.
