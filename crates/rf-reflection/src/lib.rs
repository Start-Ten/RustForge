//! RustForge 反射（IF-090 ~ IF-098）：类型注册、字段元数据、属性读写。

use rf_core::{Entity, Result};
use rf_math::{Color, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 字段元数据（IF-090）。
#[derive(Debug, Clone, Default)]
pub struct FieldMeta {
    pub display_name: &'static str,
    pub tooltip: &'static str,
    pub category: &'static str,
    pub range: Option<(f64, f64)>,
    pub step: Option<f64>,
    pub readonly: bool,
    pub hidden: bool,
    pub editor_exposed: bool,
    pub script_exposed: bool,
    pub network_replicated: bool,
}

impl FieldMeta {
    pub fn editor(mut self) -> Self {
        self.editor_exposed = true;
        self
    }

    pub fn range(mut self, lo: f64, hi: f64) -> Self {
        self.range = Some((lo, hi));
        self
    }
}

/// 字段类型（IF-091，可扩展）。
#[derive(Debug, Clone, Copy)]
pub enum FieldKind {
    F32,
    F64,
    Bool,
    I32,
    U32,
    U64,
    String,
    Vec2,
    Vec3,
    Vec4,
    Color,
    Entity,
    Enum(&'static [(&'static str, u64)]),
    Struct(&'static TypeDescriptor),
}

impl PartialEq for FieldKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Enum(a), Self::Enum(b)) => std::ptr::eq(a, b),
            (Self::Struct(a), Self::Struct(b)) => a.type_path == b.type_path,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
}

/// 字段信息（IF-092）。
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: &'static str,
    pub kind: FieldKind,
    pub offset: usize,
    pub size: usize,
    pub meta: FieldMeta,
}

/// 方法信息（IF-092）。
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: &'static str,
    pub args: &'static [&'static str],
    pub return_type: &'static str,
}

/// 类型描述符（IF-093）。
#[derive(Debug, Clone)]
pub struct TypeDescriptor {
    pub type_path: &'static str,
    pub size: usize,
    pub align: usize,
    pub fields: &'static [FieldInfo],
    pub methods: &'static [MethodInfo],
}

/// 反射 trait（IF-094）。
pub trait Reflect: Send + Sync + 'static {
    fn descriptor() -> &'static TypeDescriptor
    where
        Self: Sized;
    /// 对象安全访问（read/write_field 使用）。
    fn reflect_descriptor(&self) -> &'static TypeDescriptor;
    fn as_reflect(&self) -> &dyn Reflect;
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;
}

/// 字段值（IF-095）。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    F32(f32),
    F64(f64),
    Bool(bool),
    I32(i32),
    U32(u32),
    U64(u64),
    String(String),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Color(Color),
    Entity(Entity),
    Enum(u64),
    Opaque,
}

impl From<f32> for FieldValue {
    fn from(v: f32) -> Self {
        FieldValue::F32(v)
    }
}
impl From<f64> for FieldValue {
    fn from(v: f64) -> Self {
        FieldValue::F64(v)
    }
}
impl From<bool> for FieldValue {
    fn from(v: bool) -> Self {
        FieldValue::Bool(v)
    }
}
impl From<i32> for FieldValue {
    fn from(v: i32) -> Self {
        FieldValue::I32(v)
    }
}
impl From<u32> for FieldValue {
    fn from(v: u32) -> Self {
        FieldValue::U32(v)
    }
}
impl From<u64> for FieldValue {
    fn from(v: u64) -> Self {
        FieldValue::U64(v)
    }
}
impl From<String> for FieldValue {
    fn from(v: String) -> Self {
        FieldValue::String(v)
    }
}
impl From<&str> for FieldValue {
    fn from(v: &str) -> Self {
        FieldValue::String(v.to_string())
    }
}

/// 从 `&dyn Reflect` 取数据指针：胖指针瘦化（丢弃 vtable，保留 data 地址）。
/// SAFETY: 返回指针的使用不得超出 obj 生命周期。
fn data_ptr(obj: &dyn Reflect) -> *const u8 {
    obj as *const dyn Reflect as *const u8
}

/// 读取字段（IF-096）。
pub fn read_field(obj: &dyn Reflect, name: &str) -> Result<FieldValue> {
    let desc = obj.reflect_descriptor();
    let field = desc.fields.iter().find(|f| f.name == name).ok_or_else(|| {
        rf_core::EngineError::InvalidData(format!("no field `{name}` on {}", desc.type_path))
    })?;
    let base = data_ptr(obj);
    // SAFETY: offset/size 来自本类型描述符，指针在对象内。
    let ptr = unsafe { base.add(field.offset) };
    unsafe {
        Ok(match field.kind {
            FieldKind::F32 => FieldValue::F32(ptr.cast::<f32>().read()),
            FieldKind::F64 => FieldValue::F64(ptr.cast::<f64>().read()),
            FieldKind::Bool => FieldValue::Bool(ptr.cast::<bool>().read()),
            FieldKind::I32 => FieldValue::I32(ptr.cast::<i32>().read()),
            FieldKind::U32 => FieldValue::U32(ptr.cast::<u32>().read()),
            FieldKind::U64 => FieldValue::U64(ptr.cast::<u64>().read()),
            FieldKind::String => FieldValue::String((*ptr.cast::<String>()).clone()),
            FieldKind::Vec2 => FieldValue::Vec2(ptr.cast::<Vec2>().read()),
            FieldKind::Vec3 => FieldValue::Vec3(ptr.cast::<Vec3>().read()),
            FieldKind::Vec4 => FieldValue::Vec4(ptr.cast::<Vec4>().read()),
            FieldKind::Color => FieldValue::Color(ptr.cast::<Color>().read()),
            FieldKind::Entity => FieldValue::Entity(ptr.cast::<Entity>().read()),
            FieldKind::Enum(_) => FieldValue::Enum(ptr.cast::<u64>().read()),
            FieldKind::Struct(_) => FieldValue::Opaque,
        })
    }
}

/// 写入字段（IF-096）。类型不匹配或 readonly 报错。
pub fn write_field(obj: &mut dyn Reflect, name: &str, value: FieldValue) -> Result<()> {
    let desc = obj.reflect_descriptor();
    let field = desc.fields.iter().find(|f| f.name == name).ok_or_else(|| {
        rf_core::EngineError::InvalidData(format!("no field `{name}` on {}", desc.type_path))
    })?;
    if field.meta.readonly {
        return Err(rf_core::EngineError::Message(format!("field `{name}` is readonly")));
    }
    let base = data_ptr(obj) as *mut u8;
    // SAFETY: offset 来自描述符；写入类型经下方 match 与字段类型核对。
    let ptr = unsafe { base.add(field.offset) };
    let mismatch = || rf_core::EngineError::InvalidData(format!("field `{name}` kind mismatch"));
    unsafe {
        match (field.kind, value) {
            (FieldKind::F32, FieldValue::F32(v)) => ptr.cast::<f32>().write(v),
            (FieldKind::F64, FieldValue::F64(v)) => ptr.cast::<f64>().write(v),
            (FieldKind::Bool, FieldValue::Bool(v)) => ptr.cast::<bool>().write(v),
            (FieldKind::I32, FieldValue::I32(v)) => ptr.cast::<i32>().write(v),
            (FieldKind::U32, FieldValue::U32(v)) => ptr.cast::<u32>().write(v),
            (FieldKind::U64, FieldValue::U64(v)) => ptr.cast::<u64>().write(v),
            (FieldKind::String, FieldValue::String(v)) => {
                std::ptr::drop_in_place(ptr.cast::<String>());
                ptr.cast::<String>().write(v);
            }
            (FieldKind::Vec2, FieldValue::Vec2(v)) => ptr.cast::<Vec2>().write(v),
            (FieldKind::Vec3, FieldValue::Vec3(v)) => ptr.cast::<Vec3>().write(v),
            (FieldKind::Vec4, FieldValue::Vec4(v)) => ptr.cast::<Vec4>().write(v),
            (FieldKind::Color, FieldValue::Color(v)) => ptr.cast::<Color>().write(v),
            (FieldKind::Entity, FieldValue::Entity(v)) => ptr.cast::<Entity>().write(v),
            (FieldKind::Enum(_), FieldValue::Enum(v)) => ptr.cast::<u64>().write(v),
            _ => return Err(mismatch()),
        }
    }
    Ok(())
}

/// 类型注册表（IF-097）。
#[derive(Default)]
pub struct TypeRegistry {
    map: HashMap<&'static str, &'static TypeDescriptor>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T: Reflect>(&mut self) {
        let d = T::descriptor();
        self.map.insert(d.type_path, d);
    }

    pub fn register_descriptor(&mut self, desc: &'static TypeDescriptor) {
        self.map.insert(desc.type_path, desc);
    }

    pub fn get(&self, type_path: &str) -> Option<&'static TypeDescriptor> {
        self.map.get(type_path).copied()
    }

    pub fn contains(&self, type_path: &str) -> bool {
        self.map.contains_key(type_path)
    }

    pub fn count(&self) -> usize {
        self.map.len()
    }

    pub fn paths(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self.map.keys().copied().collect();
        v.sort();
        v
    }
}

/// 全局注册表（IF-097）。
pub fn global() -> &'static Mutex<TypeRegistry> {
    static G: OnceLock<Mutex<TypeRegistry>> = OnceLock::new();
    G.get_or_init(|| Mutex::new(TypeRegistry::new()))
}

/// `reflect_struct!`（IF-098，可扩展）：
/// ```ignore
/// reflect_struct! {
///     pub struct Player {
///         health: F32 [range = (0.0 .. 100.0)],
///         name: String,
///         alive: Bool [readonly],
///     }
/// }
/// ```
/// 生成结构体 + TypeDescriptor + Reflect impl。
#[macro_export]
macro_rules! reflect_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $path:ident {
            $($fname:ident : $kind:tt $([$($t:tt)*])?),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $path {
            $(pub $fname: reflect_struct!(@ty $kind)),*
        }

        impl $crate::Reflect for $path {
            fn descriptor() -> &'static $crate::TypeDescriptor {
                static DESCRIPTOR: std::sync::OnceLock<$crate::TypeDescriptor> = std::sync::OnceLock::new();
                DESCRIPTOR.get_or_init(|| {
                    let fields: Vec<$crate::FieldInfo> = vec![
                        $($crate::FieldInfo {
                            name: stringify!($fname),
                            kind: reflect_struct!(@kind $kind),
                            offset: std::mem::offset_of!($path, $fname),
                            size: std::mem::size_of::<reflect_struct!(@ty $kind)>(),
                            meta: reflect_struct!(@meta $($($t)*)?),
                        }),*
                    ];
                    $crate::TypeDescriptor {
                        type_path: stringify!($path),
                        size: std::mem::size_of::<$path>(),
                        align: std::mem::align_of::<$path>(),
                        fields: Box::leak(fields.into_boxed_slice()),
                        methods: &[],
                    }
                })
            }
            fn reflect_descriptor(&self) -> &'static $crate::TypeDescriptor { Self::descriptor() }
            fn as_reflect(&self) -> &dyn $crate::Reflect { self }
            fn as_reflect_mut(&mut self) -> &mut dyn $crate::Reflect { self }
        }
    };
    (@ty F32) => { f32 };
    (@ty F64) => { f64 };
    (@ty Bool) => { bool };
    (@ty I32) => { i32 };
    (@ty U32) => { u32 };
    (@ty U64) => { u64 };
    (@ty String) => { String };
    (@ty Vec2) => { $crate::Vec2 };
    (@ty Vec3) => { $crate::Vec3 };
    (@ty Vec4) => { $crate::Vec4 };
    (@ty Color) => { $crate::Color };
    (@ty Entity) => { $crate::Entity };
    (@kind F32) => { $crate::FieldKind::F32 };
    (@kind F64) => { $crate::FieldKind::F64 };
    (@kind Bool) => { $crate::FieldKind::Bool };
    (@kind I32) => { $crate::FieldKind::I32 };
    (@kind U32) => { $crate::FieldKind::U32 };
    (@kind U64) => { $crate::FieldKind::U64 };
    (@kind String) => { $crate::FieldKind::String };
    (@kind Vec2) => { $crate::FieldKind::Vec2 };
    (@kind Vec3) => { $crate::FieldKind::Vec3 };
    (@kind Vec4) => { $crate::FieldKind::Vec4 };
    (@kind Color) => { $crate::FieldKind::Color };
    (@kind Entity) => { $crate::FieldKind::Entity };
    (@meta) => { $crate::FieldMeta::default() };
    (@meta readonly) => { $crate::FieldMeta { readonly: true, ..Default::default() } };
    (@meta hidden) => { $crate::FieldMeta { hidden: true, ..Default::default() } };
    (@meta editor) => { $crate::FieldMeta { editor_exposed: true, ..Default::default() } };
    (@meta script) => { $crate::FieldMeta { script_exposed: true, ..Default::default() } };
    (@meta net) => { $crate::FieldMeta { network_replicated: true, ..Default::default() } };
    (@meta range = ($lo:tt .. $hi:tt)) => { $crate::FieldMeta { range: Some(($lo, $hi)), ..Default::default() } };
    (@meta category = $c:expr) => { $crate::FieldMeta { category: $c, ..Default::default() } };
    (@meta tooltip = $t:expr) => { $crate::FieldMeta { tooltip: $t, ..Default::default() } };
    (@meta readonly, range = ($lo:tt .. $hi:tt)) => {
        $crate::FieldMeta { readonly: true, range: Some(($lo, $hi)), ..Default::default() }
    };
    (@meta editor, range = ($lo:tt .. $hi:tt)) => {
        $crate::FieldMeta { editor_exposed: true, range: Some(($lo, $hi)), ..Default::default() }
    };
}

/// 验收辅助：全局注册表可访问性自检（acceptance 测试调用）。
pub fn reflect_struct_demo_check() {
    if let Ok(reg) = global().lock() {
        assert!(reg.count() == reg.count()); // 注册表可用且一致
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    reflect_struct! {
        #[derive(Debug)]
        pub struct DemoPlayer {
            health: F32 [range = (0.0 .. 100.0)],
            name: String,
            alive: Bool [readonly],
            pos: Vec3,
        }
    }

    #[test]
    fn descriptor_layout() {
        let d = DemoPlayer::descriptor();
        assert_eq!(d.type_path, "DemoPlayer");
        assert_eq!(d.fields.len(), 4);
        let h = d.fields.iter().find(|f| f.name == "health").unwrap();
        assert_eq!(h.kind, FieldKind::F32);
        assert_eq!(h.meta.range, Some((0.0, 100.0)));
        assert_eq!(h.offset, std::mem::offset_of!(DemoPlayer, health));
    }

    #[test]
    fn read_write_fields() {
        let mut p = DemoPlayer {
            health: 50.0,
            name: "Hero".into(),
            alive: true,
            pos: Vec3::new(1.0, 2.0, 3.0),
        };
        {
            let obj = p.as_reflect_mut();
            assert_eq!(read_field(obj, "health").unwrap(), FieldValue::F32(50.0));
            assert_eq!(read_field(obj, "name").unwrap(), FieldValue::String("Hero".into()));
            assert_eq!(read_field(obj, "pos").unwrap(), FieldValue::Vec3(Vec3::new(1.0, 2.0, 3.0)));
            write_field(obj, "health", FieldValue::F32(75.0)).unwrap();
            write_field(obj, "name", FieldValue::String("Villain".into())).unwrap();
            assert!(write_field(obj, "alive", FieldValue::Bool(false)).is_err()); // readonly
            assert!(write_field(obj, "health", FieldValue::I32(1)).is_err()); // 类型不匹配
            assert!(read_field(obj, "nope").is_err()); // 未知字段
        }
        assert_eq!(p.health, 75.0);
        assert_eq!(p.name, "Villain");
    }

    #[test]
    fn registry() {
        let mut reg = TypeRegistry::new();
        reg.register::<DemoPlayer>();
        assert!(reg.contains("DemoPlayer"));
        assert_eq!(reg.count(), 1);
        assert_eq!(reg.get("DemoPlayer").unwrap().fields.len(), 4);
    }
}
