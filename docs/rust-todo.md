# Rust TODO

These steps remain before the 44-item Rust SCARS goal is complete. The last
desktop attempt could not pass readiness because the Mac reported zero active
displays. Do not treat skipped or guard-stopped native checks as passes.

## Reproducible baseline

At checkpoint `0e346e2`, the headless suite passed all 10 gates and the Rust 1.88
minimum-version suite passed all 5 gates. Rerun them from the repository root
with:

```sh
python3 scripts/check_rust.py --suite headless --offline
python3 scripts/check_rust.py --suite msrv --offline
```

Reports are written to `rust/target/validation/headless/report.json` and
`rust/target/validation/msrv/report.json`. The MSRV run builds Skia from source;
if sandboxed Cargo cannot update a cached Skia git remote, rerun in an environment
where the existing Cargo cache is writable. Do not clear the shared Cargo/target
caches; the full source build is expensive.

## M8.5 — Native example smoke matrix

Run from the repository root on the logged-in Mac after waking and unlocking the
desktop. Keep Terminal in front and close other windows that may take focus.

```sh
python3 scripts/check_scars_desktop.py
python3 scripts/check_rust_examples.py --native --offline
```

The desktop probe must report `ready=1`. If it does, the example runner builds
the native-only windowing diagnostics and exercises the component examples in
both themes plus native sheet presentation. For the complete final native suite,
including the protocol, visible Metal/lifecycle, full-screen and secure-input
checks, run:

```sh
python3 scripts/check_rust.py --suite native --offline
```

The previous native suite passed its first 19 steps but stopped at the M8.5
foreground readiness guard. Rerun on the current checkout and retain the report
under `rust/target/validation/`; only a passing foreground example run closes
M8.5.

## M8.6 — Final inventory sign-off

After M8.5 and M5.5 pass, reconcile all 44 entries in
`rust-completion-checklist.md` with `rust-release-audit.md`, update stale status
claims and record the final commands/reports. Confirm no item is marked deferred
or open, run `git diff --check`, validate local documentation links, and commit
the final audit.

## M5.5 — Interactive VoiceOver and IME audit

This item was deferred at the user's request while M8 was being closed. It is
still in scope and must be completed before the goal can be marked complete.

- Run `m5_semantics` with VoiceOver. Recheck the tab announcement; the previous
  session reported “selected, collapsed tab 1 of 2.” The settings focus return
  check passed, but the example was overly verbose and the latest speech has not
  been verified.

  ```sh
  cd rust
  cargo run --example m5_semantics --locked --offline
  ```

- Audit navigation, editing and validation, password reveal, popup/sheet focus
  restoration, and actual IME candidate placement while moving, resizing and
  scrolling the window.
- Record macOS version, input source, display and actual backing scale. Cover
  both 1× and 2×. The earlier session was macOS 27.2 on a PA278CV at 1×; the
  composing input method and 2× display remain unknown/untested.
- Update the checklist and release audit only from observed VoiceOver speech and
  candidate-window placement. Protocol tests and synthetic raster scales are
  not substitutes.
