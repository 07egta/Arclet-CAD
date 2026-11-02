# Arclet CAD — Geometry diagnostics and conservative mesh repair

Mission 36. This branch extends integrity checks without changing Boolean operators,
the design-file format, or imported B-rep geometry. Base: `main` at
`2ea11ed3f597ec4a5d4d93f9a1c7289057030551`.

## Baseline and design boundary

- `mesh.check` already reports repeated/near vertices, zero-area triangles, naked
  edges, non-manifold edges and boundary components, but **does not verify winding,
  duplicate faces, invalid references, or self-intersection**. Its `closed` flag
  is not sufficient evidence that a watertight printable solid exists.
- Imported STL/3MF mesh bodies are indexed, and their importer already merges
  exactly coincident vertices and drops zero-area facets. Repair on a loaded
  model can only act on defects surviving this normalization.
- B-rep `Body::validity` checks shell conditions, unmeshed faces and volume,
  and `Body::crossed_face_loops` detects planar trim-loop crossings.
  Rebuilding invalid analytic curve/surface references is out of scope for this
  first patch; such bodies must be reported rather than guessed into solids.
- Full root instructions reference contributor skills missing from this
  snapshot. `AGENTS.md`, `docs/policy.md` and actual source constraints govern.

## Safety invariants

1. Inspection is **read-only**. A repair proposal works on cloned triangle
   indices and positions; failure never changes the design, import file or
   active selection/history.
2. Safe repairs are limited to *exact* coincidence: removing zero-area or exact
   duplicate facets, indexing identical positions only once, and orienting
   otherwise-manifold triangles consistently. No snapping across a gap by
   tolerance alone, no speculative hole fill, no shell or Boolean changes.
3. Invalid references, unresolved manifold violations, crossings, or ambiguous
   winding block successful watertight certification. Large or small gaps
   remain visibly open until repaired by an explicit geometry-editing tool.
4. A separate validation pass must inspect the candidate after changes.
   Reports include before/after counts, surface area, signed volume where
   meaningful, and zero vertex displacement (no coordinate relocation).
5. Export, if offered, must create a **new** file and refuse overwrites, never
   overwrite the original mesh or save the design implicitly.
6. Never report success from the number of fixes alone. Exact repair can be a
   partial improvement yet still require human modeling intervention.

## Required regression coverage

A normal closed tetrahedron/solid; a broken shared edge; a duplicated face;
degenerate triangles; non-manifold edges; an inverted face; a small
tolerance-near gap, a large gap and an unrepairable input. The final
post-repair report must be reproducible from its output alone.

## Acceptance / current limitations

Do not describe a triangle mesh as a certified B-rep solid. Strict triangle
penetration tests have practical budgets; an unexamined large model must
report that the self-intersection check is **incomplete**, not that it passed.
Do not treat tessellation coincidence as exact NURBS/B-rep topology repair.

Validation target: focused cargo unit tests, clippy, rustfmt, hostile inputs,
and a new-revision CI run, with `main` unchanged. All test and numerical
results must be reported as measured or marked unverified.

## Implemented patch, numerical evidence, and gaps

Current changes in `crates/engine/src/cmd/mesh.rs`:

- `mesh.check` adds exact/`tolerance`-based `duplicate_faces`, `winding_conflicts`,
  `invalid_vertex_references`, and `short_edges`. Its `closed` key is
  conservative with respect to these checks, not a full geometric certificate.
- `mesh.repair_preview` builds a separate candidate of retained triangle
  indices after filtering exact duplicate or degenerate triangles. It returns
  `before` and `after` reports from independent `analyze` calls.
  It **does not apply or export** the candidate. Vertex coordinates never move.
- Source-based regression fixtures in the same `mesh.rs` test module exercise
  7 cases. The expected face counts are exact; for a duplicate face on the
  four-triangle tetrahedron the predicted diagnostics are 1 duplicate face and
  3 non-manifold edges, while reversing one facet predicts 3 orientation
  conflicts. These are test expectations, **not yet runtime measurements**.
- For a synthetic vertex displacement of 0.00001 mm, the 0.001 mm tolerance
  comparison predicts 1 repeated/near vertex; a 0.25 mm gap remains open
  at tolerance 0.001 mm. The source coordinates remain exactly unchanged.

Known limitations:
- No self-intersection certification, duplicate edge-curve matching on analytic
  B-reps, trim-loop validation, surface-reference repair, finite-volume proof,
  or automatic hole filling. Existing `Body::validity` and
  `Body::crossed_face_loops` are separate mechanisms.
- Triangle topology classification does not establish manifold vertices
  (vertex-only pinches), nor global connectivity of the shell.
- Tolerance welding is a **diagnostic hypothesis** only, not a recommended
  automatic coordinate edit. It may change the apparent topology.
- The preview cannot be used as an edited model and does not report volume
  deltas, because it does not output a repaired B-rep or certified mesh solid.
- Build, rustfmt, clippy and actual test results are not available from this
  execution environment; PR remains a draft pending verification.
