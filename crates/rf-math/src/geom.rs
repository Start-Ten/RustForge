//! 几何类型与相交测试（IF-026 ~ IF-029, IF-033）。

use crate::{Vec2, Vec3};

/// 2D 轴对齐包围盒（IF-026）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AABB2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB2 {
    pub fn from_center_half(center: Vec2, half: Vec2) -> Self {
        Self { min: center - half, max: center + half }
    }

    pub fn from_points(points: &[Vec2]) -> Option<Self> {
        let mut iter = points.iter();
        let first = *iter.next()?;
        let mut bb = Self { min: first, max: first };
        for p in iter {
            bb.expand(*p);
        }
        Some(bb)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn intersects(&self, o: &AABB2) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
    }

    pub fn union(&self, o: &AABB2) -> AABB2 {
        AABB2 { min: self.min.min(o.min), max: self.max.max(o.max) }
    }

    pub fn expand(&mut self, p: Vec2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// min > max（空/无效）判定。
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }
}

/// 3D 轴对齐包围盒（IF-026）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AABB3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB3 {
    pub fn from_center_half(center: Vec3, half: Vec3) -> Self {
        Self { min: center - half, max: center + half }
    }

    pub fn from_points(points: &[Vec3]) -> Option<Self> {
        let mut iter = points.iter();
        let first = *iter.next()?;
        let mut bb = Self { min: first, max: first };
        for p in iter {
            bb.expand(*p);
        }
        Some(bb)
    }

    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn intersects(&self, o: &AABB3) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
            && self.min.z <= o.max.z
            && self.max.z >= o.min.z
    }

    pub fn union(&self, o: &AABB3) -> AABB3 {
        AABB3 { min: self.min.min(o.min), max: self.max.max(o.max) }
    }

    pub fn expand(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
}

/// 2D 圆 / 3D 球（IF-027）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }
    pub fn contains(&self, p: Vec2) -> bool {
        (p - self.center).length_sq() <= self.radius * self.radius
    }
    pub fn intersects(&self, o: &Circle) -> bool {
        (self.center - o.center).length() <= self.radius + o.radius
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
    pub fn contains(&self, p: Vec3) -> bool {
        (p - self.center).length_sq() <= self.radius * self.radius
    }
    pub fn intersects(&self, o: &Sphere) -> bool {
        (self.center - o.center).length() <= self.radius + o.radius
    }
}

/// 平面（IF-028）：`dot(normal, p) + d = 0`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn from_normal_point(normal: Vec3, point: Vec3) -> Self {
        let n = normal.normalized();
        Self { d: -n.dot(point), normal: n }
    }

    pub fn distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }

    /// 正 = 法线侧，负 = 背侧，0 = 面上。
    pub fn sign(&self, p: Vec3) -> f32 {
        self.distance(p).signum()
    }
}

/// 2D/3D 射线（IF-028）。dir 约定归一化。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray2 {
    pub origin: Vec2,
    pub dir: Vec2,
}

impl Ray2 {
    pub fn new(origin: Vec2, dir: Vec2) -> Self {
        Self { origin, dir: dir.normalized() }
    }
    pub fn point_at(&self, t: f32) -> Vec2 {
        self.origin + self.dir * t
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3 {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray3 {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self { origin, dir: dir.normalized() }
    }
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// 视锥体（IF-029）：左/右/上/下/近/远 六平面，法线朝内。
/// 【CP-001 变更提案】新增两个**私有**字段（`view_projection`/`perspective`）用于
/// 背面点判定（透视除法 w<0 的经典陷阱）；公开字段 `planes` 不变，构造仍仅经
/// `from_view_projection`，对 API 使用者零影响。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
    view_projection: crate::Mat4,
    perspective: bool,
}

impl Frustum {
    /// 从 view*projection 矩阵行提取（列主序 Mat4）。vp = proj * view。
    pub fn from_view_projection(vp: crate::Mat4) -> Self {
        let m = vp.m;
        // Gribb-Hartmann：plane = row3 ± row_i（列主序下 m[col*4+row]）
        let row = |r: usize, c: usize| m[c * 4 + r];
        let p = |i: usize, s: f32| -> Plane {
            let n = Vec3::new(
                row(0, 3) + s * row(0, i),
                row(1, 3) + s * row(1, i),
                row(2, 3) + s * row(2, i),
            );
            let d = row(3, 3) + s * row(3, i);
            let len = n.length().max(f32::EPSILON);
            Plane { normal: n / len, d: d / len }
        };
        let perspective = m[11] < -0.5; // row3.z == -1 → 透视
        Frustum {
            planes: [p(0, 1.0), p(0, -1.0), p(1, 1.0), p(1, -1.0), p(2, 1.0), p(2, -1.0)],
            view_projection: vp,
            perspective,
        }
    }

    /// 透视投影下先做 w>0（相机前方）判定，规避背面点经 6 平面测试的假阳性。
    pub fn contains_point(&self, p: Vec3) -> bool {
        if self.perspective {
            let m = &self.view_projection.m;
            let w = m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15];
            if w <= 0.0 {
                return false;
            }
        }
        self.planes.iter().all(|pl| pl.distance(p) >= 0.0)
    }

    /// 保守 AABB 相交（任一平面全外则剔除）。
    pub fn intersects_aabb(&self, bb: &AABB3) -> bool {
        for pl in &self.planes {
            let positive = Vec3::new(
                if pl.normal.x >= 0.0 { bb.max.x } else { bb.min.x },
                if pl.normal.y >= 0.0 { bb.max.y } else { bb.min.y },
                if pl.normal.z >= 0.0 { bb.max.z } else { bb.min.z },
            );
            if pl.distance(positive) < 0.0 {
                return false;
            }
        }
        true
    }
}

// ---- 相交（IF-033） ----

/// 射线 vs AABB2，返回 (t_near, t_far)；平行不相交返回 None。
pub fn ray_aabb2(ray: &Ray2, bb: &AABB2) -> Option<(f32, f32)> {
    let mut t_near = f32::MIN;
    let mut t_far = f32::MAX;
    for (o, d, mn, mx) in [
        (ray.origin.x, ray.dir.x, bb.min.x, bb.max.x),
        (ray.origin.y, ray.dir.y, bb.min.y, bb.max.y),
    ] {
        if d.abs() < f32::EPSILON {
            if o < mn || o > mx {
                return None;
            }
        } else {
            let mut t1 = (mn - o) / d;
            let mut t2 = (mx - o) / d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_near = t_near.max(t1);
            t_far = t_far.min(t2);
            if t_near > t_far {
                return None;
            }
        }
    }
    Some((t_near, t_far))
}

/// 射线 vs AABB3，返回 (t_near, t_far)；不相交返回 None。
pub fn ray_aabb3(ray: &Ray3, bb: &AABB3) -> Option<(f32, f32)> {
    let mut t_near = f32::MIN;
    let mut t_far = f32::MAX;
    for (o, d, mn, mx) in [
        (ray.origin.x, ray.dir.x, bb.min.x, bb.max.x),
        (ray.origin.y, ray.dir.y, bb.min.y, bb.max.y),
        (ray.origin.z, ray.dir.z, bb.min.z, bb.max.z),
    ] {
        if d.abs() < f32::EPSILON {
            if o < mn || o > mx {
                return None;
            }
        } else {
            let mut t1 = (mn - o) / d;
            let mut t2 = (mx - o) / d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_near = t_near.max(t1);
            t_far = t_far.min(t2);
            if t_near > t_far {
                return None;
            }
        }
    }
    Some((t_near, t_far))
}

/// 射线 vs 球，返回最近正 t。
pub fn ray_sphere(ray: &Ray3, s: &Sphere) -> Option<f32> {
    let oc = ray.origin - s.center;
    let b = oc.dot(ray.dir);
    let c = oc.dot(oc) - s.radius * s.radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t1 = -b - sq;
    let t2 = -b + sq;
    if t1 >= 0.0 {
        Some(t1)
    } else if t2 >= 0.0 {
        Some(t2)
    } else {
        None
    }
}

/// 射线 vs 平面，返回 t（背面相交也算）。
pub fn ray_plane(ray: &Ray3, pl: &Plane) -> Option<f32> {
    let denom = pl.normal.dot(ray.dir);
    if denom.abs() < f32::EPSILON {
        return None;
    }
    let t = -pl.distance(ray.origin) / denom;
    if t >= 0.0 {
        Some(t)
    } else {
        None
    }
}

/// 线段相交，返回交点。
pub fn segment_intersect_2d(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> Option<Vec2> {
    let d1 = a2 - a1;
    let d2 = b2 - b1;
    let denom = d1.cross(d2);
    if denom.abs() < f32::EPSILON {
        return None;
    }
    let t = (b1 - a1).cross(d2) / denom;
    let u = (b1 - a1).cross(d1) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(a1 + d1 * t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_ops() {
        let a = AABB2::from_center_half(Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0));
        assert!(a.contains(Vec2::new(1.5, 0.5)));
        assert!(!a.contains(Vec2::new(3.0, 3.0)));
        let b = AABB2::from_center_half(Vec2::new(2.6, 1.0), Vec2::new(1.0, 1.0));
        assert!(a.intersects(&b));
        let u = a.union(&b);
        assert!((u.size().x - 3.6).abs() < 1e-5);
        assert!(AABB2::from_points(&[]).is_none());
        assert!(AABB3::default().is_valid());
    }

    #[test]
    fn circle_sphere() {
        let c = Circle::new(Vec2::ZERO, 1.0);
        assert!(c.contains(Vec2::new(0.5, 0.5)));
        assert!(c.intersects(&Circle::new(Vec2::new(1.5, 0.0), 1.0)));
        let s = Sphere::new(Vec3::ZERO, 2.0);
        assert!(s.contains(Vec3::new(0.0, 0.0, 1.9)));
        assert!(!s.intersects(&Sphere::new(Vec3::new(5.0, 0.0, 0.0), 1.0)));
    }

    #[test]
    fn ray_hits() {
        let bb = AABB2::from_center_half(Vec2::ZERO, Vec2::splat(1.0));
        let r = Ray2::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
        let (tn, tf) = ray_aabb2(&r, &bb).unwrap();
        assert!((tn - 4.0).abs() < 1e-4 && (tf - 6.0).abs() < 1e-4);
        assert!(ray_aabb2(&Ray2::new(Vec2::new(-5.0, 5.0), Vec2::new(1.0, 0.0)), &bb).is_none());

        let s = Sphere::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let t = ray_sphere(&Ray3::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)), &s)
            .unwrap();
        assert!((t - 4.0).abs() < 1e-4);
        assert!(ray_sphere(&Ray3::new(Vec3::new(-5.0, 5.0, 0.0), Vec3::new(1.0, 0.0, 0.0)), &s)
            .is_none());

        let pl = Plane::from_normal_point(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 3.0));
        let t = ray_plane(&Ray3::new(Vec3::new(0.0, 0.0, -2.0), Vec3::new(0.0, 0.0, 1.0)), &pl)
            .unwrap();
        assert!((t - 5.0).abs() < 1e-4);
    }

    #[test]
    fn segment_cross() {
        let p = segment_intersect_2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(2.0, 0.0),
        )
        .unwrap();
        assert!((p.x - 1.0).abs() < 1e-5);
        assert!(segment_intersect_2d(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0)
        )
        .is_none());
    }

    #[test]
    fn frustum_culling() {
        let proj = crate::Mat4::perspective(1.2, 1.0, 0.1, 100.0);
        let view =
            crate::Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        let f = Frustum::from_view_projection(proj.mul_mat4(view));
        assert!(f.contains_point(Vec3::new(0.0, 0.0, 4.0))); // 相机前方 1 单位
        assert!(!f.contains_point(Vec3::new(0.0, 0.0, 6.0))); // 相机背后
        assert!(f.intersects_aabb(&AABB3::from_center_half(Vec3::ZERO, Vec3::splat(0.5))));
        assert!(!f.intersects_aabb(&AABB3::from_center_half(
            Vec3::new(0.0, 0.0, -200.0),
            Vec3::splat(1.0)
        )));
    }
}
