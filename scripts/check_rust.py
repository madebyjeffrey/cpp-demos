#!/usr/bin/env python3
"""One entry point for portable, headless renderer, and explicit native checks."""
from __future__ import annotations

import argparse
from dataclasses import dataclass, field
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import shlex
import signal
import subprocess
import sys
import time
from typing import Optional
from uuid import uuid4

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "rust"
NATIVE_FLAGS = (
    "SCARS_WINDOW_DESKTOP_TESTS",
    "SCARS_WINDOW_FULLSCREEN_TESTS",
    "SCARS_WINDOW_SECURE_INPUT_TESTS",
)
PROTOCOL_TESTS = (
    "components", "accessibility_focus", "accessibility_lifecycle",
    "accessibility_notifications", "control_accessibility", "tree_accessibility",
    "table_accessibility", "date_picker", "time_picker", "datetime_picker",
    "zoned_picker", "color_picker", "localized_values", "inspector",
)


@dataclass(frozen=True)
class Step:
    name: str
    command: tuple[str, ...]
    cwd: Path = RUST
    env: dict[str, str] = field(default_factory=dict)
    timeout: float = 7200


def plan(suite: str, offline: bool) -> list[Step]:
    def cargo(name: str, *args: str, env: Optional[dict[str, str]] = None) -> Step:
        split = args.index("--") if "--" in args else len(args)
        command = ("cargo", *args[:split], "--locked")
        if offline:
            command += ("--offline",)
        return Step(name, command + args[split:], env=env or {})

    if suite == "native":
        desktop = {NATIVE_FLAGS[0]: "1"}
        return [
            Step("desktop-readiness", (sys.executable, str(ROOT / "scripts/check_scars_desktop.py")), ROOT, timeout=30),
            cargo("visible-metal-and-lifecycle", "test", "--test", "desktop", env=desktop),
            cargo("fullscreen", "test", "--test", "fullscreen", env={NATIVE_FLAGS[1]: "1"}),
            cargo("secure-input", "test", "--test", "secure_input", env={NATIVE_FLAGS[2]: "1"}),
            cargo("native-sheets", "test", "--test", "sheets", "--", "--foreground", env=desktop),
            *[cargo(f"native-{test}", "test", "--test", test, env=desktop) for test in PROTOCOL_TESTS],
        ]
    steps = [
        Step("validator-tests", (sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests", "-p", "test_check_rust.py"), ROOT),
        Step("inventory", (sys.executable, str(ROOT / "scripts/check_rust_inventory.py")), ROOT),
        Step("format", ("cargo", "fmt", "--all", "--", "--check")),
        Step("consumer-format", ("cargo", "fmt", "--manifest-path", "checks/renamed-consumer/Cargo.toml", "--", "--check")),
    ]
    packages = ("--workspace",) if suite == "headless" else ("-p", "scars-ui", "-p", "scars-ui-macros")
    steps.extend([
        cargo("clippy", "clippy", *packages, "--all-targets", "--", "-D", "warnings"),
        cargo("tests", "test", *packages),
        cargo("portable-compile", "check", "-p", "scars-ui", "--lib", "--target", "x86_64-unknown-linux-gnu"),
    ])
    if suite == "headless":
        steps.append(cargo("renamed-consumer", "check", "--manifest-path", "checks/renamed-consumer/Cargo.toml"))
        steps.append(Step("raster-gallery", (sys.executable, str(ROOT / "scripts/rust_gallery.py")), ROOT))
    return steps


def environment(output: Path, suite: str) -> dict[str, str]:
    env = os.environ.copy()
    for key in NATIVE_FLAGS:
        env.pop(key, None)
    env.pop("SCARS_LAB_ARTIFACTS", None)
    # Never download upstream's prebuilt Skia libraries. Cargo still reuses local builds.
    env["FORCE_SKIA_BUILD"] = "1"
    env["MACOSX_DEPLOYMENT_TARGET"] = "26.0"
    target = Path(env.get("CARGO_TARGET_DIR", RUST / "target"))
    env["CARGO_TARGET_DIR"] = str((RUST / target).resolve() if not target.is_absolute() else target)
    env["CARGO_TERM_COLOR"] = "never"
    if suite == "headless":
        # Keep old artifacts out of the current run without deleting user files.
        run = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid4().hex[:8]
        env["SCARS_LAB_ARTIFACTS"] = str(output / "lab" / run)
    return env


def _stop(process: subprocess.Popen) -> None:
    # Cargo starts test/rustc children; a timeout must retire that entire group.
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    else:
        process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        pass
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    elif process.poll() is None:
        process.kill()
    process.wait()


def execute(step: Step, env: dict[str, str], log: Path) -> int:
    with log.open("w", encoding="utf-8") as stream:
        stream.write(f"$ {shlex.join(step.command)}\n")
        stream.flush()
        try:
            process = subprocess.Popen(step.command, cwd=step.cwd, env={**env, **step.env},
                                       stdout=stream, stderr=subprocess.STDOUT,
                                       start_new_session=os.name == "posix")
        except OSError as error:
            stream.write(f"Cannot start check: {error}\n")
            return 127
        try:
            return process.wait(timeout=step.timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
            _stop(process)
            code = 124 if isinstance(error, subprocess.TimeoutExpired) else 130
            stream.write("Check timed out\n" if code == 124 else "Check interrupted\n")
            return code


def write_report(path: Path, report: dict) -> None:
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_steps(steps: list[Step], output: Path, env: dict[str, str], report: dict) -> int:
    output.mkdir(parents=True, exist_ok=True)
    entries = [{"name": step.name, "command": list(step.command), "cwd": str(step.cwd),
                "env": step.env, "status": "not_run"} for step in steps]
    report.update(status="running", steps=entries)
    path = output / "report.json"
    write_report(path, report)
    for index, (step, entry) in enumerate(zip(steps, entries), 1):
        log = output / f"{index:02d}-{step.name}.log"
        entry.update(status="running", log=str(log))
        write_report(path, report)
        print(f"[{index}/{len(steps)}] {step.name} · {log}", flush=True)
        start = time.monotonic()
        code = execute(step, env, log)
        entry.update(status="passed" if code == 0 else "failed", exit_code=code,
                     seconds=round(time.monotonic() - start, 3))
        write_report(path, report)
        if code:
            report.update(status="failed", finished_at=now())
            write_report(path, report)
            print(log.read_text(encoding="utf-8", errors="replace")[-6000:], file=sys.stderr)
            print(f"FAILED: {step.name}; remaining checks were not run. Report: {path}", file=sys.stderr)
            return 1
    report.update(status="passed", finished_at=now())
    write_report(path, report)
    print(f"PASS: {report['suite']} checks. Report: {path}")
    print("Actual VoiceOver speech and IME candidate placement are separate, unverified manual audits.")
    return 0


def now() -> str:
    return datetime.now(timezone.utc).isoformat()


def metadata(command: tuple[str, ...], cwd: Path = RUST) -> str:
    try:
        return subprocess.check_output(command, cwd=cwd, text=True, stderr=subprocess.STDOUT).strip()
    except (OSError, subprocess.CalledProcessError) as error:
        return f"Unavailable: {error}"


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=("headless", "portable", "native"), default="headless")
    parser.add_argument("--offline", action="store_true", help="Pass --offline to Cargo (requires cached sources/build prerequisites)")
    parser.add_argument("--output", type=Path, help="Log/report directory; default rust/target/validation/SUITE")
    parser.add_argument("--plan", action="store_true", help="Print commands without running or claiming checks passed")
    args = parser.parse_args(argv)
    if args.suite == "native" and sys.platform != "darwin" and not args.plan:
        parser.error("The native suite requires macOS 26+ and an awake logged-in desktop")
    output = (args.output or RUST / "target/validation" / args.suite).resolve()
    steps = plan(args.suite, args.offline)
    if args.plan:
        for step in steps:
            print(f"{step.name}: {shlex.join(step.command)}")
        print("PLAN ONLY: no checks were run.")
        return 0
    env = environment(output, args.suite)
    report = {
        "schema": 1, "suite": args.suite, "started_at": now(), "platform": platform.platform(),
        "revision": metadata(("git", "rev-parse", "HEAD")),
        "dirty": bool(metadata(("git", "status", "--porcelain"))),
        "rustc": metadata(("rustc", "-Vv")), "cargo": metadata(("cargo", "-V")),
        "source_build": {key: env[key] for key in ("FORCE_SKIA_BUILD", "MACOSX_DEPLOYMENT_TARGET", "CARGO_TARGET_DIR")},
        "build_profile": {key: env.get(key, "Cargo default") for key in ("CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG", "CARGO_BUILD_JOBS")},
        "native_execution": "requested" if args.suite == "native" else "not_run",
        "manual_audits": {"voiceover": "not_run", "ime_1x": "not_run", "ime_2x": "not_run"},
        "lab_artifacts": env.get("SCARS_LAB_ARTIFACTS"),
    }
    return run_steps(steps, output, env, report)


if __name__ == "__main__":
    raise SystemExit(main())
