//! PAK 归档（IF-168）与图集打包（IF-169）。

use crate::GlyphRect;
use rf_core::{fnv1a64, EngineError, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

const PAK_MAGIC: [u8; 4] = [b'R', b'F', b'P', b'K'];
const PAK_VERSION: u16 = 1;

struct PakEntry {
    name: String,
    offset: u64,
    len: u64,
    hash: u64,
    compressed: bool,
}

/// 写 PAK（IF-168）：索引 + 块数据，可选 Deflate 压缩。返回文件字节长度。
pub fn write_pak(path: &str, files: &[(String, Vec<u8>)], compress: bool) -> Result<u64> {
    let mut blob: Vec<u8> = Vec::new();
    let mut entries: Vec<PakEntry> = Vec::new();
    let mut compressed_names: Vec<String> = Vec::new();
    for (name, data) in files {
        let offset = blob.len() as u64;
        let (stored, compressed) = if compress {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data)?;
            (enc.finish()?, true)
        } else {
            (data.clone(), false)
        };
        blob.extend_from_slice(&stored);
        entries.push(PakEntry {
            name: name.clone(),
            offset,
            len: stored.len() as u64,
            hash: fnv1a64(data),
            compressed,
        });
        if compressed {
            compressed_names.push(name.clone());
        }
    }
    // 索引
    let mut index: Vec<u8> = Vec::new();
    index.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for e in &entries {
        let name_bytes = e.name.as_bytes();
        index.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        index.extend_from_slice(name_bytes);
        index.extend_from_slice(&e.offset.to_le_bytes());
        index.extend_from_slice(&e.len.to_le_bytes());
        index.extend_from_slice(&e.hash.to_le_bytes());
        index.push(e.compressed as u8);
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&PAK_MAGIC);
    out.extend_from_slice(&PAK_VERSION.to_le_bytes());
    out.extend_from_slice(&(index.len() as u64).to_le_bytes());
    out.extend_from_slice(&index);
    out.extend_from_slice(&blob);
    let path = Path::new(path);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, &out)?;
    Ok(out.len() as u64)
}

/// PAK 读取器（IF-168）。
pub struct PakReader {
    data: Vec<u8>,
    entries: HashMap<String, PakEntry>,
}

impl PakReader {
    pub fn open(path: &str) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(data)
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        if data.len() < 14 || data[0..4] != PAK_MAGIC {
            return Err(EngineError::InvalidData("pak: bad magic".into()));
        }
        let version = u16::from_le_bytes(data[4..6].try_into().unwrap());
        if version != PAK_VERSION {
            return Err(EngineError::InvalidData(format!(
                "pak: version {version} != {PAK_VERSION}"
            )));
        }
        let index_len = u64::from_le_bytes(data[6..14].try_into().unwrap()) as usize;
        let mut pos = 14usize;
        let index_end = pos + index_len;
        if index_end > data.len() {
            return Err(EngineError::InvalidData("pak: index oob".into()));
        }
        let count = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let mut entries = HashMap::new();
        for _ in 0..count {
            let nlen = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            let name = String::from_utf8_lossy(&data[pos..pos + nlen]).into_owned();
            pos += nlen;
            let offset = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let len = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let hash = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let compressed = data[pos] != 0;
            pos += 1;
            entries.insert(name, PakEntry { name: String::new(), offset, len, hash, compressed });
        }
        Ok(Self { data, entries })
    }

    pub fn list(&self) -> Vec<String> {
        let mut v: Vec<String> = self.entries.keys().cloned().collect();
        v.sort();
        v
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let e = self
            .entries
            .get(name)
            .ok_or_else(|| EngineError::Message(format!("pak: no entry {name}")))?;
        let start = 14 + self.index_len_hint() + e.offset as usize;
        let end = start + e.len as usize;
        if end > self.data.len() {
            return Err(EngineError::InvalidData("pak: entry oob".into()));
        }
        let raw = &self.data[start..end];
        let out = if e.compressed {
            let mut dec = flate2::read::DeflateDecoder::new(raw);
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut dec, &mut out)?;
            out
        } else {
            raw.to_vec()
        };
        if fnv1a64(&out) != e.hash {
            return Err(EngineError::InvalidData(format!("pak: hash mismatch for {name}")));
        }
        Ok(out)
    }

    /// 完整性校验（IF-168）。
    pub fn verify(&self, name: &str) -> bool {
        self.read(name).is_ok()
    }

    fn index_len_hint(&self) -> usize {
        // 索引长度 = 总长记录（14..22）——重算：由首个条目偏移反推不稳健；
        // 写入时索引长度记录在头部，这里重新解析。
        u64::from_le_bytes(self.data[6..14].try_into().unwrap()) as usize
    }
}

/// 打包矩形。
pub struct PackedRect {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// 图集打包器（IF-169）。
pub trait AtlasPacker {
    fn pack(&self, images: &[(String, rf_core::Rgba8Image)], padding: u32) -> PackedAtlas;
}

/// 打包结果。
pub struct PackedAtlas {
    pub atlas: rf_core::Rgba8Image,
    pub rects: Vec<PackedRect>,
}

/// 货架打包（按高度降序逐行摆放）。
pub struct ShelfPacker;

impl AtlasPacker for ShelfPacker {
    fn pack(&self, images: &[(String, rf_core::Rgba8Image)], padding: u32) -> PackedAtlas {
        // 逐档放大重试直至全部放下（货架启发式对 √面积 有碎片开销，1.25 系数起试）
        let pad = padding as u64;
        let total_area: u64 =
            images.iter().map(|(_, i)| (i.width as u64 + pad) * (i.height as u64 + pad)).sum();
        let mut side =
            (((total_area as f64).sqrt() * 1.25).ceil() as u32).max(16).next_power_of_two();
        loop {
            let (rects, atlas) = pack_into(images, padding, side);
            if rects.len() == images.len() || side >= 1 << 16 {
                return PackedAtlas { atlas, rects };
            }
            side *= 2;
        }
    }
}

fn pack_into(
    images: &[(String, rf_core::Rgba8Image)],
    padding: u32,
    side: u32,
) -> (Vec<PackedRect>, rf_core::Rgba8Image) {
    let mut sorted: Vec<&(String, rf_core::Rgba8Image)> = images.iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1.height));
    let mut atlas = rf_core::Rgba8Image::new(side, side);
    let mut rects = Vec::new();
    let mut shelf_y = 0u32;
    let mut shelf_h = 0u32;
    let mut x = 0u32;
    for (name, img) in sorted {
        let w = img.width + padding;
        if x + w > side {
            x = 0;
            shelf_y += shelf_h;
            shelf_h = 0;
        }
        if shelf_y + img.height > side {
            continue; // 当前边长放不下（外层放大重试）
        }
        for py in 0..img.height {
            for px in 0..img.width {
                if let Some(c) = img.get(px, py) {
                    atlas.set(x + px, shelf_y + py, c);
                }
            }
        }
        rects.push(PackedRect { name: name.clone(), x, y: shelf_y, w: img.width, h: img.height });
        x += w;
        shelf_h = shelf_h.max(img.height + padding);
    }
    (rects, atlas)
}

/// GlyphRect 兼容别名（FontAsset 使用）。
pub type AtlasGlyphRect = GlyphRect;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pak_roundtrip_stored_and_compressed() {
        let dir = std::env::temp_dir().join("rf_pak_test");
        std::fs::create_dir_all(&dir).unwrap();
        let files = vec![
            ("a.txt".to_string(), b"hello rustforge".to_vec()),
            ("b.bin".to_string(), vec![7u8; 1024]),
        ];
        for compress in [false, true] {
            let p = dir.join(if compress { "c.pak" } else { "s.pak" });
            let size = write_pak(p.to_str().unwrap(), &files, compress).unwrap();
            assert!(size > 0);
            let reader = PakReader::open(p.to_str().unwrap()).unwrap();
            assert_eq!(reader.list(), vec!["a.txt", "b.bin"]);
            assert_eq!(reader.read("a.txt").unwrap(), b"hello rustforge");
            assert_eq!(reader.read("b.bin").unwrap(), vec![7u8; 1024]);
            assert!(reader.verify("a.txt"));
            assert!(reader.read("missing").is_err());
            assert!(PakReader::from_bytes(vec![1, 2, 3]).is_err());
        }
    }

    #[test]
    fn shelf_packer_packs_all() {
        let imgs: Vec<(String, rf_core::Rgba8Image)> = (0..8)
            .map(|i| {
                (
                    format!("s{i}"),
                    rf_core::Rgba8Image::filled(16, 16 + i as u32 * 2, [i as u8, 0, 0, 255]),
                )
            })
            .collect();
        let packed = ShelfPacker.pack(&imgs, 2);
        assert_eq!(packed.rects.len(), 8);
        // 无重叠
        for i in 0..packed.rects.len() {
            for j in i + 1..packed.rects.len() {
                let a = &packed.rects[i];
                let b = &packed.rects[j];
                let overlap =
                    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
                assert!(!overlap, "rects overlap");
            }
        }
        assert!(packed.atlas.width >= 16);
    }
}
