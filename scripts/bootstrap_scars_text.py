#!/usr/bin/env python3
"""Build SCARS' pinned SkParagraph/HarfBuzz/ICU stack without changing the existing Skia build."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from bootstrap_skia import ROOT, run, gn_value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=["arm64", "x86_64"], default=platform.machine())
    parser.add_argument("--config", choices=["Debug", "RelWithDebInfo"], default="RelWithDebInfo")
    parser.add_argument("--jobs", type=int, default=min(os.cpu_count() or 4, 12))
    args = parser.parse_args()
    checkout = ROOT / ".deps/skia"
    lock = json.loads((ROOT / "dependencies/skia.json").read_text())
    base = checkout / "out" / f"{args.arch}-{args.config}" / "skia-window-build.json"
    if not base.exists():
        parser.error(f"Bootstrap the existing Skia dependency first: python3 scripts/bootstrap_skia.py --arch {args.arch} --config {args.config}")
    manifest = json.loads(base.read_text())
    current = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=checkout, text=True).strip()
    if current != lock["revision"] or manifest["revision"] != current:
        parser.error("Skia checkout/build does not match the pinned revision")
    settings = dict(manifest["gn_args"], skia_enable_skparagraph=True, skia_enable_skshaper=True,
                    skia_enable_skunicode=True, skia_use_harfbuzz=True, skia_use_icu=True,
                    skia_use_system_harfbuzz=False, skia_use_system_icu=False,
                    skia_pdf_subset_harfbuzz=False, paragraph_tests_enabled=False,
                    paragraph_gms_enabled=False, paragraph_bench_enabled=False,
                    skunicode_tests_enabled=False)
    out = checkout / "out" / f"scars-text-{args.arch}-{args.config}"
    gn_args = " ".join(f"{key}={gn_value(value)}" for key, value in settings.items())
    run(checkout / "bin/gn", "gen", out, "--fail-on-unused-args", f"--args={gn_args}", cwd=checkout)
    run("ninja", "-C", out, f"-j{args.jobs}", "skparagraph", "skshaper", "skunicode", cwd=checkout)
    (out / "scars-text-build.json").write_text(json.dumps({
        "revision": current, "arch": args.arch, "config": args.config,
        "deployment_target": manifest["deployment_target"], "gn_args": settings,
    }, indent=2) + "\n")
    print(f"\nSCARS text stack ready: {out}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
