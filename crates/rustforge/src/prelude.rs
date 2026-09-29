//! RustForge prelude（IF-330）：常用类型 re-export。

pub use rf_core::{EngineError, Entity, Result, Rgba8Image, Tick, ENGINE_NAME, ENGINE_VERSION};
pub use rf_ecs::{
    Bundle, Command, Commands, Component, EventReader, Events, Parent, Query, Resource, Schedule,
    Stage, System, SystemSet, World,
};
pub use rf_math::{
    angle_lerp, clamp01, lerp_f32, smoothstep, Color, Mat2x3, Mat4, Quat, Rot2, Transform,
    Transform2D, Vec2, Vec3, Vec4,
};
pub use rf_render::{
    Camera2D, Camera3D, Light, Light2D, MaterialParams, Renderer2D, SoftwareRenderer2D,
};
pub use rf_task::{JobHandle, JobPriority, JobSystem};
