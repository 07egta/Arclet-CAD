# Mission 43 — Command workflow reachability and regression evidence

Status: implementation submitted for CI; no desktop execution or Rust test success is claimed.

## Existing behavior observed in source

- Registered command IDs and aliases are resolved by `apps/arclet/src/commands.rs`. The command bar previously passed only the full text as a command name, with no structured arguments. Short commands start UI tools; a plain `sketch.chain` is **read-only chain selection**, not a polyline creation operation. Current `polyline` alias points at `sketch.line`, while `pl` points at `sketch.chain`. PR #9 owns real `sketch.polyline` and join changes; this PR does not touch those engine commands.
- The shared egui tool/dialog subsystem already supports many picks, previews and numeric parameters. The new parser does not replace that path; **bare names continue starting the existing interactive tools**. This PR focuses on explicit numeric input that the command prompt previously couldn't execute.
- A subset of aliases are broad families rather than precise operations: `BooleanDifference` still opens `solid.combine`, `Join` points to `surface.stitch`, and `Hide/Show` points to the same visibility command. These are not newly claimed complete.
- The existing command-history ring and suggestion highlight operate on labels/IDs. Typed explicit arguments now bypass guesswork and are never interpreted as scripts.

## Implemented UI contract

After opening a sketch, type `Line 0,0 20mm,0` to invoke `sketch.line` with explicit coordinates. Or type `Line 0,0`, Enter, then `20mm,0`, Enter. The partial line is kept only in the command-bar memory and is not written to the CAD document until validation and execution finish.

- `Polyline 0,0 20,0 20,10`: begins a staged path, Enter on an empty prompt commits `sketch.line` with the point list. This is a chain of native sketch line segments, **not** one new polycurve entity; the engine's true sketch polyline remains PR #9 scope.
- `Circle 0,0 5mm` executes an exact radius. `Circle 0,0` begins a radius prompt.
- `Box 10mm 20mm 30mm` runs an existing box feature; partial dimensional input is accepted for sequential entry.
- `solid.box {"length":"20 mm","width":"10 mm","height":"5 mm"}` executes a typed JSON object through the existing engine command and its validation rules. The same approach supports other registered commands when full parameters are known.
- Supported literal length units: mm, cm, m, in, ft. Sketch point coordinates are **2D sketch-plane X,Y**. No command parser evaluates arbitrary script expressions or shells.
- The input guide reports the next field; Enter finishes a pending polyline, Esc cancels without editing the document. Clicking toolbar/menu commands discards the pending guided state.
- Explicit engine runs refuse to cross a currently active tool/dialog instead of silently applying data to the wrong command. Existing UI tools are not replaced or bypassed for bare names.
- Autocomplete candidates are hidden while a guided coordinate/dimension sequence is active.

## Validation table (expected vs actual)

| Workflow | Expected result | Actual status |
|---|---|---|
| Line → Move → Copy | line plus two transforms | **Not executed**; numerical line input and engine call wired; Move/Copy still use existing UI |
| Polyline → Join → Offset | editable connected curve then offset | **Partial**; staged path produces sketch segments. True polycurve join handled in PR #9 |
| Spline → Rebuild → Edit Points | editable spline and retained control points | **Not executed**; existing dialogs remain |
| Curve → Loft → Match Surface | selectable ordered sections, preview, match | **Not executed**; existing UI dialogs remain |
| Box → FilletEdge → Shell | consistent B-rep, fillet radius, shell, undo | **Not executed**; box numeric input added |
| Circle → Extrude → Difference | volumetric Boolean cut | **Not executed**; circle input added |
| Select → Hide → Show | hide/restore object | **Not executed**; existing visibility UI remains |
| Layer → Object Move | geometry reassigns to new layer | **Not executed**; existing layer UI remains |
| Save → Open → Edit | same document and geometry | **Not executed**; save format unchanged |
| Undo → Redo → Re-edit | same geometry restored | **Test authored, not run**; typed line undo/redo and document roundtrip assertions |

No workflow is reported passing without a real test run. This is an incremental core workflow improvement, **not** full Rhino behavior.

## Required test commands

```sh
cargo test --locked -p arclet --bin arclet command_workflows
cargo test --locked -p arclet --bin arclet typed_geometry
cargo test --locked -p arclet --bin arclet invalid_guided
cargo xtask ci
```

The working environment has no `cargo` or `rustc` and cannot clone GitHub directly; source changes were made through the GitHub connector. CI and real Windows/macOS screenshots are needed before declaring the product ready.

## Incomplete/explicit exclusions

- True single polycurve entity/Join (PR #9), smooth spline reconstruction, end-to-end fillet/shell/loft interaction, ordered section preview, 3D world coordinate digitizing, object preselection/selection filters, and generalized multi-stage command metadata/state machine.
- All commands support typed JSON parameters via registered ID/alias, **not** arbitrary free-text positional arguments. Prompt-based positional arguments are deliberately limited to Line, Polyline, Circle and Box.
- ArrowUp/ArrowDown still browse autocomplete suggestions, **not** recalled history entries.
- Existing engine errors and disabled command restrictions remain authoritative.
- No browser-specific functionality or prohibited third-party code added.

Changes a default: bare-name command behavior unchanged; the command field now also accepts explicit typed arguments.