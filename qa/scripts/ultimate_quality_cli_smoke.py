#!/usr/bin/env python3
"""Bounded end-to-end CAD QA: real CLI processes, independent file checks, no GUI claim.

No extra Python packages. The CLI binary must be compiled beforehand. All writes are
inside an auto-cleaned temporary directory. Any failure produces a JSON evidence file.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import tempfile
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]


def script_commands(payload: object) -> list[dict]:
    """The CLI recipe and history exports use {"commands": [...]}, not a bare list."""
    if isinstance(payload, dict):
        payload = payload.get("commands")
    if not isinstance(payload, list) or not all(isinstance(step, dict) for step in payload):
        raise AssertionError("expected a command script object or array")
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    binary = args.cli.resolve()
    report_path = args.report.resolve()
    events: list[dict] = []
    evidence: dict = {"scope": "native headless subprocess only, NOT an interactive GUI test",
                      "binary": str(binary), "events": events, "status": "running"}

    def run(*argv: object, timeout: int = 180, success: bool = True) -> str:
        command = [str(binary), *(str(arg) for arg in argv)]
        before = time.monotonic()
        proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True,
                              errors="replace", timeout=timeout, check=False)
        events.append({"args": command[1:], "returncode": proc.returncode,
                       "seconds": round(time.monotonic() - before, 3),
                       "stdout_tail": proc.stdout[-1000:], "stderr_tail": proc.stderr[-1200:]})
        if success and proc.returncode != 0:
            raise AssertionError(f"CLI exited {proc.returncode}: {command[1:]}\n{proc.stderr[-1800:]}")
        if not success and proc.returncode == 0:
            raise AssertionError(f"CLI accepted malformed input: {command[1:]}")
        if "panicked at" in proc.stderr:
            raise AssertionError(f"Rust panic: {proc.stderr[-1200:]}")
        return proc.stdout

    def inspect(path: Path) -> dict:
        payload = json.loads(run("eval", "--in", path))
        if payload.get("body_count") != 1:
            raise AssertionError(f"unexpected body_count: {payload.get('body_count')}")
        return payload

    def volume(m: dict) -> float:
        bodies = m.get("bodies")
        if not isinstance(bodies, list) or len(bodies) != 1:
            raise AssertionError("expected one measured body")
        v = bodies[0].get("volume_mm3")
        if not isinstance(v, (float, int)) or not math.isfinite(v):
            raise AssertionError("measured volume must be finite")
        return float(v)

    def near(actual: float, expected: float, rel: float = 1e-3) -> None:
        if not math.isclose(actual, expected, rel_tol=rel, abs_tol=1e-7):
            raise AssertionError(f"volume mismatch: {actual}, expected {expected}, rel={rel}")

    def check_stl(path: Path) -> bytes:
        data = path.read_bytes()
        if len(data) < 84:
            raise AssertionError("STL shorter than binary header")
        count = struct.unpack_from("<I", data, 80)[0]
        if count < 12 or count > 1_000_000 or len(data) != 84 + 50 * count:
            raise AssertionError(f"invalid binary STL triangle count/size: {count}, {len(data)}")
        lo, hi = [float("inf")] * 3, [float("-inf")] * 3
        for offset in range(84, len(data), 50):
            scalars = struct.unpack_from("<12f", data, offset)
            if not all(math.isfinite(v) for v in scalars):
                raise AssertionError(f"non-finite STL normal/vertex near offset {offset}")
            for k in range(3):
                for vertex in range(3):
                    value = scalars[3 + vertex * 3 + k]
                    lo[k], hi[k] = min(lo[k], value), max(hi[k], value)
        for actual, expected in zip(lo + hi, [0.0, 0.0, 0.0, 40.0, 30.0, 20.0]):
            if not math.isclose(actual, expected, abs_tol=1e-3):
                raise AssertionError(f"STL bounds differ: {lo}, {hi}")
        evidence.setdefault("stl", []).append({"name": path.name, "triangles": count,
                "bbox": {"min": lo, "max": hi}, "sha256": hashlib.sha256(data).hexdigest()})
        return data

    try:
        if not binary.is_file():
            raise FileNotFoundError(f"CLI binary not found: {binary}")
        with tempfile.TemporaryDirectory(prefix="arclet-ultimate-qa-") as td:
            d = Path(td)
            recipe = ROOT / "tests/oracle/01-box/recipe.json"
            source = d / "commands.json"
            source.write_text(run("recipe", recipe), encoding="utf-8")
            steps = script_commands(json.loads(source.read_text(encoding="utf-8")))
            if len(steps) < 4:
                raise AssertionError("recipe did not produce an executable script")
            base = d / "original.sc"
            stl = d / "original.stl"
            step = d / "original.step"
            run("run", source, "--save", base, "--out", stl, "--out", step, "--quiet")
            measured = inspect(base)
            near(volume(measured), 40.0 * 30.0 * 20.0)
            evidence["original_mm3"] = volume(measured)
            a_stl = check_stl(stl)
            step_data = step.read_bytes()
            if not step_data.startswith(b"ISO-10303-21;") or b"END-ISO-10303-21;" not in step_data[-2048:]:
                raise AssertionError("STEP Part21 envelope missing or truncated")
            evidence["step_sha256"] = hashlib.sha256(step_data).hexdigest()

            # Every command is a new process; old and new STL triangle buffers must match.
            reopened = d / "reopened.stl"
            run("run", "--in", base, "--out", reopened, "--quiet")
            if a_stl[80:] != check_stl(reopened)[80:]:
                raise AssertionError("STL geometry changed across a fresh process save/reopen")

            # STEP is independently read back and measured through the actual CAD importer.
            near(volume(inspect(step)), 24000.0, rel=2e-3)

            # Native document -> command script -> fresh native document.
            recovered = d / "history.json"
            run("script", base, "--out", recovered)
            if not script_commands(json.loads(recovered.read_text(encoding="utf-8"))):
                raise AssertionError("exported history has no executable commands")
            replayed = d / "replayed.sc"
            run("run", recovered, "--save", replayed, "--quiet")
            near(volume(inspect(replayed)), 24000.0)

            # Undo and redo should leave the exact original result.
            history = steps + [{"command": "edit.undo", "params": {}},
                               {"command": "edit.redo", "params": {}}]
            undo_script = d / "undo_redo.json"
            undo_script.write_text(json.dumps({"commands": history}), encoding="utf-8")
            undone = d / "undo_redo.sc"
            run("run", undo_script, "--save", undone, "--quiet")
            near(volume(inspect(undone)), 24000.0)

            # Real dimension edit and recomputation on an oracle with parameter-driven sketch.
            driven = script_commands(json.loads(run(
                "recipe", ROOT / "tests/oracle/24-user-parameters/recipe.json"
            )))
            driven.append({"command": "parameters.change",
                           "params": {"name": "width", "expression": "70 mm"}})
            edited = d / "parameter_edit.json"
            edited.write_text(json.dumps({"commands": driven}), encoding="utf-8")
            changed = d / "parameter_edit.sc"
            run("run", edited, "--save", changed, "--quiet")
            expected = (70.0 * 40.0 - math.pi * 5.0 * 5.0) * 15.0
            near(volume(inspect(changed)), expected, rel=2e-3)
            evidence["parameter_edit_mm3"] = volume(inspect(changed))

            # Invalid files and out-of-range fuzz seeds must be rejected quickly, never panic.
            corrupt = d / "broken.sc"
            corrupt.write_bytes(b"PK\x03\x04" + b"\x00" * 60)
            run("eval", "--in", corrupt, success=False, timeout=20)
            run("fuzz", "--from", "18446744073709551615", "--count", "2",
                success=False, timeout=20)

            # A known-good baseline robustness seed; log outcome and any caught panics.
            fuzz = json.loads(run("fuzz-one", "0", "--steps", "8", timeout=120))
            evidence["seed0"] = {
                "design_outcome": fuzz.get("design", {}).get("outcome", {}),
                "caught_panics": fuzz.get("design", {}).get("panics"),
                "boolean_outcome": fuzz.get("boolean", {}).get("outcome"),
                "boolean_panics": fuzz.get("boolean_panics"),
            }
            if evidence["seed0"]["design_outcome"].get("kind") != "ok":
                raise AssertionError(f"seed 0 was not clean: {evidence['seed0']}")
            if evidence["seed0"]["caught_panics"] != 0 or evidence["seed0"]["boolean_panics"] != 0:
                raise AssertionError(f"seed 0 had caught native panics: {evidence['seed0']}")

        evidence["status"] = "passed"
    except Exception:
        evidence["status"] = "failed"
        evidence["error"] = traceback.format_exc()[-6000:]
        raise
    finally:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"status": evidence["status"], "operations": len(events),
                          "report": str(report_path)}, sort_keys=True))


if __name__ == "__main__":
    main()
