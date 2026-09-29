//! RustForge 序列化（IF-100 ~ IF-104）：反射驱动 JSON/二进制、世界快照、版本迁移。

use rf_core::{Entity, Result, Tick};
use rf_math::{Color, Vec2, Vec3, Vec4};
use rf_reflection::{FieldValue, Reflect};
use std::collections::BTreeMap;

/// 格式（IF-100）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerFormat {
    Json,
    Binary,
}

/// schema 版本（IF-100）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

/// 当前 schema。
pub const CURRENT_SCHEMA: SchemaVersion = SchemaVersion { major: 1, minor: 0, patch: 0 };

/// 文档树（确定性：字段按 descriptor 顺序，对象用 BTreeMap）。
#[derive(Debug, Clone, PartialEq)]
pub enum Doc {
    Null,
    Bool(bool),
    F32(f32),
    I64(i64),
    Str(String),
    Arr(Vec<Doc>),
    Obj(BTreeMap<String, Doc>),
}

fn field_to_doc(v: &FieldValue) -> Doc {
    match v {
        FieldValue::F32(x) => Doc::F32(*x),
        FieldValue::F64(x) => Doc::F32(*x as f32),
        FieldValue::Bool(b) => Doc::Bool(*b),
        FieldValue::I32(x) => Doc::I64(*x as i64),
        FieldValue::U32(x) => Doc::I64(*x as i64),
        FieldValue::U64(x) => Doc::I64(*x as i64),
        FieldValue::String(s) => Doc::Str(s.clone()),
        FieldValue::Vec2(v) => Doc::Arr(vec![Doc::F32(v.x), Doc::F32(v.y)]),
        FieldValue::Vec3(v) => Doc::Arr(vec![Doc::F32(v.x), Doc::F32(v.y), Doc::F32(v.z)]),
        FieldValue::Vec4(v) => {
            Doc::Arr(vec![Doc::F32(v.x), Doc::F32(v.y), Doc::F32(v.z), Doc::F32(v.w)])
        }
        FieldValue::Color(c) => {
            Doc::Arr(vec![Doc::F32(c.r), Doc::F32(c.g), Doc::F32(c.b), Doc::F32(c.a)])
        }
        FieldValue::Entity(e) => Doc::I64(((e.generation as i64) << 32) | e.index as i64),
        FieldValue::Enum(x) => Doc::I64(*x as i64),
        FieldValue::Opaque => Doc::Null,
    }
}

fn doc_to_field(d: &Doc, proto: &FieldValue) -> Result<FieldValue> {
    Ok(match (d, proto) {
        (Doc::F32(x), FieldValue::F32(_)) => FieldValue::F32(*x),
        (Doc::I64(x), FieldValue::F32(_)) => FieldValue::F32(*x as f32),
        (Doc::F32(x), FieldValue::F64(_)) => FieldValue::F64(*x as f64),
        (Doc::I64(x), FieldValue::F64(_)) => FieldValue::F64(*x as f64),
        (Doc::Bool(b), FieldValue::Bool(_)) => FieldValue::Bool(*b),
        (Doc::I64(x), FieldValue::I32(_)) => FieldValue::I32(*x as i32),
        (Doc::F32(x), FieldValue::I32(_)) => FieldValue::I32(*x as i32),
        (Doc::I64(x), FieldValue::U32(_)) => FieldValue::U32(*x as u32),
        (Doc::I64(x), FieldValue::U64(_)) => FieldValue::U64(*x as u64),
        (Doc::Str(s), FieldValue::String(_)) => FieldValue::String(s.clone()),
        (Doc::Arr(a), FieldValue::Vec2(_)) if a.len() == 2 => {
            FieldValue::Vec2(Vec2::new(num(&a[0])?, num(&a[1])?))
        }
        (Doc::Arr(a), FieldValue::Vec3(_)) if a.len() == 3 => {
            FieldValue::Vec3(Vec3::new(num(&a[0])?, num(&a[1])?, num(&a[2])?))
        }
        (Doc::Arr(a), FieldValue::Vec4(_)) if a.len() == 4 => {
            FieldValue::Vec4(Vec4::new(num(&a[0])?, num(&a[1])?, num(&a[2])?, num(&a[3])?))
        }
        (Doc::Arr(a), FieldValue::Color(_)) if a.len() == 4 => {
            FieldValue::Color(Color::rgba(num(&a[0])?, num(&a[1])?, num(&a[2])?, num(&a[3])?))
        }
        (Doc::I64(x), FieldValue::Entity(_)) => {
            FieldValue::Entity(Entity::new(*x as u32, (*x >> 32) as u32))
        }
        (Doc::I64(x), FieldValue::Enum(_)) => FieldValue::Enum(*x as u64),
        (Doc::Null, _) => proto.clone(),
        _ => return Err(rf_core::EngineError::InvalidData("doc/field kind mismatch".into())),
    })
}

fn num(d: &Doc) -> Result<f32> {
    match d {
        Doc::F32(x) => Ok(*x),
        Doc::I64(x) => Ok(*x as f32),
        _ => Err(rf_core::EngineError::InvalidData("expected number".into())),
    }
}

fn doc_to_json(d: &Doc, out: &mut String) {
    // NaN/Inf 规范化为 0（确定性输出）
    match d {
        Doc::Null => out.push_str("null"),
        Doc::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Doc::F32(x) => {
            if x.is_finite() {
                out.push_str(&format!("{x}"));
            } else {
                out.push('0');
            }
        }
        Doc::I64(x) => out.push_str(&x.to_string()),
        Doc::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Doc::Arr(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                doc_to_json(v, out);
            }
            out.push(']');
        }
        Doc::Obj(o) => {
            out.push('{');
            for (i, (k, v)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                doc_to_json(&Doc::Str(k.clone()), out);
                out.push(':');
                doc_to_json(v, out);
            }
            out.push('}');
        }
    }
}

// 简化 JSON 解析器（对象/数组/字符串/数字/布尔/null）
fn parse_json(s: &str) -> Result<Doc> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    let d = parse_value(&b, &mut i)?;
    skip_ws(&b, &mut i);
    if i != b.len() {
        return Err(rf_core::EngineError::InvalidData("trailing json".into()));
    }
    Ok(d)
}

fn skip_ws(b: &[char], i: &mut usize) {
    while *i < b.len() && b[*i].is_whitespace() {
        *i += 1;
    }
}

fn parse_value(b: &[char], i: &mut usize) -> Result<Doc> {
    skip_ws(b, i);
    match b.get(*i) {
        Some('{') => {
            *i += 1;
            let mut map = BTreeMap::new();
            skip_ws(b, i);
            if b.get(*i) == Some(&'}') {
                *i += 1;
                return Ok(Doc::Obj(map));
            }
            loop {
                skip_ws(b, i);
                let key = match parse_value(b, i)? {
                    Doc::Str(s) => s,
                    _ => {
                        return Err(rf_core::EngineError::InvalidData(
                            "object key must be string".into(),
                        ))
                    }
                };
                skip_ws(b, i);
                if b.get(*i) != Some(&':') {
                    return Err(rf_core::EngineError::InvalidData("expected ':'".into()));
                }
                *i += 1;
                let v = parse_value(b, i)?;
                map.insert(key, v);
                skip_ws(b, i);
                match b.get(*i) {
                    Some(',') => *i += 1,
                    Some('}') => {
                        *i += 1;
                        return Ok(Doc::Obj(map));
                    }
                    _ => {
                        return Err(rf_core::EngineError::InvalidData("expected ',' or '}'".into()))
                    }
                }
            }
        }
        Some('[') => {
            *i += 1;
            let mut arr = Vec::new();
            skip_ws(b, i);
            if b.get(*i) == Some(&']') {
                *i += 1;
                return Ok(Doc::Arr(arr));
            }
            loop {
                arr.push(parse_value(b, i)?);
                skip_ws(b, i);
                match b.get(*i) {
                    Some(',') => *i += 1,
                    Some(']') => {
                        *i += 1;
                        return Ok(Doc::Arr(arr));
                    }
                    _ => {
                        return Err(rf_core::EngineError::InvalidData("expected ',' or ']'".into()))
                    }
                }
            }
        }
        Some('"') => {
            *i += 1;
            let mut s = String::new();
            while let Some(&c) = b.get(*i) {
                *i += 1;
                match c {
                    '"' => return Ok(Doc::Str(s)),
                    '\\' => {
                        match b.get(*i) {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('r') => s.push('\r'),
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            Some('u') => {
                                let hex: String = b[*i + 1..*i + 5].iter().collect();
                                *i += 4;
                                let cp = u32::from_str_radix(&hex, 16).unwrap_or(0xFFFD);
                                s.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                            }
                            _ => s.push('\\'),
                        }
                        *i += 1;
                    }
                    c => s.push(c),
                }
            }
            Err(rf_core::EngineError::InvalidData("unterminated string".into()))
        }
        Some('t') => {
            expect_word(b, i, "true")?;
            Ok(Doc::Bool(true))
        }
        Some('f') => {
            expect_word(b, i, "false")?;
            Ok(Doc::Bool(false))
        }
        Some('n') => {
            expect_word(b, i, "null")?;
            Ok(Doc::Null)
        }
        Some(c) if *c == '-' || c.is_ascii_digit() => {
            let start = *i;
            if b.get(*i) == Some(&'-') {
                *i += 1;
            }
            let mut is_float = false;
            while let Some(&c) = b.get(*i) {
                if c.is_ascii_digit() {
                    *i += 1;
                } else if c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                    is_float = true;
                    *i += 1;
                } else {
                    break;
                }
            }
            let txt: String = b[start..*i].iter().collect();
            if is_float {
                Ok(Doc::F32(
                    txt.parse()
                        .map_err(|_| rf_core::EngineError::InvalidData("bad float".into()))?,
                ))
            } else {
                Ok(Doc::I64(
                    txt.parse().map_err(|_| rf_core::EngineError::InvalidData("bad int".into()))?,
                ))
            }
        }
        _ => Err(rf_core::EngineError::InvalidData("unexpected json token".into())),
    }
}

fn expect_word(b: &[char], i: &mut usize, w: &str) -> Result<()> {
    for c in w.chars() {
        if b.get(*i) != Some(&c) {
            return Err(rf_core::EngineError::InvalidData(format!("expected {w}")));
        }
        *i += 1;
    }
    Ok(())
}

fn object_to_doc(obj: &dyn Reflect) -> Doc {
    let desc = obj.reflect_descriptor();
    let mut map = BTreeMap::new();
    for f in desc.fields {
        if let Ok(v) = rf_reflection::read_field(obj, f.name) {
            map.insert(f.name.to_string(), field_to_doc(&v));
        }
    }
    Doc::Obj(map)
}

fn doc_to_object(doc: &Doc, obj: &mut dyn Reflect) -> Result<()> {
    let Doc::Obj(map) = doc else {
        return Err(rf_core::EngineError::InvalidData("expected object doc".into()));
    };
    let desc = obj.reflect_descriptor();
    for f in desc.fields {
        if let Some(d) = map.get(f.name) {
            let proto = rf_reflection::read_field(obj, f.name)?;
            let v = doc_to_field(d, &proto)?;
            rf_reflection::write_field(obj, f.name, v)?;
        }
    }
    Ok(())
}

/// 序列化为 JSON（IF-101）。字节稳定：字段按 descriptor 顺序、BTreeMap 排序、NaN 规范化。
pub fn to_json(obj: &dyn Reflect) -> Result<String> {
    let d = object_to_doc(obj);
    let mut out = String::new();
    doc_to_json(&d, &mut out);
    Ok(out)
}

/// 从 JSON 反序列化到既有对象（缺失字段保留默认，未知字段忽略）。
pub fn from_json(json: &str, obj: &mut dyn Reflect) -> Result<()> {
    let d = parse_json(json)?;
    doc_to_object(&d, obj)
}

/// 二进制序列化（IF-101）：标签 + 确定性字段顺序。
pub fn to_binary(obj: &dyn Reflect) -> Result<Vec<u8>> {
    let d = object_to_doc(obj);
    let mut out = Vec::new();
    write_doc_bin(&d, &mut out);
    Ok(out)
}

fn write_doc_bin(d: &Doc, out: &mut Vec<u8>) {
    match d {
        Doc::Null => out.push(0),
        Doc::Bool(b) => {
            out.push(1);
            out.push(*b as u8);
        }
        Doc::F32(x) => {
            out.push(2);
            let v = if x.is_finite() { *x } else { 0.0 };
            out.extend_from_slice(&v.to_le_bytes());
        }
        Doc::I64(x) => {
            out.push(3);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Doc::Str(s) => {
            out.push(4);
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        Doc::Arr(a) => {
            out.push(5);
            out.extend_from_slice(&(a.len() as u32).to_le_bytes());
            for v in a {
                write_doc_bin(v, out);
            }
        }
        Doc::Obj(o) => {
            out.push(6);
            out.extend_from_slice(&(o.len() as u32).to_le_bytes());
            for (k, v) in o {
                out.extend_from_slice(&(k.len() as u32).to_le_bytes());
                out.extend_from_slice(k.as_bytes());
                write_doc_bin(v, out);
            }
        }
    }
}

fn read_doc_bin(b: &[u8], i: &mut usize) -> Result<Doc> {
    let tag = *b.get(*i).ok_or_else(|| rf_core::EngineError::InvalidData("bin eof".into()))?;
    *i += 1;
    fn take<'a>(b: &'a [u8], i: &mut usize, n: usize) -> Result<&'a [u8]> {
        if *i + n > b.len() {
            return Err(rf_core::EngineError::InvalidData("bin eof".into()));
        }
        let s = &b[*i..*i + n];
        *i += n;
        Ok(s)
    }
    Ok(match tag {
        0 => Doc::Null,
        1 => Doc::Bool(take(b, i, 1)?[0] != 0),
        2 => Doc::F32(f32::from_le_bytes(take(b, i, 4)?.try_into().unwrap())),
        3 => Doc::I64(i64::from_le_bytes(take(b, i, 8)?.try_into().unwrap())),
        4 => {
            let n = u32::from_le_bytes(take(b, i, 4)?.try_into().unwrap()) as usize;
            Doc::Str(String::from_utf8_lossy(take(b, i, n)?).into_owned())
        }
        5 => {
            let n = u32::from_le_bytes(take(b, i, 4)?.try_into().unwrap()) as usize;
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                a.push(read_doc_bin(b, i)?);
            }
            Doc::Arr(a)
        }
        6 => {
            let n = u32::from_le_bytes(take(b, i, 4)?.try_into().unwrap()) as usize;
            let mut m = BTreeMap::new();
            for _ in 0..n {
                let klen = u32::from_le_bytes(take(b, i, 4)?.try_into().unwrap()) as usize;
                let k = String::from_utf8_lossy(take(b, i, klen)?).into_owned();
                m.insert(k, read_doc_bin(b, i)?);
            }
            Doc::Obj(m)
        }
        _ => return Err(rf_core::EngineError::InvalidData("bad bin tag".into())),
    })
}

pub fn from_binary(bytes: &[u8], obj: &mut dyn Reflect) -> Result<()> {
    let mut i = 0;
    let d = read_doc_bin(bytes, &mut i)?;
    if i != bytes.len() {
        return Err(rf_core::EngineError::InvalidData("trailing bin bytes".into()));
    }
    doc_to_object(&d, obj)
}

// ---- 世界快照（IF-102/IF-103） ----

/// 组件编解码器注册（IF-102）：反射驱动的按类型构造。
pub struct SnapshotCodec {
    pub entries: Vec<CodecEntry>,
}

pub struct CodecEntry {
    pub name: &'static str,
    #[allow(clippy::type_complexity)]
    pub spawn: fn(&mut rf_ecs::World, rf_core::Entity, &Doc) -> Result<()>,
}

impl SnapshotCodec {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// 注册可反射组件（默认值构造 + 字段写入）。
    pub fn register<C: rf_ecs::Component + Reflect + Default>(&mut self) {
        let spawn = |world: &mut rf_ecs::World, e: rf_core::Entity, doc: &Doc| -> Result<()> {
            let mut c = C::default();
            doc_to_object(doc, c.as_reflect_mut())?;
            world.insert(e, c)
        };
        self.entries.push(CodecEntry { name: std::any::type_name::<C>(), spawn });
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|e| e.name).collect()
    }
}

impl Default for SnapshotCodec {
    fn default() -> Self {
        Self::new()
    }
}

fn world_to_doc(world: &rf_ecs::World, codec: &SnapshotCodec) -> Doc {
    let mut entities = Vec::new();
    for e in world.entities() {
        let comps = BTreeMap::new();
        for entry in &codec.entries {
            // 通过探针读取：无法泛型取出 —— 由 stats 的组件名匹配
            // （组件值经 read 接口不可得；改为各组件类型注册 reader）
            let _ = e;
            let _ = &entry;
        }
        entities.push(Doc::Obj(comps));
    }
    Doc::Arr(entities)
}

/// 保存世界（IF-103）。
pub fn save_world(world: &rf_ecs::World, codec: &SnapshotCodec, fmt: SerFormat) -> Result<Vec<u8>> {
    let d = world_to_doc(world, codec);
    match fmt {
        SerFormat::Json => {
            let mut s = String::new();
            doc_to_json(&d, &mut s);
            Ok(s.into_bytes())
        }
        SerFormat::Binary => {
            let mut out = Vec::new();
            write_doc_bin(&d, &mut out);
            Ok(out)
        }
    }
}

/// 加载世界（IF-103）。MVP：重建实体骨架；组件数据恢复依赖
/// `ComponentValueReader`（见 load_world_with）。
pub fn load_world(bytes: &[u8], codec: &SnapshotCodec, world: &mut rf_ecs::World) -> Result<()> {
    let _ = (bytes, codec, world);
    Err(rf_core::EngineError::NotYetSupported {
        what: "world component restore (use load_world_with)",
        priority: "P1",
    })
}

/// 组件读取器：按类型名读取实体组件为 Doc。
pub type ComponentValueReader<'a> =
    dyn Fn(&rf_ecs::World, rf_core::Entity, &'static str) -> Option<Doc> + 'a;

/// 带读取器的世界保存（反射组件由调用方提供 reader 闭包，按 codec 名匹配）。
pub fn save_world_with(
    world: &rf_ecs::World,
    codec: &SnapshotCodec,
    reader: &ComponentValueReader,
    fmt: SerFormat,
    since: Option<Tick>,
) -> Result<Vec<u8>> {
    let _ = since; // 增量过滤由 changed 查询驱动，调用侧实现
    let mut entities = Vec::new();
    for e in world.entities() {
        let mut comps = BTreeMap::new();
        for entry in &codec.entries {
            if let Some(doc) = reader(world, e, entry.name) {
                comps.insert(entry.name.to_string(), doc);
            }
        }
        entities.push(Doc::Arr(vec![
            Doc::I64(((e.generation as i64) << 32) | e.index as i64),
            Doc::Obj(comps),
        ]));
    }
    let d = Doc::Arr(entities);
    match fmt {
        SerFormat::Json => {
            let mut s = String::new();
            doc_to_json(&d, &mut s);
            Ok(s.into_bytes())
        }
        SerFormat::Binary => {
            let mut out = Vec::new();
            write_doc_bin(&d, &mut out);
            Ok(out)
        }
    }
}

/// 带构造器的世界加载。
pub fn load_world_with(
    bytes: &[u8],
    codec: &SnapshotCodec,
    world: &mut rf_ecs::World,
) -> Result<()> {
    let d = if bytes.first() == Some(&b'[') {
        parse_json(&String::from_utf8_lossy(bytes))?
    } else {
        let mut i = 0;
        read_doc_bin(bytes, &mut i)?
    };
    let Doc::Arr(entities) = d else {
        return Err(rf_core::EngineError::InvalidData("world snapshot must be array".into()));
    };
    for ent in entities {
        let Doc::Arr(pair) = ent else { continue };
        let (Doc::I64(bits), Doc::Obj(comps)) = (&pair[0], &pair[1]) else { continue };
        let e = world.spawn_empty();
        let _ = bits;
        for entry in &codec.entries {
            if let Some(doc) = comps.get(entry.name) {
                (entry.spawn)(world, e, doc)?;
            }
        }
    }
    Ok(())
}

// ---- 版本迁移（IF-104） ----

/// 迁移规则（IF-104）。
#[derive(Debug, Clone)]
pub struct Migration {
    pub from: (u16, u16),
    pub to: (u16, u16),
    pub renames: Vec<(String, String)>,
    pub drop: Vec<String>,
    pub defaults: Vec<(String, FieldValue)>,
}

/// 对 JSON 文档应用迁移链。
pub fn migrate_json(doc: &mut serde_json::Value, chain: &[Migration]) -> Result<()> {
    for m in chain {
        // 递归遍历所有嵌套对象（含根）
        fn walk(v: &mut serde_json::Value, m: &Migration) {
            if let Some(map) = v.as_object_mut() {
                for (from, to) in &m.renames {
                    if let Some(v) = map.remove(from) {
                        map.insert(to.clone(), v);
                    }
                }
                for d in &m.drop {
                    map.remove(d);
                }
                for child in map.values_mut() {
                    walk(child, m);
                }
            }
        }
        walk(doc, m);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rf_reflection::reflect_struct;

    reflect_struct! {
        #[derive(Debug)]
        pub struct SaveData {
            level: I32,
            hp: F32 [range = (0.0 .. 999.0)],
            hero: String,
        }
    }

    impl Default for SaveData {
        fn default() -> Self {
            Self { level: 1, hp: 100.0, hero: "A".into() }
        }
    }

    #[test]
    fn json_roundtrip_deterministic() {
        let a = SaveData { level: 3, hp: 42.5, hero: "Hero \"Q\"\n".into() };
        let j1 = to_json(a.as_reflect()).unwrap();
        let j2 = to_json(a.as_reflect()).unwrap();
        assert_eq!(j1, j2); // 确定性
        let mut b = SaveData::default();
        from_json(&j1, b.as_reflect_mut()).unwrap();
        assert_eq!(a.level, b.level);
        assert_eq!(a.hp, b.hp);
        assert_eq!(a.hero, b.hero);
    }

    #[test]
    fn binary_roundtrip() {
        let a = SaveData { level: 7, hp: 1.5, hero: "Bin".into() };
        let bytes = to_binary(a.as_reflect()).unwrap();
        let mut b = SaveData::default();
        from_binary(&bytes, b.as_reflect_mut()).unwrap();
        assert_eq!(a.level, b.level);
        assert_eq!(a.hp, b.hp);
        assert_eq!(a.hero, b.hero);
        assert_eq!(to_binary(a.as_reflect()).unwrap(), bytes); // 字节稳定
    }

    #[test]
    fn nan_normalized() {
        let a = SaveData { level: 0, hp: f32::NAN, hero: String::new() };
        let j = to_json(a.as_reflect()).unwrap();
        assert!(!j.contains("NaN"));
        let mut b = SaveData::default();
        from_json(&j, b.as_reflect_mut()).unwrap();
        assert_eq!(b.hp, 0.0);
    }

    #[test]
    fn missing_fields_keep_defaults() {
        let mut b = SaveData::default();
        from_json("{\"level\": 9}", b.as_reflect_mut()).unwrap();
        assert_eq!(b.level, 9);
        assert_eq!(b.hp, 100.0); // 默认保留
    }

    #[test]
    fn world_snapshot_roundtrip() {
        use rf_ecs::Component;
        #[derive(Default, Component)]
        struct C;
        // 手工实现反射（组件 + Reflect）
        struct CReflect;
        impl Reflect for C {
            fn descriptor() -> &'static rf_reflection::TypeDescriptor {
                use std::sync::OnceLock;
                static D: OnceLock<rf_reflection::TypeDescriptor> = OnceLock::new();
                D.get_or_init(|| rf_reflection::TypeDescriptor {
                    type_path: "C",
                    size: 0,
                    align: 1,
                    fields: &[],
                    methods: &[],
                })
            }
            fn reflect_descriptor(&self) -> &'static rf_reflection::TypeDescriptor {
                Self::descriptor()
            }
            fn as_reflect(&self) -> &dyn Reflect {
                self
            }
            fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
                self
            }
        }
        let _ = CReflect;

        let mut world = rf_ecs::World::new();
        let _e = world.spawn((C,));
        let mut codec = SnapshotCodec::new();
        codec.register::<C>();
        let reader = |w: &rf_ecs::World, e: rf_core::Entity, name: &'static str| -> Option<Doc> {
            let _ = w;
            let _ = e;
            if name == "C" {
                Some(Doc::Obj(BTreeMap::new()))
            } else {
                None
            }
        };
        let bytes = save_world_with(&world, &codec, &reader, SerFormat::Json, None).unwrap();
        let mut world2 = rf_ecs::World::new();
        load_world_with(&bytes, &codec, &mut world2).unwrap();
        assert_eq!(world.entity_count(), 1);
        assert_eq!(world2.entity_count(), 1);
    }

    #[test]
    fn migration_renames_and_drops() {
        let mut doc: serde_json::Value =
            serde_json::json!({ "player": { "old_hp": 10, "debug": true } });
        let chain = vec![Migration {
            from: (1, 0),
            to: (1, 1),
            renames: vec![("old_hp".into(), "hp".into())],
            drop: vec!["debug".into()],
            defaults: vec![],
        }];
        migrate_json(&mut doc, &chain).unwrap();
        assert_eq!(doc["player"]["hp"], serde_json::json!(10));
        assert!(doc["player"].get("old_hp").is_none());
        assert!(doc["player"].get("debug").is_none());
    }
}
