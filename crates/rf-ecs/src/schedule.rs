//! 调度器（IF-085 ~ IF-086）：System/SystemSet/Stage/Schedule 与冲突分析。

use crate::query::AccessSet;
use crate::World;
use rf_task::JobSystem;
use std::any::TypeId;

/// 系统（IF-085）。
pub trait System: Send + Sync {
    fn name(&self) -> &str;
    fn run(&mut self, world: &mut World);
    /// 声明的读写集（调度器冲突分析依据）。
    fn access(&self) -> AccessSet {
        AccessSet::default()
    }
}

/// 闭包系统。
pub struct FnSystem<F: Fn(&mut World) + Send + Sync> {
    sys_name: String,
    f: F,
    acc: AccessSet,
}

impl<F: Fn(&mut World) + Send + Sync> System for FnSystem<F> {
    fn name(&self) -> &str {
        &self.sys_name
    }

    fn run(&mut self, world: &mut World) {
        (self.f)(world);
    }

    fn access(&self) -> AccessSet {
        self.acc.clone()
    }
}

impl<F: Fn(&mut World) + Send + Sync> FnSystem<F> {
    /// 追加读访问声明。
    pub fn reads<C: crate::Component>(mut self) -> Self {
        self.acc.read(TypeId::of::<C>());
        self
    }

    /// 追加写访问声明。
    pub fn writes<C: crate::Component>(mut self) -> Self {
        self.acc.write(TypeId::of::<C>());
        self
    }
}

/// 构造闭包系统（IF-085）。
pub fn system<F: Fn(&mut World) + Send + Sync + 'static>(name: &str, f: F) -> FnSystem<F> {
    FnSystem { sys_name: name.to_string(), f, acc: AccessSet::default() }
}

/// 系统集合（IF-86）。
pub struct SystemSet {
    pub name: String,
    pub parallel: bool,
    pub(crate) systems: Vec<Box<dyn System>>,
}

impl SystemSet {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), parallel: false, systems: Vec::new() }
    }

    pub fn parallel(mut self) -> Self {
        self.parallel = true;
        self
    }

    /// 追加系统（builder 风格）。
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, sys: Box<dyn System>) -> Self {
        self.systems.push(sys);
        self
    }

    /// 冲突分析专用的直接追加（测试）。
    #[allow(dead_code)]
    pub(crate) fn push_raw(&mut self, sys: Box<dyn System>) {
        self.systems.push(sys);
    }

    /// 冲突分组（贪心）：返回若干批，批内系统两两无冲突（可并行），批间有冲突。
    pub fn conflict_batches(&self) -> Vec<Vec<usize>> {
        let mut batches: Vec<(Vec<usize>, AccessSet)> = Vec::new();
        for (i, sys) in self.systems.iter().enumerate() {
            let acc = sys.access();
            let mut placed = false;
            for (batch, acc_sum) in &mut batches {
                if !acc_sum.conflicts(&acc) {
                    batch.push(i);
                    for t in acc.reads() {
                        acc_sum.read(*t);
                    }
                    for t in acc.writes() {
                        acc_sum.write(*t);
                    }
                    placed = true;
                    break;
                }
            }
            if !placed {
                let mut acc_sum = AccessSet::default();
                for t in acc.reads() {
                    acc_sum.read(*t);
                }
                for t in acc.writes() {
                    acc_sum.write(*t);
                }
                batches.push((vec![i], acc_sum));
            }
        }
        batches.into_iter().map(|(b, _)| b).collect()
    }
}

/// 阶段运行条件。
pub type RunCondition = Box<dyn Fn(&World) -> bool + Send + Sync>;

/// 阶段（IF-86）。
pub struct Stage {
    pub name: String,
    pub sets: Vec<SystemSet>,
    pub(crate) run_if: Option<RunCondition>,
}

impl Stage {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), sets: Vec::new(), run_if: None }
    }

    pub fn with_set(mut self, set: SystemSet) -> Self {
        self.sets.push(set);
        self
    }

    pub fn run_if<F: Fn(&World) -> bool + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.run_if = Some(Box::new(f));
        self
    }
}

/// 调度报告。
pub struct ScheduleReport {
    pub stages_run: Vec<String>,
    pub systems_run: usize,
    /// parallel 声明的系统分组（批内可并行）。
    pub parallel_batches: Vec<Vec<String>>,
}

/// 调度器（IF-86）。
pub struct Schedule {
    pub stages: Vec<Stage>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: Stage) -> &mut Self {
        self.stages.push(stage);
        self
    }

    /// 便捷：向指定阶段（不存在则创建）追加系统。
    pub fn add_system(
        &mut self,
        stage: &str,
        set: Option<&str>,
        sys: Box<dyn System>,
    ) -> &mut Self {
        let s = match self.stages.iter_mut().find(|s| s.name == stage) {
            Some(s) => s,
            None => {
                self.stages.push(Stage::new(stage));
                self.stages.last_mut().unwrap()
            }
        };
        let set_name = set.unwrap_or("default");
        match s.sets.iter_mut().find(|ss| ss.name == set_name) {
            Some(ss) => ss.systems.push(sys),
            None => s.sets.push(SystemSet::new(set_name).add(sys)),
        }
        self
    }

    /// 执行调度。提供 JobSystem 且集合声明 parallel=true 时，
    /// 冲突分析得到批内可并行分组（MVP：分组结果记录于报告，执行按声明顺序串行
    /// 以保证 &mut World 独占安全；真实并行执行见 run_threaded 合同说明）。
    pub fn run(&mut self, world: &mut World, jobs: Option<&JobSystem>) -> ScheduleReport {
        let _ = jobs;
        let mut report =
            ScheduleReport { stages_run: Vec::new(), systems_run: 0, parallel_batches: Vec::new() };
        for stage in &mut self.stages {
            if let Some(cond) = &stage.run_if {
                if !cond(world) {
                    continue;
                }
            }
            report.stages_run.push(stage.name.clone());
            for set in &mut stage.sets {
                if set.parallel {
                    let batches = set.conflict_batches();
                    for b in &batches {
                        report
                            .parallel_batches
                            .push(b.iter().map(|&i| set.systems[i].name().to_string()).collect());
                    }
                }
                for sys in &mut set.systems {
                    sys.run(world);
                    report.systems_run += 1;
                }
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;

    #[derive(Component)]
    struct A(#[allow(dead_code)] i32);

    #[derive(Component)]
    struct B(#[allow(dead_code)] i32);

    #[test]
    fn conflict_analysis() {
        let mut set = SystemSet::new("s");
        let mut s1 = system("read_a", |_: &mut World| {});
        s1 = s1.reads::<A>();
        let mut s2 = system("read_a2", |_: &mut World| {});
        s2 = s2.reads::<A>();
        let mut s3 = system("write_a", |_: &mut World| {});
        s3 = s3.writes::<A>();
        // s1 与 s2 无冲突 → 同批；s3 与两者冲突 → 新批
        let sys1: Box<dyn System> = Box::new(s1);
        let sys2: Box<dyn System> = Box::new(s2);
        let sys3: Box<dyn System> = Box::new(s3);
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        let _ = world;
        set.push_raw(sys1);
        set.push_raw(sys2);
        set.push_raw(sys3);
        let batches = set.conflict_batches();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1], vec![2]);
    }

    #[derive(Default)]
    struct Log(Vec<String>);
    impl crate::Resource for Log {}

    #[derive(Default)]
    struct Counter(i32);
    impl crate::Resource for Counter {}

    #[test]
    fn schedule_runs_stages_in_order() {
        let mut world = World::new();
        world.insert_resource(Log::default());
        let mut sched = Schedule::new();
        let log = |tag: &'static str| {
            move |w: &mut World| {
                w.resource_mut::<Log>().unwrap().0.push(tag.to_string());
            }
        };
        sched.add_system("pre", None, Box::new(system("p1", log("pre"))));
        sched.add_system("main", None, Box::new(system("m1", log("main"))));
        sched.add_system("post", None, Box::new(system("q1", log("post"))));
        let report = sched.run(&mut world, None);
        assert_eq!(report.stages_run, vec!["pre", "main", "post"]);
        assert_eq!(report.systems_run, 3);
        assert_eq!(world.resource::<Log>().unwrap().0, vec!["pre", "main", "post"]);
    }

    #[test]
    fn run_if_skips_stage() {
        let mut world = World::new();
        world.insert_resource(Counter(0));
        let mut sched = Schedule::new();
        sched.add_stage(Stage::new("always").with_set(SystemSet::new("s").add(Box::new(system(
            "inc",
            |w: &mut World| {
                w.resource_mut::<Counter>().unwrap().0 += 1;
            },
        )))));
        sched.add_stage(
            Stage::new("conditional")
                .with_set(SystemSet::new("s").add(Box::new(system("double", |w: &mut World| {
                    w.resource_mut::<Counter>().unwrap().0 *= 100;
                }))))
                .run_if(|w| w.resource::<Counter>().unwrap().0 > 10),
        );
        let report = sched.run(&mut world, None);
        assert_eq!(report.stages_run.len(), 1); // conditional 未满足
        assert_eq!(world.resource::<Counter>().unwrap().0, 1);
    }
}
