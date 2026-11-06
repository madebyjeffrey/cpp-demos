# Rust example regression checks

From the repository root:

```sh
python3 scripts/check_rust.py --offline                 # full headless validation
python3 scripts/check_rust_examples.py --offline        # examples only
python3 scripts/check_rust_examples.py --native --offline
```

Omit `--offline` for an initial build with uncached sources. The example runner
builds the workspace examples and reads Cargo metadata to require an explicit
mode for every example. Adding, removing or duplicating a target without updating
the runner fails the check. `--plan` prints commands and coverage without launching
examples or changing the output directory.

| Examples | Headless coverage | Foreground coverage |
| --- | --- | --- |
| Portable `foundation`, `text_buffer` | Execute their terminal demonstrations | Already covered by headless mode |
| `m8_resources` | Execute the resource workload with 10,000 rows and 1,100 edits | Already covered by headless mode |
| 34 renderer examples: Lab, Todo, stages 2–6 and M1–M7 examples | CPU Skia previews in dark/light at 1×/2× | Run each in dark/light with `--smoke` |
| `stage1`, `windows` | Explicitly `not_run: native-only` | Run each with `--smoke` |

Headless checks additionally run sheet content, Arabic localization and the
inspector's box/flex/popover variants in both themes and densities. That produces
156 PNGs, grouped as 78 pairs. The last step requires every expected file, checks
paired framebuffer dimensions and writes a searchable `index.html`.

The full entry point records the fresh output path as `example_artifacts` in its
report. Each example run has its own `report.json` with coverage and per-command
logs, status, exit codes and timings. Old output cannot satisfy a current check;
standalone `--output PATH` requires a fresh directory. Nested timeout/interruption
handling retires the example runner's active child process group. A regression
test checks that cancellation cannot leave a separate child group running later.

## Todo behavior

Run the actual application with sample data:

```sh
cd rust
cargo run --example todo --locked --offline -- --preview --light
cargo run --example todo --locked --offline -- --light --scale 2 --render /tmp/todo-2x.png
cargo test --test todo_model --locked --offline
```

Seven focused tests cover adding, editing, cancellation, completion, filters,
clearing, stable keys, stale bindings, Unicode storage, atomic save/reload and
recovery from an unavailable storage directory. The mounted view is exercised in
both themes: external input and item-state updates reach existing controls,
unchanged keys retain identity, filter controls affect visible rows, and Save and
Clear completed persist their actual results.

The external-item regression reproduced duplicate keys (`[1, 1]`) when adding
after supplying an item through state. Allocation now advances beyond both the
monotonic counter and existing item IDs, preserving unique keys and preventing
deleted completion bindings from affecting later tasks.

Visual review also found the inspector example's transparent popover overlapping
content beneath it. Its body now uses a themed group box, padding and an explicit
280-point width. The example still uses ordinary declarative component composition.

Preview, render and smoke modes use sample data. The regression runner supplies a
deliberately invalid saved-data fixture: the renders must succeed, their inspected
scenes must contain sample tasks, and the fixture must remain unchanged. Normal
application storage remains described in the [quick guide](rust-guide.md).

## Remaining native evidence

The native runner first requires the awake/foreground desktop probe. It runs the
two windowing diagnostics, all component examples in both themes, and actual
sheet presentation, then verifies the saved-data fixture remains unchanged.
The main native validation suite includes these example checks after its protocol,
Metal/lifecycle and full-screen checks.

Native smoke and interactive behavior are separate claims: a timed smoke exit
does not certify speech, candidate-window placement or every gesture. Those
remain assigned to their dedicated audits. M8.5 remains open until the native
example run actually passes; a locked desktop and headless `not_run` entries
cannot count as completion.
