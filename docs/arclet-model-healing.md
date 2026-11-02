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
