#!/usr/bin/env python3
"""Compare two saved criterion baselines and fail on regression.

Reads target/criterion/<bench>/<baseline>/estimates.json, writes a markdown
report (bench-report.md + $GITHUB_STEP_SUMMARY), and exits non-zero on a
regression.

A regression needs two things to be true, not one: the mean must be more than
--max-regression-pct slower, **and** the two confidence intervals must not
overlap. A percentage alone cannot distinguish a real slowdown from runner
jitter, and jitter is worst in relative terms exactly where it matters least —
the nanosecond-scale microbenchmarks. A 100 ns bench swinging 20% on a shared
runner failed this gate while every macro benchmark sat inside 3%, which is
how a required check teaches people to ignore it. Disjoint intervals are the
cheap statistical answer, and criterion already computes them.
"""

import argparse
import json
import os
import sys
from pathlib import Path

CRITERION_DIR = Path("target/criterion")


def mean_estimate(bench_dir: Path, baseline: str) -> tuple[float, float, float] | None:
    """Mean point estimate with its confidence interval, in nanoseconds."""
    f = bench_dir / baseline / "estimates.json"
    if not f.is_file():
        return None
    with open(f) as fh:
        mean = json.load(fh)["mean"]
    interval = mean.get("confidence_interval", {})
    point = mean["point_estimate"]
    return (
        point,
        interval.get("lower_bound", point),
        interval.get("upper_bound", point),
    )


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

    rows, failures, missing, noisy = [], [], [], []
    for bench_dir in sorted(CRITERION_DIR.iterdir()):
        if not bench_dir.is_dir() or bench_dir.name == "report":
            continue
        base_est = mean_estimate(bench_dir, args.base)
        new_est = mean_estimate(bench_dir, args.new)
        if base_est is None or new_est is None:
            missing.append(bench_dir.name)
            continue
        base, _, base_high = base_est
        new, new_low, _ = new_est
        delta_pct = (new - base) / base * 100.0
        over_threshold = delta_pct > args.max_regression_pct
        # Disjoint intervals: the PR's best case is still worse than the base's
        # worst case. Overlapping intervals mean the run cannot tell them apart.
        separated = new_low > base_high
        regressed = over_threshold and separated
        if regressed:
            failures.append(bench_dir.name)
        elif over_threshold:
            noisy.append(f"{bench_dir.name} ({delta_pct:+.1f}%, intervals overlap)")
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
            "<sub>criterion means, same runner & job. A regression must both "
            "exceed the threshold and have confidence intervals that do not "
            "overlap the base's, because hosted runners jitter ±5–10% and most "
            "of that lands on the nanosecond-scale benches.</sub>",
        ]
    )
    if noisy:
        report += (
            "\n\n<sub>ℹ over the threshold but not separable from noise, so not "
            f"failed: {', '.join(noisy)}</sub>"
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
