//! 类型擦除的 SoA 组件列 + Archetype（内部结构，crate 私有细节经 World 暴露只读统计）。

use rf_core::Tick;
use std::alloc::Layout;
use std::any::TypeId;

/// 组件标记（IF-070）。要求 Send+Sync 以便跨线程调度。
pub trait Component: Send + Sync + 'static {}

/// 资源标记（IF-070）。
pub trait Resource: Send + Sync + 'static {}

/// 类型擦除列：裸内存 + drop 函数 + 变更 tick。
pub struct Column {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    elem_layout: Layout,
    /// 分配布局（size = cap*elem，对齐取 elem 对齐与 16 的最大值，保持幂次）。
    alloc_layout: Layout,
    len: usize,
    cap: usize,
    data: *mut u8,
    drop_fn: unsafe fn(*mut u8, usize),
    pub(crate) added_ticks: Vec<Tick>,
    pub(crate) changed_ticks: Vec<Tick>,
}

// SAFETY: Column 的所有访问都经由 World 的借用纪律（调度器读写集分析）；
// 组件要求 Send+Sync，数据本身可跨线程读取。
unsafe impl Send for Column {}
unsafe impl Sync for Column {}

/// SAFETY: ptr 指向 len 个构造完成的 T。
unsafe fn drop_array<T>(ptr: *mut u8, len: usize) {
    // SAFETY: 见函数级契约。
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut T, len) };
    for item in slice.iter_mut() {
        // SAFETY: 元素构造完成且尚未 drop。
        unsafe { std::ptr::drop_in_place(item as *mut T) };
    }
}

impl Column {
    pub fn new<T: Component>() -> Self {
        let elem = Layout::new::<T>();
        let align = elem.align().max(16).next_power_of_two();
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            elem_layout: elem,
            alloc_layout: Layout::from_size_align(0, align).expect("column alloc layout"),
            len: 0,
            cap: 0,
            data: std::ptr::null_mut(),
            drop_fn: drop_array::<T>,
            added_ticks: Vec::new(),
            changed_ticks: Vec::new(),
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
        let elem = self.elem_layout.size().max(1);
        let new_size = new_cap * elem;
        let new_layout = Layout::from_size_align(
            new_size.max(self.alloc_layout.align()),
            self.alloc_layout.align(),
        )
        .expect("column grow layout");
        // SAFETY: 全新分配后按元素拷贝（元素是可移动位拷贝），再释放旧块。
        let new_data = unsafe { std::alloc::alloc(new_layout) };
        assert!(!new_data.is_null(), "column OOM");
        if self.len > 0 {
            // SAFETY: 旧块有 len 个元素。
            unsafe { std::ptr::copy_nonoverlapping(self.data, new_data, self.len * elem) };
        }
        if !self.data.is_null() && self.alloc_layout.size() > 0 {
            // SAFETY: 旧块按旧布局释放。
            unsafe { std::alloc::dealloc(self.data, self.alloc_layout) };
        }
        self.data = new_data;
        self.cap = new_cap;
        self.alloc_layout = new_layout;
    }

    /// 拷贝写入一个元素（value 指向 T 的位）。
    /// SAFETY: value 必须指向合法 T；调用后原值仍由调用方拥有。
    pub(crate) unsafe fn push_raw(&mut self, value: *const u8, added: Tick, changed: Tick) {
        if self.len == self.cap {
            self.grow();
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                value,
                self.data.add(self.len * self.elem_layout.size()),
                self.elem_layout.size(),
            )
        };
        self.added_ticks.push(added);
        self.changed_ticks.push(changed);
        self.len += 1;
    }

    pub(crate) fn row_ptr(&self, row: usize) -> *const u8 {
        debug_assert!(row < self.len);
        // SAFETY: row < len 由调用方/迭代器保证。
        unsafe { self.data.add(row * self.elem_layout.size()) }
    }

    /// SAFETY: 调用者必须保证无别名引用存活。
    pub(crate) unsafe fn row_ptr_mut(&mut self, row: usize) -> *mut u8 {
        debug_assert!(row < self.len);
        unsafe { self.data.add(row * self.elem_layout.size()) }
    }

    /// 取出某行字节（拷贝），并 swap_remove 尾部行到该行。
    /// 返回 (bytes, moved_from_last)。
    pub(crate) fn swap_take(&mut self, row: usize) -> (Vec<u8>, bool) {
        debug_assert!(row < self.len);
        let size = self.elem_layout.size();
        // SAFETY: row 合法。
        let mut bytes = vec![0u8; size];
        unsafe { std::ptr::copy_nonoverlapping(self.row_ptr(row), bytes.as_mut_ptr(), size) };
        let last = self.len - 1;
        let moved = row != last;
        if moved {
            // SAFETY: 搬移 last → row（row != last 无重叠问题）。
            unsafe { std::ptr::copy(self.row_ptr(last), self.data.add(row * size), size) };
            self.added_ticks.swap(row, last);
            self.changed_ticks.swap(row, last);
        }
        self.len -= 1;
        self.added_ticks.truncate(self.len);
        self.changed_ticks.truncate(self.len);
        (bytes, moved)
    }

    /// 标记 row 变更。
    pub(crate) fn touch(&mut self, row: usize, tick: Tick) {
        if let Some(t) = self.changed_ticks.get_mut(row) {
            *t = tick;
        }
    }

    /// 清空（drop 元素）。
    pub(crate) fn clear(&mut self) {
        if self.len > 0 && !self.data.is_null() {
            // SAFETY: data 指向 len 个构造完成的元素。
            unsafe { (self.drop_fn)(self.data, self.len) };
        }
        self.len = 0;
        self.added_ticks.clear();
        self.changed_ticks.clear();
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        self.clear();
        if !self.data.is_null() && self.alloc_layout.size() > 0 {
            // SAFETY: data 由 grow 分配，alloc_layout 与之一致。
            unsafe { std::alloc::dealloc(self.data, self.alloc_layout) };
        }
    }
}

/// Archetype：一组组件类型集合 + 列式存储 + 实体行表。
pub struct Archetype {
    /// 按 TypeId 升序排列（作为 HashMap 键）。
    pub(crate) signature: Vec<TypeId>,
    pub(crate) columns: Vec<Column>,
    /// 行 → 实体。
    pub(crate) entities: Vec<rf_core::Entity>,
}

impl Archetype {
    pub fn empty() -> Self {
        Self { signature: Vec::new(), columns: Vec::new(), entities: Vec::new() }
    }

    pub fn from_signature(sig: Vec<(TypeId, &'static str)>) -> Self {
        let mut sig = sig;
        sig.sort_by_key(|(id, _)| *id);
        Self {
            signature: sig.iter().map(|(i, _)| *i).collect(),
            columns: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn signature(&self) -> &[TypeId] {
        &self.signature
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub(crate) fn column_index(&self, id: TypeId) -> Option<usize> {
        self.signature.binary_search(&id).ok()
    }

    /// 读取某行某组件的引用。
    /// SAFETY: row < len；T 与列类型一致；无并存可变引用。
    pub(crate) unsafe fn get<T: Component>(&self, row: usize) -> Option<&T> {
        let idx = self.column_index(TypeId::of::<T>())?;
        let col = &self.columns[idx];
        Some(unsafe { &*(col.row_ptr(row) as *const T) })
    }

    /// SAFETY: row < len；无并存引用。
    pub(crate) unsafe fn get_mut<T: Component>(&mut self, row: usize) -> Option<&mut T> {
        let idx = self.column_index(TypeId::of::<T>())?;
        let col = &mut self.columns[idx];
        Some(unsafe { &mut *(col.row_ptr_mut(row) as *mut T) })
    }

    /// 行字节拷贝（跨 archetype 迁移用）。
    pub(crate) fn extract_row_bytes(&self, row: usize) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(self.columns.len());
        for col in &self.columns {
            out.push(
                unsafe { std::slice::from_raw_parts(col.row_ptr(row), col.layout_size()) }.to_vec(),
            );
        }
        out
    }

    /// 空行占位（bytes 版本的写入接口在 bundle attach 时使用）。
    pub(crate) fn push_entity(&mut self, entity: rf_core::Entity) -> usize {
        self.entities.push(entity);
        self.entities.len() - 1
    }
}

impl Column {
    pub(crate) fn layout_size(&self) -> usize {
        self.elem_layout.size()
    }
}
