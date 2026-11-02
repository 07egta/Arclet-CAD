# Arclet CAD viewport performance: reproducible measurement and conservative changes

**Scope:** `crates/gpu` offscreen wgpu viewport and its `Frame`, not the Arclet window compositor. This document distinguishes observations from hypotheses; a numerical speedup is **not** implied just because a cache was added.

## Measurement contract

- Workload: deterministic orthographic grid at **640 × 360**, 256 (`small`), 4,096 (`medium`), or 32,768 (`large`) triangles, emitted by `crates/gpu/examples/arclet_viewport_bench.rs`.
- Build/run: `cargo build --release --locked -p solidcraft-gpu --example arclet_viewport_bench`; one separate process per case, 6 warm-up frames, 32 measured frames. Camera matrix changes every frame, so it does not just replay the still-picture cache. Each frame includes command recording, submission, and `wgpu::Device::poll(wait_indefinitely)` to wait for GPU completion.
- Values: `frame_p50_ms` and `frame_p95_ms` are nearest-rank per-frame wall-clock intervals; `first_draw_ms` spans process benchmark entry through adapter/device initialization, scene construction, pipeline setup, and the first offscreen GPU-completed frame. It is **not** the time an OS or browser window first paints. `peak_rss_kib` is the Linux process's `/proc/self/status` `VmHWM`, **not** peak dedicated GPU memory.
- Explicit adapter: `ARCLET_BENCH_POLICY=software|hardware`. The result contains actual adapter `name`, `backend`, `device_type`, and `tier`. Missing or unsuitable adapters fail instead of silently attributing software performance to hardware.
- The CI workflow `.github/workflows/arclet-viewport-bench.yml` specifies the lavapipe Vulkan ICD and uploads `small.json`, `medium.json`, `large.json` as artifacts.
- A single run is noisy: shader-cache state, competing runner jobs, OS scheduler and CPU frequency vary. Repeat benchmarks on the **same** machine, adapter and driver, ideally several runs, before claiming a significant change.

## Pre-change baseline — actual CI measurement

- Revision: [`ba6b76c1717e706d869d17cc27fa03d96f911612`](https://github.com/07egta/Arclet-CAD/commit/ba6b76c1717e706d869d17cc27fa03d96f911612); it introduced the reproducible probe and corrected the reversed-depth clear **before** renderer changes.
- [Successful Actions run 41402483216](https://github.com/07egta/Arclet-CAD/actions/runs/41402483216) and its `arclet-software-vulkan-metrics` artifact.
- Adapter: **llvmpipe (LLVM 20.1.2, 256 bits)**, `device_type=Cpu`, `tier=Software`, `backend=Vulkan` on GitHub-hosted Ubuntu Linux with Mesa lavapipe. **No physical GPU acceleration was measured.**

| Workload | p50 frame (ms) | p95 frame (ms) | First offscreen completion (ms) | Peak process RSS (KiB) |
|---|---:|---:|---:|---:|
| small (256 triangles) | 0.938606 | 1.428238 | 616.910135 | 144348 |
| medium (4,096 triangles) | 1.577368 | 1.726168 | 110.096627 | 138204 |
| large (32,768 triangles) | 9.118158 | 9.708319 | 121.005708 | 152060 |

The first case was notably slower on first draw than later cases; the suite can benefit from warm caches after the first process. Do not interpret this as a size-dependent speed difference or a cold-start benchmark across three independent environments.

## Implementation changes (after baseline)

1. **Reference-image vertex buffers:** retain a GPU quad buffer across camera redraws when the four corners and opacity have identical floating-point bit patterns. Pixel-version changes replace texture content, but do not regenerate static quad vertex geometry. If geometry/opacity changes, rebuild only that quad. Current `Frame.images` order is preserved for blending. Unit test covers all invalidation cases.
2. **Instanced LOD:** choose the coarsest acceptable level directly from the existing `LevelDraw` iterator, instead of allocating a temporary `Vec<f32>` error table for each instanced mesh on every moving frame. Existing `LOD_PX` threshold and public `level_for` API are retained. Unit test checks equivalent level choices for finite and non-finite scales.
3. **Measurement harness and CI:** fixed-size synthetic scenes; JSON with process and adapter information. A target test step checks the reference-image regression after benchmark cases.

These changes do **not** imply a measurable gain in the three baseline scenes: they contain no reference images or instanced meshes. A separate matched scene with those features is required to quantify those particular optimizations. CPU overhead, RSS, and offscreen frame timings from the generic scene can still identify unrelated regressions.

## Post-change results

Populate only from a successfully completed run on the optimized revision with matching backend, adapter, and workload; never substitute guesses. Compare using:

```bash
python3 scripts/arclet_bench_compare.py baseline/software optimized/software
```

The comparison script rejects mismatched adapters, policies, sizes, sample counts and other workload metadata. Repeated experiments and real Intel hardware must be recorded separately from Mesa software numbers.

## Backend support inventory (code paths, not a hardware certification)

| Platform | Backends configured | Verification status |
|---|---|---|
| Windows native headless | Direct3D 12 and Vulkan (`crates/gpu/src/select.rs`) | Declared in code; no real Intel GPU device observed by this benchmark |
| Linux native headless | Vulkan and GL | Mesa Vulkan software GPU **measured**; hardware driver not measured |
| macOS native headless | Metal | Declared in code; not measured |
| Arclet desktop (`apps/arclet/src/main.rs`) | eframe + wgpu with usual native backend selection and `WGPU_BACKEND` override | OS window startup, presentation and device-loss recovery **not measured** |
| Arclet browser | WebGPU by default; explicit `?webgl` requests GL/WebGL2 | Paths configured; neither browser backend was measured |

`select::Policy::Auto` ranks discrete before integrated GPUs and before CPU devices; thus an Intel iGPU is preferred to Mesa software when both are available. Driver/device initialization failure and automatic failover on real Intel hardware require separate testing. `WGPU_BACKEND` allows an explicit backend override; using that is not equivalent to proven automatic fallback.

## Gaps before declaring this mission fully complete

- Actual Arclet desktop first **visible window** time and input-event-to-paint latency; camera/point dragging p95 on a real GPU.
- Repeated matched image-rich and instancing-heavy scenes to substantiate optimization speedups.
- Hardware GPU measurements on an actual Intel integrated-GPU machine, Vulkan vs DX12 under the same project and driver.
- VRAM allocations, real large B-rep/model retessellation, and emergency recovery from device loss.
- Full `cargo xtask ci`, desktop input/visual checks and WebGPU/WebGL2 regression tests. Do not mark these complete merely because an offscreen software Vulkan probe passes.