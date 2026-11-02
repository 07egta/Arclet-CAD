# Mission 30: Arclet CAD reliability and numerical QA

Scope: **reviewable evidence**, not product-wide certification.
Main baseline at review: 2ea11ed3f597ec4a5d4d93f9a1c7289057030551 (2026-11-02).

## Existing quality infrastructure (inspected, not newly rerun)

- docs/robustness.md: deterministic feature-sequence fuzzing, per-seed process and timeout,
  Boolean identity, bounded golden samples and minimisation.
- tests/robustness/goldens.json: **96 recorded seeds; 83 ok, 13 failed**. These are expected
  stored outcomes, **not 96 passing tests**. Representative known failures: seed 2 Boolean
  Cut returns no result; seed 6 shell/offset contact error; seed 45 union has implausible volume.
  These belong to the kernel owner; do not bless or hide them in this PR.
- docs/oracle.md: analytic and independent-reference oracles, whose historical results are
  not evidence of reruns in this PR. Fixtures 01-box and 24-user-parameters drive the QA CLI.
- TEST_RESULTS.md: distinguishes prior Windows tests, headless binary tests and source-only
  changes. Not all historical tests certify this branch.
- Existing open PRs claim NURBS, sketch solver, B-rep Boolean and file-I/O responsibilities;
  this PR touches only geometry primitives, existing tests and isolated QA infrastructure.

## P1: incorrect mesh volume with large world coordinates

Input: oriented parallelepiped with integer-valued edge vectors a=(7,3,2),
b=(1,5,2), c=(2,1,6), translated to (1e9,1e9,1e9) mm.
Analytical signed volume is det(a,b,c) = 172 cubic mm.
Old Mesh::measure computes the tetrahedra scalar triple products relative to the
global origin; products cancel at large magnitudes, losing significant bits.

**RED confirmed by compiled Rust (Linux rustc 1.99.1):**
Actions run 41426285424, job 125213569318:
- volume test: **13,000,000,170.666666** observed against 172 expected;
- fixed-seed corpus case 1: **-3,333,332,309.3333335** against 636 expected;
- both new mesh-volume tests failed.

Fix: use a finite mesh vertex as the local origin for signed tetrahedra; add
world translation back to the measured centroid. Geometry and public APIs are
unchanged; the independent determinant oracle measures a closed triangle mesh.

New existing-file regressions in crates/geom/src/tests.rs:
- one exact determinant fixture under translations 0, 1e9 and 1e12;
- 256 bounded, deterministic affine cases with analytical determinants;
- finite centroid invariants and clear absolute error tolerances.

This is *finite sampled evidence*, not mathematical proof for all meshes.
Signed volume is only defined as an enclosed volume for a closed, oriented mesh.
Coordinates so large that vertex differences overflow remain a risk.

## P2: AABB centres overflow for finite endpoints

Input: min.x = max.x = 1e308, both finite. Old (min + max) * 0.5
returns infinity despite a finite true midpoint.
**RED confirmed in the same compiled Rust job** (AABB regression failed).
Fix: prefer min + (max-min)*0.5; if the span overflows, use min*0.5 + max*0.5.
The patch applies identically to 2D and 3D bounding boxes. True spans that
cannot be represented still require safeguards at use sites.

## New native headless acceptance harness (pending compiled CLI test)

qa/scripts/ultimate_quality_cli_smoke.py executes the built solidcraft-cli in
fresh processes, imposing a timeout on each call and writing machine-readable
evidence even on failure. The dedicated Linux CI job will exercise:
- Oracle recipe -> sketch -> extrusion -> native .sc save -> new-process eval;
- STL export: binary record lengths, finite values and independent bounds checks;
- fresh-process STL re-export: exact triangle record comparison;
- STEP Part21 envelope check and fresh-process import/volume oracle;
- document -> exported command history -> replay -> independent save;
- Undo/Redo restores original volume;
- parameter-driven dimension edit and analytical holed-prism volume;
- malformed design and overflowed fuzz seed range are explicitly rejected;
- historical baseline fuzz seed 0 with 8 feature steps, tracking caught panics.

These verify a CLI/headless workflow, **not** interactive GUI, file choosers,
GPU drivers, native window lifecycle or visual appearance.

## Scope, environment, and review gate

CI: .github/workflows/arclet-qa-numerics.yml with Linux/Windows bounded numerical
tests (full geom library suite) and a separate Linux real-CLI smoke job.
Runs are capped at 35/55 minutes, one Rust test thread, fixed numerical corpus,
and per-process CLI timeouts. Different commits may produce different results;
inspect their actual CI run IDs instead of borrowing historical green results.

The local inspection container has no Rust toolchain and no direct GitHub DNS.
The independent Python demonstration is not a compiled Rust test.
Rust test evidence is supplied by GitHub Actions, not guessed.
No full TSAN, ASAN, long-run resource stress, large hostile file suite, or
real interactive GUI testing has been performed in this slice.

Commands for reproduction:

    cargo test --profile ci --locked -p solidcraft-geom --lib ultimate_quality_ -- --test-threads=1
    cargo test --profile ci --locked -p solidcraft-geom --lib -- --test-threads=1
    cargo build --profile ci --locked -p solidcraft-cli --bin solidcraft-cli
    python3 qa/scripts/ultimate_quality_cli_smoke.py --cli target/ci/solidcraft-cli --report headless-evidence.json

Do not bless robustness goldens, suppress existing tests, merge automatically
or push directly to main. Keep the PR a draft while numerical CI or the
end-to-end CLI workflow is failing or unverified.

## Follow-up precision check (post first green run)

A separate high-precision Python Decimal oracle sampled 30,000 finite sorted AABB endpoint pairs spanning ~1e-300 to 1e300. The first overflow-safe formula, low + (high-low)/2, still lost >1.5 ULP in 3,690 near-cancelling mixed-sign cases (worst 16,384 ULP). The follow-up uses (low+high)/2 where the signs differ (the sum cannot overflow), and low + (high-low)/2 where both signs agree (the difference cannot overflow). In that independent sample the revised formula differed from the correctly rounded Decimal midpoint by at most one ULP; this is NOT an exhaustive numerical proof. A deterministic binary-endpoint Rust fixture with a 16,384-ULP old error and a subnormal tie case was added. Compile/test result for this follow-up must be verified from the future branch CI; do not claim green from Python alone.
