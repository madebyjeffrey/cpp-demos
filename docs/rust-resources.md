# Rust resource checks

From `rust/`, run the terminal example with real Skia layout and painting:

```sh
cargo run --example m8_resources --locked --offline
cargo run --example m8_resources --locked --offline -- --rows 100000 --visits 128 --edits 20000
```

The probe visits evenly spaced positions in separate list, table and tree models,
then repeatedly replaces an editor's text and paints 300 alternating images.
It checks viewport realization and cache sizes, then counts the mounted editor's
actual undo entries and verifies that redo restores the latest text. Models and
resources are constructed outside view builders. The smaller automated test uses
10,000 rows, 32 visits and 1,100 edits; it crosses both cache eviction limits.

To measure the executable's peak memory separately from compilation on macOS:

```sh
cargo build --example m8_resources --locked --offline
/usr/bin/time -l target/debug/examples/m8_resources --rows 100000 --visits 128 --edits 5000
/usr/bin/time -l target/debug/examples/m8_resources --rows 100000 --visits 128 --edits 20000
```

Measured locally on macOS 27.2, Apple silicon, Rust 1.99.1 and source-built Skia
(development Rust profile, optimized Skia), 2026-11-06:

| Measurement | 5,000 edits | 20,000 edits |
| --- | ---: | ---: |
| Source rows per model | 100,000 | 100,000 |
| Visits per collection | 128 | 128 |
| Peak list/table/tree mounted nodes | 29 / 38 / 47 | 29 / 38 / 47 |
| Peak paragraph/image cache entries | 512 / 128 | 512 / 128 |
| Mounted undo entries after editing | 100 | 100 |
| Cache entries after explicit clearing | 0 / 0 | 0 / 0 |
| Workload time | 3,400 ms | 6,487 ms |
| Process wall time, including initialization | 4.35 s | 6.49 s |
| Maximum resident set, bytes | 285,376,512 | 285,425,664 |
| Peak memory footprint, bytes | 217,465,672 | 218,776,416 |

Peak RSS differed by 48 KiB between these separate processes despite four times
as many edits. This workload shows bounded realization and retained cache/history
entry counts; it does not establish a universal memory ceiling. Source models
still own their rows. Cache entries and undo snapshots can contain large text or
images, and system allocators and Skia maintain their own caches. Timing and RSS
are observations, not flaky pass/fail thresholds.

## Cache ownership

`Renderer::cache_stats()` returns paragraph and image entry counts through
`RendererCacheStats`. The current limits are 512 and 128 entries. Cache eviction
does not discard a live host's layout. `Renderer::clear_caches()` releases the
renderer's cached ownership; current host layouts can retain paragraphs until
relayout or drop. A focused test holds a layout across clearing and proves that
the paragraph releases after the final layout drops, then verifies remeasurement.
Native unmount drops its renderer with the mount.

Text editing defaults to 100 undo snapshots; applications can set `history_limit`
through `TextOptions` or the field modifier. Large documents may need a smaller
limit. The lifetime test also checks the actual mounted editor's history, replaces
300 keyed editor lifetimes, removes the last editor and verifies runtime maps,
focus and callback resources retire. A stale editor cannot mutate the model.

## Work cancellation and idle behavior

The portable test repeats 50 hide/deactivate/remove cycles each with pending
search work, a held repeat button and indeterminate progress. After settling,
there are no deadlines or animation requests. Advancing synthetic time nine more
times per cycle produces zero invalidations and zero delayed application callbacks.
The existing tooltip, search, repeat and collection tests cover their individual
cancellation policies as well.

The inspected hidden-window AppKit lifecycle harness passed, including coalesced
notifications settling, cancellation on unmount and native proxies releasing
application callbacks. It runs without a visible Metal frame.

The foreground desktop suite now additionally prepares each of three real-window
cases: hide the component subtree, hide/deactivate its window, and unmount it.
Each starts with delivered animated Metal frames and pending search/repeat work,
then must settle with no new frames or callbacks during an idle observation.
Run it through the native validation entry point once the Mac is awake/unlocked:

```sh
# From the repository root:
python3 scripts/check_rust.py --suite native --offline
```

This new visible resource check is compiled but **has not run**: the current
desktop readiness probe reports the login window and zero active displays.
M8.4 stays open until its visible idle-frame checks pass. No process-wide OS
wakeup, interactive VoiceOver/IME or visible Metal pass is inferred from the
portable or hidden-window results.

Focused checks, from `rust/`:

```sh
cargo test --test resources --locked --offline -- --nocapture
cargo test -p scars-ui --lib resource_tests --locked --offline
cargo test --lib clearing_caches --locked --offline
```
