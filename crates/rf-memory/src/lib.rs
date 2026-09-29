//! RustForge 内存层（IF-040 ~ IF-045）：分配器、追踪、泄漏检测。

pub mod tracking;

pub use tracking::{leak_report, AllocReport, AllocSite, TrackingGlobal};

use std::ptr::NonNull;

/// 线性（Bump）分配器：批量 reset，帧临时数据。
/// 不运行元素 Drop（约定仅存 POD/由所有者手动处理），reset 语义见文档。
#[derive(Debug)]
pub struct BumpArena {
    data: Vec<u8>,
    offset: usize,
}

impl BumpArena {
    pub fn with_capacity(bytes: usize) -> Self {
        Self { data: vec![0u8; bytes], offset: 0 }
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    pub fn used(&self) -> usize {
        self.offset
    }

    fn align_up(&self, ptr: usize, align: usize) -> usize {
        (ptr + align - 1) & !(align - 1)
    }

    /// 分配并写入 T，返回独占引用；耗尽返回 None。
    pub fn alloc<T>(&mut self, value: T) -> Option<&mut T> {
        let start = self.align_up(self.offset, std::mem::align_of::<T>());
        let end = start.checked_add(std::mem::size_of::<T>())?;
        if end > self.data.len() {
            return None;
        }
        self.offset = end;
        // SAFETY: start..end 位于 data 内且按 align_of::<T> 对齐。
        let ptr = unsafe { self.data.as_mut_ptr().add(start) as *mut T };
        unsafe { ptr.write(value) };
        // SAFETY: 指针来自本轮写入，未别名。
        Some(unsafe { &mut *ptr })
    }

    /// 分配切片拷贝。
    pub fn alloc_slice<T: Copy>(&mut self, values: &[T]) -> Option<&mut [T]> {
        let start = self.align_up(self.offset, std::mem::align_of::<T>());
        let bytes = std::mem::size_of_val(values);
        let end = start.checked_add(bytes)?;
        if end > self.data.len() {
            return None;
        }
        self.offset = end;
        // SAFETY: 同上，拷贝入已对齐区间。
        unsafe {
            std::ptr::copy_nonoverlapping(
                values.as_ptr(),
                self.data.as_mut_ptr().add(start) as *mut T,
                values.len(),
            );
            Some(std::slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(start) as *mut T,
                values.len(),
            ))
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

/// 索引句柄对象池：O(1) 分配/释放。
#[derive(Debug)]
pub struct Pool<T> {
    slots: Vec<Option<T>>,
    free: Vec<u32>,
}

impl<T> Pool<T> {
    pub fn with_capacity(cap: usize) -> Self {
        Self { slots: Vec::with_capacity(cap), free: Vec::new() }
    }

    pub fn alloc(&mut self, value: T) -> u32 {
        match self.free.pop() {
            Some(i) => {
                self.slots[i as usize] = Some(value);
                i
            }
            None => {
                self.slots.push(Some(value));
                (self.slots.len() - 1) as u32
            }
        }
    }

    pub fn free(&mut self, index: u32) -> Option<T> {
        let slot = self.slots.get_mut(index as usize)?;
        let value = slot.take();
        if value.is_some() {
            self.free.push(index);
        }
        value
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        self.slots.get(index as usize).and_then(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.slots.get_mut(index as usize).and_then(|s| s.as_mut())
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| s.as_ref().map(|v| (i as u32, v)))
    }
}

/// LIFO 栈分配器：标记-回滚。
#[derive(Debug)]
pub struct StackAllocator {
    data: Vec<u8>,
    top: usize,
}

impl StackAllocator {
    pub fn with_capacity(bytes: usize) -> Self {
        Self { data: vec![0u8; bytes], top: 0 }
    }

    pub fn used(&self) -> usize {
        self.top
    }

    /// 按字节大小+对齐分配裸内存；耗尽返回 None。
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let start = (self.top + align - 1) & !(align - 1);
        let end = start.checked_add(size)?;
        if end > self.data.len() {
            return None;
        }
        self.top = end;
        // SAFETY: start..end 在 Vec 缓冲区内，非零。
        Some(unsafe { NonNull::new_unchecked(self.data.as_mut_ptr().add(start)) })
    }

    pub fn mark(&self) -> usize {
        self.top
    }

    /// 回滚标记（mark 必须 ≤ 当前 top 且来自本实例）。
    pub fn rewind(&mut self, mark: usize) {
        debug_assert!(mark <= self.top);
        if mark <= self.top {
            self.top = mark;
        }
    }
}

/// Buddy 分配器：2 的幂块分裂/合并。
#[derive(Debug)]
pub struct BuddyAllocator {
    data: Box<[u8]>,
    total: usize,
    min_block: usize,
    /// 每级空闲块起始偏移列表（级别 0 = total 大小）。
    free_lists: Vec<Vec<usize>>,
    /// 已分配块 (offset → level)。
    allocated: std::collections::HashMap<usize, usize>,
}

impl BuddyAllocator {
    /// total 与 min_block 自动向上取整为 2 的幂（total 不小于 min_block）。
    pub fn new(total: usize, min_block: usize) -> Self {
        let total = total.max(1).next_power_of_two();
        let min_block = min_block.max(1).next_power_of_two().min(total);
        let levels = (total / min_block).trailing_zeros() as usize + 1;
        let mut free_lists = vec![Vec::new(); levels];
        free_lists[0].push(0);
        Self {
            data: vec![0u8; total].into_boxed_slice(),
            total,
            min_block,
            free_lists,
            allocated: Default::default(),
        }
    }

    fn level_size(&self, level: usize) -> usize {
        self.total >> level
    }

    pub fn alloc(&mut self, size: usize) -> Option<NonNull<u8>> {
        let need = size.max(1).next_power_of_two().max(self.min_block);
        if need > self.total {
            return None;
        }
        let want_level = (self.total / need).trailing_zeros() as usize;
        // 从 want_level 向更高级别（更大块，级别号更小）找可用块
        let mut found = None;
        for lvl in (0..=want_level).rev() {
            if !self.free_lists[lvl].is_empty() {
                found = Some(lvl);
                break;
            }
        }
        let level = found?; // OOM
        let offset = self.free_lists[level].pop().unwrap();
        // 逐级分裂下探：块一分为二，伙伴入空闲表，保留左半继续分
        let mut cur_lvl = level;
        while cur_lvl < want_level {
            cur_lvl += 1;
            let buddy = offset + self.level_size(cur_lvl);
            self.free_lists[cur_lvl].push(buddy);
        }
        self.allocated.insert(offset, want_level);
        // SAFETY: 块起始对齐且 offset+need ≤ total（分配器不变量）。
        Some(unsafe { NonNull::new_unchecked(self.data.as_mut_ptr().add(offset)) })
    }

    pub fn free(&mut self, ptr: NonNull<u8>) {
        // SAFETY: ptr 必须来自本实例的 alloc（调用方契约）。
        let offset = (ptr.as_ptr() as usize).wrapping_sub(self.data.as_ptr() as usize);
        let Some(level) = self.allocated.remove(&offset) else { return };
        let mut cur = offset;
        let mut lvl = level;
        // 与伙伴合并上行
        while lvl > 0 {
            let size = self.level_size(lvl);
            let buddy = cur ^ size;
            if let Some(pos) = self.free_lists[lvl].iter().position(|&o| o == buddy) {
                self.free_lists[lvl].swap_remove(pos);
                cur = cur.min(buddy);
                lvl -= 1;
            } else {
                break;
            }
        }
        self.free_lists[lvl].push(cur);
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let free_bytes: usize = self
            .free_lists
            .iter()
            .enumerate()
            .map(|(lvl, list)| list.len() * self.level_size(lvl))
            .sum();
        (self.total, free_bytes, self.allocated.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_arena_basic() {
        let mut arena = BumpArena::with_capacity(1024);
        let a = arena.alloc(42u64).unwrap();
        assert_eq!(*a, 42);
        let _ = arena.alloc([1u8; 8]).unwrap();
        assert_eq!(arena.used(), 16);
        arena.reset();
        assert_eq!(arena.used(), 0);
        let mut small = BumpArena::with_capacity(4);
        assert!(small.alloc(999u64).is_none()); // 耗尽
    }

    #[test]
    fn pool_roundtrip() {
        let mut p = Pool::with_capacity(4);
        let a = p.alloc("a");
        let b = p.alloc("b");
        assert_eq!(p.len(), 2);
        assert_eq!(p.free(a), Some("a"));
        assert_eq!(p.get(a), None);
        let c = p.alloc("c"); // 复用槽 a
        assert_eq!(c, a);
        assert_eq!(p.get(b), Some(&"b"));
        assert_eq!(p.iter().count(), 2);
        assert!(p.free(99).is_none());
    }

    #[test]
    fn stack_mark_rewind() {
        let mut s = StackAllocator::with_capacity(256);
        let m1 = s.mark();
        let p1 = s.alloc(64, 16).unwrap();
        assert_eq!(p1.as_ptr() as usize % 16, 0);
        let m2 = s.mark();
        let _ = s.alloc(128, 8).unwrap();
        assert_eq!(s.used(), 192);
        s.rewind(m2);
        assert_eq!(s.used(), 64);
        s.rewind(m1);
        assert_eq!(s.used(), 0);
    }

    #[test]
    fn buddy_split_merge() {
        let mut b = BuddyAllocator::new(1024, 16);
        let p1 = b.alloc(100).unwrap();
        let p2 = b.alloc(100).unwrap();
        assert_ne!(p1.as_ptr(), p2.as_ptr());
        let (total, free_before, count) = b.stats();
        assert_eq!((total, count), (1024, 2));
        b.free(p1);
        b.free(p2);
        let (_, free_after, count) = b.stats();
        assert_eq!(count, 0);
        assert!(free_after > free_before);
        assert_eq!(b.free_lists[0].len(), 1); // 完全合并回顶级单块
    }

    #[test]
    fn buddy_oom() {
        let mut b = BuddyAllocator::new(64, 16);
        let p = b.alloc(64).unwrap();
        assert!(b.alloc(16).is_none());
        b.free(p);
        assert!(b.alloc(64).is_some());
    }
}
