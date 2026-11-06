#!/usr/bin/env python3
"""Run every Cargo example in its declared headless or native mode."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
from uuid import uuid4

import check_rust as check
from rust_gallery import gallery

RASTER = frozenset("""lab m1_layout m1_scroll m2_editor m2_fields m2_numbers
m3_alerts m3_menu m3_overlay m3_split m3_tabs m3_wizard m4_combo m4_list
m4_list_edit m4_table m4_table_edit m4_tree m5_semantics m6_color m6_date
m6_datetime m6_drafts m6_formats m6_locales m6_time m6_zoned m7_inspector
stage2 stage3 stage4 stage5 stage6 todo""".split())
TERMINAL = {"foundation": (), "text_buffer": (),
            "m8_resources": ("--rows", "10000", "--visits", "32", "--edits", "1100")}
NATIVE_ONLY = frozenset(("stage1", "windows"))
VARIANTS = {
    "m3_alerts-sheet": ("m3_alerts", ("--sheet",)),
    "m6_locales-ar": ("m6_locales", ("--locale", "ar-EG")),
    "m7_inspector-box": ("m7_inspector", ("--box",)),
    "m7_inspector-flex": ("m7_inspector", ("--flex",)),
    "m7_inspector-popover": ("m7_inspector", ("--popover",)),
}
FIXTURE = "This is deliberately not valid todo data. Preview mode must ignore it.\n"


def discover(metadata: dict) -> set[str]:
    found = [target["name"] for package in metadata["packages"]
             for target in package["targets"] if "example" in target["kind"]]
    expected = RASTER | TERMINAL.keys() | NATIVE_ONLY
    if len(found) != len(set(found)) or set(found) != expected:
        raise ValueError(f"Update example modes: unclassified={sorted(set(found) - expected)}, missing={sorted(expected - set(found))}")
    return set(found)


def plan(names: set[str], target: Path, output: Path, native: bool, offline: bool):
    build = ("cargo", "build", "--workspace", "--examples", "--locked") + (("--offline",) if offline else ())
    steps = [check.Step("build-examples", build)]
    if native:
        steps.insert(0, check.Step("desktop-readiness", (sys.executable, str(check.ROOT / "scripts/check_scars_desktop.py")), check.ROOT, timeout=30))
    coverage, previews = {}, []
    fixture = output / "todo-saved-data.txt"
    for name in sorted(names):
        executable = str(target / "debug/examples" / name)
        if name in TERMINAL:
            coverage[name] = "not_run: headless-only" if native else "headless-terminal"
            if not native:
                steps.append(check.Step(name, (executable, *TERMINAL[name]), timeout=60))
        elif name in NATIVE_ONLY:
            coverage[name] = "native-smoke" if native else "not_run: native-only"
            if native:
                steps.append(check.Step(name, (executable, "--smoke"), timeout=30))
        else:
            coverage[name] = "native-smoke" if native else "cpu-raster"
    variants = {name: (name, ()) for name in sorted(RASTER)}
    variants.update({key: value for key, value in VARIANTS.items() if not native or key == "m3_alerts-sheet"})
    for label, (name, args) in variants.items():
        for theme in ("dark", "light"):
            for scale in ((1,) if native else (1, 2)):
                suffix = theme if native else f"{theme}-{scale}x"
                command = (str(target / "debug/examples" / name), *args)
                if theme == "light":
                    command += ("--light",)
                if name == "todo":
                    command += ("--data", str(fixture))
                if native:
                    command += ("--smoke",)
                else:
                    png = f"{label}-{suffix}.png"
                    previews.append(png)
                    command += ("--scale", str(scale), "--render", str(output / png))
                    if name == "todo":
                        command += ("--inspect",)
                steps.append(check.Step(f"{label}-{suffix}", command, timeout=30))
    steps.append(check.Step("example-artifacts", (sys.executable, str(Path(__file__).resolve()), "--verify", str(output)), check.ROOT, timeout=30))
    return steps, coverage, previews


def verify(output: Path) -> None:
    report = json.loads((output / "report.json").read_text())
    expected = set(report["expected_previews"])
    actual = {path.name for path in output.glob("*.png")}
    if actual != expected:
        raise ValueError(f"Preview mismatch: missing={sorted(expected - actual)}, unexpected={sorted(actual - expected)}")
    if (output / "todo-saved-data.txt").read_text() != FIXTURE:
        raise ValueError("Todo preview/smoke modified the saved-data fixture")
    if expected:
        for entry in report["steps"]:
            if entry["name"].startswith("todo-"):
                scene = Path(entry["log"]).read_text()
                if "Make room for the important things" not in scene or FIXTURE.strip() in scene:
                    raise ValueError("Todo render did not use isolated preview data")
        print(f"Verified {len(expected)} previews; indexed {gallery(output)} paired scenes")
    print("Saved todo fixture unchanged. Native-only skips remain explicit in the report.")


def main(argv=None) -> int:
    check.handle_termination()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--plan", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--verify", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    if args.verify:
        verify(args.verify)
        return 0
    if args.native and sys.platform != "darwin" and not args.plan:
        parser.error("Native example smoke checks require macOS")
    output = args.output or Path(os.environ.get("SCARS_EXAMPLE_ARTIFACTS", check.RUST / "target/validation/examples" / uuid4().hex[:12]))
    output = output.resolve()
    env = check.environment(output, "native" if args.native else "portable")
    command = ("cargo", "metadata", "--format-version", "1", "--no-deps", "--locked") + (("--offline",) if args.offline else ())
    metadata = json.loads(subprocess.check_output(command, cwd=check.RUST, env=env, text=True))
    names = discover(metadata)
    steps, coverage, previews = plan(names, Path(metadata["target_directory"]), output, args.native, args.offline)
    if args.plan:
        for step in steps:
            print(f"{step.name}: {shlex.join(step.command)}")
        print(f"Coverage: {coverage}\nPLAN ONLY: no examples were run.")
        return 0
    if output.exists() and any(output.iterdir()):
        parser.error("Use a fresh --output directory so old artifacts cannot supply missing previews")
    output.mkdir(parents=True, exist_ok=True)
    (output / "todo-saved-data.txt").write_text(FIXTURE)
    report = {"schema": 1, "suite": "examples-native" if args.native else "examples-headless",
              "started_at": check.now(), "revision": check.metadata(("git", "rev-parse", "HEAD")),
              "dirty": bool(check.metadata(("git", "status", "--porcelain"))),
              "coverage": coverage, "expected_previews": previews,
              "native_execution": "requested" if args.native else "not_run",
              "manual_audits": {"voiceover": "not_run", "ime_1x": "not_run", "ime_2x": "not_run"}}
    return check.run_steps(steps, output, env, report)


if __name__ == "__main__":
    raise SystemExit(main())
