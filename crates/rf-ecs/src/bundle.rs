//! Bundle（IF-083）：组件打包 spawn/attach。

use crate::{Archetype, Component, World};
use rf_core::{Entity, Tick};
use std::any::TypeId;

/// 组件集合打包。attach 将各组件依序 insert 到实体（触发 archetype 迁移）。
///
/// # Safety
/// attach 实现必须仅使用 `World::insert` 等公开安全 API 写入自身组件。
pub trait Bundle {
    fn signatures(out: &mut Vec<TypeId>);
    /// SAFETY: entity 必须存活且调用方保证无并发访问。
    fn attach(self, world: &mut World, entity: Entity, tick: Tick);
}

impl Bundle for () {
    fn signatures(_out: &mut Vec<TypeId>) {}
    fn attach(self, _world: &mut World, _entity: Entity, _tick: Tick) {}
}

macro_rules! tuple_bundle {
    ($(($gen:ident, $val:ident)),+) => {
        impl<$($gen: Component),+> Bundle for ($($gen,)+) {
            fn signatures(out: &mut Vec<TypeId>) {
                $( out.push(TypeId::of::<$gen>()); )+
            }
            fn attach(self, world: &mut World, entity: Entity, _tick: Tick) {
                let ($($val,)+) = self;
                $( let _ = world.insert::<$gen>(entity, $val); )+
            }
        }
    };
}

tuple_bundle!((A, a));
tuple_bundle!((A, a), (B, b));
tuple_bundle!((A, a), (B, b), (C, c));
tuple_bundle!((A, a), (B, b), (C, c), (D, d));
tuple_bundle!((A, a), (B, b), (C, c), (D, d), (E, e));
tuple_bundle!((A, a), (B, b), (C, c), (D, d), (E, e), (F, f));

/// 空 archetype 访问（调试/统计用）。
pub fn empty_archetype_len(arch: &Archetype) -> usize {
    arch.len()
}
