//! RustForge 动画（IF-210 ~ IF-216）：骨骼/蒙皮/状态机/IK/时间轴。

use rf_math::{Mat2x3, Mat4, Quat, Transform, Transform2D, Vec2, Vec3};

// ---- 3D 骨骼（IF-210） ----

/// 骨骼。
#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    /// -1 = 根。
    pub parent: i32,
    pub local_bind: Transform,
}

/// 骨架。
#[derive(Debug, Clone, Default)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn add(&mut self, name: &str, parent: i32, local_bind: Transform) -> usize {
        self.bones.push(Bone { name: name.to_string(), parent, local_bind });
        self.bones.len() - 1
    }

    /// 全局绑定矩阵（父链复合）。
    pub fn global_binds(&self) -> Vec<Mat4> {
        let mut out = vec![Mat4::identity(); self.bones.len()];
        for i in 0..self.bones.len() {
            let b = &self.bones[i];
            let parent_m = if b.parent >= 0 { out[b.parent as usize] } else { Mat4::identity() };
            out[i] = parent_m.mul_mat4(b.local_bind.to_mat4());
        }
        out
    }

    pub fn inverse_binds(&self) -> Vec<Mat4> {
        self.global_binds().iter().map(|m| m.inverse()).collect()
    }
}

/// 姿态（局部空间）。
#[derive(Debug, Clone, Default)]
pub struct Pose {
    pub locals: Vec<Transform>,
}

impl Pose {
    pub fn identity(skeleton: &Skeleton) -> Self {
        Self { locals: skeleton.bones.iter().map(|b| b.local_bind).collect() }
    }

    pub fn blend(&self, other: &Pose, t: f32) -> Pose {
        Pose {
            locals: self
                .locals
                .iter()
                .zip(&other.locals)
                .map(|(a, b)| Transform {
                    position: a.position.lerp(b.position, t),
                    rotation: a.rotation.slerp(b.rotation, t),
                    scale: a.scale.lerp(b.scale, t),
                })
                .collect(),
        }
    }
}

/// 蒙皮矩阵 = global_pose · inverse_bind。
pub fn skinning_matrices(skeleton: &Skeleton, pose: &Pose) -> Vec<Mat4> {
    let inv_binds = skeleton.inverse_binds();
    let mut globals = vec![Mat4::identity(); skeleton.bones.len()];
    for i in 0..skeleton.bones.len() {
        let parent_m = if skeleton.bones[i].parent >= 0 {
            globals[skeleton.bones[i].parent as usize]
        } else {
            Mat4::identity()
        };
        let local = pose.locals.get(i).copied().unwrap_or(Transform::identity());
        globals[i] = parent_m.mul_mat4(local.to_mat4());
    }
    globals.iter().zip(&inv_binds).map(|(g, ib)| g.mul_mat4(*ib)).collect()
}

// ---- 2D 骨骼（IF-211） ----

#[derive(Debug, Clone)]
pub struct Bone2 {
    pub name: String,
    pub parent: i32,
    pub local: Transform2D,
}

#[derive(Debug, Clone, Default)]
pub struct Skeleton2 {
    pub bones: Vec<Bone2>,
}

#[derive(Debug, Clone, Default)]
pub struct Pose2 {
    pub locals: Vec<Transform2D>,
}

impl Pose2 {
    pub fn identity(skeleton: &Skeleton2) -> Self {
        Self { locals: skeleton.bones.iter().map(|b| b.local).collect() }
    }

    pub fn blend(&self, other: &Pose2, t: f32) -> Pose2 {
        Pose2 { locals: self.locals.iter().zip(&other.locals).map(|(a, b)| a.lerp(b, t)).collect() }
    }
}

pub fn bone_world_matrices(skeleton: &Skeleton2, pose: &Pose2) -> Vec<Mat2x3> {
    let mut out = vec![Mat2x3::identity(); skeleton.bones.len()];
    for i in 0..skeleton.bones.len() {
        let parent_m = if skeleton.bones[i].parent >= 0 {
            out[skeleton.bones[i].parent as usize]
        } else {
            Mat2x3::identity()
        };
        let local = pose.locals.get(i).copied().unwrap_or(Transform2D::identity());
        out[i] = parent_m.mul_mat2x3(local.to_mat2x3());
    }
    out
}

// ---- 2D 帧动画（IF-212） ----

/// 帧步。
#[derive(Debug, Clone)]
pub struct FrameStep {
    pub sprite: String,
    pub duration: f32,
    pub event: Option<String>,
}

/// 帧动画剪辑。
#[derive(Debug, Clone, Default)]
pub struct FrameAnimation {
    pub frames: Vec<FrameStep>,
}

/// 播放事件。
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackEvent {
    FrameChanged(usize),
    Event(String),
}

/// 帧动画播放器（IF-212）。
pub struct AnimationPlayer2D {
    pub looping: bool,
    clip: Option<FrameAnimation>,
    time: f32,
    frame: usize,
}

impl Default for AnimationPlayer2D {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationPlayer2D {
    pub fn new() -> Self {
        Self { looping: true, clip: None, time: 0.0, frame: 0 }
    }

    pub fn play(&mut self, clip: FrameAnimation) {
        self.clip = Some(clip);
        self.time = 0.0;
        self.frame = 0;
    }

    pub fn current_sprite(&self) -> Option<&str> {
        self.clip.as_ref().and_then(|c| c.frames.get(self.frame)).map(|f| f.sprite.as_str())
    }

    pub fn progress(&self) -> f32 {
        let total: f32 =
            self.clip.as_ref().map(|c| c.frames.iter().map(|f| f.duration).sum()).unwrap_or(0.0);
        if total > 0.0 {
            (self.time / total).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// 推进并产出事件。
    pub fn update(&mut self, dt: f32) -> Vec<PlaybackEvent> {
        let mut events = Vec::new();
        let Some(clip) = &self.clip else { return events };
        if clip.frames.is_empty() {
            return events;
        }
        self.time += dt;
        let mut guard = 0;
        loop {
            let dur = clip.frames[self.frame].duration.max(f32::EPSILON);
            if self.time < dur || guard > 64 {
                break;
            }
            guard += 1;
            self.time -= dur;
            if let Some(ev) = &clip.frames[self.frame].event {
                events.push(PlaybackEvent::Event(ev.clone()));
            }
            let next = self.frame + 1;
            if next >= clip.frames.len() {
                if !self.looping {
                    self.time = 0.0;
                    break;
                }
                self.frame = 0;
                events.push(PlaybackEvent::FrameChanged(0));
            } else {
                self.frame = next;
                events.push(PlaybackEvent::FrameChanged(next));
            }
        }
        events
    }
}

// ---- 状态机（IF-213/214） ----

/// 泛型状态机（2D/3D 共用，IF-213）。
pub struct StateMachine<S: Copy + Eq> {
    current: S,
    previous: S,
    fade: f32,
    fade_duration: f32,
    transitions: Vec<(S, S, f32)>,
    force_queue: Vec<S>,
}

impl<S: Copy + Eq> StateMachine<S> {
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            previous: initial,
            fade: 1.0,
            fade_duration: 0.0,
            transitions: Vec::new(),
            force_queue: Vec::new(),
        }
    }

    pub fn add_transition(&mut self, from: S, to: S, fade: f32) {
        self.transitions.push((from, to, fade));
    }

    pub fn force(&mut self, s: S) {
        if s != self.current {
            let fade = self
                .transitions
                .iter()
                .find(|(f, t, _)| *f == self.current && *t == s)
                .map(|(_, _, d)| *d)
                .unwrap_or(0.0);
            self.previous = self.current;
            self.current = s;
            self.fade_duration = fade;
            self.fade = 0.0;
        }
    }

    pub fn queue_force(&mut self, s: S) {
        self.force_queue.push(s);
    }

    /// 返回 (当前, 前一, 混合权重 0..1)。
    pub fn update(&mut self, dt: f32) -> (S, S, f32) {
        if !self.force_queue.is_empty() {
            let next = self.force_queue.remove(0);
            self.force(next);
        }
        if self.fade_duration > 0.0 {
            self.fade = (self.fade + dt / self.fade_duration).min(1.0);
        } else {
            self.fade = 1.0;
        }
        (self.current, self.previous, self.fade)
    }

    pub fn current(&self) -> S {
        self.current
    }

    /// 只读内部状态（sample 用，不推进）。
    pub fn peek_state(&self) -> (S, S, f32) {
        (self.current, self.previous, self.fade)
    }
}

/// 动画图（IF-214）：状态机 × 剪辑。
pub struct ClipStateMachine<S: Copy + Eq> {
    pub machine: StateMachine<S>,
    pub clips: Vec<(S, Pose)>,
}

impl<S: Copy + Eq> ClipStateMachine<S> {
    pub fn sample(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        let (cur, prev, t) = self.machine.peek_state();
        let blended = self.blended_pose(cur, prev, t);
        skinning_matrices(skeleton, &blended)
    }

    fn blended_pose(&self, cur: S, prev: S, t: f32) -> Pose {
        let cur_pose = self.clips.iter().find(|(s, _)| *s == cur).map(|(_, p)| p.clone());
        let prev_pose = self.clips.iter().find(|(s, _)| *s == prev).map(|(_, p)| p.clone());
        match (cur_pose, prev_pose) {
            (Some(c), Some(p)) => p.blend(&c, t),
            (Some(c), None) => c,
            (None, Some(p)) => p,
            _ => Pose::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.machine.update(dt);
    }
}

/// AnimationGraph marker trait（IF-214）。
pub trait AnimationGraph {
    fn update(&mut self, dt: f32);
}

impl<S: Copy + Eq> AnimationGraph for ClipStateMachine<S> {
    fn update(&mut self, dt: f32) {
        Self::update(self, dt)
    }
}

// ---- IK（IF-215） ----

/// 双骨骼解析 IK：返回 (中间关节, 末端)。
pub fn two_bone_ik(
    root: Vec3,
    mid_len: f32,
    end_len: f32,
    target: Vec3,
    pole: Vec3,
) -> (Vec3, Vec3) {
    let to_target = target - root;
    let dist = to_target.length().clamp(
        (mid_len - end_len).abs() + f32::EPSILON,
        (mid_len + end_len - f32::EPSILON).max(f32::EPSILON),
    );
    let dir = to_target.normalized();
    let cos_a = ((mid_len * mid_len + dist * dist - end_len * end_len) / (2.0 * mid_len * dist))
        .clamp(-1.0, 1.0);
    let angle = cos_a.acos();
    let side = (pole - root).normalized();
    let bend_dir = {
        let d = side - dir * side.dot(dir);
        if d.length_sq() < f32::EPSILON {
            side
        } else {
            d.normalized()
        }
    };
    let (cos, sin) = angle.sin_cos();
    let mid = root + (dir * cos + bend_dir * sin) * mid_len;
    let end_dir = (target - mid).normalized();
    let end = mid + end_dir * end_len;
    (mid, end)
}

/// FABRIK（3D，IF-215）。
pub fn fabrik(chain: &mut [Vec3], target: Vec3, iterations: usize, tolerance: f32) -> bool {
    if chain.len() < 2 {
        return false;
    }
    let root = chain[0];
    for _ in 0..iterations {
        let last = chain.len() - 1;
        if (chain[last] - target).length() <= tolerance {
            return true;
        }
        // 后向：末端钉到目标，逐节拉回
        chain[last] = target;
        for i in (0..chain.len() - 1).rev() {
            let d = chain[i] - chain[i + 1];
            let l = d.length().max(f32::EPSILON);
            chain[i] = chain[i + 1] + d / l;
        }
        // 前向：根重新锚定，逐节推出
        chain[0] = root;
        for i in 1..chain.len() {
            let d = chain[i] - chain[i - 1];
            let l = d.length().max(f32::EPSILON);
            chain[i] = chain[i - 1] + d / l;
        }
    }
    (chain[chain.len() - 1] - target).length() <= tolerance
}

/// FABRIK（2D，IF-215）。
pub fn ik_2d(chain: &mut [Vec2], target: Vec2, iterations: usize, tolerance: f32) -> bool {
    let mut c3: Vec<Vec3> = chain.iter().map(|p| Vec3::new(p.x, p.y, 0.0)).collect();
    let ok = fabrik(&mut c3, Vec3::new(target.x, target.y, 0.0), iterations, tolerance);
    for (p, q) in chain.iter_mut().zip(&c3) {
        p.x = q.x;
        p.y = q.y;
    }
    ok
}

// ---- 时间轴（IF-216，Sequencer-lite） ----

/// 关键帧插值 trait。
pub trait Interp: Copy {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Interp for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self + (*other - *self) * t
    }
}

impl Interp for Vec2 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }
}

impl Interp for Quat {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self.nlerp(*other, t)
    }
}

/// 关键帧轨道。
pub struct KeyTrack<T: Interp> {
    pub keys: Vec<(f32, T)>,
}

impl<T: Interp> Default for KeyTrack<T> {
    fn default() -> Self {
        Self { keys: Vec::new() }
    }
}

impl<T: Interp> KeyTrack<T> {
    pub fn add(&mut self, t: f32, v: T) {
        self.keys.push((t, v));
        self.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn sample(&self, t: f32) -> Option<T> {
        if self.keys.is_empty() {
            return None;
        }
        if t <= self.keys[0].0 {
            return Some(self.keys[0].1);
        }
        if t >= self.keys[self.keys.len() - 1].0 {
            return Some(self.keys[self.keys.len() - 1].1);
        }
        for i in 1..self.keys.len() {
            if t < self.keys[i].0 {
                let (t0, v0) = self.keys[i - 1];
                let (t1, v1) = self.keys[i];
                let u = (t - t0) / (t1 - t0).max(f32::EPSILON);
                return Some(v0.lerp(&v1, u));
            }
        }
        Some(self.keys[self.keys.len() - 1].1)
    }
}

/// 时间轴（tracks + 事件）。
#[derive(Default)]
pub struct Timeline {
    pub float_tracks: Vec<KeyTrack<f32>>,
    pub vec2_tracks: Vec<KeyTrack<Vec2>>,
    pub events: Vec<(f32, String)>,
}

/// 时间轴播放器（IF-216）。
pub struct TimelinePlayer {
    pub time: f32,
    pub playing: bool,
    pub looping: bool,
    fired: std::collections::HashSet<usize>,
}

impl Default for TimelinePlayer {
    fn default() -> Self {
        Self { time: 0.0, playing: true, looping: false, fired: Default::default() }
    }
}

impl TimelinePlayer {
    /// 推进并返回本帧触发的事件。
    pub fn update(&mut self, timeline: &Timeline, dt: f32) -> Vec<String> {
        if !self.playing {
            return Vec::new();
        }
        self.time += dt;
        let mut out = Vec::new();
        for (i, (t, name)) in timeline.events.iter().enumerate() {
            if *t <= self.time && !self.fired.contains(&i) {
                self.fired.insert(i);
                out.push(name.clone());
            }
        }
        out
    }

    pub fn sample_f32(&self, timeline: &Timeline, track: usize) -> Option<f32> {
        timeline.float_tracks.get(track).and_then(|tr| tr.sample(self.time))
    }

    pub fn seek(&mut self, t: f32) {
        self.time = t;
        self.fired.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_bone_skeleton() -> Skeleton {
        let mut s = Skeleton::default();
        s.add("root", -1, Transform::identity());
        s.add("mid", 0, Transform { position: Vec3::new(0.0, 1.0, 0.0), ..Transform::identity() });
        s.add("end", 1, Transform { position: Vec3::new(0.0, 1.0, 0.0), ..Transform::identity() });
        s
    }

    #[test]
    fn skeleton_binds_and_skinning() {
        let s = two_bone_skeleton();
        let binds = s.global_binds();
        assert_eq!(binds.len(), 3);
        assert!((binds[2].translation().y - 2.0).abs() < 1e-5);
        let pose = Pose::identity(&s);
        let mats = skinning_matrices(&s, &pose);
        for m in &mats {
            assert!((m.m[0] - 1.0).abs() < 1e-4 && (m.m[5] - 1.0).abs() < 1e-4);
            // 绑定姿势 → 单位蒙皮
        }
    }

    #[test]
    fn pose_blend() {
        let s = two_bone_skeleton();
        let a = Pose::identity(&s);
        let mut b = a.clone();
        b.locals[1].position = Vec3::new(0.5, 2.0, 0.0);
        let mid = a.blend(&b, 0.5);
        assert!((mid.locals[1].position.x - 0.25).abs() < 1e-5);
        assert!((mid.locals[1].position.y - 1.5).abs() < 1e-5);
    }

    #[test]
    fn frame_animation_events() {
        let clip = FrameAnimation {
            frames: vec![
                FrameStep { sprite: "walk0".into(), duration: 0.1, event: None },
                FrameStep { sprite: "walk1".into(), duration: 0.1, event: Some("step".into()) },
            ],
        };
        let mut p = AnimationPlayer2D::new();
        p.play(clip);
        assert_eq!(p.current_sprite(), Some("walk0"));
        let evs = p.update(0.15);
        assert!(evs.contains(&PlaybackEvent::FrameChanged(1)));
        let evs = p.update(0.1);
        assert!(evs.contains(&PlaybackEvent::Event("step".into())));
        assert_eq!(p.current_sprite(), Some("walk0")); // 回环
    }

    #[test]
    fn state_machine_fade() {
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        enum St {
            Idle,
            Run,
        }
        let mut sm = StateMachine::new(St::Idle);
        sm.add_transition(St::Idle, St::Run, 0.2);
        let (_, _, t) = sm.update(0.0);
        assert_eq!(t, 1.0);
        sm.force(St::Run);
        let (cur, prev, t) = sm.update(0.1);
        assert_eq!((cur, prev), (St::Run, St::Idle));
        assert!((t - 0.5).abs() < 1e-5);
        let (_, _, t) = sm.update(0.2);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn two_bone_ik_reaches_target() {
        let (mid, end) =
            two_bone_ik(Vec3::ZERO, 1.0, 1.0, Vec3::new(1.5, 0.5, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let dist = (end - Vec3::new(1.5, 0.5, 0.0)).length();
        assert!(dist < 0.3, "end error {dist}");
        let mid_len = (mid - Vec3::ZERO).length();
        assert!(mid_len > 0.9 && mid_len < 1.1);
    }

    #[test]
    fn fabrik_converges() {
        let mut chain = vec![Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
        assert!(fabrik(&mut chain, Vec3::new(1.2, 1.2, 0.0), 20, 0.01));
        assert!((chain[2] - Vec3::new(1.2, 1.2, 0.0)).length() < 0.02);
        // 不可达 → 收敛到伸展方向且链长保持
        let mut c2 = vec![Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 2.0, 0.0)];
        assert!(!fabrik(&mut c2, Vec3::new(10.0, 0.0, 0.0), 10, 0.01));
        let l = (c2[1] - c2[0]).length();
        assert!((l - 1.0).abs() < 0.01);
    }

    #[test]
    fn ik_2d_works() {
        let mut chain = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)];
        assert!(ik_2d(&mut chain, Vec2::new(1.0, 1.0), 20, 0.01));
    }

    #[test]
    fn keytrack_and_timeline() {
        let mut tr = KeyTrack::<f32>::default();
        tr.add(1.0, 10.0);
        tr.add(0.0, 0.0); // 乱序添加
        assert_eq!(tr.sample(-1.0), Some(0.0));
        assert_eq!(tr.sample(0.5), Some(5.0));
        assert_eq!(tr.sample(2.0), Some(10.0));
        let mut tl = Timeline::default();
        tl.float_tracks.push(tr);
        tl.events.push((0.5, "halfway".to_string()));
        let mut player = TimelinePlayer::default();
        let mut fired = Vec::new();
        for _ in 0..60 {
            fired.extend(player.update(&tl, 1.0 / 60.0));
        }
        assert!(fired.contains(&"halfway".to_string()));
        let v = player.sample_f32(&tl, 0).unwrap();
        assert!((v - 10.0).abs() < 0.05); // 60×(1/60) ≈ 1.0
        player.seek(0.0);
        assert!(player.fired.is_empty());
    }

    #[test]
    fn skeleton2_bone_world() {
        let mut sk = Skeleton2::default();
        sk.bones.push(Bone2 { name: "a".into(), parent: -1, local: Transform2D::identity() });
        sk.bones.push(Bone2 {
            name: "b".into(),
            parent: 0,
            local: Transform2D { position: Vec2::new(2.0, 0.0), ..Transform2D::identity() },
        });
        let pose = Pose2::identity(&sk);
        let world = bone_world_matrices(&sk, &pose);
        let end = world[1].transform_point(Vec2::ZERO);
        assert!((end.x - 2.0).abs() < 1e-5);
    }
}
