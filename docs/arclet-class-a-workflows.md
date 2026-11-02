# Arclet Surface Workflows — Mission 45

Scope: safeguard ordered surface selections and preserve the current parametric modeling workflow, without changing NURBS mathematical kernels being modified by other open PRs.

## Existing workflows and commands

- A — Multiple section Loft: `surface.loft` takes 2–50 ordered sections, one per sketch or a named B-rep body edge chain. Edit source sketches or the history inputs, then rebuild; use `inspect.measure` and sample-grid checks. The existing `surface_loft_open_and_closed_sections` test covers areas, undo/redo and JSON reload.
- B — G0/G1/G2 Match / Blend: `surface.match` and `surface.blend`. Use `inspect.continuity` for independently sampled G0 gap, G1 normal-angle, G2 normal-curvature jump, and `inspect.zebra` for visualization. Do not treat requested order as measured achievement. `surface.match` has its own numerical and undo/reload tests.
- C — Fill: `surface.patch` and `surface.boundary`; two through four loop curves or a U/V network. More complicated boundaries depend on the existing general-fill/network implementations and are **not** universally guaranteed.
- D — Re-edit: `timeline.edit`, sketch/control point editing, `parameters.change`, undo/redo and feature recomputation. For modified source shapes, measure again.
- E — Save: document JSON roundtrip and existing native B-rep/NURBS serialization pathways; arbitrary STEP/3DM mathematical fidelity is outside the scope of this change.

## Bug corrected

Some commands formerly converted a JSON edge/face selection using `filter_map(vec3)`. An invalid point in the middle vanished silently, altering the requested chain without an error. Loft, Blend, and Match now reject malformed lists atomically with the offending field and index. This is especially important for selection order and history-preserving modifications.

The new regression tests assert that malformed picks do not modify the document, alongside existing Loft geometry, Match seam numerical validation, cancellation, undo/redo and cold-reload tests.

## Numerical interpretation

The seam diagnostic is *sampled* at its requested stations; it reports a positional gap in mm, normal angle in degrees and normal curvature jump in 1/mm. It does not bound extrema between stations, prove an entire G2 connection, certify G3, or prove Class-A quality. Zebra patterns are visual evidence, not a numerical certificate. Finite sample point coverage and topology/trim validity remain outstanding for complex patches.

## Verification

Windows/macOS native CI: `cargo test --locked --profile ci -p solidcraft-engine --lib cmd::surface::tests` and `cmd::surface_match::tests`. The current environment lacks Rust toolchains, and CI outcomes must be checked before merging. No screenshots are claimed.

Remaining open work: 5+ boundary fills, surface self-intersection and foldover classification, near-singular NURBS parameter derivatives, principal-curvature distribution, comprehensive G2/G3 matching across entire seams, precise trimmed-face interchange, richer viewport selection order and per-station failure visualization. No GPL/LGPL/AGPL dependencies added.
