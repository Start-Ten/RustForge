//! PCG32 确定性随机数（IF-011）：可种子、可复现，供物理/噪声/测试共用。

/// PCG-XSH-RR 64/32（O'Neill 2014）。周期 2^64，统计质量足够游戏用途。
#[derive(Debug, Clone)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut r = Pcg32 { state: 0, inc: (seed << 1) | 1 };
        let s = r.next_u32() as u64;
        r.state = r.state.wrapping_add(s);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// [0,1) 均匀浮点。
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// [lo, hi) 整数。
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(hi > lo);
        let span = (hi - lo) as u32;
        lo + (self.next_u32() % span) as i32
    }

    pub fn pick<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.range(0, slice.len() as i32) as usize])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Pcg32::new(99);
        let mut b = Pcg32::new(99);
        for _ in 0..64 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Pcg32::new(1);
        let mut b = Pcg32::new(2);
        assert_ne!(a.next_u32(), b.next_u32());
    }
}
