# C++ libraries in Arclet (`solidcraft-polyclip-cpp` is the first)

## Direction (decided by the project owner, 2026-11-02)

- Arclet is a **desktop** application. It does not have to run in the browser, so desktop builds
  may use C++ libraries freely where they give a real gain. The wasm build keeps the pure-Rust
  fallback only.
- Reusing parts of the SolidCraft code base is allowed where it helps (Arclet is built on it).
- Licence limits still apply until the owner changes them: no GPL/LGPL/AGPL (so no OpenCASCADE),
  and nothing whose licence grant is unclear (so no openNURBS yet). See `docs/cpp-library-survey.md`.
- Candidates in order: Clipper2 (2D offset/boolean, BSL-1.0, spike done), Manifold (solid
  booleans, Apache-2.0), Eigen and Ceres (linear algebra and constraint solving, MPL-2.0 and BSD-3).
- The old text below was written before this decision. Where it says "not in wasm" or "off by
  default", read it as the current state of the spike, not as a ban on enabling it for desktop.


Status: spike. Not part of any default build, not part of the product, and not in wasm.

## Why this is an exception to "Rust only"

AGENTS.md says application code is Rust only. This crate is a deliberate, optional exception to
that rule. It lets us measure whether Clipper2 (C++, BSL-1.0) would be a better 2D polygon offset
and boolean backend than the pure-Rust code. The exception is limited in four ways:

- **Optional.** The `clipper2` cargo feature is off by default. Nothing in the workspace depends
  on the crate, so `cargo build`, `cargo test` and `cargo xtask ci` for the product never compile
  C++.
- **Not in wasm.** The C++ shim needs a native C++ toolchain. The crate's default build is pure
  Rust, but it is not a dependency of `apps/solidcraft-web` or any wasm target.
- **Not vendored.** Clipper2 source is not in this repository. The build reads it from a checkout
  named by the `CLIPPER2_SRC` environment variable.
- **Its own unsafe surface.** The shim is reached through FFI, so `src/cpp.rs` allows
  `unsafe_code`. That module is the only unsafe code in the crate. The rest of the workspace keeps
  `unsafe_code = "forbid"`. Adding this crate to the unsafe allowlist would be a separate decision
  for the crash-safety rules, and it is not made here.

## Layout

| Path | Purpose |
|---|---|
| `crates/polyclip-cpp/src/lib.rs` | Public API (`offset_polygon`, `boolean`, `Error`), input limits, `backend()` |
| `crates/polyclip-cpp/src/pure.rs` | Default backend. Convex offset (miter) and convex intersection only |
| `crates/polyclip-cpp/src/cpp.rs` | Clipper2 backend (feature `clipper2`): all operations, fixed-point scale 1e-6 |
| `crates/polyclip-cpp/cpp/shim.cpp` | C ABI over `Clipper64` and `ClipperOffset`. Exceptions never cross the ABI |
| `crates/polyclip-cpp/build.rs` | Compiles the shim and Clipper2's `clipper.engine.cpp` and `clipper.offset.cpp` with `cc` |
| `crates/polyclip-cpp/tests/api.rs` | Tests for both backends, plus a deterministic hostile-input test |

The crate is registered in `xtask/src/layers.rs` as a standalone crate. It depends on no workspace
crate.

## Behaviour

Default build (pure Rust):

- `offset_polygon`: convex simple polygons only. Miter joins, unlimited miter length. An inward
  offset that reverses an edge returns no rings (the polygon collapsed). Non-convex or
  self-intersecting input returns `Error::Unsupported`.
- `boolean`: intersection of two convex polygons only. Union, difference and xor return
  `Error::Unsupported`.

`clipper2` feature:

- `offset_polygon` and `boolean`: any simple polygon, all four boolean operations.
- Coordinates are rounded to 1e-6 before Clipper2 sees them. Results are on that grid.
- The miter spike limit is 1000 times the offset distance. The pure backend has no limit, so the
  two can differ for very sharp corners.
- Clipper2 may return a ring with a different start vertex or winding than the input.

Hostile input, both builds: empty or fewer than three points, NaN or infinite coordinates or
offsets, zero-area rings, more than `MAX_POINTS` (1,000,000) points, and magnitudes above
`MAX_COORD` (1e9) return `Error`. None of them panic. `hostile_values_never_panic` checks a
deterministic mix of such values.

## Building and testing

Default (pure Rust, no C++):

```sh
cargo test -p solidcraft-polyclip-cpp
```

With Clipper2. Clone it outside the repository, then point the build at its library directory:

```sh
git clone --depth 1 https://github.com/AngusJohnson/Clipper2 /path/outside/repo/Clipper2
CLIPPER2_SRC=/path/outside/repo/Clipper2/CPP/Clipper2Lib \
  cargo test -p solidcraft-polyclip-cpp --features clipper2
```

Needs a C++17 compiler. The build fails with a clear message if `CLIPPER2_SRC` is not set.

## Licence

- The crate's own code is MIT OR Apache-2.0, like the rest of the workspace.
- Clipper2 is BSL-1.0 (Boost Software License 1.0). It is not included in this repository, so no
  `ATTRIBUTION.md` or `attribution/pending` row is needed for it. Anyone who builds with the
  feature downloads it and must keep its licence notice.
- No GPL, LGPL or AGPL code is involved. `cargo xtask licences --check` is unaffected because
  Clipper2 is not a Cargo dependency.

## Open questions before any real use

- Whether the unsafe FFI surface is acceptable under the crash-safety rules, or should move behind
  a separate, reviewed allowlist entry.
- Whether a 1e-6 fixed-point grid is precise enough for CAD units, or whether Clipper2's double
  API (`ClipperD`) should be used instead.
- Clipper2 is not wired into the product, and no performance measurements exist for it here. The
  spike shows that the API can be called safely and that results agree with the pure backend on
  convex inputs. It does not show that it is better.
