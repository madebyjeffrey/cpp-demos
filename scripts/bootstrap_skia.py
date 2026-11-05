#!/usr/bin/env python3
"""Build our locked Skia dependency. Never runs implicitly from CMake."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def run(*args, cwd=None, env=None):
    print("+", " ".join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=cwd, env=env, check=True)


def gn_value(value):
    return json.dumps(value)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=["arm64", "x86_64"],
                        default=platform.machine())
    parser.add_argument("--config", choices=["Debug", "RelWithDebInfo"],
                        default="RelWithDebInfo")
    parser.add_argument("--jobs", type=int, default=min(os.cpu_count() or 4, 12))
    parser.add_argument("--resync", action="store_true", help="Recheck all downloaded Skia dependencies")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("The initial backend requires macOS 26 or later.")
    for tool in ["git", "ninja", "xcrun"]:
        if not shutil.which(tool):
            parser.error(f"Missing {tool}; install Xcode command-line tools and Ninja.")
    lock = json.loads((ROOT / "dependencies/skia.json").read_text())
    checkout = ROOT / ".deps/skia"
    checkout.mkdir(parents=True, exist_ok=True)
    if not (checkout / ".git").exists():
        run("git", "init", checkout)
        run("git", "remote", "add", "origin", lock["repository"], cwd=checkout)
    current = subprocess.run(["git", "rev-parse", "HEAD"], cwd=checkout,
                             capture_output=True, text=True)
    if current.stdout.strip() != lock["revision"]:
        dirty = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=checkout, text=True)
        if dirty.strip():
            parser.error("Skia checkout has local changes; refusing to replace them.")
        run("git", "fetch", "--depth=1", "origin", lock["revision"], cwd=checkout)
        run("git", "checkout", "--detach", "FETCH_HEAD", cwd=checkout)
    env = os.environ.copy()
    env["GIT_SYNC_DEPS_QUIET"] = "1"
    env["GIT_SYNC_DEPS_SKIP_EMSDK"] = "1" # This native desktop build does not use Emscripten.
    sync_stamp = checkout / ".skia-window-deps"
    if args.resync or not sync_stamp.exists() or sync_stamp.read_text() != lock["revision"]:
        run(sys.executable, "tools/git-sync-deps", cwd=checkout, env=env)
        sync_stamp.write_text(lock["revision"])
    if not (checkout / "bin/gn").exists():
        run(sys.executable, "bin/fetch-gn", cwd=checkout)
    out = checkout / "out" / f"{args.arch}-{args.config}"
    sdk = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
    # Skia supplies a macOS 11 target triple. Override the full triple: Clang gives
    # -target precedence over -mmacosx-version-min, regardless of argument order.
    flags = ["-target", f"{args.arch}-apple-macos{lock['deployment_target']}", "-isysroot", sdk]
    if args.config == "RelWithDebInfo":
        flags += ["-gline-tables-only"]
    settings = dict(lock["gn_args"],
                    target_os="mac", target_cpu="x64" if args.arch == "x86_64" else "arm64",
                    is_debug=args.config == "Debug", is_official_build=args.config != "Debug",
                    cc="clang", cxx="clang++", extra_cflags=flags,
                    extra_ldflags=flags)
    gn_args = " ".join(f"{key}={gn_value(value)}" for key, value in settings.items())
    run(checkout / "bin/gn", "gen", out, "--fail-on-unused-args", f"--args={gn_args}", cwd=checkout)
    run("ninja", "-C", out, f"-j{args.jobs}", "skia", cwd=checkout)
    (out / "skia-window-build.json").write_text(json.dumps({
        "revision": lock["revision"], "arch": args.arch, "config": args.config,
        "deployment_target": lock["deployment_target"], "gn_args": settings
    }, indent=2) + "\n")
    print(f"\nSkia ready: {out}\nConfigure with: cmake --preset {args.config.lower()}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
