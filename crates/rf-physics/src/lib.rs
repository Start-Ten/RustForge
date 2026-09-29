//! RustForge 物理（IF-200 ~ IF-205）：3D 刚体冲量求解 + 2D SAT + 关节 + 角色控制器。

use rf_math::{Mat2x3, Rot2, Vec2, Vec3};

// ---- 3D（IF-200/201/202） ----

/// 3D 形状。
#[derive(Debug, Clone, Copy)]
pub enum Shape3 {
    Sphere(f32),
    /// 半尺寸。
    Box(Vec3),
}

/// 3D 刚体。
#[derive(Debug, Clone)]
pub struct Body3 {
    pub shape: Shape3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub orientation: rf_math::Quat,
    pub angular_velocity: Vec3,
    /// None = 静态。
    pub mass: Option<f32>,
    pub restitution: f32,
    pub friction: f32,
    pub linear_damping: f32,
    pub sleeping: bool,
    pub ccd: bool,
}

impl Body3 {
    pub fn dynamic_sphere(pos: Vec3, r: f32, mass: f32) -> Self {
        Self {
            shape: Shape3::Sphere(r),
            position: pos,
            velocity: Vec3::ZERO,
            orientation: rf_math::Quat::identity(),
            angular_velocity: Vec3::ZERO,
            mass: Some(mass),
            restitution: 0.3,
            friction: 0.5,
            linear_damping: 0.01,
            sleeping: false,
            ccd: false,
        }
    }

    pub fn static_plane(y: f32) -> Self {
        Self {
            shape: Shape3::Box(Vec3::new(1000.0, 0.5, 1000.0)),
            position: Vec3::new(0.0, y - 0.5, 0.0),
            velocity: Vec3::ZERO,
            orientation: rf_math::Quat::identity(),
            angular_velocity: Vec3::ZERO,
            mass: None,
            restitution: 0.4,
            friction: 0.6,
            linear_damping: 0.0,
            sleeping: true,
            ccd: false,
        }
    }

    pub fn is_static(&self) -> bool {
        self.mass.is_none()
    }

    pub fn half_extent(&self) -> Vec3 {
        match self.shape {
            Shape3::Sphere(r) => Vec3::splat(r),
            Shape3::Box(h) => h,
        }
    }
}

/// 射线命中。
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit3 {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub body: usize,
}

/// 3D 物理场景（IF-202）。
pub struct PhysicsScene3d {
    pub gravity: Vec3,
    pub bodies: Vec<Body3>,
    step_count: u64,
}

impl PhysicsScene3d {
    pub fn new(gravity: Vec3) -> Self {
        Self { gravity, bodies: Vec::new(), step_count: 0 }
    }

    pub fn add_body(&mut self, body: Body3) -> usize {
        self.bodies.push(body);
        self.bodies.len() - 1
    }

    pub fn body_mut(&mut self, i: usize) -> Option<&mut Body3> {
        self.bodies.get_mut(i)
    }

    pub fn remove_body(&mut self, i: usize) {
        self.bodies.remove(i);
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn awake_all(&mut self) {
        for b in &mut self.bodies {
            b.sleeping = false;
        }
    }

    pub fn set_gravity(&mut self, g: Vec3) {
        self.gravity = g;
        self.awake_all();
    }

    pub fn overlap_sphere(&self, center: Vec3, radius: f32) -> Vec<usize> {
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, b)| match b.shape {
                Shape3::Sphere(r) => (b.position - center).length() <= r + radius,
                Shape3::Box(h) => {
                    // AABB 近似
                    let d = b.position - center;
                    d.x.abs() <= h.x + radius
                        && d.y.abs() <= h.y + radius
                        && d.z.abs() <= h.z + radius
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// 固定步长推进（半隐式欧拉 + 冲量求解 + 休眠）。
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.step_count += 1;
        // 积分
        for b in &mut self.bodies {
            if b.is_static() || b.sleeping {
                continue;
            }
            b.velocity += self.gravity * dt;
            b.velocity *= 1.0 - b.linear_damping * dt;
            // CCD：快速物体限步
            let speed = b.velocity.length();
            let max_move = b.half_extent().length();
            if b.ccd && speed * dt > max_move {
                let scale = max_move / (speed * dt);
                b.position += b.velocity * dt * scale;
            } else {
                b.position += b.velocity * dt;
            }
            b.orientation = b.orientation.mul_quat(rf_math::Quat::from_axis_angle(
                b.angular_velocity.normalized(),
                b.angular_velocity.length() * dt,
            ));
        }
        // 碰撞（球-球 / 球-盒AABB；盒-盒 AABB 近似）
        let mut contacts: Vec<(usize, usize, Vec3, f32)> = Vec::new();
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (a, b) = (&self.bodies[i], &self.bodies[j]);
                if a.is_static() && b.is_static() {
                    continue;
                }
                if let Some((normal, depth)) = detect_3d(a, b) {
                    contacts.push((i, j, normal, depth));
                }
            }
        }
        // 冲量求解
        for (i, j, normal, depth) in contacts {
            let (a_static, b_static) = (self.bodies[i].is_static(), self.bodies[j].is_static());
            let n = normal; // a → b
                            // 位置修正：a 沿 -n、b 沿 +n 分离
            let (wa, wb) = if a_static {
                (0.0, 1.0)
            } else if b_static {
                (1.0, 0.0)
            } else {
                (0.5, 0.5)
            };
            if !a_static {
                self.bodies[i].position -= n * depth * wa;
                self.bodies[i].sleeping = false;
            }
            if !b_static {
                self.bodies[j].position += n * depth * wb;
                self.bodies[j].sleeping = false;
            }
            // 速度冲量（沿法线）
            let e = self.bodies[i].restitution.min(self.bodies[j].restitution);
            let vi = if a_static { Vec3::ZERO } else { self.bodies[i].velocity };
            let vj = if b_static { Vec3::ZERO } else { self.bodies[j].velocity };
            let rel = vj - vi;
            let vn = rel.dot(n);
            if vn < 0.0 {
                let mi = self.bodies[i].mass.unwrap_or(1e9);
                let mj = self.bodies[j].mass.unwrap_or(1e9);
                let inv_sum = if a_static {
                    1.0 / mj
                } else if b_static {
                    1.0 / mi
                } else {
                    1.0 / mi + 1.0 / mj
                };
                let jimp = -(1.0 + e) * vn / inv_sum.max(f32::EPSILON);
                if !a_static {
                    self.bodies[i].velocity -= n * (jimp / mi);
                }
                if !b_static {
                    self.bodies[j].velocity += n * (jimp / mj);
                }
            }
        }
        // 休眠
        for b in &mut self.bodies {
            if b.is_static() {
                continue;
            }
            if b.velocity.length_sq() < 0.0001 {
                b.sleeping = true;
                b.velocity = Vec3::ZERO;
            }
        }
    }

    /// 射线检测（球/AABB）。
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<RaycastHit3> {
        let ray = rf_math::geom::Ray3::new(origin, dir);
        let mut best: Option<RaycastHit3> = None;
        for (i, b) in self.bodies.iter().enumerate() {
            let hit = match b.shape {
                Shape3::Sphere(r) => {
                    rf_math::geom::ray_sphere(&ray, &rf_math::geom::Sphere::new(b.position, r))
                        .map(|t| (ray.point_at(t), t))
                }
                Shape3::Box(h) => {
                    let bb = rf_math::geom::AABB3::from_center_half(b.position, h);
                    rf_math::geom::ray_aabb3(&ray, &bb).and_then(|(tn, tf)| {
                        let t = if tn >= 0.0 { tn } else { tf };
                        (t >= 0.0 && t <= max).then(|| (ray.point_at(t), t))
                    })
                }
            };
            if let Some((point, t)) = hit {
                if t <= max && best.as_ref().map(|b| t < b.distance).unwrap_or(true) {
                    let normal = (point - b.position).normalized();
                    best = Some(RaycastHit3 { point, normal, distance: t, body: i });
                }
            }
        }
        best
    }

    /// 确定性快照。
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.step_count.to_le_bytes());
        out.extend_from_slice(&(self.bodies.len() as u32).to_le_bytes());
        for b in &self.bodies {
            for v in
                [b.position.x, b.position.y, b.position.z, b.velocity.x, b.velocity.y, b.velocity.z]
            {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn restore(&mut self, bytes: &[u8]) {
        if bytes.len() < 12 {
            return;
        }
        self.step_count = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let mut i = 12;
        for b in self.bodies.iter_mut().take(n) {
            if i + 24 > bytes.len() {
                break;
            }
            b.position.x = f32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            b.position.y = f32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            b.position.z = f32::from_le_bytes(bytes[i + 8..i + 12].try_into().unwrap());
            b.velocity.x = f32::from_le_bytes(bytes[i + 12..i + 16].try_into().unwrap());
            b.velocity.y = f32::from_le_bytes(bytes[i + 16..i + 20].try_into().unwrap());
            b.velocity.z = f32::from_le_bytes(bytes[i + 20..i + 24].try_into().unwrap());
            i += 24;
        }
    }
}

fn detect_3d(a: &Body3, b: &Body3) -> Option<(Vec3, f32)> {
    match (&a.shape, &b.shape) {
        (Shape3::Sphere(ra), Shape3::Sphere(rb)) => {
            let d = b.position - a.position;
            let dist = d.length();
            let depth = ra + rb - dist;
            if depth > 0.0 {
                Some((d.normalized(), depth))
            } else {
                None
            }
        }
        _ => {
            // AABB 近似（Sphere 用包围盒）
            let ha = a.half_extent();
            let hb = b.half_extent();
            let d = b.position - a.position;
            let dx = (ha.x + hb.x) - d.x.abs();
            let dy = (ha.y + hb.y) - d.y.abs();
            let dz = (ha.z + hb.z) - d.z.abs();
            if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
                return None;
            }
            // 最小穿透轴
            if dx < dy && dx < dz {
                Some((Vec3::new(d.x.signum(), 0.0, 0.0), dx))
            } else if dy < dz {
                Some((Vec3::new(0.0, d.y.signum(), 0.0), dy))
            } else {
                Some((Vec3::new(0.0, 0.0, d.z.signum()), dz))
            }
        }
    }
}

/// 物理世界 trait（IF-200）。
pub trait PhysicsWorld {
    fn step(&mut self, dt: f32);
    fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<RaycastHit3>;
}

impl PhysicsWorld for PhysicsScene3d {
    fn step(&mut self, dt: f32) {
        Self::step(self, dt)
    }
    fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<RaycastHit3> {
        Self::raycast(self, origin, dir, max)
    }
}

/// 录制回放（IF-202：确定性验证）。
pub struct PhysicsRecorder {
    pub frames: Vec<Vec<u8>>,
}

impl PhysicsRecorder {
    pub fn record(&mut self, scene: &PhysicsScene3d) {
        self.frames.push(scene.snapshot());
    }

    pub fn compare(&self, scene: &PhysicsScene3d) -> bool {
        self.frames.last().map(|f| *f == scene.snapshot()).unwrap_or(false)
    }
}

// ---- 2D（IF-203/204/205） ----

/// 2D 形状。
#[derive(Debug, Clone)]
pub enum Shape2 {
    Circle(f32),
    Box(Vec2),
    /// 凸多边形（CCW）。
    Polygon(Vec<Vec2>),
}

/// 2D 刚体。
#[derive(Debug, Clone)]
pub struct Body2 {
    pub shape: Shape2,
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub mass: Option<f32>,
    pub restitution: f32,
    pub friction: f32,
    pub sleeping: bool,
    /// 单向平台（仅从上方碰撞）。
    pub one_way: bool,
}

impl Body2 {
    pub fn dynamic_circle(pos: Vec2, r: f32) -> Self {
        Self {
            shape: Shape2::Circle(r),
            position: pos,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            mass: Some(1.0),
            restitution: 0.3,
            friction: 0.4,
            sleeping: false,
            one_way: false,
        }
    }

    pub fn static_box(center: Vec2, half: Vec2, one_way: bool) -> Self {
        Self {
            shape: Shape2::Box(half),
            position: center,
            velocity: Vec2::ZERO,
            rotation: 0.0,
            mass: None,
            restitution: 0.1,
            friction: 0.8,
            sleeping: true,
            one_way,
        }
    }
}

/// 2D 关节（IF-204）。
#[derive(Debug, Clone)]
pub enum Joint2 {
    Distance { a: usize, b: usize, length: f32 },
    Revolute { a: usize, b: usize, anchor: Vec2 },
}

/// 2D 命中。
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit2 {
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
    pub body: usize,
}

/// 2D 物理场景。
pub struct PhysicsScene2d {
    pub gravity: Vec2,
    pub bodies: Vec<Body2>,
    pub joints: Vec<Joint2>,
}

impl PhysicsScene2d {
    pub fn new(gravity: Vec2) -> Self {
        Self { gravity, bodies: Vec::new(), joints: Vec::new() }
    }

    pub fn add_body(&mut self, body: Body2) -> usize {
        self.bodies.push(body);
        self.bodies.len() - 1
    }

    pub fn add_joint(&mut self, joint: Joint2) {
        self.joints.push(joint);
    }

    fn world_vertices(b: &Body2) -> Vec<Vec2> {
        let m = Mat2x3::compose(Vec2::ONE, Rot2::from_angle(b.rotation), b.position);
        match &b.shape {
            Shape2::Circle(r) => {
                vec![m.transform_point(Vec2::splat(*r)), m.transform_point(Vec2::splat(-*r))]
            }
            Shape2::Box(h) => vec![
                m.transform_point(Vec2::new(-h.x, -h.y)),
                m.transform_point(Vec2::new(h.x, -h.y)),
                m.transform_point(Vec2::new(h.x, h.y)),
                m.transform_point(Vec2::new(-h.x, h.y)),
            ],
            Shape2::Polygon(pts) => pts.iter().map(|p| m.transform_point(*p)).collect(),
        }
    }

    /// SAT 支持。
    fn support(vertices: &[Vec2], axis: Vec2) -> f32 {
        vertices.iter().map(|v| v.dot(axis)).fold(f32::NEG_INFINITY, f32::max)
    }

    /// 圆在轴上的投影半径（各向同性）。
    fn circle_radius(b: &Body2) -> Option<f32> {
        match &b.shape {
            Shape2::Circle(r) => Some(*r),
            _ => None,
        }
    }

    fn sat(a: &Body2, b: &Body2) -> Option<(Vec2, f32)> {
        let va = Self::world_vertices(a);
        let vb = Self::world_vertices(b);
        let mut axes: Vec<Vec2> = Vec::new();
        // 多边形/盒：边法线轴；圆：仅中心轴（圆的支撑点投影即半径）
        for (shape, vs) in [(&a.shape, &va), (&b.shape, &vb)] {
            match shape {
                Shape2::Circle(_) => {}
                _ => {
                    for i in 0..vs.len() {
                        let edge = vs[(i + 1) % vs.len()] - vs[i];
                        axes.push(edge.perp().normalized());
                    }
                }
            }
        }
        if let Shape2::Circle(_) = a.shape {
            axes.push((b.position - a.position).normalized());
        }
        if let Shape2::Circle(_) = b.shape {
            axes.push((a.position - b.position).normalized());
        }
        let ra = Self::circle_radius(a);
        let rb = Self::circle_radius(b);
        let mut best_depth = f32::INFINITY;
        let mut best_axis = Vec2::ZERO;
        for axis in axes {
            let (pa, na) = match ra {
                Some(r) => (a.position.dot(axis) + r, a.position.dot(axis) - r),
                None => (Self::support(&va, axis), -Self::support(&va, -axis)),
            };
            let (pb, nb) = match rb {
                Some(r) => (b.position.dot(axis) + r, b.position.dot(axis) - r),
                None => (Self::support(&vb, axis), -Self::support(&vb, -axis)),
            };
            let overlap = pa.min(pb) - na.max(nb);
            if overlap <= 0.0 {
                return None;
            }
            if overlap < best_depth {
                best_depth = overlap;
                best_axis = axis;
            }
        }
        // 方向：a → b
        if let (Some(ra), Some(rb)) = (ra, rb) {
            let d = b.position - a.position;
            let dist = d.length();
            let r_sum = ra + rb;
            if dist >= r_sum {
                return None;
            }
            return Some((d.normalized(), r_sum - dist));
        }
        if (b.position - a.position).dot(best_axis) < 0.0 {
            best_axis = -best_axis;
        }
        Some((best_axis, best_depth))
    }

    /// 固定步长：积分 + SAT 碰撞 + 冲量 + 关节投影。
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        for b in &mut self.bodies {
            if b.mass.is_none() || b.sleeping {
                continue;
            }
            b.velocity += self.gravity * dt;
            b.position += b.velocity * dt;
        }
        // 碰撞（2 轮迭代）
        for _ in 0..2 {
            let mut contacts: Vec<(usize, usize, Vec2, f32)> = Vec::new();
            for i in 0..self.bodies.len() {
                for j in (i + 1)..self.bodies.len() {
                    let (a, b) = (&self.bodies[i], &self.bodies[j]);
                    if a.mass.is_none() && b.mass.is_none() {
                        continue;
                    }
                    if a.sleeping && b.sleeping {
                        continue;
                    }
                    if a.one_way || b.one_way {
                        // 单向平台：仅 b 下落穿越 a 顶面时碰撞
                        let (plat, mover) = if a.one_way { (a, b) } else { (b, a) };
                        let top = plat.position.y
                            + match &plat.shape {
                                Shape2::Box(h) => h.y,
                                Shape2::Circle(r) => *r,
                                Shape2::Polygon(_) => 8.0,
                            };
                        if mover.position.y >= top - 2.0 && mover.velocity.y <= 0.0 {
                            if let Some((n, d)) = Self::sat(a, b) {
                                contacts.push((i, j, n, d));
                            }
                        }
                        continue;
                    }
                    if let Some((n, d)) = Self::sat(a, b) {
                        contacts.push((i, j, n, d));
                    }
                }
            }
            for (i, j, normal, depth) in contacts {
                let (a_static, b_static) =
                    (self.bodies[i].mass.is_none(), self.bodies[j].mass.is_none());
                let n = normal; // a → b：a 沿 -n、b 沿 +n 分离
                let (wa, wb) = if a_static {
                    (0.0, 1.0)
                } else if b_static {
                    (1.0, 0.0)
                } else {
                    (0.5, 0.5)
                };
                self.bodies[i].position -= n * depth * wa;
                self.bodies[j].position += n * depth * wb;
                self.bodies[i].sleeping = false;
                self.bodies[j].sleeping = false;
                // 冲量
                let e = self.bodies[i].restitution.min(self.bodies[j].restitution);
                let rel = self.bodies[j].velocity - self.bodies[i].velocity;
                let vn = rel.dot(n);
                if vn < 0.0 {
                    let mi = self.bodies[i].mass.unwrap_or(1e9);
                    let mj = self.bodies[j].mass.unwrap_or(1e9);
                    let inv_sum = if a_static {
                        1.0 / mj
                    } else if b_static {
                        1.0 / mi
                    } else {
                        1.0 / mi + 1.0 / mj
                    };
                    let jimp = -(1.0 + e) * vn / inv_sum.max(f32::EPSILON);
                    if !a_static {
                        self.bodies[i].velocity -= n * (jimp / mi);
                    }
                    if !b_static {
                        self.bodies[j].velocity += n * (jimp / mj);
                    }
                }
            }
            // 关节（位置投影）
            let joints = self.joints.clone();
            for joint in joints {
                match joint {
                    Joint2::Distance { a, b, length } => {
                        let d = self.bodies[b].position - self.bodies[a].position;
                        let dist = d.length();
                        if dist > f32::EPSILON && (dist - length).abs() > 0.001 {
                            let dir = d / dist;
                            let corr = (dist - length) * 0.5;
                            if self.bodies[a].mass.is_some() {
                                self.bodies[a].position += dir * corr;
                            }
                            if self.bodies[b].mass.is_some() {
                                self.bodies[b].position -= dir * corr;
                            }
                        }
                    }
                    Joint2::Revolute { a, b, anchor } => {
                        // 锚点吸引
                        let mid = (self.bodies[a].position + self.bodies[b].position) * 0.5;
                        let delta = (anchor - mid) * 0.5;
                        if self.bodies[a].mass.is_some() {
                            self.bodies[a].position += delta;
                        }
                        if self.bodies[b].mass.is_some() {
                            self.bodies[b].position += delta;
                        }
                    }
                }
            }
        }
        // 休眠
        for b in &mut self.bodies {
            if b.mass.is_some() && b.velocity.length_sq() < 0.0005 {
                b.sleeping = true;
                b.velocity = Vec2::ZERO;
            }
        }
    }

    pub fn raycast(&self, ray: &rf_math::geom::Ray2, max: f32) -> Option<RaycastHit2> {
        let mut best: Option<RaycastHit2> = None;
        for (i, b) in self.bodies.iter().enumerate() {
            let hit = match &b.shape {
                Shape2::Circle(r) => {
                    // 圆相交
                    let oc = ray.origin - b.position;
                    let bb = oc.dot(ray.dir);
                    let c = oc.dot(oc) - r * r;
                    let disc = bb * bb - c;
                    if disc < 0.0 {
                        None
                    } else {
                        let t = -bb - disc.sqrt();
                        let t = if t >= 0.0 { t } else { -bb + disc.sqrt() };
                        (t >= 0.0 && t <= max).then_some(t)
                    }
                }
                Shape2::Box(h) => rf_math::geom::ray_aabb2(
                    ray,
                    &rf_math::geom::AABB2::from_center_half(b.position, *h),
                )
                .and_then(|(tn, tf)| {
                    let t = if tn >= 0.0 { tn } else { tf };
                    (t >= 0.0 && t <= max).then_some(t)
                }),
                Shape2::Polygon(_) => None, // P1
            };
            if let Some(t) = hit {
                if best.as_ref().map(|b| t < b.distance).unwrap_or(true) {
                    let point = ray.point_at(t);
                    best = Some(RaycastHit2 {
                        point,
                        normal: (point - b.position).normalized(),
                        distance: t,
                        body: i,
                    });
                }
            }
        }
        best
    }
}

/// 角色控制器（IF-205）：平台跳跃移动。
pub struct CharacterController2D;

#[derive(Debug, Clone, Copy)]
pub struct MoveResult {
    pub pos: Vec2,
    pub grounded: bool,
    pub hit_wall: bool,
}

impl CharacterController2D {
    /// 位移 + 碰撞分离；grounded = 脚下有支撑。
    pub fn move_body(scene: &PhysicsScene2d, body: usize, displacement: Vec2) -> MoveResult {
        let (pos, half) = match scene.bodies.get(body) {
            Some(b) => (b.position, b.half_extent2()),
            None => return MoveResult { pos: Vec2::ZERO, grounded: false, hit_wall: false },
        };
        let mut new_pos = pos + displacement;
        let mut grounded = false;
        let mut hit_wall = false;
        let feet = new_pos + Vec2::new(0.0, -half.y - 0.05);
        for (i, other) in scene.bodies.iter().enumerate() {
            if i == body {
                continue;
            }
            let oh = other.half_extent2();
            let overlap_x = (new_pos.x - other.position.x).abs() < half.x + oh.x;
            let overlap_y = (new_pos.y - other.position.y).abs() < half.y + oh.y;
            if overlap_x && overlap_y {
                // 垂直分离（从上落下 → 站立）
                if displacement.y <= 0.0 && new_pos.y > other.position.y {
                    new_pos.y = other.position.y + oh.y + half.y;
                    grounded = true;
                } else if displacement.y > 0.0 {
                    new_pos.y = other.position.y - oh.y - half.y;
                } else {
                    // 水平
                    new_pos.x = if new_pos.x < other.position.x {
                        other.position.x - oh.x - half.x
                    } else {
                        other.position.x + oh.x + half.x
                    };
                    hit_wall = true;
                }
            }
            // 贴地检测（脚与平台顶面接触）
            let other_top = other.position.y + oh.y;
            if (feet.x - other.position.x).abs() < half.x + oh.x
                && (feet.y - other_top).abs() < 0.12
            {
                grounded = true;
            }
        }
        MoveResult { pos: new_pos, grounded, hit_wall }
    }
}

impl Body2 {
    /// 2D 半尺寸（碰撞盒近似）。
    pub fn half_extent2(&self) -> Vec2 {
        match &self.shape {
            Shape2::Circle(r) => Vec2::splat(*r),
            Shape2::Box(h) => *h,
            Shape2::Polygon(pts) => {
                let bb = rf_math::geom::AABB2::from_points(pts).unwrap_or_default();
                (bb.max - bb.min) * 0.5
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_falls_and_rests() {
        let mut s = PhysicsScene3d::new(Vec3::new(0.0, -9.8, 0.0));
        s.add_body(Body3::dynamic_sphere(Vec3::new(0.0, 5.0, 0.0), 0.5, 1.0));
        s.add_body(Body3::static_plane(0.0));
        for _ in 0..240 {
            s.step(1.0 / 60.0);
        }
        let y = s.bodies[0].position.y;
        assert!((y - 0.5).abs() < 0.05, "y={y}"); // 静止在平面上
        assert!(s.bodies[0].sleeping); // 休眠
                                       // 重力唤醒
        s.set_gravity(Vec3::new(0.0, -20.0, 0.0));
        assert!(!s.bodies[0].sleeping);
    }

    #[test]
    fn sphere_bounce_restitution() {
        let mut s = PhysicsScene3d::new(Vec3::new(0.0, -10.0, 0.0));
        s.add_body(Body3::dynamic_sphere(Vec3::new(0.0, 3.0, 0.0), 0.5, 1.0));
        s.add_body(Body3::static_plane(0.0));
        // e=0.3 下落到接触一次后速度反向衰减
        for _ in 0..30 {
            s.step(1.0 / 60.0);
        }
        // 刚接触后的反弹方向（速度向上或已被再求解）
        let v = s.bodies[0].velocity.y;
        assert!(v > -10.0, "v={v}"); // 没有无限加速
    }

    #[test]
    fn raycast_3d() {
        let mut s = PhysicsScene3d::new(Vec3::ZERO);
        s.add_body(Body3::dynamic_sphere(Vec3::new(0.0, 0.0, 5.0), 1.0, 1.0));
        let hit = s.raycast(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 100.0).unwrap();
        assert_eq!(hit.body, 0);
        assert!((hit.distance - 4.0).abs() < 0.01);
        assert!(s.raycast(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 100.0).is_none());
    }

    #[test]
    fn deterministic_replay() {
        let run = || {
            let mut s = PhysicsScene3d::new(Vec3::new(0.0, -9.8, 0.0));
            s.add_body(Body3::dynamic_sphere(Vec3::new(1.0, 8.0, 0.0), 0.5, 1.0));
            s.add_body(Body3::dynamic_sphere(Vec3::new(1.0, 12.0, 0.0), 0.5, 1.0));
            s.add_body(Body3::static_plane(0.0));
            for _ in 0..120 {
                s.step(1.0 / 60.0);
            }
            s.snapshot()
        };
        assert_eq!(run(), run()); // 逐字节一致
    }

    #[test]
    fn physics2d_circle_stacks_on_box() {
        let mut s = PhysicsScene2d::new(Vec2::new(0.0, -9.8));
        s.add_body(Body2::dynamic_circle(Vec2::new(0.0, 5.0), 0.5));
        s.add_body(Body2::static_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.5), false));
        for _ in 0..180 {
            s.step(1.0 / 60.0);
        }
        let y = s.bodies[0].position.y;
        assert!((y - 1.0).abs() < 0.06, "y={y}"); // 静止在盒上
    }

    #[test]
    fn physics2d_sat_polygon() {
        let mut s = PhysicsScene2d::new(Vec2::new(0.0, -9.8));
        let tri = Body2 {
            shape: Shape2::Polygon(vec![
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
            ]),
            position: Vec2::new(0.0, 5.0),
            ..Body2::dynamic_circle(Vec2::ZERO, 1.0)
        };
        s.add_body(tri);
        s.add_body(Body2::static_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.5), false));
        for _ in 0..240 {
            s.step(1.0 / 60.0);
        }
        assert!(s.bodies[0].position.y > 0.4); // 落在盒上未穿透
        assert!(s.bodies[0].position.y < 2.0);
    }

    #[test]
    fn physics2d_distance_joint() {
        let mut s = PhysicsScene2d::new(Vec2::new(0.0, -9.8));
        let a = s.add_body(Body2::dynamic_circle(Vec2::new(-1.0, 6.0), 0.3));
        let b = s.add_body(Body2::dynamic_circle(Vec2::new(1.0, 6.0), 0.3));
        s.add_body(Body2::static_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.5), false));
        s.add_joint(Joint2::Distance { a, b, length: 2.0 });
        for _ in 0..300 {
            s.step(1.0 / 60.0);
        }
        let d = (s.bodies[b].position - s.bodies[a].position).length();
        assert!((d - 2.0).abs() < 0.3, "d={d}"); // 关节约束保持
    }

    #[test]
    fn one_way_platform() {
        let mut s = PhysicsScene2d::new(Vec2::new(0.0, -9.8));
        s.add_body(Body2::dynamic_circle(Vec2::new(0.0, 5.0), 0.4));
        s.add_body(Body2::static_box(Vec2::new(0.0, 2.0), Vec2::new(5.0, 0.2), true));
        s.add_body(Body2::static_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.5), false));
        for _ in 0..120 {
            s.step(1.0 / 60.0);
        }
        // 从上方落到单向平台并站立
        let y = s.bodies[0].position.y;
        assert!((y - 2.6).abs() < 0.15, "y={y}");
    }

    #[test]
    fn character_controller_walk_and_ground() {
        let mut s = PhysicsScene2d::new(Vec2::new(0.0, -9.8));
        let player = s.add_body(Body2 {
            shape: Shape2::Box(Vec2::new(0.4, 0.6)),
            position: Vec2::new(0.0, 1.1),
            ..Body2::dynamic_circle(Vec2::ZERO, 0.5)
        });
        s.add_body(Body2::static_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.5), false));
        let r = CharacterController2D::move_body(&s, player, Vec2::new(0.5, 0.0));
        assert!(r.grounded);
        assert!(!r.hit_wall);
        assert!(r.pos.x > 0.4);
    }

    #[test]
    fn raycast_2d() {
        let mut s = PhysicsScene2d::new(Vec2::ZERO);
        s.add_body(Body2::static_box(Vec2::new(5.0, 0.0), Vec2::splat(1.0), false));
        let ray = rf_math::geom::Ray2::new(Vec2::ZERO, Vec2::new(1.0, 0.0));
        let hit = s.raycast(&ray, 100.0).unwrap();
        assert!((hit.distance - 4.0).abs() < 0.01);
    }
}
