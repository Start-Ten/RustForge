//! 变换（IF-025）：3D TRS 与 2D TRS。

use crate::{Mat2x3, Mat4, Quat, Rot2, Vec2, Vec3};

/// 3D 平移/旋转/缩放。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    pub const fn identity() -> Self {
        Self { position: Vec3::ZERO, rotation: Quat::identity(), scale: Vec3::ONE }
    }

    /// T * R * S。
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            .mul_mat4(Mat4::from_quat(self.rotation))
            .apply_scale(self.scale)
    }

    pub fn lerp(&self, o: &Transform, t: f32) -> Transform {
        Transform {
            position: self.position.lerp(o.position, t),
            rotation: self.rotation.nlerp(o.rotation, t),
            scale: self.scale.lerp(o.scale, t),
        }
    }
}

/// 2D 平移/旋转/缩放。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: Rot2,
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform2D {
    pub const fn identity() -> Self {
        Self { position: Vec2::ZERO, rotation: Rot2::identity(), scale: Vec2::ONE }
    }

    pub fn to_mat2x3(&self) -> Mat2x3 {
        Mat2x3::compose(self.scale, self.rotation, self.position)
    }

    pub fn lerp(&self, o: &Transform2D, t: f32) -> Transform2D {
        // 角度走最短弧
        let a = self.rotation.angle();
        let b = o.rotation.angle();
        let ang = crate::angle_lerp(a, b, t);
        Transform2D {
            position: self.position.lerp(o.position, t),
            rotation: Rot2::from_angle(ang),
            scale: self.scale.lerp(o.scale, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform3_roundtrip() {
        let t = Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(0.7),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let m = t.to_mat4();
        let p = m.transform_point3(Vec3::ZERO);
        assert!((p.x - 1.0).abs() < 1e-4);
        assert!((p.y - 2.0).abs() < 1e-4);
        assert!((p.z - 3.0).abs() < 1e-4);
        let scaled = m.transform_vector4(crate::Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert!((scaled.length() - 2.0).abs() < 1e-4); // |R·S·x| = 2
    }

    #[test]
    fn transform2_compose_matches_mat() {
        let t = Transform2D {
            position: Vec2::new(3.0, -2.0),
            rotation: Rot2::from_angle(0.4),
            scale: Vec2::new(2.0, 1.5),
        };
        let m = t.to_mat2x3();
        let p = m.transform_point(Vec2::ZERO);
        assert!((p.x - 3.0).abs() < 1e-4);
        assert!((p.y + 2.0).abs() < 1e-4);
    }
}
