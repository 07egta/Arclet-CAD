# Rhino-class coverage research

Research date: 2026-11-02. Scope: public documentation only, plus a read-only pass over this
repository. This document is internal research. It is not a compatibility statement, not a
roadmap commitment, and not a licence grant.

## 1. Scope and method

**What this is.** A sourced inventory of what a general-purpose NURBS surface and solid modeller
is expected to do, followed by an honest mapping onto Arclet's own command registry. The point is
to ground an "almost like Rhino" roadmap in evidence instead of in feature-name similarity.

**What this is not.** No compatibility claim is made here, for file formats, APIs, plugins or
commands. Nothing in the `COMPATIBILITY_MATRIX.md` "not claimed" rows is softened by this
document. Nothing here was produced by driving, scripting, screenshotting or extracting data from
a commercial CAD application.

**Clean-room statement.** No Rhino code, help text, icons, templates, sample files or other assets
were copied into this repository or into this document. Every sentence below is written in our own
words; where a public page is quoted at all, it is a few words only. All command ids named in the
tables are our own engine ids, taken from `crates/engine/src/cmd/mod.rs` (`command_specs`) and the
`COMMANDS` tables in `crates/engine/src/cmd/*.rs`. No Rhino command name was adopted as an id.

**Comparison object.** "Rhino" appears below only as the name of the product being compared, which
is what the AGENTS.md ground rules allow for internal research. The name must not enter UI strings,
locale catalogs, product naming, package names or marketing copy; see section 6.

**Status vocabulary.** Verified by reading code, never by running a modelling workflow:

| Status | Meaning |
| --- | --- |
| works | Registered, and reachable in the desktop app today through a menu/toolbar entry (`apps/arclet/src/app.rs`) or an explicit alias (`apps/arclet/src/commands.rs`). |
| partial | Registered and reachable, but covers only part of what the compared capability does, or the menu exposes one variant of several. |
| engine-only | Registered in the engine, with no menu, toolbar or alias entry. Reachable only by typing the raw id in the command bar. |
| missing | No command with that capability in the registry. |

Tiering rule: a raw id typed into the command bar does not count as reachable, otherwise every
registered command would count and the tiers would carry no information. Aliases do count, because
they are explicit authored shortcuts.

**Paths.** `E/` = `crates/engine/src/cmd/`, `A/` = `apps/arclet/src/`, `U/` = `crates/ui-egui/src/`,
`D/` = `crates/doc/src/`.

**Caveat on the existing matrix.** `COMPATIBILITY_MATRIX.md` is now narrower than the code in two
places: it lists arcs, ellipses, polygons, surfaces ruled/untrim/extend and the render commands as
`engine-only`, while `A/app.rs` (lines 19-118) wires them into menus. This document follows
`app.rs`. The matrix's own statement that the menu "Join" button runs a continuity analysis is still
true and is repeated as a defect in section 5.

## 2. Taxonomy: what a user expects

Fourteen groups, in the order a modeller tends to meet them. Each group below opens with the
expectation a user brings, then the capability rows.

### 2.1 Curve creation and editing

A freeform modeller is judged first by its curves: analytic primitives with every construction
method, free-form curves with both interpolated and control-point flavours, and a full editing kit
(offset, trim, extend, join, split, blend, fillet, rebuild) that keeps the curve a curve rather than
converting it to segments. Rhino documents lines, polylines, arcs, circles, ellipses, rectangles,
polygons, helices, spirals, conics, text and both interpolated and control-point curves [S1][S3];
curve objects are degree/knot/control-point constructs, and Rebuild, Fair, ChangeDegree and Smooth
are the documented ways to change that structure [S3].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Line, single and chained | sketch.line | works | A/app.rs:20 |
| Polyline as one connected object | sketch.chain | works (alias `pl`, `polyline`) | A/commands.rs:120,123 |
| Arc: 3-point, centre, tangent | sketch.arc.three_point / .center_point / .tangent | works | A/app.rs:77-79 |
| Circle from centre and radius/diameter | sketch.circle.center | works | A/app.rs:21 |
| Circle 2-point, 3-point, 2-tangent, 3-tangent | sketch.circle.two_point / .three_point / .two_tangent / .three_tangent | engine-only | E/sketch.rs |
| Rectangle by corners | sketch.rectangle.two_point | works | A/app.rs:22 |
| Rectangle centre, 3-point, rounded | sketch.rectangle.center / .three_point / .rounded | engine-only | E/sketch.rs |
| Ellipse | sketch.ellipse | works | A/app.rs:81 |
| Polygon inscribed | sketch.polygon.inscribed | works | A/app.rs:80 |
| Polygon circumscribed, by edge | sketch.polygon.circumscribed / .edge | engine-only | E/sketch.rs |
| Conic with rho | sketch.conic | engine-only | E/sketch.rs |
| Interpolated curve through points | sketch.spline.fit_point | works | A/app.rs:26 |
| Control-point curve, degree 3 and 5 | sketch.spline.control_point / _5 | works | A/app.rs:23,27 |
| Insert a point into a curve | sketch.spline.insert_point | works | A/app.rs:430 |
| Blend curve between two curves | sketch.blend_curve | works | A/app.rs:28 |
| Offset curve | sketch.offset | works (alias `o`, `offset`) | A/commands.rs:112-114 |
| Trim curve | sketch.trim | works (alias `tr`) | A/commands.rs:162 |
| Extend curve | sketch.extend | works (alias `extend`) | A/commands.rs:74 |
| Split/break curve | sketch.break | engine-only | E/sketch_modify.rs |
| Join curves into one polycurve | none | missing | E/mod.rs |
| Fillet between curves | sketch.fillet | works | A/app.rs:85 |
| Chamfer between curves | sketch.chamfer.equal_distance | works (one of three methods) | A/app.rs:86 |
| Reverse curve direction | sketch.spline.reverse | engine-only | E/sketch_create.rs |
| Rebuild curve with a point count or tolerance | sketch.spline.rebuild | works | A/app.rs:29 |
| Change curve degree, simplify, convert to arcs/lines | none | missing | E/mod.rs |
| Explode a polycurve into its segments | none | missing | E/mod.rs |
| Helix, spiral | none (`solid.coil` makes a helical solid) | missing | E/mod.rs |
| Curve on a surface: project, pull, isocurve, intersection | sketch.project / sketch.project_to_surface / sketch.isoparametric_curve / sketch.intersection_curve | engine-only | E/sketch_project.rs |
| Text as curves | sketch.text | works (alias `text`) | A/commands.rs:157 |
| Point object, 2D and 3D | sketch.point / sketch.point3d | works / engine-only | A/commands.rs:121, E/sketch_3d.rs |

### 2.2 Surface creation, editing and analysis

Every surface command should produce the same kind of object (a NURBS surface, trimmed or
untrimmed), and the user expects to be able to build a surface from points, from 2-4 curves, from a
crossing curve network, from profiles (loft, sweep, revolve), by offsetting or thickening, and then
to trim, untrim, extend, match and fair it. Rhino's documented surface creation set includes SrfPt,
EdgeSrf, NetworkSrf, Loft, Sweep1, Sweep2, Revolve, RailRevolve, Patch, PlanarSrf, OffsetSrf and
Drape [S5][S6]; trimmed surfaces keep their underlying surface and can be untrimmed [S3]; Loft
offers loose/normal/tight/uniform styles and end-tangent matching [S9].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Loft through ordered profiles | surface.loft | works (end continuity g0-g3 as a parameter) | E/surface.rs:51 |
| Sweep one profile along a path | surface.sweep | works | A/app.rs:34 |
| Sweep with two rails | surface.sweep (rails as a parameter) | partial, unverified | E/surface.rs:50 |
| Revolve, rail revolve | surface.revolve | works; rail variant unverified | A/app.rs:36 |
| Surface from closed planar curves (planar surface) | surface.boundary | partial | E/surface_boundary.rs:29 |
| Surface from 2-4 curves (edge surface) | none | missing | E/mod.rs |
| Surface from a network of curves | none | missing; alias `networksrf` points at Patch | A/commands.rs:109 |
| Patch through curves and points | surface.patch | works | A/app.rs:35 |
| Ruled surface between two curves | surface.ruled | works | A/app.rs:92 |
| Extrude curve to surface | surface.extrude | works (alias `extrudesrf`) | A/commands.rs:78 |
| Offset surface | surface.offset | works | A/app.rs:41 |
| Trim and untrim surface | surface.trim / surface.untrim | works | A/app.rs:42,94 |
| Extend surface | surface.extend | works | A/app.rs:95 |
| Match surface continuity | surface.match | works | A/app.rs:39 |
| Fair (smooth) surface | surface.fair | works | A/app.rs:40 |
| Join surfaces into a polysurface, unjoin | surface.stitch / surface.unstitch | works | A/app.rs:43,93 |
| Thicken surface to a solid | surface.thicken | works | A/app.rs:44 |
| Flip surface normal | surface.reverse_normal | works (alias `flip`) | A/commands.rs:84 |
| Fillet surface between two surfaces, variable radius | none | missing | E/mod.rs |
| Rebuild surface to a new structure, convert to single span | none | missing | E/mod.rs |
| Split surface by curve or isocurve | none (`solid.split_face` is solid-only) | missing | E/mod.rs |
| Move a surface seam, adjust closed-curve seam | none | missing | E/mod.rs |
| Unroll/develop a developable surface | none | missing | E/mod.rs |
| Surface from a mesh | none | missing | E/mod.rs |
| Surface seam/normal display and direction tools | surface.reverse_normal only | partial | E/surface_analysis.rs |

### 2.3 Solids

A solid here is a closed surface or polysurface, so the expectation is really about closing and
combining: primitives, extrude/revolve/loft/sweep as solids, cap planar holes, join surfaces into a
solid, booleans with tools kept or discarded, shell, fillet/chamfer edges, thin-wall and
draft-sensitive operations. Rhino describes a solid as a surface or polysurface that encloses a
volume and documents Boss, ExtrudeCrv/Srf, Pipe, Rib and Slab as solid creators [S7]; CreateSolid
turns intersecting surfaces into closed polysurfaces [S8].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Box, sphere, cylinder | solid.box / .sphere / .cylinder | works | A/app.rs:63-65 |
| Cone, torus | solid.cone / solid.torus | works | A/app.rs:98-99 |
| Extrude, revolve, loft, sweep as solids | solid.extrude / .revolve / .loft / .sweep | works | A/app.rs:66-69 |
| Pipe along a curve | solid.pipe | works | A/app.rs:100 |
| Fillet and chamfer edges | solid.fillet / solid.chamfer | works | A/app.rs:70-71 |
| Variable-radius fillet | none | missing | E/mod.rs |
| Boolean union, difference, intersection | solid.combine (`operation: join/cut/intersect`) | partial: one menu entry for three operations | E/features.rs:83 |
| Boolean split, keep-tools control | solid.combine (`keep_tools`) | partial | E/features.rs:83 |
| Shell / hollow a solid | solid.shell | works | A/app.rs:74 |
| Cap planar holes, create a solid from surfaces | none | missing | E/mod.rs |
| Explode a polysurface into faces | surface.unstitch | partial (surface path only) | A/app.rs:93 |
| Draft angle, hole, thread, rib, web, emboss, coil | solid.draft / .hole / .thread / .rib / .web / .emboss / .coil | engine-only | E/features_more.rs |
| Split body, split face, offset face, replace face, delete face | solid.split_body / .split_face / .offset_face / .replace_face, face.delete | engine-only | E/features_more.rs, E/face.rs |
| Bounding solid, boundary fill | solid.bounding_solid / solid.boundary_fill | engine-only | E/features_more.rs:18 |
| Block definition and instance, nesting | none (components are the nearest model) | missing | E/mod.rs |
| Edge/face inspection of a body | model.edges / model.faces | engine-only | E/inspect.rs:35-36 |

### 2.4 Mesh

Meshes are second-class in a NURBS modeller but indispensable for export, 3D printing and repair.
The expectation is: turn NURBS into a mesh at a chosen density, import mesh files, check and repair
(naked edges, non-manifold, weld, normals), reduce/smooth, boolean meshes, and occasionally rebuild
a surface from a scanned mesh. Rhino documents Mesh as producing editable triangle/quad meshes from
surfaces and polysurfaces, watertight when the input is a solid, with density and tolerance controls
[S18].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Tessellate a body to a mesh file (STL/3MF), with quality | file.save_mesh | works | A/app.rs:110, E/file.rs:60 |
| Create an editable mesh object in the document | none (only export-time tessellation) | missing | E/mesh.rs |
| Insert a mesh file | file.insert_mesh | works | A/app.rs:107 |
| Mesh -> editable planar/cylindrical faces | mesh.convert | engine-only | E/mesh.rs:17 |
| Mesh repair: weld, fill holes, unify normals | none | missing | E/mod.rs |
| Mesh check: naked and non-manifold edges | none | missing | E/mod.rs |
| Mesh reduce/decimate, smooth, subdivide | none | missing | E/mod.rs |
| Mesh boolean | none | missing | E/mod.rs |
| Mesh to NURBS | none | missing | E/mod.rs |

### 2.5 Transforms and deformation

Move, copy, rotate, scale, mirror, array and align are the backbone; then orientation tools (orient
on surface, flow along a curve or surface) and free-form deformer cages. Rhino's gumball is the
documented one-stop transform widget: arrows, plane handles, rotation arcs, scale handles, an
extrude handle, a cut handle, Alt for copy, Shift/Ctrl modifiers, numeric entry and relocatable
alignment [S10].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Move and copy | solid.move | works | A/app.rs:46 |
| Rotate | solid.move (move/rotate tool) | works | U/gizmo.rs:1 |
| Scale | solid.scale | works | A/app.rs:46 |
| Mirror | solid.mirror | works | A/app.rs:46 |
| Rectangular and polar array | solid.pattern.rectangular / .circular, sketch.pattern.rectangular / .circular | works (menus) | A/app.rs:88-89,104 |
| Array along a path | solid.pattern.path | engine-only | E/features_more.rs |
| Align / orient objects | solid.align | engine-only | E/features_more.rs |
| Drag a feature by handles with numeric entry | per-feature canvas manipulators | partial | U/canvas.rs:291 |
| Orient on a surface, flow along a surface | none | missing | E/mod.rs |
| Bend, twist, taper, shear | none | missing | E/mod.rs |
| Cage deform / lattice | none | missing | E/mod.rs |

### 2.6 Analysis and measurement

The expectation is both numeric (distance, angle, length, area, volume, centroid, minimum radius)
and visual (zebra, curvature false colour, curvature graph, environment map, draft analysis), plus
model-health checks (naked edges, self-intersection, bad objects). Rhino documents zebra as a
stripe-map test that separates G0, G1 and G2 continuity by eye [S17].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Measure distance, length, angle | inspect.measure | works | A/app.rs:53 |
| Area, volume, mass, centroid | component.properties / inspect.center_of_mass | engine-only | E/component_properties.rs, E/inspect.rs |
| Zebra stripes | inspect.zebra | works | A/app.rs:48 |
| Curvature false-colour analysis | inspect.curvature_map | works | A/app.rs:50 |
| Curvature graph / comb | inspect.curvature_comb | works | A/app.rs:51 |
| Environment map reflection | inspect.environment_map | works | A/app.rs:49 |
| Surface continuity G0-G2 between surfaces | inspect.continuity | works | A/app.rs:52 |
| Curve end continuity G0-G3 | inspect.curve_continuity | works (alias `gcon`) | A/commands.rs:86 |
| Draft-angle analysis | inspect.draft | works (alias) | A/commands.rs:66 |
| Minimum radius of curvature | inspect.minimum_radius | engine-only | E/surface_analysis.rs |
| Section / contour curves | inspect.section | works (alias `section`, `contour`) | A/commands.rs:55,143 |
| Isocurve extraction | sketch.isoparametric_curve | engine-only | E/sketch_project.rs |
| Interference, clearance, clash | inspect.interference / inspect.clearance | engine-only | E/interference.rs, E/clearance.rs |
| Deviation: curve-to-curve, surface-to-mesh, point-to-object | none (`inspect.compare` compares bodies) | missing | E/mod.rs |
| Object check: naked edges, self-intersection, bad geometry | none | missing | E/mod.rs |
| Parameter sweep of a design | inspect.parameter_sweep | engine-only | E/parameter_sweep.rs |
| Accessibility / pull-direction analysis | inspect.accessibility | engine-only | E/inspect.rs |

### 2.7 Layers, objects and organisation

Layers are the primary organisational unit: named, coloured, lockable, on/off, hierarchical,
per-layout and per-detail, with the current layer driving new geometry, plus layer states for
snapshots. Rhino's layer panel documents name, current, on/off, lock, colour, material, linetype,
print colour, print width, sublayers, layer filters and a layer state manager [S12].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Named layers with colour, on/off, lock, current | none | missing | E/mod.rs |
| Sublayers / layer hierarchy | browser.group gives folders, not layers | partial | E/browser.rs:25 |
| Move/copy objects to a layer | none | missing | E/mod.rs |
| Layer states (save/restore a layer setup) | none | missing | E/mod.rs |
| Groups of objects | browser.group / browser.ungroup | engine-only | E/browser.rs:25-28 |
| Rename body/component | body.rename / component.rename | engine-only | E/features.rs, E/component.rs |
| Object user text / attributes | document.metadata.set, component.metadata | engine-only | E/metadata.rs |
| Hide/show objects | browser.visibility | partial (menu exposes Show All only) | A/app.rs:383 |
| Isolate selection | ui.isolate (UI item, not a registry id) | works | A/app.rs:376 |
| Named selections | none | missing | E/mod.rs |
| Select by filter, same appearance, clear | select.filter / select.same_look / select.clear | works | A/app.rs:403-406 |
| Delete and clipboard | selection.delete, clipboard commands | works / engine-only | A/app.rs:109 |

### 2.8 Display and named views

Standard views and fit, multiple viewports, saved cameras with thumbnails, display modes,
grid and snap spacing, clipping planes and per-object display overrides. Rhino's NamedView panel
saves, restores, renames, duplicates and animates cameras, and can import them from a 3DM file [S13].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Fit, isometric, top, front, right | view.fit / .isometric / .top / .front / .right | works | A/app.rs:55-61 |
| Back, bottom, left | view.back / .bottom / .left | works (aliases) | A/commands.rs:35,42,96 |
| Save, apply, list, rename, delete named views | view.save / .apply / .list / .rename / .delete | engine-only | E/browser.rs:35-44 |
| Named-view panel with thumbnails, animation | none | missing | E/mod.rs |
| Split/4-view model viewports | none found | missing | E/mod.rs |
| Perspective / parallel toggle | view.perspective | works (alias) | A/commands.rs:118 |
| Shaded / wireframe / rendered style switch | view.visual_style | engine-only | E/view_navigation.rs |
| Grid display and snap spacing | view.grid | engine-only | E/view_navigation.rs |
| Clipping plane | none | missing | E/mod.rs |
| Per-object display mode, ghosted | appearance.assign only | partial | E/inspect.rs:47 |
| Ambient occlusion, ground shadow | view.ambient_occlusion / view.ground_shadow | engine-only | E/view_navigation.rs |
| Workspace/screen layout | view.layout | works | E/view_navigation.rs:75 |

### 2.9 Annotation, drafting and layouts

The drafting side is a different product surface: model-space dimensions that reference geometry,
leaders and callouts, text and dots, hatch and gradient hatch, linetypes and print widths, then
paper space with layouts, detail views at a scale, title blocks and print/PDF. Rhino's Layout
command documents a paper-space viewport holding detail views of the model, plus annotation, title
blocks and printing [S14].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Sketch dimensions (linear, angular, radius/diameter) | sketch.dimension, sketch.toggle_diameter | works | A/app.rs:84 |
| Driven vs driving dimension | sketch.toggle_driven | engine-only | E/sketch_dimensioned.rs |
| Move dimension text | sketch.dimension_text | engine-only | E/sketch_dimensioned.rs |
| Sketch constraints (coincident, tangent, parallel, ...) | sketch.constraint.* | engine-only | E/sketch_constraints.rs |
| Model-space dimensions outside a sketch | none | missing | E/mod.rs |
| Leader / callout | none | missing | E/mod.rs |
| Text, dot, annotation styles | sketch.text only (in sketch) | partial | E/sketch.rs |
| Hatch and gradient hatch | none | missing | E/mod.rs |
| Linetypes, print colour, print width | none | missing | E/mod.rs |
| Layout / paper space with title block | none (`sheet.*` is sheet metal) | missing | E/mod.rs |
| Detail views at a scale | none | missing | E/mod.rs |
| Print or PDF/vector output of a drawing | none | missing | E/mod.rs |
| Export the current view as DXF | file.export_view_dxf | engine-only | E/view_dxf.rs:22 |
| Section / clipping drawings | none | missing | E/mod.rs |

### 2.10 File formats and interop

Import and export of the exchange formats a studio actually receives: 3DM, STEP, IGES, DXF/DWG,
OBJ, glTF/GLB, PLY, STL, 3MF, PDF, plus point clouds. Rhino's published list covers well over
thirty formats in both directions, including 3DM, STEP, IGES, DXF/DWG, OBJ, glTF/GLB, PLY, PDF,
STL and 3MF [S20].

The rows below were checked against the format table (`I/formats.rs:179-330`), not against the
menu, and each direction is stated separately: a format this app reads but cannot write is not
interoperable in both directions, and a format it writes but cannot read is the same problem
mirrored.

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Native design file open/save | doc.open / file.save / file.save_as | works | A/app.rs:337-340 |
| STEP import and export | doc.open / file.insert_step, file.export (`step`) | import and export | I/formats.rs:188-195, E/file.rs:647 |
| IGES import and export | doc.open, file.export (`igs`/`iges`) | import and export | I/formats.rs:196-203, I/lib.rs:369 |
| STL import and export (binary and ASCII) | doc.open, file.export, file.save_mesh | import and export | I/formats.rs:220-227, I/lib.rs:365-366 |
| 3MF import and export, plus 3MF with face tags | doc.open, file.export (`3mf`, `3mf-tagged`) | import and export | I/formats.rs:228-243 |
| OBJ import and export (with `.mtl` library) | doc.open, file.export (`obj`) | import and export; the export writes a material library beside the mesh | I/formats.rs:244-251, E/file.rs:621-678 |
| Rhino 3DM import | doc.open (`3dm`) | import only, desktop builds; decoded through the `cadmpeg` crate and restated as STEP | I/formats.rs:212-219, I/native_cad.rs:1-46 |
| Rhino 3DM export | none | missing | I/formats.rs:218 (`export: None`) |
| FreeCAD `.FCStd` import | doc.open (`fcstd`) | import only, desktop builds | I/formats.rs:204-211 |
| glTF / GLB import | doc.open (`gltf`, `glb`) | import only | I/formats.rs:252-259 |
| glTF / GLB export | none | missing | I/formats.rs:258 (`export: None`) |
| PLY, XYZ and PTS point-cloud import | doc.open, pointcloud.insert | import only | I/formats.rs:260-283 |
| DXF sketch import | sketch.insert_dxf | import only | I/formats.rs:300-307 |
| DXF of the current view | file.export_view_dxf | export only | I/formats.rs:308-321, E/view_dxf.rs |
| SVG sketch import | sketch.insert_svg | import only | I/formats.rs:322-329 |
| Sheet-metal DXF export | sheet.export_dxf | engine-only | E/sheet.rs |
| PDF import or export | none | missing | I/formats.rs |
| USD and FBX | none | missing | I/formats.rs |
| Document units | document.units | engine-only | E/documents.rs |
| URDF / JSON export of an assembly | file.export (`urdf`, `json`) | export only | I/formats.rs:284-299 |

### 2.11 Scripting and automation

The expectation is a command line that accepts names, aliases and abbreviations, repetition of the
last command, macro recording, a scripting language with access to the modelling API, and a
parametric/node workspace. Rhino documents aliases as user-defined abbreviations that can run a
command or a command script [S15]; Rhino.Python runs CPython/IronPython scripts with access to
RhinoCommon and rhinoscriptsyntax [S22]; Grasshopper is a graphical algorithm editor integrated
with the modelling tools [S23][S24].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Command bar with ids, labels, prefix suggestions | registry + command bar | works | A/app.rs:486, A/commands.rs:222 |
| User-editable aliases | fixed ALIASES table | partial | A/commands.rs:29 |
| Repeat the last command | edit.repeat, empty command line | works | A/app.rs:469, A/commands.rs:132 |
| Undo and redo | edit.undo / edit.redo | works | A/app.rs:362 |
| Checkpoint / restore a design state | edit.checkpoint / edit.restore_checkpoint | engine-only | E/edit.rs |
| Command list and per-command description | engine.commands / engine.describe | engine-only | E/inspect.rs:68-69 |
| Feature timeline: rollback, edit, suppress, reorder | timeline.* | engine-only | E/features.rs |
| Command script export (JSON recipe) | timeline.export_script | engine-only | E/script.rs |
| Batch execution from an external client | control protocol / MCP | works (not a UI feature) | U/agent_chat/api_client.rs |
| Node/parametric workspace, own JSON format | workspace switch in app.rs | partial | A/app.rs:301 |
| Macro recording | none | missing | E/mod.rs |
| Python, VBScript, .NET plugins, GH/GHX | none, and deliberately unsupported | missing | COMPATIBILITY_MATRIX.md:17-18 |

### 2.12 Rendering

Expected: a built-in renderer with PBR materials, an environment and studio lights, a ground plane,
quality/denoise settings, background rendering and image export, plus turntable animation. This
group is a product in its own right in Rhino, with a built-in engine and many third-party engines
[S24].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Start, cancel, poll and export a render | render.start / .cancel / .status / .export | works (menu) | A/app.rs:111-117 |
| Fit the render camera | render.camera.fit | works | A/app.rs:113 |
| Render settings read/write | render.settings.get / .set / .apply | engine-only | E/render.rs |
| PBR material create/assign/edit | material.create / .assign / .edit | engine-only | E/materials.rs |
| Environment, studio light presets | render.library.environment.apply / render.library.studio.apply | engine-only | E/render.rs |
| Ground plane / backdrop | render.stage.ground / .backdrop | engine-only | E/render.rs |
| Quality presets, output size, preflight | render.quality.prepare / render.output.size / render.preflight | engine-only | E/render.rs |
| Turntable or animation export | motion.export_animation | engine-only | E/motion_animation.rs |
| Render quality and speed claims | none | not measured | COMPATIBILITY_MATRIX.md:347 |

### 2.13 Usability: command line, selection, snapping, gumball, properties

This is where "feels like a modeller" lives, and where command counts mislead most. Users expect:
type-ahead command entry and aliases; Enter to repeat; unlimited labelled undo; click, window and
sub-object selection with a filter; object snaps with one-shot overrides and a disable toggle [S11];
a transform widget with numeric entry [S10]; a construction plane per viewport that can be set from
geometry [S19]; and a properties panel for the selected object [S16].

| Capability | Nearest Arclet id | Status | Evidence |
| --- | --- | --- | --- |
| Type-ahead command bar, ambiguity reported not guessed | commands::resolve | works | A/commands.rs:222 |
| Aliases (authored, not user-editable) | ALIASES | partial | A/commands.rs:29 |
| Repeat on empty Enter | last_command | works | A/app.rs:470 |
| Undo/redo with checkpoint granularity | edit.undo / edit.redo / edit.checkpoint | partial | E/edit.rs |
| Undo history panel with named steps | none | missing | E/mod.rs |
| Selection filter by object type | select.filter | works | A/app.rs:403 |
| Select by same appearance, clear selection | select.same_look / select.clear | works | A/app.rs:405 |
| Window/crossing box selection | not found | missing | E/mod.rs |
| Sub-object selection (face, edge, vertex) | partial via select.filter | partial | A/app.rs:403 |
| Object snaps (end, mid, centre, intersection, perp, near, quadrant, knot, vertex) | none (`sketch.snap` is constraint inference) | missing | E/mod.rs, E/sketch_constraints.rs:62 |
| One-shot snap override, disable toggle, snap radius | none | missing | E/mod.rs |
| Ortho, polar tracking, grid snap, distance/angle constraint while drawing | sketch inference and dimensions only | partial | E/sketch_constraints.rs |
| Transform widget: move/rotate triad with numeric box | U/gizmo.rs | partial (no scale/extrude/copy handles) | U/gizmo.rs:1-5 |
| Relocatable widget, align to object/world/view | none | missing | E/mod.rs |
| Construction plane per viewport, auto-align to a face | construct.plane.*, sketch-on-face | partial (per sketch, not per view) | E/construct.rs |
| Named construction planes | none | missing | E/mod.rs |
| Object properties panel (name, layer, colour, material, mass) | component.properties (engine) | missing as a panel | E/component_properties.rs |
| Layers panel | none | missing | E/mod.rs |
| Keyboard shortcuts for common tools | CommandSpec::key | works (some) | E/sketch.rs:51,61 |

## 3. How far Arclet is, in one paragraph

Coverage is lopsided rather than uniformly behind. Curve and surface *creation* is strong: almost
every create/edit verb a freeform modeller needs exists and most are wired into the Curves and
Surfaces menus (loft with end continuity, patch, blend, match, fair, offset, trim, stitch, thicken).
What is missing is concentrated in four places: (a) organisation and drafting, where there are no
layers, no paper space, no model-space annotation, no hatch and no print/PDF; (b) precision input,
where there are no object snaps and the transform widget has no scale handles or relocatable
alignment; (c) interop, where reading is much stronger than writing: 3DM, IGES, STEP, OBJ, glTF and
point clouds can be read, but 3DM, glTF and point clouds cannot be written and there is no PDF;
(d) kernel-hard surface topologies, with no surface-to-surface fillet, no network surface, no
surface split and no unroll. A large middle band of commands is registered but unreachable from the
desktop UI, which is the cheapest source of user-visible improvement available.

## 4. Prioritised gaps

Ordered by user value per unit of effort, not by absolute importance. Class: **R** = plausibly
implementable in pure Rust, clean-room; **S** = needs a public file-format specification (and a
licence review) before implementation; **X** = should stay unimplemented.

| # | Gap (what a user cannot do today) | Where it would touch | Risk | Class |
| --- | --- | --- | --- | --- |
| 1 | No layers: cannot group, colour, lock or hide a set of objects; nothing to set "current" for new geometry | D/ (design format), E/browser.rs or new E/layer.rs, U/browser.rs, A/app.rs | Touches the persisted design format (one file in D/format/pending/), and every visibility/selection path. Highest reach of any item. | R |
| 2 | No object snaps: cannot place a point on an endpoint, midpoint, centre or intersection while a command is running | new U/snap.rs, U/navigation.rs, kernel closest-point/intersection queries | Depends on kernel queries that may not exist yet; snapping must not break the single camera path. | R |
| 3 | The transform widget only moves and rotates: no scale handles, no copy modifier, no extrude/cut handle, no relocation | U/gizmo.rs, U/canvas.rs | Self-contained but touchy; modifier keys collide with existing navigation. | R |
| 4 | "Join" in the Curves menu does not join: it runs a continuity analysis, and there is no curve join at all | A/app.rs:30, E/sketch_modify.rs | Low effort, high trust value: a wrong label is a compatibility claim by implication. | R |
| 5 | Cannot explode a polycurve into segments, split a curve, or change a curve's degree | E/sketch_modify.rs, E/sketch_create.rs, sketch curve model | Degree change can silently alter shape; needs a tolerance story and tests. | R |
| 6 | No network surface and no 2-4 curve edge surface; the `networksrf` alias was removed because `surface.patch` is not a network surface | E/surface.rs, kernel surface fitting | Kernel work. A misleading alias is worse than a missing one: it promises a surface the kernel does not build. | R |
| 7 | No surface-to-surface fillet, and no variable-radius edge fillet | kernel (surface/edge blending), E/surface.rs, E/features.rs | Hardest kernel item in this list; expect years of edge cases. Do not ship a partial that breaks on common joints. | R |
| 8 | Cannot split a surface, move a seam, or unroll a developable surface | kernel, E/surface.rs, E/sketch_modify.rs | Split needs robust curve-surface intersection; unroll needs developability detection. | R |
| 9 | No model-space annotation, hatch, leader, linetype, print width | D/ (new annotation entities), E/ new annotate.rs, U painting | New entity family plus a renderer for it; also a format change. | R |
| 10 | No paper space: no layouts, detail views, title blocks, print or PDF | D/, new drawing crate or E/sheet-like module, PDF writer in io | Large. A pure-Rust PDF writer is feasible; a WYSIWYG sheet editor is not a small feature. | R (PDF writer: R) |
| 11 | No mesh object in the document, and no mesh repair/check/reduce/boolean | E/mesh.rs, kernel mesh module | Mesh booleans and repair are a sub-project. Check (naked/non-manifold) is much cheaper and worth doing first. | R |
| 12 | Cannot write 3DM, so a model cannot round-trip back to the compared product; reading exists and is desktop-only | `crates/io` (writer), `crates/io/src/formats.rs` (turn `export` on) | Reading already goes through the `cadmpeg` crate and restates the file as STEP; writing is a separate, large job. Do not vendor the C++ openNURBS toolkit. | S |
| 13 | No DWG read or write, and DXF output is limited to the current view rather than a full drawing with layers and dimensions | `crates/io` (DWG), `crates/io/src/dxf_out.rs` (drawing export) | DXF writing is already here for sketches and views; DWG is a separately licensed specification and should not be attempted without a licence review. | S |
| 14 | glTF/GLB, PLY and USD: import exists for glTF/GLB and PLY, export does not; USD and FBX do not exist at all | `crates/io` (writers) | glTF is an open Khronos spec and its exporter is a moderate job; OBJ already imports and exports. | R (glTF export), S (USD) |
| 15 | No helix or spiral curve | E/sketch_create.rs | Trivial maths, needs only a curve type and UI parameters. | R |
| 16 | No section/clipping plane and no clipping drawings | U/viewport, E/inspect.rs | Clipping needs shader or geometry-level support; drawings on top are much harder. | R |
| 17 | Named views exist but are invisible: no panel, no thumbnails, no restore from the UI | U/browser.rs, A/app.rs | Cheap: the commands already exist (`view.save/apply/list`). | R |
| 18 | Display modes, grid and per-object overrides are engine-only | U/, A/app.rs | Cheap wiring for style/grid; per-object overrides need a render-state model. | R |
| 19 | No properties panel: name, layer, colour, material, mass properties are command-only | U/browser.rs, A/app.rs | Medium; becomes much easier once layers exist. | R |
| 20 | Undo is not user-legible: no history list, no labelled steps | E/edit.rs, U/ | Medium; the checkpoint mechanism exists but the UX does not. | R |
| 21 | Box/crossing selection and richer sub-object selection are absent | U/selection, E/select | Medium; interacts with snapping and the selection filter. | R |
| 22 | No user-editable aliases and no macro recording | A/commands.rs, A/app.rs, preferences | Cheap for aliases (needs a preferences store); macro recording needs a recorder over the command bus. | R |
| 23 | No constraint-free deformer cage: bend, twist, taper, shear, flow along surface | kernel, D/ feature kinds | High effort, high wow factor; deformation must survive the timeline. | R |
| 24 | No Python/VBScript/.NET plugin host and no GH/GHX support | none planned | Embedding a general interpreter plus a foreign API is a compatibility claim we cannot evidence. Keep unsupported. | X |

Items 4, 5, 15, 17, 18 and 22 are the cheapest visible wins; item 1 and 2 change how the product
feels more than any ten commands.

## 5. Legal and clean-room constraints

These are restated from `AGENTS.md` and `skills/solidcraft-ground-rules/` and are binding on any
work that cites this document.

- **Independence.** Never copy another product's code, icons, text or templates; never drive or take
  data from commercial CAD (`AGENTS.md`, hard rules, line 9). No commercial CAD application was
  launched, scripted, screenshotted or mined for this research.
- **Licences.** No GPL, LGPL or AGPL code, and no reading of those sources: OpenCASCADE, FreeCAD /
  planegcs, SolveSpace, CadQuery/OCP, LibreCAD, QCAD (`AGENTS.md` line 10, enforced by
  `cargo xtask licences --check`). None of those sources informed this document.
- **Assets.** Every asset is original or openly licensed with a row in `ATTRIBUTION.md`; read
  `assets/AGENTS.md` before icon work (`AGENTS.md` line 11). This document adds no assets and no
  images.
- **No compatibility claim without evidence.** The registry and menu wiring are evidence of what is
  wired, not of what works. Rows above are marked "works" on reachability, and nothing here
  overrides the "not claimed" rows in `COMPATIBILITY_MATRIX.md` (3DM, GH/GHX, RhinoCommon, Python,
  .rhp/.gha, Class-A, G3).
- **Trademark care.** "Rhino" is used here only as the name of the compared product, in an internal
  research document. It must not appear in UI strings, locale catalogs
  (`assets/locales/pending/*.json`), product or package names, installer metadata, screenshots used
  as evidence, or marketing copy. Alias names that happen to match a competitor's command names
  (`sweep2`, `networksrf`, `unjoin`) are functional short names, not branding; where they overstate
  what the command does, they should be corrected for accuracy, not for legal reasons.
- **File formats.** Reading a public specification (IGES 5.3, ISO 10303, the glTF specification,
  the openNURBS 3DM description) is acceptable; copying an implementation is not. A format reader
  must be written from the specification, and any third-party library needs a licence row and a
  review before it enters `Cargo.toml`.
- **Evidence discipline.** Behaviour claims require a test or a captured screenshot produced by us
  (`AGENTS.md` line 25). This document contains neither, and says so throughout.

## 6. Sources

All URLs opened 2026-11-02. Official vendor documentation is preferred; Wikipedia is used only for
history and format background.

| Id | Source | Used for |
| --- | --- | --- |
| S1 | https://www.rhino3d.com/en/features/ | Feature overview: curve, surface, solid and mesh tool families; development platform |
| S2 | https://www.rhino3d.com/en/features/nurbs/ | NURBS as the geometry model; why precision matters |
| S3 | https://docs.mcneel.com/rhino/8/usersguide/en-us/html/ch-02_rhinoobjects.htm | Object model: curves, surfaces, trimmed/untrimmed, degree, control points, polysurfaces, solids, light extrusions, meshes |
| S4 | https://docs.mcneel.com/rhino/8/help/en-us/commandlist/command_list.htm | Command reference: per-command one-line purpose and menu location |
| S5 | https://docs.mcneel.com/rhino/mac/help/en-us/seealso/sak_surface.htm | Surface creation set: EdgeSrf, NetworkSrf, Loft, Sweep1/2, Revolve, RailRevolve, Patch, PlanarSrf, OffsetSrf, Drape, UnrollSrf, MeshToNURB |
| S6 | https://docs.mcneel.com/rhino/8/help/en-us/toolbarmap/surface_creation_toolbar.htm | Surface creation toolbar contents and one-line descriptions |
| S7 | https://docs.mcneel.com/rhino/8/help/en-us/seealso/sak_solid.htm | Solid definition and solid creators (Boss, ExtrudeCrv, ExtrudeSrf, Pipe, Rib, Slab) |
| S8 | https://docs.mcneel.com/rhino/8/help/en-us/commands/createsolid.htm | CreateSolid: closed polysurface from intersecting surfaces |
| S9 | https://docs.mcneel.com/rhino/8/help/en-us/commands/loft.htm | Loft styles, seam handling, end-tangent matching |
| S10 | https://docs.mcneel.com/rhino/8/help/en-us/commands/gumball.htm | Gumball: move/rotate/scale/extrude/cut handles, modifiers, numeric entry, relocation and alignment |
| S11 | https://docs.mcneel.com/rhino/8/help/en-us/user_interface/object_snaps.htm | Object snap set, persistent and one-shot snaps, projection, suspend |
| S12 | https://docs.mcneel.com/rhino/8/help/en-us/commands/layer.htm | Layer properties, sublayers, layer states, change/copy to layer |
| S13 | https://docs.mcneel.com/rhino/8/help/en-us/commands/namedview.htm | Named views: save, restore, rename, thumbnails, animation, import |
| S14 | https://docs.mcneel.com/rhino/8/help/en-us/commands/layout.htm | Layout/paper space, detail viewports, annotation, printing |
| S15 | https://docs.mcneel.com/rhino/8/help/en-us/options/aliases.htm | Aliases run a command or a command macro; import/export of alias lists |
| S16 | https://docs.mcneel.com/rhino/8/help/en-us/commands/properties.htm | Properties panel contents by object type |
| S17 | https://docs.mcneel.com/rhino/8/help/en-us/commands/zebra.htm | Zebra analysis and G0/G1/G2 interpretation |
| S18 | https://docs.mcneel.com/rhino/8/help/en-us/commands/mesh.htm | Mesh from NURBS, watertight meshes, density and tolerance controls |
| S19 | https://docs.mcneel.com/rhino/8/help/en-us/commands/cplane.htm | Construction plane: per-viewport origin/axes, set from object/curve/surface, Auto CPlane |
| S20 | https://www.rhino3d.com/en/features/file-formats/ | Supported import/export formats and directions |
| S21 | https://developer.rhino3d.com/guides/opennurbs/what-is-opennurbs/ | 3DM/openNURBS: open toolkit, no copyleft, and an explicit list of what it does not do |
| S22 | https://developer.rhino3d.com/guides/rhinopython/what-is-rhinopython/ | Rhino.Python: interpreters, RhinoCommon and rhinoscriptsyntax |
| S23 | https://www.grasshopper3d.com/ | Grasshopper as a graphical algorithm editor integrated with the modeller |
| S24 | https://en.wikipedia.org/wiki/Rhinoceros_3D | Product overview, NURBS basis, format breadth, scripting, rendering engines |
| S25 | https://en.wikipedia.org/wiki/IGES | IGES as a vendor-neutral ASCII exchange format; 5.3 (1996) is the last published version; entity/record structure |

**Pages that could not be read.** `https://docs.mcneel.com/rhino/8/usersguide/en-us/html/ch-09_layers.htm`
returned HTTP 403, so the layer group relies on the Layer command page [S12] instead.
`https://www.rhino3d.com/en/features/grasshopper/` returned HTTP 404, so the Grasshopper claims rely
on [S23] and [S24]. No claim above depends on either unread page.

**Sources not consulted on purpose.** No competitor source code, no decompiled binaries, no
third-party reverse-engineering notes for proprietary formats, and no use of the restricted projects
listed in section 5.
