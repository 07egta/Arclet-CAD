#!/usr/bin/env python3
"""Mission 40: isolated, evidence-based Arclet desktop CLI workflow checks.

The catalog is NOT an execution record. Only subprocess observations can produce PASS.
GUI-only workflows always remain NOT_RUN here. No network or third-party modules.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
CATALOG = Path(__file__).with_name("catalog.json")
KINDS = {"eval", "roundtrip", "export", "snapshot", "negative", "negative_run", "manual"}


class WorkflowFailure(Exception):
    """An observed failure of an executable scenario or its assertion."""


def load_catalog(path: Path = CATALOG) -> dict:
    doc = json.loads(path.read_text(encoding="utf-8"))
    cases = doc["scenarios"]
    if len(cases) != 100:
        raise ValueError(f"expected exactly 100 scenarios, found {len(cases)}")
    if len({c["id"] for c in cases}) != 100:
        raise ValueError("duplicate scenario ids")
    for category in "ABCDEFGHIJ":
        expected_ids = {f"{category}{n:02}" for n in range(1, 11)}
        found = {c["id"] for c in cases if c["category"] == category}
        if found != expected_ids:
            raise ValueError(f"category {category}: ids do not match {sorted(expected_ids)}")
    if [sum(c["stage"] == n for c in cases) for n in range(1, 5)] != [20, 40, 30, 10]:
        raise ValueError("stage distribution must be 20/40/30/10")
    for c in cases:
        for field in ("title", "purpose", "preconditions", "input_model", "steps", "expected_behavior", "quality_axes", "reproduction", "issues", "pull_requests", "status", "actual_result"):
            if field not in c:
                raise ValueError(f"{c['id']} missing {field}")
        if not c["steps"] or not all(isinstance(s, str) and s for s in c["steps"]):
            raise ValueError(f"{c['id']}: missing meaningful steps")
        if c["status"] != "NOT_RUN" or c["actual_result"] is not None:
            raise ValueError(f"{c['id']}: source catalog must not pretend that it was executed")
        e = c["execution"]
        if e["kind"] not in KINDS:
            raise ValueError(f"{c['id']}: unknown execution kind")
        if e["kind"] != "manual" and not ("commands" in e or "source" in e):
            raise ValueError(f"{c['id']}: executable case has no input")
        if e.get("repeat", 1) < 1 or e.get("repeat", 1) > 300:
            raise ValueError(f"{c['id']}: invalid repeat")
    return doc


def resolve_cli(arg: str) -> str | None:
    path = Path(arg)
    if path.is_file():
        return str(path.resolve())
    found = shutil.which(arg)
    return str(Path(found).resolve()) if found else None


def repo_fixture(name: str) -> Path:
    source = (ROOT / name).resolve()
    if not source.is_relative_to(ROOT.resolve()) or not source.is_file():
        raise WorkflowFailure(f"fixture is missing or escapes repository: {name}")
    return source


def assert_close(value: float, wanted: float, rel: float, absolute: float, label: str) -> None:
    if not (math.isfinite(value) and math.isfinite(wanted)):
        raise WorkflowFailure(f"{label}: non-finite measurement {value} / {wanted}")
    if not math.isclose(value, wanted, rel_tol=rel, abs_tol=absolute):
        raise WorkflowFailure(f"{label}: observed {value}, expected {wanted} (rel {rel}, abs {absolute})")


def measurements(report: dict, expected: dict) -> list[str]:
    checks = []
    bodies = report.get("bodies", [])
    n = report.get("body_count")
    if not isinstance(n, int) or not isinstance(bodies, list):
        raise WorkflowFailure("not an inspect.measure report: body_count / bodies missing")
    if n != len(bodies):
        raise WorkflowFailure(f"report is inconsistent: body_count={n}, len(bodies)={len(bodies)}")
    if "body_count" in expected and n != expected["body_count"]:
        raise WorkflowFailure(f"body_count={n}, expected {expected['body_count']}")
    if "min_body_count" in expected and n < expected["min_body_count"]:
        raise WorkflowFailure(f"body_count={n} below {expected['min_body_count']}")
    checks.append("body_count")
    timeline = report.get("timeline")
    if timeline is not None:
        failures = [(f.get("name"), f.get("error")) for f in timeline if f.get("error")]
        if failures:
            raise WorkflowFailure(f"timeline evaluation failed: {failures[:3]}")
        checks.append("timeline_errors_absent")
    elif expected.get("timeline_clean"):
        raise WorkflowFailure("timeline missing from measurement")
    total = report.get("total", {})
    for key, floor in (("volume_mm3", "min_volume_mm3"), ("area_mm2", "min_area_mm2")):
        if key in expected or floor in expected:
            observed = total.get(key)
            if not isinstance(observed, (float, int)) or not math.isfinite(observed):
                raise WorkflowFailure(f"{key} missing or not finite")
            if key in expected:
                tol = expected.get("volume_rel_tol", 1e-5) if key == "volume_mm3" else expected.get("area_rel_tol", 1e-5)
                assert_close(float(observed), float(expected[key]), tol, 1e-4, key)
            if floor in expected and not observed > expected[floor]:
                raise WorkflowFailure(f"{key}={observed}, must exceed {expected[floor]}")
            checks.append(key)
    if "sketch_count" in expected and len(report.get("sketches", [])) != expected["sketch_count"]:
        raise WorkflowFailure(f"sketch_count={len(report.get('sketches', []))}, expected {expected['sketch_count']}")
    if "sketch_count" in expected:
        checks.append("sketch_count")
    if "profiles_at_least" in expected or "profiles_exactly" in expected:
        profiles = sum(s.get("profiles", 0) for s in report.get("sketches", []))
        if "profiles_at_least" in expected and profiles < expected["profiles_at_least"]:
            raise WorkflowFailure(f"profiles={profiles} below {expected['profiles_at_least']}")
        if "profiles_exactly" in expected and profiles != expected["profiles_exactly"]:
            raise WorkflowFailure(f"profiles={profiles}, expected {expected['profiles_exactly']}")
        checks.append("closed_profiles")
    if "bbox_size_mm" in expected or expected.get("com_at_bbox_center"):
        if n != 1:
            raise WorkflowFailure("bbox/COM assertion requires exactly one body")
        body = bodies[0]
        bounds = body.get("bbox", {})
        lo, hi = bounds.get("min"), bounds.get("max")
        if not (isinstance(lo, list) and isinstance(hi, list) and len(lo) == len(hi) == 3):
            raise WorkflowFailure("body bbox missing")
        if "bbox_size_mm" in expected:
            for i, axis in enumerate("xyz"):
                assert_close(float(hi[i] - lo[i]), float(expected["bbox_size_mm"][i]), 0, expected.get("bbox_abs_tol_mm", 0.001), f"bbox {axis}")
            checks.append("bbox_extents")
        if expected.get("com_at_bbox_center"):
            com = body.get("center_of_mass")
            if not isinstance(com, list) or len(com) != 3:
                raise WorkflowFailure("center_of_mass missing")
            for i, axis in enumerate("xyz"):
                assert_close(float(com[i]), (hi[i] + lo[i]) / 2, 0, expected.get("bbox_abs_tol_mm", 0.001), f"COM {axis}")
            checks.append("center_of_mass")
    return checks


def run_one(case: dict, cli: str | None, stress: bool, seconds: int) -> dict:
    cfg = case["execution"]
    result = {"id": case["id"], "status": "NOT_RUN", "actual_result": None,
              "reproduction": case["reproduction"], "issues": case["issues"], "pull_requests": case["pull_requests"]}
    if cfg["kind"] == "manual":
        result["reason"] = "requires native desktop GUI and recorded operator evidence"
        return result
    if cfg.get("stress") and not stress:
        result["reason"] = "stress case; enable --include-stress deliberately"
        return result
    if cli is None:
        result["reason"] = "solidcraft-cli binary unavailable; Rust commands were not executed"
        return result
    start = time.perf_counter()
    calls = []
    try:
        with tempfile.TemporaryDirectory(prefix="arclet-m40-") as folder:
            scratch = Path(folder)
            commands = cfg.get("commands", []) * cfg.get("repeat", 1)
            script = scratch / "script.json"
            script.write_text(json.dumps({"commands": commands}, ensure_ascii=False), encoding="utf-8")
            source = repo_fixture(cfg["source"]) if cfg.get("source") else None
            target = script if commands or source is None else source
            base = ["--in", str(source)] if source and commands else []
            env = os.environ.copy()
            env.setdefault("SOLIDCRAFT_GPU", "cpu")

            def invoke(op: str, extra: list[str] | None = None, parse: bool = True) -> dict | subprocess.CompletedProcess:
                argv = [cli, op, str(target), *base, *(extra or [])]
                try:
                    completed = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True,
                                               text=True, encoding="utf-8", errors="replace", timeout=seconds, check=False)
                except subprocess.TimeoutExpired as exc:
                    raise WorkflowFailure(f"timeout after {seconds}s: {argv[1]}") from exc
                calls.append({"argv": argv[1:], "returncode": completed.returncode,
                              "stderr_excerpt": completed.stderr[-1500:]})
                if not parse:
                    return completed
                if completed.returncode:
                    raise WorkflowFailure(f"{op} returned {completed.returncode}: {completed.stderr[-1200:]}")
                try:
                    return json.loads(completed.stdout)
                except json.JSONDecodeError as exc:
                    raise WorkflowFailure(f"{op} did not produce JSON: {completed.stdout[-500:]}") from exc

            kind = cfg["kind"]
            expected = cfg.get("expected", {})
            checks = []
            if kind in ("negative", "negative_run"):
                extra = ["--save", str(scratch / "must-not-exist.solidcraft")] if kind == "negative_run" else []
                bad = invoke("run" if kind == "negative_run" else "eval", extra, parse=False)
                if bad.returncode == 0:
                    raise WorkflowFailure("command unexpectedly succeeded")
                if expected.get("error_contains") and expected["error_contains"] not in bad.stderr:
                    raise WorkflowFailure("expected error not found in stderr")
                if (scratch / "must-not-exist.solidcraft").exists():
                    raise WorkflowFailure("failed run wrote a design")
                checks = ["rejected_invalid_command", "no_success_exit"]
                actual = {"exit_code": bad.returncode, "stderr": bad.stderr[-1000:]}
            elif kind == "eval":
                inspected = invoke("eval")
                checks = measurements(inspected, expected)
                actual = {"body_count": inspected["body_count"], "total": inspected.get("total"),
                          "sketches": inspected.get("sketches"), "timeline_errors": 0}
            elif kind in ("roundtrip", "export"):
                ext = cfg.get("extension", ".solidcraft")
                saved = scratch / ("cold-reload" + ext)
                run_args = ["--save", str(saved)] if kind == "roundtrip" else []
                if kind == "export":
                    fmt = cfg["format"]
                    files = [scratch / f"export-{i}.{fmt}" for i in range(cfg.get("repeat_exports", 1))]
                    run_args.extend(x for f in files for x in ("--out", str(f)))
                warm = invoke("run", run_args)
                checks = measurements(warm, expected)
                if kind == "roundtrip":
                    cold = subprocess.run([cli, "eval", str(saved)], cwd=ROOT, env=env, capture_output=True,
                                          text=True, encoding="utf-8", errors="replace", timeout=seconds, check=False)
                    calls.append({"argv": ["eval", str(saved)], "returncode": cold.returncode, "stderr_excerpt": cold.stderr[-1000:]})
                    if cold.returncode:
                        raise WorkflowFailure(f"cold reopen failed: {cold.stderr[-1000:]}")
                    try:
                        reopened = json.loads(cold.stdout)
                    except json.JSONDecodeError as exc:
                        raise WorkflowFailure("cold reopen returned invalid JSON") from exc
                    checks.extend(measurements(reopened, expected))
                    if reopened["body_count"] != warm["body_count"]:
                        raise WorkflowFailure("body count changed after cold reload")
                    if warm["body_count"]:
                        a, b = warm["total"]["volume_mm3"], reopened["total"]["volume_mm3"]
                        assert_close(float(a), float(b), 1e-4, 1e-3, "roundtrip volume")
                    if not saved.is_file() or saved.stat().st_size < 50:
                        raise WorkflowFailure("design output missing/empty")
                    checks.append("saved_design_cold_reload_invariant")
                    actual = {"body_count": reopened["body_count"], "volume_mm3": reopened["total"].get("volume_mm3"),
                              "saved_bytes": saved.stat().st_size}
                else:
                    sizes = [f.stat().st_size if f.is_file() else 0 for f in files]
                    if any(size < expected["export_min_bytes"] for size in sizes):
                        raise WorkflowFailure(f"export missing/too small: {sizes}")
                    checks.append("all_export_files_exist")
                    actual = {"body_count": warm["body_count"], "volume_mm3": warm.get("total", {}).get("volume_mm3"),
                              "export_sizes": sizes}
            elif kind == "snapshot":
                png = scratch / "iso.png"
                response = invoke("snapshot", ["--out", str(png), "--view", "iso", "--width", "480", "--height", "320"])
                data = png.read_bytes() if png.is_file() else b""
                if len(data) < 100 or not data.startswith(b"\x89PNG\r\n\x1a\n") or int.from_bytes(data[16:20], "big") != 480 or int.from_bytes(data[20:24], "big") != 320:
                    raise WorkflowFailure("snapshot missing, invalid PNG or wrong dimensions")
                checks = ["png_signature", "png_dimensions"]
                actual = {"png_bytes": len(data), "renderer": response.get("renderer")}
            else:
                raise WorkflowFailure(f"unsupported execution kind {kind}")
            result.update(status="PASS", actual_result={**actual, "checks": checks, "invocations": calls,
                                                        "elapsed_ms": round((time.perf_counter() - start) * 1000, 2)})
    except (WorkflowFailure, ValueError, OSError, subprocess.TimeoutExpired, TypeError, KeyError) as exc:
        result.update(status="FAIL", actual_result={"error": str(exc), "invocations": calls,
                                                        "elapsed_ms": round((time.perf_counter() - start) * 1000, 2)})
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", default="solidcraft-cli", help="path to a built desktop headless CLI")
    parser.add_argument("--stage", type=int, choices=(1, 2, 3, 4), help="only one mission stage")
    parser.add_argument("--ids", help="comma-separated exact scenario IDs")
    parser.add_argument("--include-stress", action="store_true", help="enable potentially expensive fourth-stage cases")
    parser.add_argument("--timeout-s", type=int, default=180, help="per CLI invocation timeout")
    parser.add_argument("--validate-only", action="store_true", help="validate the 100-case catalog without running CAD")
    parser.add_argument("--json-out", type=Path, help="write complete machine-readable results")
    args = parser.parse_args(argv)
    catalog = load_catalog()
    chosen = set(args.ids.split(",")) if args.ids else None
    if chosen and chosen - {c["id"] for c in catalog["scenarios"]}:
        parser.error(f"unknown IDs: {sorted(chosen - {c['id'] for c in catalog['scenarios']})}")
    selected = [c for c in catalog["scenarios"] if (args.stage is None or c["stage"] == args.stage) and (chosen is None or c["id"] in chosen)]
    cli = None if args.validate_only else resolve_cli(args.cli)
    results = []
    for c in catalog["scenarios"]:
        if c not in selected:
            results.append({"id": c["id"], "status": "NOT_RUN", "reason": "not selected"})
            continue
        r = ({"id": c["id"], "status": "NOT_RUN", "reason": "catalog validation only"}
             if args.validate_only else run_one(c, cli, args.include_stress, args.timeout_s))
        results.append(r)
        print(f"{r['status']:7} {c['id']} {c['title']}")
    counts = {state: sum(r["status"] == state and r["id"] in {c["id"] for c in selected} for r in results)
              for state in ("PASS", "FAIL", "NOT_RUN")}
    report = {"mission": catalog["mission"], "source_baseline": catalog.get("baseline_commit"),
              "timestamp_utc": dt.datetime.now(dt.timezone.utc).isoformat(), "host_os": platform.platform(),
              "cli": cli, "selected_count": len(selected), "summary": counts, "results": results}
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("SUMMARY", json.dumps(counts, ensure_ascii=False))
    if counts["FAIL"]:
        return 1
    if not args.validate_only and cli is None:
        print("ERROR: CLI missing; no CAD tests were executed", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())