//! RustForge 数学库（IF-020 ~ IF-034）：2D/3D 平级。
//! 标量实现 + 布局对 SIMD 友好；SIMD 加速列 P1（见功能矩阵 A7）。

pub mod color;
pub mod curve;
pub mod geom;
pub mod noise;
pub mod transform;

pub use color::Color;
pub use curve::{Curve1, CurveKey};
pub use noise::{fbm_2d, perlin_2d, value_noise_2d};
pub use transform::{Transform, Transform2D};

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

macro_rules! vec_impl {
    ($name:ident, $($f:ident),+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct $name { $(pub $f: f32),+ }

        impl $name {
            pub const ZERO: $name = $name { $($f: 0.0),+ };
            pub const ONE: $name = $name { $($f: 1.0),+ };

            pub const fn new($($f: f32),+) -> Self { $name { $($f),+ } }
            pub const fn splat(v: f32) -> Self { $name { $($f: v),+ } }

            pub fn dot(self, o: Self) -> f32 { $(self.$f * o.$f +)+ 0.0 }
            pub fn length_sq(self) -> f32 { self.dot(self) }
            pub fn length(self) -> f32 { self.length_sq().sqrt() }
            /// 就地归一（零向量保持为零，避免 NaN）。
            pub fn normalize(&mut self) { let l = self.length(); if l > f32::EPSILON { let inv = 1.0 / l; $(self.$f *= inv;)+ } }
            /// 归一化副本（零向量返回零向量）。
            pub fn normalized(self) -> Self { let mut v = self; v.normalize(); v }
            pub fn lerp(self, o: Self, t: f32) -> Self { $name { $($f: self.$f + (o.$f - self.$f) * t),+ } }
            pub fn distance(self, o: Self) -> f32 { (self - o).length() }
            pub fn min(self, o: Self) -> Self { $name { $($f: self.$f.min(o.$f)),+ } }
            pub fn max(self, o: Self) -> Self { $name { $($f: self.$f.max(o.$f)),+ } }
            pub fn sum(self) -> f32 { $(self.$f +)+ 0.0 }
        }

        impl Add for $name { type Output = Self; fn add(self, o: Self) -> Self { $name { $($f: self.$f + o.$f),+ } } }
        impl Sub for $name { type Output = Self; fn sub(self, o: Self) -> Self { $name { $($f: self.$f - o.$f),+ } } }
        impl Mul<f32> for $name { type Output = Self; fn mul(self, s: f32) -> Self { $name { $($f: self.$f * s),+ } } }
        impl Div<f32> for $name { type Output = Self; fn div(self, s: f32) -> Self { let inv = 1.0 / s; $name { $($f: self.$f * inv),+ } } }
        impl Mul for $name { type Output = Self; fn mul(self, o: Self) -> Self { $name { $($f: self.$f * o.$f),+ } } }
        impl Div for $name { type Output = Self; fn div(self, o: Self) -> Self { $name { $($f: self.$f / o.$f),+ } } }
        impl Neg for $name { type Output = Self; fn neg(self) -> Self { $name { $($f: -self.$f),+ } } }
        impl AddAssign for $name { fn add_assign(&mut self, o: Self) { $(self.$f += o.$f;)+ } }
        impl SubAssign for $name { fn sub_assign(&mut self, o: Self) { $(self.$f -= o.$f;)+ } }
        impl MulAssign<f32> for $name { fn mul_assign(&mut self, s: f32) { $(self.$f *= s;)+ } }
        impl DivAssign<f32> for $name { fn div_assign(&mut self, s: f32) { $(self.$f /= s;)+ } }
    };
}

vec_impl!(Vec2, x, y);
vec_impl!(Vec3, x, y, z);
vec_impl!(Vec4, x, y, z, w);

impl Vec2 {
    pub fn from_angle(rad: f32) -> Self {
        Self { x: rad.cos(), y: rad.sin() }
    }

    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// 逆时针旋转 90°。
    pub fn perp(self) -> Self {
        Self { x: -self.y, y: self.x }
    }

    pub fn rotate(self, r: Rot2) -> Self {
        Self { x: self.x * r.cos - self.y * r.sin, y: self.x * r.sin + self.y * r.cos }
    }

    pub fn cross(self, o: Self) -> f32 {
        self.x * o.y - self.y * o.x
    }
}

impl Vec3 {
    pub fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }
}

/// 2D 旋转（IF-021）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rot2 {
    pub cos: f32,
    pub sin: f32,
}

impl Default for Rot2 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Rot2 {
    pub const fn identity() -> Self {
        Self { cos: 1.0, sin: 0.0 }
    }

    pub fn from_angle(rad: f32) -> Self {
        Self { cos: rad.cos(), sin: rad.sin() }
    }

    pub fn angle(&self) -> f32 {
        self.sin.atan2(self.cos)
    }

    pub fn mul_rot(self, o: Self) -> Self {
        Self { cos: self.cos * o.cos - self.sin * o.sin, sin: self.sin * o.cos + self.cos * o.sin }
    }

    pub fn inverse(self) -> Self {
        Self { cos: self.cos, sin: -self.sin }
    }

    pub fn rotate(self, v: Vec2) -> Vec2 {
        v.rotate(self)
    }
}

/// 2D 仿射矩阵（IF-022）。布局（列主序 2×3）：`[m00, m10, tx, m01, m11, ty]`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat2x3 {
    pub m: [f32; 6],
}

impl Default for Mat2x3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mat2x3 {
    pub const fn identity() -> Self {
        Self { m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] }
    }

    pub const fn from_scale(s: Vec2) -> Self {
        Self { m: [s.x, 0.0, 0.0, 0.0, s.y, 0.0] }
    }

    pub const fn from_translation(t: Vec2) -> Self {
        Self { m: [1.0, 0.0, t.x, 0.0, 1.0, t.y] }
    }

    /// T * R * S 复合。
    pub fn compose(scale: Vec2, rot: Rot2, trans: Vec2) -> Self {
        Self {
            m: [
                scale.x * rot.cos,
                scale.x * rot.sin,
                trans.x,
                -scale.y * rot.sin,
                scale.y * rot.cos,
                trans.y,
            ],
        }
    }

    pub fn mul_mat2x3(self, o: Self) -> Self {
        let a = self.m;
        let b = o.m;
        Self {
            m: [
                a[0] * b[0] + a[3] * b[1],
                a[1] * b[0] + a[4] * b[1],
                a[0] * b[2] + a[3] * b[3] + a[2],
                a[0] * b[4] + a[3] * b[5],
                a[1] * b[4] + a[4] * b[5],
                a[1] * b[2] + a[4] * b[3] + a[5],
            ],
        }
    }

    pub fn transform_point(self, p: Vec2) -> Vec2 {
        Vec2 {
            x: self.m[0] * p.x + self.m[3] * p.y + self.m[2],
            y: self.m[1] * p.x + self.m[4] * p.y + self.m[5],
        }
    }

    pub fn transform_vector(self, v: Vec2) -> Vec2 {
        Vec2 { x: self.m[0] * v.x + self.m[3] * v.y, y: self.m[1] * v.x + self.m[4] * v.y }
    }

    pub fn determinant(self) -> f32 {
        self.m[0] * self.m[4] - self.m[1] * self.m[3]
    }

    /// 奇异矩阵返回 identity（保护下游渲染/物理）。
    pub fn inverse(self) -> Self {
        let det = self.determinant();
        if det.abs() < f32::EPSILON {
            return Self::identity();
        }
        let inv = 1.0 / det;
        let m = self.m;
        // A_lin = [[m0, m3], [m1, m4]]；逆 = [[m4, -m3], [-m1, m0]]/det，逆平移 = -Ainv*t
        Self {
            m: [
                m[4] * inv,
                -m[1] * inv,
                (m[3] * m[5] - m[4] * m[2]) * inv,
                -m[3] * inv,
                m[0] * inv,
                (m[1] * m[2] - m[0] * m[5]) * inv,
            ],
        }
    }

    pub fn to_scale_rot_trans(self) -> (Vec2, Rot2, Vec2) {
        let sx = (self.m[0] * self.m[0] + self.m[1] * self.m[1]).sqrt().max(f32::EPSILON);
        let sy = self.determinant() / sx;
        let rot = Rot2 { cos: self.m[0] / sx, sin: self.m[1] / sx };
        (Vec2::new(sx, sy), rot, Vec2::new(self.m[2], self.m[5]))
    }
}

/// 4×4 列主序矩阵（IF-023）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
const fn mat(
    c0r0: f32, c0r1: f32, c0r2: f32, c0r3: f32,
    c1r0: f32, c1r1: f32, c1r2: f32, c1r3: f32,
    c2r0: f32, c2r1: f32, c2r2: f32, c2r3: f32,
    c3r0: f32, c3r1: f32, c3r2: f32, c3r3: f32,
) -> Mat4 {
    Mat4 { m: [c0r0, c0r1, c0r2, c0r3, c1r0, c1r1, c1r2, c1r3, c2r0, c2r1, c2r2, c2r3, c3r0, c3r1, c3r2, c3r3] }
}

impl Mat4 {
    pub const fn identity() -> Self {
        mat(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0)
    }

    pub const fn from_translation(t: Vec3) -> Self {
        mat(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, t.x, t.y, t.z, 1.0)
    }

    pub const fn from_scale(s: Vec3) -> Self {
        mat(s.x, 0.0, 0.0, 0.0, 0.0, s.y, 0.0, 0.0, 0.0, 0.0, s.z, 0.0, 0.0, 0.0, 0.0, 1.0)
    }

    pub fn from_axis_angle(axis: Vec3, rad: f32) -> Self {
        let mut a = axis.normalized();
        if a.length() < f32::EPSILON {
            a = Vec3::new(0.0, 0.0, 1.0);
        }
        let (s, c) = rad.sin_cos();
        let t = 1.0 - c;
        let (x, y, z) = (a.x, a.y, a.z);
        // 列主序：col0 = (txx+c, txy-sz, txz+sy) …（行主序公式按列存放）
        mat(
            t * x * x + c,
            t * x * y - s * z,
            t * x * z + s * y,
            0.0,
            t * x * y + s * z,
            t * y * y + c,
            t * y * z - s * x,
            0.0,
            t * x * z - s * y,
            t * y * z + s * x,
            t * z * z + c,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    pub fn from_quat(q: Quat) -> Self {
        let Quat { x, y, z, w } = q.normalized();
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;
        // 列主序：col0 = (1-(yy+zz), xy-wz, xz+wy) …
        mat(
            1.0 - (yy + zz),
            xy - wz,
            xz + wy,
            0.0,
            xy + wz,
            1.0 - (xx + zz),
            yz - wx,
            0.0,
            xz - wy,
            yz + wx,
            1.0 - (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    /// 透视投影（深度 0..1，Vulkan/D3D 风格）。
    pub fn perspective(fovy_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fovy_rad / 2.0).tan();
        let nf = 1.0 / (near - far);
        mat(
            f / aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            0.0,
            0.0,
            far * nf,
            -1.0,
            0.0,
            0.0,
            far * near * nf,
            0.0,
        )
    }

    /// 正交投影（深度 0..1）。
    pub fn orthographic(l: f32, r: f32, b: f32, t: f32, near: f32, far: f32) -> Self {
        mat(
            2.0 / (r - l),
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / (t - b),
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0 / (far - near),
            0.0,
            (l + r) / (l - r),
            (b + t) / (b - t),
            -near / (far - near),
            1.0,
        )
    }

    /// 右手 look-at（相机看向 target）。
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let z = (eye - target).normalized();
        let x = up.cross(z).normalized();
        let y = z.cross(x);
        mat(
            x.x,
            y.x,
            z.x,
            0.0,
            x.y,
            y.y,
            z.y,
            0.0,
            x.z,
            y.z,
            z.z,
            0.0,
            -x.dot(eye),
            -y.dot(eye),
            -z.dot(eye),
            1.0,
        )
    }

    pub fn mul_mat4(self, o: Self) -> Self {
        let mut out = [0.0f32; 16];
        for c in 0..4usize {
            for r in 0..4usize {
                out[c * 4 + r] = self.m[r] * o.m[c * 4]
                    + self.m[4 + r] * o.m[c * 4 + 1]
                    + self.m[8 + r] * o.m[c * 4 + 2]
                    + self.m[12 + r] * o.m[c * 4 + 3];
            }
        }
        Mat4 { m: out }
    }

    pub fn transform_point3(self, p: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
            m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
            m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
        )
    }

    pub fn transform_vector4(self, v: Vec4) -> Vec4 {
        let m = &self.m;
        Vec4::new(
            m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12] * v.w,
            m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13] * v.w,
            m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14] * v.w,
            m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15] * v.w,
        )
    }

    pub fn transpose(self) -> Self {
        let m = self.m;
        mat(
            m[0], m[4], m[8], m[12], m[1], m[5], m[9], m[13], m[2], m[6], m[10], m[14], m[3], m[7],
            m[11], m[15],
        )
    }

    /// 通用 4×4 逆（余子式展开）；奇异返回 identity。
    pub fn inverse(self) -> Self {
        let m = self.m;
        let c00 = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        let c01 = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        let c02 = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        let c03 = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        let det = m[0] * c00 + m[1] * c01 + m[2] * c02 + m[3] * c03;
        if det.abs() < f32::EPSILON {
            return Self::identity();
        }
        let inv = 1.0 / det;
        mat(
            c00 * inv,
            (-m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
                - m[9] * m[3] * m[14]
                - m[13] * m[2] * m[11]
                + m[13] * m[3] * m[10])
                * inv,
            (m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
                + m[5] * m[3] * m[14]
                + m[13] * m[2] * m[7]
                - m[13] * m[3] * m[6])
                * inv,
            (-m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
                - m[5] * m[3] * m[10]
                - m[9] * m[2] * m[7]
                + m[9] * m[3] * m[6])
                * inv,
            c01 * inv,
            (m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
                + m[8] * m[3] * m[14]
                + m[12] * m[2] * m[11]
                - m[12] * m[3] * m[10])
                * inv,
            (-m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
                - m[4] * m[3] * m[14]
                - m[12] * m[2] * m[7]
                + m[12] * m[3] * m[6])
                * inv,
            (m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
                + m[4] * m[3] * m[10]
                + m[8] * m[2] * m[7]
                - m[8] * m[3] * m[6])
                * inv,
            c02 * inv,
            (-m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
                - m[8] * m[3] * m[13]
                - m[12] * m[1] * m[11]
                + m[12] * m[3] * m[9])
                * inv,
            (m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
                + m[4] * m[3] * m[13]
                + m[12] * m[1] * m[7]
                - m[12] * m[3] * m[5])
                * inv,
            (-m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
                - m[4] * m[3] * m[9]
                - m[8] * m[1] * m[7]
                + m[8] * m[3] * m[5])
                * inv,
            c03 * inv,
            (m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
                + m[8] * m[2] * m[13]
                + m[12] * m[1] * m[10]
                - m[12] * m[2] * m[9])
                * inv,
            (-m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
                - m[4] * m[2] * m[13]
                - m[12] * m[1] * m[6]
                + m[12] * m[2] * m[5])
                * inv,
            (m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
                + m[4] * m[2] * m[9]
                + m[8] * m[1] * m[6]
                - m[8] * m[2] * m[5])
                * inv,
        )
    }

    pub fn translation(self) -> Vec3 {
        Vec3::new(self.m[12], self.m[13], self.m[14])
    }

    /// 右乘对角缩放（M · S，列 c 乘 s_c；用于 T·R·S 复合）。
    pub fn apply_scale(self, s: Vec3) -> Self {
        let mut m = self.m;
        let sc = [s.x, s.y, s.z, 1.0];
        for (c, factor) in sc.iter().enumerate() {
            m[c * 4] *= factor;
            m[c * 4 + 1] *= factor;
            m[c * 4 + 2] *= factor;
        }
        Mat4 { m }
    }
}

/// 四元数（IF-024），(x,y,z,w) 布局。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}

impl Quat {
    pub const fn identity() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }

    pub fn from_axis_angle(axis: Vec3, rad: f32) -> Self {
        let a = axis.normalized();
        let (s, c) = (rad / 2.0).sin_cos();
        Self { x: a.x * s, y: a.y * s, z: a.z * s, w: c }
    }

    pub fn from_rotation_x(rad: f32) -> Self {
        Self::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), rad)
    }
    pub fn from_rotation_y(rad: f32) -> Self {
        Self::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), rad)
    }
    pub fn from_rotation_z(rad: f32) -> Self {
        Self::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), rad)
    }

    pub fn mul_quat(self, o: Self) -> Self {
        Self {
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
        }
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    pub fn normalize(&mut self) {
        let l = self.length_sq().sqrt();
        if l > f32::EPSILON {
            let inv = 1.0 / l;
            self.x *= inv;
            self.y *= inv;
            self.z *= inv;
            self.w *= inv;
        }
    }

    pub fn normalized(self) -> Self {
        let mut q = self;
        q.normalize();
        q
    }

    pub fn conjugate(self) -> Self {
        Self { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }

    /// 球面插值（短弧处理）。
    pub fn slerp(self, o: Self, t: f32) -> Self {
        let mut cos = self.dot(o);
        let mut end = o;
        if cos < 0.0 {
            end = Quat { x: -o.x, y: -o.y, z: -o.z, w: -o.w };
            cos = -cos;
        }
        if cos > 0.9995 {
            return self.nlerp(end, t);
        }
        let theta = cos.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        if sin_theta.abs() < f32::EPSILON {
            return self.nlerp(end, t);
        }
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;
        Self {
            x: self.x * a + end.x * b,
            y: self.y * a + end.y * b,
            z: self.z * a + end.z * b,
            w: self.w * a + end.w * b,
        }
        .normalized()
    }

    pub fn nlerp(self, o: Self, t: f32) -> Self {
        Self {
            x: self.x + (o.x - self.x) * t,
            y: self.y + (o.y - self.y) * t,
            z: self.z + (o.z - self.z) * t,
            w: self.w + (o.w - self.w) * t,
        }
        .normalized()
    }

    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        Mat4::from_quat(self).transform_point3(v)
    }

    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_quat(self)
    }
}

// ---- 自由函数（IF-031） ----

pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp01((x - edge0) / (edge1 - edge0).max(f32::EPSILON));
    t * t * (3.0 - 2.0 * t)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// 最短弧角度插值。
pub fn angle_lerp(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    a + d * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32, eps: f32) {
        assert!((a - b).abs() <= eps, "{a} vs {b}");
    }

    #[test]
    fn vec_basics() {
        let a = Vec2::new(3.0, 4.0);
        assert_close(a.length(), 5.0, 1e-6);
        assert_close(Vec2::new(1.0, 0.0).angle(), 0.0, 1e-6);
        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
        assert_close(Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0)).z, 1.0, 1e-6);
    }

    #[test]
    fn rot2_roundtrip() {
        let r = Rot2::from_angle(0.7);
        let v = Vec2::new(1.0, 2.0);
        let w = v.rotate(r).rotate(r.inverse());
        assert_close(w.x, v.x, 1e-5);
        assert_close(w.y, v.y, 1e-5);
    }

    #[test]
    fn mat2x3_inverse() {
        let m = Mat2x3::compose(Vec2::new(2.0, 3.0), Rot2::from_angle(0.3), Vec2::new(5.0, -1.0));
        let p = Vec2::new(0.5, -0.25);
        let w = m.inverse().transform_point(m.transform_point(p));
        assert_close(w.x, p.x, 1e-4);
        assert_close(w.y, p.y, 1e-4);
    }

    #[test]
    fn mat4_inverse_and_mul() {
        let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            .mul_mat4(Mat4::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5));
        let p = Vec3::new(0.3, -0.7, 0.1);
        let w = m.inverse().transform_point3(m.transform_point3(p));
        assert_close(w.x, p.x, 1e-4);
        assert_close(w.y, p.y, 1e-4);
        assert_close(w.z, p.z, 1e-4);
    }

    #[test]
    fn perspective_depth_range() {
        let proj = Mat4::perspective(1.2, 16.0 / 9.0, 0.1, 100.0);
        let near = proj.transform_vector4(Vec4::new(0.0, 0.0, -0.1, 1.0));
        let far = proj.transform_vector4(Vec4::new(0.0, 0.0, -100.0, 1.0));
        assert!(near.w > 0.0 && far.w > 0.0);
        assert_close(near.z / near.w, 0.0, 1e-3);
        assert_close(far.z / far.w, 1.0, 1e-3);
    }

    #[test]
    fn look_at_forward() {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        let origin = view.transform_point3(Vec3::new(0.0, 0.0, 5.0));
        assert_close(origin.x, 0.0, 1e-4);
        assert_close(origin.y, 0.0, 1e-4);
        assert_close(origin.z, 0.0, 1e-4);
    }

    #[test]
    fn quat_slerp_shortest_arc() {
        // 注：f32 下 cos(π/2) 为 -4.37e-8（负），恰好跨越半球边界；
        // 用 3.0 rad 避免测试依赖 ULP 级精度。
        let a = Quat::identity();
        let b = Quat::from_rotation_y(3.0);
        let mid = a.slerp(b, 0.5);
        let half = Quat::from_rotation_y(1.5);
        assert_close(mid.dot(half).abs(), 1.0, 1e-3);
        // 反向输入（四元数取负表示同一旋转）也取短弧，结果一致
        let neg = Quat { x: -b.x, y: -b.y, z: -b.z, w: -b.w };
        assert_close(a.slerp(neg, 0.5).dot(half).abs(), 1.0, 1e-3);
    }

    #[test]
    fn quat_rotate_matches_mat() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.5).normalized(), 1.1);
        let v = Vec3::new(1.0, 2.0, 3.0);
        let a = q.rotate_vec3(v);
        let b = q.to_mat4().transform_point3(v);
        assert_close(a.x, b.x, 1e-4);
        assert_close(a.y, b.y, 1e-4);
        assert_close(a.z, b.z, 1e-4);
    }

    #[test]
    fn angle_lerp_wrap() {
        assert_close(angle_lerp(0.1, std::f32::consts::TAU - 0.1, 0.5), 0.0, 1e-4);
    }
}
