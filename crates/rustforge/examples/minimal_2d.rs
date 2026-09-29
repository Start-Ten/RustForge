//! 最小 2D 示例：窗口 + 精灵渲染（Software 后端）。
//! `cargo run -p rustforge --example minimal_2d`
//! CI/无窗口模式：`-- --frames 3`（HeadlessPlatform 仍渲染帧并计数）。

use rustforge::prelude::*;
use rustforge::{App, Engine, EngineConfig, FrameCtx};

#[derive(Component)]
struct Bounce {
    vel: Vec2,
}

struct BouncingSprites {
    sprites: Vec<Mat2x3>,
    frame_count: u64,
}

impl App for BouncingSprites {
    fn setup(&mut self, world: &mut World, _pipeline: &mut rf_asset::ImportPipeline) {
        for i in 0..8 {
            let x = 60.0 + i as f32 * 90.0;
            world.spawn((
                rf_editor::PositionComponent(Vec2::new(x, 300.0)),
                Bounce { vel: Vec2::new(60.0 + i as f32 * 17.0, -(80.0 + i as f32 * 11.0)) },
            ));
        }
        self.sprites = Vec::new();
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        self.sprites.clear();
        let q = world.query::<(&mut rf_editor::PositionComponent, &mut Bounce), ()>();
        for (pos, bounce) in q.iter() {
            pos.0 += bounce.vel * dt;
            if pos.0.x < 0.0 || pos.0.x > 800.0 {
                bounce.vel.x = -bounce.vel.x;
            }
            if pos.0.y < 0.0 || pos.0.y > 600.0 {
                bounce.vel.y = -bounce.vel.y;
            }
            let t = Mat2x3::compose(Vec2::splat(40.0), Rot2::identity(), pos.0);
            self.sprites.push(t);
        }
        self.frame_count += 1;
    }

    fn render(&mut self, frame: &mut FrameCtx) {
        let palette = [
            Color::rgb(0.95, 0.30, 0.35),
            Color::rgb(0.35, 0.75, 0.40),
            Color::rgb(0.35, 0.55, 0.95),
            Color::rgb(0.95, 0.75, 0.30),
        ];
        frame.image.clear([24, 26, 34, 255]);
        let tex = Rgba8Image::filled(8, 8, [255, 255, 255, 255]);
        let cam = *frame.camera2d;
        let mut r2 = SoftwareRenderer2D::new();
        r2.begin_frame(frame.image, &cam);
        for (i, t) in self.sprites.iter().enumerate() {
            r2.draw_sprite(
                Some(&tex),
                &[0.0, 0.0, 1.0, 1.0],
                palette[i % palette.len()],
                i as i32,
                t,
            );
        }
        let stats = r2.flush();
        r2.take_target(frame.image);
        if self.frame_count % 60 == 0 {
            rf_debugger::rf_debug!(
                "minimal_2d",
                "frame {} sprites={} draws={}",
                self.frame_count,
                stats.sprites,
                stats.draw_calls
            );
        }
    }
}

fn main() -> Result<(), EngineError> {
    let frames = std::env::args().nth(1).map(|a| a == "--frames").unwrap_or(false);
    let config = EngineConfig {
        window: rf_platform::WindowConfig {
            title: "RustForge — minimal_2d".into(),
            width: 800,
            height: 600,
            ..Default::default()
        },
        headless: frames,
        frames: frames.then_some(3),
        ..Default::default()
    };
    let mut engine = Engine::new(config)?;
    let summary = engine.run(BouncingSprites { sprites: Vec::new(), frame_count: 0 })?;
    rf_debugger::rf_info!(
        "minimal_2d",
        "exited after {} frames ({:.1} fps)",
        summary.frames,
        summary.avg_fps
    );
    Ok(())
}
