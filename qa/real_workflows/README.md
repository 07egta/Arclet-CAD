# Mission 40 — 100 real CAD design workflows

This suite separates **planned coverage** from **observed test evidence**. The catalog has 100 unique scenarios: 10 each for A–J, organized into four stages (20/40/30/10). The runner currently implements CLI drivers and assertions for 54 of these, while 46 require native desktop GUI evaluation and remain `NOT_RUN`. A command existing in the engine registry is never evidence that a real workflow passes.

## Execute

From the repository root with Python 3.10+ and an already-built Rust CLI:

```sh
cargo build --locked -p solidcraft-cli
python3 qa/real_workflows/run.py --validate-only
python3 qa/real_workflows/run.py --cli target/debug/solidcraft-cli --stage 1 --json-out /tmp/arclet-m40-stage1.json
python3 qa/real_workflows/run.py --cli target/debug/solidcraft-cli --json-out /tmp/arclet-m40-results.json
python3 qa/real_workflows/run.py --cli target/debug/solidcraft-cli --include-stress --timeout-s 600 --json-out /tmp/arclet-m40-stress.json
```

Use `--ids A02,H03,G01` for a focused reproduction. Each scenario runs from a fresh CLI process and isolated temporary directory. Fixtures refer to files in this repository. Stressed workflows are opt-in; native GUI cases must be executed manually on an actual desktop. The runner requires a CLI binary: missing binaries do **not** turn tests into successes. It writes observed `PASS`, `FAIL`, or `NOT_RUN` plus command exit codes, numerical measurements, file sizes, timestamps and reproduction commands to the JSON report. It exits non-zero on observed failures.

## Criteria

- Numeric geometry checks compare measured volume, area, bounding dimensions and center of mass with independent analytic expectations when defined; tolerance is per scenario.
- Timeline feature errors fail an automated scenario. A positive return code is not enough.
- Native or JSON save/reopen checks invoke a **new** CLI process and compare body count and volume.
- Export checks verify files are present and non-empty. This is **not** a fidelity or format-conformance certification.
- Snapshot checks verify PNG signature and pixel dimensions, **not** UI correctness.
- CLI runs cannot validate ergonomics, drag behavior, real GPU performance, Rhino/3DM exchange fidelity, or actual UI look.
- Each item has an ID, goal, prerequisites, action, model, expectation, status, reproduction, and issue/PR fields in `catalog.json`. Machine observations belong in output reports, not retroactively invented in the source catalog.

## Stages

| Stage | Entries | Focus |
|---|---:|---|
| 1 | 20 | Initial simple workflows |
| 2 | 40 | Moderately complex workflows |
| 3 | 30 | Advanced cross-feature workflows |
| 4 | 10 | Larger and longer-running workflows |

**CAUTION:** The stage counts are catalog partitions, not proof of passing. A skipped/manual scenario is `NOT_RUN` even when related unit tests have passed elsewhere.

## Baseline and environment

The catalog was prepared against main commit `2ea11ed3f597ec4a5d4d93f9a1c7289057030551`. The original environment has no installed Rust toolchain or network-capable clone; consequently **no Arclet CLI or desktop run has been observed in the preparation session**. Python source syntax was checked separately. CI results, when available, are a distinct evidence source and should be linked by exact workflow run.

## Corrective loop

For failures: collect the JSON report, isolate the case by ID, capture a minimal input, examine command/timeline errors and measurements, fix only the owned feature, rerun that ID, then run stage and related regressions. For large/overlapping issues, link a separate owning PR and keep the case `FAIL` until independently rerun. Do not infer a pass from command reachability or a GUI mock.
