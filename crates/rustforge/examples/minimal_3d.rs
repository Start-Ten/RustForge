//! 最小 3D 示例：窗口 + 旋转立方体（Lambert 光照，CPU 光栅化）。
//! `cargo run -p rustforge --example minimal_3d`；CI：`-- --frames 3`。

use rustforge::prelude::*;
use rustforge::{App, Engine, EngineConfig, FrameCtx};

#[derive(Component)]
struct Spin {
    speed: f32,
    angle: f32,
}

struct SpinningCube {
    angle: f32,
    mesh: rf_asset::MeshAsset,
}

impl App for SpinningCube {
    fn setup(&mut self, world: &mut World, _pipeline: &mut rf_asset::ImportPipeline) {
        world.spawn((Spin { speed: 0.9, angle: 0.0 },));
        self.mesh = rf_asset::MeshAsset::unit_cube();
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        let q = world.query::<&mut Spin, ()>();
        for spin in q.iter() {
            spin.angle += spin.speed * dt;
        }
        self.angle = world.query::<&Spin, ()>().iter().next().map(|s| s.angle).unwrap_or(0.0);
    }

    fn render(&mut self, frame: &mut FrameCtx) {
        // 3D：CPU 光栅化到独立缓冲再合成
        let mut cam = *frame.camera3d;
        cam.position = Vec3::new(self.angle.sin() * 6.0, 3.0, self.angle.cos() * 6.0 + 0.1);
        cam.yaw = -self.angle;
        cam.pitch = -0.35;
        let model = Mat4::from_rotation_y(self.angle).mul_mat4(Mat4::from_scale(Vec3::splat(1.6)));
        let rendered = rf_render::render_mesh_cpu(
            frame.image.width,
            frame.image.height,
            &self.mesh,
            model,
            &cam,
            &MaterialParams::default(),
            None,
            &[Light::Directional {
                dir: Vec3::new(-0.5, -1.0, -0.3).normalized(),
                color: Color::WHITE,
                intensity: 0.9,
            }],
        );
        // 合成到底帧（3D 层在下）
        for (dst, src) in frame.image.data.chunks_exact_mut(4).zip(rendered.data.chunks_exact(4)) {
            dst.copy_from_slice(src);
        }
    }
}

fn main() -> Result<(), EngineError> {
    let frames = std::env::args().nth(1).map(|a| a == "--frames").unwrap_or(false);
    let config = EngineConfig {
        window: rf_platform::WindowConfig {
            title: "RustForge — minimal_3d".into(),
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
        engine.run(SpinningCube { angle: 0.0, mesh: rf_asset::MeshAsset::unit_cube() })?;
    rf_debugger::rf_info!(
        "minimal_3d",
        "exited after {} frames ({:.1} fps)",
        summary.frames,
        summary.avg_fps
    );
    Ok(())
}
