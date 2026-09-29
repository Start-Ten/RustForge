//! RustForge 渲染（IF-180 ~ IF-191）：相机、2D 批次渲染、3D 软件管线、
//! 渲染图、LayerStack、后处理、上采样、Gizmo。

pub mod post;

pub use post::*;
use rf_core::{Result, Rgba8Image};
use rf_math::Color;
use rf_math::{Mat2x3, Mat4, Vec2, Vec3};
use rf_rhi::RhiDevice;

// ---- 相机（IF-180 / IF-181） ----

/// 2D 正交相机。
#[derive(Debug, Clone, Copy)]
pub struct Camera2D {
    pub position: Vec2,
    pub rotation: f32,
    /// 缩放：每世界单位的像素数（越大越放大）。
    pub zoom: f32,
    /// 视口像素尺寸。
    pub viewport: (f32, f32),
    /// Y 轴向上（true，数学惯例）或向下（false，屏幕惯例）。
    pub y_up: bool,
    /// 像素完美：世界坐标取整采样。
    pub pixel_perfect: bool,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            viewport: (800.0, 600.0),
            y_up: false,
            pixel_perfect: true,
        }
    }
}

impl Camera2D {
    /// 世界 → 屏幕（像素，左上原点）。
    pub fn world_to_screen(&self, p: Vec2) -> Vec2 {
        let d = p - self.position;
        let rot = rf_math::Rot2::from_angle(-self.rotation).rotate(d);
        let s = self.zoom;
        Vec2::new(rot.x * s + self.viewport.0 * 0.5, rot.y * s + self.viewport.1 * 0.5)
    }

    /// 屏幕 → 世界。
    pub fn screen_to_world(&self, p: Vec2) -> Vec2 {
        let d = Vec2::new(
            (p.x - self.viewport.0 * 0.5) / self.zoom,
            (p.y - self.viewport.1 * 0.5) / self.zoom,
        );
        self.position + rf_math::Rot2::from_angle(self.rotation).rotate(d)
    }

    /// 世界 → 像素完美的屏幕坐标（半像素偏移修正）。
    pub fn snap(&self, p: Vec2) -> Vec2 {
        let s = self.world_to_screen(p);
        if self.pixel_perfect {
            Vec2::new(s.x.round(), s.y.round())
        } else {
            s
        }
    }

    /// 视口逆变换（渲染器使用）。
    pub fn view_matrix(&self) -> Mat2x3 {
        Mat2x3::from_translation(Vec2::new(self.viewport.0 * 0.5, self.viewport.1 * 0.5))
            .mul_mat2x3(Mat2x3::from_scale(Vec2::splat(self.zoom)))
            .mul_mat2x3(Mat2x3::from_translation(-self.position))
    }
}

/// 3D 相机（透视/正交）。
#[derive(Debug, Clone, Copy)]
pub struct Camera3D {
    pub position: Vec3,
    /// 偏航（绕 Y，弧度）。
    pub yaw: f32,
    /// 俯仰（绕 X，弧度）。
    pub pitch: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub ortho: bool,
    pub ortho_size: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            fov_y: 1.2,
            near: 0.1,
            far: 100.0,
            ortho: false,
            ortho_size: 5.0,
        }
    }
}

impl Camera3D {
    pub fn forward(&self) -> Vec3 {
        let (cp, sp) = self.pitch.sin_cos();
        let (cy, sy) = self.yaw.sin_cos();
        Vec3::new(cy * cp, sp, -sy * cp).normalized()
    }

    pub fn right(&self) -> Vec3 {
        let (cy, _) = self.yaw.sin_cos();
        let (_, sy) = self.yaw.sin_cos();
        Vec3::new(cy, 0.0, -sy).normalized()
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.position + self.forward(), Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        if self.ortho {
            let h = self.ortho_size;
            let w = h * aspect;
            Mat4::orthographic(-w, w, -h, h, self.near, self.far)
        } else {
            Mat4::perspective(self.fov_y, aspect, self.near, self.far)
        }
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect).mul_mat4(self.view_matrix())
    }
}

// ---- 2D 渲染（IF-182 / IF-183） ----

/// 精灵实例。
#[derive(Debug, Clone, Copy)]
pub struct SpriteInstance {
    /// 2D 仿射变换（世界 → 屏幕）。
    pub transform: Mat2x3,
    /// uv 矩形 [u0, v0, u1, v1]。
    pub uv: [f32; 4],
    pub color: Color,
    /// 排序键（图层×级别 + 序）。
    pub sort_key: u32,
}

/// 批次数据。
#[derive(Debug, Clone, Default)]
pub struct SpriteBatchData {
    pub texture: Option<u64>,
    pub instances: Vec<SpriteInstance>,
}

/// 2D 绘制统计。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DrawStats2D {
    pub draw_calls: u32,
    pub sprites: u32,
    pub texture_switches: u32,
    pub batches_merged: u32,
}

/// 按 (sort_key, texture) 稳定排序并合并批次（IF-182）。
pub fn build_batches(batches: &mut Vec<SpriteBatchData>) -> Vec<SpriteBatchData> {
    // 展平
    let mut all: Vec<(u32, Option<u64>, SpriteInstance)> = Vec::new();
    for b in batches.drain(..) {
        for inst in b.instances {
            all.push((inst.sort_key, b.texture, inst));
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<SpriteBatchData> = Vec::new();
    let mut merged = 0u32;
    for (key, tex, inst) in all {
        match out.last_mut() {
            Some(b) if b.texture == tex && b.instances.last().map(|i| i.sort_key) == Some(key) => {
                b.instances.push(inst)
            }
            _ => {
                merged += 1;
                out.push(SpriteBatchData { texture: tex, instances: vec![inst] });
            }
        }
    }
    // 修正 merged 计数（初始 0，每个新批次 +1 → 批次数）
    let _ = merged;
    out
}

/// 2D 渲染器 trait（IF-183）。
pub trait Renderer2D {
    fn begin_frame(&mut self, target: &mut Rgba8Image, camera: &Camera2D);
    /// uv: [u0,v0,u1,v1]；transform：世界 → 屏幕仿射。
    fn draw_sprite(
        &mut self,
        tex: Option<&Rgba8Image>,
        uv: &[f32; 4],
        color: Color,
        layer: i32,
        transform: &Mat2x3,
    );
    fn flush(&mut self) -> DrawStats2D;
}

/// Software 2D 渲染器：CPU 位块传输 + Alpha 混合 + 批次统计。
pub struct SoftwareRenderer2D {
    target: Option<Box<Rgba8Image>>,
    camera: Camera2D,
    stats: DrawStats2D,
    last_texture: Option<u64>,
}

impl Default for SoftwareRenderer2D {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareRenderer2D {
    pub fn new() -> Self {
        Self {
            target: None,
            camera: Camera2D::default(),
            stats: DrawStats2D::default(),
            last_texture: None,
        }
    }
}

impl Renderer2D for SoftwareRenderer2D {
    fn begin_frame(&mut self, target: &mut Rgba8Image, camera: &Camera2D) {
        // 借用整个帧：swap 出来渲染，flush 后 take_target 归还
        let swapped = std::mem::replace(target, Rgba8Image::new(1, 1));
        self.target = Some(Box::new(swapped));
        self.camera = *camera;
        self.stats = DrawStats2D::default();
        self.last_texture = None;
    }

    fn draw_sprite(
        &mut self,
        tex: Option<&Rgba8Image>,
        uv: &[f32; 4],
        color: Color,
        _layer: i32,
        transform: &Mat2x3,
    ) {
        let Some(target) = self.target.as_mut() else { return };
        let (sw, sh) = match tex {
            Some(t) => (t.width as f32, t.height as f32),
            None => (1.0, 1.0),
        };
        // 采样矩形像素范围
        let x0 = uv[0] * sw;
        let y0 = uv[1] * sh;
        let x1 = uv[2] * sw;
        let y1 = uv[3] * sh;
        let (pw, ph) = ((x1 - x0).abs().ceil() as u32, (y1 - y0).abs().ceil() as u32);
        self.stats.sprites += 1;
        if self.last_texture != tex.map(|t| t.data.as_ptr() as u64) {
            self.stats.texture_switches += 1;
            self.last_texture = tex.map(|t| t.data.as_ptr() as u64);
        }
        self.stats.draw_calls += 1;
        let tint = color.to_rgba8();
        // 屏幕空间四角（local → world → camera → 屏幕）
        let half = Vec2::new(pw as f32 * 0.5, ph as f32 * 0.5);
        let mut corners = Vec::with_capacity(4);
        for (lx, ly) in [(-half.x, -half.y), (half.x, -half.y), (half.x, half.y), (-half.x, half.y)]
        {
            let world = transform.transform_point(Vec2::new(lx, ly));
            let mut sc = self.camera.world_to_screen(world);
            if !self.camera.y_up {
                sc.y = self.camera.viewport.1 - sc.y;
            }
            corners.push(sc);
        }
        let minx = corners.iter().map(|c| c.x.floor()).fold(f32::INFINITY, f32::min).max(0.0);
        let miny = corners.iter().map(|c| c.y.floor()).fold(f32::INFINITY, f32::min).max(0.0);
        let maxx = corners
            .iter()
            .map(|c| c.x.ceil())
            .fold(f32::NEG_INFINITY, f32::max)
            .min(target.width as f32 - 1.0);
        let maxy = corners
            .iter()
            .map(|c| c.y.ceil())
            .fold(f32::NEG_INFINITY, f32::max)
            .min(target.height as f32 - 1.0);
        let inv_transform = transform.inverse();
        for sy in miny as i32..=maxy as i32 {
            for sx in minx as i32..=maxx as i32 {
                let mut screen = Vec2::new(sx as f32 + 0.5, sy as f32 + 0.5);
                if !self.camera.y_up {
                    screen.y = self.camera.viewport.1 - screen.y;
                }
                let world = self.camera.screen_to_world(screen);
                let local = inv_transform.transform_point(world);
                if local.x < -half.x || local.x >= half.x || local.y < -half.y || local.y >= half.y
                {
                    continue;
                }
                let u = x0 + (local.x + half.x) / pw.max(1) as f32 * (x1 - x0);
                let v = y0 + (local.y + half.y) / ph.max(1) as f32 * (y1 - y0);
                let sample = match tex {
                    Some(t) => {
                        let fx = (u.fract() + 1.0).fract() * sw;
                        let fy = (v.fract() + 1.0).fract() * sh;
                        let ix = (fx as u32).min(t.width - 1);
                        let iy = (fy as u32).min(t.height - 1);
                        t.get(ix, iy).unwrap_or([0, 0, 0, 0])
                    }
                    None => [255, 255, 255, 255],
                };
                let a = (sample[3] as u16 * tint[3] as u16 / 255) as u8;
                if a == 0 {
                    continue;
                }
                let (dx, dy) = (sx as u32, sy as u32);
                let dst = target.get(dx, dy).unwrap_or([0, 0, 0, 255]);
                let mixed = [
                    (sample[0] as u16 * tint[0] as u16 / 255) as u8,
                    (sample[1] as u16 * tint[1] as u16 / 255) as u8,
                    (sample[2] as u16 * tint[2] as u16 / 255) as u8,
                    a,
                ];
                if a == 255 {
                    target.set(dx, dy, mixed);
                } else {
                    let inv = 255 - a as u16;
                    let out = [
                        ((mixed[0] as u16 * a as u16 + dst[0] as u16 * inv) / 255) as u8,
                        ((mixed[1] as u16 * a as u16 + dst[1] as u16 * inv) / 255) as u8,
                        ((mixed[2] as u16 * a as u16 + dst[2] as u16 * inv) / 255) as u8,
                        (a as u16).max(dst[3] as u16) as u8,
                    ];
                    target.set(dx, dy, out);
                }
            }
        }
    }

    fn flush(&mut self) -> DrawStats2D {
        self.stats.batches_merged = self.stats.draw_calls;
        self.stats
    }
}

impl SoftwareRenderer2D {
    /// 取回渲染目标（呈现用）。
    pub fn take_target(&mut self, out: &mut Rgba8Image) {
        if let Some(t) = self.target.take() {
            *out = *t;
        }
    }
}

// ---- 光照与材质（IF-184） ----

/// 3D 光源。
#[derive(Debug, Clone, Copy)]
pub enum Light {
    Directional { dir: Vec3, color: Color, intensity: f32 },
    Point { pos: Vec3, color: Color, intensity: f32, range: f32 },
    Spot { pos: Vec3, dir: Vec3, angle_rad: f32, color: Color, intensity: f32 },
}

/// 2D 光源。
#[derive(Debug, Clone, Copy)]
pub enum Light2D {
    Point { pos: Vec2, color: Color, intensity: f32, radius: f32 },
    Directional { dir: Vec2, color: Color, intensity: f32 },
    Ambient { color: Color, intensity: f32 },
}

/// 材质参数（IF-184）。
#[derive(Debug, Clone, Copy)]
pub struct MaterialParams {
    pub base_color: Color,
    pub emissive: Color,
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            emissive: Color::TRANSPARENT,
            roughness: 0.8,
            metallic: 0.0,
        }
    }
}

/// 3D 软件渲染器（IF-185）：经 rf-rhi SoftwareDevice 光栅化。
pub struct Renderer3D {
    device: rf_rhi::SoftwareDevice,
}

impl Renderer3D {
    pub fn new(device: rf_rhi::SoftwareDevice) -> Self {
        Self { device }
    }

    /// 上传网格 → (vb, ib)。
    pub fn upload_mesh(
        &mut self,
        mesh: &rf_asset::MeshAsset,
    ) -> (rf_rhi::BufferHandle, rf_rhi::BufferHandle) {
        let vb = self
            .device
            .create_buffer(
                rf_rhi::BufferDesc {
                    size: mesh.vertex_bytes().len(),
                    usage: rf_rhi::BufferUsage::VERTEX,
                    label: "mesh_vb".into(),
                },
                Some(&mesh.vertex_bytes()),
            )
            .unwrap();
        let ib_data: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let ib = self
            .device
            .create_buffer(
                rf_rhi::BufferDesc {
                    size: ib_data.len(),
                    usage: rf_rhi::BufferUsage::INDEX,
                    label: "mesh_ib".into(),
                },
                Some(&ib_data),
            )
            .unwrap();
        (vb, ib)
    }

    pub fn upload_texture(&mut self, image: &Rgba8Image) -> rf_rhi::TextureHandle {
        self.device
            .create_texture(
                rf_rhi::TextureDesc {
                    width: image.width,
                    height: image.height,
                    usage: rf_rhi::TextureUsage::SAMPLED,
                    ..Default::default()
                },
                Some(&image.data),
            )
            .unwrap()
    }

    /// 渲染一个网格到 target（CPU z-buffer Lambert）。
    #[allow(clippy::too_many_arguments)]
    pub fn render_mesh(
        &mut self,
        target: rf_rhi::TextureHandle,
        camera: &Camera3D,
        mesh: &rf_asset::MeshAsset,
        model: Mat4,
        material: &MaterialParams,
        texture: Option<rf_rhi::TextureHandle>,
        lights: &[Light],
    ) {
        let aspect = {
            let img = self.device.read_texture(target).unwrap();
            img.width as f32 / img.height.max(1) as f32
        };
        let (vb, ib) = self.upload_mesh(mesh);
        let pipe = self
            .device
            .create_pipeline(
                rf_rhi::PipelineDesc {
                    model: rf_rhi::ShaderModel::LambertTextured,
                    blend: rf_rhi::BlendMode::Opaque,
                    depth_test: true,
                    depth_write: true,
                    ..Default::default()
                },
                rf_rhi::VertexLayout::mesh(),
            )
            .unwrap();
        self.device.begin_frame(Some(target)).unwrap();
        let tex = texture.unwrap_or(rf_rhi::TextureHandle::INVALID);
        let mut lights_packed: Vec<[f32; 8]> = Vec::new();
        for l in lights {
            match l {
                Light::Directional { dir, color, intensity } => lights_packed
                    .push([dir.x, dir.y, dir.z, 0.0, color.r, color.g, color.b, *intensity]),
                Light::Point { pos, color, intensity, .. } => {
                    // 点光近似为朝向原点的方向光（P1：真实点光衰减）
                    let d = -*pos;
                    lights_packed.push([d.x, d.y, d.z, 0.0, color.r, color.g, color.b, *intensity]);
                }
                Light::Spot { dir, color, intensity, .. } => {
                    lights_packed
                        .push([dir.x, dir.y, dir.z, 0.0, color.r, color.g, color.b, *intensity]);
                }
            }
        }
        if lights_packed.is_empty() {
            lights_packed.push([0.0, 0.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.8]);
        }
        let img = self.device.read_texture(target).unwrap();
        let mut cmd = self.device.create_command_list();
        cmd.begin();
        cmd.clear_color(material.base_color).ok();
        cmd.set_pipeline(pipe);
        cmd.set_vertex_buffer(vb);
        cmd.set_index_buffer(ib, rf_rhi::IndexFormat::Uint32);
        cmd.set_texture(0, tex);
        let mvp = camera.view_projection(aspect).mul_mat4(model);
        cmd.set_uniform_mat4("u_mvp", &mvp);
        cmd.set_uniform_mat4("u_model", &model);
        cmd.set_viewport(0, 0, img.width, img.height);
        cmd.draw_indexed(0, mesh.indices.len() as u32);
        cmd.end();
        self.device.submit(cmd).unwrap();
        self.device.end_frame().unwrap();
    }

    pub fn device_mut(&mut self) -> &mut rf_rhi::SoftwareDevice {
        &mut self.device
    }
}

// ---- 渲染图（IF-186 / IF-187） ----

/// 资源 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    Raster,
    Compute,
    Copy,
    Present,
    TwoD,
}

/// 渲染图 Pass（IF-186，可扩展）。
pub trait RenderGraphPass {
    fn name(&self) -> &str;
    fn kind(&self) -> PassKind;
    fn inputs(&self) -> Vec<ResourceId>;
    fn outputs(&self) -> Vec<ResourceId>;
    fn execute(&mut self, ctx: &mut PassContext) -> Result<()>;
}

/// Pass 执行上下文。
pub struct PassContext {
    pub resources: std::collections::HashMap<ResourceId, Rgba8Image>,
    pub counters: std::collections::HashMap<String, u64>,
}

impl PassContext {
    pub fn new() -> Self {
        Self { resources: Default::default(), counters: Default::default() }
    }

    pub fn count(&mut self, name: &str, delta: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += delta;
    }
}

impl Default for PassContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 逻辑屏障计划。
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierPlan {
    pub resource: ResourceId,
    pub state_from: &'static str,
    pub state_to: &'static str,
}

/// 编译后的执行图。
pub struct CompiledGraph {
    pub order: Vec<usize>,
    pub barriers: Vec<BarrierPlan>,
}

/// 帧统计。
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub pass_times: Vec<(String, u64)>,
}

/// 渲染图（IF-187）。
#[derive(Default)]
pub struct RenderGraph {
    passes: Vec<Box<dyn RenderGraphPass>>,
    resources: Vec<ResourceId>,
    names: Vec<String>,
}

impl RenderGraph {
    pub fn declare_resource(&mut self, _name: &str, _desc: ()) -> ResourceId {
        let id = ResourceId(self.resources.len() as u64 + 1);
        self.resources.push(id);
        id
    }

    pub fn add_pass(&mut self, pass: Box<dyn RenderGraphPass>) -> usize {
        self.names.push(pass.name().to_string());
        self.passes.push(pass);
        self.passes.len() - 1
    }

    /// 拓扑排序 + 循环检测 + 自动屏障推导（IF-187）。
    pub fn compile(&self) -> Result<CompiledGraph> {
        let n = self.passes.len();
        // 依赖边：A 产出 r，B 消费 r → A→B
        let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (bi, b) in self.passes.iter().enumerate() {
            for r in b.inputs() {
                for (ai, a) in self.passes.iter().enumerate() {
                    if ai != bi && a.outputs().contains(&r) {
                        deps[bi].push(ai);
                    }
                }
            }
        }
        // Kahn：边 ai→bi（deps[bi] 含生产者 ai）
        let mut rev: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (bi, producers) in deps.iter().enumerate() {
            for &ai in producers {
                rev[ai].push(bi);
            }
        }
        let mut indeg: Vec<usize> = deps.iter().map(|p| p.len()).collect();
        let mut queue: std::collections::VecDeque<usize> =
            indeg.iter().enumerate().filter(|(_, d)| **d == 0).map(|(i, _)| i).collect();
        let mut order = Vec::with_capacity(n);
        while let Some(i) = queue.pop_front() {
            order.push(i);
            for &consumer in &rev[i] {
                indeg[consumer] -= 1;
                if indeg[consumer] == 0 {
                    queue.push_back(consumer);
                }
            }
        }
        if order.len() != n {
            return Err(rf_core::EngineError::Message("render graph: cycle detected".into()));
        }
        // 屏障：资源被写入→读取处插屏障
        let mut barriers = Vec::new();
        for (bi, b) in self.passes.iter().enumerate() {
            for r in b.inputs() {
                for (ai, a) in self.passes.iter().enumerate() {
                    if ai != bi
                        && a.outputs().contains(&r)
                        && order.iter().position(|x| *x == ai) < order.iter().position(|x| *x == bi)
                    {
                        barriers.push(BarrierPlan {
                            resource: r,
                            state_from: "Write",
                            state_to: "Read",
                        });
                    }
                }
            }
        }
        Ok(CompiledGraph { order, barriers })
    }

    /// 执行（按编译顺序），记录 pass 耗时。
    pub fn execute(
        &mut self,
        compiled: &CompiledGraph,
        ctx: &mut PassContext,
    ) -> Result<FrameStats> {
        let mut stats = FrameStats::default();
        for &i in &compiled.order {
            let start = std::time::Instant::now();
            self.passes[i].execute(ctx)?;
            stats.pass_times.push((self.names[i].clone(), start.elapsed().as_micros() as u64));
        }
        Ok(stats)
    }
}

// ---- LayerStack（IF-188） ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    TwoD,
    ThreeD,
}

/// 2D/3D 图层交叉编排（N7）。
#[derive(Default)]
pub struct LayerStack {
    layers: Vec<(String, LayerKind, i32)>,
}

impl LayerStack {
    pub fn add(&mut self, name: &str, kind: LayerKind, order: i32) -> usize {
        self.layers.push((name.to_string(), kind, order));
        self.layers.len() - 1
    }

    /// 按 order 排序的渲染顺序（2D/3D 交叉）。
    pub fn layers_in_order(&self) -> Vec<(&str, LayerKind)> {
        let mut v: Vec<&(String, LayerKind, i32)> = self.layers.iter().collect();
        v.sort_by_key(|(_, _, o)| *o);
        v.into_iter().map(|(n, k, _)| (n.as_str(), *k)).collect()
    }
}

// ---- Gizmo（IF-191） ----

/// 2D 句柄类型。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Handle2D {
    Body,
    Corner(u8),
    Rotation,
}

/// AABB 命中测试（角优先）。
pub fn aabb2_hit_test(point: Vec2, bb: &rf_math::geom::AABB2, grip: f32) -> Option<Handle2D> {
    if !bb.contains(point) {
        return None;
    }
    let corners =
        [(bb.min.x, bb.min.y), (bb.max.x, bb.min.y), (bb.max.x, bb.max.y), (bb.min.x, bb.max.y)];
    for (i, (cx, cy)) in corners.iter().enumerate() {
        if (point.x - cx).abs() <= grip && (point.y - cy).abs() <= grip {
            return Some(Handle2D::Corner(i as u8));
        }
    }
    Some(Handle2D::Body)
}

/// Gizmo 描边绘制（8 向阶梯线，无依赖）。
pub fn draw_gizmo2d(target: &mut Rgba8Image, bb: &rf_math::geom::AABB2, color: Color) {
    let rgba = color.to_rgba8();
    let (x0, y0) = (bb.min.x.round() as i32, bb.min.y.round() as i32);
    let (x1, y1) = (bb.max.x.round() as i32, bb.max.y.round() as i32);
    for x in x0.max(0)..x1.min(target.width as i32 - 1).max(x0) {
        target.set(x as u32, y0.max(0) as u32, rgba);
        target.set(x as u32, y1.clamp(0, target.height as i32 - 1) as u32, rgba);
    }
    for y in y0.max(0)..y1.min(target.height as i32 - 1).max(y0) {
        target.set(x0.max(0) as u32, y as u32, rgba);
        target.set(x1.clamp(0, target.width as i32 - 1) as u32, y as u32, rgba);
    }
}

/// 3D 轴向射线命中：返回 (axis, t)。axis: 0=X 1=Y 2=Z。
pub fn axis_ray_hit(ray: &rf_math::geom::Ray3, origin: Vec3, scale: f32) -> Option<(u8, f32)> {
    let axes = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)];
    let mut best: Option<(u8, f32)> = None;
    for (i, a) in axes.iter().enumerate() {
        // 射线到线段的最近点近似（阈值 = 0.1×scale）
        let end = origin + *a * scale;
        let threshold = (0.1 * scale).max(f32::EPSILON);
        for t in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let p = origin + *a * scale * t;
            let to_p = p - ray.origin;
            let proj = to_p.dot(ray.dir);
            if proj < 0.0 {
                continue;
            }
            let closest = ray.origin + ray.dir * proj;
            if (closest - p).length() <= threshold {
                let _ = end;
                if best.map(|(_, bt)| proj < bt).unwrap_or(true) {
                    best = Some((i as u8, proj));
                }
            }
        }
    }
    best
}

/// 便捷 CPU 3D 渲染：网格 + 相机 + 光照 → 独立 RGBA 帧（示例/编辑器预览用）。
#[allow(clippy::too_many_arguments)]
pub fn render_mesh_cpu(
    width: u32,
    height: u32,
    mesh: &rf_asset::MeshAsset,
    model: Mat4,
    camera: &Camera3D,
    material: &MaterialParams,
    texture: Option<&Rgba8Image>,
    lights: &[Light],
) -> Rgba8Image {
    let mut device = rf_rhi::SoftwareDevice::new(width, height);
    let target = device
        .create_texture(
            rf_rhi::TextureDesc {
                width,
                height,
                usage: rf_rhi::TextureUsage::RENDER_TARGET,
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let mut r = Renderer3D::new(device);
    let tex_handle = texture.map(|t| {
        r.device_mut()
            .create_texture(
                rf_rhi::TextureDesc {
                    width: t.width,
                    height: t.height,
                    usage: rf_rhi::TextureUsage::SAMPLED,
                    ..Default::default()
                },
                Some(&t.data),
            )
            .unwrap()
    });
    r.render_mesh(target, camera, mesh, model, material, tex_handle, lights);
    let out = r.device_mut().read_texture(target).unwrap();
    let mut img = Rgba8Image::new(width, height);
    img.data.copy_from_slice(&out.data);
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera2d_roundtrip() {
        let cam = Camera2D { viewport: (200.0, 100.0), zoom: 2.0, ..Default::default() };
        let world = Vec2::new(10.0, -5.0);
        let screen = cam.world_to_screen(world);
        let back = cam.screen_to_world(screen);
        assert!((back.x - world.x).abs() < 1e-4 && (back.y - world.y).abs() < 1e-4);
        // 屏幕中心 = 相机位置
        let c = cam.world_to_screen(Vec2::ZERO);
        assert!((c.x - 100.0).abs() < 1e-4 && (c.y - 50.0).abs() < 1e-4);
    }

    #[test]
    fn camera3d_axes() {
        let cam = Camera3D::default();
        let f = cam.forward();
        assert!(f.length() > 0.99 && f.length() < 1.01);
        assert!(f.dot(cam.right()).abs() < 1e-5);
        let vp = cam.view_projection(16.0 / 9.0);
        let origin_ndc = vp.transform_vector4(rf_math::Vec4::new(0.0, 0.0, 5.0, 1.0)); // 相机前 0（自身位置→at 前方点）
        let _ = origin_ndc;
    }

    #[test]
    fn sprite_batch_merge() {
        let mut batches = vec![
            SpriteBatchData {
                texture: Some(1),
                instances: vec![
                    SpriteInstance {
                        transform: Mat2x3::identity(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        color: Color::WHITE,
                        sort_key: 1,
                    },
                    SpriteInstance {
                        transform: Mat2x3::identity(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        color: Color::WHITE,
                        sort_key: 1,
                    },
                ],
            },
            SpriteBatchData {
                texture: Some(2),
                instances: vec![SpriteInstance {
                    transform: Mat2x3::identity(),
                    uv: [0.0, 0.0, 1.0, 1.0],
                    color: Color::WHITE,
                    sort_key: 0,
                }],
            },
        ];
        let merged = build_batches(&mut batches);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].instances.len(), 1); // sort_key 0 先
        assert_eq!(merged[1].instances.len(), 2); // 同纹理同层合并
    }

    #[test]
    fn software_renderer2d_draws() {
        let tex = Rgba8Image::filled(4, 4, [255, 128, 0, 255]);
        let mut target = Rgba8Image::filled(32, 32, [0, 0, 0, 255]);
        let mut r = SoftwareRenderer2D::new();
        let cam = Camera2D { viewport: (32.0, 32.0), zoom: 4.0, y_up: false, ..Default::default() };
        r.begin_frame(&mut target, &cam);
        r.draw_sprite(Some(&tex), &[0.0, 0.0, 1.0, 1.0], Color::WHITE, 0, &Mat2x3::identity());
        let stats = r.flush();
        r.take_target(&mut target);
        assert_eq!(stats.sprites, 1);
        assert!(stats.draw_calls >= 1);
        // 中心应被精灵覆盖（4×4 zoom → 16×16 像素块）
        assert_eq!(target.get(16, 16), Some([255, 128, 0, 255]));
        // 角落（>8 像素外）保持背景
        assert_eq!(target.get(31, 31), Some([0, 0, 0, 255]));
    }

    #[test]
    fn render_graph_topo_and_cycle() {
        struct ClearPass;
        impl RenderGraphPass for ClearPass {
            fn name(&self) -> &str {
                "clear"
            }
            fn kind(&self) -> PassKind {
                PassKind::Raster
            }
            fn inputs(&self) -> Vec<ResourceId> {
                vec![]
            }
            fn outputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(1)]
            }
            fn execute(&mut self, ctx: &mut PassContext) -> Result<()> {
                ctx.resources.insert(ResourceId(1), Rgba8Image::filled(4, 4, [1, 2, 3, 255]));
                Ok(())
            }
        }
        struct BlurPass;
        impl RenderGraphPass for BlurPass {
            fn name(&self) -> &str {
                "blur"
            }
            fn kind(&self) -> PassKind {
                PassKind::Compute
            }
            fn inputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(1)]
            }
            fn outputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(2)]
            }
            fn execute(&mut self, ctx: &mut PassContext) -> Result<()> {
                let src = ctx.resources.get(&ResourceId(1)).cloned().unwrap();
                ctx.count("blur_pixels", src.pixel_count() as u64);
                ctx.resources.insert(ResourceId(2), src);
                Ok(())
            }
        }
        struct PresentPass;
        impl RenderGraphPass for PresentPass {
            fn name(&self) -> &str {
                "present"
            }
            fn kind(&self) -> PassKind {
                PassKind::Present
            }
            fn inputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(2)]
            }
            fn outputs(&self) -> Vec<ResourceId> {
                vec![]
            }
            fn execute(&mut self, _ctx: &mut PassContext) -> Result<()> {
                Ok(())
            }
        }
        struct CyclePass;
        impl RenderGraphPass for CyclePass {
            fn name(&self) -> &str {
                "cycle"
            }
            fn kind(&self) -> PassKind {
                PassKind::Compute
            }
            fn inputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(2)]
            }
            fn outputs(&self) -> Vec<ResourceId> {
                vec![ResourceId(1)]
            }
            fn execute(&mut self, _ctx: &mut PassContext) -> Result<()> {
                Ok(())
            }
        }
        let mut g = RenderGraph::default();
        g.declare_resource("rt", ());
        g.add_pass(Box::new(ClearPass));
        g.add_pass(Box::new(BlurPass));
        g.add_pass(Box::new(PresentPass));
        let compiled = g.compile().unwrap();
        assert_eq!(compiled.order, vec![0, 1, 2]);
        assert!(!compiled.barriers.is_empty());
        let mut ctx = PassContext::new();
        let stats = g.execute(&compiled, &mut ctx).unwrap();
        assert_eq!(stats.pass_times.len(), 3);
        assert_eq!(ctx.counters.get("blur_pixels"), Some(&16));
        assert_eq!(ctx.resources[&ResourceId(2)].get(0, 0), Some([1, 2, 3, 255]));
        // 循环检测
        let mut g2 = RenderGraph::default();
        g2.declare_resource("rt", ());
        g2.add_pass(Box::new(ClearPass));
        g2.add_pass(Box::new(BlurPass));
        g2.add_pass(Box::new(CyclePass));
        assert!(g2.compile().is_err());
    }

    #[test]
    fn layer_stack_interleaves() {
        let mut ls = LayerStack::default();
        ls.add("bg3d", LayerKind::ThreeD, 0);
        ls.add("sprites", LayerKind::TwoD, 1);
        ls.add("fg3d", LayerKind::ThreeD, 2);
        let order = ls.layers_in_order();
        assert_eq!(
            order,
            vec![
                ("bg3d", LayerKind::ThreeD),
                ("sprites", LayerKind::TwoD),
                ("fg3d", LayerKind::ThreeD)
            ]
        );
    }

    #[test]
    fn gizmo_hit_test() {
        let bb = rf_math::geom::AABB2::from_center_half(Vec2::ZERO, Vec2::splat(10.0));
        assert_eq!(aabb2_hit_test(Vec2::new(0.0, 0.0), &bb, 4.0), Some(Handle2D::Body));
        assert_eq!(aabb2_hit_test(Vec2::new(10.0, 10.0), &bb, 4.0), Some(Handle2D::Corner(2)));
        assert_eq!(aabb2_hit_test(Vec2::new(50.0, 50.0), &bb, 4.0), None);
    }
}
