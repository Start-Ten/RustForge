//! 命令缓冲（IF-084）：延迟到帧同步点执行。

use crate::{Bundle, Component, World};
use rf_core::{Entity, Result as CoreResult};

/// 单条命令（类型擦除闭包）。
pub struct Command {
    #[allow(clippy::type_complexity)]
    run: Box<dyn FnOnce(&mut World) -> CoreResult<()> + Send>,
}

impl Command {
    /// 自定义命令。
    pub fn custom<F: FnOnce(&mut World) -> CoreResult<()> + Send + 'static>(f: F) -> Self {
        Self { run: Box::new(f) }
    }

    pub(crate) fn apply(self, world: &mut World) -> CoreResult<()> {
        (self.run)(world)
    }
}

/// 延迟命令缓冲（IF-084）。spawn 返回“预留实体”，flush 前仅作标识。
/// 约定：同一时刻只应有一个未 flush 的 Commands 指向同一 World。
pub struct Commands {
    queued: Vec<Command>,
    reserve_base: u32,
    next_reserved: u32,
}

impl Default for Commands {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Commands {
    pub fn new(world_slot_count: u32) -> Self {
        Self { queued: Vec::new(), reserve_base: world_slot_count, next_reserved: 0 }
    }

    fn reserve(&mut self) -> Entity {
        let index = self.reserve_base + self.next_reserved;
        self.next_reserved += 1;
        Entity::new(index, 1)
    }

    /// 延迟 spawn；返回预留实体（flush 前仅可作本缓冲内后续命令的参数）。
    pub fn spawn<B: Bundle + Send + 'static>(&mut self, bundle: B) -> Entity {
        let entity = self.reserve();
        self.queued.push(Command::custom(move |world| {
            world.adopt_reserved(entity.index);
            let real = world.spawn_empty_adopted(entity.index);
            let _ = real;
            // 直接向预留槽写入 bundle
            unsafe { bundle_attach(bundle, world, entity) };
            Ok(())
        }));
        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queued.push(Command::custom(move |world| {
            let _ = world.despawn(entity);
            Ok(())
        }));
    }

    pub fn insert<C: Component + Send + 'static>(&mut self, entity: Entity, component: C) {
        self.queued.push(Command::custom(move |world| world.insert(entity, component)));
    }

    pub fn remove<C: Component + Send + 'static>(&mut self, entity: Entity) {
        self.queued.push(Command::custom(move |world| {
            let _ = world.remove::<C>(entity);
            Ok(())
        }));
    }

    pub fn set_parent(&mut self, child: Entity, parent: Option<Entity>) {
        self.queued.push(Command::custom(move |world| world.set_parent(child, parent)));
    }

    pub fn len(&self) -> usize {
        self.queued.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }

    /// 应用全部命令（World::flush_commands 调用）。
    pub fn apply(&mut self, world: &mut World) -> CoreResult<()> {
        for cmd in std::mem::take(&mut self.queued) {
            cmd.apply(world)?;
        }
        self.next_reserved = 0;
        Ok(())
    }
}

/// SAFETY: bundle 的 attach 需要实体已存在——由 adopt_reserved 保证。
unsafe fn bundle_attach<B: Bundle>(bundle: B, world: &mut World, entity: Entity) {
    let tick = world.change_tick();
    bundle.attach(world, entity, tick)
}

impl World {
    /// 命令缓冲预留实体的槽位落地（crate 外经 Commands::spawn 自动调用）。
    pub(crate) fn adopt_reserved(&mut self, index: u32) {
        while self.slots.len() <= index as usize {
            self.slots.push(crate::Slot { generation: 1, alive: false, location: None });
        }
    }

    pub(crate) fn spawn_empty_adopted(&mut self, index: u32) -> Entity {
        let s = &mut self.slots[index as usize];
        s.alive = true;
        let entity = Entity::new(index, s.generation);
        let row = self.archetypes[0].push_entity(entity);
        self.slots[index as usize].location = Some(crate::Location { archetype: 0, row });
        entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;

    #[derive(Debug, PartialEq, Component)]
    struct Pos(i32);

    #[derive(Debug, PartialEq, Component)]
    struct Vel(i32);

    #[test]
    fn commands_roundtrip() {
        let mut world = World::new();
        let mut cmds = Commands::new(world.slots_len());
        let e = cmds.spawn((Pos(1), Vel(2)));
        cmds.insert(e, Pos(10)); // flush 前的同缓冲覆盖
        world.flush_commands(&mut cmds).unwrap();
        assert!(world.contains(e));
        assert_eq!(world.get::<Pos>(e), Some(&Pos(10)));
        cmds.despawn(e);
        world.flush_commands(&mut cmds).unwrap();
        assert!(!world.contains(e));
    }

    #[test]
    fn commands_set_parent() {
        let mut world = World::new();
        let mut cmds = Commands::new(world.slots_len());
        let parent = cmds.spawn((Pos(0),));
        let child = cmds.spawn((Pos(1),));
        cmds.set_parent(child, Some(parent));
        world.flush_commands(&mut cmds).unwrap();
        assert_eq!(world.parent(child), Some(parent));
        assert_eq!(world.children(parent), &[child]);
    }
}
