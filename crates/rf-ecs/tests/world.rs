//! ECS 集成测试：实体生命周期、查询、变更检测、层级、事件、命令。

use rf_core::Tick;
use rf_ecs::*;

#[derive(Debug, PartialEq, Component)]
struct Pos {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq, Component)]
struct Vel {
    dx: f32,
    dy: f32,
}

#[derive(Debug, PartialEq, Component)]
struct Tag(&'static str);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Collision {
    other: rf_core::Entity,
}

#[test]
fn entity_lifecycle_generations() {
    let mut world = World::new();
    let e1 = world.spawn_empty();
    assert!(world.contains(e1));
    assert!(world.despawn(e1));
    assert!(!world.contains(e1));
    // 槽位复用：新实体同 index 不同 generation
    let e2 = world.spawn_empty();
    assert_eq!(e2.index, e1.index);
    assert_ne!(e2.generation, e1.generation);
    assert!(!world.contains(e1)); // 旧句柄失效
    assert!(world.contains(e2));
    // 重复 despawn 幂等
    assert!(world.despawn(e2));
    assert!(!world.despawn(e2));
}

#[test]
fn spawn_bundle_and_get() {
    let mut world = World::new();
    let e = world.spawn((Pos { x: 1.0, y: 2.0 }, Vel { dx: 0.5, dy: -0.5 }));
    assert_eq!(world.get::<Pos>(e), Some(&Pos { x: 1.0, y: 2.0 }));
    assert_eq!(world.get::<Vel>(e), Some(&Vel { dx: 0.5, dy: -0.5 }));
    assert!(!world.has::<Tag>(e));
    world.get_mut::<Pos>(e).unwrap().x = 9.0;
    assert_eq!(world.get::<Pos>(e).unwrap().x, 9.0);
}

#[test]
fn insert_remove_migrates_archetypes() {
    let mut world = World::new();
    let e = world.spawn((Pos { x: 0.0, y: 0.0 },));
    assert_eq!(world.stats().archetypes, 2); // empty + {Pos}
    world.insert(e, Vel { dx: 1.0, dy: 1.0 }).unwrap();
    assert_eq!(world.get::<Vel>(e), Some(&Vel { dx: 1.0, dy: 1.0 }));
    assert_eq!(world.get::<Pos>(e), Some(&Pos { x: 0.0, y: 0.0 })); // 迁移保留旧组件
    let removed = world.remove::<Vel>(e).unwrap();
    assert_eq!(removed, Vel { dx: 1.0, dy: 1.0 });
    assert!(!world.has::<Vel>(e));
    assert_eq!(world.get::<Pos>(e), Some(&Pos { x: 0.0, y: 0.0 }));
    // 覆盖插入
    world.insert(e, Pos { x: 5.0, y: 5.0 }).unwrap();
    assert_eq!(world.get::<Pos>(e), Some(&Pos { x: 5.0, y: 5.0 }));
    // 死实体报错
    let dead = world.spawn_empty();
    world.despawn(dead);
    assert!(matches!(world.insert(dead, Tag("x")), Err(rf_core::EngineError::EntityDead(_))));
}

#[test]
fn swap_remove_fixes_moved_entity() {
    let mut world = World::new();
    let a = world.spawn((Pos { x: 1.0, y: 1.0 },));
    let b = world.spawn((Pos { x: 2.0, y: 2.0 },));
    let c = world.spawn((Pos { x: 3.0, y: 3.0 },));
    world.despawn(b); // b 在中间，c 应搬移到 b 的行
    assert_eq!(world.get::<Pos>(a), Some(&Pos { x: 1.0, y: 1.0 }));
    assert_eq!(world.get::<Pos>(c), Some(&Pos { x: 3.0, y: 3.0 })); // c 数据完好
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn typed_query_iteration() {
    let mut world = World::new();
    for i in 0..10 {
        world.spawn((Pos { x: i as f32, y: 0.0 }, Vel { dx: 1.0, dy: 0.0 }));
    }
    for i in 0..5 {
        world.spawn((Pos { x: i as f32, y: 100.0 },)); // 无 Vel
    }
    // 只读查询
    let sum: f32 = world.query::<&Pos, ()>().iter().map(|p| p.x).sum();
    assert_eq!(sum, 45.0 + 10.0); // 0..9 + 0..4
                                  // 双组件查询只匹配 10 个
    let count = world.query::<(&Pos, &Vel), ()>().iter().count();
    assert_eq!(count, 10);
    // 可变查询修改
    for v in world.query::<&mut Vel, ()>().iter() {
        v.dx *= 2.0;
    }
    assert!(world.query::<&Vel, ()>().iter().all(|v| v.dx == 2.0));
    // Entity 数据查询
    let entities: Vec<rf_core::Entity> =
        world.query::<rf_core::Entity, With<Vel>>().iter().collect();
    assert_eq!(entities.len(), 10);
    // 元组含 Entity
    let pairs = world.query::<(rf_core::Entity, &Pos), Without<Vel>>().iter().count();
    assert_eq!(pairs, 5);
    // Option 查询
    let with_opt = world.query::<(&Pos, Option<&Vel>), ()>().iter().count();
    assert_eq!(with_opt, 15);
    let some_vel =
        world.query::<(&Pos, Option<&Vel>), ()>().iter().filter(|(_, v)| v.is_some()).count();
    assert_eq!(some_vel, 10);
}

#[test]
fn query_with_filter() {
    let mut world = World::new();
    let _a = world.spawn((Pos { x: 0.0, y: 0.0 }, Tag("player")));
    let _b = world.spawn((Pos { x: 1.0, y: 1.0 }, Tag("enemy")));
    let _c = world.spawn((Pos { x: 2.0, y: 2.0 },));
    let tagged = world.query::<&Pos, With<Tag>>().iter().count();
    assert_eq!(tagged, 2);
    let untagged = world.query::<&Pos, Without<Tag>>().iter().count();
    assert_eq!(untagged, 1);
}

#[test]
fn change_detection() {
    let mut world = World::new();
    let e = world.spawn((Pos { x: 0.0, y: 0.0 },));
    // tick 语义：since 为"上一观察点"，变更标记 tick > since 才计数。
    let t0 = world.change_tick(); // spawn 发生在 t0
    assert_eq!(world.query::<&Pos, Changed<Pos>>().changed_since(t0).iter().count(), 0);

    // 帧：先推进 tick，再修改（变更标记为 t1 > t0）
    world.advance_tick();
    let t1 = world.change_tick();
    world.get_mut::<Pos>(e).unwrap().x = 1.0;
    assert_eq!(world.query::<&Pos, Changed<Pos>>().changed_since(t0).iter().count(), 1);
    assert_eq!(world.query::<&Pos, Changed<Pos>>().changed_since(t1).iter().count(), 0);

    // 新实体 Added：观察点取在 spawn 所在 tick 之前
    let t1_end = world.change_tick(); // == t1
    world.advance_tick(); // spawn 将标记为 t2 > t1
    let _e2 = world.spawn((Pos { x: 9.0, y: 9.0 },));
    assert_eq!(world.query::<&Pos, Added<Pos>>().changed_since(t1_end).iter().count(), 1);
    assert_eq!(world.query::<&Pos, Added<Pos>>().changed_since(Tick(0)).iter().count(), 2); // 两个都在 Tick(0) 之后
    assert_eq!(world.query::<&Pos, Changed<Pos>>().changed_since(t1).iter().count(), 1);
    // 只有 e2 新增/变更
}

#[test]
fn resources() {
    #[derive(Default, Resource, PartialEq, Debug)]
    struct Score(u32);
    let mut world = World::new();
    assert!(world.resource::<Score>().is_none());
    world.insert_resource(Score(0));
    assert_eq!(world.resource::<Score>(), Some(&Score(0)));
    *world.resource_mut::<Score>().unwrap() = Score(42);
    assert_eq!(world.resource::<Score>(), Some(&Score(42)));
    assert_eq!(world.remove_resource::<Score>(), Some(Score(42)));
    assert!(world.resource::<Score>().is_none());
}

#[test]
fn events_double_buffer() {
    let mut world = World::new();
    world.add_event::<Collision>();
    world.send(Collision { other: rf_core::Entity::new(1, 1) });
    world.send(Collision { other: rf_core::Entity::new(2, 1) });
    let mut reader = EventReader::new();
    let read1 = reader.iter(world.events::<Collision>().unwrap()).copied().count();
    assert_eq!(read1, 2);
    // update 后仍在 previous 缓冲内，新读者可读
    let _ = world.events::<Collision>().unwrap();
    // 手动推进（引擎帧循环职责）
    if let Some(store) = world.resource_mut::<Events<Collision>>() {
        store.update(rf_core::Tick(2));
    }
    let mut reader2 = EventReader::new();
    assert_eq!(reader2.iter(world.events::<Collision>().unwrap()).count(), 2); // 上一帧保留
    world.send(Collision { other: rf_core::Entity::new(3, 1) });
    assert_eq!(reader.iter(world.events::<Collision>().unwrap()).count(), 1); // 增量
}

#[test]
fn hierarchy_and_cycle_detection() {
    let mut world = World::new();
    let root = world.spawn((Pos { x: 0.0, y: 0.0 },));
    let child = world.spawn((Pos { x: 1.0, y: 0.0 },));
    let grand = world.spawn((Pos { x: 2.0, y: 0.0 },));
    world.set_parent(child, Some(root)).unwrap();
    world.set_parent(grand, Some(child)).unwrap();
    assert_eq!(world.parent(child), Some(root));
    assert_eq!(world.children(root), &[child]);
    let mut walk = Vec::new();
    world.walk_children(root, &mut walk);
    assert_eq!(walk, vec![child, grand]);
    // 循环：root → grand 会成环
    assert!(world.set_parent(root, Some(grand)).is_err());
    // 自引用
    assert!(world.set_parent(child, Some(child)).is_err());
    // 换父
    world.set_parent(grand, Some(root)).unwrap();
    assert_eq!(world.children(root), &[child, grand]);
    assert_eq!(world.children(child), &[]);
    // despawn_recursive
    assert!(world.despawn_recursive(root));
    assert!(!world.contains(child));
    assert!(!world.contains(grand));
}

#[test]
fn commands_roundtrip_integration() {
    let mut world = World::new();
    let mut cmds = Commands::new(world.slots_len());
    let a = cmds.spawn((Pos { x: 1.0, y: 1.0 }, Vel { dx: 1.0, dy: 0.0 }));
    let b = cmds.spawn((Pos { x: 2.0, y: 2.0 },));
    cmds.set_parent(b, Some(a));
    world.flush_commands(&mut cmds).unwrap();
    assert_eq!(world.entity_count(), 2);
    assert_eq!(world.parent(b), Some(a));
    // spawn_batch
    let spawned = world.spawn_batch((0..5).map(|i| (Pos { x: i as f32, y: 0.0 },)));
    assert_eq!(spawned.len(), 5);
    assert_eq!(world.entity_count(), 7);
}

#[test]
fn scheduler_integration() {
    let mut world = World::new();
    #[derive(Default, Resource)]
    struct Frame(u32);
    world.insert_resource(Frame::default());
    let mut sched = Schedule::new();
    sched.add_stage(Stage::new("update").with_set(SystemSet::new("physics").add(Box::new(
        system("integrate", |w: &mut World| {
            for v in w.query::<&mut Vel, ()>().iter() {
                let _ = v;
            }
            *w.resource_mut::<Frame>().unwrap() = Frame(w.resource::<Frame>().unwrap().0 + 1);
        }),
    ))));
    for _ in 0..3 {
        sched.run(&mut world, None);
    }
    assert_eq!(world.resource::<Frame>().unwrap().0, 3);
}

#[test]
fn drop_components_on_despawn() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    #[derive(Component)]
    struct DropCounter(#[allow(dead_code)] Arc<()>);
    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }
    let mut world = World::new();
    let keep = Arc::new(());
    {
        let e = world.spawn((DropCounter(keep.clone()),));
        let _ = e;
    }
    world.clear();
    assert!(DROPS.load(Ordering::SeqCst) >= 1);
}

#[test]
fn stress_many_entities() {
    // 冒烟级压力：10 万实体（规格 Target 10M 见 bench 示例）
    let mut world = World::new();
    for i in 0..100_000u32 {
        world.spawn((Pos { x: i as f32, y: 0.0 }, Vel { dx: 1.0, dy: 1.0 }));
    }
    assert_eq!(world.entity_count(), 100_000);
    let count = world.query::<(&Pos, &Vel), ()>().iter().count();
    assert_eq!(count, 100_000);
    // 删除一半
    let all: Vec<_> = world.entities();
    for e in all.iter().step_by(2) {
        world.despawn(*e);
    }
    assert_eq!(world.entity_count(), 50_000);
    let after = world.query::<&Pos, ()>().iter().count();
    assert_eq!(after, 50_000);
}
