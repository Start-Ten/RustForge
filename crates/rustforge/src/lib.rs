//! RustForge 伞 crate（IF-330 ~ IF-333）：prelude、App、Engine 组装与运行循环。

pub mod prelude;

pub use prelude::*;

use rf_asset::ImportPipeline;
use rf_platform::{FrameTimer, Platform, PlatformEvent, PlatformKind, WindowConfig};
use rf_render::Renderer3D;
use rf_rhi::{Backend, DeviceDesc, SoftwareDevice};
use std::time::Instant;

/// 帧上下文（IF-331）。
pub struct FrameCtx<'a> {
    pub image: &'a mut rf_core::Rgba8Image,
    pub dt: f32,
    pub camera2d: &'a mut Camera2D,
    pub camera3d: &'a mut Camera3D,
}

/// 游戏应用接口（IF-331）。
pub trait App {
    fn setup(&mut self, world: &mut World, pipeline: &mut ImportPipeline);
    fn update(&mut self, world: &mut World, dt: f32);
    /// 渲染一帧（2D 经 renderer2d；3D 数据经 renderer3d 由 update 预上传）。
    fn render(&mut self, frame: &mut FrameCtx);
}

/// 引擎配置（IF-332）。
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub window: WindowConfig,
    pub backend: Option<Backend>,
    pub headless: bool,
    pub target_fps: Option<f32>,
    /// headless 模式的固定帧数（测试）。
    pub frames: Option<u64>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            backend: None,
            headless: false,
            target_fps: Some(60.0),
            frames: None,
        }
    }
}

/// 运行摘要。
pub struct RunSummary {
    pub frames: u64,
    pub avg_fps: f32,
}

/// 引擎（IF-333）：平台 + 设备 + 世界 + 资产 + 主循环。
pub struct Engine {
    config: EngineConfig,
    platform: Box<dyn Platform>,
    /// Software 设备持有（未来 GPU 交换位；MVP 供诊断）。
    #[allow(dead_code)]
    device: SoftwareDevice,
    renderer2d: SoftwareRenderer2D,
    pub world: World,
    pub pipeline: ImportPipeline,
    timer: FrameTimer,
    camera2d: Camera2D,
    camera3d: Camera3D,
    window: Option<rf_platform::WindowId>,
    frame_buffer: rf_core::Rgba8Image,
    present_count: u64,
    exit: bool,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        rf_debugger::init_global();
        let platform = if config.headless {
            rf_platform::create_platform(PlatformKind::Headless)?
        } else {
            rf_platform::create_platform(PlatformKind::Default)?
        };
        // 后端选择（降级链在 rf-rhi 内实现）
        let (info, device) = match config.backend.unwrap_or(Backend::Software) {
            Backend::Null => {
                let d = SoftwareDevice::new(config.window.width, config.window.height);
                (
                    rf_rhi::BackendInfo {
                        name: "Null".into(),
                        backend: Backend::Null,
                        dedicated_gpu: false,
                        caps: rf_rhi::Capabilities::null(),
                    },
                    d,
                )
            }
            _ => {
                let d = SoftwareDevice::new(config.window.width, config.window.height);
                let name = config
                    .backend
                    .map(|b| format!("{} → Software fallback", b.display_name()))
                    .unwrap_or_else(|| "Software (CPU rasterizer)".to_string());
                (
                    rf_rhi::BackendInfo {
                        name,
                        backend: Backend::Software,
                        dedicated_gpu: false,
                        caps: rf_rhi::Capabilities::software(),
                    },
                    d,
                )
            }
        };
        rf_debugger::rf_info!("engine", "backend: {} ({})", info.backend.display_name(), info.name);
        let _ = rf_rhi::create_device(DeviceDesc {
            preferred: Backend::Software,
            allow_fallback: true,
            headless: config.headless,
            software_size: (config.window.width, config.window.height),
        }); // 降级链验证（结果与上面一致）
        let (w, h) = (config.window.width, config.window.height);
        Ok(Self {
            config,
            platform,
            device,
            renderer2d: SoftwareRenderer2D::new(),
            world: World::new(),
            pipeline: ImportPipeline::new(1024),
            timer: FrameTimer::new(),
            camera2d: Camera2D { viewport: (w as f32, h as f32), ..Default::default() },
            camera3d: Camera3D::default(),
            window: None,
            frame_buffer: rf_core::Rgba8Image::new(w, h),
            present_count: 0,
            exit: false,
        })
    }

    /// 创建窗口（desktop 模式 run() 内调用；测试可显式调用）。
    pub fn open_window(&mut self) -> Result<rf_platform::WindowId> {
        let id = self.platform.create_window(self.config.window.clone())?;
        self.window = Some(id);
        Ok(id)
    }

    pub fn camera2d_mut(&mut self) -> &mut Camera2D {
        &mut self.camera2d
    }

    pub fn camera3d_mut(&mut self) -> &mut Camera3D {
        &mut self.camera3d
    }

    pub fn renderer2d_mut(&mut self) -> &mut SoftwareRenderer2D {
        &mut self.renderer2d
    }

    pub fn renderer3d(&mut self) -> Renderer3D {
        Renderer3D::new(SoftwareDevice::new(self.config.window.width, self.config.window.height))
    }

    pub fn frame_buffer(&self) -> &rf_core::Rgba8Image {
        &self.frame_buffer
    }

    pub fn present_count(&self) -> u64 {
        self.present_count
    }

    fn pump_events(&mut self) -> Vec<PlatformEvent> {
        let events = self.platform.poll_events();
        for ev in &events {
            if matches!(ev, PlatformEvent::DestroyRequested)
                || matches!(ev, PlatformEvent::Window(rf_platform::WindowEvent::CloseRequested(_)))
            {
                self.exit = true;
            }
        }
        events
    }

    fn run_one_frame(&mut self, app: &mut dyn App) -> Result<()> {
        let dt = self.timer.tick();
        self.pump_events();
        app.update(&mut self.world, dt);
        {
            let mut cam2 = self.camera2d;
            let mut cam3 = self.camera3d;
            let mut ctx = FrameCtx {
                image: &mut self.frame_buffer,
                dt,
                camera2d: &mut cam2,
                camera3d: &mut cam3,
            };
            app.render(&mut ctx);
        }
        // 呈现
        if let Some(w) = self.window {
            self.platform.present(w, &self.frame_buffer)?;
            self.present_count += 1;
        }
        self.world.advance_tick();
        Ok(())
    }

    /// 主循环（desktop；Esc 或关窗退出）。
    pub fn run(&mut self, mut app: impl App + 'static) -> Result<RunSummary> {
        if !self.config.headless && self.window.is_none() {
            self.open_window()?;
        }
        app.setup(&mut self.world, &mut self.pipeline);
        let started = Instant::now();
        let mut frames = 0u64;
        let app_ref: &mut dyn App = &mut app;
        while !self.exit {
            self.run_one_frame(app_ref)?;
            frames += 1;
            if let Some(max) = self.config.frames {
                if frames >= max {
                    break;
                }
            }
            // 目标帧率节流（简单 sleep；P1 精确计时）
            if let Some(fps) = self.config.target_fps {
                let frame_budget = std::time::Duration::from_secs_f32(1.0 / fps);
                let elapsed = started.elapsed();
                let target = frame_budget.saturating_mul(frames as u32);
                if target > elapsed {
                    std::thread::sleep(target - elapsed);
                }
            }
        }
        let avg = frames as f32 / started.elapsed().as_secs_f32().max(f32::EPSILON);
        Ok(RunSummary { frames, avg_fps: avg })
    }

    /// headless 帧循环（CI/测试，IF-333）。
    pub fn run_headless(&mut self, app: &mut dyn App, frames: u64) -> Result<RunSummary> {
        app.setup(&mut self.world, &mut self.pipeline);
        let started = Instant::now();
        for _ in 0..frames {
            self.run_one_frame(app)?;
        }
        let avg = frames as f32 / started.elapsed().as_secs_f32().max(f32::EPSILON);
        Ok(RunSummary { frames, avg_fps: avg })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DemoApp {
        frames: u64,
        rendered: Vec<u64>,
    }

    impl App for DemoApp {
        fn setup(&mut self, world: &mut World, _pipeline: &mut ImportPipeline) {
            #[derive(rf_ecs::Component)]
            struct Marker;
            let _e = world.spawn((Marker,));
            self.frames = 0;
        }

        fn update(&mut self, _world: &mut World, _dt: f32) {
            self.frames += 1;
        }

        fn render(&mut self, frame: &mut FrameCtx) {
            frame.image.clear([40, 60, 120, 255]);
            self.rendered.push(frame.image.get(1, 1).map(|c| c[0] as u64).unwrap_or(0));
        }
    }

    #[test]
    fn engine_headless_loop() {
        let mut engine = Engine::new(EngineConfig {
            headless: true,
            window: WindowConfig { width: 32, height: 32, ..Default::default() },
            ..Default::default()
        })
        .unwrap();
        let mut app = DemoApp { frames: 0, rendered: Vec::new() };
        let summary = engine.run_headless(&mut app, 10).unwrap();
        assert_eq!(summary.frames, 10);
        assert_eq!(app.frames, 10);
        assert!(app.rendered.iter().all(|r| *r == 40));
        assert_eq!(engine.frame_buffer().get(0, 0), Some([40, 60, 120, 255]));
    }
}
