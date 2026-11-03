# Arclet native C++ FFI bridge (Mission 17)

**Status: opt-in experimental bridge. Wired into the engine as commands (`sketch.offset_native`, `sketch.boolean_native`, `mesh.boolean`), desktop only, behind the engine features `clipper2` and `manifold`, both off by default. Unverified until CI: this wiring was written without a local build.** Without the feature (and in wasm) those commands return an ordinary error saying the backend is not built in. `mesh.boolean` returns a triangle mesh (not NURBS/B-rep) from the display meshes of two bodies and never replaces a body; the polygon commands take explicit [x,y] rings in mm and change nothing in the design. Build with e.g. `cargo build -p solidcraft-engine --features manifold`.
The presence of an implementation or an unexecuted GitHub workflow does **not** prove it works. Check the PR's native-cpp-bridge Windows/macOS test logs for actual results.

## Architecture

- Existing solidcraft-polyclip-cpp: default **pure Rust** convex polygon offset and convex intersection; optional Clipper2 for complex offset and all polygon boolean operations. Its fixed-point coordinates have resolution 1e-6 in caller units. Returned rings do not encode a parent/hole hierarchy. Miter joining can differ across backends.
- New standalone solidcraft-manifold-cpp: optional Manifold feature provides mesh Union, Difference and Intersection. Input and output are oriented triangle meshes, **not accurate NURBS/B-rep**. Material, UV and normals are not preserved. Invalid topology may be rejected by Manifold.
- Neither crate depends on crates/kernel or changes B-rep APIs. If Manifold is disabled, calling its Boolean API returns Error::Unavailable rather than a fake result. If Clipper2 is disabled, unsupported pure-Rust operations return Error::Unsupported. An enabled native feature with missing source or compiler fails at build time.
- Native desktop work only; no WASM claim or product-wide backend toggle.

## Reproducible prerequisites

Rust matching workspace rust-version, a C++17 toolchain (MSVC/Apple Clang), CMake >= 3.18, and upstream sources explicitly checked out **outside** the Arclet source tree. No native upstream is vendored or auto-downloaded.

Pinned CI sources:
- Clipper2 BSL-1.0, AngusJohnson/Clipper2 commit 4be4655694d7e15e76ac6bfe9024fa534eed3428
- Manifold Apache-2.0, elalish/manifold commit 0209eae1b279735c966bb490d0ab91926fe27c69

In a POSIX shell:

    export CLIPPER2_SRC=/absolute/path/Clipper2/CPP/Clipper2Lib
    export MANIFOLD_SRC=/absolute/path/manifold
    cargo test --locked -p solidcraft-polyclip-cpp --features clipper2
    cargo test --locked -p solidcraft-manifold-cpp --features manifold

In Windows PowerShell:

    $env:CLIPPER2_SRC = 'C:\path\Clipper2\CPP\Clipper2Lib'
    $env:MANIFOLD_SRC = 'C:\path\manifold'
    cargo test --locked -p solidcraft-polyclip-cpp --features clipper2
    cargo test --locked -p solidcraft-manifold-cpp --features manifold

Non-native tests and unavailable-backend checks:

    cargo test --locked -p solidcraft-polyclip-cpp
    cargo test --locked -p solidcraft-manifold-cpp

Manifold builds a static C++ library via CMake; optional Parallel/TBB, Python, C bindings, GTest, Assimp and Tracy are disabled, as are CMake dependency downloads. The Clipper2 shim is built by the Rust cc build dependency. Do not silently bump upstream revisions.

## FFI and ownership safety

- Rust validates sizes, finite coordinates, vertex ranges, repeated triangle indices and obvious zero-area triangles before an FFI call. The C++ Manifold shim independently rejects null pointers, bad lengths and coordinates. Topological manifoldness remains Manifold's responsibility.
- A stable extern C ABI only exposes scalar arrays, counts, status codes and opaque C++-owned result objects. C++ exceptions are caught and reported; never unwound across Rust.
- Allocation and free are paired inside the C++ shim. Rust RAII Handle drops on normal and error paths. Clipper2's result creation uses RAII to avoid an allocation leak if a vector push throws.
- Rust bounds-checks reported buffer sizes, multiplication, pointers and indices before copying native data to owned Rust vectors.
- Unsafe is scoped to crates/polyclip-cpp/src/cpp.rs and crates/manifold-cpp/src/ffi.rs. Each owning crate denies unsafe by default. The workspace-wide unsafe_code=forbid setting is unchanged.
- The FFI does not prevent all C++ process-level failures (e.g. abort/undefined behaviour) and a repeated-call test is not proof of leak freedom. Sanitizers and heap analysis are needed before shipping.

## Results, comparisons and limitations

The dedicated native-cpp-bridge GitHub Actions job **attempts** the no-feature, Clipper2 and Manifold feature tests on both Windows and macOS. It compares convex polygon intersection results against the pure Rust implementation and runs an ignored performance measurement with total nanoseconds printed to its logs (2,000 calls each). No speedup is claimed before results are observed.

Manifold tests independently integrate signed tetrahedron volumes of overlapping unit cubes: expected Union 1.875, Difference 0.875, Intersection 0.125. They also cover disjoint intersection, invalid input, topology rejection and 100 repeated calls. This is **mesh** validation; it does not verify an exact CAD Boolean or preserve feature history.

A fair Manifold-vs-Rust mesh-Boolean speedup comparison is **not yet possible** inside this standalone crate because there is no equivalent Rust mesh Boolean algorithm. Comparing different B-rep and mesh outputs would be misleading. Any future representative workload comparison must specify geometry, size, tolerance, CPU/compiler, memory and correctness, not just elapsed time.

## License and dependency review

| Feature | Native dependency | Terms | Native transitive dependencies |
|---|---|---|---|
| clipper2 | Clipper2 core (pinned revision) | BSL-1.0 | no additional native dependency of this shim identified |
| manifold | Manifold static core (pinned revision) | Apache-2.0 | optional TBB, nanobind, GTest, Assimp, Tracy disabled; CMake downloads OFF |
| neither | Rust cc build dependency and thiserror | existing Cargo.lock licenses | no new Rust package versions |

For Clipper2, retain the Boost licence and copyright notices as the licence requires. For Manifold redistribution, provide the Apache-2.0 licence, relevant upstream NOTICE if applicable and modification notices when modifying source. License declarations and source trees must be re-reviewed before shipping; the Rust licence-check task **is not a copyleft deny-list**.

**Eigen:** header-only MPL-2.0, weak copyleft applies to modified Eigen source files; not linked here. Benchmark a defined workload versus existing Rust linear algebra first and account for FFI overhead. **Ceres Solver:** BSD-3-Clause core with additional licensed source/dependency pieces; not linked here. No replacement for the Rust sketch solver until a measured quality or performance deficit is shown. No GPL/LGPL/AGPL or unverified openNURBS source introduced.

## Review gate

The PR must list the actual per-platform build/test results, compiler versions if available and measured benchmark output. If CI has not run or is red, it must remain draft. Future release work must include sanitizer-based or platform-heap leak checks. No direct push to main.
