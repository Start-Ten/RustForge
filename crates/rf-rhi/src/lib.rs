//! RustForge RHI（IF-140 ~ IF-151）：设备/纹理/缓冲/管线抽象，
//! Software（CPU 光栅化）与 Null 后端 + 运行时选择与降级链。

pub mod software;

pub use software::SoftwareDevice;

use rf_core::{EngineError, Result, Rgba8Image};
use rf_math::Color;
use rf_math::{Mat4, Vec2, Vec3, Vec4};

/// 后端（IF-140）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    Vulkan,
    Dx12,
    Dx11,
    OpenGL,
    OpenGLES,
    WebGPU,
    Metal,
    Software,
    Null,
}

impl Backend {
    pub fn display_name(&self) -> &'static str {
        match self {
            Backend::Vulkan => "Vulkan 1.3 (skeleton)",
            Backend::Dx12 => "DirectX 12 (skeleton)",
            Backend::Dx11 => "DirectX 11 (skeleton)",
            Backend::OpenGL => "OpenGL 4.6 (skeleton)",
            Backend::OpenGLES => "OpenGL ES 3.2 (skeleton)",
            Backend::WebGPU => "WebGPU (skeleton)",
            Backend::Metal => "Metal (skeleton)",
            Backend::Software => "Software (CPU rasterizer)",
            Backend::Null => "Null (no-op)",
        }
    }
}

/// 能力集（IF-141，可扩展）。
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub max_texture_size: u32,
    pub max_vertex_attributes: u32,
    pub max_color_attachments: u32,
    pub supports_compute: bool,
    pub supports_raytracing: bool,
    pub supports_geometry_shader: bool,
    pub supports_bc: bool,
    pub supports_astc: bool,
    pub uniform_buffer_alignment: usize,
    pub timestamp_period_ns: f32,
}

impl Capabilities {
    /// Software 后端能力。
    pub fn software() -> Self {
        Self {
            max_texture_size: 16384,
            max_vertex_attributes: 16,
            max_color_attachments: 1,
            supports_compute: false,
            supports_raytracing: false,
            supports_geometry_shader: false,
            supports_bc: false,
            supports_astc: false,
            uniform_buffer_alignment: 4,
            timestamp_period_ns: 0.0,
        }
    }

    pub fn null() -> Self {
        Self { max_texture_size: 0, ..Self::software() }
    }
}

/// 后端信息（IF-142）。
#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub name: String,
    pub backend: Backend,
    pub dedicated_gpu: bool,
    pub caps: Capabilities,
}

/// 设备创建描述（IF-143）。
#[derive(Debug, Clone)]
pub struct DeviceDesc {
    pub preferred: Backend,
    pub allow_fallback: bool,
    pub headless: bool,
    pub software_size: (u32, u32),
}

impl Default for DeviceDesc {
    fn default() -> Self {
        Self {
            preferred: Backend::Software,
            allow_fallback: true,
            headless: false,
            software_size: (800, 600),
        }
    }
}

/// 创建设备（IF-143）：GPU 后端为 feature 门控骨架 → 按链 GPU→Software→Null 降级。
pub fn create_device(desc: DeviceDesc) -> Result<(Box<dyn RhiDevice>, BackendInfo)> {
    match desc.preferred {
        Backend::Software => {
            let d = SoftwareDevice::new(desc.software_size.0, desc.software_size.1);
            let info = d.backend_info().clone();
            Ok((Box::new(d), info))
        }
        Backend::Null => {
            let d = NullDevice;
            let info = d.backend_info().clone();
            Ok((Box::new(d), info))
        }
        gpu => {
            if !desc.allow_fallback {
                return Err(EngineError::Unsupported("gpu backend requires feature `gpu` (P1)"));
            }
            // 降级链：GPU → Software
            let d = SoftwareDevice::new(desc.software_size.0, desc.software_size.1);
            let mut info = d.backend_info().clone();
            info.name = format!("{} → Software fallback", gpu.display_name());
            Ok((Box::new(d), info))
        }
    }
}

/// 纹理格式（IF-144）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    R8Unorm,
    Rgba16Float,
    R32Float,
    Depth24Plus,
}

/// 纹理用途位（IF-144）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureUsage(pub u32);

impl TextureUsage {
    pub const SAMPLED: TextureUsage = TextureUsage(1);
    pub const RENDER_TARGET: TextureUsage = TextureUsage(2);
    pub const COPY_SRC: TextureUsage = TextureUsage(4);
    pub const COPY_DST: TextureUsage = TextureUsage(8);

    pub fn contains(self, other: TextureUsage) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn union(self, other: TextureUsage) -> TextureUsage {
        TextureUsage(self.0 | other.0)
    }
}

/// 纹理描述（IF-145）。
#[derive(Debug, Clone)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub mip_levels: u32,
    pub label: String,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::SAMPLED,
            mip_levels: 1,
            label: String::new(),
        }
    }
}

/// 缓冲用途位（IF-145）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferUsage(pub u32);

impl BufferUsage {
    pub const VERTEX: BufferUsage = BufferUsage(1);
    pub const INDEX: BufferUsage = BufferUsage(2);
    pub const UNIFORM: BufferUsage = BufferUsage(4);
    pub const STORAGE: BufferUsage = BufferUsage(8);
    pub const COPY_SRC: BufferUsage = BufferUsage(16);
    pub const COPY_DST: BufferUsage = BufferUsage(32);
}

/// 缓冲描述（IF-145）。
#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub size: usize,
    pub usage: BufferUsage,
    pub label: String,
}

/// 句柄（IF-146）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64);

impl TextureHandle {
    pub const INVALID: TextureHandle = TextureHandle(0);
}
impl BufferHandle {
    pub const INVALID: BufferHandle = BufferHandle(0);
}
impl PipelineHandle {
    pub const INVALID: PipelineHandle = PipelineHandle(0);
}

/// 内建着色模型（IF-147，可扩展）：Software 后端可解释执行；
/// GPU 后端（P1）对应编译 WGSL 变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderModel {
    /// 纯色。
    Flat,
    /// 纹理 × 顶点色（2D Sprite 主路径）。
    UnlitTextured,
    /// Lambert + 纹理（3D 主路径）。
    LambertTextured,
    /// 线框/调试。
    Wireframe,
}

/// 混合模式（IF-147）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Alpha,
    Additive,
    Multiply,
    PremultipliedAlpha,
}

/// 剔除模式（IF-147）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Back,
    Front,
}

/// 管线描述（IF-147）。
#[derive(Debug, Clone)]
pub struct PipelineDesc {
    pub model: ShaderModel,
    pub vertex_wgsl: Option<String>,
    pub fragment_wgsl: Option<String>,
    pub blend: BlendMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub cull: CullMode,
    pub label: String,
}

impl Default for PipelineDesc {
    fn default() -> Self {
        Self {
            model: ShaderModel::Flat,
            vertex_wgsl: None,
            fragment_wgsl: None,
            blend: BlendMode::Alpha,
            depth_test: false,
            depth_write: false,
            cull: CullMode::None,
            label: String::new(),
        }
    }
}

/// 顶点属性（IF-148，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
    Uint8x4,
}

#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub name: &'static str,
    pub format: VertexFormat,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct VertexLayout {
    pub stride: u32,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexLayout {
    /// 标准 2D 顶点：pos(2) + uv(2) + rgba(4×u8)。
    pub fn sprite() -> Self {
        Self {
            stride: 20,
            attributes: vec![
                VertexAttribute { name: "position", format: VertexFormat::Float32x2, offset: 0 },
                VertexAttribute { name: "uv", format: VertexFormat::Float32x2, offset: 8 },
                VertexAttribute { name: "color", format: VertexFormat::Uint8x4, offset: 16 },
            ],
        }
    }

    /// 标准 3D 顶点：pos(3) + normal(3) + uv(2)。
    pub fn mesh() -> Self {
        Self {
            stride: 32,
            attributes: vec![
                VertexAttribute { name: "position", format: VertexFormat::Float32x3, offset: 0 },
                VertexAttribute { name: "normal", format: VertexFormat::Float32x3, offset: 12 },
                VertexAttribute { name: "uv", format: VertexFormat::Float32x2, offset: 24 },
            ],
        }
    }
}

/// 索引格式（IF-148）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}

/// 绘制状态快照（CommandList 内部记录，调试可见）。
#[derive(Debug, Clone)]
pub struct DrawCallInfo {
    pub pipeline_label: String,
    pub shader_model: ShaderModel,
    pub index_count: u32,
    pub texture: Option<TextureHandle>,
}

/// RHI 设备（IF-149，可扩展）。
pub trait RhiDevice: Send {
    fn backend_info(&self) -> &BackendInfo;
    fn create_texture(&mut self, desc: TextureDesc, data: Option<&[u8]>) -> Result<TextureHandle>;
    fn update_texture(
        &mut self,
        tex: TextureHandle,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Result<()>;
    fn read_texture(&self, tex: TextureHandle) -> Result<Rgba8Image>;
    fn create_buffer(&mut self, desc: BufferDesc, data: Option<&[u8]>) -> Result<BufferHandle>;
    fn update_buffer(&mut self, buf: BufferHandle, offset: usize, data: &[u8]) -> Result<()>;
    fn create_pipeline(
        &mut self,
        desc: PipelineDesc,
        layout: VertexLayout,
    ) -> Result<PipelineHandle>;
    fn create_command_list(&mut self) -> Box<dyn CommandList>;
    fn submit(&mut self, cmd: Box<dyn CommandList>) -> Result<()>;
    fn begin_frame(&mut self, target: Option<TextureHandle>) -> Result<()>;
    fn end_frame(&mut self) -> Result<()>;
    /// Software 模式的呈现目标；GPU 模式对应交换链。
    fn set_present_target(&mut self, target: Option<TextureHandle>);
    /// 上一帧统计（DrawCall/三角形数；Profiler 展示）。
    fn frame_stats(&self) -> FrameCounters;
}

/// 帧计数器（IF-305 渲染统计的数据源）。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameCounters {
    pub draw_calls: u32,
    pub triangles: u32,
    pub clears: u32,
    pub blits: u32,
}

/// 命令列表（IF-150，可扩展）。
pub trait CommandList {
    fn begin(&mut self);
    /// 【CP-002 变更提案】内部下转支持（Software 后端 submit 取回录制命令）。
    fn as_any(&mut self) -> &mut dyn std::any::Any;
    fn set_pipeline(&mut self, pipe: PipelineHandle);
    fn set_texture(&mut self, slot: u32, tex: TextureHandle);
    fn set_uniform_f32(&mut self, name: &str, values: &[f32]);
    fn set_uniform_mat4(&mut self, name: &str, m: &Mat4);
    fn set_viewport(&mut self, x: u32, y: u32, w: u32, h: u32);
    fn clear_color(&mut self, color: Color) -> Result<()>;
    fn clear_depth(&mut self, depth: f32);
    fn set_vertex_buffer(&mut self, buf: BufferHandle);
    fn set_index_buffer(&mut self, buf: BufferHandle, format: IndexFormat);
    fn draw(&mut self, first: u32, count: u32);
    fn draw_indexed(&mut self, first_index: u32, count: u32);
    fn blit(&mut self, src: TextureHandle, dst_x: u32, dst_y: u32, dst_w: u32, dst_h: u32);
    fn push_debug_group(&mut self, label: &str);
    fn pop_debug_group(&mut self);
    fn end(&mut self);
}

/// Null 后端（IF-151）：吞掉一切命令，验收选择/降级链与接口完备性。
pub struct NullDevice;

impl RhiDevice for NullDevice {
    fn backend_info(&self) -> &BackendInfo {
        static INFO: std::sync::OnceLock<BackendInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| BackendInfo {
            name: "Null".into(),
            backend: Backend::Null,
            dedicated_gpu: false,
            caps: Capabilities::null(),
        })
    }

    fn create_texture(
        &mut self,
        _desc: TextureDesc,
        _data: Option<&[u8]>,
    ) -> Result<TextureHandle> {
        Ok(TextureHandle(1))
    }

    fn update_texture(
        &mut self,
        _t: TextureHandle,
        _x: u32,
        _y: u32,
        _w: u32,
        _h: u32,
        _d: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    fn read_texture(&self, _t: TextureHandle) -> Result<Rgba8Image> {
        Ok(Rgba8Image::new(1, 1))
    }

    fn create_buffer(&mut self, _desc: BufferDesc, _data: Option<&[u8]>) -> Result<BufferHandle> {
        Ok(BufferHandle(1))
    }

    fn update_buffer(&mut self, _b: BufferHandle, _o: usize, _d: &[u8]) -> Result<()> {
        Ok(())
    }

    fn create_pipeline(
        &mut self,
        _desc: PipelineDesc,
        _layout: VertexLayout,
    ) -> Result<PipelineHandle> {
        Ok(PipelineHandle(1))
    }

    fn create_command_list(&mut self) -> Box<dyn CommandList> {
        Box::new(NullCommandList)
    }

    fn submit(&mut self, _cmd: Box<dyn CommandList>) -> Result<()> {
        Ok(())
    }

    fn begin_frame(&mut self, _target: Option<TextureHandle>) -> Result<()> {
        Ok(())
    }

    fn end_frame(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_present_target(&mut self, _target: Option<TextureHandle>) {}

    fn frame_stats(&self) -> FrameCounters {
        FrameCounters::default()
    }
}

struct NullCommandList;

impl CommandList for NullCommandList {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn begin(&mut self) {}
    fn set_pipeline(&mut self, _p: PipelineHandle) {}
    fn set_texture(&mut self, _s: u32, _t: TextureHandle) {}
    fn set_uniform_f32(&mut self, _n: &str, _v: &[f32]) {}
    fn set_uniform_mat4(&mut self, _n: &str, _m: &Mat4) {}
    fn set_viewport(&mut self, _x: u32, _y: u32, _w: u32, _h: u32) {}
    fn clear_color(&mut self, _c: Color) -> Result<()> {
        Ok(())
    }
    fn clear_depth(&mut self, _d: f32) {}
    fn set_vertex_buffer(&mut self, _b: BufferHandle) {}
    fn set_index_buffer(&mut self, _b: BufferHandle, _f: IndexFormat) {}
    fn draw(&mut self, _f: u32, _c: u32) {}
    fn draw_indexed(&mut self, _f: u32, _c: u32) {}
    fn blit(&mut self, _s: TextureHandle, _x: u32, _y: u32, _w: u32, _h: u32) {}
    fn push_debug_group(&mut self, _l: &str) {}
    fn pop_debug_group(&mut self) {}
    fn end(&mut self) {}
}

/// 顶点数据解释辅助（Software 光栅共用）。
pub fn read_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

pub fn read_vertex_sprite(data: &[u8], base: usize) -> (Vec2, Vec2, [u8; 4]) {
    (
        Vec2::new(read_f32(data, base), read_f32(data, base + 4)),
        Vec2::new(read_f32(data, base + 8), read_f32(data, base + 12)),
        [data[base + 16], data[base + 17], data[base + 18], data[base + 19]],
    )
}

pub fn read_vertex_mesh(data: &[u8], base: usize) -> (Vec3, Vec3, Vec2) {
    (
        Vec3::new(read_f32(data, base), read_f32(data, base + 4), read_f32(data, base + 8)),
        Vec3::new(read_f32(data, base + 12), read_f32(data, base + 16), read_f32(data, base + 20)),
        Vec2::new(read_f32(data, base + 24), read_f32(data, base + 28)),
    )
}

pub fn clip_pos(v: Vec4) -> (f32, f32, f32, f32) {
    if v.w.abs() < f32::EPSILON {
        (v.x, v.y, v.z, 1.0)
    } else {
        (v.x / v.w, v.y / v.w, v.z / v.w, v.w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_selection_and_fallback() {
        // Software 直连
        let (d, info) = create_device(DeviceDesc::default()).unwrap();
        assert_eq!(info.backend, Backend::Software);
        assert_eq!(d.backend_info().name, "Software (CPU rasterizer)");
        // 请求 Vulkan → 降级 Software（记录链名）
        let (_, info) =
            create_device(DeviceDesc { preferred: Backend::Vulkan, ..Default::default() }).unwrap();
        assert!(info.name.contains("fallback"));
        // 禁止降级 → 明确错误
        let err = create_device(DeviceDesc {
            preferred: Backend::Dx12,
            allow_fallback: false,
            ..Default::default()
        });
        assert!(matches!(err, Err(EngineError::Unsupported(_))));
        // Null
        let (_, info) =
            create_device(DeviceDesc { preferred: Backend::Null, ..Default::default() }).unwrap();
        assert_eq!(info.backend, Backend::Null);
    }

    #[test]
    fn usage_bits() {
        let u = TextureUsage::SAMPLED.union(TextureUsage::RENDER_TARGET);
        assert!(u.contains(TextureUsage::SAMPLED));
        assert!(u.contains(TextureUsage::RENDER_TARGET));
        assert!(!u.contains(TextureUsage::COPY_DST));
    }
}
