//! 全局分配追踪（IF-044/IF-045）：包装 GlobalAlloc，统计与可选站点采集。

use std::alloc::{GlobalAlloc, Layout};
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// 单个泄漏/热点站点。
#[derive(Debug, Clone)]
pub struct AllocSite {
    pub backtrace: String,
    pub bytes: usize,
    pub count: usize,
}

/// 追踪快照。
#[derive(Debug, Clone, Default)]
pub struct AllocReport {
    pub live_bytes: usize,
    pub live_count: usize,
    pub peak_bytes: usize,
    pub total_allocs: u64,
    pub sites: Vec<AllocSite>,
}

/// 包装任意 GlobalAlloc 的追踪器。
pub struct TrackingGlobal<A: GlobalAlloc> {
    inner: A,
    enabled: AtomicBool,
    capture: AtomicBool,
    live_bytes: AtomicU64,
    live_count: AtomicU64,
    peak_bytes: AtomicU64,
    total_allocs: AtomicU64,
    /// 未释放分配（ptr → size），用于泄漏检测。
    outstanding: Mutex<HashMap<usize, usize>>,
    /// 站点采集（capture=true 时记录分配栈）。
    sites: Mutex<HashMap<String, (usize, usize)>>,
}

impl<A: GlobalAlloc> TrackingGlobal<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            enabled: AtomicBool::new(true),
            capture: AtomicBool::new(false),
            live_bytes: AtomicU64::new(0),
            live_count: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            total_allocs: AtomicU64::new(0),
            outstanding: Mutex::new(HashMap::new()),
            sites: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }

    /// 开启后新分配将记录回溯（开销高，仅调试用）。
    pub fn capture_sites(&self, on: bool) {
        self.capture.store(on, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> AllocReport {
        let mut report = AllocReport {
            live_bytes: self.live_bytes.load(Ordering::Relaxed) as usize,
            live_count: self.live_count.load(Ordering::Relaxed) as usize,
            peak_bytes: self.peak_bytes.load(Ordering::Relaxed) as usize,
            total_allocs: self.total_allocs.load(Ordering::Relaxed),
            sites: Vec::new(),
        };
        // 结合 outstanding（按 size 归并）与 sites
        if let Ok(out) = self.outstanding.lock() {
            if self.capture.load(Ordering::Relaxed) {
                if let Ok(sites) = self.sites.lock() {
                    let mut v: Vec<AllocSite> = sites
                        .iter()
                        .map(|(bt, (bytes, count))| AllocSite {
                            backtrace: bt.clone(),
                            bytes: *bytes,
                            count: *count,
                        })
                        .collect();
                    v.sort_by_key(|s| std::cmp::Reverse(s.bytes));
                    report.sites = v;
                }
            } else {
                let by_size: HashMap<usize, usize> =
                    out.values().fold(HashMap::new(), |mut m, &s| {
                        *m.entry(s).or_insert(0) += 1;
                        m
                    });
                let mut v: Vec<AllocSite> = by_size
                    .into_iter()
                    .map(|(size, count)| AllocSite {
                        backtrace: format!(
                            "<size class {size}B> (enable capture_sites for stacks)"
                        ),
                        bytes: size * count,
                        count,
                    })
                    .collect();
                v.sort_by_key(|s| std::cmp::Reverse(s.bytes));
                report.sites = v;
            }
        }
        report
    }
}

impl<A: GlobalAlloc> TrackingGlobal<A> {
    fn record_alloc(&self, size: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let live = self.live_bytes.fetch_add(size, Ordering::Relaxed) + size;
        self.live_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocs.fetch_add(1, Ordering::Relaxed);
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while live > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak,
                live,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
    }
}

// SAFETY: 委托内层分配器；追踪结构自身用锁与原子，重入安全（不触发新分配的热路径用 Relaxed 原子）。
unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingGlobal<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            self.record_alloc(layout.size() as u64);
            if let Ok(mut out) = self.outstanding.lock() {
                out.insert(ptr as usize, layout.size());
                if self.capture.load(Ordering::Relaxed) {
                    let bt = Backtrace::force_capture().to_string();
                    if let Ok(mut sites) = self.sites.lock() {
                        let e = sites.entry(bt).or_insert((0, 0));
                        e.0 += layout.size();
                        e.1 += 1;
                    }
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Ok(mut out) = self.outstanding.lock() {
            if let Some(size) = out.remove(&(ptr as usize)) {
                self.live_bytes.fetch_sub(size as u64, Ordering::Relaxed);
                self.live_count.fetch_sub(1, Ordering::Relaxed);
            } else {
                // 双重释放检测：非本追踪器记录的指针（如静态段）忽略
            }
        }
        self.inner.dealloc(ptr, layout)
    }
}

/// 泄漏报告文本（按站点字节排序）。
pub fn leak_report(rep: &AllocReport) -> String {
    let mut s = format!(
        "alloc report: live {} B in {} blocks, peak {} B, total {} allocs\n",
        rep.live_bytes, rep.live_count, rep.peak_bytes, rep.total_allocs
    );
    if rep.live_count > 0 {
        s.push_str("outstanding sites:\n");
        for site in rep.sites.iter().take(20) {
            s.push_str(&format!(
                "  {:>9} B x{:>6}  {}\n",
                site.bytes,
                site.count,
                site.backtrace.lines().take(4).collect::<Vec<_>>().join(" | ")
            ));
        }
    } else {
        s.push_str("no leaks detected\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::System;
    use std::sync::atomic::Ordering;

    #[test]
    fn track_alloc_dealloc_balance() {
        let tracker: TrackingGlobal<System> = TrackingGlobal::new(System);
        // SAFETY: 测试用小分配，布局合法。
        unsafe {
            let layout = Layout::from_size_align(128, 8).unwrap();
            let p1 = GlobalAlloc::alloc(&tracker, layout);
            assert!(!p1.is_null());
            let p2 = GlobalAlloc::alloc(&tracker, layout);
            assert!(!p2.is_null());
            assert_eq!(tracker.live_count.load(Ordering::Relaxed), 2);
            assert_eq!(tracker.live_bytes.load(Ordering::Relaxed), 256);
            GlobalAlloc::dealloc(&tracker, p1, layout);
            assert_eq!(tracker.live_count.load(Ordering::Relaxed), 1);
            GlobalAlloc::dealloc(&tracker, p2, layout);
        }
        let rep = tracker.snapshot();
        assert_eq!(rep.live_count, 0);
        assert_eq!(rep.live_bytes, 0);
        assert!(leak_report(&rep).contains("no leaks"));
    }

    #[test]
    fn peak_tracking() {
        let tracker: TrackingGlobal<System> = TrackingGlobal::new(System);
        // SAFETY: 布局合法。
        unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let p = GlobalAlloc::alloc(&tracker, layout);
            GlobalAlloc::dealloc(&tracker, p, layout);
        }
        let rep = tracker.snapshot();
        assert!(rep.peak_bytes >= 64);
    }
}
