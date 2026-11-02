# Mission 47: bounded organic T-Spline editing

## Mathematical scope (not SubD)

The existing `solidcraft-geom::tspline` implementation has **odd bi-degrees**
(typically cubic), a rectangular knot index domain, positive finite rational
weights and restricted T-junction/analysis-suitable refinement. It is *not*
arbitrary polygon subdivision, extraordinary-vertex Catmull–Clark, or a general
watertight NURBS conversion. The existing `refine` performs knot-preserving
refinement where admitted; an incompatible topology is rejected.

## Added editor operations

- `translated_controls(indices, delta_mm)` validates the whole selection
  before constructing an independent spline; repeated/out-of-range indices and
  nonfinite or excessive coordinates refuse without partial mutation.
- `positioned_control(index, position_mm)` sets Cartesian position while
  preserving the vertex's rational weight.
- `verified_refine(cuts, lattice, tolerance_mm)` uses the existing T-spline
  exact refinement, then evaluates the before/after surfaces on a bounded
  2–65 by 2–65 parameter grid. It reports the **maximum sampled** point
  deviation. Finite sampling cannot prove global equality or detect all
  foldovers/self-intersections. The existing exact knot-refinement algorithm is
  the mathematical preservation mechanism.

## Desktop UI scope

`Organic T-Spline Workshop`, accessible from the desktop app chrome,
is an **opt-in, in-memory isolated editor**, not a new saved CAD feature.
It shows a bicubic tensor example with control edges and evaluated surface
isocurves, highlights selected controls, supports Shift-multiselect and
XY point dragging, applies bounded Z increments, a sample exact refinement
and a maximum-32 undo/redo state history. This is an entry point for further
integration with document features, but **edits do not change an engineering
body and are not saved in a .sc document.** There is no marketing claim
that ordinary SolidCraft bodies now support sculpting.

Mouse handle behavior is screen-plane/orthographic in this workshop only,
not the full ray/plane gizmo; preview renders evaluated wireframe rather than
a shaded production tessellation. Drag updates are undoable per frame, not
yet grouped as one gesture. It does not provide edge/face editing, local
symmetry, deletes, certified foldover diagnostics, arbitrary SubD or
spline-to-B-rep conversion.

## Numerical acceptance and evidence

The existing `tspline/tests.rs` now exercises stylized chair-seat,
ergonomic handle, bottle panel, lamp shade and wave panel rational grids;
checks exact weights across multi-point moves, surface response to changed
controls, reversible displacement, invalid/duplicate selection refusal,
degenerate normal handling, and knot-refinement positional drift.
The original T-Spline unit tests already check NURBS/tensor equivalence,
partition of unity, local preservation and Bézier extraction.

CI runs targeted geometry tests, geometric crate compile, and egui
compile from the PR branch. **No passing CI or human desktop screenshot
is claimed until actually observed.** The current tool environment
lacks Rust and a route to clone GitHub; the PR stays Draft.

## Remaining acceptance blockers

1. Make a first-class command and persist T-mesh/control positions in
   a validated versioned design feature with a planned file-format change.
2. Route selection and dragging through the unified viewport and revisioned
   editing, group one gesture into one Undo/Redo step.
3. Connect edited surfaces to bounded tessellation and on-demand model
   evaluation for actual CAD bodies.
4. Implement certified surface foldover/self-intersection diagnostics, degree
   and boundary continuity checks and stress tests.
5. Test product workflows in an actual Windows/macOS application with
   screenshots and .sc save/reopen assertions.

**No other CAD library, external code or GPL/LGPL/AGPL dependency was
introduced; existing UI/policies remain the default.**
