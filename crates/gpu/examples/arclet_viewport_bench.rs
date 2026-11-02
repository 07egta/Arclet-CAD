//! Reproducible *offscreen* viewport timings for Arclet CAD (not an interactive latency test).
//!
//! Run one process per size so VmHWM is attributable to that scene:
//!   WGPU_BACKEND=vulkan ARCLET_BENCH_POLICY=software cargo run --release --locked \
//!       -p solidcraft-gpu --example arclet_viewport_bench -- small
//!
//! The printed JSON is a wall-clock CPU+GPU-completion measurement. It is never a
//! hardware-GPU result unless the adapter report explicitly says "Hardware".

use std::sync::{Arc, Mutex};
use std::time::Instant;

use solidcraft_gpu::select::{Policy, Tier};
use solidcraft_gpu::view::{Frame, GpuScene, GpuTarget, Renderer, AO_SAMPLES};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const WARMUP: usize = 6;
const SAMPLES: usize = 32;

fn scene(triangles: usize) -> GpuScene {
    let mut scene = GpuScene::default();
    let color = scene.color([80, 170, 220, 255]);
    let n = (triangles as f64).sqrt().ceil() as usize;
    let step = 16.0 / n.max(1) as f32;
    for i in 0..triangles {
        let x = -8.0 + (i % n) as f32 * step;
        let y = -8.0 + (i / n) as f32 * step;
        // One call to tri adds one vertex; three calls make one triangle.
        for p in [[x, y, 0.0], [x + step * 0.8, y, 0.0], [x, y + step * 0.8, 0.0]] {
            scene.tri(p, [0.0, 0.0, 1.0], color);
        }
    }
    scene
}

fn frame(scene: GpuScene) -> Frame {
    let empty = || Arc::new(Mutex::new(None));
    Frame {
        product: false,
        key: 1,
        slot: Arc::new(Mutex::new(Some(scene))),
        sketch_key: 0,
        sketch_slot: empty(),
        hl_key: 0,
        hl_slot: empty(),
        pv_key: 0,
        pv_slot: empty(),
        pv_hide: false,
        view_proj: [[0.1, 0.0, 0.0, 0.0], [0.0, 0.1, 0.0, 0.0], [0.0, 0.0, -0.01, 0.0], [0.0, 0.0, 0.5, 1.0]],
        back: [0.0, 0.0, 1.0],
        size_px: [WIDTH as f32, HEIGHT as f32],
        clip: [0.0; 4],
        cap: [0.0; 4],
        images: Vec::new(),
        points: Vec::new(),
        analysis: [0.0; 8],
        analysis_colors: [0.0; 36],
        access: None,
        outline: false,
        eye: [0.0, 0.0, 100.0, 0.0],
        ao_radius: 0.0,
        ao_samples: AO_SAMPLES,
        ssaa: 1.0,
        hover_tint: [0.0; 3],
        selection_tint: [0.0; 3],
    }
}

/// One complete frame, including submission and GPU completion (not just command encoding).
fn draw(
    renderer: &mut Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frame: &Frame,
    color_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
) -> Result<(), String> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("arclet benchmark frame") });
    renderer.prepare(device, queue, &mut encoder, frame);
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("arclet benchmark viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(solidcraft_gpu::view::far_depth(solidcraft_gpu::view::reversed_depth(Some(
                            wgpu::TextureFormat::Depth32Float,
                        )))),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        renderer.paint(frame, &mut pass, [0.0, 0.0, WIDTH as f32, HEIGHT as f32]);
    }
    queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("GPU completion failed: {e:?}"))?;
    Ok(())
}

fn texture(device: &wgpu::Device, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("arclet benchmark target"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn peak_rss_kib() -> Option<u64> {
    // Linux: peak resident-set memory for this process (not GPU VRAM).
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| line.strip_prefix("VmHWM:")?.split_whitespace().next()?.parse().ok())
}

fn percentile(sorted: &[f64], percentile: usize) -> f64 {
    // Nearest rank (at least one element; SAMPLES > 0).
    let idx = (sorted.len() * percentile).div_ceil(100).saturating_sub(1);
    sorted.get(idx).copied().unwrap_or(0.0)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();
    let case = std::env::args().nth(1).unwrap_or_else(|| "small".to_owned());
    let triangles = match case.as_str() {
        "small" => 256,
        "medium" => 4096,
        "large" => 32768,
        _ => return Err(format!("unknown case {case}; use small, medium or large").into()),
    };
    let policy_name = std::env::var("ARCLET_BENCH_POLICY").unwrap_or_else(|_| "software".to_owned());
    let policy = Policy::parse(&policy_name).ok_or("ARCLET_BENCH_POLICY must be hardware or software")?;
    if !matches!(policy, Policy::Hardware | Policy::Software) {
        return Err("benchmark policy must explicitly be hardware or software".into());
    }
    let (adapter, report) = solidcraft_gpu::select::pick(policy).ok_or_else(|| format!("no {policy_name} adapter; cannot measure this case"))?;
    if (policy == Policy::Software) != (report.tier == Tier::Software) {
        return Err(format!("selected adapter tier mismatch: {:?}", report.tier).into());
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("arclet viewport benchmark"),
        required_features: solidcraft_gpu::depth_features(&adapter),
        required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))?;
    let device_init_ms = begin.elapsed().as_secs_f64() * 1e3;

    let scene_begin = Instant::now();
    let scene = scene(triangles);
    let mut frame = frame(scene);
    let scene_build_ms = scene_begin.elapsed().as_secs_f64() * 1e3;

    let pipeline_begin = Instant::now();
    let depth = wgpu::TextureFormat::Depth32Float;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = Renderer::new(&device, GpuTarget { format, depth: Some(depth), samples: 1, tess: false, oit: false }, None);
    let pipeline_create_ms = pipeline_begin.elapsed().as_secs_f64() * 1e3;

    let color = texture(&device, format, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let depth_texture = texture(&device, depth, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let (color_view, depth_view) = (color.create_view(&Default::default()), depth_texture.create_view(&Default::default()));

    draw(&mut renderer, &device, &queue, &frame, &color_view, &depth_view)?;
    let first_draw_ms = begin.elapsed().as_secs_f64() * 1e3;

    let mut timings = Vec::with_capacity(SAMPLES);
    for i in 0..WARMUP + SAMPLES {
        // Move the camera each time so the renderer cannot blit a still-frame cache.
        frame.view_proj[3][0] = (i as f32 * 0.071).sin() * 0.015 + 0.03;
        let start = Instant::now();
        draw(&mut renderer, &device, &queue, &frame, &color_view, &depth_view)?;
        if i >= WARMUP {
            timings.push(start.elapsed().as_secs_f64() * 1e3);
        }
    }
    timings.sort_by(f64::total_cmp);
    let out = serde_json::json!({
        "case": case,
        "triangles": triangles,
        "resolution": [WIDTH, HEIGHT],
        "warmup_frames": WARMUP,
        "sample_frames": SAMPLES,
        "adapter": report.name,
        "backend": report.backend,
        "device_type": report.device_type,
        "tier": format!("{:?}", report.tier),
        "policy": policy.name(),
        "device_init_ms": device_init_ms,
        "scene_build_ms": scene_build_ms,
        "pipeline_create_ms": pipeline_create_ms,
        "first_draw_ms": first_draw_ms,
        "frame_p50_ms": percentile(&timings, 50),
        "frame_p95_ms": percentile(&timings, 95),
        "peak_rss_kib": peak_rss_kib(),
        "source": "wgpu offscreen; wait for GPU completion; not OS-input latency"
    });
    println!("{out}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("arclet viewport benchmark failed: {error}");
        std::process::exit(1);
    }
}
