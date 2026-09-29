//! 关键帧曲线（IF-032）。

/// 插值模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurveMode {
    #[default]
    Linear,
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveKey {
    pub t: f32,
    pub v: f32,
}

/// 单通道关键帧曲线（时间升序）。
#[derive(Debug, Clone, Default)]
pub struct Curve1 {
    pub keys: Vec<CurveKey>,
    pub mode: CurveMode,
}

impl Curve1 {
    pub fn new(mode: CurveMode) -> Self {
        Self { keys: Vec::new(), mode }
    }

    /// 插入并保持时间升序（重复时间点替换）。
    pub fn add(&mut self, t: f32, v: f32) {
        match self.keys.binary_search_by(|k| k.t.partial_cmp(&t).unwrap()) {
            Ok(i) => self.keys[i].v = v,
            Err(i) => self.keys.insert(i, CurveKey { t, v }),
        }
    }

    /// 首尾钳制采样；无键返回 0。
    pub fn sample(&self, t: f32) -> f32 {
        if self.keys.is_empty() {
            return 0.0;
        }
        if t <= self.keys[0].t {
            return self.keys[0].v;
        }
        if t >= self.keys[self.keys.len() - 1].t {
            return self.keys[self.keys.len() - 1].v;
        }
        let i = self.keys.partition_point(|k| k.t < t).max(1);
        let a = self.keys[i - 1];
        let b = self.keys[i];
        let span = (b.t - a.t).max(f32::EPSILON);
        let u = (t - a.t) / span;
        match self.mode {
            CurveMode::Linear => a.v + (b.v - a.v) * u,
            CurveMode::Smooth => {
                let s = u * u * (3.0 - 2.0 * u);
                a.v + (b.v - a.v) * s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_sampling() {
        let mut c = Curve1::new(CurveMode::Linear);
        c.add(0.0, 0.0);
        c.add(1.0, 10.0);
        assert_eq!(c.sample(-1.0), 0.0);
        assert_eq!(c.sample(2.0), 10.0);
        assert_eq!(c.sample(0.5), 5.0);
        c.add(0.5, 20.0); // 替换重复时间点
        assert_eq!(c.sample(0.5), 20.0);
        assert_eq!(Curve1::default().sample(0.3), 0.0);
    }

    #[test]
    fn smooth_mode() {
        let mut c = Curve1::new(CurveMode::Smooth);
        c.add(0.0, 0.0);
        c.add(1.0, 1.0);
        let m = c.sample(0.5);
        assert!((m - 0.5).abs() < 1e-6); // smoothstep 中点对称
        assert!(c.sample(0.25) < 0.25); // 前段慢
    }
}
