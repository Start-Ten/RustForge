//! RustForge AI（IF-240 ~ IF-245）：行为树、黑板、A*/流场、感知、群体。

use rf_math::Vec2;
use std::collections::HashMap;

// ---- 行为树（IF-240/241） ----

/// 行为树状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtStatus {
    Success,
    Failure,
    Running,
}

/// 行为树节点（IF-240）。
pub trait BtNode {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus;
    fn name(&self) -> &str;
}

/// 叶子：动作。
pub fn action<F: Fn(&mut Blackboard) -> BtStatus + Send + 'static>(
    name: impl Into<String>,
    f: F,
) -> Box<dyn BtNode> {
    struct Action<F>(String, F);
    impl<F: Fn(&mut Blackboard) -> BtStatus + Send> BtNode for Action<F> {
        fn tick(&mut self, bb: &mut Blackboard, _dt: f32) -> BtStatus {
            (self.1)(bb)
        }
        fn name(&self) -> &str {
            &self.0
        }
    }
    Box::new(Action(name.into(), f))
}

/// 叶子：条件。
pub fn condition<F: Fn(&Blackboard) -> bool + Send + 'static>(
    name: impl Into<String>,
    f: F,
) -> Box<dyn BtNode> {
    struct Cond<F>(String, F);
    impl<F: Fn(&Blackboard) -> bool + Send> BtNode for Cond<F> {
        fn tick(&mut self, bb: &mut Blackboard, _dt: f32) -> BtStatus {
            if (self.1)(bb) {
                BtStatus::Success
            } else {
                BtStatus::Failure
            }
        }
        fn name(&self) -> &str {
            &self.0
        }
    }
    Box::new(Cond(name.into(), f))
}

/// 顺序节点：全部成功才成功。
pub struct Sequence(pub Vec<Box<dyn BtNode>>);

impl BtNode for Sequence {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus {
        for child in &mut self.0 {
            match child.tick(bb, dt) {
                BtStatus::Success => continue,
                other => return other,
            }
        }
        BtStatus::Success
    }
    fn name(&self) -> &str {
        "sequence"
    }
}

/// 选择节点：任一成功即成功。
pub struct Selector(pub Vec<Box<dyn BtNode>>);

impl BtNode for Selector {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus {
        for child in &mut self.0 {
            match child.tick(bb, dt) {
                BtStatus::Failure => continue,
                other => return other,
            }
        }
        BtStatus::Failure
    }
    fn name(&self) -> &str {
        "selector"
    }
}

/// 并行节点：至少 success_needed 个成功。
pub struct Parallel {
    pub children: Vec<Box<dyn BtNode>>,
    pub success_needed: usize,
}

impl BtNode for Parallel {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus {
        let mut ok = 0;
        let mut any_running = false;
        for c in &mut self.children {
            match c.tick(bb, dt) {
                BtStatus::Success => ok += 1,
                BtStatus::Running => any_running = true,
                BtStatus::Failure => {}
            }
        }
        if ok >= self.success_needed {
            BtStatus::Success
        } else if any_running {
            BtStatus::Running
        } else {
            BtStatus::Failure
        }
    }
    fn name(&self) -> &str {
        "parallel"
    }
}

/// 反转装饰。
pub struct Invert(pub Box<dyn BtNode>);

impl BtNode for Invert {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus {
        match self.0.tick(bb, dt) {
            BtStatus::Success => BtStatus::Failure,
            BtStatus::Failure => BtStatus::Success,
            BtStatus::Running => BtStatus::Running,
        }
    }
    fn name(&self) -> &str {
        "invert"
    }
}

/// 冷却装饰。
pub struct Cooldown {
    pub child: Box<dyn BtNode>,
    pub duration: f32,
    remaining: f32,
}

impl Cooldown {
    pub fn new(child: Box<dyn BtNode>, duration: f32) -> Self {
        Self { child, duration, remaining: 0.0 }
    }
}

impl BtNode for Cooldown {
    fn tick(&mut self, bb: &mut Blackboard, dt: f32) -> BtStatus {
        if self.remaining > 0.0 {
            self.remaining -= dt;
            return BtStatus::Failure;
        }
        let r = self.child.tick(bb, dt);
        if r == BtStatus::Success {
            self.remaining = self.duration;
        }
        r
    }
    fn name(&self) -> &str {
        "cooldown"
    }
}

/// 恒成功。
pub struct AlwaysSucceed;

impl BtNode for AlwaysSucceed {
    fn tick(&mut self, _bb: &mut Blackboard, _dt: f32) -> BtStatus {
        BtStatus::Success
    }
    fn name(&self) -> &str {
        "always_succeed"
    }
}

// ---- 黑板（IF-242） ----

/// 黑板值。
#[derive(Debug, Clone, PartialEq)]
pub enum BbValue {
    Bool(bool),
    F32(f32),
    I64(i64),
    Str(String),
    Vec2(Vec2),
}

/// 观察者条目。
type Observer = (u64, Box<dyn Fn() + Send>);

/// 黑板：键值 + 变更观察者。
pub struct Blackboard {
    data: HashMap<String, BbValue>,
    observers: HashMap<String, Vec<Observer>>,
    next_obs: u64,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Blackboard {
    pub fn new() -> Self {
        Self { data: HashMap::new(), observers: HashMap::new(), next_obs: 1 }
    }

    pub fn set(&mut self, key: &str, v: BbValue) {
        let changed = self.data.get(key) != Some(&v);
        self.data.insert(key.to_string(), v);
        if changed {
            if let Some(obs) = self.observers.get(key) {
                for (_, cb) in obs {
                    cb();
                }
            }
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.data.get(key) {
            Some(BbValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    pub fn get_f32(&self, key: &str) -> Option<f32> {
        match self.data.get(key) {
            Some(BbValue::F32(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_vec2(&self, key: &str) -> Option<Vec2> {
        match self.data.get(key) {
            Some(BbValue::Vec2(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.data.get(key) {
            Some(BbValue::Str(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// 注册变更观察者，返回 id。
    pub fn on_change(&mut self, key: &str, cb: Box<dyn Fn() + Send>) -> u64 {
        let id = self.next_obs;
        self.next_obs += 1;
        self.observers.entry(key.to_string()).or_default().push((id, cb));
        id
    }

    pub fn remove_observer(&mut self, key: &str, id: u64) {
        if let Some(v) = self.observers.get_mut(key) {
            v.retain(|(i, _)| *i != id);
        }
    }
}

// ---- 寻路（IF-243） ----

/// 导航网格。
pub struct NavGrid {
    pub width: i32,
    pub height: i32,
    blocked: Vec<bool>,
}

impl NavGrid {
    pub fn new(w: i32, h: i32) -> Self {
        Self { width: w, height: h, blocked: vec![false; (w * h) as usize] }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn blocked(&self, x: i32, y: i32) -> bool {
        if !self.in_bounds(x, y) {
            return true;
        }
        self.blocked[(y * self.width + x) as usize]
    }

    pub fn set_blocked(&mut self, x: i32, y: i32, b: bool) {
        if self.in_bounds(x, y) {
            self.blocked[(y * self.width + x) as usize] = b;
        }
    }
}

/// A*（8 向、禁切角，IF-243）。
pub fn astar(grid: &NavGrid, start: (i32, i32), goal: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct State(f32, (i32, i32));
    impl Eq for State {}
    impl Ord for State {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
        }
    }
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    if grid.blocked(start.0, start.1) || grid.blocked(goal.0, goal.1) {
        return None;
    }
    let h = |p: (i32, i32)| -> f32 { ((p.0 - goal.0).abs() + (p.1 - goal.1).abs()) as f32 };
    let mut open = BinaryHeap::new();
    let mut g = HashMap::<(i32, i32), f32>::new();
    let mut came = HashMap::<(i32, i32), (i32, i32)>::new();
    open.push(State(h(start), start));
    g.insert(start, 0.0);
    let mut closed = std::collections::HashSet::new();
    while let Some(State(_, cur)) = open.pop() {
        if cur == goal {
            let mut path = vec![cur];
            let mut c = cur;
            while let Some(&prev) = came.get(&c) {
                path.push(prev);
                c = prev;
            }
            path.reverse();
            return Some(path);
        }
        if !closed.insert(cur) {
            continue;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let n = (cur.0 + dx, cur.1 + dy);
            if grid.blocked(n.0, n.1) {
                continue;
            }
            // 禁切角
            if dx != 0
                && dy != 0
                && (grid.blocked(cur.0 + dx, cur.1) || grid.blocked(cur.0, cur.1 + dy))
            {
                continue;
            }
            let cost = if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
            let ng = g[&cur] + cost;
            if ng < *g.get(&n).unwrap_or(&f32::INFINITY) {
                g.insert(n, ng);
                came.insert(n, cur);
                open.push(State(ng + h(n), n));
            }
        }
    }
    None
}

/// 流场方向编码：位 0-3 = E W N S，4-7 = 对角。
pub fn flow_field(grid: &NavGrid, goal: (i32, i32)) -> Vec<u8> {
    // BFS 从 goal 反向扩散
    let mut dist = vec![i32::MAX; (grid.width * grid.height) as usize];
    let mut queue = std::collections::VecDeque::new();
    let gi = (goal.1 * grid.width + goal.0) as usize;
    dist[gi] = 0;
    queue.push_back(goal);
    while let Some(c) = queue.pop_front() {
        let cd = dist[(c.1 * grid.width + c.0) as usize];
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let n = (c.0 + dx, c.1 + dy);
            if grid.blocked(n.0, n.1) {
                continue;
            }
            let ni = (n.1 * grid.width + n.0) as usize;
            if dist[ni] > cd + 1 {
                dist[ni] = cd + 1;
                queue.push_back(n);
            }
        }
    }
    // 方向 = 邻居中 dist 最小者
    let idx = |d: (i32, i32), bits: u8| -> u8 {
        match (d, bits) {
            ((1, 0), _) => 1,  // E
            ((-1, 0), _) => 2, // W
            ((0, -1), _) => 4, // N
            ((0, 1), _) => 8,  // S
            _ => bits,
        }
    };
    let mut out = vec![0u8; (grid.width * grid.height) as usize];
    for y in 0..grid.height {
        for x in 0..grid.width {
            let i = (y * grid.width + x) as usize;
            if grid.blocked(x, y) {
                continue;
            }
            let mut best = dist[i];
            let mut best_dir = (0, 0);
            for d in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let n = (x + d.0, y + d.1);
                if grid.blocked(n.0, n.1) {
                    continue;
                }
                let nd = dist[(n.1 * grid.width + n.0) as usize];
                if nd < best {
                    best = nd;
                    best_dir = d;
                }
            }
            out[i] = idx(best_dir, 0);
        }
    }
    out
}

/// 沿流场走一步（IF-243）。
pub fn follow(flow: &[u8], grid: &NavGrid, pos: (i32, i32)) -> Option<(i32, i32)> {
    if !grid.in_bounds(pos.0, pos.1) {
        return None;
    }
    let bits = flow[(pos.1 * grid.width + pos.0) as usize];
    let d = match bits {
        1 => (1, 0),
        2 => (-1, 0),
        4 => (0, -1),
        8 => (0, 1),
        _ => return None,
    };
    Some((pos.0 + d.0, pos.1 + d.1))
}

// ---- 感知（IF-244） ----

/// 感知结果。
#[derive(Debug, Clone, Copy)]
pub struct Perceived {
    pub id: u64,
    pub last_seen: Vec2,
    pub visible: bool,
}

/// 2D 感知（视距 + 视锥 + 记忆）。
pub struct Perception2d {
    pub view_range: f32,
    pub fov_rad: f32,
    pub memory: f32,
    records: HashMap<u64, (Vec2, f32)>,
}

impl Perception2d {
    pub fn new(view_range: f32, fov_rad: f32, memory: f32) -> Self {
        Self { view_range, fov_rad, memory, records: HashMap::new() }
    }

    /// 感知一批目标（now 单调秒）。
    pub fn sense(
        &mut self,
        now: f32,
        observer: Vec2,
        facing: Vec2,
        targets: &[(u64, Vec2)],
    ) -> Vec<Perceived> {
        let facing = facing.normalized();
        let mut out = Vec::new();
        for (id, pos) in targets {
            let d = *pos - observer;
            let dist = d.length();
            let mut visible = dist <= self.view_range;
            if visible {
                let ang = d.normalized().dot(facing);
                visible = ang >= (self.fov_rad * 0.5).cos();
            }
            if visible {
                self.records.insert(*id, (*pos, now));
            }
            let last = self.records.get(id).copied();
            if let Some((last_seen, seen_at)) = last {
                if now - seen_at <= self.memory {
                    out.push(Perceived { id: *id, last_seen, visible });
                }
            }
        }
        out
    }
}

// ---- 群体（IF-245） ----

/// Boids 合力。
pub fn boids(
    self_pos: Vec2,
    self_vel: Vec2,
    neighbors: &[(Vec2, Vec2)],
    separation_w: f32,
    alignment_w: f32,
    cohesion_w: f32,
) -> Vec2 {
    let mut sep = Vec2::ZERO;
    let mut align = Vec2::ZERO;
    let mut coh = Vec2::ZERO;
    let n = neighbors.len().max(1) as f32;
    for (p, v) in neighbors {
        let d = self_pos - *p;
        let dist = d.length().max(f32::EPSILON);
        sep += d / (dist * dist);
        align += *v;
        coh += *p;
    }
    sep / n * separation_w
        + (align / n - self_vel) * alignment_w
        + (coh / n - self_pos) * cohesion_w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_tree_composites() {
        let mut bb = Blackboard::new();
        let mut tree = Selector(vec![
            Box::new(Sequence(vec![
                condition("hungry", |bb| bb.get_bool("hungry").unwrap_or(false)),
                action("eat", |bb| {
                    bb.set("fed", BbValue::Bool(true));
                    BtStatus::Success
                }),
            ])),
            action("wander", |_bb| BtStatus::Running),
        ]);
        bb.set("hungry", BbValue::Bool(false));
        assert_eq!(tree.tick(&mut bb, 0.1), BtStatus::Running); // 走 wander
        bb.set("hungry", BbValue::Bool(true));
        assert_eq!(tree.tick(&mut bb, 0.1), BtStatus::Success);
        assert_eq!(bb.get_bool("fed"), Some(true));
        // Invert + Cooldown
        let mut inv = Invert(action("fail", |_| BtStatus::Failure));
        assert_eq!(inv.tick(&mut bb, 0.1), BtStatus::Success);
        let mut cd = Cooldown::new(action("ok", |_| BtStatus::Success), 1.0);
        assert_eq!(cd.tick(&mut bb, 0.1), BtStatus::Success);
        assert_eq!(cd.tick(&mut bb, 0.1), BtStatus::Failure); // 冷却中
                                                              // Parallel
        let mut par = Parallel {
            children: vec![action("a", |_| BtStatus::Success), action("b", |_| BtStatus::Failure)],
            success_needed: 1,
        };
        assert_eq!(par.tick(&mut bb, 0.1), BtStatus::Success);
    }

    #[test]
    fn blackboard_observers() {
        let mut bb = Blackboard::new();
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2 = count.clone();
        let id = bb.on_change(
            "hp",
            Box::new(move || {
                c2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );
        bb.set("hp", BbValue::F32(100.0));
        bb.set("hp", BbValue::F32(90.0));
        bb.set("hp", BbValue::F32(90.0)); // 同值不触发
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
        bb.remove_observer("hp", id);
        bb.set("hp", BbValue::F32(10.0));
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn astar_shortest_path() {
        let mut g = NavGrid::new(10, 10);
        // 中间竖墙（留上方通道）
        for y in 1..9 {
            g.set_blocked(4, y, true);
        }
        let path = astar(&g, (1, 5), (8, 5)).unwrap();
        assert_eq!(path.first(), Some(&(1, 5)));
        assert_eq!(path.last(), Some(&(8, 5)));
        // 无穿墙
        for p in &path {
            assert!(!g.blocked(p.0, p.1));
        }
        // 曼哈顿 7 + 绕行
        assert!(path.len() >= 9 && path.len() <= 20);
        // 完全封闭 → 无解
        let mut g2 = NavGrid::new(3, 3);
        g2.set_blocked(1, 0, true);
        g2.set_blocked(1, 1, true);
        g2.set_blocked(1, 2, true);
        assert!(astar(&g2, (0, 1), (2, 1)).is_none());
    }

    #[test]
    fn astar_no_corner_cutting() {
        let mut g = NavGrid::new(3, 3);
        g.set_blocked(1, 1, true);
        let path = astar(&g, (0, 0), (2, 2)).unwrap();
        // 不允许对角切过 (1,1)
        assert!(path.len() > 2); // 必须绕行
    }

    #[test]
    fn flow_field_follows_to_goal() {
        let mut g = NavGrid::new(8, 8);
        g.set_blocked(3, 3, true);
        let flow = flow_field(&g, (7, 7));
        let mut pos = (0, 0);
        for _ in 0..64 {
            match follow(&flow, &g, pos) {
                Some(next) if !g.blocked(next.0, next.1) => pos = next,
                _ => break,
            }
            if pos == (7, 7) {
                break;
            }
        }
        assert_eq!(pos, (7, 7));
    }

    #[test]
    fn perception_fov_and_memory() {
        let mut p = Perception2d::new(10.0, std::f32::consts::FRAC_PI_2, 2.0);
        let observer = Vec2::ZERO;
        let facing = Vec2::new(1.0, 0.0);
        // 视锥内
        let seen = p.sense(0.0, observer, facing, &[(1, Vec2::new(5.0, 0.0))]);
        assert_eq!(seen.len(), 1);
        assert!(seen[0].visible);
        // 背后 → 不可见但记忆保留
        let seen = p.sense(1.0, observer, facing, &[(1, Vec2::new(-5.0, 0.0))]);
        assert_eq!(seen.len(), 1);
        assert!(!seen[0].visible);
        assert_eq!(seen[0].last_seen, Vec2::new(5.0, 0.0)); // 最后可见位置
                                                            // 超时遗忘
        let seen = p.sense(5.0, observer, facing, &[(1, Vec2::new(-5.0, 0.0))]);
        assert!(seen.is_empty());
    }

    #[test]
    fn boids_separation_dominates() {
        let f = boids(Vec2::ZERO, Vec2::ZERO, &[(Vec2::new(0.1, 0.0), Vec2::ZERO)], 10.0, 0.1, 0.1);
        assert!(f.x < 0.0); // 远离近邻
    }
}
