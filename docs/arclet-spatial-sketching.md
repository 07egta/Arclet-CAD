# Arclet CAD: 3D sketching and reusable spatial workplanes

This document describes *implemented* command-level workflows on the Mission 32 branch.
The examples use millimetres and rely on the engine's existing 3D wire evaluator. They
do not imply an unconstrained Rhino/NURBS-equivalent 3D sketch solver or a new GUI.

## Coordinate convention

- By default, `sketch.point3d`, `sketch.line3d`, `sketch.polyline3d`,
  `sketch.spline3d`, `sketch.arc3d`, `sketch.move3d` and
  `sketch.insert_fit3d` receive **world** XYZ points.
- `"frame": "sketch"` interprets XYZ in the *evaluated workplane's* local
  coordinates: world = plane.origin + x * plane.x + y * plane.y +
  z * plane.normal(). The axes form a right-handed orthonormal basis.
- The stored geometry is in the owning component's frame, so a placed
  occurrence follows its transform. A world-fixed legacy wire retains
  its own coordinate meaning.
- The `frame` switch does not change sketch constraints or 2D sketch inputs.
  To use it, explicitly opt in per 3D command. The previous world default remains.

## Practical workflow 1: chair backrest spine

1. `sketch.create` with `{"plane":"XY","name":"Chair spine"}`.
2. `sketch.options` with `{"sketch_3d":true}`.
3. `sketch.spline3d` with
   `{"points":[[-180,0,450],[-120,-45,650],[0,-80,780],[120,-45,650],[180,0,450]]}`.
4. Read the returned `curves[0]` wire id, e.g. `w1`.
5. `sketch.insert_fit3d` with
   `{"wire":"w1","index":3,"at":[60,-70,720]}` inserts an adjustable shape point
   without changing the wire id. `sketch.move3d` edits its XYZ position.
6. `sketch.finish`. The curve remains a reference/path for downstream features.

The fit points are joined by sampled centripetal Catmull-Rom interpolation.
They are **not** analytic NURBS CVs; increasing samples does not grant CAD
surface class-A accuracy.

## Practical workflow 2: inclined lamp framework and saved planes

1. `construct.plane.normal` with
   `{"origin":[0,0,100],"normal":[0,1,1],"name":"Lamp section"}`.
   It creates a named construction plane with a normalized normal.
2. `sketch.create` with `{"plane":"Lamp section","name":"Lamp guide"}`.
3. `sketch.options` with `{"sketch_3d":true}`.
4. `sketch.spline3d` with
   `{"frame":"sketch","points":[[0,0,0],[20,0,40],[35,15,100],[0,30,150]]}`.
5. `sketch.finish` and save the design. Named workplanes and wires are
   retained by the existing document serialization.

The same approach applies to chair sections, electronics housings and
surface-guide skeletons.

## Practical workflow 3: 3D pipe centreline

1. `sketch.create` with `{"plane":"XY","name":"Pipe route"}`.
2. `sketch.options` with `{"sketch_3d":true}`.
3. `sketch.spline3d` with
   `{"points":[[0,0,0],[0,0,25],[5,0,50],[10,10,75]]}`.
4. `sketch.finish` and keep the returned `sketch` id and wire id.
5. Use the **existing** `solid.pipe` with
   `{"path_sketch":<sketch id>,"path":["w1"],"diameter":4}`,
   substituting the actual returned wire id. This is a geometric operation,
   not a guarantee that every small-radius or self-intersecting path will succeed.

Analytic circular arcs remain available with `sketch.arc3d`. Use them when
exact curvature, rather than a sampled spline, is required.

## Practical workflow 4: reusable guide-point planes

Create 3D guide curves and a construction plane from their fit points:

1. Start a 3D sketch named `Guides` and add a spline, saving the returned wire id.
2. Add a distinct `sketch.point3d` point, saving its id (usually `q1`).
3. Finish the sketch.
4. `construct.plane.three_points`:
   `{"a":{"sketch":"Guides","point":"w1.start"},
     "b":{"sketch":"Guides","point":"w1.1"},
     "c":{"sketch":"Guides","point":"q1"},
     "name":"Guide cross section"}`
   (substitute actual id strings).
5. `sketch.create` with `{"plane":"Guide cross section"}`.
6. When the fit points move, the construction plane is resolved again.

To keep a plane parallel to XY but at a 3D point's height, use
`construct.plane.through_point` with
`{"plane":"XY","point":{"sketch":"Guides","point":"q1"},"name":"At point height"}`.

For individual 3D fit points, `w1.0`, `w1.1` etc. are zero-based
**indices**, not individually persistent IDs. Inserting or removing fit
points changes later indices, though the wire id is preserved. `w1.start`
and `w1.end` keep their endpoint meaning; `w1.through` is for 3D arcs.
The existing `construct.plane.tangent` operates on supported CAD faces;
`construct.plane.along_path` makes a plane perpendicular to a path.
Existing `sketch.include_3d`, `sketch.project`, and `sketch.intersect`
are the projection/intersection/reference workflows.

## Boundaries and safety

- Enable 3D Sketch through `sketch.options` before drawing outside a plane.
- `sketch.polyline3d` stores joined straight wire segments with separate ids,
  not one new CAD polyline B-rep entity.
- `closed:true` requires at least three distinct input points; do not manually
  repeat the first one. Closed smooth splines preserve their repeated endpoint
  internally.
- `sketch.insert_fit3d` and `sketch.remove_fit3d` work only on *drawn*
  splines with three or more fit points, not linked references, straight
  segments, or analytic arcs. Insertions must be interior, at most 500 fit
  points. Bad coordinates and degeneracies fail rather than partially editing.
- 3D sketch wires are not the 2D constrained/profile entities. Advanced 3D
  tangency, curvature constraints, NURBS control points, and arbitrary
  3D curve/surface intersection require separate numerical implementations.
- 3D wire-to-Loft/Sweep rail suitability is determined by the existing
  modelling engine and the specific geometry. Do not assume any spatial path
  succeeds merely because it can be drawn.

## Regression commands

```sh
cargo test --profile ci --locked -p solidcraft-geom --lib spatial_workplane_axes_remain_orthonormal_and_recover_large_world_coordinates
cargo test --profile ci --locked -p solidcraft-engine --lib sketch_3d::frame_tests::
cargo test --profile ci --locked -p solidcraft-sketch --lib wire::tests::
cargo fmt --all -- --check
```

These tests check orthonormality, large finite coordinates, local/world
placement conversion, construction plane rebuild, spline fit insertion and
removal, preserved IDs, document save/reopen, undo/redo, and rejected edits
leaving documents unchanged. Numerical tests cannot substitute for future
interactive viewport tests.
