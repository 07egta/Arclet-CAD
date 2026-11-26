# Arclet CAD — product render material/optical quality acceptance

This is a reproducible **CPU-renderer** quality gate, not a GPU performance benchmark,
a calibrated material library, a path-traced marketing render, or evidence that the
desktop application UI has been exercised.

## Current renderer capabilities, kept separate

| Consumer | What it actually evaluates | Not part of that consumer |
| --- | --- | --- |
| Built-in CPU raster | Existing per-triangle normals, opaque GGX/Schlick BRDF, three fixed direct lights, clear coat, linear-light alpha blending and 2×2 sampling | Refractive glass, cast shadows, interreflections, indirect GI |
| CPU raytrace (native feature) | Scene ray intersections, explicit lights and visibility, reflection/refraction transport, dielectric absorption and supported scattering, image accumulation | Unbounded/path-complete global illumination or certified noise-free convergence |
| Optional GPU executor | Only capabilities advertised by its installed `gpu-path` descriptor; explicit sample/depth, lens, tone map and denoise controls | No capability claimed merely because intent can be saved |
| CPU photographic image effects | Bounded postprocessing applied to an existing image | An optical camera, accurate bloom physics or recalculated geometry |

### Optical data

`surface::OpaqueFinish::surface()` gives **illustrative linear RGB** and validated
single-scattering GGX parameters. The recipes are SatinAluminium, PolishedMetal,
MattePlastic, GlossyPlastic, UnglazedCeramic, GlazedCeramic, UntexturedWood
and VarnishedWood. No alloy conductor (n, k) measurement is implied.
VarnishedWood has a coat but intentionally **no grain**: wood grain requires
the existing texture/mapping pipeline and a separately justified resource.

`transmission::OpticalFinish::data()` returns a validated interface and
a distinct volumetric medium for ClearGlass, BlueGreenGlass, SmokedGlass and
ScatteringPolymer. The latter requires a supported **closed** dielectric
scattering volume. Glass tint is Beer–Lambert absorption **per millimetre**,
not a scalar RGBA alpha. A saved material or preview mesh does not automatically
activate the CPU raytrace dielectric selector.

Roughness is perceptual roughness, not a measured surface RMS; surface
reflectances and studio lighting are linear sRGB, not byte-encoded sRGB.
The renderer must apply the exposure and tone map **once** after linear light
integration, and then encode sRGB once at the PNG boundary. EXR is a separate
linear master and must not receive the display transform as if it were PNG.

### Deterministic generation

On a Rust-capable checkout:

```sh
cargo test --locked -p solidcraft-render --lib
cargo test --locked -p solidcraft-render --lib --features raytrace-cpu
cargo clippy --locked -p solidcraft-render --all-targets --features raytrace-cpu -- -D warnings
cargo run --locked -p solidcraft-render --example premium_product_gallery -- arclet-render-evidence
```

The last command refuses an already existing output directory. It creates
six **actual** `ProductScene`-backed PNGs plus a `receipt.json` recording
their dimensions, PNG SHA-256, subject pixel count, near-white / near-black
counts, mean linear subject luminance, triangle count and measured wall time:

1. `metal-chair.png`: separate box meshes, hard normals, satin metal finish.
2. `transparent-container-alpha-preview.png`: a tubular shell and visible insert, explicitly an **alpha preview**, **not** refractive glass.
3. `matte-plastic-case.png`: matte body and glossy lid.
4. `untextured-wood-furniture.png`: an assembled table with uniform wood and coated top.
5. `roughness-spheres.png`: identical metallic colour, four different GGX roughnesses.
6. `multi-material-product.png`: a lamp with polished metal, matte polymer, ceramic and satin details.

The optics-specific regression in `raytrace/image_tests.rs` renders and compares
real *clear glass* and *tinted glass* under a constant environment, using the
CPU raytrace dielectric transport. To emit those two additional PNG files,
set `ARCLET_RENDER_OPTICAL_EVIDENCE` to a writable directory when running
the feature-enabled tests. The separate
[render-quality workflow](../.github/workflows/arclet-render-quality.yml)
does both, and uploads its PNGs and receipts as an Actions artifact.

The gallery records timings for its **actual** host environment only. It
does not compare against the main branch, and no speedup claim is made.

### What to inspect in the generated images

- Verify that chair, table and lamp have physically consistent face normals,
  no missing triangles and convincing front/back surface behavior.
- Check equal base colours at different roughnesses on the sphere strip;
  compare highlight width and intensity, not only pixel-average brightness.
- Check that the matte box is not accidentally as sharp as the glossy lid.
- Examine glass shell transparency **only as a preview**. For physical glass
  compare `raytraced-clear-glass.png` and `raytraced-tinted-glass.png`:
  the green-channel transmitted radiance must survive more than red in the
  tinted sheet; this is tested on decoded actual PNG pixels.
- Check near-white / near-black counts and their positions. A zero count is
  not a guarantee of accurate exposure; large counts require identifying
  whether they are material highlights, geometry gaps or output clipping.
- Check transparency seams at crossed/shared triangle boundaries, artifacts
  around cylinder caps, sharp edges, and any color shift between PNG and linear
  master. Retain original PNGs for visual review; a generated image alone
  is not evidence that a person examined it.
- For any perceived regression, record both renderer revision and request
  settings. Image SHA comparisons across unrelated devices/codec versions
  are not substitute for tolerance-based numeric tests.

### Cancellation and stale-frame publication

`JobHandle::frame_for(job, scene_generation)` and
`JobHandle::preview_for(job, scene_generation)` are explicit opt-in guards
that return `None` for obsolete job/scene identities. The caller **still**
has to compare the current scene revision **under its own publication lock**.
A provisional preview cannot be exported as a final frame. This patch does
not introduce another UI worker or duplicate the CPU renderer.

### Proof status and limits

An executable gallery and test coverage are **not** the same as having
executed or visually inspected the images. A PR only claims checked output
when its GitHub Actions run is complete with inspectable artifacts.
The numerical PBR is single-scattering GGX plus configured clear coat,
not a spectral or multiscattering full path tracer. Original geometry
and document serialization are unchanged. No external textures or
third-party copyrighted assets are bundled here.
