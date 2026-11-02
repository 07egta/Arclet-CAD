#!/usr/bin/env python3
"""Compare matched Arclet viewport benchmark artifacts without mixing GPU tiers.

Usage: python3 scripts/arclet_bench_compare.py BASELINE_DIR CHANGED_DIR
Both directories must contain small.json, medium.json and large.json.
"""

import json
import math
import sys
from pathlib import Path

CASES = ("small", "medium", "large")
METRICS = ("frame_p50_ms", "frame_p95_ms", "first_draw_ms", "peak_rss_kib")
COMPARABLE = (
    "case", "triangles", "resolution", "warmup_frames", "sample_frames",
    "adapter", "backend", "device_type", "tier", "policy", "source",
)


def load(directory, case):
    file = Path(directory) / (case + ".json")
    data = json.loads(file.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{file}: expected a JSON object")
    for key in COMPARABLE:
        if key not in data:
            raise ValueError(f"{file}: missing {key}")
    for key in METRICS:
        value = data.get(key)
        if value is None or isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError(f"{file}: missing or invalid {key}: {value!r}")
    if data["case"] != case:
        raise ValueError(f"{file}: case mismatch {data['case']!r}")
    if data["tier"] not in {"Hardware", "Software"}:
        raise ValueError(f"{file}: unknown GPU tier; refusing to compare")
    return data


def compare(baseline, changed):
    for key in COMPARABLE:
        if baseline[key] != changed[key]:
            raise ValueError(f"unmatched workloads/adapters: {key}: {baseline[key]!r} != {changed[key]!r}")
    return [(metric, baseline[metric], changed[metric]) for metric in METRICS]


def markdown(before, after):
    results = []
    for case in CASES:
        left, right = load(before, case), load(after, case)
        results.append((case, compare(left, right)))
    print("# Arclet viewport benchmark comparison")
    print()
    print("**Warning:** single-run offscreen software/hardware-adapter results, not interactive FPS or input latency.")
    print("The improvement column describes these samples only; it is not a real-device claim.")
    print()
    print("| Case | Metric | Baseline | Changed | Change |")
    print("|---|---|---:|---:|---:|")
    for case, metrics in results:
        for key, old, new in metrics:
            delta = "n/a" if old == 0 else f"{100 * (old - new) / old:+.1f}%"
            decimals = 1 if key == "peak_rss_kib" else 3
            print(f"| {case} | {key} | {old:.{decimals}f} | {new:.{decimals}f} | {delta} |")


def main(argv):
    if len(argv) != 3:
        print(f"Usage: {Path(argv[0]).name} BASELINE_DIR CHANGED_DIR", file=sys.stderr)
        return 2
    try:
        markdown(argv[1], argv[2])
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"refused an invalid comparison: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
