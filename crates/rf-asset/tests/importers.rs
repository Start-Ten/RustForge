//! 导入器集成测试：生成真实格式字节 → 导入 → 校验（O1/O2/O4/O7 验收）。

use rf_asset::importers::{encode_qoi_test, ObjImporter};
use rf_asset::*;

fn tmp_dir(name: &str) -> String {
    let d = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&d).unwrap();
    d.to_string_lossy().into_owned()
}

fn import_bytes(
    importer: &dyn AssetImporter,
    path: &str,
    bytes: &[u8],
) -> rf_core::Result<ImportedAsset> {
    let mut ctx = ImportContext::new(path);
    importer.import(&mut ctx, bytes)
}

/// 构造最小 PNG（RGBA，filter 0）。
fn make_png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut raw = Vec::new();
    for _ in 0..h {
        let mut row = vec![0u8; 1 + (w as usize) * 4]; // filter none + RGBA 像素
        row[1..].chunks_exact_mut(4).for_each(|px| px.copy_from_slice(&rgba));
        raw.extend_from_slice(&row);
    }
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut zlib, &raw).unwrap();
    let idat = zlib.finish().unwrap();
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let chunk = |out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]| {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(kind);
        crc_data.extend_from_slice(body);
        out.extend_from_slice(&crc32_simple(&crc_data).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.push(8);
    ihdr.push(6); // RGBA
    ihdr.push(0);
    ihdr.push(0);
    ihdr.push(0);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn crc32_simple(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[test]
fn png_roundtrip() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "png").unwrap();
    let png = make_png(4, 4, [10, 200, 30, 255]);
    let imported = import_bytes(importer.as_ref(), "t.png", &png).unwrap();
    assert_eq!(imported.kind_name, "TextureAsset");
    let dir = tmp_dir("rf_import_test");
    // 经管线导入（含 get 下转）
    std::fs::write(format!("{dir}/t.png"), &png).unwrap();
    let mut pipeline = ImportPipeline::new(16);
    let id = pipeline.import_sync(&format!("{dir}/t.png")).unwrap();
    let tex = pipeline.get::<TextureAsset>(id).unwrap();
    assert_eq!(tex.image.get(2, 3), Some([10, 200, 30, 255]));
    assert_eq!(pipeline.state(id), AssetState::Loaded);
    // 损坏 PNG 报错
    assert!(import_bytes(importer.as_ref(), "bad.png", b"notpng").is_err());
}

#[test]
fn qoi_roundtrip() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "qoi").unwrap();
    let img = rf_core::Rgba8Image::filled(6, 4, [1, 2, 3, 255]);
    let qoi = encode_qoi_test(&img);
    let imported = import_bytes(importer.as_ref(), "t.qoi", &qoi).unwrap();
    let tex = imported.data.as_any().downcast_ref::<TextureAsset>().unwrap();
    assert_eq!(tex.image.get(5, 3), Some([1, 2, 3, 255]));
    assert!(import_bytes(importer.as_ref(), "bad.qoi", b"xxxx").is_err());
}

#[test]
fn bmp_24bit_roundtrip() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "bmp").unwrap();
    // 2×2 24bpp BMP（bottom-up）
    let stride: u32 = 8; // ((2*3+3)/4)*4
    let mut bmp: Vec<u8> = b"BM".to_vec();
    bmp.extend_from_slice(&(54u32 + stride * 2).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes()); // DIB size
    bmp.extend_from_slice(&2i32.to_le_bytes());
    bmp.extend_from_slice(&2i32.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&[0u8; 24]); // DIB 40 = 4+4+4+2+2+24
                                       // 行（bottom-up，行尾 2 字节 stride 填充）：y=1 在前
    bmp.extend_from_slice(&[9, 8, 7, 9, 8, 7, 0, 0]); // BGR → RGB (7,8,9)
    bmp.extend_from_slice(&[1, 2, 3, 1, 2, 3, 0, 0]);
    let imported = import_bytes(importer.as_ref(), "t.bmp", &bmp).unwrap();
    let tex = imported.data.as_any().downcast_ref::<TextureAsset>().unwrap();
    assert_eq!(tex.image.get(0, 0), Some([3, 2, 1, 255])); // 顶行 = 后写行
    assert_eq!(tex.image.get(1, 1), Some([7, 8, 9, 255]));
}

#[test]
fn obj_import() {
    let importer = ObjImporter;
    let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n";
    let imported = import_bytes(&importer, "t.obj", obj).unwrap();
    let mesh = imported.data.as_any().downcast_ref::<MeshAsset>().unwrap();
    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.indices, vec![0, 1, 2]);
    assert_eq!(mesh.normals[0], [0.0, 0.0, 1.0]);
    let _ = &mesh.positions;
    // 空 obj 警告
    let imported = import_bytes(&importer, "empty.obj", b"v 0 0 0\n").unwrap();
    assert!(imported.warnings.iter().any(|w| w.contains("no faces")));
}

#[test]
fn wav_import() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "wav").unwrap();
    let samples: Vec<f32> = (0..64).map(|i| (i as f32 / 64.0).sin()).collect();
    let wav = rf_audio::encode_wav(&samples, 8000);
    let imported = import_bytes(importer.as_ref(), "t.wav", &wav).unwrap();
    let audio = imported.data.as_any().downcast_ref::<rf_audio::AudioAsset>().unwrap();
    assert_eq!(audio.sample_rate, 8000);
    assert_eq!(audio.samples.len(), 64);
}

#[test]
fn json_data_and_spritesheet() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "json").unwrap();
    // 纯数据
    let imported =
        import_bytes(importer.as_ref(), "d.json", br#"{"hp": 100, "name": "hero"}"#).unwrap();
    let data = imported.data.as_any().downcast_ref::<DataAsset>().unwrap();
    assert_eq!(data.0["hp"], serde_json::json!(100));
    // 精灵表（TexturePacker 格式）
    let sheet = br#"{"frames": [{"filename": "walk 0", "frame": {"x": 0, "y": 0, "w": 16, "h": 16}}, {"filename": "walk 1", "frame": {"x": 16, "y": 0, "w": 16, "h": 16}}], "meta": {"image": "atlas.png"}}"#;
    let imported = import_bytes(importer.as_ref(), "sheet.json", sheet).unwrap();
    assert_eq!(imported.kind_name, "SpriteSheetAsset");
    assert_eq!(imported.dependencies.len(), 1); // atlas.png 依赖
    let sheet = imported.data.as_any().downcast_ref::<SpriteSheetAsset>().unwrap();
    assert_eq!(sheet.sprites.len(), 2);
    assert_eq!(sheet.sprites[1].x, 16);
    assert!(!sheet.animations.is_empty());
}

#[test]
fn tommini_and_csv() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "toml").unwrap();
    let imported = import_bytes(
        importer.as_ref(),
        "c.toml",
        b"title = \"demo\"\n[player]\nhp = 100\nflying = false\n",
    )
    .unwrap();
    let data = imported.data.as_any().downcast_ref::<DataAsset>().unwrap();
    assert_eq!(data.0["title"], serde_json::json!("demo"));
    assert_eq!(data.0["player"]["hp"], serde_json::json!(100));
    assert_eq!(data.0["player"]["flying"], serde_json::json!(false));
    let csv = importers::builtin_importers().into_iter().find(|i| i.name() == "csv").unwrap();
    let imported = import_bytes(csv.as_ref(), "t.csv", b"a,b\n1,2\n").unwrap();
    let data = imported.data.as_any().downcast_ref::<DataAsset>().unwrap();
    assert_eq!(data.0[0][1], serde_json::json!("b"));
}

#[test]
fn stub_reports_not_yet_supported() {
    let importer = importers::builtin_importers().into_iter().find(|i| i.name() == "jpg").unwrap();
    let err = import_bytes(importer.as_ref(), "t.jpg", b"whatever");
    assert!(
        matches!(err, Err(rf_core::EngineError::NotYetSupported { what, .. }) if what.contains("jpeg"))
    );
}

#[test]
fn pipeline_hot_reload_generation() {
    let dir = tmp_dir("rf_reload_test");
    let path = format!("{dir}/a.qoi");
    std::fs::write(&path, encode_qoi_test(&rf_core::Rgba8Image::filled(2, 2, [1, 1, 1, 255])))
        .unwrap();
    let mut pipeline = ImportPipeline::new(8);
    let id = pipeline.import_sync(&path).unwrap();
    // 修改文件 → notify_changed 重导入
    std::fs::write(&path, encode_qoi_test(&rf_core::Rgba8Image::filled(2, 2, [9, 9, 9, 255])))
        .unwrap();
    assert!(pipeline.notify_changed(&path));
    let tex = pipeline.get::<TextureAsset>(id).unwrap();
    assert_eq!(tex.image.get(0, 0), Some([9, 9, 9, 255]));
    // 写坏文件 → 保留旧版本
    std::fs::write(&path, b"garbage").unwrap();
    assert!(!pipeline.notify_changed(&path));
    let tex = pipeline.get::<TextureAsset>(id).unwrap();
    assert_eq!(tex.image.get(0, 0), Some([9, 9, 9, 255]));
}

#[test]
fn pipeline_build_index() {
    let dir = tmp_dir("rf_index_test");
    std::fs::write(format!("{dir}/x.png"), make_png(1, 1, [0, 0, 0, 255])).unwrap();
    std::fs::write(format!("{dir}/y.obj"), b"v 0 0 0\n").unwrap();
    std::fs::write(format!("{dir}/z.txt"), b"ignored").unwrap();
    let mut pipeline = ImportPipeline::new(8);
    let count = pipeline.build_index(&dir).unwrap();
    assert_eq!(count, 2); // png + obj，txt 无导入器
}
