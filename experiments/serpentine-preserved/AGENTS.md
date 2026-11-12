# Serpentine original-UI experiment

Owner direction, 2026-11-12:
- Preserve the upstream Serpentine3D UI and interaction design. No egui replacement.
- Improve native modelling reliability and measured performance iteratively.
- Subagents may use any available model; there is no Luna-only requirement.
- Only report agents actually launched and checks actually executed.
- Rhino-level or better capability is an aspiration, not a current certification.

This is an opt-in, isolated checkout/patch experiment. It does not change the
main Arclet workspace, its default kernel, licence policy, or the separate Rust
OCCT experiment. Upstream remains MIT; its native/runtime dependencies retain
their own licences. Do not import these dependencies into the main workspace.

Pin upstream and verify source checksums before applying patches. Keep all UI
and asset files byte-identical. Add reproducible regression tests before fixes.
Measure equal input/output, not speed alone. No geometry tolerance relaxation,
feature removal, skipped failing tests, or unbounded caches for benchmark wins.
Use bounded CI workers; these native tests can consume gigabytes per worker.
Do not distribute standalone fonts, credentials, user data, or build caches.
Leave this work in a draft PR until the new checks and package inspection pass.
