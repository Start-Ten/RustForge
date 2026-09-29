//! Software 后端：CPU 光栅化（三角形填充 + z-buffer + 双线性采样 + Lambert）。

use crate::*;
use rf_core::Rgba8Image;
use rf_math::{Mat4, Vec3, Vec4};
use std::collections::HashMap;

struct TextureRes {
    image: Rgba8Image,
    #[allow(dead_code)]
    desc: TextureDesc,
}

struct BufferRes {
    data: Vec<u8>,
    #[allow(dead_code)]
    usage: BufferUsage,
}

struct PipelineRes {
    desc: PipelineDesc,
    #[allow(dead_code)]
    layout: VertexLayout,
}

struct RecordedCmd {
    kind: CmdKind,
}

/// 绘制状态快照。
#[derive(Clone)]
struct DrawState {
    pipe: Option<PipelineHandle>,
    vb: Option<BufferHandle>,
    ib: Option<(BufferHandle, IndexFormat)>,
    tex: [Option<TextureHandle>; 4],
    mvp: Mat4,
    model: Mat4,
    lights: Vec<[f32; 8]>,
    viewport: (u32, u32, u32, u32),
}

enum CmdKind {
    ClearColor(rf_math::Color),
    ClearDepth(#[allow(dead_code)] f32),
    DrawIndexed { first: u32, count: u32, state: Box<DrawState> },
    Blit { src: TextureHandle, x: u32, y: u32, w: u32, h: u32 },
}

/// Software 设备：资源在 CPU 侧，命令录制后于 submit 即时执行。
pub struct SoftwareDevice {
    info: BackendInfo,
    textures: HashMap<u64, TextureRes>,
    buffers: HashMap<u64, BufferRes>,
    pipelines: HashMap<u64, PipelineRes>,
    next_handle: u64,
    present_target: Option<TextureHandle>,
    stats: FrameCounters,
    // 命令执行态
    state_pipe: Option<PipelineHandle>,
    state_vb: Option<BufferHandle>,
    state_ib: Option<(BufferHandle, IndexFormat)>,
    state_tex: [Option<TextureHandle>; 4],
    state_mvp: Mat4,
    state_model: Mat4,
    state_lights: Vec<[f32; 8]>, // dir.xyz, pad, color.rgb, intensity
    state_viewport: (u32, u32, u32, u32),
}

impl SoftwareDevice {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            info: BackendInfo {
                name: "Software (CPU rasterizer)".into(),
                backend: Backend::Software,
                dedicated_gpu: false,
                caps: Capabilities::software(),
            },
            textures: HashMap::new(),
            buffers: HashMap::new(),
            pipelines: HashMap::new(),
            next_handle: 1,
            present_target: None,
            stats: FrameCounters::default(),
            state_pipe: None,
            state_vb: None,
            state_ib: None,
            state_tex: [None; 4],
            state_mvp: Mat4::identity(),
            state_model: Mat4::identity(),
            state_lights: Vec::new(),
            state_viewport: (0, 0, width, height),
        }
    }

    fn alloc_handle(&mut self) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    fn target_mut(&mut self) -> &mut Rgba8Image {
        let fallback_w = self.info.caps.max_texture_size.min(1);
        let _ = fallback_w;
        let tex = self
            .present_target
            .or_else(|| self.textures.keys().next().copied().map(TextureHandle))
            .unwrap_or(TextureHandle::INVALID);
        self.textures
            .entry(tex.0)
            .or_insert_with(|| TextureRes {
                image: Rgba8Image::new(1, 1),
                desc: TextureDesc::default(),
            })
            .image_mut()
    }

    fn execute(&mut self, cmds: Vec<RecordedCmd>) {
        for cmd in cmds {
            match cmd.kind {
                CmdKind::ClearColor(c) => {
                    self.stats.clears += 1;
                    let rgba = c.to_rgba8();
                    self.target_mut().clear(rgba);
                }
                CmdKind::ClearDepth(_) => {}
                CmdKind::Blit { src, x, y, w, h } => {
                    self.stats.blits += 1;
                    let Some(src_img) = self.textures.get(&src.0).map(|t| t.image.clone()) else {
                        continue;
                    };
                    let (tw, th) = (self.state_viewport.2.max(1), self.state_viewport.3.max(1));
                    let dst = self.target_mut();
                    for py in 0..h.min(th.saturating_sub(y)) {
                        for px in 0..w.min(tw.saturating_sub(x)) {
                            let u = (px as f32 + 0.5) / w as f32;
                            let v = (py as f32 + 0.5) / h as f32;
                            let sx = (u * src_img.width as f32 - 0.5).round() as u32;
                            let sy = (v * src_img.height as f32 - 0.5).round() as u32;
                            if let Some(c) =
                                src_img.get(sx.min(src_img.width - 1), sy.min(src_img.height - 1))
                            {
                                dst.set(x + px, y + py, c);
                            }
                        }
                    }
                }
                CmdKind::DrawIndexed { first, count, state } => {
                    self.state_pipe = state.pipe;
                    self.state_vb = state.vb;
                    self.state_ib = state.ib;
                    self.state_tex = state.tex;
                    self.state_mvp = state.mvp;
                    self.state_model = state.model;
                    self.state_lights = state.lights;
                    self.state_viewport = state.viewport;
                    self.rasterize(first, count);
                }
            }
        }
    }

    fn rasterize(&mut self, first: u32, count: u32) {
        let Some(pipe_h) = self.state_pipe else { return };
        let Some(pipe) = self.pipelines.get(&pipe_h.0) else { return };
        let Some(vb) = self.state_vb.and_then(|h| self.buffers.get(&h.0)) else { return };
        let Some((ib_h, ifmt)) = self.state_ib else { return };
        let Some(ib) = self.buffers.get(&ib_h.0) else { return };
        let model = pipe.desc.model;
        let depth_test = pipe.desc.depth_test;
        let tex = self.state_tex[0].and_then(|h| self.textures.get(&h.0).map(|t| t.image.clone()));
        // 帧尺寸
        let target_h = self
            .present_target
            .and_then(|t| self.textures.get(&t.0))
            .map(|t| t.image.height)
            .unwrap_or(1);
        let fb_h = target_h as f32;
        let mvp = self.state_mvp;
        let model_mat = self.state_model;
        let lights = self.state_lights.clone();
        self.stats.draw_calls += 1;

        let mut zbuf: Vec<f32> = Vec::new();
        if depth_test {
            // 每次绘制使用临时 z（正确性优先；持久 z-buffer 列 P1）
            zbuf =
                vec![1.0f32; (self.state_viewport.2 as usize) * (self.state_viewport.3 as usize)];
        }
        let vp = self.state_viewport;
        let mut tris = 0u32;

        let tri_count = count / 3;
        for t in 0..tri_count {
            #[derive(Clone, Copy)]
            struct Vtx {
                clip: Vec4,
                normal: Vec3,
                uv: [f32; 2],
                #[allow(dead_code)]
                color: [u8; 4],
            }
            let mut tri_v =
                [Vtx { clip: Vec4::ZERO, normal: Vec3::ZERO, uv: [0.0; 2], color: [0; 4] }; 3];
            let mut valid = true;
            for (i, v) in tri_v.iter_mut().enumerate() {
                let idx = match ifmt {
                    IndexFormat::Uint16 => {
                        let o = ((first + t * 3 + i as u32) * 2) as usize;
                        if o + 2 > ib.data.len() {
                            valid = false;
                            break;
                        }
                        u16::from_le_bytes([ib.data[o], ib.data[o + 1]]) as usize
                    }
                    IndexFormat::Uint32 => {
                        let o = ((first + t * 3 + i as u32) * 4) as usize;
                        if o + 4 > ib.data.len() {
                            valid = false;
                            break;
                        }
                        u32::from_le_bytes(ib.data[o..o + 4].try_into().unwrap()) as usize
                    }
                };
                let base = idx * 32; // mesh stride
                if base + 32 > vb.data.len() {
                    valid = false;
                    break;
                }
                let (pos, normal, uv) = read_vertex_mesh(&vb.data, base);
                let world = model_mat.transform_point3(pos);
                let clip = mvp.transform_vector4(Vec4::new(pos.x, pos.y, pos.z, 1.0));
                *v = Vtx { clip, normal, uv: [uv.x, uv.y], color: [255, 255, 255, 255] };
                let _ = world;
            }
            if !valid {
                continue;
            }
            tris += 1;
            // NDC
            let ndc: [(f32, f32, f32); 3] = {
                let a = clip_pos(tri_v[0].clip);
                let b = clip_pos(tri_v[1].clip);
                let c = clip_pos(tri_v[2].clip);
                [(a.0, a.1, a.2), (b.0, b.1, b.2), (c.0, c.1, c.2)]
            };
            // 屏幕坐标（y 翻转：NDC +y 上 → 屏幕 +y 下）
            let w = vp.2 as f32;
            let h = vp.3 as f32;
            let to_screen = |x: f32, y: f32| ((x * 0.5 + 0.5) * w, (1.0 - (y * 0.5 + 0.5)) * h);
            let (ax, ay) = to_screen(ndc[0].0, ndc[0].1);
            let (bx, by) = to_screen(ndc[1].0, ndc[1].1);
            let (cx, cy) = to_screen(ndc[2].0, ndc[2].1);
            let area = (bx - ax) * (cy - ay) - (cx - ax) * (by - ay);
            if area.abs() < f32::EPSILON {
                continue;
            }
            // 背面剔除（屏幕空间顺时针为正面）
            let front = area > 0.0;
            match pipe.desc.cull {
                CullMode::Back if !front => continue,
                CullMode::Front if front => continue,
                _ => {}
            }
            let minx = ax.min(bx).min(cx).floor().max(0.0) as i32;
            let maxx = ax.max(bx).max(cx).ceil().min(w - 1.0) as i32;
            let miny = ay.min(by).min(cy).floor().max(0.0) as i32;
            let maxy = ay.max(by).max(cy).ceil().min(h - 1.0) as i32;
            let inv_area = 1.0 / area;
            let target =
                self.present_target.and_then(|t| self.textures.get(&t.0)).map(|t| t.image.clone());
            let Some(mut target) = target else { continue };
            for py in miny..=maxy {
                for px in minx..=maxx {
                    let fx = px as f32 + 0.5;
                    let fy = py as f32 + 0.5;
                    let w0 = ((bx - ax) * (fy - ay) - (by - ay) * (fx - ax)) * inv_area;
                    let w1 = ((cx - bx) * (fy - by) - (cy - by) * (fx - bx)) * inv_area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * ndc[0].2 + w1 * ndc[1].2 + w2 * ndc[2].2;
                    if depth_test {
                        let zi = py as usize * vp.2 as usize + px as usize;
                        if let Some(zref) = zbuf.get(zi) {
                            if z >= *zref {
                                continue;
                            }
                        }
                    }
                    // 着色
                    let uv_u = w0 * tri_v[0].uv[0] + w1 * tri_v[1].uv[0] + w2 * tri_v[2].uv[0];
                    let uv_v = w0 * tri_v[0].uv[1] + w1 * tri_v[1].uv[1] + w2 * tri_v[2].uv[1];
                    let rgba: [u8; 4] = match model {
                        ShaderModel::Flat => [255, 255, 255, 255],
                        ShaderModel::Wireframe => [128, 255, 128, 255],
                        ShaderModel::UnlitTextured => match &tex {
                            Some(t) => sample_bilinear(t, uv_u, uv_v),
                            None => [255, 255, 255, 255],
                        },
                        ShaderModel::LambertTextured => {
                            // 世界空间法线（近似用 model 旋转部分）
                            let n0 = tri_v[0].normal;
                            let n1 = tri_v[1].normal;
                            let n2 = tri_v[2].normal;
                            let n = rf_math::Vec3::new(
                                w0 * n0.x + w1 * n1.x + w2 * n2.x,
                                w0 * n0.y + w1 * n1.y + w2 * n2.y,
                                w0 * n0.z + w1 * n1.z + w2 * n2.z,
                            );
                            let wn = model_mat.transform_vector4(Vec4::new(n.x, n.y, n.z, 0.0));
                            let wn = Vec3::new(wn.x, wn.y, wn.z).normalized();
                            let mut light_r = 0.35f32; // 环境
                            let mut light_g = 0.35;
                            let mut light_b = 0.35;
                            for l in &lights {
                                let dir = Vec3::new(-l[0], -l[1], -l[2]).normalized();
                                let ndl = wn.dot(dir).max(0.0);
                                let it = l[7];
                                light_r += l[4] * ndl * it;
                                light_g += l[5] * ndl * it;
                                light_b += l[6] * ndl * it;
                            }
                            let base = match &tex {
                                Some(t) => sample_bilinear(t, uv_u, uv_v),
                                None => [255, 255, 255, 255],
                            };
                            [
                                (base[0] as f32 * light_r).min(255.0) as u8,
                                (base[1] as f32 * light_g).min(255.0) as u8,
                                (base[2] as f32 * light_b).min(255.0) as u8,
                                base[3],
                            ]
                        }
                    };
                    target.set(px as u32, py as u32, rgba);
                    if depth_test {
                        let zi = py as usize * vp.2 as usize + px as usize;
                        if let Some(zref) = zbuf.get_mut(zi) {
                            *zref = z;
                        }
                    }
                }
            }
            // 写回
            if let Some(t) = self.present_target {
                if let Some(res) = self.textures.get_mut(&t.0) {
                    res.image = target;
                }
            }
            let _ = fb_h;
        }
        self.stats.triangles += tris;
    }
}

fn sample_bilinear(img: &Rgba8Image, u: f32, v: f32) -> [u8; 4] {
    // MVP 用最近邻（双线性列 P1）；uv 重复寻址
    let x = ((u.fract() + 1.0).fract() * img.width as f32) as u32 % img.width;
    let y = ((v.fract() + 1.0).fract() * img.height as f32) as u32 % img.height;
    img.get(x, y).unwrap_or([255, 0, 255, 255])
}

impl TextureRes {
    fn image_mut(&mut self) -> &mut Rgba8Image {
        &mut self.image
    }
}

impl RhiDevice for SoftwareDevice {
    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    fn create_texture(&mut self, desc: TextureDesc, data: Option<&[u8]>) -> Result<TextureHandle> {
        if desc.width == 0
            || desc.height == 0
            || desc.width > self.info.caps.max_texture_size
            || desc.height > self.info.caps.max_texture_size
        {
            return Err(rf_core::EngineError::InvalidData(format!(
                "texture size {}x{} out of limits",
                desc.width, desc.height
            )));
        }
        let h = self.alloc_handle();
        let image = if let Some(d) = data {
            Rgba8Image::from_raw(desc.width, desc.height, d.to_vec())?
        } else {
            Rgba8Image::new(desc.width, desc.height)
        };
        self.textures.insert(h, TextureRes { image, desc });
        Ok(TextureHandle(h))
    }

    fn update_texture(
        &mut self,
        tex: TextureHandle,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Result<()> {
        let res = self.textures.get_mut(&tex.0).ok_or(rf_core::EngineError::StaleHandle)?;
        if data.len() != (w as usize * h as usize * 4) {
            return Err(rf_core::EngineError::InvalidData("texture update size mismatch".into()));
        }
        for py in 0..h {
            for px in 0..w {
                let i = ((py * w + px) * 4) as usize;
                res.image.set(x + px, y + py, [data[i], data[i + 1], data[i + 2], data[i + 3]]);
            }
        }
        Ok(())
    }

    fn read_texture(&self, tex: TextureHandle) -> Result<Rgba8Image> {
        self.textures.get(&tex.0).map(|t| t.image.clone()).ok_or(rf_core::EngineError::StaleHandle)
    }

    fn create_buffer(&mut self, desc: BufferDesc, data: Option<&[u8]>) -> Result<BufferHandle> {
        let h = self.alloc_handle();
        let mut buf = vec![0u8; desc.size];
        if let Some(d) = data {
            if d.len() > desc.size {
                return Err(rf_core::EngineError::InvalidData("buffer data exceeds size".into()));
            }
            buf[..d.len()].copy_from_slice(d);
        }
        self.buffers.insert(h, BufferRes { data: buf, usage: desc.usage });
        Ok(BufferHandle(h))
    }

    fn update_buffer(&mut self, buf: BufferHandle, offset: usize, data: &[u8]) -> Result<()> {
        let res = self.buffers.get_mut(&buf.0).ok_or(rf_core::EngineError::StaleHandle)?;
        if offset + data.len() > res.data.len() {
            return Err(rf_core::EngineError::InvalidData("buffer update out of range".into()));
        }
        res.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    fn create_pipeline(
        &mut self,
        desc: PipelineDesc,
        layout: VertexLayout,
    ) -> Result<PipelineHandle> {
        let h = self.alloc_handle();
        self.pipelines.insert(h, PipelineRes { desc, layout });
        Ok(PipelineHandle(h))
    }

    fn create_command_list(&mut self) -> Box<dyn CommandList> {
        let (w, h) = (
            self.present_target
                .and_then(|t| self.textures.get(&t.0))
                .map(|t| t.image.width)
                .unwrap_or(self.state_viewport.2),
            self.present_target
                .and_then(|t| self.textures.get(&t.0))
                .map(|t| t.image.height)
                .unwrap_or(self.state_viewport.3),
        );
        Box::new(SoftwareCommandList {
            recorded: Vec::new(),
            begun: false,
            pipe: None,
            vb: None,
            ib: None,
            tex: [None; 4],
            mvp: Mat4::identity(),
            model: Mat4::identity(),
            lights: Vec::new(),
            viewport: (0, 0, w, h),
        })
    }

    fn submit(&mut self, mut cmd: Box<dyn CommandList>) -> Result<()> {
        // 下转回具体命令列表取回录制结果（Software 专属类型）
        let cmds = cmd
            .as_any()
            .downcast_mut::<SoftwareCommandList>()
            .map(|c| std::mem::take(&mut c.recorded))
            .unwrap_or_default();
        self.execute(cmds);
        Ok(())
    }

    fn begin_frame(&mut self, target: Option<TextureHandle>) -> Result<()> {
        self.present_target = target;
        self.stats = FrameCounters::default();
        Ok(())
    }

    fn end_frame(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_present_target(&mut self, target: Option<TextureHandle>) {
        self.present_target = target;
    }

    fn frame_stats(&self) -> FrameCounters {
        self.stats
    }
}

struct SoftwareCommandList {
    recorded: Vec<RecordedCmd>,
    begun: bool,
    pipe: Option<PipelineHandle>,
    vb: Option<BufferHandle>,
    ib: Option<(BufferHandle, IndexFormat)>,
    tex: [Option<TextureHandle>; 4],
    mvp: Mat4,
    model: Mat4,
    lights: Vec<[f32; 8]>,
    viewport: (u32, u32, u32, u32),
}

impl CommandList for SoftwareCommandList {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn begin(&mut self) {
        self.begun = true;
    }

    fn set_pipeline(&mut self, pipe: PipelineHandle) {
        self.pipe = Some(pipe);
    }

    fn set_texture(&mut self, slot: u32, tex: TextureHandle) {
        if (slot as usize) < 4 {
            self.tex[slot as usize] = Some(tex);
        }
    }

    /// "u_light[%d]"：[dir.xyz, 0, color.rgb, intensity] ×8。
    fn set_uniform_f32(&mut self, name: &str, values: &[f32]) {
        if let Some(rest) = name.strip_prefix("u_light[") {
            if let Some(idx_str) = rest.strip_suffix(']') {
                if let Ok(i) = idx_str.parse::<usize>() {
                    while self.lights.len() <= i {
                        self.lights.push([0.0; 8]);
                    }
                    for (j, v) in values.iter().take(8).enumerate() {
                        self.lights[i][j] = *v;
                    }
                }
            }
        }
    }

    fn set_uniform_mat4(&mut self, name: &str, m: &Mat4) {
        match name {
            "u_mvp" | "u_view_proj" => self.mvp = *m,
            "u_model" => self.model = *m,
            _ => {}
        }
    }

    fn set_viewport(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.viewport = (x, y, w, h);
    }

    fn clear_color(&mut self, color: rf_math::Color) -> Result<()> {
        if !self.begun {
            return Err(rf_core::EngineError::Message("clear before begin".into()));
        }
        self.recorded.push(RecordedCmd { kind: CmdKind::ClearColor(color) });
        Ok(())
    }

    fn clear_depth(&mut self, depth: f32) {
        self.recorded.push(RecordedCmd { kind: CmdKind::ClearDepth(depth) });
    }

    fn set_vertex_buffer(&mut self, buf: BufferHandle) {
        self.vb = Some(buf);
    }

    fn set_index_buffer(&mut self, buf: BufferHandle, format: IndexFormat) {
        self.ib = Some((buf, format));
    }

    fn draw(&mut self, _first: u32, _count: u32) {}

    fn draw_indexed(&mut self, first: u32, count: u32) {
        let state = Box::new(DrawState {
            pipe: self.pipe,
            vb: self.vb,
            ib: self.ib,
            tex: self.tex,
            mvp: self.mvp,
            model: self.model,
            lights: self.lights.clone(),
            viewport: self.viewport,
        });
        self.recorded.push(RecordedCmd { kind: CmdKind::DrawIndexed { first, count, state } });
    }

    fn blit(&mut self, src: TextureHandle, x: u32, y: u32, w: u32, h: u32) {
        self.recorded.push(RecordedCmd { kind: CmdKind::Blit { src, x, y, w, h } });
    }

    fn push_debug_group(&mut self, _label: &str) {}

    fn pop_debug_group(&mut self) {}

    fn end(&mut self) {}
}
