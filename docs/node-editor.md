# Node editor

The reusable Rust editor is `solidcraft_ui_egui::node_editor::NodeEditor`. A host draws it inside
its own workspace and handles the returned `NodeAction`s. It has no dependency on the Melt3D
application; the same widget can be used by a standalone CAD application and by a browser host.

## Controls

- **Add** opens the component groups. Select a component and left-click the canvas to place it.
- Left-click a card header to select it; drag the header to move it. One drag is one undo step.
- Click an output socket, then an input socket to connect them. A wire that would make a cycle
  is rejected and the previous connection is retained. **Disconnect** is in the input inspector.
- The inspector contains editable numeric sliders and each input's Keep tree / Flatten / Graft
  control. Connected inputs show their source instead of editing an unused constant.
- Right-click ends placement or connection; **Undo**, **Redo**, **Run**, **Stop**, **Open** and
  **Save** have visible labels. **Focus** scrolls to the selected card.
- **Add to model** passes generated triangles to the host, which decides how to insert them.
  Evaluation alone never replaces a model. G-code output has a separate export action.

The starter graph connects a Circle and a numeric height to Extrude. This provides a working
example immediately without printing, importing a file or downloading assets.

## Responsiveness and limits

Desktop evaluation runs on one worker. Edits cancel its result and queue one replacement run;
slider drags cannot create a queue of worker threads. The revision check discards stale results.
Cancellation is checked between list items and inside the slicer's layer and path work.

Browser evaluation processes at most one bounded node per frame. Browser Print nodes return an
explicit node error directing the user to the desktop application; browser background printing
is not implemented. This limitation is deliberate: running a complete slicer on the browser's
UI thread would make Stop unusable. A real Web Worker is the future extension point.

The document limit is 2,000 nodes and 4 MiB of JSON; the browser preview limit is 256 nodes.
Coordinates are finite, nonnegative and at most 1,000,000 canvas units. Input constants have
per-value and aggregate size limits. Evaluation has both per-node and global retained-output
budgets, including vertex counts rather than just the number of meshes. The default desktop
budget is 2,000,000 scalar/geometry units; browser results retain at most 131,072 units. A node
which exceeds a budget reports an error; unrelated nodes can still finish.

## Saved format

The versioned JSON document has `format: "melt3d.node-graph"`, `version: 1`, `next_id` and `nodes`.
The historical format identifier is retained for stable interchange; it does not couple the
editor to an application executable. Nodes store component name, stable numeric ID, position,
input constants, wires and input modifiers. Generated meshes, slices and evaluation errors are
not automatically included in the saved document. Explicit mesh constants are supported within
the input limits; cached sliced-layer constants are rejected with a clear error.

Loading validates the complete replacement before modifying the open graph. Unknown versions,
unknown components, duplicate IDs, missing wire endpoints, invalid output indices, cycles,
non-finite numbers and oversized input data are errors. `mark_saved_json` marks the exact saved
snapshot so an edit made during an asynchronous write remains dirty.

## Decisions and verification boundary

The editor uses the existing egui dependency, original SVG icons and theme tokens. No node UI
dependency, Python runtime, copied CAD interface text or copyleft geometry implementation was
introduced. Graph layout is an ordinary scrollable document, independent of the model camera.

Automated tests cover JSON round-trips and hostile files, connection rollback, propagation of
node errors, cancellation within list processing, bounded browser work, shared retained-output
limits, stale results, dirty tracking, pointer-driven wiring, numeric slider edits, visible node
errors and one-step drag undo. Headless rendering checks the light and dark themes with the
real SVG icon loader; these checks do not measure a physical GPU.

**Bambu A1 / A1 mini: 未確認.** Neither the editor nor its generic G-code export establishes
printer compatibility. Machine profiles, AMS packaging, hardware motion and actual prints must
be verified separately. Browser printing, real GPU performance and real printer output are not
claimed by the graph tests.

The existing Extrude component is a triangle prism with fan caps, suitable for convex or
star-shaped closed profiles. Merge Solids combines triangle lists; it is not a Boolean union.
The node editor does not turn these existing mesh operations into a NURBS surface kernel.
