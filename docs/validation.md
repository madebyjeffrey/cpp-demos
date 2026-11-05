# Validation

## Repeatable checks

Build both dependency configurations before configuring the corresponding presets, then run from a logged-in macOS desktop with Metal access:

```sh
cmake --build --preset relwithdebinfo
ctest --preset relwithdebinfo
cmake --build --preset debug
ctest --preset debug
```

The desktop/GPU integration test intentionally fails if Metal is unavailable; it does not silently substitute CPU rendering. A restricted execution sandbox may hide the GPU or WindowServer even on a capable Mac. CLI test runners can also remain background applications: the harness explicitly exposes its own windows so intentional occlusion suspension does not cause false timeouts. Do not cover the test canvases while running it.

The integration test exercises:

- Actual GPU-backed canvas rendering and physical-pixel readback of a known red rectangle.
- Drawable/logical size consistency, native resize and pointer/key event translation.
- GPU drawing in both modeless dialogs and modal sheets.
- Menu switching, native action/shortcut dispatch, enabled/checked state, invalid menu replacement and callback exception containment.
- Parent-modal command blocking, duplicate-sheet rejection, accepted/cancelled completion, and completion that destroys its own dialog.
- Close vetoes, minimize/restore, repeated animating-window destruction, parent/child cleanup, and posted work discarded on application destruction.

The C++ consumer target checks that dialogs cannot set menus at compile time and links using C++ source only. It can also be configured as a separate project:

```sh
cmake -S tests/consumer -B build/consumer -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build/consumer
ctest --test-dir build/consumer --output-on-failure
```

## Visual and GPU diagnostics

Normal drawing performs no pixel readback. An explicitly requested screenshot reads the GPU canvas after 30 frames and encodes a PNG. Smoke mode exits after those frames:

```sh
SKIA_WINDOW_SCREENSHOT="$PWD/build/gallery.png" \
  build/relwithdebinfo/skia_window_demo.app/Contents/MacOS/skia_window_demo --smoke-test
```

When launched directly by a background runner, bring the gallery forward if it is fully covered; rendering intentionally waits for visible content. For normal use, launch the `.app` through Finder, CLion, or `open`.

To capture the first presented frame for Xcode's Metal debugger, choose an absolute output path that does not already exist:

```sh
MTL_CAPTURE_ENABLED=1 SKIA_WINDOW_METAL_CAPTURE="$PWD/build/gallery.gputrace" \
  build/relwithdebinfo/skia_window_demo.app/Contents/MacOS/skia_window_demo
```

Capture is opt-in, reports setup failures through the error handler, and stops after the first frame's presentation completes. The capture-only path waits for that completion; normal rendering does not. Open the resulting `.gputrace` in Xcode to inspect the render encoders, shaders, drawable texture and `skia_window present` command buffer.

## Original gallery baseline

Validated October 29, 2026 on Apple M4 Pro, macOS 27.2, Xcode 27.1 / Apple Clang 21.0.0, and CMake 4.4.4:

| Check | Result |
| --- | --- |
| RelWithDebInfo library, gallery and tests | Built; 2/2 CTest tests passed |
| Debug library, gallery and tests | Built; 2/2 CTest tests passed |
| Separate C++ consumer project | Built; 1/1 CTest test passed |
| GPU pixel correctness | Known rectangle rendered and read back from a Metal canvas |
| Gallery visual review | All six cards rendered; native menus, shortcuts, pause/resume, modeless inspector, modal acceptance/cancellation and second-window creation exercised |
| Metal frame capture | Replayed in Xcode: 2 command buffers, 7 render encoders, 28 draw calls, 1 blit encoder; 1200×860 BGRA8Unorm drawable |

Local diagnostic artifacts are `build/validation/gallery.png` and `build/validation/gallery-full.gputrace`. The first frame includes lazy Skia shader/resource initialization; Xcode's late-object-creation performance suggestions in that startup capture are expected. The earlier `gallery.gputrace` captured presentation only and is superseded by `gallery-full.gputrace`.

Two test-harness details are intentional: test windows are ordered forward so a background runner does not leave them fully occluded, and minimize/restore assertions wait for native completion notifications. The capture starts before Ganesh context creation so its first command buffer is included.

Physical movement between Retina/non-Retina displays, native macOS 26 runtime execution, and Intel hardware remain unverified. These should not be inferred from the successful Apple Silicon/macOS 27 build. The deployment target is 26.0; the configured Intel build path has not been built or run locally.


## Declarative UI extension

The current build adds a retained declarative UI host, state/bindings, nested view identity, flex/grid/overlay/custom layout, scrolling, standard controls, native accessibility, and interruptible transitions. See [the UI guide](ui.md).

| Check | Current result |
| --- | --- |
| RelWithDebInfo library and both demo windows | Built |
| Debug library and both demo windows | Built |
| RelWithDebInfo full suite | 8/8 passed, including visible desktop, resize/performance regressions, Metal pixels, native actions and C++ consumer |
| Debug full suite | 8/8 passed: the same checks |
| Standalone C++ consumer | Built; 1/1 passed, including declarative state and controls |
| UI visual review | Offline previews reviewed at 1220 and 760 logical units, plus grid and collection panels |
| UI GPU rendering | Actual Metal offscreen surface; control bindings, opaque midpoint cross-fade pixels, final pixels, and return to idle checked |
| Native actions | Checkbox/slider/button accessibility roles/actions/value setting, consumed pointer input, parent-modal blocking, dialog bindings, failed mount recovery, and safe clearing after close checked |
| Visible desktop regression tests | Passed November 1, 2026, after the Mac became available |
| Hands-on Controls Lab inspection | Native Metal window, push panel replacement, conditional details, and slider bindings exercised; full VoiceOver and physical mixed-DPI review still pending |

Local artifacts: `build/validation/controls-wide.png`, `controls-narrow.png`, `controls-grid.png`, `controls-collection.png`, and `controls-ui.gputrace` (a Metal capture of the offscreen UI test). The PNGs are explicitly generated raster layout previews; the normal app renders through Metal. The UI capture was produced with GPU validation passing during the original extension work; interactive Xcode replay of that UI capture remains pending.

To reproduce the checked test subset with Metal/WindowServer access:

```sh
ctest --preset relwithdebinfo -LE desktop
ctest --preset debug -LE desktop
```

To rerun just the visible-desktop checks:

```sh
ctest --preset relwithdebinfo -L desktop
ctest --preset debug -L desktop
```

These cover the original native window lifecycle suite, UI frame/transition scheduling, and native resize presentation. They are not silently skipped or replaced by software rendering. Physical mixed-DPI displays, Intel hardware and macOS 26 runtime execution remain separate unverified environments.

An optional offscreen GPU capture can be recorded without a visible window:

```sh
MTL_CAPTURE_ENABLED=1 SKIA_WINDOW_UI_CAPTURE="$PWD/build/controls-ui.gputrace" \
  build/relwithdebinfo/skia_window_ui_gpu_tests
```

Use a new absolute output path for every capture.

## Resize and animation stutter investigation

Validated November 1, 2026 on the same Apple M4 Pro. The actual Controls Lab CPU diagnostic initially failed its 4 ms UI-work budget. A minimized nested-grid test also failed: five nested grids measured one leaf 157 times during one layout. A separate native test failed because resizing returned without drawing the matching content for that sizing transaction.

The fixes cache repeated measurement proposals within each layout pass, preserve layout across paint-only animation frames, avoid laying out both the old and new size for one resize event, and deduplicate native size notifications. Text wrapping now uses bulk glyph advances and a linear pass instead of repeatedly measuring growing prefixes; this was a major remaining cost in a sampled Debug profile. Size/bounds animations and scrolling still update layout. Native resize drawing uses transaction-synchronized Metal presentation; ordinary animation retains asynchronous presentation.

The same actual-demo diagnostic (150 frames per scenario, first 10 excluded) measured these 95th-percentile UI CPU times:

| Scenario | Original RelWithDebInfo | Fixed RelWithDebInfo |
| --- | ---: | ---: |
| Steady canvas animation | 2.028 ms | 0.047 ms |
| Resizing | 4.995 ms | 0.062 ms |
| Resizing with panel transitions | 8.382 ms | 0.115 ms |

After the layout fixes but before the text optimization, Debug measured 3.735 / 9.814 / 23.800 ms for those scenarios. After the text optimization it measured 1.236 / 1.479 / 3.042 ms. These measurements use a no-pixels canvas to isolate UI CPU work. They exclude GPU execution, submission and presentation and do not guarantee a particular end-to-end frame rate. Ordinary source builds, the standalone C++ consumer and the existing CLion Debug output were rebuilt.

`ctest --preset relwithdebinfo -R ui_performance` now reports 2 leaf measurements initially, 0 over 60 unchanged animation frames, and 2 after resize. State changes invalidate the cache; content-only transitions avoid layout. The UI suite additionally checks UTF-8 wrapping and explicit/trailing newlines at changing widths. Preview PNGs captured immediately before and after the text optimization compare byte-for-byte equal. `skia_window.resize` verifies an immediate GPU draw at each resized content size, synchronous presentation only during resizing, one resize notification per size, and continued animation in `NSEventTrackingRunLoopMode`.

The retained diagnostic entry point is `--benchmark-controls`; see [the UI guide](ui.md) for its interpretation and exit status. A sampled Debug profile and intermediate benchmark output are under `build/validation/controls-debug.*`. The profile includes sampling/startup interference in its maximum values, so the table uses percentile measurements, not those maxima.
