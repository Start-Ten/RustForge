//! 颜色（IF-030）：线性 f32，渲染前转 sRGB8。

/// 线性 RGBA 颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const YELLOW: Color = Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const GRAY: Color = Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// sRGB 字节 → 线性 f32（伽马 2.2 近似查表式公式）。
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: srgb_to_linear(r),
            g: srgb_to_linear(g),
            b: srgb_to_linear(b),
            a: a as f32 / 255.0,
        }
    }

    /// 线性 → sRGB 字节。
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    pub fn lerp(self, o: Color, t: f32) -> Color {
        Color {
            r: self.r + (o.r - self.r) * t,
            g: self.g + (o.g - self.g) * t,
            b: self.b + (o.b - self.b) * t,
            a: self.a + (o.a - self.a) * t,
        }
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (v * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba8_roundtrip() {
        let c = Color::from_rgba8(200, 10, 250, 128);
        let [r, g, b, a] = c.to_rgba8();
        assert!((r as i32 - 200).abs() <= 1);
        assert!((g as i32 - 10).abs() <= 1);
        assert!((b as i32 - 250).abs() <= 1);
        assert_eq!(a, 128);
    }

    #[test]
    fn lerp_and_alpha() {
        let c = Color::RED.lerp(Color::BLUE, 0.5);
        assert!((c.r - 0.5).abs() < 1e-6);
        assert_eq!(Color::RED.with_alpha(0.25).a, 0.25);
    }
}
