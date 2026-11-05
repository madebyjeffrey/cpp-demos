#!/usr/bin/env python3
"""Diagnose the foreground desktop required by SCARS native verification."""
from pathlib import Path
import subprocess
import sys
import tempfile


def main() -> int:
    if sys.platform != "darwin":
        print("SCARS native desktop verification currently requires macOS.", file=sys.stderr)
        return 1
    source = Path(__file__).resolve().parent / "diagnostics" / "scars_desktop_probe.mm"
    try:
        compiler = subprocess.check_output(
            ["xcrun", "--sdk", "macosx", "--find", "clang++"], text=True
        ).strip()
        sdk = subprocess.check_output(
            ["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True
        ).strip()
        with tempfile.TemporaryDirectory(prefix="scars-desktop-") as directory:
            executable = Path(directory) / "probe"
            subprocess.run(
                [compiler, "-std=c++23", "-fobjc-arc", "-mmacosx-version-min=26.0",
                 "-isysroot", sdk, str(source), "-framework", "AppKit", "-framework",
                 "QuartzCore", "-o", str(executable)],
                check=True,
            )
            result = subprocess.run([str(executable)], timeout=10)
            if result.returncode:
                print("Desktop not ready for foreground verification. Wake/unlock the Mac, "
                      "then run this probe from the logged-in desktop session.", file=sys.stderr)
            return result.returncode
    except (OSError, subprocess.SubprocessError) as error:
        print(f"SCARS desktop probe failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
