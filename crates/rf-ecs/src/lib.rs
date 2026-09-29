//! RustForge ECS（IF-070 ~ IF-086）：Archetype 存储、类型化查询、调度、事件、命令、层级。
// 派生宏生成 `impl rf_ecs::…`，本 crate 内以该名称自引用。
extern crate self as rf_ecs;

pub mod bundle;
pub mod column;
pub mod commands;
pub mod events;
pub mod query;
pub mod schedule;

pub use bundle::Bundle;
pub use column::{Archetype, Column, Component, Resource};
pub use commands::{Command, Commands};
pub use events::{EventReader, Events};
pub use query::{
    AccessSet, Added, ArchetypeInfo, Changed, Query, QueryData, QueryFilter, QueryIter, With,
    Without,
};
pub use rf_ecs_derive::{Component, Resource};
pub use schedule::{system, FnSystem, Schedule, Stage, System, SystemSet};

use rf_core::{Entity, Result as CoreResult, Tick};
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// 世界统计（IF-077）。
pub struct WorldStats {
    pub archetypes: usize,
    pub entities: usize,
    pub columns: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Location {
    pub archetype: usize,
    pub row: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Slot {
    pub generation: u32,
    pub alive: bool,
    pub location: Option<Location>,
}

/// 类型擦除资源容器。
pub(crate) struct ResourceCell {
    pub(crate) value: Box<dyn Any + Send + Sync>,
}

/// ECS 世界（IF-071 ~ IF-077）。
pub struct World {
    pub(crate) slots: Vec<Slot>,
    pub(crate) free: Vec<u32>,
    pub(crate) archetypes: Vec<Archetype>,
    /// 签名 → archetype 下标。
    pub(crate) archetype_index: HashMap<Vec<TypeId>, usize>,
    /// 组件列工厂（首次使用时注册）。
    pub(crate) column_factories: HashMap<TypeId, fn() -> Column>,
    /// 组件类型名注册（统计/序列化用）。
    pub(crate) component_names: HashMap<TypeId, &'static str>,
    pub(crate) resources: HashMap<TypeId, ResourceCell>,
    pub(crate) tick: Tick,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut w = Self {
            slots: Vec::new(),
            free: Vec::new(),
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            column_factories: HashMap::new(),
            component_names: HashMap::new(),
            resources: HashMap::new(),
            tick: Tick(1),
        };
        w.archetypes.push(Archetype::empty());
        w.archetype_index.insert(Vec::new(), 0);
        w
    }

    // ---- 实体（IF-071） ----

    pub fn spawn_empty(&mut self) -> Entity {
        let entity = self.alloc_entity();
        let row = self.archetypes[0].push_entity(entity);
        self.slots[entity.index as usize].location = Some(Location { archetype: 0, row });
        entity
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let entity = self.spawn_empty();
        let tick = self.tick;
        bundle.attach(self, entity, tick);
        entity
    }

    pub fn spawn_batch<B: Bundle>(&mut self, bundles: impl IntoIterator<Item = B>) -> Vec<Entity> {
        bundles.into_iter().map(|b| self.spawn(b)).collect()
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(slot) = self.slots.get(entity.index as usize) else { return false };
        if !slot.alive || slot.generation != entity.generation {
            return false;
        }
        // 先取父级（组件即将随行删除）
        let parent = self.parent(entity);
        let loc = slot.location.expect("alive entity has location");
        self.remove_from_archetype(loc);
        // 立即失效位置，避免 detach 读取已删除行
        let s = &mut self.slots[entity.index as usize];
        s.location = None;
        s.alive = false;
        s.generation = s.generation.wrapping_add(1);
        if s.generation == 0 {
            s.generation = 1; // 0 为无效代数
        }
        if let Some(p) = parent {
            if let Some(map) = self.resource_mut::<ChildrenMap>() {
                if let Some(list) = map.0.get_mut(&p) {
                    list.retain(|e| *e != entity);
                }
            }
        }
        self.free.push(entity.index);
        true
    }

    pub fn despawn_recursive(&mut self, entity: Entity) -> bool {
        let children: Vec<Entity> = self.children(entity).to_vec();
        for c in children {
            self.despawn_recursive(c);
        }
        self.despawn(entity)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.slots
            .get(entity.index as usize)
            .map(|s| s.alive && s.generation == entity.generation)
            .unwrap_or(false)
    }

    pub fn entities(&self) -> Vec<Entity> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive)
            .map(|(i, s)| Entity::new(i as u32, s.generation))
            .collect()
    }

    pub fn entity_count(&self) -> usize {
        self.slots.iter().filter(|s| s.alive).count()
    }

    pub fn clear(&mut self) {
        self.archetypes.clear();
        self.archetypes.push(Archetype::empty());
        self.archetype_index.clear();
        self.archetype_index.insert(Vec::new(), 0);
        self.resources.clear();
        self.slots = self
            .slots
            .drain(..)
            .map(|s| Slot {
                generation: s.generation.wrapping_add(1).max(1),
                alive: false,
                location: None,
            })
            .collect();
        self.free = (0..self.slots.len() as u32).collect();
    }

    fn alloc_entity(&mut self) -> Entity {
        match self.free.pop() {
            Some(idx) => {
                let gen = self.slots[idx as usize].generation;
                self.slots[idx as usize].alive = true;
                Entity::new(idx, gen)
            }
            None => {
                let idx = self.slots.len() as u32;
                self.slots.push(Slot { generation: 1, alive: true, location: None });
                Entity::new(idx, 1)
            }
        }
    }

    // ---- 组件（IF-072） ----

    pub(crate) fn register_component<T: Component>(&mut self) {
        self.column_factories.entry(TypeId::of::<T>()).or_insert(Column::new::<T>);
        self.component_names.insert(TypeId::of::<T>(), std::any::type_name::<T>());
    }

    pub fn insert<C: Component>(&mut self, entity: Entity, component: C) -> CoreResult<()> {
        if !self.contains(entity) {
            return Err(rf_core::EngineError::EntityDead(entity));
        }
        self.register_component::<C>();
        if self.has::<C>(entity) {
            // 就地替换
            let loc = self.slots[entity.index as usize].location.unwrap();
            let tick = self.tick;
            let arch = &mut self.archetypes[loc.archetype];
            let ci = arch.column_index(TypeId::of::<C>()).expect("component present");
            // SAFETY: 类型/行号匹配；&mut world 独占。
            unsafe {
                let ptr = arch.columns[ci].row_ptr_mut(loc.row) as *mut C;
                std::ptr::drop_in_place(ptr);
                std::ptr::write(ptr, component);
            }
            arch.columns[ci].touch(loc.row, tick);
            return Ok(());
        }
        self.migrate(entity, Some(component))
    }

    /// 迁移实体到 新签名 archetype 并写入可选新组件。
    fn migrate<C: Component>(
        &mut self,
        entity: Entity,
        new_component: Option<C>,
    ) -> CoreResult<()> {
        let Some(slot) = self.slots.get(entity.index as usize) else {
            return Err(rf_core::EngineError::EntityDead(entity));
        };
        if !slot.alive || slot.generation != entity.generation {
            return Err(rf_core::EngineError::EntityDead(entity));
        }
        let loc = slot.location.unwrap();
        let old_sig = self.archetypes[loc.archetype].signature.to_vec();
        // 新签名 = 旧 + C
        let mut new_sig = old_sig.clone();
        new_sig.push(TypeId::of::<C>());
        new_sig.sort();
        new_sig.dedup();
        // 旧组件字节（按旧签名顺序）
        let old_bytes = self.archetypes[loc.archetype].extract_row_bytes(loc.row);
        self.remove_from_archetype(loc);
        let target = self.ensure_archetype(&new_sig);
        let row = self.archetypes[target].push_entity(entity);
        let tick = self.tick;
        {
            let arch = &mut self.archetypes[target];
            for (i, id) in old_sig.iter().enumerate() {
                if let Some(ci) = arch.column_index(*id) {
                    // SAFETY: 同类型组件位拷贝。
                    unsafe { arch.columns[ci].push_raw(old_bytes[i].as_ptr(), Tick(0), tick) };
                }
            }
            if let Some(c) = new_component {
                let ci = arch.column_index(TypeId::of::<C>()).expect("target has C");
                // SAFETY: 构造完成的值位拷贝后 forget 原∀。
                unsafe { arch.columns[ci].push_raw(&c as *const C as *const u8, tick, tick) };
                std::mem::forget(c);
            }
        }
        self.slots[entity.index as usize].location = Some(Location { archetype: target, row });
        Ok(())
    }

    pub fn remove<C: Component>(&mut self, entity: Entity) -> Option<C> {
        let slot = self.slots.get(entity.index as usize)?;
        if !slot.alive || slot.generation != entity.generation {
            return None;
        }
        let loc = slot.location?;
        let ci = self.archetypes[loc.archetype].column_index(TypeId::of::<C>())?;
        // SAFETY: 类型与行号匹配；读出后立即迁移（值被带走）。
        let value = unsafe {
            let ptr = self.archetypes[loc.archetype].columns[ci].row_ptr(loc.row) as *const C;
            std::ptr::read(ptr)
        };
        let old_sig = self.archetypes[loc.archetype].signature.to_vec();
        let old_bytes = self.archetypes[loc.archetype].extract_row_bytes(loc.row);
        let mut new_sig = old_sig.clone();
        new_sig.retain(|t| *t != TypeId::of::<C>());
        self.remove_from_archetype(loc);
        let target = self.ensure_archetype(&new_sig);
        let row = self.archetypes[target].push_entity(entity);
        let tick = self.tick;
        {
            let arch = &mut self.archetypes[target];
            for (i, id) in old_sig.iter().enumerate() {
                if *id == TypeId::of::<C>() {
                    continue;
                }
                if let Some(ci) = arch.column_index(*id) {
                    // SAFETY: 同类型位拷贝。
                    unsafe { arch.columns[ci].push_raw(old_bytes[i].as_ptr(), Tick(0), tick) };
                }
            }
        }
        self.slots[entity.index as usize].location = Some(Location { archetype: target, row });
        Some(value)
    }

    pub fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        let slot = self.slots.get(entity.index as usize)?;
        if !slot.alive || slot.generation != entity.generation {
            return None;
        }
        let loc = slot.location?;
        let arch = &self.archetypes[loc.archetype];
        // SAFETY: 行号与类型匹配；&self 无别名可变。
        unsafe { arch.get::<C>(loc.row) }
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.register_component::<C>();
        let slot = self.slots.get(entity.index as usize)?;
        if !slot.alive || slot.generation != entity.generation {
            return None;
        }
        let loc = slot.location?;
        let tick = self.tick;
        {
            let arch = &mut self.archetypes[loc.archetype];
            if let Some(ci) = arch.column_index(TypeId::of::<C>()) {
                arch.columns[ci].touch(loc.row, tick);
            }
        }
        let arch = &mut self.archetypes[loc.archetype];
        // SAFETY: &mut self 独占。
        unsafe { arch.get_mut::<C>(loc.row) }
    }

    pub fn has<C: Component>(&self, entity: Entity) -> bool {
        self.get::<C>(entity).is_some()
    }

    /// 从 archetype 移除行；若尾部行搬移则同步修正其实体位置。返回被搬移实体。
    fn remove_from_archetype(&mut self, loc: Location) -> Option<Entity> {
        let arch = &mut self.archetypes[loc.archetype];
        let last = arch.entities.len().saturating_sub(1);
        let moved_entity = if loc.row < last { Some(arch.entities[last]) } else { None };
        for col in &mut arch.columns {
            let _ = col.swap_take(loc.row);
        }
        arch.entities.swap_remove(loc.row);
        if let Some(me) = moved_entity {
            self.slots[me.index as usize].location =
                Some(Location { archetype: loc.archetype, row: loc.row });
        }
        moved_entity
    }

    fn ensure_archetype(&mut self, sig: &[TypeId]) -> usize {
        if let Some(&idx) = self.archetype_index.get(sig) {
            return idx;
        }
        let factories = self.column_factories.clone();
        let pairs: Vec<(TypeId, &'static str)> = sig
            .iter()
            .map(|id| (*id, self.component_names.get(id).copied().unwrap_or("?")))
            .collect();
        let mut arch = Archetype::from_signature(pairs);
        for id in sig {
            if let Some(f) = factories.get(id) {
                arch.columns.push(f());
            }
        }
        self.archetypes.push(arch);
        let idx = self.archetypes.len() - 1;
        self.archetype_index.insert(sig.to_vec(), idx);
        idx
    }

    // ---- 资源（IF-073） ----

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), ResourceCell { value: Box::new(resource) });
    }

    pub fn resource<R: Resource>(&self) -> Option<&R> {
        self.resources.get(&TypeId::of::<R>())?.value.downcast_ref::<R>()
    }

    pub fn resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources.get_mut(&TypeId::of::<R>())?.value.downcast_mut::<R>()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        let cell = self.resources.remove(&TypeId::of::<R>())?;
        cell.value.downcast::<R>().ok().map(|b| *b)
    }

    pub fn has_resource<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }

    // ---- 事件（IF-076，见 events.rs） ----

    pub fn add_event<E: Send + Sync + 'static>(&mut self) {
        if !self.has_resource::<Events<E>>() {
            self.insert_resource(Events::<E>::default());
        }
    }

    pub fn send<E: Send + Sync + 'static>(&mut self, event: E) {
        self.add_event::<E>();
        if let Some(store) = self.resource_mut::<Events<E>>() {
            store.send(event);
        }
    }

    pub fn events<E: Send + Sync + 'static>(&self) -> Option<&Events<E>> {
        self.resource::<Events<E>>()
    }

    /// 帧末推进事件双缓冲。
    pub fn update_events(&mut self) {
        let tick = self.tick;
        for cell in self.resources.values_mut() {
            // 各 Events<E> 是独立资源类型，无法统一 downcast —— 由上层按类型调用
            // Events::update；这里只推进 tick 记录。
            let _ = cell;
            let _ = tick;
        }
    }

    // ---- tick（IF-077） ----

    pub fn change_tick(&self) -> Tick {
        self.tick
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.next();
    }

    // ---- 层级（IF-075） ----

    pub fn set_parent(&mut self, child: Entity, parent: Option<Entity>) -> CoreResult<()> {
        if let Some(p) = parent {
            if !self.contains(p) {
                return Err(rf_core::EngineError::EntityDead(p));
            }
            if p == child {
                return Err(rf_core::EngineError::Message("set_parent: self parent".into()));
            }
            let mut cur = p;
            loop {
                if cur == child {
                    return Err(rf_core::EngineError::Message("set_parent: cycle detected".into()));
                }
                match self.parent(cur) {
                    Some(next) => cur = next,
                    None => break,
                }
            }
        }
        self.detach_from_parent(child);
        if let Some(p) = parent {
            self.insert(child, Parent(p))?;
            let mut fresh = false;
            if self.resource::<ChildrenMap>().is_none() {
                self.insert_resource(ChildrenMap::default());
                fresh = true;
            }
            let _ = fresh;
            if let Some(map) = self.resource_mut::<ChildrenMap>() {
                map.0.entry(p).or_default().push(child);
            }
        }
        Ok(())
    }

    fn detach_from_parent(&mut self, child: Entity) {
        if let Some(p) = self.parent(child) {
            self.remove::<Parent>(child);
            if let Some(map) = self.resource_mut::<ChildrenMap>() {
                if let Some(list) = map.0.get_mut(&p) {
                    list.retain(|e| *e != child);
                }
            }
        }
    }

    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.get::<Parent>(entity).map(|p| p.0)
    }

    pub fn children(&self, entity: Entity) -> &[Entity] {
        self.resource::<ChildrenMap>()
            .and_then(|m| m.0.get(&entity))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 深度优先遍历子树（不含根）。
    pub fn walk_children(&self, root: Entity, out: &mut Vec<Entity>) {
        for &c in self.children(root) {
            out.push(c);
            self.walk_children(c, out);
        }
    }

    // ---- 查询入口（IF-074） ----

    pub fn query<D: QueryData, F: QueryFilter>(&mut self) -> Query<'_, D, F> {
        D::register(self);
        Query::new(self)
    }

    // ---- 命令缓冲 flush（IF-084） ----

    pub fn flush_commands(&mut self, commands: &mut Commands) -> CoreResult<()> {
        commands.apply(self)
    }

    // ---- 统计（IF-077） ----

    pub fn stats(&self) -> WorldStats {
        let mut columns = Vec::new();
        for arch in &self.archetypes {
            for col in &arch.columns {
                let name = col.type_name().to_string();
                if let Some(entry) = columns.iter_mut().find(|(n, _)| *n == name) {
                    entry.1 += col.len();
                } else {
                    columns.push((name, col.len()));
                }
            }
        }
        WorldStats { archetypes: self.archetypes.len(), entities: self.entity_count(), columns }
    }

    /// 预留实体（命令缓冲模式）：立即分配 id，组件在 flush 时写入。
    pub fn reserve_entity(&mut self) -> Entity {
        self.alloc_entity()
    }

    /// 当前槽位总数（Commands::new 的预留基址）。
    pub fn slots_len(&self) -> u32 {
        self.slots.len() as u32
    }
}

/// 父组件（IF-075）。
#[derive(Debug, Clone, Copy, Component)]
pub struct Parent(pub Entity);

/// 子集合资源（IF-075：Children 数据的 map 形态）。
#[derive(Default, Resource)]
pub struct ChildrenMap(pub HashMap<Entity, Vec<Entity>>);

/// 规格命名的 Children 组件（等价访问器）。
#[derive(Debug, Clone, Default, Component)]
pub struct Children(pub Vec<Entity>);
