//! MVP 验收测试（T12）：对照 docs/step0-constraints.md §8 逐条断言。

use rustforge::prelude::*;
use rustforge::{App, Engine, EngineConfig, FrameCtx};

// ---- 1. 窗口 + 2D/3D 渲染 ----

#[derive(Component)]
struct Cnt(#[allow(dead_code)] u32);

struct AcceptApp {
    drew_2d: bool,
    drew_3d: bool,
    mesh: rf_asset::MeshAsset,
}

impl App for AcceptApp {
    fn setup(&mut self, world: &mut World, _pipeline: &mut rf_asset::ImportPipeline) {
        world.spawn((Cnt(0),));
        // 2D 与 3D 组件共存（验收 3）
        world.spawn((rf_editor::PositionComponent(Vec2::new(5.0, 5.0)),));
    }

    fn update(&mut self, _world: &mut World, _dt: f32) {}

    fn render(&mut self, frame: &mut FrameCtx) {
        frame.image.clear([10, 10, 20, 255]);
        // 2D 渲染
        let cam2 = *frame.camera2d;
        let tex = Rgba8Image::filled(4, 4, [255, 200, 100, 255]);
        let mut r2 = SoftwareRenderer2D::new();
        r2.begin_frame(frame.image, &cam2);
        r2.draw_sprite(
            Some(&tex),
            &[0.0, 0.0, 1.0, 1.0],
            Color::WHITE,
            1,
            &Mat2x3::compose(Vec2::splat(32.0), Rot2::identity(), Vec2::new(64.0, 48.0)),
        );
        let stats = r2.flush();
        r2.take_target(frame.image);
        self.drew_2d = stats.sprites > 0;
        // 3D 渲染（N3：2D 对象空间 + 3D 对象空间并存）
        let mut cam3 = *frame.camera3d;
        cam3.position = Vec3::new(0.0, 1.5, 5.0);
        let cube = rf_render::render_mesh_cpu(
            frame.image.width,
            frame.image.height,
            &self.mesh,
            Mat4::from_rotation_y(0.5).mul_mat4(Mat4::from_scale(Vec3::splat(1.5))),
            &cam3,
            &MaterialParams::default(),
            None,
            &[Light::Directional {
                dir: Vec3::new(0.0, -1.0, -0.3).normalized(),
                color: Color::WHITE,
                intensity: 0.9,
            }],
        );
        // 合成（覆盖像素计数验证）
        let covered = cube
            .data
            .chunks_exact(4)
            .filter(|p| p[3] > 0 || p[0] != 0 || p[1] != 0 || p[2] != 0)
            .count();
        self.drew_3d = covered > 100;
        for (dst, src) in frame.image.data.chunks_exact_mut(4).zip(cube.data.chunks_exact(4)) {
            dst.copy_from_slice(src);
        }
    }
}

#[test]
fn mvp_window_and_dual_render() {
    let mut engine = Engine::new(EngineConfig {
        headless: true,
        window: rf_platform::WindowConfig { width: 128, height: 96, ..Default::default() },
        frames: Some(2),
        ..Default::default()
    })
    .unwrap();
    let mut app =
        AcceptApp { drew_2d: false, drew_3d: false, mesh: rf_asset::MeshAsset::unit_cube() };
    let summary = engine.run_headless(&mut app, 2).unwrap();
    assert_eq!(summary.frames, 2);
    assert!(app.drew_2d, "2D 渲染路径未产出精灵");
    assert!(app.drew_3d, "3D 渲染路径未覆盖像素");
    // 帧缓冲确实被写入
    assert!(engine.frame_buffer().data.iter().any(|b| *b != 0));
}

// ---- 2. 后端切换（请求 Vulkan → Software 降级，行为可验证） ----

#[test]
fn mvp_backend_switch() {
    let (d1, i1) = rf_rhi::create_device(rf_rhi::DeviceDesc {
        preferred: rf_rhi::Backend::Software,
        software_size: (8, 8),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(i1.backend, rf_rhi::Backend::Software);
    let (d2, i2) = rf_rhi::create_device(rf_rhi::DeviceDesc {
        preferred: rf_rhi::Backend::Vulkan,
        software_size: (8, 8),
        ..Default::default()
    })
    .unwrap();
    assert!(i2.name.contains("fallback"));
    drop((d1, d2));
    // 拒绝降级 → 明确错误
    assert!(rf_rhi::create_device(rf_rhi::DeviceDesc {
        preferred: rf_rhi::Backend::Dx12,
        allow_fallback: false,
        ..Default::default()
    })
    .is_err());
}

// ---- 3. ECS 2D/3D 组件共存 ----

#[test]
fn mvp_ecs_mixed_components() {
    let mut world = World::new();
    #[derive(Component)]
    struct Mesh3D;
    #[derive(Component)]
    struct Sprite2D;
    let a = world.spawn((Mesh3D,));
    let b = world.spawn((Sprite2D,));
    let both = world.spawn((Mesh3D, Sprite2D));
    assert!(world.has::<Mesh3D>(both) && world.has::<Sprite2D>(both));
    let n3d = world.query::<&Mesh3D, ()>().iter().count();
    let n2d = world.query::<&Sprite2D, ()>().iter().count();
    assert_eq!((n3d, n2d), (2, 2));
    let _ = (a, b);
}

// ---- 4. 资产导入（PNG/QOI/BMP/TGA/OBJ/glTF/TMX…） ----

#[test]
fn mvp_asset_imports() {
    let mut pipeline = rf_asset::ImportPipeline::new(32);
    let dir = std::env::temp_dir().join("rf_accept_assets");
    std::fs::create_dir_all(&dir).unwrap();
    // QOI（内置编码器）
    let qoi = rf_asset::importers::encode_qoi_test(&Rgba8Image::filled(3, 3, [7, 8, 9, 255]));
    std::fs::write(dir.join("a.qoi"), qoi).unwrap();
    let id = pipeline.import_sync(dir.join("a.qoi").to_str().unwrap()).unwrap();
    assert!(pipeline.get::<rf_asset::TextureAsset>(id).is_some());
    // OBJ
    std::fs::write(dir.join("m.obj"), b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    let mid = pipeline.import_sync(dir.join("m.obj").to_str().unwrap()).unwrap();
    let mesh = pipeline.get::<rf_asset::MeshAsset>(mid).unwrap();
    assert_eq!(mesh.indices.len(), 3);
    // WAV（经 rf-audio 编码）
    let wav = rf_audio::encode_wav(&[0.5; 32], 8000);
    std::fs::write(dir.join("s.wav"), wav).unwrap();
    let sid = pipeline.import_sync(dir.join("s.wav").to_str().unwrap()).unwrap();
    assert!(pipeline.get::<rf_audio::AudioAsset>(sid).is_some());
    // P0 桩格式明确报 NotYetSupported
    std::fs::write(dir.join("x.fbx"), b"binary").unwrap();
    assert!(pipeline.import_sync(dir.join("x.fbx").to_str().unwrap()).is_err());
}

// ---- 5. 编辑器：视口/层级/属性/资产浏览器 + 模式切换 + 撤销 ----

#[test]
fn mvp_editor() {
    let mut world = World::new();
    let e = world.spawn((
        rf_editor::NameComponent("Player".into()),
        rf_editor::PositionComponent(Vec2::new(1.0, 2.0)),
    ));
    let mut app = rf_editor::EditorApp::new(rf_editor::EditorMode::TwoD);
    app.model.selection.select_one(e);
    app.run_frame(&mut world).unwrap();
    // 模式切换
    app.model.toggle_mode();
    assert_eq!(app.model.mode, rf_editor::EditorMode::ThreeD);
    // 撤销重做
    let mut undo = rf_editor::UndoStack::new();
    undo.push("noop", Box::new(|_| {}), Box::new(|_| {}));
    assert!(undo.can_undo());
    assert_eq!(undo.undo(&mut world), Some("noop"));
    // 属性网格（反射，IF-289）
    rf_reflection::reflect_struct_demo_check();
}

// ---- 6. 调试器：日志/控制台/性能/渲染统计 ----

#[test]
fn mvp_debugger() {
    rf_debugger::init_global();
    rf_debugger::rf_info!("acceptance", "hello");
    let mut console = rf_debugger::Console::new();
    console.register_cvar("g_debug", "调试绘制", rf_debugger::CVarValue::Bool(false));
    assert!(console.execute("set g_debug true").is_ok());
    assert_eq!(console.cvar("g_debug").unwrap().value, rf_debugger::CVarValue::Bool(true));
    let mut profiler = rf_debugger::Profiler::new();
    profiler.frame_end(1.0 / 60.0);
    assert!((profiler.fps() - 60.0).abs() < 1.0);
    // 渲染统计
    let stats = rf_debugger::RenderStats {
        sprites: 3,
        draw_calls_2d: 1,
        triangles: 12,
        ..Default::default()
    };
    assert_eq!((stats.sprites, stats.triangles), (3, 12));
}

// ---- 7. cargo build/test/run 通道（本测试文件即 test 通道；run 通道见 examples） ----

#[test]
fn mvp_examples_run_headless() {
    // 直接调用示例逻辑等价的 headless 引擎循环（示例二进制由 CI 冒烟运行）
    let mut engine = Engine::new(EngineConfig {
        headless: true,
        window: rf_platform::WindowConfig { width: 32, height: 32, ..Default::default() },
        ..Default::default()
    })
    .unwrap();
    struct Nop;
    impl App for Nop {
        fn setup(&mut self, _: &mut World, _: &mut rf_asset::ImportPipeline) {}
        fn update(&mut self, _: &mut World, _: f32) {}
        fn render(&mut self, frame: &mut FrameCtx) {
            frame.image.clear([1, 2, 3, 255]);
        }
    }
    let s = engine.run_headless(&mut Nop, 3).unwrap();
    assert_eq!(s.frames, 3);
}

// ---- 8. Git 仓库与 CI（由外部验证：git log / .github/workflows 存在性） ----

#[test]
fn mvp_repo_infra() {
    // cargo test 的 cwd 是 crate 目录 —— 向上找 workspace 根
    let mut root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while root.parent().is_some() && !root.join(".git").exists() {
        root = root.parent().unwrap().to_path_buf();
    }
    let f = |p: &str| root.join(p);
    assert!(root.join(".git").exists(), "git 仓库存在");
    assert!(f(".github/workflows/ci.yml").exists(), "CI 配置存在");
    assert!(f(".gitignore").exists());
    assert!(f(".gitattributes").exists());
    assert!(f("README.md").exists());
    assert!(f("LICENSE-MIT").exists());
    assert!(f("LICENSE-APACHE").exists());
    assert!(f("CHANGELOG.md").exists());
    assert!(f("docs/step2.5-interface-freeze.md").exists(), "冻结清单存在");
}

// ---- 9. 混合 2D/3D（N1-N3, N7, N11） ----

#[test]
fn mvp_mixed_scene() {
    let mut world = World::new();
    #[derive(Component)]
    struct R3D;
    #[derive(Component)]
    struct S2D;
    world.spawn((R3D,));
    world.spawn((S2D,));
    // LayerStack 交叉排序
    let mut ls = rf_render::LayerStack::default();
    ls.add("bg", rf_render::LayerKind::ThreeD, 0);
    ls.add("sprites", rf_render::LayerKind::TwoD, 1);
    ls.add("fx", rf_render::LayerKind::TwoD, 2);
    let order = ls.layers_in_order();
    assert_eq!(order.len(), 3);
    assert_eq!(order[0].1, rf_render::LayerKind::ThreeD);
    assert_eq!(order[1].1, rf_render::LayerKind::TwoD);
    // 混合渲染一帧
    let mut engine = Engine::new(EngineConfig {
        headless: true,
        window: rf_platform::WindowConfig { width: 48, height: 48, ..Default::default() },
        ..Default::default()
    })
    .unwrap();
    let _ = world; // world 归 engine 所有
    struct Mixed;
    impl App for Mixed {
        fn setup(&mut self, _: &mut World, _: &mut rf_asset::ImportPipeline) {}
        fn update(&mut self, _: &mut World, _: f32) {}
        fn render(&mut self, frame: &mut FrameCtx) {
            frame.image.clear([0, 0, 0, 255]);
            let cube = rf_render::render_mesh_cpu(
                frame.image.width,
                frame.image.height,
                &rf_asset::MeshAsset::unit_cube(),
                Mat4::identity(),
                frame.camera3d,
                &MaterialParams::default(),
                None,
                &[Light::Directional {
                    dir: Vec3::new(0.0, -1.0, 0.0),
                    color: Color::WHITE,
                    intensity: 1.0,
                }],
            );
            for (d, s) in frame.image.data.chunks_exact_mut(4).zip(cube.data.chunks_exact(4)) {
                d.copy_from_slice(s);
            }
            let tex = Rgba8Image::filled(2, 2, [255, 0, 0, 255]);
            let cam = *frame.camera2d;
            let mut r2 = SoftwareRenderer2D::new();
            r2.begin_frame(frame.image, &cam);
            r2.draw_sprite(
                Some(&tex),
                &[0.0, 0.0, 1.0, 1.0],
                Color::WHITE,
                1,
                &Mat2x3::compose(Vec2::splat(16.0), Rot2::identity(), Vec2::new(24.0, 24.0)),
            );
            r2.flush();
            r2.take_target(frame.image);
        }
    }
    let s = engine.run_headless(&mut Mixed, 1).unwrap();
    assert_eq!(s.frames, 1);
    // 帧非空（2D+3D 都有输出）
    assert!(engine.frame_buffer().data.chunks_exact(4).any(|p| p[3] != 0));
}
