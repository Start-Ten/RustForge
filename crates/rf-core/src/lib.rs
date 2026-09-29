//! RustForge 内核：实体 ID、tick、句柄、错误、版本、CPU 图像、哈希、随机数。
//! 零外部运行时依赖（IF-001 ~ IF-011）。

pub mod error;
pub mod image;
pub mod random;

pub use error::{EngineError, Result};
pub use image::Rgba8Image;
pub use random::Pcg32;

use std::fmt;

/// 引擎名称（IF-001）。
pub const ENGINE_NAME: &str = "RustForge";

/// 引擎版本（IF-002）。
pub const ENGINE_VERSION: Version = Version::new(0, 1, 0);

/// 语义化版本（IF-003）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 实体 ID：index + generation 打包为 64 位（IF-004）。
/// generation 用于代数回绕检测：槽位复用后旧句柄失效。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl Entity {
    /// 占位实体（generation 为 0 视为无效代数）。
    pub const PLACEHOLDER: Entity = Entity { index: u32::MAX, generation: 0 };

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn to_bits(self) -> u64 {
        ((self.generation as u64) << 32) | self.index as u64
    }

    pub const fn from_bits(bits: u64) -> Option<Entity> {
        let generation = (bits >> 32) as u32;
        let index = bits as u32;
        if generation == 0 {
            None
        } else {
            Some(Entity { index, generation })
        }
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}v{}", self.index, self.generation)
    }
}

/// 单调递增 tick，u32 回绕安全（IF-005）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Tick(pub u32);

impl Tick {
    pub const ZERO: Tick = Tick(0);

    /// 回绕安全自增。
    pub const fn next(self) -> Tick {
        Tick(self.0.wrapping_add(1))
    }

    /// 回绕比较：tick 差在 ±2^31 内时给出真实先后顺序（差按 i32 解释）。
    pub fn wrapping_cmp(&self, other: &Tick) -> std::cmp::Ordering {
        (self.0.wrapping_sub(other.0) as i32).cmp(&0)
    }

    pub fn wrapping_lt(&self, other: &Tick) -> bool {
        matches!(self.wrapping_cmp(other), std::cmp::Ordering::Less)
    }

    pub fn wrapping_gt(&self, other: &Tick) -> bool {
        matches!(self.wrapping_cmp(other), std::cmp::Ordering::Greater)
    }
}

/// 泛型槽位句柄（IF-006）：generation 与槽位当前代数比对判断存活。
#[derive(Debug, Clone, Copy)]
pub struct Handle<T> {
    pub index: u32,
    pub generation: u32,
    marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation, marker: std::marker::PhantomData }
    }

    /// 槽位当前代数与本句柄是否一致（存活）。
    pub const fn matches(&self, generation: u32) -> bool {
        self.generation == generation
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for Handle<T> {}
impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

/// FNV-1a 64 位哈希（IF-010）。
pub const fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// FNV-1a 128 位哈希（双素数链拼接，IF-010）。
pub fn fnv1a128(data: &[u8]) -> u128 {
    let mut h1: u64 = 0x6c62_272e_07bb_0142;
    let mut h2: u64 = 0x84bc_ed6d_2b39_f01b;
    for &b in data {
        h1 ^= b as u64;
        h1 = h1.wrapping_mul(0x0000_0100_0000_01b3);
        h2 ^= (b as u64).rotate_left(3);
        h2 = h2.wrapping_mul(0x0000_0181_0000_0193);
    }
    ((h1 as u128) << 64) | h2 as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_bits_roundtrip() {
        let e = Entity::new(7, 3);
        assert_eq!(Entity::from_bits(e.to_bits()), Some(e));
        assert_eq!(Entity::from_bits(1234), None); // generation=0 无效
    }

    #[test]
    fn entity_placeholder_invalid() {
        assert_ne!(Entity::PLACEHOLDER, Entity::new(1, 1));
    }

    #[test]
    fn tick_wraparound() {
        let a = Tick(u32::MAX - 1);
        let b = a.next().next(); // 回绕到 0
        assert_eq!(b, Tick(0));
        assert!(a.wrapping_lt(&b));
        assert!(b.wrapping_gt(&a));
    }

    #[test]
    fn fnv_known_values() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_ne!(fnv1a128(b"hello"), fnv1a128(b"hellp"));
    }

    #[test]
    fn handle_matches_generation() {
        let h = Handle::<u8>::new(0, 1);
        assert!(h.matches(1));
        assert!(!h.matches(2));
    }

    #[test]
    fn pcg_deterministic_and_range() {
        let mut a = Pcg32::new(42);
        let mut b = Pcg32::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        let mut r = Pcg32::new(7);
        for _ in 0..1000 {
            let v = r.range(-5, 5);
            assert!((-5..5).contains(&v));
        }
        assert!(Pcg32::new(1).next_f32() < 1.0);
    }

    #[test]
    fn version_ord() {
        assert!(Version::new(0, 2, 0) > Version::new(0, 1, 9));
    }
}
