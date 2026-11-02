# Arclet parametric graph: opt-in incremental foundation

## Scope and compatibility

The existing `solidcraft_graph::Graph` and `solidcraft_ui_egui::node_editor::NodeEditor` remain the default paths. This change adds reusable **opt-in** APIs in `crates/graph`; it does not replace the desktop host, the mesh insertion action, the v1 JSON wire format or existing worker/revision handling. There is **no new executable or UI integration** in this branch.

The existing `Extrude` node still produces a **triangle mesh** from a polyline curve; `Merge Solids` combines triangle lists, not B-rep Boolean unions. Neither is NURBS.

This is not GH/GHX file compatibility or Grasshopper component/plugin compatibility.

## New APIs

| API | Behavior |
| --- | --- |
| `IncrementalEvaluator::update(&graph, &stop)` | Reuses outputs/errors for nodes with unchanged component and inputs and unchanged upstream dependencies. Ignores canvas position changes. |
| `IncrementalEvaluator::update_revision(&graph, revision, &latest, &stop)` | Cancels older revisions before publishing a cache update; caller still checks revision when delivering worker results to the UI. |
| `IncrementalEvaluator::current()` / `clear()` | Reads last complete evaluation or releases cached geometry and errors. |
| `Graph::connect_typed_checked(...)` | Rejects incompatible wires and cycles without mutating the previous connection. Does not replace the legacy `connect`/`connect_checked` APIs. |
| `Graph::type_issues()` | Describes incompatible existing wires using node IDs, socket indices, names and expected/actual types. |
| `GraphWorkspace::to_json()` / `from_json()` | Separate versioned workspace file for groups, per-node view state and external model-output references, with full validation before replacement. |

The type checker models the types implemented by the runtime (`Number`, `Boolean`, `Text`, `Point`, polyline `Curve`, triangle-mesh `Solid`, `Layers`, and polymorphic point-or-curve `Shape`). `Vector` and `Surface` have reserved type names but **do not yet have executable value payloads or surface nodes**. The type checker is conservative; a polymorphic `Shape` cannot be connected to a `Curve`-only input without a runtime type guarantee. Boolean-to-number compatibility preserves the existing numeric coercion.

`GraphWorkspace` stores `arclet.graph-workspace` version 1 and embeds the **unchanged existing graph v1 document**. Workspace group membership is flat and exclusive. Model references have unique host-defined object keys and can only target graph outputs declared as triangle-mesh solids. References persist across workspace save/load but do **not** automatically replace shapes in the CAD document; the host must implement object lookup, stale-reference handling, transactional updates, Undo integration and document persistence before this is advertised as linked modeling. A successful graph evaluation never overwrites CAD geometry by itself.

## Invalidation algorithm

1. Validate the complete graph (nodes, sockets, references, DAG) and calculate a topological order.
2. For each node in order, mark it dirty if its component or any input value/wire/modifier differs from the last completed graph, if an upstream node was dirtied, or if its cache is missing.
3. Reuse retained output trees and node errors for clean nodes. Recalculate only dirty nodes with the existing per-item cancellation and shared output budget.
4. Preserve existing per-node failure isolation. Before publishing, recheck the captured editor revision and cancellation token.
5. Publish the new graph snapshot and results together, only when the entire update completed without cancellation. Cancellation/stale work leaves the previous result untouched.

The cache is per workspace and remains in memory; **computed geometry is not serialized**. Host integrations should reuse one evaluator for consecutive edits, clear it on document replacement, and avoid sharing a mutable evaluator between workers. Current invalidation conservatively recomputes all descendants of an edited node, even when its actual output remains the same; a future output-fingerprint optimization can make this more selective.

The graph has an existing upper bound of 2,000 nodes and a global retained-output budget of 2,000,000 scalar/geometry units. The incremental cache reuses `Arc` mesh payloads where available, but snapshot/result ownership still has memory costs. This change does not claim measured GPU, browser, UI-latency, or real 3D-printer performance.

## Reproducible tests

Run from the repository root using the supported Rust toolchain:

```sh
cargo test --locked -p solidcraft-graph
cargo fmt --all -- --check
cargo xtask ci
```

The added tests (in the crate's **existing** `tests.rs`) cover:

- A small number → circle → extrude graph, independent branch reuse, edits, full-evaluation equivalence and legacy graph JSON save/load.
- Rejection of a point-to-curve connection without mutating the graph; inspection of legacy incompatible wires.
- Workspace groups, node-view state and model-link metadata save/load; rejection of invalid object output or missing group node.
- Cancellation during a long series plus revision-obsolete results, preserving the prior cached evaluation.
- A full 2,000-node chain, unchanged reevaluation and a mid-chain edit that invalidates only the affected half, plus graph JSON roundtrip.

**Verification status:** Source changes and tests have been committed for review. Local Rust compilation and benchmark timings have not been run in the present execution environment (Rust/Cargo are unavailable); CI results must be checked before merging. Do not infer that a drafted test proves behavior until it passes.
