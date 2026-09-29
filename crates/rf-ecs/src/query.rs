//! 类型化查询（IF-074, IF-078 ~ IF-082）。

use crate::column::{Archetype, Component};
use crate::World;
use rf_core::{Entity, Tick};
use std::any::TypeId;
use std::collections::BTreeSet;
use std::marker::PhantomData;

/// 读写访问集（IF-082）：调度器冲突分析。
#[derive(Debug, Clone, Default)]
pub struct AccessSet {
    reads: BTreeSet<TypeId>,
    writes: BTreeSet<TypeId>,
}

impl AccessSet {
    pub fn read(&mut self, id: TypeId) -> &mut Self {
        self.reads.insert(id);
        self
    }

    pub fn write(&mut self, id: TypeId) -> &mut Self {
        self.writes.insert(id);
        self
    }

    pub fn reads(&self) -> &BTreeSet<TypeId> {
        &self.reads
    }

    pub fn writes(&self) -> &BTreeSet<TypeId> {
        &self.writes
    }

    /// 冲突 = 任一写重叠（含相互写、读写互冲）。
    pub fn conflicts(&self, other: &AccessSet) -> bool {
        !self.writes.is_disjoint(&other.reads)
            || !self.reads.is_disjoint(&other.writes)
            || !self.writes.is_disjoint(&other.writes)
    }
}

/// 查询数据（IF-078）。
///
/// # Safety
/// fetch 返回的引用生命周期必须不超过 archetype 借用；
/// &mut T 的可变性由调度器读写集分析保证（同一时刻仅一个可变访问者）。
pub unsafe trait QueryData {
    type Item<'a>;
    fn access(acc: &mut AccessSet);
    /// 迭代前过滤：archetype 必须包含的全部组件类型。
    fn required(out: &mut Vec<TypeId>);
    fn register(world: &mut World);
    /// # Safety
    /// row < arch.len()；类型与列匹配；`&mut T` 的排他性由调度器保证。
    unsafe fn fetch<'a>(arch: &'a Archetype, row: usize) -> Self::Item<'a>;
}

// SAFETY: &T 只读；fetch 返回的引用生命周期绑定 archetype。
unsafe impl<T: Component> QueryData for &T {
    type Item<'a> = &'a T;
    fn access(acc: &mut AccessSet) {
        acc.read(TypeId::of::<T>());
    }
    fn required(out: &mut Vec<TypeId>) {
        out.push(TypeId::of::<T>());
    }
    fn register(world: &mut World) {
        world.register_component::<T>();
    }
    unsafe fn fetch(arch: &Archetype, row: usize) -> &T {
        arch.get::<T>(row).expect("query component present in matched archetype")
    }
}

// SAFETY: 可变访问的排他性由调度器保证。
unsafe impl<T: Component> QueryData for &mut T {
    type Item<'a> = &'a mut T;
    fn access(acc: &mut AccessSet) {
        acc.write(TypeId::of::<T>());
    }
    fn required(out: &mut Vec<TypeId>) {
        out.push(TypeId::of::<T>());
    }
    fn register(world: &mut World) {
        world.register_component::<T>();
    }
    unsafe fn fetch(arch: &Archetype, row: usize) -> &mut T {
        // SAFETY: 见 trait Safety 契约（调度器已验证排他写）。
        unsafe {
            let arch_mut = (arch as *const Archetype as *mut Archetype).as_mut().unwrap();
            arch_mut.get_mut::<T>(row).expect("query component present in matched archetype")
        }
    }
}

// SAFETY: 可选只读。
unsafe impl<T: Component> QueryData for Option<&T> {
    type Item<'a> = Option<&'a T>;
    fn access(acc: &mut AccessSet) {
        acc.read(TypeId::of::<T>());
    }
    fn required(_out: &mut Vec<TypeId>) {
        // 可选组件不构成过滤条件
    }
    fn register(world: &mut World) {
        world.register_component::<T>();
    }
    unsafe fn fetch(arch: &Archetype, row: usize) -> Option<&T> {
        arch.get::<T>(row)
    }
}

// SAFETY: 实体 ID 为 Copy 值。
unsafe impl QueryData for Entity {
    type Item<'a> = Entity;
    fn access(_acc: &mut AccessSet) {}
    fn required(_out: &mut Vec<TypeId>) {}
    fn register(_world: &mut World) {}
    unsafe fn fetch(arch: &Archetype, row: usize) -> Entity {
        arch.entities[row]
    }
}

macro_rules! tuple_query {
    ($(($name:ident, $idx:tt)),+) => {
        // SAFETY: 元组各成员独立安全 → 组合安全。
        unsafe impl<$($name: QueryData),+> QueryData for ($($name,)+) {
            type Item<'a> = ($($name::Item<'a>,)+);
            fn access(acc: &mut AccessSet) {
                $($name::access(acc);)+
            }
            fn required(out: &mut Vec<TypeId>) {
                $($name::required(out);)+
            }
            fn register(world: &mut World) {
                $($name::register(world);)+
            }
            unsafe fn fetch<'a>(arch: &'a Archetype, row: usize) -> Self::Item<'a> {
                ($($name::fetch(arch, row),)+)
            }
        }
    };
}

tuple_query!((A, 0));
tuple_query!((A, 0), (B, 1));
tuple_query!((A, 0), (B, 1), (C, 2));
tuple_query!((A, 0), (B, 1), (C, 2), (D, 3));
tuple_query!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
tuple_query!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));

/// Archetype 只读视图（过滤器输入，IF-080）。
pub struct ArchetypeInfo<'a> {
    pub archetype: &'a Archetype,
    pub since: Tick,
    pub world_tick: Tick,
}

impl<'a> ArchetypeInfo<'a> {
    /// 返回 (added_ticks, changed_ticks)（列存在时）。
    pub fn column_ticks(&self, id: TypeId) -> Option<(&[Tick], &[Tick])> {
        let ci = self.archetype.column_index(id)?;
        let col = &self.archetype.columns[ci];
        Some((&col.added_ticks, &col.changed_ticks))
    }

    pub fn has(&self, id: TypeId) -> bool {
        self.archetype.column_index(id).is_some()
    }
}

/// 查询过滤器（IF-080）。
pub trait QueryFilter {
    fn matches(info: &ArchetypeInfo, row: usize) -> bool;
}

impl QueryFilter for () {
    fn matches(_info: &ArchetypeInfo, _row: usize) -> bool {
        true
    }
}

/// 有 T 组件。
pub struct With<T: Component>(PhantomData<fn() -> T>);

impl<T: Component> QueryFilter for With<T> {
    fn matches(info: &ArchetypeInfo, _row: usize) -> bool {
        info.has(TypeId::of::<T>())
    }
}

/// 无 T 组件。
pub struct Without<T: Component>(PhantomData<fn() -> T>);

impl<T: Component> QueryFilter for Without<T> {
    fn matches(info: &ArchetypeInfo, _row: usize) -> bool {
        !info.has(TypeId::of::<T>())
    }
}

/// added_tick 在 (since, now] 窗口内。
pub struct Added<T: Component>(PhantomData<fn() -> T>);

impl<T: Component> QueryFilter for Added<T> {
    fn matches(info: &ArchetypeInfo, row: usize) -> bool {
        match info.column_ticks(TypeId::of::<T>()) {
            Some((added, _)) => added[row].wrapping_gt(&info.since),
            None => false,
        }
    }
}

/// changed_tick ≥ since（本查询窗口内变更）。
pub struct Changed<T: Component>(PhantomData<fn() -> T>);

impl<T: Component> QueryFilter for Changed<T> {
    fn matches(info: &ArchetypeInfo, row: usize) -> bool {
        match info.column_ticks(TypeId::of::<T>()) {
            Some((_, changed)) => changed[row].wrapping_gt(&info.since),
            None => false,
        }
    }
}

/// 查询（IF-081）。
pub struct Query<'w, D: QueryData, F: QueryFilter = ()> {
    archetypes: &'w [Archetype],
    required: Vec<TypeId>,
    since: Tick,
    world_tick: Tick,
    marker: PhantomData<fn() -> (D, F)>,
}

impl<'w, D: QueryData, F: QueryFilter> Query<'w, D, F> {
    pub(crate) fn new(world: &'w World) -> Self {
        let mut required = Vec::new();
        D::required(&mut required);
        Self {
            archetypes: &world.archetypes,
            required,
            since: Tick(0),
            world_tick: world.tick,
            marker: PhantomData,
        }
    }

    /// 变更检测窗口：仅遍历 since 之后变更的实体（配合 Changed/Added）。
    pub fn changed_since(mut self, since: Tick) -> Self {
        self.since = since;
        self
    }

    pub fn iter(&self) -> QueryIter<'w, D, F> {
        QueryIter {
            archetypes: self.archetypes,
            required: self.required.clone(),
            arch_idx: 0,
            row: 0,
            since: self.since,
            world_tick: self.world_tick,
            marker: PhantomData,
        }
    }

    /// 单结果：恰好一个返回 Some，多个返回 None。
    pub fn single(&self) -> Option<D::Item<'w>> {
        let mut iter = self.iter();
        let first = iter.next()?;
        if iter.next().is_some() {
            None
        } else {
            Some(first)
        }
    }
}

/// 查询迭代器（IF-081）。
pub struct QueryIter<'w, D: QueryData, F: QueryFilter> {
    archetypes: &'w [Archetype],
    required: Vec<TypeId>,
    arch_idx: usize,
    row: usize,
    since: Tick,
    world_tick: Tick,
    marker: PhantomData<fn() -> (D, F)>,
}

impl<'w, D: QueryData, F: QueryFilter> Iterator for QueryIter<'w, D, F> {
    type Item = D::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let arch = self.archetypes.get(self.arch_idx)?;
            if self.row >= arch.len() {
                self.arch_idx += 1;
                self.row = 0;
                continue;
            }
            // 必需组件过滤：archetype 缺任一必需类型则整块跳过
            if self.required.iter().any(|id| arch.column_index(*id).is_none()) {
                self.arch_idx += 1;
                self.row = 0;
                continue;
            }
            let info =
                ArchetypeInfo { archetype: arch, since: self.since, world_tick: self.world_tick };
            if F::matches(&info, self.row) {
                let row = self.row;
                self.row += 1;
                // SAFETY: row < len；类型由 D 与 archetype 匹配（QueryData access 与迭代器构造一致）。
                return Some(unsafe { D::fetch(arch, row) });
            }
            self.row += 1;
        }
    }
}
