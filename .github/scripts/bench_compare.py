#!/usr/bin/env python3
"""Compare two saved criterion baselines and fail on regression.

Reads mean point estimates from target/criterion/<bench>/<baseline>/estimates.json,
writes a markdown report (bench-report.md + $GITHUB_STEP_SUMMARY), and exits
non-zero if any benchmark's mean regressed more than --max-regression-pct.
"""

import argparse
import json
import os
import sys
from pathlib import Path

CRITERION_DIR = Path("target/criterion")


def mean_estimate(bench_dir: Path, baseline: str) -> float | None:
    f = bench_dir / baseline / "estimates.json"
    if not f.is_file():
        return None
    with open(f) as fh:
        return json.load(fh)["mean"]["point_estimate"]  # nanoseconds


def fmt_ns(ns: float) -> str:
    for unit, factor in (("s", 1e9), ("ms", 1e6), ("µs", 1e3)):
        if ns >= factor:
            return f"{ns / factor:.2f} {unit}"
    return f"{ns:.0f} ns"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True, help="baseline name for the base commit")
    ap.add_argument("--new", required=True, help="baseline name for the PR commit")
    ap.add_argument("--max-regression-pct", type=float, default=15.0)
    args = ap.parse_args()

    if not CRITERION_DIR.is_dir():
        print(f"error: {CRITERION_DIR} not found — did the benches run?", file=sys.stderr)
        return 2

    rows, failures, missing = [], [], []
    for bench_dir in sorted(CRITERION_DIR.iterdir()):
        if not bench_dir.is_dir() or bench_dir.name == "report":
            continue
        base = mean_estimate(bench_dir, args.base)
        new = mean_estimate(bench_dir, args.new)
        if base is None or new is None:
            missing.append(bench_dir.name)
            continue
        delta_pct = (new - base) / base * 100.0
        regressed = delta_pct > args.max_regression_pct
        if regressed:
            failures.append(bench_dir.name)
        icon = "❌" if regressed else ("🟡" if delta_pct > 0 else "✅")
        rows.append(f"| `{bench_dir.name}` | {fmt_ns(base)} | {fmt_ns(new)} | {delta_pct:+.1f}% | {icon} |")

    if not rows:
        print("error: no benchmarks with both baselines found", file=sys.stderr)
        return 2

    verdict = (
        f"**❌ Regression check failed** — {', '.join(f'`{f}`' for f in failures)} "
        f"slower than the {args.max_regression_pct:.0f}% threshold."
        if failures
        else f"**✅ No regression** above the {args.max_regression_pct:.0f}% threshold."
    )
    report = "\n".join(
        [
            "## Benchmark comparison (base vs PR)",
            "",
            "| benchmark | base (mean) | PR (mean) | Δ | |",
            "|---|---|---|---|---|",
            *rows,
            "",
            verdict,
            "",
            "<sub>criterion mean point estimates, same runner & job. "
            "Hosted runners jitter ±5–10%; re-run the check if a small overshoot looks spurious.</sub>",
        ]
    )
    if missing:
        report += f"\n\n<sub>⚠ skipped (baseline missing): {', '.join(missing)}</sub>"

    print(report)
    Path("bench-report.md").write_text(report + "\n")
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as fh:
            fh.write(report + "\n")

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
