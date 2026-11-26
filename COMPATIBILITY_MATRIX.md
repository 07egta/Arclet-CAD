# Compatibility is feature-specific

| Capability | Current evidence / status |
|---|---|
| Independent Arclet desktop shell/settings | Existing Linux app executed; no remote Melt3D changes |
| NURBS curve evaluation / G0-G3 curve-end analysis | Baseline implementation retained; numerical hardening patch and 7 regression tests added, uncompiled |
| Curve Rebuild | Existing GUI accept/undo/redo/save/reload tested on one sample; deviation is sampled, not a certified maximum |
| Native `.sc` design format | Curve and embedded node mesh cold round-trips verified in original binary |
| Node JSON format | Version-1 original format retained; normal/invalid/resource-limit GUI tests pass |
| 2,000-node evaluation | Simple Number chain verified; no claim for 2,000 complex surfaces |
| Mission 14 parametric history validation | Draft source patch on `feat/arclet-parametric-history`: reports explicit missing/forward/suppressed/cyclic feature dependencies and guards result reuse; regression tests added but not compiled or run in this authoring environment. Not a verified release feature. |
| Unsaved graph Close protection | Defect reproduced; patch + 11 state/UI tests added, uncompiled |
| Normal-exit graph preferences recovery | Edited value 31 restored as unsaved in original binary; crash/power-loss not tested |
| STL export | Node mesh 192 triangles exactly preserved across native save/reload; torus meshes finite and closed by edge incidence |
| STEP | One analytic torus exported/imported/re-exported; mm units retained; representation changes; partial smoke coverage only |
| IGES / OBJ / GLB / glTF | Not exercised in this session; no new compatibility claim |
| 3DM | Import (desktop only) goes through the existing cadmpeg path; no export. A separate in-house reader (`crates/io/src/rhino3dm.rs`) lists the header and top-level chunks only, no geometry yet, and is not used by import. Fidelity of the cadmpeg path is not measured |
| GH / GHX | Unsupported in this delivery |
| RhinoCommon / Python / .rhp / .gha binary plugins | Unsupported in this delivery |
| G3 surface continuity | Not implemented/verified as a general capability |
| Class-A certification / xNURBS / PolySplines equivalence | Not claimed; names and visible smoothness do not establish equivalence |
| Linux original executable | GUI workflow tests on Xvfb + software renderer |
| Linux new Rust patches | Not built |
| Windows / macOS | No new executable; no native launch test |
| WebAssembly / browser | Historical baseline code-check records retained; not rerun; actual browser unverified |
| Intel GPU / performance comparison with Rhino or Plasticity | Not measured |
| Bambu A1 / A1 mini G-code / AMS | Not tested; do not print unverified generic output |

File compatibility, API compatibility and native plugin execution compatibility are not interchangeable.

### Mission 14: parametric-history scope (unverified source patch)

The `Document::feature_dependency_graph()` API is a read-only audit of **explicit** references in the Rust feature walk, sketch projection links and certified sketch face origin. It reports source IDs, downstream IDs, missing sketch and feature references, incorrect sketch targets, unsafe forward sketch references, suppressed sources, and direct or indirect active cycles. `Model::evaluate()` checks blocking diagnoses before returning unchanged or cached results for the affected feature, retaining an error on the feature rather than treating old success geometry as current. The existing per-feature fingerprints, state digest, bounded cache and `stale` preview mechanism remain the evaluator's implementation of incremental reuse.

**Not covered by the explicit graph:** arbitrary body name/face/edge picks, unverified fallback geometry, exact topological identity for every Boolean split, a proof of all indirect dependencies, optimal global minimal recomputation, asynchronous revision-pinning beyond the existing context/fingerprint checks, or a complete Rhino-compatible feature history. Geometry-based references continue to be resolved and warned about by the existing `name_lookup`, `lineage`, `face_anchor` and evaluators.

The Mission 14 patch does not change the serialized document format or `FeatureId` representation. New tests cover repeated parameter edits, missing-source cache invalidation, reorder/suppression, cycles, mirror dependencies and long chains, but execution and UI validation were not performed when the draft patch was authored. This is **not** a successful-test or production-readiness claim.


## Rhino-class feature coverage

This is a feature inventory, not a support claim. No 3DM, Grasshopper (GH/GHX), RhinoCommon, Python, or .rhp/.gha plugin support is claimed. Class-A and G3 are not certified (see the rows above).

**Sources and method.** Command IDs come from the 503-entry engine registry. Reachability comes from the menu and toolbar wiring in `apps/arclet/src/app.rs` (curves, curve tools, surfaces, solids, transforms, analysis, views, File, Select, Isolate) and the explicit aliases in `apps/arclet/src/commands.rs`. Rows were mapped by command label and by the menu button name. No behaviour was tested for this table.

**Tiers**

- `main-works`: registered, and wired into an Arclet menu, toolbar or alias. Sketch-mode-only tools are marked in the feature name.
- `partial`: registered, but Arclet exposes only part of it, such as one method out of several or one menu action.
- `engine-only`: registered in the engine, with no Arclet menu or toolbar item.
- `missing`: no engine command.

**Not counted as reachable.** The command bar also accepts raw engine IDs and labels, and the "All tools" palette lists the registry. Counting either would make every registered command reachable, which would make the tiers meaningless. Aliases add no IDs beyond the menus.

**Other limits**

- Desktop only: `sketch.spline.rebuild`, `inspect.curve_continuity`, `surface.match` and `surface.fair` are blocked on WebAssembly in `app.rs`, so their `main-works` tier does not hold in the browser build.
- The menu button "Curve continuity" (`arclet.curve_continuity`) runs `inspect.curve_continuity`, which is a continuity analysis. It does not join curves, so joining is listed as missing.
- `ui.isolate` is a UI context item, not one of the 503 registry IDs. It is counted as `main-works` and marked.
- The File menu's New, Open, Save, Save As and export actions go through UI helper functions. Their IDs (`doc.open`, `file.new`, `file.save`, `file.save_as`, `file.export`, `file.export_view_dxf`, `file.save_mesh`) are matched by label and not traced in code.

### Curve

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Line | sketch.line | main-works |
| Polyline (connected segments) | sketch.chain | engine-only |
| Arc (center, 3-point, tangent) | sketch.arc.center_point, sketch.arc.three_point, sketch.arc.tangent | main-works |
| Circle, center and diameter | sketch.circle.center | main-works |
| Circle, 2-point, 3-point, 2-tangent, 3-tangent | sketch.circle.two_point, sketch.circle.three_point, sketch.circle.two_tangent, sketch.circle.three_tangent | engine-only |
| Rectangle, corner | sketch.rectangle.two_point | main-works |
| Rectangle, center, 3-point, rounded | sketch.rectangle.center, sketch.rectangle.three_point, sketch.rectangle.rounded | engine-only |
| Ellipse | sketch.ellipse | main-works |
| Polygon, inscribed, circumscribed, edge | sketch.polygon.inscribed, sketch.polygon.circumscribed, sketch.polygon.edge | partial |
| Conic | sketch.conic | engine-only |
| Interpolated curve through points | sketch.spline.fit_point | main-works |
| Control-point curve, degree 3 | sketch.spline.control_point | main-works |
| Control-point curve, degree 5 | sketch.spline.control_point_5 | main-works |
| Add point to spline (sketch mode) | sketch.spline.insert_point | main-works |
| Rebuild curve | sketch.spline.rebuild | main-works |
| Blend curve | sketch.blend_curve | main-works |
| Offset curve (sketch mode) | sketch.offset | main-works |
| Trim curve (sketch mode) | sketch.trim | main-works |
| Trim along path | sketch.trim.path | engine-only |
| Fillet between curves | sketch.fillet | main-works |
| Chamfer between curves | sketch.chamfer.distance_angle, sketch.chamfer.equal_distance, sketch.chamfer.two_distance | partial |
| Extend curve | sketch.extend | engine-only |
| Split or break curve | sketch.break | engine-only |
| Join curves | — | missing |
| Project curve onto surface | sketch.project, sketch.project_to_surface | engine-only |
| Intersection curve | sketch.intersect, sketch.intersection_curve | engine-only |
| Isoparametric curve | sketch.isoparametric_curve | engine-only |
| 3D line, 3D spline, 3D point | sketch.line3d, sketch.spline3d, sketch.point3d | engine-only |
| Helix or spiral | — | missing |
| Curve degree change or simplify | — | missing |
| Explode curve into segments | — | missing |

**Top gaps**

- Join curves: no join command exists. The Curve menu's Curve continuity entry analyzes curve ends instead.
- Helix, spiral, and degree change or simplify: no engine command.
- Explode: no engine command.
- Conic exists only in the engine, not in the menus.
- Polyline is unconfirmed. Only the `sketch.chain` label was read, not its behaviour.

### Surface

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Loft | surface.loft | main-works |
| Sweep (profile along rail) | surface.sweep | main-works |
| Extrude curve to surface | surface.extrude | engine-only |
| Revolve | surface.revolve | main-works |
| Patch (fill through boundary) | surface.patch | main-works |
| Blend between edges | surface.blend | main-works |
| Boundary surface from closed curves | surface.boundary | main-works |
| Edge or network surface | — | missing |
| Ruled surface | surface.ruled | main-works |
| Offset surface | surface.offset | main-works |
| Trim surface | surface.trim | main-works |
| Untrim surface | surface.untrim | main-works |
| Extend surface | surface.extend | main-works |
| Join (stitch) surfaces | surface.stitch | main-works |
| Unjoin surfaces | surface.unstitch | main-works |
| Thicken surface to solid | surface.thicken | main-works |
| Fair surface | surface.fair | main-works |
| Match surface continuity | surface.match | main-works |
| Flip surface normal | surface.reverse_normal | engine-only |
| Spun profile to surface | sketch.spun_profile | engine-only |
| Fillet surface | — | missing |
| Rebuild surface | — | missing |
| Split surface | — | missing |
| Surface from mesh | — | missing |

**Top gaps**

- Edge or network surface: no engine command.
- Fillet surface and rebuild surface: no engine command.
- Split surface: no engine command. `solid.split_face` works on solids only.
- Surface from mesh: no engine command.
- Extrude exists only in the engine. Ruled, untrim, extend and unjoin are in the Surface menu.
- Two-rail sweep is not confirmed by the command label.

### Solid

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Box | solid.box | main-works |
| Sphere | solid.sphere | main-works |
| Cylinder | solid.cylinder | main-works |
| Cone | solid.cone | main-works |
| Torus | solid.torus | main-works |
| Extrude | solid.extrude | main-works |
| Revolve | solid.revolve | main-works |
| Loft | solid.loft | main-works |
| Sweep | solid.sweep | main-works |
| Pipe | solid.pipe | main-works |
| Fillet | solid.fillet | main-works |
| Chamfer | solid.chamfer | main-works |
| Boolean union, difference, intersection | solid.combine, solid.remove | partial |
| Press-pull | solid.press_pull | main-works |
| Shell | solid.shell | main-works |
| Draft angle | solid.draft | engine-only |
| Hole | solid.hole | main-works |
| Thread | solid.thread | engine-only |
| Split body | solid.split_body | main-works |
| Split face | solid.split_face | engine-only |
| Offset face | solid.offset_face | engine-only |
| Replace face | solid.replace_face | engine-only |
| Delete face | face.delete | engine-only |
| Rib | solid.rib | engine-only |
| Web | solid.web | engine-only |
| Emboss | solid.emboss | engine-only |
| Coil | solid.coil | engine-only |
| Bounding solid | solid.bounding_solid | engine-only |
| Boundary fill | solid.boundary_fill | engine-only |
| Polysurface to solid, cap open edges | — | missing |
| Explode polysurface | — | missing |
| Variable-radius fillet | — | missing |

**Top gaps**

- Polysurface to solid and capping open edges: no engine command.
- Explode polysurface: no engine command.
- Variable-radius fillet: no engine command.
- Boolean: only the Combine action is in the menu. Difference and intersection are not confirmed in the menu.
- Thread, draft, offset face, rib, web, emboss, coil and bounding solid exist only in the engine.

### Mesh

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Convert surface or solid to mesh | mesh.convert | main-works |
| Insert mesh file | file.insert_mesh | main-works |
| Mesh repair (holes, weld, normals) | — | missing |
| Mesh reduce, smooth, boolean | — | missing |
| Mesh to NURBS | — | missing |
| Mesh check (naked edges, non-manifold, degenerate triangles, duplicate vertices) | mesh.check | main-works |

**Top gaps**

- Mesh repair, reduce, boolean, smooth and mesh-to-NURBS: no engine commands.
- Mesh check reports naked and non-manifold edges, degenerate triangles, duplicate vertices, boundary loops and whether the mesh is closed. It is read-only: it does not repair, weld or reorient.
- Mesh convert and insert are in the Mesh and File menus.
- Mesh export to STL and 3MF is listed under File I/O.

### Transform

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Move and copy | solid.move | main-works |
| Rotate (axis and angle, via Move/Copy tool) | solid.move | main-works |
| Scale | solid.scale | main-works |
| Mirror | solid.mirror | main-works |
| Linear array (2D and 3D) | sketch.pattern.rectangular, solid.pattern.rectangular | main-works |
| Polar array (2D and 3D) | sketch.pattern.circular, solid.pattern.circular | main-works |
| Array along path | solid.pattern.path | engine-only |
| Align or orient | solid.align | engine-only |
| Transform (sketch) | sketch.transform | engine-only |
| Scale 2D sketch | sketch.scale | engine-only |
| Drag geometry or occurrence | sketch.drag, occurrence.drag | engine-only |
| Twist, bend, taper | — | missing |
| Shear, flow, morph, cage deform | — | missing |

**Top gaps**

- Twist, bend, taper, shear, flow, morph and cage deform: no engine commands.
- Path arrays and Align exist only in the engine. Linear and polar arrays are in the Curve and Solid menus.

### Analyze

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Zebra analysis | inspect.zebra | main-works |
| Environment reflection map | inspect.environment_map | main-works |
| Curvature map | inspect.curvature_map | main-works |
| Curvature comb | inspect.curvature_comb | main-works |
| Surface continuity (G0, G1, G2) | inspect.continuity | main-works |
| Curve end continuity (G0 to G3) | inspect.curve_continuity | main-works |
| Measure | inspect.measure | main-works |
| Draft analysis | inspect.draft | main-works |
| Minimum radius | inspect.minimum_radius | main-works |
| Section | inspect.section | main-works |
| Isocurve | inspect.isocurve | main-works |
| Interference | inspect.interference | main-works |
| Clearance | inspect.clearance | main-works |
| Compare bodies | inspect.compare | main-works |
| Center of mass | inspect.center_of_mass | main-works |
| Mass, area and volume properties | component.properties | engine-only |
| Accessibility (pull direction) | inspect.accessibility | main-works |
| Parameter sweep | inspect.parameter_sweep | engine-only |
| Deviation from mesh or scan | — | missing |
| Object check (naked edges, self-intersection) | — | missing |

**Top gaps**

- Object check for naked or self-intersecting edges: no engine command.
- Deviation from a mesh or scan: no engine command.
- Mass properties are only available through the component command, which is not in the menus. Section, isocurve, interference, clearance, minimum radius, draft, center of mass, accessibility and compare are in the Analyze menu.

### Dimension/Annotate

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Sketch dimension (linear, angle, radius) | sketch.dimension | main-works |
| Radius and diameter toggle | sketch.toggle_diameter | engine-only |
| Driven dimension | sketch.toggle_driven | engine-only |
| Move dimension text | sketch.dimension_text | engine-only |
| Sketch constraints (coincident, tangent, parallel, perpendicular, equal, symmetry, horizontal/vertical, midpoint, concentric, fix) | sketch.constraint.coincident, sketch.constraint.tangent, sketch.constraint.parallel, sketch.constraint.perpendicular, sketch.constraint.equal, sketch.constraint.symmetry, sketch.constraint.horizontal_vertical, sketch.constraint.midpoint, sketch.constraint.concentric, sketch.constraint.fix | engine-only |
| Text annotation | sketch.text | engine-only |
| Leader or callout | — | missing |
| Hatch or fill pattern | — | missing |
| Model-space linear annotation (outside sketches) | — | missing |
| Layout or drawing sheet with views | — | missing |

**Top gaps**

- Leader or callout, hatch, model-space annotation and layout sheets: no engine commands. `sketch.drawing_bounds` only sets a sketch size.
- Sketch dimension is in the Curve menu. Sketch constraints exist only in the engine. No menu or toolbar item shows the constraints.

### Layers/Display

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Show or hide objects (menu offers Show All only) | browser.visibility | partial |
| Select, select filter, clear selection | select.set, select.filter, select.clear | main-works |
| Isolate | ui.isolate | main-works |
| Standard views and fit (top, front, right, isometric) | view.fit, view.top, view.front, view.right, view.isometric | main-works |
| Back, bottom and left views | view.back, view.bottom, view.left | engine-only |
| Named views (save, apply, list) | view.save, view.apply, view.list | main-works |
| Perspective or parallel projection | view.perspective | main-works |
| Grid | view.grid | main-works |
| Visual style (shaded, wireframe, rendered) | view.visual_style | main-works |
| Ambient occlusion and ground shadow | view.ambient_occlusion, view.ground_shadow | engine-only |
| Browser groups and folders | browser.group, browser.rename_group, browser.ungroup | engine-only |
| Object colour and appearance | appearance.assign | engine-only |
| Layers (named, coloured, locked) | — | missing |
| Lock and unlock objects | — | missing |

**Top gaps**

- Layers with names, colours and lock states: no engine command. Browser groups are the nearest equivalent and exist only in the engine.
- Lock and unlock objects: no engine command.
- Show or hide is partial. The menu offers Show All only.
- Named views, grid, perspective and visual style are in the View menu. Browser groups exist only in the engine.

### File I/O

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| New | file.new | main-works |
| Open | doc.open | main-works |
| Save | file.save | main-works |
| Save As | file.save_as | main-works |
| Export STEP | file.export | main-works |
| Export STL and 3MF (mesh) | file.save_mesh | main-works |
| Import STEP | file.insert_step | main-works |
| Import mesh | file.insert_mesh | main-works |
| IGES import and export | doc.open, file.export | main-works |
| OBJ import and export | doc.open, file.export | main-works |
| glTF / GLB import only | doc.open | main-works |
| DXF sketch import and view DXF export | sketch.insert_dxf, file.export_view_dxf | main-works |
| SVG sketch import only | doc.open | main-works |
| PLY / XYZ / PTS point cloud import only | doc.open | main-works |
| Rhino 3DM import only (desktop) | doc.open | main-works |
| Recover unsaved design | doc.recover, doc.recovery_list | engine-only |
| Send to slicer | file.send_to_slicer | engine-only |
| PDF or drawing export | — | missing |

**Top gaps**

- 3DM imports only (desktop; no export). glTF/GLB, SVG and PLY/XYZ/PTS import only. OBJ and IGES import and export.
- PDF or drawing export: no engine command.
- Recovery and sending to a slicer exist only in the engine. DXF sketch export is not in a menu; DXF export is for the current view.

### Render

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Render image | render.start | main-works |
| Export render image | render.export | main-works |
| Frame render subject | render.camera.fit | main-works |
| Render settings | render.settings.get, render.settings.set | engine-only |
| Render camera | render.camera.set | engine-only |
| Physical material and appearance | material.create, material.assign | engine-only |
| Studio lights and environment | render.preset.apply, render.library.environment.apply | engine-only |
| Ground plane and backdrop | render.stage.ground | engine-only |
| Render backends | render.backends | engine-only |
| Turntable or motion animation export | motion.export_animation | engine-only |

**Top gaps**

- The Render menu starts a render, frames and exports it, and shows progress or cancels. Render settings, materials, lights, ground, backends and animation remain engine-only.
- No render output is measured or compared in this matrix.
- Rendering quality and speed are not measured here.

### Scripting/Plugins

| Rhino-style feature | Arclet/engine command id | Tier |
|---|---|---|
| Command bar (aliases and command names) | — (aliases in apps/arclet/src/commands.rs) | main-works |
| Undo | edit.undo | main-works |
| Redo | edit.redo | main-works |
| Repeat last command | edit.repeat | main-works |
| Checkpoint and restore | edit.checkpoint, edit.restore_checkpoint | main-works |
| Compute all | timeline.compute_all | main-works |
| List and describe commands | engine.commands, engine.describe | engine-only |
| Feature timeline (rollback, edit, suppress) | timeline.rollback, timeline.edit, timeline.suppress | engine-only |
| Expressions and parameters | expr.evaluate, parameters.change | engine-only |
| Export command script | timeline.export_script | engine-only |
| Node graph workspace (own JSON format) | — (workspace switch arclet.nodes in app.rs) | partial |
| Python or RhinoScript | — | missing |
| RhinoCommon, .rhp and .gha plugins | — | missing |
| Grasshopper files (GH, GHX) | — | missing |
| Macro recording | — | missing |

**Top gaps**

- Python or RhinoScript, plugins, and Grasshopper files: no engine commands. The matrix states these are unsupported.
- Macro recording: no engine command.
- Node graph uses its own JSON format, not Grasshopper. It is partial.
- Repeat, checkpoints and Compute all are in the Edit menu. Timeline editing and parameters exist only in the engine.
