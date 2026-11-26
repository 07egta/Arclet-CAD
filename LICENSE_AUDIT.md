# Licence audit: Arclet / SolidCraft workspace

Date: 2026-11-02
Scope: the `arclet` binary's normal (non-dev) dependency closure, plus the workspace's own licence files.
Not legal advice. No legal review has been done.

## 1. Commands run and outcome

| Command | Outcome |
|---|---|
| `cargo xtask licences --check` (built with `CARGO_TARGET_DIR` in the session scratchpad) | Exit 0. Printed: `licences: THIRD-PARTY-LICENSES.txt will change when this lands (the Generated files workflow rewrites it on main)` |
| `cargo tree -p arclet --prefix none --edges normal` (default target, Linux) | 348 unique package lines |
| `cargo metadata --format-version 1 --locked` | Read the `license` field of every package, then filtered to the 348-package closure. Every closure package resolved to a row; none unmatched |

## 2. What the xtask check does and does not prove

- `--check` regenerates THIRD-PARTY-LICENSES.txt from `cargo metadata` and compares it with the committed file. It fails only if `cargo metadata` cannot be read. A stale file is reported, not treated as an error.
- It does not enforce a licence allow-list or deny-list. A GPL, LGPL, AGPL or MPL crate would not make it fail.
- So the exit 0 means the generator runs. It is not evidence that the dependencies are licence-compatible.
- The committed THIRD-PARTY-LICENSES.txt is out of date against the current Cargo.lock: the check says it will change, and it lists 560 crates for all shipped programs.

## 3. The project's own licence

- Workspace `Cargo.toml`: `license = "MIT OR Apache-2.0"` (every local crate inherits it via `license.workspace = true`).
- Root files present: `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `THIRD-PARTY-LICENSES.txt`.
- `apps/arclet/Cargo.toml`: `license.workspace = true`, `publish = false`.
- `vendor/` crates keep their upstream licences: egui-wgpu and egui-winit are MIT OR Apache-2.0; matext4cgmath and the truck-* crates are Apache-2.0.

## 4. Licence families in the closure

Counts from the `cargo metadata` `license` field (348 packages, Linux target):

- MIT OR Apache-2.0 (and close variants): about 280 packages
- MIT alone: 64
- Apache-2.0 alone: 27
- Zlib, BSD-2/3-Clause, ISC, Unicode-3.0, Unlicense OR MIT, 0BSD: a few each
- Apache-2.0 WITH LLVM-exception: 4
- Font and data licences: 1 package each for OFL-1.1 with Ubuntu-font-1.0 (`epaint_default_fonts`)

No package in the closure is MPL, AGPL or EPL-only.

## 5. Flagged crates

Each flag is a copyleft-family term in an OR or AND expression. None is GPL-only.

1. **self_cell 1.3.0**: `Apache-2.0 OR GPL-2.0-only`. Pulled in by epaint, egui and eframe (so by `arclet` itself). This is a dual licence. The Apache-2.0 option can be taken. The package ships both LICENSE-APACHE and LICENSE-GPLv2. Action: record that Apache-2.0 is the chosen option, or replace the crate. Not checked: whether the upstream Apache option is the only one the project relies on.
2. **epaint_default_fonts 0.36.2**: `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`. Bundled font data, not code. The Ubuntu Font Licence 1.0 is not in ATTRIBUTION.md. Action: confirm that the Ubuntu font licence text and attribution are shipped with the binary.
3. **r-efi 5.3.0 and 6.0.0**: `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. Reached through getrandom, tempfile and `cadmpeg-codec-freecad` (in `solidcraft-io`) on the Windows and UEFI targets, not the Linux tree above. The MIT and Apache options are available. Action: none beyond recording the choice. This is the only LGPL-family term found.

## 6. Not checked

- The `cargo tree` above is for the Linux target only. Platform-specific dependencies on Windows, macOS and wasm32 were checked only through the `cargo metadata` field for each package that appears in the closure.
- The `license` field is the declared SPDX expression. The text of the licence files inside each crate was not compared against it, except for the three flagged crates.
- OpenNURBS and OpenCASCADE are not dependencies. This was confirmed by the absence of those names in the closure. Other GPL-family dependencies (FreeCAD, SolveSpace, CadQuery, LibreCAD, QCAD) were not searched for by name beyond what the closure lists.
- No copyright, patent or trademark review. No review of cadmpeg's own licence terms (its crates are Apache-2.0 per the workspace comment), beyond the `license` field.
- The three flagged crates are the only ones inspected by hand. The rest rely on the `license` field.
- The AGENTS.md rule "No GPL/LGPL/AGPL code" is not enforced by `cargo xtask licences --check`, which does not fail on licence content.
- Not legal advice.

## 7. Opt-in Arclet native C++ bridge (Mission 17)

The optional standalone native crates are not included in the normal arclet binary closure audited above. The dedicated Windows/macOS native-cpp-bridge workflow uses pinned upstream revisions (listed in docs/native-cpp-bridge.md). Clipper2 is BSL-1.0 and Manifold's core is Apache-2.0. Manifold optional TBB/parallel, nanobind/Python, GTest, Assimp and Tracy are deliberately disabled, and CMake downloads are disabled. The Rust bridge adds no new external Cargo packages: it uses already-locked cc and thiserror. Preserve upstream licence/NOTICE obligations in any distribution that actually includes the native libraries. These dependencies and optional build flags must be re-audited if the upstream revisions change. The project-wide cargo xtask licences --check command does not inspect the actual external C++ source closure and does not enforce the GPL/LGPL/AGPL exclusion.

Eigen (MPL-2.0, file-level obligations) and Ceres (BSD-3-Clause core, review optional dependencies) have **not** been adopted in the bridge. No GPL/LGPL/AGPL or unverified openNURBS library was added. See docs/native-cpp-bridge.md for limits and review gates.
