# SCARS native verification

The full native gates require a usable macOS foreground desktop with active displays, Metal, application activation, key-window ownership, and real display-link callbacks. Portable/raster tests and dedicated input/accessibility protocol tests cover different requirements and cannot replace these gates.

## Check desktop readiness

Run from the repository root in the logged-in desktop session:

```sh
python3 scripts/check_scars_desktop.py
```

This builds a small, temporary AppKit executable using the selected Xcode SDK. It opens a window for one second and reports active-display count, the CoreGraphics result, foreground bundle identifier, application/key-window state, and display-link callbacks. It uses neither SCARS nor Skia and removes its temporary executable afterward. Exit zero requires active displays, foreground/key ownership, and actual callbacks. A nonzero result is diagnostic evidence; it does not change or skip any CTest assertion.

If the foreground app is `com.apple.loginwindow`, no displays are active, or callbacks remain zero, wake/unlock the Mac and rerun the probe. If it is still not ready, run from a Terminal in the interactive desktop session and inspect the individual fields. A cached `NSScreen` or a window reported as visible does not establish that display-link callbacks or foreground input ownership are available.

## Run the gates

Build both configurations, then run them sequentially because they share desktop, clipboard, and secure-input state:

```sh
cmake --build --preset debug
cmake --build --preset relwithdebinfo
ctest --test-dir build/debug -R '^scars\.native(_actions|_foreground|_alerts|_datetime|_colors)?$' --output-on-failure
ctest --test-dir build/relwithdebinfo -R '^scars\.native(_actions|_foreground|_alerts|_datetime|_colors)?$' --output-on-failure
```

`native_foreground` launches the bundle through LaunchServices and requires explicit success markers for secure-input ownership and the redraw scheduler, as well as the full test's completion marker. A successful `open -W` command alone is insufficient. The dedicated color/date/alert suites verify their native protocol and lifecycle assertions; they do not establish foreground/frame completion.

## Verified completion — 2026-11-03

After the desktop was awake/unlocked, the standalone probe passed:

```text
SCARS desktop: active-displays=1 display-error=0 front=none app-active=1 key-window=1 visible=1 link-paused=0 callbacks=72 ready=1
```

The probe is an unbundled command-line app; a missing foreground bundle identifier does not invalidate successful application/key-window ownership and actual callbacks. Readiness uses the explicit state fields, not that identifier alone.

Both full builds and all 29 registered CTest tests pass in Debug and RelWithDebInfo. This includes 1,220 portable SCARS checks, all six SCARS native suites, and the existing UI/window regressions. Both LaunchServices foreground runs include these post-assertion markers:

```text
Secure-input ownership enabled and released with password focus.
Native one-shot redraw delivered the debounced search without continuous animation.
Secure-input ownership balanced across hide, unmount, and window close.
SCARS native scrolling, accessibility, inspector, and lifetime checks passed
```

All eight control phases meet the available-platform completion criteria. The [completion audit](scars-controls-verification.md) records phase coverage and compiler limits.

## Previous locked/asleep desktop diagnosis

Before wake/unlock, the broad suites passed their ListBox/TreeView/TableView protocols but failed scheduled search and frame checks. A standalone AppKit window, without SCARS or Skia, reproduced the failure:

```text
SCARS desktop: active-displays=0 display-error=0 front=com.apple.loginwindow app-active=0 key-window=0 visible=1 link-paused=0 callbacks=0 ready=0
```

The CoreGraphics query succeeded, establishing an actual zero-active-display result. Ordinary/legacy activation requests and delayed display-link attachment reproduced the same state. Wake/unlock restored native callbacks and allowed the original gates to pass without assertion changes. Retain the readiness probe and failure diagnostics to distinguish this environment condition from future runtime regressions.
