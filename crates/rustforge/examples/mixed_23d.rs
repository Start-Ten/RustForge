//! 混合 2D/3D 示例：同一场景同时包含 3D 立方体与 2D 精灵（N1-N3 验收）。
//! `cargo run -p rustforge --example mixed_23d`；CI：`-- --frames 3`。

use rustforge::prelude::*;
use rustforge::{App, Engine, EngineConfig, FrameCtx};

#[derive(Component)]
struct CubeSpin(f32);

#[derive(Component)]
struct HudPulse {
    t: f32,
}

struct MixedApp {
    mesh: rf_asset::MeshAsset,
    angle: f32,
    pulse: f32,
}

impl App for MixedApp {
    fn setup(&mut self, world: &mut World, _pipeline: &mut rf_asset::ImportPipeline) {
        // N1：2D 与 3D 组件共存于同一 World
        world.spawn((CubeSpin(0.7),));
        world.spawn((HudPulse { t: 0.0 }, rf_editor::PositionComponent(Vec2::new(400.0, 80.0))));
        self.mesh = rf_asset::MeshAsset::unit_cube();
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        let q = world.query::<&mut CubeSpin, ()>();
        if let Some(spin) = q.iter().next() {
            spin.0 += dt;
        }
        self.angle = world.query::<&CubeSpin, ()>().iter().next().map(|s| s.0).unwrap_or(0.0);
        let q2 = world.query::<&mut HudPulse, ()>();
        if let Some(p) = q2.iter().next() {
            p.t += dt;
        }
        self.pulse = (self.angle * 2.0).sin() * 0.5 + 0.5;
    }

    fn render(&mut self, frame: &mut FrameCtx) {
        frame.image.clear([16, 18, 26, 255]);
        // ---- 3D 层（N3：3D 对象渲染到主帧） ----
        let mut cam3 = *frame.camera3d;
        cam3.position = Vec3::new(0.0, 2.2, 5.0);
        cam3.pitch = -0.3;
        let model = Mat4::from_rotation_y(self.angle).mul_mat4(Mat4::from_scale(Vec3::splat(1.4)));
        let cube = rf_render::render_mesh_cpu(
            frame.image.width,
            frame.image.height,
            &self.mesh,
            model,
            &cam3,
            &MaterialParams::default(),
            None,
            &[
                Light::Directional {
                    dir: Vec3::new(-0.4, -1.0, -0.4).normalized(),
                    color: Color::rgb(1.0, 0.95, 0.9),
                    intensity: 0.85,
                },
                Light::Directional {
                    dir: Vec3::new(0.6, 0.2, 0.5).normalized(),
                    color: Color::rgb(0.3, 0.4, 0.9),
                    intensity: 0.4,
                },
            ],
        );
        for (dst, src) in frame.image.data.chunks_exact_mut(4).zip(cube.data.chunks_exact(4)) {
            dst.copy_from_slice(src);
        }
        // ---- 2D 层（N7：LayerStack 交叉顺序之上层） ----
        let cam2 = *frame.camera2d;
        let tex = Rgba8Image::filled(4, 4, [255, 255, 255, 255]);
        let mut r2 = SoftwareRenderer2D::new();
        r2.begin_frame(frame.image, &cam2);
        let s = 30.0 + self.pulse * 25.0;
        r2.draw_sprite(
            Some(&tex),
            &[0.0, 0.0, 1.0, 1.0],
            Color::rgba(0.98, 0.8, 0.25, 0.9),
            10,
            &Mat2x3::compose(Vec2::new(s, s), Rot2::from_angle(self.angle), Vec2::new(400.0, 90.0)),
        );
        // HUD 底条
        r2.draw_sprite(
            Some(&tex),
            &[0.0, 0.0, 1.0, 1.0],
            Color::rgba(0.1, 0.12, 0.2, 0.85),
            5,
            &Mat2x3::compose(
                Vec2::new(800.0, 26.0),
                Rot2::identity(),
                Vec2::new(400.0, 600.0 - 26.0),
            ),
        );
        let stats = r2.flush();
        r2.take_target(frame.image);
        rf_debugger::rf_debug!("mixed", "sprites={} draws={}", stats.sprites, stats.draw_calls);
    }
}

fn main() -> Result<(), EngineError> {
    let frames = std::env::args().nth(1).map(|a| a == "--frames").unwrap_or(false);
    let config = EngineConfig {
        window: rf_platform::WindowConfig {
            title: "RustForge — mixed 2D/3D".into(),
            width: 800,
            height: 600,
            ..Default::default()
        },
        headless: frames,
        frames: frames.then_some(3),
        ..Default::default()
    };
    let mut engine = Engine::new(config)?;
    let summary =
        engine.run(MixedApp { mesh: rf_asset::MeshAsset::unit_cube(), angle: 0.0, pulse: 0.0 })?;
    rf_debugger::rf_info!(
        "mixed",
        "exited after {} frames ({:.1} fps)",
        summary.frames,
        summary.avg_fps
    );
    Ok(())
}
