"""Keep the example inventory exhaustive and planning safe."""
import contextlib
import copy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_rust_examples as examples


class ExampleChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata = json.loads(subprocess.check_output(
            ("cargo", "metadata", "--format-version", "1", "--no-deps", "--locked", "--offline"),
            cwd=examples.check.RUST, text=True))

    def test_actual_manifest_has_no_unclassified_or_accidentally_native_headless_examples(self):
        names = examples.discover(self.metadata)
        steps, coverage, previews = examples.plan(names, Path("/target"), Path("/output"), False, True)
        for name in examples.NATIVE_ONLY:
            self.assertEqual(coverage[name], "not_run: native-only")
            self.assertFalse(any(Path(step.command[0]).name == name for step in steps))
        self.assertTrue(all("--smoke" not in step.command for step in steps))
        self.assertEqual(len(previews), (len(examples.RASTER) + len(examples.VARIANTS)) * 4)
        self.assertEqual(len(previews), len(set(previews)))
        self.assertIn("--locked", steps[0].command)
        self.assertIn("--offline", steps[0].command)

    def test_added_or_removed_examples_require_an_explicit_mode(self):
        added = copy.deepcopy(self.metadata)
        added["packages"][0]["targets"].append({"kind": ["example"], "name": "unclassified"})
        with self.assertRaisesRegex(ValueError, "unclassified"):
            examples.discover(added)
        removed = copy.deepcopy(self.metadata)
        for package in removed["packages"]:
            package["targets"] = [target for target in package["targets"] if target["name"] != "todo"]
        with self.assertRaisesRegex(ValueError, "missing=.*todo"):
            examples.discover(removed)

    def test_native_plan_does_not_execute_examples_or_modify_existing_output(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "keep"
            marker.write_text("unchanged")
            with patch.object(examples.sys, "platform", "linux"), patch.object(examples.check, "run_steps", side_effect=AssertionError("plan executed")), contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(examples.main(["--native", "--offline", "--plan", "--output", directory]), 0)
            self.assertIn("PLAN ONLY", output.getvalue())
            self.assertEqual(marker.read_text(), "unchanged")
            self.assertEqual(list(Path(directory).iterdir()), [marker])


if __name__ == "__main__":
    unittest.main()
