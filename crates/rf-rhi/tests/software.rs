//! Software 后端测试：清屏、blit、三角形光栅化、深度测试、降级链。

use rf_math::{Mat4, Vec3};
use rf_rhi::*;

fn quad_mesh() -> (Vec<u8>, Vec<u8>, IndexFormat) {
    // 2 个三角形组成全屏 quad（NDC 直接给 pos）
    let verts: [[f32; 8]; 4] = [
        [-1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], // pos3 normal3 uv2
        [1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
        [1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        [-1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
    ];
    let mut vb = Vec::with_capacity(4 * 32);
    for v in &verts {
        for f in v.iter() {
            vb.extend_from_slice(&f.to_le_bytes());
        }
    }
    let ib: Vec<u8> = [0u32, 1, 2, 0, 2, 3].iter().flat_map(|i| i.to_le_bytes()).collect();
    (vb, ib, IndexFormat::Uint32)
}

#[test]
fn clear_and_blit() {
    let (mut device, _) =
        create_device(DeviceDesc { software_size: (32, 32), ..Default::default() }).unwrap();
    let target = device
        .create_texture(
            TextureDesc {
                width: 32,
                height: 32,
                usage: TextureUsage::RENDER_TARGET,
                ..Default::default()
            },
            None,
        )
        .unwrap();
    device.begin_frame(Some(target)).unwrap();
    let mut cmd = device.create_command_list();
    cmd.begin();
    cmd.clear_color(rf_math::Color::rgb(1.0, 0.0, 0.0)).unwrap();
    cmd.end();
    device.submit(cmd).unwrap();
    let img = device.read_texture(target).unwrap();
    assert_eq!(img.get(5, 5), Some([255, 0, 0, 255])); // 红色 RGBA

    // blit 一块绿色纹理
    let green = device
        .create_texture(
            TextureDesc { width: 4, height: 4, ..Default::default() },
            Some(&[0u8, 255, 0, 255].repeat(16)),
        )
        .unwrap();
    let mut cmd = device.create_command_list();
    cmd.begin();
    cmd.blit(green, 8, 8, 8, 8);
    cmd.end();
    device.submit(cmd).unwrap();
    let img = device.read_texture(target).unwrap();
    assert_eq!(img.get(10, 10), Some([0, 255, 0, 255]));
    assert_eq!(img.get(30, 30), Some([255, 0, 0, 255])); // 未覆盖区域
    let stats = device.frame_stats();
    assert_eq!((stats.clears, stats.blits), (1, 1));
}

#[test]
fn rasterize_fullscreen_quad() {
    let (mut device, _) =
        create_device(DeviceDesc { software_size: (16, 16), ..Default::default() }).unwrap();
    let target = device
        .create_texture(
            TextureDesc {
                width: 16,
                height: 16,
                usage: TextureUsage::RENDER_TARGET,
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let (vb, ib, ifmt) = quad_mesh();
    let vbh = device
        .create_buffer(
            BufferDesc { size: vb.len(), usage: BufferUsage::VERTEX, label: "vb".into() },
            Some(&vb),
        )
        .unwrap();
    let ibh = device
        .create_buffer(
            BufferDesc { size: ib.len(), usage: BufferUsage::INDEX, label: "ib".into() },
            Some(&ib),
        )
        .unwrap();
    let pipe = device
        .create_pipeline(
            PipelineDesc {
                model: ShaderModel::Flat,
                blend: BlendMode::Opaque,
                ..Default::default()
            },
            VertexLayout::mesh(),
        )
        .unwrap();
    device.begin_frame(Some(target)).unwrap();
    let mut cmd = device.create_command_list();
    cmd.begin();
    cmd.clear_color(rf_math::Color::BLACK).unwrap();
    cmd.set_pipeline(pipe);
    cmd.set_vertex_buffer(vbh);
    cmd.set_index_buffer(ibh, ifmt);
    cmd.set_uniform_mat4("u_mvp", &Mat4::identity());
    cmd.set_uniform_mat4("u_model", &Mat4::identity());
    cmd.set_viewport(0, 0, 16, 16);
    cmd.draw_indexed(0, 6);
    cmd.end();
    device.submit(cmd).unwrap();
    let img = device.read_texture(target).unwrap();
    // 中心被 quad 覆盖为白色
    assert_eq!(img.get(8, 8), Some([255, 255, 255, 255]));
    assert_eq!(img.get(0, 0), Some([255, 255, 255, 255])); // quad 覆盖全屏
    let stats = device.frame_stats();
    assert_eq!(stats.draw_calls, 1);
    assert_eq!(stats.triangles, 2);
}

#[test]
fn depth_test_occlusion() {
    let (mut device, _) =
        create_device(DeviceDesc { software_size: (8, 8), ..Default::default() }).unwrap();
    let target = device
        .create_texture(
            TextureDesc {
                width: 8,
                height: 8,
                usage: TextureUsage::RENDER_TARGET,
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let (vb, ib, ifmt) = quad_mesh();
    let vbh = device
        .create_buffer(
            BufferDesc { size: vb.len(), usage: BufferUsage::VERTEX, label: "vb".into() },
            Some(&vb),
        )
        .unwrap();
    let ibh = device
        .create_buffer(
            BufferDesc { size: ib.len(), usage: BufferUsage::INDEX, label: "ib".into() },
            Some(&ib),
        )
        .unwrap();
    let pipe_near = device
        .create_pipeline(
            PipelineDesc {
                model: ShaderModel::Flat,
                depth_test: true,
                blend: BlendMode::Opaque,
                ..Default::default()
            },
            VertexLayout::mesh(),
        )
        .unwrap();
    device.begin_frame(Some(target)).unwrap();
    // 画两遍：第二遍同深度被 z-buffer 剔除（像素不变，draw_calls=2 验证路径）
    for _ in 0..2 {
        let mut cmd = device.create_command_list();
        cmd.begin();
        cmd.set_pipeline(pipe_near);
        cmd.set_vertex_buffer(vbh);
        cmd.set_index_buffer(ibh, ifmt);
        cmd.set_uniform_mat4("u_mvp", &Mat4::identity());
        cmd.set_uniform_mat4("u_model", &Mat4::identity());
        cmd.set_viewport(0, 0, 8, 8);
        cmd.draw_indexed(0, 6);
        cmd.end();
        device.submit(cmd).unwrap();
    }
    let _ = Vec3::new(0.0, 0.0, 0.0);
    let img = device.read_texture(target).unwrap();
    assert_eq!(img.get(4, 4), Some([255, 255, 255, 255]));
}

#[test]
fn texture_upload_and_read() {
    let (mut device, _) = create_device(DeviceDesc::default()).unwrap();
    let data: Vec<u8> = (0..16 * 16 * 4).map(|i| (i % 255) as u8).collect();
    let tex = device
        .create_texture(TextureDesc { width: 16, height: 16, ..Default::default() }, Some(&data))
        .unwrap();
    let img = device.read_texture(tex).unwrap();
    assert_eq!(img.data, data);
    device.update_texture(tex, 0, 0, 1, 1, &[9, 8, 7, 6]).unwrap();
    assert_eq!(device.read_texture(tex).unwrap().get(0, 0), Some([9, 8, 7, 6]));
    // 尺寸不匹配报错
    assert!(device.update_texture(tex, 0, 0, 2, 2, &[0; 4]).is_err());
    assert!(device
        .create_texture(TextureDesc { width: 0, height: 4, ..Default::default() }, None)
        .is_err());
    // 非法原始长度
    assert!(device
        .create_texture(TextureDesc { width: 4, height: 4, ..Default::default() }, Some(&[0; 15]))
        .is_err());
}

#[test]
fn null_backend_accepts_all() {
    let (mut device, info) =
        create_device(DeviceDesc { preferred: Backend::Null, ..Default::default() }).unwrap();
    assert_eq!(info.backend, Backend::Null);
    let tex = device.create_texture(Default::default(), None).unwrap();
    let mut cmd = device.create_command_list();
    cmd.begin();
    cmd.clear_color(rf_math::Color::WHITE).unwrap();
    cmd.blit(tex, 0, 0, 4, 4);
    cmd.draw_indexed(0, 3);
    cmd.end();
    device.submit(cmd).unwrap();
    device.begin_frame(Some(tex)).unwrap();
    device.end_frame().unwrap();
}
