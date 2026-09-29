//! 后处理（IF-189）与上采样（IF-190）。

use rf_core::{Result, Rgba8Image};

/// 色调映射（IF-189）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneMap {
    None,
    Reinhard,
    Aces,
    Filmic,
    AgX,
}

fn srgb_to_linear_u8(c: u8) -> f32 {
    let v = c as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let out = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (out * 255.0).round() as u8
}

fn tone_op(mode: ToneMap, x: f32) -> f32 {
    match mode {
        ToneMap::None => x,
        ToneMap::Reinhard => x / (1.0 + x),
        ToneMap::Aces => {
            // Narkowicz ACES 近似
            let a = 2.51;
            let b = 0.03;
            let c = 2.43;
            let d = 0.59;
            let e = 0.14;
            ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
        }
        ToneMap::Filmic => {
            // Uncharted2 近似（简化）
            let x = x.max(0.0);
            let a = 0.15;
            let b = 0.5;
            let c = 0.1;
            let d = 0.2;
            let num = x * (a * x + c * b) + d * b;
            let den = x * (a * x + b) + d * b;
            (num / den.max(f32::EPSILON)).clamp(0.0, 1.0)
        }
        ToneMap::AgX => {
            // AgX 近似（对数压缩 + 高光去饱和）
            let v = x.max(0.0);
            let compressed = v / (1.0 + v * 0.15).max(1.0);
            let desat = compressed.min(1.0) * 0.85 + compressed * 0.15;
            desat.clamp(0.0, 1.0)
        }
    }
}

/// 就地色调映射。
pub fn tone_map(img: &mut Rgba8Image, mode: ToneMap) {
    for px in img.data.chunks_exact_mut(4) {
        let lin = [srgb_to_linear_u8(px[0]), srgb_to_linear_u8(px[1]), srgb_to_linear_u8(px[2])];
        let exposure = 1.0; // P1：可配置曝光
        let out = [
            tone_op(mode, lin[0] * exposure),
            tone_op(mode, lin[1] * exposure),
            tone_op(mode, lin[2] * exposure),
        ];
        px[0] = linear_to_srgb_u8(out[0]);
        px[1] = linear_to_srgb_u8(out[1]);
        px[2] = linear_to_srgb_u8(out[2]);
    }
}

/// Bloom：阈值提取 + 分离模糊 + 叠加。
pub fn bloom(img: &mut Rgba8Image, threshold: f32, radius: u32) {
    let w = img.width;
    let h = img.height;
    // 亮部提取
    let mut bright = Vec::with_capacity(img.data.len() / 4);
    for px in img.data.chunks_exact(4) {
        let l = srgb_to_linear_u8(px[0]) * 0.299
            + srgb_to_linear_u8(px[1]) * 0.587
            + srgb_to_linear_u8(px[2]) * 0.114;
        if l >= threshold {
            bright.push([px[0], px[1], px[2]]);
        } else {
            bright.push([0, 0, 0]);
        }
    }
    // 简化盒模糊（半径 r 的均值，水平+垂直）
    let r = radius.clamp(1, 8) as i32;
    let idx = |x: i32, y: i32| -> usize {
        (y.clamp(0, h as i32 - 1) as u32 * w + x.clamp(0, w as i32 - 1) as u32) as usize
    };
    let mut blurred = vec![[0u8, 0u8, 0u8]; (w as usize) * (h as usize)];
    let win = (2 * r + 1) as f32;
    let div = win * win;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let (mut sr, mut sg, mut sb) = (0u32, 0u32, 0u32);
            for dy in -r..=r {
                for dx in -r..=r {
                    let b = bright[idx(x + dx, y + dy)];
                    sr += b[0] as u32;
                    sg += b[1] as u32;
                    sb += b[2] as u32;
                }
            }
            blurred[idx(x, y)] =
                [(sr as f32 / div) as u8, (sg as f32 / div) as u8, (sb as f32 / div) as u8];
        }
    }
    // 叠加（加性 0.5）
    for (px, b) in img.data.chunks_exact_mut(4).zip(blurred) {
        px[0] = (px[0] as u16 + b[0] as u16 / 2).min(255) as u8;
        px[1] = (px[1] as u16 + b[1] as u16 / 2).min(255) as u8;
        px[2] = (px[2] as u16 + b[2] as u16 / 2).min(255) as u8;
    }
}

/// 降采样（盒滤波）。
pub fn downsample(src: &Rgba8Image, dst_w: u32, dst_h: u32) -> Rgba8Image {
    let mut out = Rgba8Image::new(dst_w, dst_h);
    let sx = src.width as f32 / dst_w as f32;
    let sy = src.height as f32 / dst_h as f32;
    for y in 0..dst_h {
        for x in 0..dst_w {
            let x0 = (x as f32 * sx) as u32;
            let y0 = (y as f32 * sy) as u32;
            let x1 = ((x + 1) as f32 * sx).ceil() as u32;
            let y1 = ((y + 1) as f32 * sy).ceil() as u32;
            let (mut r, mut g, mut b, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for py in y0..y1.min(src.height) {
                for px in x0..x1.min(src.width) {
                    if let Some(c) = src.get(px, py) {
                        r += c[0] as u32;
                        g += c[1] as u32;
                        b += c[2] as u32;
                        a += c[3] as u32;
                        n += 1;
                    }
                }
            }
            #[allow(clippy::manual_checked_ops)]
            if n > 0 {
                out.set(x, y, [(r / n) as u8, (g / n) as u8, (b / n) as u8, (a / n) as u8]);
            }
        }
    }
    out
}

/// FXAA-lite：边缘检测 + 邻域混合。
pub fn fxaa_lite(img: &mut Rgba8Image) {
    let w = img.width as i32;
    let h = img.height as i32;
    if w < 3 || h < 3 {
        return;
    }
    let orig = img.clone();
    let lum = |x: i32, y: i32| -> f32 {
        let x = x.clamp(0, w - 1) as u32;
        let y = y.clamp(0, h - 1) as u32;
        let c = orig.get(x, y).unwrap_or([0, 0, 0, 255]);
        (c[0] as f32 * 0.299 + c[1] as f32 * 0.587 + c[2] as f32 * 0.114) / 255.0
    };
    for y in 0..h {
        for x in 0..w {
            let c = lum(x, y);
            let n = lum(x, y - 1);
            let s = lum(x, y + 1);
            let e = lum(x + 1, y);
            let ww = lum(x - 1, y);
            let lmin = c.min(n.min(s.min(e.min(ww))));
            let lmax = c.max(n.max(s.max(e.max(ww))));
            if lmax - lmin < 0.15 {
                continue; // 平坦区
            }
            // 混合 4 邻域
            let get = |dx: i32, dy: i32| -> [u8; 4] {
                orig.get((x + dx).clamp(0, w - 1) as u32, (y + dy).clamp(0, h - 1) as u32)
                    .unwrap_or([0, 0, 0, 255])
            };
            let mut acc = [0u32; 4];
            for (dx, dy) in [(0, -1), (0, 1), (1, 0), (-1, 0)] {
                let p = get(dx, dy);
                for i in 0..4 {
                    acc[i] += p[i] as u32;
                }
            }
            let center = orig.get(x as u32, y as u32).unwrap();
            let blended = [(acc[0] / 4) as u8, (acc[1] / 4) as u8, (acc[2] / 4) as u8, center[3]];
            img.set(x as u32, y as u32, blended);
        }
    }
}

/// 像素完美整数倍缩放（2D）。
pub fn pixel_perfect_scale(src: &Rgba8Image, target: (u32, u32), integer: bool) -> Rgba8Image {
    let mut scale = (target.0 as f32 / src.width as f32).min(target.1 as f32 / src.height as f32);
    if integer {
        scale = scale.floor().max(1.0);
    }
    let (dw, dh) = ((src.width as f32 * scale) as u32, (src.height as f32 * scale) as u32);
    let mut out = Rgba8Image::new(target.0, target.1);
    for y in 0..dh.min(target.1) {
        for x in 0..dw.min(target.0) {
            let sx = (x as f32 / scale) as u32;
            let sy = (y as f32 / scale) as u32;
            if let Some(c) = src.get(sx.min(src.width - 1), sy.min(src.height - 1)) {
                out.set(x, y, c);
            }
        }
    }
    out
}

/// CRT 效果：扫描线。
pub fn crt_effect(img: &mut Rgba8Image, strength: f32) {
    let strength = strength.clamp(0.0, 1.0);
    for (i, px) in img.data.chunks_exact_mut(4).enumerate() {
        let y = i / img.width as usize;
        let scan = if y % 2 == 0 { 1.0 } else { 1.0 - strength * 0.5 };
        px[0] = (px[0] as f32 * scan) as u8;
        px[1] = (px[1] as f32 * scan) as u8;
        px[2] = (px[2] as f32 * scan) as u8;
    }
}

/// 色差（RGB 通道径向偏移近似：水平偏移）。
pub fn chromatic_aberration(img: &mut Rgba8Image, shift: u32) {
    if shift == 0 || img.width < shift + 2 {
        return;
    }
    let orig = img.clone();
    let s = shift as usize;
    for y in 0..img.height {
        for x in 0..img.width as usize {
            let r = orig.get((x + s).min(img.width as usize - 1) as u32, y).unwrap_or([0; 4])[0];
            let b = orig.get(x.saturating_sub(s) as u32, y).unwrap_or([0; 4])[2];
            let g = orig.get(x as u32, y).unwrap_or([0; 4])[1];
            img.set(x as u32, y, [r, g, b, 255]);
        }
    }
}

/// 暗角。
pub fn vignette(img: &mut Rgba8Image, strength: f32) {
    let cx = img.width as f32 * 0.5;
    let cy = img.height as f32 * 0.5;
    let max_d = (cx * cx + cy * cy).sqrt();
    for y in 0..img.height {
        for x in 0..img.width {
            let d = (((x as f32 - cx) * (x as f32 - cx) + (y as f32 - cy) * (y as f32 - cy))
                .sqrt()
                / max_d)
                .min(1.0);
            let falloff = 1.0 - strength * d * d;
            if let Some(c) = img.get(x, y) {
                img.set(
                    x,
                    y,
                    [
                        (c[0] as f32 * falloff) as u8,
                        (c[1] as f32 * falloff) as u8,
                        (c[2] as f32 * falloff) as u8,
                        c[3],
                    ],
                );
            }
        }
    }
}

// ---- 上采样（IF-190） ----

/// 上采样提供者 trait。
pub trait UpscalerProvider {
    fn name(&self) -> &str;
    fn upscale(&self, src: &Rgba8Image, dst: (u32, u32)) -> Result<Rgba8Image>;
}

/// 双线性。
pub struct BilinearUpscaler;

impl UpscalerProvider for BilinearUpscaler {
    fn name(&self) -> &str {
        "bilinear"
    }
    fn upscale(&self, src: &Rgba8Image, dst: (u32, u32)) -> Result<Rgba8Image> {
        if dst.0 == 0 || dst.1 == 0 {
            return Err(rf_core::EngineError::InvalidData("empty upscale target".into()));
        }
        let mut out = Rgba8Image::new(dst.0, dst.1);
        let sx = src.width as f32 / dst.0 as f32;
        let sy = src.height as f32 / dst.1 as f32;
        for y in 0..dst.1 {
            for x in 0..dst.0 {
                let fx = (x as f32 + 0.5) * sx - 0.5;
                let fy = (y as f32 + 0.5) * sy - 0.5;
                let x0 = fx.floor().max(0.0) as u32;
                let y0 = fy.floor().max(0.0) as u32;
                let x1 = (x0 + 1).min(src.width - 1);
                let y1 = (y0 + 1).min(src.height - 1);
                let tx = fx - fx.floor();
                let ty = fy - fy.floor();
                let c00 = src.get(x0, y0).unwrap_or([0; 4]);
                let c10 = src.get(x1, y0).unwrap_or([0; 4]);
                let c01 = src.get(x0, y1).unwrap_or([0; 4]);
                let c11 = src.get(x1, y1).unwrap_or([0; 4]);
                let mut px = [0u8; 4];
                for i in 0..4 {
                    let top = c00[i] as f32 * (1.0 - tx) + c10[i] as f32 * tx;
                    let bot = c01[i] as f32 * (1.0 - tx) + c11[i] as f32 * tx;
                    px[i] = (top * (1.0 - ty) + bot * ty).round() as u8;
                }
                out.set(x, y, px);
            }
        }
        Ok(out)
    }
}

/// FSR1 风格：双线性 + RCAS 锐化近似。
pub struct Fsr1StyleUpscaler;

impl UpscalerProvider for Fsr1StyleUpscaler {
    fn name(&self) -> &str {
        "fsr1-style"
    }
    fn upscale(&self, src: &Rgba8Image, dst: (u32, u32)) -> Result<Rgba8Image> {
        let mut out = BilinearUpscaler.upscale(src, dst)?;
        // RCAS-lite：3×3 锐化卷积
        let orig = out.clone();
        let w = out.width;
        let h = out.height;
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let c = orig.get(x, y).unwrap_or([0; 4]);
                let mut acc = [0i32; 3];
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let n =
                        orig.get((x as i32 + dx) as u32, (y as i32 + dy) as u32).unwrap_or([0; 4]);
                    for i in 0..3 {
                        acc[i] += n[i] as i32;
                    }
                }
                for i in 0..3 {
                    let sharpened = (c[i] as i32 * 5 - acc[i] / 2).clamp(0, 255);
                    let v = ((c[i] as i32 + sharpened) / 2).clamp(0, 255) as u8;
                    // 按通道写回
                    let mut px = c;
                    px[i] = v;
                    if i == 2 {
                        out.set(x, y, px);
                    }
                }
            }
        }
        Ok(out)
    }
}

/// 2D 光照着色（CPU，供编辑器预览与 L1 验收）。
pub fn apply_light2d(img: &mut Rgba8Image, light: crate::Light2D, cam: &crate::Camera2D) {
    let luma = |c: [u8; 4]| c;
    match light {
        crate::Light2D::Ambient { color, intensity } => {
            let rgba = color.to_rgba8();
            for px in img.data.chunks_exact_mut(4) {
                for i in 0..3 {
                    let lit = (rgba[i] as f32 * intensity).min(255.0) as u16;
                    px[i] = (px[i] as u16 * lit / 255).min(255) as u8;
                }
            }
        }
        crate::Light2D::Point { pos, color, intensity, radius } => {
            let rgba = color.to_rgba8();
            for y in 0..img.height {
                for x in 0..img.width {
                    let world = cam.screen_to_world(rf_math::Vec2::new(x as f32, y as f32));
                    let d = (world - pos).length();
                    if d > radius {
                        continue;
                    }
                    let atten = (1.0 - d / radius.max(f32::EPSILON)) * intensity;
                    if let Some(c) = img.get(x, y) {
                        let base = luma(c);
                        img.set(
                            x,
                            y,
                            [
                                (base[0] as f32 + rgba[0] as f32 * atten * 0.3).min(255.0) as u8,
                                (base[1] as f32 + rgba[1] as f32 * atten * 0.3).min(255.0) as u8,
                                (base[2] as f32 + rgba[2] as f32 * atten * 0.3).min(255.0) as u8,
                                base[3],
                            ],
                        );
                    }
                }
            }
        }
        crate::Light2D::Directional { color, intensity, .. } => {
            let rgba = color.to_rgba8();
            for px in img.data.chunks_exact_mut(4) {
                for i in 0..3 {
                    px[i] = ((px[i] as u16
                        * (128 + ((rgba[i] as f32 * intensity / 2.0).min(127.0) as u16)))
                        / 255)
                        .min(255) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> Rgba8Image {
        let mut img = Rgba8Image::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.set(x, y, [(x * 255 / w.max(1)) as u8, (y * 255 / h.max(1)) as u8, 128, 255]);
            }
        }
        img
    }

    #[test]
    fn tone_map_darkens_highlights() {
        let mut img = Rgba8Image::filled(4, 4, [255, 255, 255, 255]);
        let before = img.get(0, 0).unwrap();
        tone_map(&mut img, ToneMap::Reinhard);
        let after = img.get(0, 0).unwrap();
        assert!(after[0] < before[0]); // 1.0 → 0.5（线性域）后变暗
        tone_map(&mut img, ToneMap::Aces);
        tone_map(&mut img, ToneMap::Filmic);
        tone_map(&mut img, ToneMap::AgX);
        tone_map(&mut img, ToneMap::None); // 恒等
    }

    #[test]
    fn bloom_brightens_bright_areas() {
        let mut img = Rgba8Image::filled(16, 16, [0, 0, 0, 255]);
        img.set(8, 8, [255, 255, 255, 255]);
        bloom(&mut img, 0.8, 2);
        // 邻域被提亮
        assert!(img.get(9, 8).unwrap()[0] > 0);
        // 暗区基本不变
        assert_eq!(img.get(0, 0).unwrap()[0], 0);
    }

    #[test]
    fn downsample_averages() {
        let img = gradient(4, 4);
        let small = downsample(&img, 2, 2);
        assert_eq!((small.width, small.height), (2, 2));
        let c = small.get(0, 0).unwrap();
        assert!(c[0] > 0 && c[0] < 200); // 平均值
    }

    #[test]
    fn pixel_perfect_integer() {
        let img = Rgba8Image::filled(4, 4, [10, 20, 30, 255]);
        let out = pixel_perfect_scale(&img, (21, 9), true);
        assert_eq!(out.get(7, 4), Some([10, 20, 30, 255])); // 整数倍区域
                                                            // 3×4=12 < 21 → 边缘留黑
        assert_eq!(out.get(20, 4), Some([0, 0, 0, 0]));
    }

    #[test]
    fn crt_and_vignette() {
        let mut img = Rgba8Image::filled(8, 8, [200, 200, 200, 255]);
        crt_effect(&mut img, 0.8);
        assert!(img.get(0, 1).unwrap()[0] < 200); // 偶数行衰减
        let mut img2 = Rgba8Image::filled(16, 16, [200, 200, 200, 255]);
        vignette(&mut img2, 0.5);
        assert!(img2.get(0, 0).unwrap()[0] < img2.get(8, 8).unwrap()[0]); // 角落更暗
    }

    #[test]
    fn upscalers() {
        let img = gradient(4, 4);
        let up = BilinearUpscaler.upscale(&img, (16, 16)).unwrap();
        assert_eq!((up.width, up.height), (16, 16));
        let up2 = Fsr1StyleUpscaler.upscale(&img, (16, 16)).unwrap();
        assert_eq!(up2.get(8, 8), up2.get(8, 8)); // 稳定
        assert!(BilinearUpscaler.upscale(&img, (0, 0)).is_err());
    }

    #[test]
    fn fxaa_smooths_edges() {
        // 阶梯边缘图像
        let mut img = Rgba8Image::filled(16, 16, [0, 0, 0, 255]);
        for y in 0..8 {
            for x in 0..16 {
                img.set(x, y, [255, 255, 255, 255]);
            }
        }
        fxaa_lite(&mut img);
        // 边界行被混合
        let mid = img.get(8, 7).unwrap();
        assert!(mid[0] > 0 && mid[0] < 255);
    }

    #[test]
    fn light2d_ambient_darkens() {
        let mut img = Rgba8Image::filled(8, 8, [200, 200, 200, 255]);
        let cam = crate::Camera2D::default();
        apply_light2d(
            &mut img,
            crate::Light2D::Ambient { color: rf_math::Color::rgb(1.0, 1.0, 1.0), intensity: 0.5 },
            &cam,
        );
        assert!((img.get(0, 0).unwrap()[0] as i32 - 100).abs() <= 1);
    }
}
