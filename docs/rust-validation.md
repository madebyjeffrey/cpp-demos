# Rust validation

From the repository root, run the headless validation entry point:

```sh
python3 scripts/check_rust.py --offline
```

Use `--offline` with cached Cargo sources and source-build prerequisites. Omit it
for a first build. Cargo's offline flag does not prevent Skia's own source builder
from fetching its sources; see the [build requirements](../rust/README.md#skia-source-builds).
Rustfmt, Clippy and the portable target must be installed:

```sh
rustup component add rustfmt clippy
rustup target add x86_64-unknown-linux-gnu
```

The runner uses the Rust workspace as the working directory, passes `--locked` to
build/test checks and forces source-built Skia. It respects `CARGO_TARGET_DIR` and
shares that directory with the standalone renamed-dependency consumer. The local
configuration and consumer profile preserve the same native source-build settings.

| Suite | Command | Scope |
| --- | --- | --- |
| Headless | `python3 scripts/check_rust.py` | Validator tests, inventory, workspace/consumer formatting, strict all-target Clippy, core/macro/raster/application tests, Linux core compilation and renamed-consumer compilation |
| Portable | `python3 scripts/check_rust.py --suite portable` | Validator/inventory/format checks, core/macro Clippy and tests, Linux core compilation; no Skia/native dependency build |
| Native | `python3 scripts/check_rust.py --suite native` | Desktop readiness, actual visible Metal/lifecycle, full-screen, secure input, foreground sheets and every opt-in native protocol harness, run sequentially |

Append `--offline` when appropriate. `--plan` prints commands without executing
checks or producing a passing report. Native execution requires macOS 26+ and an
awake, unlocked logged-in desktop; the readiness probe must pass before any native
test starts. Native mode opens windows and enters/exits a full-screen Space.
It should be run after the headless build has finished, when the desktop is free.

## Results

Logs and an atomic JSON report go to `rust/target/validation/SUITE/`. Choose another
directory with `--output PATH`. The report records the revision, dirty state,
platform, Rust/Cargo versions, source-build/profile environment, exact commands,
step exit codes, timings and log paths. The headless suite writes the Lab's
state/text/theme/size matrix and startup-presentation PNGs into a fresh directory
under `lab/` for each run. `lab_artifacts` in the report gives the exact path.
Open its `index.html` to filter paired 1×/2× CPU previews; the gallery step checks
that every pair exists and the 2× framebuffer has twice the width and height.
Older runs are retained separately and cannot supply missing current previews.

A failed command stops later checks. Unexecuted steps remain `not_run`; timeouts
and interruptions fail the check and terminate Cargo's process group, including
children. An interrupted report cannot become a passing report. Runner tests
exercise those boundaries using real child processes.

Headless/portable modes remove inherited native-test opt-ins before starting any
Cargo command. Ordinary workspace tests may print native harness skips; those are
not native passes. Native mode sets each harness's required opt-in explicitly.
A coverage test checks that every guarded native harness is represented.

A passing suite verifies only its stated scope. Reports always leave actual
VoiceOver speech and real IME candidate-window placement at actual 1×/2× backing
scales unverified. Those sessions, final visual/resource/example evidence and
inventory sign-off remain separate [completion requirements](rust-completion-checklist.md).
No script automatically checks off the completion report.

## CI

[The workflow](../.github/workflows/rust.yml) runs the same entry point on Ubuntu
24.04 for the portable core/macros and macOS 26 for the full headless renderer and
consumer. Rust is pinned to the locally verified 1.99.1 toolchain; checkout and
artifact-upload actions are pinned to release commits. These runner labels are
listed in GitHub's [runner-image inventory](https://github.com/actions/runner-images#available-images).
Action pins come from the official [checkout v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1)
and [upload-artifact v7.0.2](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) releases.

CI builds Skia from source, keeps no target/build cache, omits debug symbols to
reduce disk use and uploads reports, logs and Lab previews even after failure.
Hosted CI does not attempt foreground/native or interactive audits. The workflow
has been checked with `actionlint` and its commands run locally; a remote Actions
run is not claimed until the workflow is pushed and actually executes.
