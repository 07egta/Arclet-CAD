# C++ in Arclet: not used (Rust only)

Decision by the owner: Arclet is **Rust only**, desktop only. There is no C++ shim and no C++
build step. The optional Clipper2 shim that `solidcraft-polyclip-cpp` used to carry has been
removed; the crate is now pure Rust (convex polygons only) and nothing depends on it yet.

If a C++ library looks useful later (Manifold for mesh booleans, Clipper2 for 2D offsets), use a
Rust crate or write the algorithm in Rust instead. The old library survey is kept for the licence
notes only: [cpp-library-survey.md](cpp-library-survey.md).

The crate name `solidcraft-polyclip-cpp` is kept for now so the layering table and the lock file
stay valid; renaming it is a separate change.
