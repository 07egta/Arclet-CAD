# Arclet OCCT Studio (experimental)

Independent, opt-in Rust application layer around Open CASCADE. This is a bounded implementation of the owner's 2026-11-12 request to keep OCCT and replace the application layer with Rust. It is NOT a completed Serpentine3D port, a Rhino replacement, or part of Arclet's default kernel. Existing apps, main, and pending PRs are left unchanged.

Rust owns document state, transactional commands, undo/redo, validation and project persistence. An original small C++ C-ABI adapter calls the existing OCCT libraries and catches C++ exceptions. Shapes are immutable at the Rust boundary and deliberately not Send/Sync. Meshing copies topology before modifying triangulation caches.

## Build

On Ubuntu/Debian, install `build-essential libocct-modeling-algorithms-dev libocct-data-exchange-dev libocct-visualization-dev`. With Rust 1.85 or later, run from this directory:

```
cargo test --no-default-features
cargo run --release -- --demo output
cargo run --release -- --batch examples/bracket.commands
cargo run --release -- --headless
```

For custom installations set `OCCT_INCLUDE_DIR` and `OCCT_LIB_DIR`. Link dynamically; OCCT 7.6 and 7.8+ toolkit layouts are detected. Do not claim Windows/macOS support until their builds and interactive workflows have actually been exercised.

## Safety and limits

Edits are staged and commit only after successful native work. Undo retains up to 128 snapshots; IDs are not recycled by Undo. Failed imports do not replace the document. Project saves and exports use same-directory temporary files followed by rename. Input and mesh limits reduce accidental resource exhaustion; they do NOT make untrusted CAD parsers a security sandbox. A worker thread does not contain native segmentation faults. Project files retain geometry, names and layers, not a persistent parametric feature graph or cross-session Undo history.

## License

Original code in this experiment is MIT licensed. It dynamically links Open CASCADE Technology, licensed separately under LGPL-2.1 with the OCCT exception. Dynamic linking does not remove distribution obligations. Provide the applicable licenses/notices, matching source and the required relinking/replacement facilities for any binary distribution; audit all bundled dependencies. The owner's OCCT permission is scoped to this experiment: the existing Arclet no-LGPL policy is NOT silently changed.

Architecture reference: Serpentine3D (chisomobanzi/Serpentine3D), MIT; no Python source, assets or branding copied. Read `evidence/` and CI logs for actually executed checks. Source added here is not proof that compilation, tests or native UI checks passed.
