# Window backend selection with accessibility

This is egui-winit 0.36.2 from the crates.io archive with SHA-256
`98466000559d66ba4db786a30448b01483946add4188e7f10f522a27b4fb89cc`.
Its upstream revision is
[`49682f8baa058bf49e011035cfbd6e825f88a5ef`](https://github.com/emilk/egui/tree/49682f8baa058bf49e011035cfbd6e825f88a5ef/crates/egui-winit).
All five Rust source files are unchanged from that archive.

The patch changes Cargo metadata only. The optional `accesskit_winit` dependency
disables its default features and explicitly keeps `accesskit_unix`, `async-io`
and `rwh_06`. Its old defaults also enabled both `winit/x11` and `winit/wayland`,
which silently enabled Wayland in an X11-only application.

This crate already forwards its own `x11` and `wayland` features to the same
`winit` dependency. Those feature definitions and this crate's default feature
set stay unchanged. Hosts that request both backends retain both. AccessKit's
Windows and macOS adapters remain target-specific dependencies; its Unix adapter
and asynchronous runtime are kept. Accessibility is not disabled by this patch.

Arclet is built separately with its X11 feature selection. A workspace-wide
build can unify features requested by other applications, including Wayland;
that is ordinary Cargo feature unification and is not changed here. The Arclet
selection avoids the unrequested Plasma protocol dependency. No protocol source
or protocol XML is copied into this directory.

`Cargo.toml.orig` retains the upstream manifest. The normalized `Cargo.toml`
records this change and points its package include list at the local license
files and this note. Both license files were retrieved from the same upstream
revision:

- [LICENSE-MIT](https://raw.githubusercontent.com/emilk/egui/49682f8baa058bf49e011035cfbd6e825f88a5ef/LICENSE-MIT)
- [LICENSE-APACHE](https://raw.githubusercontent.com/emilk/egui/49682f8baa058bf49e011035cfbd6e825f88a5ef/LICENSE-APACHE)

Dependency-tree checks verify feature selection; they do not establish screen
reader behavior or replace native application testing on each operating system.
