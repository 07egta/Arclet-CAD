//! OpenGL rendering of actual OCCT tessellations. No shape construction occurs here.
use super::{
    camera::{Camera, ViewKind},
    worker::Scene,
};
use crate::kernel::Mesh;
use eframe::{
    egui,
    glow::{self, HasContext},
};
use glam::DVec3;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Shaded,
    Edges,
    Wire,
    Zebra,
}
impl DisplayMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Shaded => "Shaded",
            Self::Edges => "Shaded + edges",
            Self::Wire => "Wireframe",
            Self::Zebra => "Zebra preview",
        }
    }
}
struct GpuMesh {
    source: Arc<Mesh>,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    line_vao: glow::VertexArray,
    line_vbo: glow::Buffer,
    indices: i32,
    lines: i32,
}
impl GpuMesh {
    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_vertex_array(self.line_vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ebo);
            gl.delete_buffer(self.line_vbo);
        }
    }
}
struct Allocation<'a> {
    gl: &'a glow::Context,
    buffers: Vec<glow::Buffer>,
    arrays: Vec<glow::VertexArray>,
}
impl<'a> Allocation<'a> {
    fn new(gl: &'a glow::Context) -> Self {
        Self {
            gl,
            buffers: vec![],
            arrays: vec![],
        }
    }
    fn buffer(&mut self) -> Result<glow::Buffer, String> {
        let b = unsafe { self.gl.create_buffer() }?;
        self.buffers.push(b);
        Ok(b)
    }
    fn array(&mut self) -> Result<glow::VertexArray, String> {
        let a = unsafe { self.gl.create_vertex_array() }?;
        self.arrays.push(a);
        Ok(a)
    }
    fn commit(&mut self) {
        self.buffers.clear();
        self.arrays.clear();
    }
}
impl Drop for Allocation<'_> {
    fn drop(&mut self) {
        unsafe {
            for b in &self.buffers {
                self.gl.delete_buffer(*b);
            }
            for a in &self.arrays {
                self.gl.delete_vertex_array(*a);
            }
        }
    }
}
fn bytes<T: bytemuck::Pod>(data: &[T]) -> &[u8] {
    // The trait bound rejects padded or uninitialized types at compile time.
    bytemuck::cast_slice(data)
}
fn upload(gl: &glow::Context, mesh: Arc<Mesh>, origin: DVec3) -> Result<GpuMesh, String> {
    let mut allocation = Allocation::new(gl);
    let vao = allocation.array()?;
    let vbo = allocation.buffer()?;
    let ebo = allocation.buffer()?;
    let line_vao = allocation.array()?;
    let line_vbo = allocation.buffer()?;
    let mut vertices = Vec::<f32>::with_capacity(mesh.positions.len() * 6);
    for (position, normal) in mesh.positions.iter().zip(&mesh.normals) {
        let p = DVec3::from_array(*position) - origin;
        vertices.extend([
            p.x as f32,
            p.y as f32,
            p.z as f32,
            normal[0] as f32,
            normal[1] as f32,
            normal[2] as f32,
        ]);
    }
    let mut lines = Vec::<f32>::with_capacity(mesh.lines.len() * 12);
    for line in &mesh.lines {
        for point in line {
            let p = DVec3::from_array(*point) - origin;
            lines.extend([p.x as f32, p.y as f32, p.z as f32, 0.0, 0.0, 1.0]);
        }
    }
    // SAFETY: the owning window's context is current throughout the paint callback.
    // Buffer data are copied by GL, and the declared vertex layout matches six f32 values.
    unsafe {
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&vertices), glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytes(&mesh.indices),
            glow::STATIC_DRAW,
        );
        gl.bind_vertex_array(Some(line_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(line_vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&lines), glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
        if gl.get_error() != glow::NO_ERROR {
            return Err("GPU buffer allocation/upload failed".into());
        }
    }
    let indices = i32::try_from(mesh.indices.len()).map_err(|_| "Too many GPU indices")?;
    let line_count = i32::try_from(mesh.lines.len() * 2).map_err(|_| "Too many GPU lines")?;
    allocation.commit();
    Ok(GpuMesh {
        source: mesh,
        vao,
        vbo,
        ebo,
        line_vao,
        line_vbo,
        indices,
        lines: line_count,
    })
}
const VERTEX: &str = r#"#version 330 core
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
uniform mat4 mvp;
out vec3 n;
out vec3 p;
void main(){p=position;n=normal;gl_Position=mvp*vec4(position,1.0);}
"#;
const FRAGMENT: &str = r#"#version 330 core
in vec3 n;
in vec3 p;
uniform vec3 color;
uniform vec3 eye;
uniform int mode;
out vec4 out_color;
void main(){
 if(mode==0){out_color=vec4(color,1.0);return;}
 vec3 normal=normalize(n);if(!gl_FrontFacing)normal=-normal;
 vec3 view=normalize(eye-p);
 float light=0.43+0.40*max(dot(normal,normalize(vec3(-0.35,-0.6,0.75))),0.0)+0.17*max(dot(normal,normalize(vec3(0.6,0.4,0.5))),0.0);
 float spec=pow(max(dot(reflect(-normalize(vec3(-0.35,-0.6,0.75)),normal),view),0.0),40.0)*0.12;
 vec3 base=color*light+vec3(spec);
 if(mode==2){vec3 r=reflect(-view,normal);float band=smoothstep(-0.06,0.06,sin((r.x+r.z*0.35)*36.0));base=mix(vec3(0.07),vec3(0.94),band);}
 out_color=vec4(base,1.0);
}
"#;
fn program(gl: &glow::Context) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        let result = (|| {
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, FRAGMENT),
            ] {
                let shader = gl.create_shader(kind)?;
                shaders.push(shader);
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            Ok(())
        })();
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        match result {
            Ok(()) => Ok(program),
            Err(e) => {
                gl.delete_program(program);
                Err(e)
            }
        }
    }
}
pub struct PaintOptions<'a> {
    pub selected: &'a [u64],
    pub display: DisplayMode,
    pub grid: bool,
}

pub struct Renderer {
    program: glow::Program,
    mvp: Option<glow::UniformLocation>,
    color: Option<glow::UniformLocation>,
    eye: Option<glow::UniformLocation>,
    mode: Option<glow::UniformLocation>,
    grid_vao: glow::VertexArray,
    grid_vbo: glow::Buffer,
    objects: BTreeMap<u64, GpuMesh>,
    origin: DVec3,
    pub error: Option<String>,
    destroyed: bool,
}
impl Renderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        let program = program(gl)?;
        let mut allocation = Allocation::new(gl);
        let grid = (|| {
            let vao = allocation.array()?;
            let vbo = allocation.buffer()?;
            Ok::<_, String>((vao, vbo))
        })();
        let (grid_vao, grid_vbo) = match grid {
            Ok(v) => v,
            Err(e) => {
                unsafe {
                    gl.delete_program(program);
                }
                return Err(e);
            }
        };
        let (mvp, color, eye, mode) = unsafe {
            (
                gl.get_uniform_location(program, "mvp"),
                gl.get_uniform_location(program, "color"),
                gl.get_uniform_location(program, "eye"),
                gl.get_uniform_location(program, "mode"),
            )
        };
        allocation.commit();
        Ok(Self {
            program,
            mvp,
            color,
            eye,
            mode,
            grid_vao,
            grid_vbo,
            objects: BTreeMap::new(),
            origin: DVec3::ZERO,
            error: None,
            destroyed: false,
        })
    }
    fn sync(&mut self, gl: &glow::Context, scene: &Scene) -> Result<(), String> {
        let origin = scene
            .bounds(&[])
            .map(|b| {
                DVec3::new(
                    (b[0] + b[3]) * 0.5,
                    (b[1] + b[4]) * 0.5,
                    (b[2] + b[5]) * 0.5,
                )
            })
            .unwrap_or(DVec3::ZERO);
        if self.origin != origin {
            for mesh in self.objects.values() {
                mesh.destroy(gl);
            }
            self.objects.clear();
            self.origin = origin;
        }
        let remove = self
            .objects
            .keys()
            .filter(|id| {
                !scene
                    .objects
                    .iter()
                    .any(|o| o.id == **id && o.visible && o.mesh.is_some())
            })
            .copied()
            .collect::<Vec<_>>();
        for id in remove {
            if let Some(mesh) = self.objects.remove(&id) {
                mesh.destroy(gl);
            }
        }
        for o in scene.objects.iter().filter(|o| o.visible) {
            if let Some(mesh) = &o.mesh {
                if self
                    .objects
                    .get(&o.id)
                    .is_some_and(|gpu| Arc::ptr_eq(&gpu.source, mesh))
                {
                    continue;
                }
                let gpu = upload(gl, mesh.clone(), origin)?;
                if let Some(old) = self.objects.insert(o.id, gpu) {
                    old.destroy(gl);
                }
            }
        }
        Ok(())
    }
    pub fn paint(
        &mut self,
        gl: &glow::Context,
        info: egui::PaintCallbackInfo,
        scene: &Scene,
        camera: &Camera,
        options: PaintOptions<'_>,
    ) {
        let PaintOptions {
            selected,
            display,
            grid,
        } = options;
        if self.destroyed {
            return;
        }
        let viewport = info.viewport_in_pixels();
        let clip = info.clip_rect_in_pixels();
        if viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }
        if let Err(e) = self.sync(gl, scene) {
            self.error = Some(e);
            return;
        }
        self.error = None;
        let aspect = viewport.width_px as f64 / viewport.height_px as f64;
        let matrix = camera.matrix(aspect, self.origin).as_mat4().to_cols_array();
        let eye = (camera.eye() - self.origin).as_vec3();
        unsafe {
            gl.viewport(
                viewport.left_px,
                viewport.from_bottom_px,
                viewport.width_px,
                viewport.height_px,
            );
            // The painter clip may cover the whole central panel. Intersect it
            // with this viewport or a later view's clear erases earlier views.
            let left = viewport.left_px.max(clip.left_px);
            let bottom = viewport.from_bottom_px.max(clip.from_bottom_px);
            let right = (viewport.left_px + viewport.width_px).min(clip.left_px + clip.width_px);
            let top = (viewport.from_bottom_px + viewport.height_px)
                .min(clip.from_bottom_px + clip.height_px);
            gl.scissor(left, bottom, (right - left).max(0), (top - bottom).max(0));
            gl.enable(glow::SCISSOR_TEST);
            gl.depth_mask(true);
            gl.clear_color(0.105, 0.125, 0.155, 1.0);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.mvp.as_ref(), false, &matrix);
            gl.uniform_3_f32(self.eye.as_ref(), eye.x, eye.y, eye.z);
        }
        if grid {
            self.grid(gl, camera);
        }
        if display != DisplayMode::Wire {
            unsafe {
                gl.enable(glow::POLYGON_OFFSET_FILL);
                gl.polygon_offset(1.0, 1.0);
                gl.uniform_1_i32(
                    self.mode.as_ref(),
                    if display == DisplayMode::Zebra { 2 } else { 1 },
                );
            }
            for object in scene.objects.iter().filter(|o| o.visible) {
                if let Some(gpu) = self.objects.get(&object.id) {
                    let color = if selected.contains(&object.id) {
                        [235, 178, 92]
                    } else {
                        object.color
                    };
                    unsafe {
                        gl.uniform_3_f32(
                            self.color.as_ref(),
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                        );
                        gl.bind_vertex_array(Some(gpu.vao));
                        gl.draw_elements(glow::TRIANGLES, gpu.indices, glow::UNSIGNED_INT, 0);
                    }
                }
            }
            unsafe {
                gl.disable(glow::POLYGON_OFFSET_FILL);
            }
        }
        unsafe {
            gl.uniform_1_i32(self.mode.as_ref(), 0);
            gl.line_width(1.0);
        }
        for object in scene.objects.iter().filter(|o| o.visible) {
            if let Some(gpu) = self.objects.get(&object.id) {
                let selected = selected.contains(&object.id);
                if display == DisplayMode::Shaded && !selected && gpu.indices > 0 {
                    continue;
                }
                if display == DisplayMode::Zebra && !selected && gpu.indices > 0 {
                    continue;
                }
                let color = if selected {
                    [1.0, 0.72, 0.3]
                } else if display == DisplayMode::Wire || gpu.indices == 0 {
                    [0.63, 0.78, 0.86]
                } else {
                    [0.075, 0.11, 0.14]
                };
                unsafe {
                    gl.uniform_3_f32(self.color.as_ref(), color[0], color[1], color[2]);
                    gl.bind_vertex_array(Some(gpu.line_vao));
                    gl.draw_arrays(glow::LINES, 0, gpu.lines);
                }
            }
        }
        // Egui restores its shader/viewport after callbacks; leave conventional depth and blend state.
        unsafe {
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.enable(glow::BLEND);
        }
    }
    fn grid(&self, gl: &glow::Context, camera: &Camera) {
        let step = 10.0_f64
            .powf((camera.span / 8.0).log10().floor())
            .clamp(1e-5, 1e9);
        let (u, v) = match camera.kind {
            ViewKind::Front => (DVec3::X, DVec3::Z),
            ViewKind::Right => (DVec3::Y, DVec3::Z),
            _ => (DVec3::X, DVec3::Y),
        };
        let cx = (camera.target.dot(u) / step).round() * step;
        let cy = (camera.target.dot(v) / step).round() * step;
        let radius = step * 30.0;
        let mut vertices = Vec::with_capacity(61 * 24);
        for i in -30..=30 {
            let d = i as f64 * step;
            for (a, b) in [
                (
                    u * (cx + d) + v * (cy - radius),
                    u * (cx + d) + v * (cy + radius),
                ),
                (
                    v * (cy + d) + u * (cx - radius),
                    v * (cy + d) + u * (cx + radius),
                ),
            ] {
                for p in [a, b] {
                    let p = (p - self.origin).as_vec3();
                    vertices.extend([p.x, p.y, p.z, 0.0, 0.0, 1.0]);
                }
            }
        }
        unsafe {
            gl.depth_mask(false);
            gl.uniform_1_i32(self.mode.as_ref(), 0);
            gl.uniform_3_f32(self.color.as_ref(), 0.18, 0.21, 0.25);
            gl.bind_vertex_array(Some(self.grid_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.grid_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&vertices), glow::STREAM_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
            gl.draw_arrays(glow::LINES, 0, (vertices.len() / 6) as i32);
            gl.depth_mask(true);
        }
    }
    pub fn destroy(&mut self, gl: &glow::Context) {
        if self.destroyed {
            return;
        }
        for mesh in self.objects.values() {
            mesh.destroy(gl);
        }
        self.objects.clear();
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.grid_vao);
            gl.delete_buffer(self.grid_vbo);
        }
        self.destroyed = true;
    }
}
