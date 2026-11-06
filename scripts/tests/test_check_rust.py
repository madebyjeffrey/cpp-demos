"""Exercise execution boundaries, not Cargo itself."""
import contextlib
import io
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_rust as check


class ValidationTests(unittest.TestCase):
    def run_steps(self, steps, directory, env):
        report = {"suite": "test"}
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            result = check.run_steps(steps, directory, env, report)
        return result, json.loads((directory / "report.json").read_text())

    def test_headless_children_cannot_inherit_native_opt_ins(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            with patch.dict(os.environ, {flag: "1" for flag in check.NATIVE_FLAGS}):
                env = check.environment(directory, "headless")
            program = "import os,json; print(json.dumps({k:os.environ.get(k) for k in " + repr((*check.NATIVE_FLAGS, "FORCE_SKIA_BUILD")) + "}))"
            steps = [check.Step("headless", (sys.executable, "-c", program)),
                     check.Step("explicit-native", (sys.executable, "-c", program), env={check.NATIVE_FLAGS[0]: "1"})]
            code, report = self.run_steps(steps, directory, env)
            self.assertEqual(code, 0)
            values = [json.loads(Path(step["log"]).read_text().splitlines()[-1]) for step in report["steps"]]
            self.assertTrue(all(values[0][flag] is None for flag in check.NATIVE_FLAGS))
            self.assertEqual(values[0]["FORCE_SKIA_BUILD"], "1")
            self.assertEqual(values[1][check.NATIVE_FLAGS[0]], "1")
            self.assertTrue(all(values[1][flag] is None for flag in check.NATIVE_FLAGS[1:]))

    def test_minimum_suite_selects_and_reports_its_actual_toolchain_without_native_opt_ins(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {"RUSTUP_TOOLCHAIN": "stable", **{flag: "1" for flag in check.NATIVE_FLAGS}}):
                env = check.environment(Path(directory), "msrv")
            program = "import os; print(os.environ['RUSTUP_TOOLCHAIN'])"
            self.assertEqual(check.metadata((sys.executable, "-c", program), env=env), check.MINIMUM_RUST)
            self.assertTrue(all(flag not in env for flag in check.NATIVE_FLAGS))
            steps = check.plan("msrv", True)
            self.assertEqual(len(steps), 5)
            self.assertTrue(all("--locked" in step.command and "--offline" in step.command for step in steps))
            self.assertIn("--workspace", steps[1].command)
            self.assertIn("--all-targets", steps[1].command)
            self.assertIn("--release", steps[2].command)
            self.assertIn("x86_64-unknown-linux-gnu", steps[3].command)

    def test_failure_is_recorded_and_prevents_followup_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            marker = directory / "must-not-run"
            steps = [check.Step("failure", (sys.executable, "-c", "import sys;print('actual failure',flush=True);sys.exit(7)")),
                     check.Step("followup", (sys.executable, "-c", f"from pathlib import Path;Path({str(marker)!r}).touch()"))]
            code, report = self.run_steps(steps, directory, check.environment(directory, "headless"))
            self.assertEqual(code, 1)
            self.assertEqual(report["status"], "failed")
            self.assertEqual(report["steps"][0]["exit_code"], 7)
            self.assertEqual(report["steps"][1]["status"], "not_run")
            self.assertIn("actual failure", Path(report["steps"][0]["log"]).read_text())
            self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == "posix", "Native validator runs on macOS/POSIX")
    def test_timeout_retires_descendants_and_cannot_report_success(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            marker = directory / "orphan"
            child = f"import time;from pathlib import Path;time.sleep(0.8);Path({str(marker)!r}).touch()"
            parent = f"import subprocess,sys,time;subprocess.Popen([sys.executable,'-c',{child!r}]);print('started',flush=True);time.sleep(10)"
            step = check.Step("timeout", (sys.executable, "-c", parent), timeout=0.3)
            code, report = self.run_steps([step], directory, check.environment(directory, "headless"))
            self.assertEqual(code, 1)
            self.assertEqual(report["steps"][0]["exit_code"], 124)
            self.assertIn("Check timed out", Path(report["steps"][0]["log"]).read_text())
            time.sleep(0.9)
            self.assertFalse(marker.exists(), "a timed-out Cargo command must not leave children running")

    def test_native_plan_gates_and_covers_every_opt_in_harness(self):
        guards = {}
        for source in (check.RUST / "crates/scars-window/tests").rglob("*.rs"):
            found = re.search(r'var_os\("(SCARS_WINDOW_[A-Z_]+_TESTS)"\)', source.read_text())
            if found:
                guards[source.stem] = found[1]
        steps = check.plan("native", True)
        self.assertEqual(steps[0].name, "desktop-readiness")
        selected = {}
        for step in steps[1:]:
            if step.name == "example-regressions":
                self.assertIn("--native", step.command)
                continue
            self.assertIn("--locked", step.command)
            self.assertIn("--offline", step.command)
            target = step.command[step.command.index("--test") + 1]
            selected[target] = step.env
        self.assertEqual(set(selected), set(guards))
        self.assertEqual(set(check.NATIVE_FLAGS), set(guards.values()))
        for target, flag in guards.items():
            self.assertEqual(selected[target], {flag: "1"})
        for suite in ("headless", "portable"):
            self.assertTrue(all(not (set(step.env) & set(check.NATIVE_FLAGS)) for step in check.plan(suite, True)))

    @unittest.skipUnless(os.name == "posix", "Process-group termination requires POSIX")
    def test_parent_timeout_retires_a_nested_runner_and_its_separate_child_group(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            marker = directory / "orphan"
            child = f"import time;from pathlib import Path;time.sleep(1);Path({str(marker)!r}).touch()"
            program = "import sys;sys.path.insert(0," + repr(str(check.ROOT / "scripts")) + ");import check_rust as c;from pathlib import Path;c.handle_termination();c.execute(c.Step('nested-child',(sys.executable,'-c'," + repr(child) + ")),dict(__import__('os').environ),Path(" + repr(str(directory / "nested.log")) + "))"
            step = check.Step("nested-runner", (sys.executable, "-c", program), env={"SCARS_VALIDATION_NESTED": "1"}, timeout=0.4)
            code, report = self.run_steps([step], directory, check.environment(directory, "portable"))
            self.assertEqual(code, 1)
            self.assertEqual(report["steps"][0]["exit_code"], 124)
            time.sleep(1.1)
            self.assertFalse(marker.exists(), "a nested runner must retire its separate child group")

    def test_planning_native_checks_requires_no_native_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(check.sys, "platform", "linux"), patch.object(check, "execute", side_effect=AssertionError("plan executed")), contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(check.main(["--suite", "native", "--plan", "--output", directory]), 0)
            self.assertIn("PLAN ONLY", output.getvalue())
            self.assertFalse((Path(directory) / "report.json").exists())


if __name__ == "__main__":
    unittest.main()
