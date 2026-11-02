# Arclet CAD: assembly and mechanism integrity hardening (Mission 13)

## Existing engine reused

The repository already has:
- Components, nested component definitions, multiple occurrences of shared geometry, placement transforms, grounding and occurrence copying.
- `JointKind::Rigid` (Fixed), `Revolute`, `Slider`, `Cylindrical`, `Ball` and `Planar`, plus pin-slot and geometric relation types.
- Joint origins with persistent face/edge references, per-value limits, motion links (gears, racks, screws, cams), closed-loop solving, conflict messages, free-motion counts and motion studies.
- Contact sets, interference commands and motion export in existing engine layers.

This PR **does not build another assembly system**, change the saved document schema, replace the solver or alter B-rep/Boolean, STEP/NURBS, undo or application UI. The current legacy placement and solver APIs retain their interfaces.

## Additions

| API | Purpose |
|---|---|
| `Document::validate_assembly_tree()` | Detect unknown component/occurrence references, duplicate IDs, component-parent and occurrence cycles, oversized graphs and non-rigid/non-finite placement matrices. Does not mutate a document. |
| `Document::placements_checked()` | Indexed full occurrence-path traversal that explicitly rejects excessive nesting or instance expansion instead of returning silently truncated results. |
| `Document::body_frames_checked(&model_state)` | Fully validated world-space body frames with stable definition names and exact occurrence paths for downstream interference/clearance consumers. This does not perform collision detection. |
| `Document::validate_mechanism()` | Validates joint endpoints, numbers, bounds, link ports, duplicate drivers, link cycles and motion study references before a host drives the mechanism. Existing default solving remains unchanged; callers opt in. |
| `MotionStudy::validate(&assembly)` | Validates keyed joint ports, monotonic finite steps, bounded ranges and duplicate keys before playback/export. |
| `JointKind::parse("Fixed Joint")` | Maps to the existing serialized `Rigid` kind, without changing its stored name or behavior. |

The existing `Document::placements()` still returns the historical best-effort expansion for backwards compatibility. Importers, contact/interference consumers and future CAD host integrations should call the checked API when complete coverage is required; **this PR does not migrate other agents' code to it**.

The existing `apply_joint_solution` still resolves placements relative to their parent frames and retains joint identities; it is not replaced.

## Verification cases (new Rust unit tests)

- **Door and hinge:** Revolute rotation around a root joint; world-space transformed point and 1 DOF.
- **Cylinder and piston:** Slider travel along a local axis and joint limits.
- **Wheel and axle:** Cylindrical angle and translation simultaneously, 2 DOF.
- **Sliding drawer:** Slider along world X and clamping to a 10 mm limit.
- **Rigid/Fixed, Planar, Ball:** frame transforms and their reported DOF; note that Ball uses the **pre-existing Z-Y-Z angle convention**.
- **Two-link arm:** two connected revolute joints, elbow position, endpoint rotation and remaining 2 DOF for the distal part.
- **Closed constraints:** mutually incompatible rigid mates yield a conflict, not silent success.
- **Repeated instances:** the same wheel definition under 600 axle instances, hierarchy, exact instance paths and checked expansion.
- **Roundtrip:** saved component occurrences, joints and studies reload with the same IDs/values.
- **Hostile input:** unknown parents, cyclic hierarchies, duplicate IDs, invalid rigid matrices, stale joint IDs, non-finite limits, duplicate motion drivers, cyclic links, invalid study keys and excessive nesting.

Tests are source-only until CI completes. The tests use exact joint-frame geometry and analytic expected coordinates, **not mesh-intersection tests or hardware motion validation**.

## Current limits and future integration

- Limits and joint motion use radians and millimetres internally; user-facing angle commands use degrees. The new checks reject non-finite bounds, but do not change historical reversed-limit handling.
- `Ball` uses the existing Z-Y-Z parameterization. It gives three rotational parameters but can be singular; it is not a dynamic spherical bearing simulation.
- Remaining DOF and loop closure are the existing numerical analyses. Their larger-loop caps and floating component behavior are unchanged; a clean reference check does **not** prove every pose can close.
- No force, torque, friction, inertia, mass-driven dynamics, elasticity or contact impulses are simulated by this change. Contact/collision accuracy remains that of the existing geometry/interference pipeline.
- Joints attached to geometry by persistent names continue to resolve through existing geometry attachment APIs. This PR does not create a new persistent-topology naming system.
- Shared component definitions remain shared by instances; instance paths identify placements, not independent geometry copies.

## Tests to run before merging

```sh
cargo test --profile ci --locked -p solidcraft-doc --lib arclet_
cargo test --profile ci --locked -p solidcraft-doc --lib assembly_
cargo test --profile ci --locked -p solidcraft-doc --lib
cargo clippy --profile ci --locked -p solidcraft-doc --all-targets -- -D warnings
cargo xtask ci
```

The dedicated `Arclet assembly mechanisms` GitHub Actions workflow runs the focused tests, the full document-crate tests and Clippy. Source checks and GH PR creation do not establish a completed simulation or verify physical machines. **No local Rust/Cargo execution was possible in the authoring environment; remain Draft until CI is green and real application acceptance is complete.**

**Changes a default:** No. **Saved format change:** None. **Melt3D changes:** None. **GH/URDF compatibility:** Only existing supported behavior, not expanded by this PR.
