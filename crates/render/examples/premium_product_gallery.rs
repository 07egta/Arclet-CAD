//! Deterministic ProductScene evidence from the real built-in CPU raster/PBR path.
//! This is deliberately NOT a path-traced glass or global illumination demo.
//! Writes six actual renderer PNGs and numerical receipts to a new directory.
//! Usage: cargo run -p solidcraft-render --example premium_product_gallery -- NEW_DIR
use serde_json::json;
use sha2::{Digest, Sha256};
use solidcraft_geom::{Aabb3, Mesh, Vec3, srgb::encode8};
use solidcraft_render::job::{CpuHost, ImageRequest, Integrator, JobId, Phase};
use solidcraft_render::{Camera, ProductMesh, ProductScene, StandardView, surface::{OpaqueFinish, OpaqueSurface}};
use std::{error::Error, fs, path::PathBuf, sync::Arc, time::Instant};

const SIDE: u32 = 160;
const BACKGROUND: [u8; 3] = [232, 236, 242];

fn quad(mesh: &mut Mesh, points: [Vec3; 4], normal: Vec3) {
    let start = mesh.positions.len() as u32;
    mesh.positions.extend(points);
    mesh.normals.extend([normal; 4]);
    mesh.triangles.extend([[start, start + 1, start + 2], [start, start + 2, start + 3]]);
    mesh.tri_face.extend([0, 0]);
}

fn block(center: Vec3, half: Vec3) -> Mesh {
    let mut mesh = Mesh::default();
    // u × v = outward normal. Each facet owns distinct vertices to keep hard
    // geometric normals: no spurious smoothing across a product's sharp edges.
    for (normal, u, v) in [
        (Vec3::X, Vec3::Y, Vec3::Z), (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X), (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y), (-Vec3::Z, Vec3::Y, Vec3::X),
    ] {
        let extent = |axis: Vec3| axis.x.abs() * half.x + axis.y.abs() * half.y + axis.z.abs() * half.z;
        let c = center + normal * extent(normal);
        let u = u * extent(u);
        let v = v * extent(v);
        quad(&mut mesh, [c - u - v, c + u - v, c + u + v, c - u + v], normal);
    }
    mesh
}

fn sphere(center: Vec3, radius: f64) -> Mesh {
    const LAT: u32 = 20;
    const LON: u32 = 32;
    let mut mesh = Mesh::default();
    for i in 0..=LAT {
        let theta = std::f64::consts::PI * f64::from(i) / f64::from(LAT);
        for j in 0..=LON {
            let phi = std::f64::consts::TAU * f64::from(j) / f64::from(LON);
            let n = Vec3::new(theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos());
            mesh.positions.push(center + n * radius);
            mesh.normals.push(n);
        }
    }
    for i in 0..LAT {
        for j in 0..LON {
            let a = i * (LON + 1) + j;
            let b = a + 1;
            let c = a + LON + 1;
            let d = c + 1;
            mesh.triangles.extend([[a, c, b], [b, c, d]]);
            mesh.tri_face.extend([0, 0]);
        }
    }
    mesh
}

fn cylinder_shell(radius: f64, height: f64, thickness: f64) -> Mesh {
    const SIDES: u32 = 48;
    let mut mesh = Mesh::default();
    let inner = radius - thickness;
    for j in 0..SIDES {
        let angle_a = std::f64::consts::TAU * f64::from(j) / f64::from(SIDES);
        let angle_b = std::f64::consts::TAU * f64::from(j + 1) / f64::from(SIDES);
        let dir = |t: f64| Vec3::new(t.cos(), t.sin(), 0.0);
        let a = dir(angle_a);
        let b = dir(angle_b);
        let n = dir((angle_a + angle_b) * 0.5);
        let low = |r: f64, d: Vec3| d * r;
        let high = |r: f64, d: Vec3| d * r + Vec3::Z * height;
        quad(&mut mesh, [low(radius, a), low(radius, b), high(radius, b), high(radius, a)], n);
        quad(&mut mesh, [low(inner, a), high(inner, a), high(inner, b), low(inner, b)], -n);
        quad(&mut mesh, [high(radius, a), high(radius, b), high(inner, b), high(inner, a)], Vec3::Z);
        quad(&mut mesh, [low(radius, a), low(inner, a), low(inner, b), low(radius, b)], -Vec3::Z);
    }
    mesh
}

fn opaque(name: &str, geometry: Mesh, finish: OpaqueFinish) -> Result<ProductMesh, Box<dyn Error>> {
    let surface = finish.surface()?;
    let color = [surface.base_color().x, surface.base_color().y, surface.base_color().z].map(encode8);
    Ok(ProductMesh { name: name.into(), mesh: Arc::new(geometry), color, opacity: 1.0, surface: Some(surface), transmission: None, faces: Vec::new() })
}

fn custom(name: &str, geometry: Mesh, linear: Vec3, metallic: f64, roughness: f64) -> Result<ProductMesh, Box<dyn Error>> {
    let surface = OpaqueSurface::new(linear, metallic, roughness, 1.5)?;
    let color = [linear.x, linear.y, linear.z].map(encode8);
    Ok(ProductMesh { name: name.into(), mesh: Arc::new(geometry), color, opacity: 1.0, surface: Some(surface), transmission: None, faces: Vec::new() })
}

fn group(case: &str) -> Result<ProductScene, Box<dyn Error>> {
    let mut meshes: Vec<ProductMesh> = Vec::new();
    match case {
        "metal-chair" => {
            let metal = OpaqueFinish::SatinAluminium;
            meshes.push(opaque("metal seat", block(Vec3::new(0.0, 0.0, 12.0), Vec3::new(8.0, 6.0, 0.8)), metal)?);
            meshes.push(opaque("metal back", block(Vec3::new(0.0, 5.0, 23.0), Vec3::new(7.2, 0.8, 9.0)), metal)?);
            for x in [-6.5, 6.5] {
                for y in [-4.5, 4.5] {
                    meshes.push(opaque("chair leg", block(Vec3::new(x, y, 6.0), Vec3::new(0.6, 0.6, 6.0)), metal)?);
                }
            }
        }
        "transparent-container-alpha-preview" => {
            // Explicit scalar-alpha preview, NOT physical refraction. The exact
            // CPU dielectric ray tracer has separate optical and visual tests.
            meshes.push(opaque("inner matte plastic insert", block(Vec3::new(0.0, 0.0, 4.0), Vec3::new(4.0, 4.0, 1.2)), OpaqueFinish::MattePlastic)?);
            let glass = ProductMesh {
                name: "cylindrical transparent alpha shell".into(),
                mesh: Arc::new(cylinder_shell(7.0, 17.0, 0.6)),
                color: [130, 185, 215], opacity: 0.28, surface: None,
                transmission: None, faces: Vec::new(),
            };
            meshes.push(glass);
        }
        "matte-plastic-case" => {
            meshes.push(opaque("case body", block(Vec3::new(0.0, 0.0, 6.0), Vec3::new(12.0, 8.0, 6.0)), OpaqueFinish::MattePlastic)?);
            meshes.push(opaque("lid", block(Vec3::new(0.0, 0.0, 12.7), Vec3::new(12.4, 8.4, 0.5)), OpaqueFinish::GlossyPlastic)?);
        }
        "untextured-wood-furniture" => {
            let wood = OpaqueFinish::VarnishedWood;
            meshes.push(opaque("wood table top", block(Vec3::new(0.0, 0.0, 16.0), Vec3::new(15.0, 9.0, 1.2)), wood)?);
            for x in [-12.0, 12.0] {
                for y in [-6.0, 6.0] {
                    meshes.push(opaque("wood table leg", block(Vec3::new(x, y, 7.5), Vec3::new(1.1, 1.1, 7.5)), OpaqueFinish::UntexturedWood)?);
                }
            }
        }
        "roughness-spheres" => {
            for (i, roughness) in [0.09, 0.24, 0.48, 0.9].into_iter().enumerate() {
                let x = (i as f64 - 1.5) * 12.0;
                meshes.push(custom(&format!("same-metal-roughness-{roughness}"), sphere(Vec3::new(x, 0.0, 8.0), 5.0),
                    Vec3::new(0.60, 0.66, 0.73), 1.0, roughness)?);
            }
        }
        "multi-material-product" => {
            meshes.push(opaque("ceramic lamp base", block(Vec3::new(0.0, 0.0, 1.5), Vec3::new(7.0, 7.0, 1.5)), OpaqueFinish::GlazedCeramic)?);
            meshes.push(opaque("metal lamp stem", block(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.8, 0.8, 8.0)), OpaqueFinish::PolishedMetal)?);
            meshes.push(opaque("matte shade", block(Vec3::new(0.0, 0.0, 19.0), Vec3::new(9.0, 7.0, 3.0)), OpaqueFinish::MattePlastic)?);
            meshes.push(opaque("satin lamp switch", sphere(Vec3::new(4.5, -7.1, 2.5), 1.1), OpaqueFinish::SatinAluminium)?);
        }
        _ => return Err(format!("unknown gallery case: {case}").into()),
    }
    for body in &meshes {
        // Reject geometric-normal orientation regressions in the generated actual
        // mesh rather than accepting impossible black facets as material behavior.
        for indices in &body.mesh.triangles {
            let [Some(a), Some(b), Some(c)] = indices.map(|index| body.mesh.positions.get(index as usize)) else {
                return Err(format!("invalid fixture mesh indices for {}", body.name).into());
            };
            let cross = (*b - *a).cross(*c - *a);
            if cross.len() > 1e-11 {
                let normal = body.mesh.normals.get(indices[0] as usize).ok_or("missing fixture normal")?;
                if cross.dot(*normal) <= 0.0 {
                    return Err(format!("inverted fixture triangle normal: {}", body.name).into());
                }
            }
        }
    }
    let bounds = meshes.iter().fold(Aabb3::EMPTY, |bounds, b| bounds.union(&b.mesh.bounds()));
    Ok(ProductScene { meshes, bounds })
}

fn main() -> Result<(), Box<dyn Error>> {
    let folder = std::env::args_os().nth(1).map(PathBuf::from).ok_or("usage: premium_product_gallery NEW_OUTPUT_DIRECTORY")?;
    fs::create_dir(&folder)?; // Refuse to overwrite an existing evidence directory.
    let host = CpuHost::cooperative();
    let cases = [
        "metal-chair", "transparent-container-alpha-preview", "matte-plastic-case",
        "untextured-wood-furniture", "roughness-spheres", "multi-material-product",
    ];
    let mut receipts = Vec::new();
    for (number, name) in cases.into_iter().enumerate() {
        let scene = group(name)?;
        let triangles: usize = scene.meshes.iter().map(|body| body.mesh.triangles.len()).sum();
        let mut camera = Camera { fov: 0.0, ..Default::default() };
        camera.set_view(StandardView::Iso);
        camera.fit(&scene.bounds);
        let request = ImageRequest {
            environment: None, textures: None, integrator: Integrator::Raster, raytrace: None,
            camera, width: SIDE, height: SIDE, background: BACKGROUND, transparent_background: false,
        };
        let start = Instant::now();
        let handle = host.submit(JobId { session: 20261102, serial: number as u64 + 1, generation: 1 }, 1, scene, request)?;
        let mut complete = false;
        for _ in 0..200_000 {
            let ongoing = host.pump(16_384)?;
            let status = handle.status()?;
            if status.progress.phase == Phase::Completed {
                complete = true;
                break;
            }
            if status.progress.phase.terminal() || !ongoing {
                return Err(format!("{name}: render ended without a complete frame: {:?}", status.error).into());
            }
        }
        if !complete {
            return Err(format!("{name}: cooperative renderer exceeded the bounded step count").into());
        }
        let render_ms = start.elapsed().as_millis();
        let frame = handle.frame()?.ok_or("completed job missing its frame")?;
        let png = frame.png();
        let image = image::load_from_memory(png)?.to_rgba8();
        if image.dimensions() != (SIDE, SIDE) {
            return Err("renderer output dimensions do not match the camera request".into());
        }
        // Quantitative pixel evidence, computed from the generated PNG, not a
        // mock-up: detect black or clipped subject pixels while retaining the
        // raw image for actual visual review.
        let mut subject = 0u64;
        let mut clipped_white = 0u64;
        let mut crushed_black = 0u64;
        let mut linear_luminance = 0.0_f64;
        for pixel in image.pixels() {
            let [r, g, b, alpha] = pixel.0;
            if alpha != 255 {
                return Err(format!("{name}: opaque-gallery PNG has an unexpected alpha byte").into());
            }
            if [r, g, b] == BACKGROUND {
                continue;
            }
            subject += 1;
            clipped_white += u64::from([r, g, b].into_iter().all(|c| c >= 253));
            crushed_black += u64::from([r, g, b].into_iter().all(|c| c <= 2));
            let decode = solidcraft_geom::srgb::decode8;
            linear_luminance += 0.2126 * decode(r) + 0.7152 * decode(g) + 0.0722 * decode(b);
        }
        if subject < 50 {
            return Err(format!("{name}: output did not visibly contain the real mesh fixture").into());
        }
        let png_sha256 = format!("{:x}", Sha256::digest(png));
        fs::write(folder.join(format!("{name}.png")), png)?;
        let average_subject_linear_luminance = linear_luminance / subject as f64;
        receipts.push(json!({
            "case": name, "method": "builtin_cpu_raster", "physically_refractive_glass": false,
            "width": SIDE, "height": SIDE, "triangles": triangles,
            "subject_pixels": subject, "all_channels_near_white": clipped_white,
            "all_channels_near_black": crushed_black,
            "mean_subject_linear_luminance": average_subject_linear_luminance,
            "png_sha256": png_sha256, "measured_render_ms": render_ms,
        }));
        drop(frame);
        drop(handle);
    }
    let report = json!({
        "generator": "Arclet CAD built-in CPU raster, original deterministic ProductScene geometries",
        "scenes": receipts,
        "feature_honesty": "alpha vessel is a preview, not a refractive glass transport test; wood is not procedurally grained; no GI, cast shadows, or path-traced reflections are claimed",
        "measurements": "wall-clock run measurements refer to this host only, not an optimized GPU performance benchmark",
        "visual_inspection": "not automatic: review original PNG artifacts to detect aesthetic and normal anomalies",
    });
    fs::write(folder.join("receipt.json"), serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
