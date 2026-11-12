# Arclet OCCT Studio

A working, experimental desktop CAD application: **Rust application code, Open CASCADE geometry**.
This is an independent implementation, not a completed translation of Serpentine3D and not a claim of Rhino parity.
The original Arclet applications and default geometry kernel are unchanged. This directory is a separate Cargo workspace.

## What is implemented

- Real exact B-rep primitives, rational NURBS curves, interpolated curves and polyline profiles.
- Extrude, revolve, solid/surface loft, union/difference/intersection, section, edge fillet/chamfer, transforms and arrays.
- Four real 3D views (perspective/top/front/right), depth-tested shading, edges, wireframe and a visual zebra preview.
- Mouse orbit/pan/zoom, ray-triangle object selection, Shift selection, grid-snapped polyline drawing.
- A shared command path for the desktop, terminal and atomic command batches.
- Layers, colors, visibility, geometry locks, measurements, bounded Undo/Redo and dirty-state tracking.
- Exact B-rep project save/open (`.arcocct`), STEP import/export and binary STL export.
- A dedicated OCCT worker thread. Native shape handles never cross threads; the UI receives copied measurements and meshes.
- Failed edits do not replace the original geometry or commit an undo step. Failed saves do not mark the document clean.
- Unsaved document replacement and operating-system close requests are guarded.

## Start from source

Requires Rust 1.85 or newer, a C++17 compiler and OCCT development libraries.

Ubuntu 24.04 / Debian development packages:

```sh
sudo apt-get install build-essential libocct-modeling-algorithms-dev libocct-data-exchange-dev libocct-visualization-dev libxkbcommon-dev libwayland-dev libx11-dev libxi-dev libxcursor-dev libxrandr-dev libgl1-mesa-dev
cargo run --release --features desktop -- --desktop --demo
```

The desktop uses OpenGL 3.3 and starts with multisampling disabled for compatibility.
For a custom OCCT installation set `OCCT_INCLUDE_DIR` and `OCCT_LIB_DIR` and make its shared libraries available to the loader.
OCCT 7.6 and 7.8 toolkit layouts are detected. The CI workflow separately checks Linux and Windows; read its result rather than assuming every platform works.

Windows/MSVC development: open a Visual Studio x64 developer terminal, install a compatible OCCT 7.8.1 runtime/development environment, then:

```powershell
$env:OCCT_INCLUDE_DIR = "$env:CONDA_PREFIX\Library\include\opencascade"
$env:OCCT_LIB_DIR = "$env:CONDA_PREFIX\Library\lib"
$env:PATH = "$env:CONDA_PREFIX\Library\bin;$env:PATH"
cargo run --release --features desktop -- --desktop --demo
```

## Controls

Right-drag orbits the perspective camera. Middle-drag pans. The wheel zooms. Click selects; Shift-click adds/removes a selection.
Home fits all visible objects. Ctrl+L focuses the command line. Ctrl+S saves, Ctrl+O opens, Ctrl+Z undoes, Ctrl+Shift+Z redoes.
The command line supports quoted names/paths, history and syntax suggestions. Geometry units are millimeters.
`Polyline` in the left toolbar starts drawing in the active construction plane; Enter finishes and Escape cancels.
The right panel exposes layers, measurements and selected-object editing.

Try these commands in an empty document:

```text
Box 0,0,0 60 40 8
FilletEdge 1 2
Cylinder 18,20,-1 5 10
BooleanDifference 1 2
Rename 1 "Mounting plate"
Save "plate.arcocct"
ExportStep "plate.step"
```

Identifiers are not recycled after Undo. Use the object panel or `List` to find current IDs.
Boolean commands normally replace the first object and consume the second. Add `--keep` to retain both input objects.
`Help` lists implemented commands and exact syntax; unsupported names produce an error, not a fake result.

## Headless and reproducible workflows

```sh
cargo run --release -- --demo output
cargo run --release -- --batch examples/bracket.commands
cargo run --release -- --atomic your-modeling-script.commands
cargo run --release -- --headless
cargo test --features desktop -- --test-threads=1
cargo clippy --features desktop --all-targets -- -D warnings
cargo fmt --all -- --check
```

An atomic batch commits all its modeling commands as one undo step or none. File I/O and Undo/Redo are intentionally excluded inside batches.
The headless command stream is not an MCP/JSON-RPC server; OCCT file writers can emit diagnostic output.

The actual-window regression runner uses XTest keyboard/mouse input and captures real screenshots:

```sh
pip install pillow
xvfb-run -a -s '-screen 0 1600x1000x24 +extension GLX' python scripts/ui_smoke.py --binary target/debug/arclet-occt-studio --output evidence/gui
```

`ARCLET_GUI_STATE` is an optional local, read-only observation file for this test. It does not listen on a network port or execute commands.

## Verification checkpoint

The 2026-11-12 Linux checkpoint passed **52 Rust tests** (45 geometry/document/command regressions + 7 UI/worker/camera tests), strict Rust Clippy, and **20 actual-window input/render/save/export/close checks**. The screen checks verify that all four viewport regions contain colored geometry, not just that scene metadata exists.
Tested locally with Rust 1.99.1, OCCT 7.6.3 and software OpenGL under Xvfb. New CI results may add platform coverage; local success is not a Windows GPU certification.
See `QA.md` for what was actually checked and what remains unverified.

## Important limits

This is a development build. Keep separate backups of important models.

There is no 3DM/GH/GHX/plugin compatibility, Grasshopper/Hydra implementation, persistent feature graph, cross-session Undo, advanced surface continuity matching, general G2/G3/Class-A guarantee, or production-scale benchmark claim. The zebra mode is a visual normal-reflection preview, not a continuity certificate.
Edge indices can be entered for fillets/chamfers, but interactive per-edge picking is not implemented. The GUI currently uses English labels. Only millimeters are accepted in native project files.

Undo retains at most 128 snapshots; documents allow 2,048 objects and 256 layers; serialized projects are limited to 64 MiB. Display meshes have explicit budgets. These limits reduce accidental resource exhaustion but are not a hard native memory/time sandbox. A thread cannot safely kill a stuck OCCT call and does not contain a native segmentation fault. Do not open untrusted CAD files in a sensitive environment.

## Licensing and independence

Original application and adapter code here is MIT licensed. Open CASCADE Technology is **separately licensed under LGPL-2.1 with the OCCT exception**, not MIT. This program makes use of facilities provided by Open CASCADE Technology. It dynamically links OCCT; replacing compatible runtime libraries is not prohibited. Audit licenses and corresponding-source requirements when redistributing native dependencies.
The owner's request to retain OCCT is scoped to this experiment; it does not silently relax the existing Arclet project's policy elsewhere.

Serpentine3D was an architectural reference, not a source-code or asset donor. No commercial CAD code, icons, templates or branding are copied. This project is not affiliated with Rhino, McNeel, Serpentine3D or Open Cascade.
