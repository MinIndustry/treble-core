"""Exercise the actual benchmark gate against synthetic Criterion result files."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("bench_compare.py").resolve()


class ComparisonTests(unittest.TestCase):
    def compare(self, results):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, baseline, point, low, high in results:
                output = root / "target" / "criterion" / name / baseline
                output.mkdir(parents=True)
                (output / "estimates.json").write_text(json.dumps({"mean": {
                    "point_estimate": point,
                    "confidence_interval": {"lower_bound": low, "upper_bound": high},
                }}))
            result = subprocess.run([
                sys.executable, str(SCRIPT), "--base", "base", "--new", "pr",
                "--require-complete",
            ], cwd=root, capture_output=True, text=True, check=False)
            return result.returncode, result.stdout

    def test_improvement_and_unrelated_cache_pass(self):
        code, report = self.compare([
            ("render", "base", 100, 98, 102), ("render", "pr", 80, 78, 82),
            ("stale", "old", 100, 98, 102),
        ])
        self.assertEqual(code, 0)
        self.assertNotIn("stale", report)
        self.assertNotIn("skipped", report)

    def test_separated_regression_fails_without_a_waiver(self):
        code, report = self.compare([
            ("render", "base", 100, 98, 102), ("render", "pr", 140, 138, 142),
        ])
        self.assertEqual(code, 1)
        self.assertIn("Regression check failed", report)

    def test_overlapping_intervals_are_reported_as_noise(self):
        code, report = self.compare([
            ("render", "base", 100, 70, 130), ("render", "pr", 140, 120, 160),
        ])
        self.assertEqual(code, 0)
        self.assertIn("intervals overlap", report)

    def test_missing_either_baseline_fails(self):
        for baseline in ["base", "pr"]:
            code, report = self.compare([
                ("complete", "base", 100, 98, 102), ("complete", "pr", 90, 88, 92),
                ("missing", baseline, 100, 98, 102),
            ])
            self.assertEqual(code, 2)
            self.assertIn("Incomplete comparison", report)

    def test_no_comparable_benchmarks_fails(self):
        code, _ = self.compare([("stale", "old", 100, 98, 102)])
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
