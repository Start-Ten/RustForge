//! 内置导入器（IF-170）：图像/模型/音频/数据/2D 专用格式 + P1/P2 桩。

use crate::*;
use rf_audio::AudioAsset;
use rf_core::{EngineError, Result, Rgba8Image};
use std::collections::HashMap;

fn texture_imported(image: Rgba8Image, ctx: &mut ImportContext) -> Result<ImportedAsset> {
    let (warnings, deps) = ctx.finish();
    Ok(ImportedAsset {
        kind_name: "TextureAsset",
        data: Box::new(TextureAsset { image, srgb: true, premultiplied: false }),
        dependencies: deps,
        warnings,
    })
}

// ---- QOI ----

/// QOI 解码（规格 O1 P0）。
pub struct QoiImporter;

const QOI_MAGIC: [u8; 4] = *b"qoif";
/// PNG 签名（89 50 4E 47 0D 0A 1A 0A）。
const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

impl AssetImporter for QoiImporter {
    fn name(&self) -> &'static str {
        "qoi"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["qoi"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        if bytes.len() < 14 || bytes[0..4] != QOI_MAGIC {
            return Err(EngineError::InvalidData("qoi: bad magic".into()));
        }
        let w = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let h = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let channels = bytes[12];
        if w == 0 || h == 0 || (channels != 3 && channels != 4) {
            return Err(EngineError::InvalidData("qoi: bad header".into()));
        }
        let mut image = Rgba8Image::new(w, h);
        let mut px = [0u8; 4];
        let mut index = [[0u8; 4]; 64];
        let mut pos = 14usize;
        let mut i = 0usize;
        let total = w as usize * h as usize;
        while i < total && pos < bytes.len() {
            let b1 = bytes[pos];
            pos += 1;
            if b1 == 0xFE {
                // QOI_RGB
                px[0] = bytes[pos];
                px[1] = bytes[pos + 1];
                px[2] = bytes[pos + 2];
                px[3] = 255;
                pos += 3;
            } else if b1 == 0xFF {
                // QOI_RGBA
                px.copy_from_slice(&bytes[pos..pos + 4]);
                pos += 4;
            } else if b1 >> 6 == 0 {
                px = index[(b1 & 0x3F) as usize];
            } else if b1 >> 6 == 1 {
                // diff
                let dr = ((b1 >> 4) & 0x03) as i32 - 2;
                let dg = ((b1 >> 2) & 0x03) as i32 - 2;
                let db = (b1 & 0x03) as i32 - 2;
                px[0] = px[0].wrapping_add(dr as u8);
                px[1] = px[1].wrapping_add(dg as u8);
                px[2] = px[2].wrapping_add(db as u8);
            } else if b1 >> 6 == 2 {
                // luma
                let b2 = bytes[pos];
                pos += 1;
                let dg = (b2 as i32) - 32;
                let dr = dg + ((b1 >> 4) & 0x0F) as i32 - 8;
                let db = dg + (b1 & 0x0F) as i32 - 8;
                px[0] = px[0].wrapping_add(dr as u8);
                px[1] = px[1].wrapping_add(dg as u8);
                px[2] = px[2].wrapping_add(db as u8);
            } else if b1 >> 6 == 3 {
                // run
                let run = (b1 & 0x3F) as usize + 1;
                for _ in 0..run.min(total - i) {
                    image.data[i * 4..i * 4 + 4].copy_from_slice(&px);
                    i += 1;
                }
                continue;
            }
            if b1 >> 6 != 0 {
                let hash = (px[0] as usize * 3
                    + px[1] as usize * 5
                    + px[2] as usize * 7
                    + px[3] as usize * 11)
                    % 64;
                index[hash] = px;
            }
            image.data[i * 4..i * 4 + 4].copy_from_slice(&px);
            i += 1;
        }
        if i < total {
            return Err(EngineError::InvalidData("qoi: truncated".into()));
        }
        texture_imported(image, ctx)
    }
}

/// 构造一张最小 QOI 字节流（测试用）。
pub fn encode_qoi_test(image: &Rgba8Image) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&QOI_MAGIC);
    out.extend_from_slice(&image.width.to_be_bytes());
    out.extend_from_slice(&image.height.to_be_bytes());
    out.push(4);
    out.push(0);
    // 每像素 QOI_RGB（0xFE + RGB；alpha 隐含 255）
    for px in image.data.chunks_exact(4) {
        out.push(0xFE);
        out.extend_from_slice(&px[0..3]);
    }
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    out
}

// ---- PNG（flate2 inflate）----

/// PNG 导入（8-bit RGB/RGBA/灰度/调色板，非隔行）。
pub struct PngImporter;

impl AssetImporter for PngImporter {
    fn name(&self) -> &'static str {
        "png"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        if bytes.len() < 8 || bytes[0..8] != PNG_SIG {
            return Err(EngineError::InvalidData("png: bad signature".into()));
        }
        let mut pos = 8usize;
        let mut width = 0u32;
        let mut height = 0u32;
        let mut bit_depth = 8u8;
        let mut color_type = 6u8;
        let mut palette: Vec<[u8; 3]> = Vec::new();
        let mut idat: Vec<u8> = Vec::new();
        let mut seen_ihdr = false;
        while pos + 8 <= bytes.len() {
            let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
            let ctype = &bytes[pos + 4..pos + 8];
            let body = &bytes[pos + 8..(pos + 8 + len).min(bytes.len())];
            match ctype {
                b"IHDR" => {
                    if body.len() < 13 {
                        return Err(EngineError::InvalidData("png: short IHDR".into()));
                    }
                    width = u32::from_be_bytes(body[0..4].try_into().unwrap());
                    height = u32::from_be_bytes(body[4..8].try_into().unwrap());
                    bit_depth = body[8];
                    color_type = body[9];
                    if body[12] != 0 {
                        return Err(EngineError::NotYetSupported {
                            what: "png interlacing",
                            priority: "P1",
                        });
                    }
                    seen_ihdr = true;
                }
                b"PLTE" => {
                    palette = body.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                }
                b"IDAT" => idat.extend_from_slice(body),
                b"IEND" => break,
                _ => {}
            }
            pos += 12 + len;
        }
        if !seen_ihdr || width == 0 || height == 0 {
            return Err(EngineError::InvalidData("png: missing IHDR".into()));
        }
        if bit_depth != 8 {
            return Err(EngineError::NotYetSupported {
                what: "png bit depth != 8",
                priority: "P1",
            });
        }
        let channels: usize = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => return Err(EngineError::InvalidData("png: bad color type".into())),
        };
        // inflate
        let mut inflater = flate2::read::ZlibDecoder::new(&idat[..]);
        let mut raw = Vec::new();
        std::io::Read::read_to_end(&mut inflater, &mut raw)
            .map_err(|e| EngineError::InvalidData(format!("png: inflate {e}")))?;
        // unfilter
        let stride = width as usize * channels;
        let bpp = channels;
        let mut prev = vec![0u8; stride];
        let mut out = vec![0u8; stride];
        let mut image = Rgba8Image::new(width, height);
        let mut src = 0usize;
        for y in 0..height as usize {
            if src >= raw.len() {
                return Err(EngineError::InvalidData("png: truncated data".into()));
            }
            let filter = raw[src];
            src += 1;
            if src + stride > raw.len() {
                return Err(EngineError::InvalidData("png: short row".into()));
            }
            let row = &raw[src..src + stride];
            src += stride;
            for x in 0..stride {
                let a = if x >= bpp { out[x - bpp] } else { 0 };
                let b = prev[x];
                let c = if x >= bpp { prev[x - bpp] } else { 0 };
                out[x] = match filter {
                    0 => row[x],
                    1 => row[x].wrapping_add(a),
                    2 => row[x].wrapping_add(b),
                    3 => row[x].wrapping_add(((a as u16 + b as u16) / 2) as u8),
                    4 => {
                        let p = a as i16 + b as i16 - c as i16;
                        let pa = (p - a as i16).abs();
                        let pb = (p - b as i16).abs();
                        let pc = (p - c as i16).abs();
                        let pred = if pa <= pb && pa <= pc {
                            a
                        } else if pb <= pc {
                            b
                        } else {
                            c
                        };
                        row[x].wrapping_add(pred)
                    }
                    _ => return Err(EngineError::InvalidData("png: bad filter".into())),
                };
            }
            for x in 0..width as usize {
                let i = x * channels;
                let rgba: [u8; 4] = match color_type {
                    0 => [out[i], out[i], out[i], 255],
                    2 => [out[i], out[i + 1], out[i + 2], 255],
                    3 => {
                        let idx = out[i] as usize;
                        let c = palette.get(idx).copied().unwrap_or([255, 0, 255]);
                        [c[0], c[1], c[2], 255]
                    }
                    4 => [out[i], out[i], out[i], out[i + 1]],
                    _ => [out[i], out[i + 1], out[i + 2], out[i + 3]],
                };
                image.set(x as u32, y as u32, rgba);
            }
            prev.copy_from_slice(&out);
        }
        texture_imported(image, ctx)
    }
}

// ---- BMP / TGA / PNM（简化解码）----

pub struct BmpImporter;

impl AssetImporter for BmpImporter {
    fn name(&self) -> &'static str {
        "bmp"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["bmp"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Err(EngineError::InvalidData("bmp: bad header".into()));
        }
        let offset = u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize;
        let w = i32::from_le_bytes(bytes[18..22].try_into().unwrap());
        let h_abs = i32::from_le_bytes(bytes[22..26].try_into().unwrap()).abs();
        let bpp = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
        if bpp != 24 && bpp != 32 {
            return Err(EngineError::NotYetSupported { what: "bmp non 24/32bpp", priority: "P1" });
        }
        let bypp = (bpp / 8) as usize;
        let stride = (w as usize * bypp).div_ceil(4) * 4;
        let bottom_up = i32::from_le_bytes(bytes[22..26].try_into().unwrap()) > 0;
        let mut image = Rgba8Image::new(w as u32, h_abs as u32);
        for y in 0..h_abs as usize {
            let src_y = if bottom_up { h_abs as usize - 1 - y } else { y };
            let row_start = offset + src_y * stride;
            if row_start + w as usize * bypp > bytes.len() {
                return Err(EngineError::InvalidData("bmp: truncated".into()));
            }
            for x in 0..w as usize {
                let i = row_start + x * bypp;
                let rgba = if bpp == 24 {
                    [bytes[i + 2], bytes[i + 1], bytes[i], 255]
                } else {
                    [bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]
                };
                image.set(x as u32, y as u32, rgba);
            }
        }
        texture_imported(image, ctx)
    }
}

pub struct TgaImporter;

impl AssetImporter for TgaImporter {
    fn name(&self) -> &'static str {
        "tga"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["tga"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        if bytes.len() < 18 {
            return Err(EngineError::InvalidData("tga: short header".into()));
        }
        let id_len = bytes[0] as usize;
        let cmap_type = bytes[1];
        let img_type = bytes[2];
        let w = u16::from_le_bytes(bytes[12..14].try_into().unwrap()) as u32;
        let h = u16::from_le_bytes(bytes[14..16].try_into().unwrap()) as u32;
        let bpp = bytes[16];
        let top_down = bytes[17] & 0x20 != 0;
        if w == 0 || h == 0 {
            return Err(EngineError::InvalidData("tga: zero size".into()));
        }
        let mut pos = 18 + id_len;
        // 调色板
        let mut palette: Vec<[u8; 4]> = Vec::new();
        if cmap_type == 1 {
            let cmap_len = u16::from_le_bytes(bytes[5..7].try_into().unwrap()) as usize;
            let cmap_bpp = bytes[7] as usize;
            let entry = cmap_bpp / 8;
            for i in 0..cmap_len {
                let o = pos + i * entry;
                if o + entry > bytes.len() {
                    return Err(EngineError::InvalidData("tga: palette oob".into()));
                }
                palette.push(match entry {
                    3 => [bytes[o], bytes[o + 1], bytes[o + 2], 255],
                    4 => [bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]],
                    _ => [bytes[o], bytes[o], bytes[o], 255],
                });
            }
            pos += cmap_len * entry;
        }
        let bypp = (bpp / 8) as usize;
        let rle = img_type >= 9;
        let indexed = img_type == 1 || img_type == 9;
        let mut image = Rgba8Image::new(w, h);
        let read_pixel = |pos: usize| -> Result<[u8; 4]> {
            if indexed {
                let idx = bytes.get(pos).copied().unwrap_or(0) as usize;
                Ok(palette.get(idx).copied().unwrap_or([255, 0, 255, 255]))
            } else {
                let o = bytes
                    .get(pos..pos + bypp)
                    .ok_or_else(|| EngineError::InvalidData("tga: truncated".into()))?;
                Ok(match bypp {
                    1 => [o[0], o[0], o[0], 255],
                    3 => [o[2], o[1], o[0], 255], // BGR
                    _ => [o[2], o[1], o[0], o[3]],
                })
            }
        };
        let total = w as usize * h as usize;
        let mut i = 0usize;
        while i < total {
            if rle {
                if pos >= bytes.len() {
                    return Err(EngineError::InvalidData("tga: truncated rle".into()));
                }
                let packet = bytes[pos];
                pos += 1;
                let count = (packet & 0x7F) as usize + 1;
                if packet & 0x80 != 0 {
                    let px = read_pixel(pos)?;
                    pos += if indexed { 1 } else { bypp };
                    for _ in 0..count {
                        if i < total {
                            image.data[i * 4..i * 4 + 4].copy_from_slice(&px);
                            i += 1;
                        }
                    }
                } else {
                    for _ in 0..count {
                        let px = read_pixel(pos)?;
                        pos += if indexed { 1 } else { bypp };
                        if i < total {
                            image.data[i * 4..i * 4 + 4].copy_from_slice(&px);
                            i += 1;
                        }
                    }
                }
            } else {
                let px = read_pixel(pos)?;
                pos += if indexed { 1 } else { bypp };
                image.data[i * 4..i * 4 + 4].copy_from_slice(&px);
                i += 1;
            }
        }
        // 原点翻转
        if !top_down {
            for y in 0..h as usize / 2 {
                for x in 0..w as usize {
                    let a = (y * w as usize + x) * 4;
                    let b = ((h as usize - 1 - y) * w as usize + x) * 4;
                    for k in 0..4 {
                        image.data.swap(a + k, b + k);
                    }
                }
            }
        }
        texture_imported(image, ctx)
    }
}

pub struct PnmImporter;

impl AssetImporter for PnmImporter {
    fn name(&self) -> &'static str {
        "pnm"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["pnm", "pgm", "ppm"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let text = std::ffi::CString::new(bytes.to_vec())
            .map_err(|_| EngineError::InvalidData("pnm: NUL byte".into()))?;
        let s = text.to_string_lossy();
        let mut parts = s.split_whitespace();
        let magic = parts.next().ok_or_else(|| EngineError::InvalidData("pnm: empty".into()))?;
        let (w, h, maxval): (u32, u32, u32) = match magic {
            "P5" | "P6" => {
                let w: u32 = parts
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| EngineError::InvalidData("pnm: bad header".into()))?;
                let h: u32 = parts
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| EngineError::InvalidData("pnm: bad header".into()))?;
                let m: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(255);
                (w, h, m)
            }
            _ => {
                return Err(EngineError::NotYetSupported {
                    what: "pnm ascii variants",
                    priority: "P2",
                })
            }
        };
        let channels = if magic == "P6" { 3 } else { 1 };
        // 头部后单个空白分隔
        let header_end = {
            let mut idx = 0;
            let mut fields = 0;
            let mut in_field = false;
            let mut done = false;
            while idx < bytes.len() && !done {
                let ws = bytes[idx].is_ascii_whitespace();
                if !ws && !in_field {
                    fields += 1;
                    in_field = true;
                } else if ws {
                    in_field = false;
                    if fields == 4 {
                        done = true;
                    }
                }
                idx += 1;
            }
            idx.min(bytes.len())
        };
        let data = &bytes[header_end..];
        let expect = w as usize * h as usize * channels;
        if data.len() < expect {
            return Err(EngineError::InvalidData("pnm: truncated".into()));
        }
        let mut image = Rgba8Image::new(w, h);
        let scale = if maxval != 255 { 255.0 / maxval as f32 } else { 1.0 };
        for (i, px) in image.data.chunks_exact_mut(4).enumerate() {
            let o = i * channels;
            match channels {
                1 => {
                    let v = (data[o] as f32 * scale) as u8;
                    px[0] = v;
                    px[1] = v;
                    px[2] = v;
                }
                _ => {
                    px[0] = (data[o] as f32 * scale) as u8;
                    px[1] = (data[o + 1] as f32 * scale) as u8;
                    px[2] = (data[o + 2] as f32 * scale) as u8;
                }
            }
            px[3] = 255;
        }
        texture_imported(image, ctx)
    }
}

// ---- OBJ ----

pub struct ObjImporter;

impl AssetImporter for ObjImporter {
    fn name(&self) -> &'static str {
        "obj"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["obj"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let text = String::from_utf8_lossy(bytes);
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut out_pos: Vec<[f32; 3]> = Vec::new();
        let mut out_uv: Vec<[f32; 2]> = Vec::new();
        let mut out_n: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut cache: HashMap<String, u32> = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            match it.next() {
                Some("v") => {
                    let p: Vec<f32> = it.filter_map(|t| t.parse().ok()).collect();
                    positions.push([
                        *p.first().unwrap_or(&0.0),
                        *p.get(1).unwrap_or(&0.0),
                        *p.get(2).unwrap_or(&0.0),
                    ]);
                }
                Some("vt") => {
                    let t: Vec<f32> = it.filter_map(|x| x.parse().ok()).collect();
                    uvs.push([*t.first().unwrap_or(&0.0), *t.get(1).unwrap_or(&0.0)]);
                }
                Some("vn") => {
                    let n: Vec<f32> = it.filter_map(|x| x.parse().ok()).collect();
                    normals.push([
                        *n.first().unwrap_or(&0.0),
                        *n.get(1).unwrap_or(&0.0),
                        *n.get(2).unwrap_or(&0.0),
                    ]);
                }
                Some("f") => {
                    // 面扇形三角化
                    let verts: Vec<&str> = it.collect();
                    for k in 1..verts.len().saturating_sub(1) {
                        for vi in [0, k, k + 1] {
                            let key = verts[vi].to_string();
                            let idx = *cache.entry(key.clone()).or_insert_with(|| {
                                let mut lit = key.split('/');
                                let pi: usize =
                                    lit.next().and_then(|t| t.parse::<usize>().ok()).unwrap_or(1);
                                let ti: Option<usize> = lit.next().and_then(|t| {
                                    if t.is_empty() {
                                        None
                                    } else {
                                        t.parse().ok()
                                    }
                                });
                                let ni: Option<usize> = lit.next().and_then(|t| t.parse().ok());
                                let p = positions
                                    .get(pi.saturating_sub(1))
                                    .copied()
                                    .unwrap_or([0.0; 3]);
                                let uv = ti
                                    .and_then(|i| uvs.get(i.saturating_sub(1)).copied())
                                    .unwrap_or([0.0, 0.0]);
                                let n = ni
                                    .and_then(|i| normals.get(i.saturating_sub(1)).copied())
                                    .unwrap_or([0.0, 0.0, 1.0]);
                                let new_idx = out_pos.len() as u32;
                                out_pos.push(p);
                                out_uv.push(uv);
                                out_n.push(n);
                                new_idx
                            });
                            indices.push(idx);
                        }
                    }
                }
                _ => {}
            }
        }
        if indices.is_empty() {
            ctx.warn("obj: no faces");
        }
        let (warnings, deps) = ctx.finish();
        Ok(ImportedAsset {
            kind_name: "MeshAsset",
            data: Box::new(MeshAsset {
                positions: out_pos,
                normals: out_n,
                uvs: out_uv,
                indices,
                material: None,
            }),
            dependencies: deps,
            warnings,
        })
    }
}

// ---- WAV ----

pub struct WavImporter;

impl AssetImporter for WavImporter {
    fn name(&self) -> &'static str {
        "wav"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["wav"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let (samples, rate) = rf_audio::decode_wav(bytes)?;
        let (warnings, deps) = ctx.finish();
        Ok(ImportedAsset {
            kind_name: "AudioAsset",
            data: Box::new(AudioAsset { samples, channels: 2, sample_rate: rate }),
            dependencies: deps,
            warnings,
        })
    }
}

// ---- JSON / TOML-mini / CSV / SpriteSheet / Font ----

pub struct JsonImporter;

impl AssetImporter for JsonImporter {
    fn name(&self) -> &'static str {
        "json"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let v: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|e| EngineError::InvalidData(format!("json: {e}")))?;
        // TexturePacker / Aseprite JSON → 精灵表
        if v.get("frames").is_some()
            && (v.get("meta").is_some() || v["frames"].is_object() || v["frames"].is_array())
        {
            let tex_ref = v["meta"]["image"].as_str().unwrap_or("atlas.png");
            ctx.add_dep(tex_ref);
            let mut sprites = Vec::new();
            let mut animations: HashMap<String, Vec<String>> = HashMap::new();
            let mut frames = v["frames"].as_array().cloned().unwrap_or_default();
            if v["frames"].is_object() {
                frames = v["frames"]
                    .as_object()
                    .map(|m| m.values().cloned().collect())
                    .unwrap_or_default();
            }
            for f in &frames {
                let name = f["filename"]
                    .as_str()
                    .or_else(|| f["frame"]["filename"].as_str())
                    .unwrap_or("frame")
                    .to_string();
                let fr = &f["frame"];
                let (x, y, w, h) = (
                    fr["x"].as_f64().unwrap_or(0.0) as u32,
                    fr["y"].as_f64().unwrap_or(0.0) as u32,
                    fr["w"].as_f64().unwrap_or(0.0) as u32,
                    fr["h"].as_f64().unwrap_or(0.0) as u32,
                );
                let (pw, ph) = (
                    f["pivot"]["x"].as_f64().unwrap_or(0.5) as f32,
                    f["pivot"]["y"].as_f64().unwrap_or(0.5) as f32,
                );
                // Aseprite 帧名约定 name 001 → 动画 name
                let base =
                    name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.').to_string();
                let anim_key = if base.is_empty() { "default".to_string() } else { base };
                animations.entry(anim_key).or_default().push(name.clone());
                sprites.push(SpriteFrame { name, x, y, w, h, pivot: [pw, ph] });
            }
            let anims: Vec<SpriteAnimation> = animations
                .into_iter()
                .map(|(name, frames)| SpriteAnimation { name, frames, fps: 12.0, looping: true })
                .collect();
            let (warnings, deps) = ctx.finish();
            return Ok(ImportedAsset {
                kind_name: "SpriteSheetAsset",
                data: Box::new(SpriteSheetAsset {
                    texture: AssetId::from_path(tex_ref),
                    sprites,
                    animations: anims,
                }),
                dependencies: deps,
                warnings,
            });
        }
        // 位图字体 JSON {chars: {A: {x,y,w,h,advance}}, size, lineHeight}
        if v.get("chars").is_some() {
            let mut glyphs = HashMap::new();
            for (ch, g) in v["chars"].as_object().into_iter().flatten() {
                let c = ch.chars().next().unwrap_or('?');
                glyphs.insert(
                    c,
                    GlyphRect {
                        x: g["x"].as_f64().unwrap_or(0.0) as u32,
                        y: g["y"].as_f64().unwrap_or(0.0) as u32,
                        w: g["w"].as_f64().unwrap_or(8.0) as u32,
                        h: g["h"].as_f64().unwrap_or(8.0) as u32,
                        advance: g["advance"].as_f64().unwrap_or(8.0) as f32,
                    },
                );
            }
            let tex_ref = v["texture"].as_str().unwrap_or("font.png");
            ctx.add_dep(tex_ref);
            let (warnings, deps) = ctx.finish();
            return Ok(ImportedAsset {
                kind_name: "FontAsset",
                data: Box::new(FontAsset {
                    image: Rgba8Image::new(1, 1), // 图像由依赖加载方填充
                    glyphs,
                    size: v["size"].as_u64().unwrap_or(16) as u16,
                    line_height: v["lineHeight"].as_f64().unwrap_or(18.0) as f32,
                }),
                dependencies: deps,
                warnings,
            });
        }
        let (warnings, deps) = ctx.finish();
        Ok(ImportedAsset {
            kind_name: "DataAsset",
            data: Box::new(DataAsset(v)),
            dependencies: deps,
            warnings,
        })
    }
}

pub struct TomlImporter;

impl AssetImporter for TomlImporter {
    fn name(&self) -> &'static str {
        "toml"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["toml", "tmx"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let text = String::from_utf8_lossy(bytes);
        let mut root = serde_json::Map::new();
        let mut current = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current = line[1..line.len() - 1].to_string();
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().trim_matches('"');
                let raw = v.trim();
                let val: serde_json::Value = if let Some(s) =
                    raw.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                {
                    serde_json::Value::String(s.to_string())
                } else if raw == "true" {
                    serde_json::Value::Bool(true)
                } else if raw == "false" {
                    serde_json::Value::Bool(false)
                } else if let Ok(i) = raw.parse::<i64>() {
                    serde_json::Value::Number(i.into())
                } else if let Ok(f) = raw.parse::<f64>() {
                    serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into()))
                } else {
                    serde_json::Value::String(raw.to_string())
                };
                if current.is_empty() {
                    root.insert(key.to_string(), val);
                } else {
                    let section = root
                        .entry(current.clone())
                        .or_insert_with(|| serde_json::Value::Object(Default::default()));
                    if let Some(m) = section.as_object_mut() {
                        m.insert(key.to_string(), val);
                    }
                }
            }
        }
        let (warnings, deps) = ctx.finish();
        Ok(ImportedAsset {
            kind_name: "DataAsset",
            data: Box::new(DataAsset(serde_json::Value::Object(root))),
            dependencies: deps,
            warnings,
        })
    }
}

pub struct CsvImporter;

impl AssetImporter for CsvImporter {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["csv", "tsv"]
    }
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset> {
        let text = String::from_utf8_lossy(bytes);
        let sep = if ctx.source_path.ends_with(".tsv") { '\t' } else { ',' };
        let rows: Vec<serde_json::Value> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                serde_json::Value::Array(
                    l.split(sep).map(|c| serde_json::Value::String(c.trim().to_string())).collect(),
                )
            })
            .collect();
        let (warnings, deps) = ctx.finish();
        Ok(ImportedAsset {
            kind_name: "DataAsset",
            data: Box::new(DataAsset(serde_json::Value::Array(rows))),
            dependencies: deps,
            warnings,
        })
    }
}

/// 未支持格式的统一桩（IF-170：明确 NotYetSupported + 优先级）。
macro_rules! stub_importer {
    ($name:ident, $ext:expr, $what:expr, $prio:expr) => {
        pub struct $name;
        impl AssetImporter for $name {
            fn name(&self) -> &'static str {
                $ext
            }
            fn extensions(&self) -> &'static [&'static str] {
                &[$ext]
            }
            fn import(&self, _ctx: &mut ImportContext, _bytes: &[u8]) -> Result<ImportedAsset> {
                Err(EngineError::NotYetSupported { what: $what, priority: $prio })
            }
        }
    };
}

stub_importer!(JpgImporter, "jpg", "jpeg baseline decoder (P1: jpeg-decoder 集成)", "P1");
stub_importer!(JpegImporter, "jpeg", "jpeg baseline decoder", "P1");
stub_importer!(GifImporter, "gif", "gif89a decoder", "P1");
stub_importer!(WebPImporter, "webp", "webp decoder", "P1");
stub_importer!(AvifImporter, "avif", "avif decoder", "P2");
stub_importer!(ExrImporter, "exr", "openexr decoder", "P2");
stub_importer!(DdsImporter, "dds", "dds/bcn decoder", "P1");
stub_importer!(Ktx2Importer, "ktx2", "ktx2 container", "P1");
stub_importer!(FbxImporter, "fbx", "fbx sdk/转换器集成", "P1");
stub_importer!(UsdImporter, "usd", "usd 集成", "P2");
stub_importer!(UsdzImporter, "usdz", "usdz 集成", "P2");
stub_importer!(BlendImporter, "blend", "blender 导出管线", "P1");
stub_importer!(StlImporter, "stl", "stl 网格", "P2");
stub_importer!(PlyImporter, "ply", "ply 网格", "P2");
stub_importer!(Mp4Importer, "mp4", "视频解码", "P2");
stub_importer!(TtfImporter, "ttf", "freetype/ab_glyph 栅格化", "P1");
stub_importer!(OtfImporter, "otf", "freetype/ab_glyph 栅格化", "P1");

/// 注册全部内置导入器（IF-170）。
pub fn builtin_importers() -> Vec<Box<dyn AssetImporter>> {
    vec![
        Box::new(PngImporter),
        Box::new(QoiImporter),
        Box::new(BmpImporter),
        Box::new(TgaImporter),
        Box::new(PnmImporter),
        Box::new(ObjImporter),
        Box::new(WavImporter),
        Box::new(JsonImporter),
        Box::new(TomlImporter),
        Box::new(CsvImporter),
        Box::new(JpgImporter),
        Box::new(JpegImporter),
        Box::new(GifImporter),
        Box::new(WebPImporter),
        Box::new(AvifImporter),
        Box::new(ExrImporter),
        Box::new(DdsImporter),
        Box::new(Ktx2Importer),
        Box::new(FbxImporter),
        Box::new(UsdImporter),
        Box::new(UsdzImporter),
        Box::new(BlendImporter),
        Box::new(StlImporter),
        Box::new(PlyImporter),
        Box::new(Mp4Importer),
        Box::new(TtfImporter),
        Box::new(OtfImporter),
    ]
}
