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
| Headless | `python3 scripts/check_rust.py` | Validator tests, inventory, workspace/consumer formatting, strict all-target Clippy, core/macro/raster/application tests, Linux core compilation, renamed-consumer compilation and example regressions |
| Portable | `python3 scripts/check_rust.py --suite portable` | Validator/inventory/format checks, core/macro Clippy and tests, Linux core compilation; no Skia/native dependency build |
| Minimum Rust | `python3 scripts/check_rust.py --suite msrv` | Selects Rust 1.88.0 explicitly; core/macro tests, dev/release workspace/all-target compilation with source Skia, Linux-core and renamed-consumer compilation |
| Native | `python3 scripts/check_rust.py --suite native` | Desktop readiness, actual visible Metal/lifecycle, full-screen, secure input, foreground sheets, every opt-in native protocol harness and native example smoke checks, run sequentially |

Append `--offline` when appropriate. `--plan` prints commands without executing
checks or producing a passing report. Native execution requires macOS 26+ and an
awake, unlocked logged-in desktop; the readiness probe must pass before any native
test starts. Native mode opens windows and enters/exits a full-screen Space.
It should be run after the headless build has finished, when the desktop is free.

Install the minimum toolchain before using `--suite msrv`:

```sh
rustup toolchain install 1.88.0 --profile minimal --target x86_64-unknown-linux-gnu
python3 scripts/check_rust.py --suite msrv --offline
```

That suite overrides the child toolchain even when the caller's default is newer;
its report reads Rust/Cargo metadata from the same child environment. Rust 1.88.0
passed 277 portable tests, dev/release all-target macOS/source-Skia compilation, Linux-core
compilation and the renamed consumer locally. Compilation does not certify native
window interaction. The [release audit](rust-release-audit.md) records this scope.

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
`example_artifacts` points to a separate fresh directory containing every
headless example preview, its paired gallery and a coverage/command report.
See [example regressions](rust-example-regressions.md) for modes and standalone
commands; native-only examples remain explicitly unrun in headless reports.

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

The [resource report](rust-resources.md) documents the `m8_resources` terminal
example, measured collection/editing sessions, cache ownership and the visible
idle-frame cases included in the native desktop suite. Those visible cases remain
unverified until that suite actually runs on an awake foreground desktop.

## CI

[The workflow](../.github/workflows/rust.yml) runs the same entry point on Ubuntu
24.04 for the portable core/macros. Its macOS 26 matrix runs full headless checks
on Rust 1.99.1 and minimum-version checks on Rust 1.88.0. Both toolchains passed
their local checks; checkout and
artifact-upload actions are pinned to release commits. These runner labels are
listed in GitHub's [runner-image inventory](https://github.com/actions/runner-images#available-images).
Action pins come from the official [checkout v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1)
and [upload-artifact v7.0.2](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) releases.

CI builds Skia from source, keeps no target/build cache, omits debug symbols to
reduce disk use and uploads reports, logs and Lab previews even after failure.
Hosted CI does not attempt foreground/native or interactive audits. The workflow
has been checked with `actionlint` and its commands run locally; a remote Actions
run is not claimed until the workflow is pushed and actually executes.
