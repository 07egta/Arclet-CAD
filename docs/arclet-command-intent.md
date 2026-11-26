# Arclet desktop command intent: Boolean and Join correctness

Scope: Arclet-specific command-input semantics only. This does not replace
`solid.combine`, NURBS curve operators or the shared egui dialog framework.

## Reproducible defect

Before this change, typing `BooleanDifference`, `BooleanIntersection` or
`BooleanUnion` in Arclet resolved to the same `solid.combine` command.
The existing interactive Combine dialog defaulted to **Join**, whatever
the requested Boolean verb. If a user selected target and tool and pressed
Apply without adjusting the operation manually, Difference could perform a
Union; Intersection was similarly unsafe.

The old generic `Join`, `Explode` and `UnJoin` aliases started
`surface.stitch` or `surface.unstitch` even when the user was editing
sketch curves. This was not a real curve Join/Explode implementation.

## Implemented

- Carry explicit mode intent `Union / Difference / Intersection` through
  the Arclet command bar to the existing Combine dialog before Apply.
- The mode is also preserved by empty-Enter repetition and exact alias
  suggestion clicks. A partial-prefix search never silently guesses a mode.
- Generic `Boolean`, `Combine` and qualified `solid.combine` retain
  the ordinary user-selectable Join default. Starting a toolbar/menu command
  clears the previous special-mode repeat state.
- Generic `Join / Explode / UnJoin` report the unsupported curve operation
  instead of silently running a surface operation. Explicit `surface.stitch`,
  `stitch`, and `surface.unstitch` remain available. This **does not add**
  a curve joining kernel or silently turn a chain query into an edit.
- New strings are in `assets/locales/pending/arclet-command-intent.json`
  without rewriting generated language catalog files.

## Verification

Command-only tests check exact and unambiguous aliases, available registry
IDs, and unsupported Join requests. App-level tests invoke the same
`ArcletApp::run_command_text` entry used by the desktop prompt, inspect the
real `dialogs::Kind::Combine` mode, repeat it, and confirm that starting a
dialog or declining a Join request does not mutate sketch geometry.
The Rust CI command and result must come from the linked Actions run,
not this document. The tests do not claim that the GUI was mouse-operated
or that every Boolean kernel case is sound.

A manual acceptance checklist after CI:
1. Create two intersecting boxes in Arclet.
2. Type BooleanDifference; confirm the Combine dialog starts on Cut.
3. Select target/tool, preview, Apply. Measure remaining volume.
4. Undo/redo and save/reopen. Confirm the expected body remains editable.
5. Repeat for BooleanIntersection and BooleanUnion.
6. While editing a sketch, type Join or Explode; confirm it does not run
   Surface Stitch and explains the correct qualified alternatives.

No direct main push, force-push, or automatic merge.
