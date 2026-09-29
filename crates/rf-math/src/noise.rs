//! 确定性噪声（IF-034）：Value/Perlin/FBM，带种子。

use rf_core::Pcg32;

fn hash2(seed: u64, x: i32, y: i32) -> f32 {
    // 整数格点 → [0,1)
    let mut h = seed.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= (x as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= (y as u64).wrapping_mul(0x94D049BB133111EB);
    // fmix32
    let mut z = (h ^ (h >> 33)) as u32;
    z = z.wrapping_mul(0x85EBCA6B);
    z ^= z >> 13;
    z = z.wrapping_mul(0xC2B2AE35);
    z ^= z >> 16;
    (z >> 8) as f32 / (1u32 << 24) as f32
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 值噪声 [0,1]。
pub fn value_noise_2d(seed: u64, x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (fade(x - xi as f32), fade(y - yi as f32));
    let v00 = hash2(seed, xi, yi);
    let v10 = hash2(seed, xi + 1, yi);
    let v01 = hash2(seed, xi, yi + 1);
    let v11 = hash2(seed, xi + 1, yi + 1);
    lerp(lerp(v00, v10, tx), lerp(v01, v11, tx), ty)
}

fn grad(seed: u64, x: i32, y: i32, dx: f32, dy: f32) -> f32 {
    // 8 方向梯度
    let a = hash2(seed, x, y) * std::f32::consts::TAU;
    let (gx, gy) = a.sin_cos();
    dx * gy + dy * gx
}

/// Perlin 噪声 [-1,1]。
pub fn perlin_2d(seed: u64, x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let n00 = grad(seed, xi, yi, fx, fy);
    let n10 = grad(seed, xi + 1, yi, fx - 1.0, fy);
    let n01 = grad(seed, xi, yi + 1, fx, fy - 1.0);
    let n11 = grad(seed, xi + 1, yi + 1, fx - 1.0, fy - 1.0);
    let (tx, ty) = (fade(fx), fade(fy));
    lerp(lerp(n00, n10, tx), lerp(n01, n11, tx), ty)
}

/// 分形布朗运动（octaves 层叠加权），输出约 [-1,1]。
pub fn fbm_2d(seed: u64, x: f32, y: f32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    let mut s = seed;
    for _ in 0..octaves {
        sum += perlin_2d(s, x * freq, y * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
        s = s.wrapping_add(1);
    }
    sum / norm.max(f32::EPSILON)
}

/// 便捷：Pcg32 作为噪声种子来源。
pub fn seed_from(mut pc: Pcg32) -> u64 {
    (pc.next_u32() as u64) << 32 | pc.next_u32() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_bounded() {
        for i in 0..50 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.11;
            assert_eq!(value_noise_2d(7, x, y), value_noise_2d(7, x, y));
            assert!((0.0..=1.0).contains(&value_noise_2d(7, x, y)));
            assert!((-1.0..=1.0).contains(&perlin_2d(7, x, y)));
            assert!((-1.0..=1.0).contains(&fbm_2d(7, x, y, 4)));
        }
    }

    #[test]
    fn perlin_zero_at_lattice() {
        // 格点上梯度点积为 0
        assert!(perlin_2d(3, 2.0, 3.0).abs() < 1e-4);
    }
}
