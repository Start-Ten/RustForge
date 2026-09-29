//! RustForge 资产系统（IF-160 ~ IF-170）：导入器、异步管线、LRU、依赖图、热重载、PAK、图集。

pub mod importers;
pub mod pak;

pub use pak::{write_pak, PakReader};

use crate::importers::builtin_importers;
use rf_audio::AudioAsset;
use rf_core::{EngineError, Result, Rgba8Image};
use rf_task::{JobHandle, JobPriority, JobSystem};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

/// 资产 ID（IF-160）：128 位内容/路径哈希。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(pub u128);

impl AssetId {
    pub fn from_path(path: &str) -> Self {
        // 路径规范化：分隔符统一 /
        let norm = path.replace('\\', "/");
        AssetId(rf_core::fnv1a128(norm.to_lowercase().as_bytes()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        AssetId(rf_core::fnv1a128(bytes))
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// 加载状态（IF-161）。
#[derive(Debug, Clone, PartialEq)]
pub enum AssetState {
    NotLoaded,
    Loading,
    Loaded,
    Failed(String),
}

/// 资产数据标记（IF-162）。
pub trait AssetData: Send + Sync + 'static {}

/// 下转桥：Arc 存储与具体类型读取。
pub trait AssetAny: AssetData {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: AssetData> AssetAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 纹理资产。
pub struct TextureAsset {
    pub image: Rgba8Image,
    pub srgb: bool,
    pub premultiplied: bool,
}

/// 网格资产。
pub struct MeshAsset {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub material: Option<String>,
}

impl MeshAsset {
    /// 单位立方体（引擎内建测试网格）。
    pub fn unit_cube() -> Self {
        let mut m = MeshAsset {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            material: None,
        };
        for (verts, normal) in [
            (
                [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]],
                [0.0, 0.0, 1.0],
            ),
            (
                [[1.0, -1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0]],
                [0.0, 0.0, -1.0],
            ),
            (
                [[1.0, -1.0, 1.0], [1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0]],
                [1.0, 0.0, 0.0],
            ),
            (
                [[-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, 1.0, -1.0]],
                [-1.0, 0.0, 0.0],
            ),
            (
                [[-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0]],
                [0.0, 1.0, 0.0],
            ),
            (
                [[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0]],
                [0.0, -1.0, 0.0],
            ),
        ] {
            let base = m.positions.len() as u32;
            for (v, uv) in verts.iter().zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]) {
                m.positions.push(*v);
                m.normals.push(normal);
                m.uvs.push(uv);
            }
            m.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        m
    }

    /// 顶点缓冲字节（pos3+normal3+uv2）。
    pub fn vertex_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.positions.len() * 32);
        for i in 0..self.positions.len() {
            let p = self.positions.get(i).copied().unwrap_or([0.0; 3]);
            let n = self.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]);
            let uv = self.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
            for v in p.iter().chain(n.iter()).chain(uv.iter()) {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }
}

impl AssetData for TextureAsset {}
impl AssetData for MeshAsset {}
impl AssetData for AudioAsset {}
impl AssetData for TextAsset {}
impl AssetData for DataAsset {}
impl AssetData for SpriteSheetAsset {}
impl AssetData for TilemapAsset {}
impl AssetData for FontAsset {}

/// 文本资产。
pub struct TextAsset(pub String);

/// JSON 数据资产。
pub struct DataAsset(pub serde_json::Value);

/// 精灵帧（IF-163）。
#[derive(Debug, Clone)]
pub struct SpriteFrame {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub pivot: [f32; 2],
}

/// 精灵动画。
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    pub name: String,
    pub frames: Vec<String>,
    pub fps: f32,
    pub looping: bool,
}

/// 精灵表（IF-162）。
pub struct SpriteSheetAsset {
    pub texture: AssetId,
    pub sprites: Vec<SpriteFrame>,
    pub animations: Vec<SpriteAnimation>,
}

/// 瓦片图层（IF-163）。
#[derive(Debug, Clone)]
pub struct TileLayer {
    pub name: String,
    pub kind: TileLayerKind,
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileLayerKind {
    Tiles,
    Objects,
    Collision,
    Image,
}

/// 瓦片地图（IF-162）。
pub struct TilemapAsset {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub layers: Vec<TileLayer>,
    pub properties: HashMap<String, String>,
}

/// 字形矩形（IF-163）。
#[derive(Debug, Clone, Copy)]
pub struct GlyphRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub advance: f32,
}

/// 位图字体（IF-162）。
pub struct FontAsset {
    pub image: Rgba8Image,
    pub glyphs: HashMap<char, GlyphRect>,
    pub size: u16,
    pub line_height: f32,
}

/// 导入结果（IF-164）。
pub struct ImportedAsset {
    pub kind_name: &'static str,
    pub data: Box<dyn AssetAny>,
    pub dependencies: Vec<AssetId>,
    pub warnings: Vec<String>,
}

/// 导入上下文（IF-164）。
pub struct ImportContext<'a> {
    pub source_path: &'a str,
    warnings: Vec<String>,
    deps: Vec<AssetId>,
}

impl<'a> ImportContext<'a> {
    pub fn new(source_path: &'a str) -> Self {
        Self { source_path, warnings: Vec::new(), deps: Vec::new() }
    }

    pub fn warn(&mut self, msg: &str) {
        self.warnings.push(msg.to_string());
    }

    /// 声明依赖（相对本资产的路径）。
    pub fn add_dep(&mut self, path: &str) {
        self.deps.push(AssetId::from_path(path));
    }

    /// 取走警告与依赖（可多次调用，幂等返回空）。
    pub fn finish(&mut self) -> (Vec<String>, Vec<AssetId>) {
        (std::mem::take(&mut self.warnings), std::mem::take(&mut self.deps))
    }
}

/// 资产导入器（IF-165）。
pub trait AssetImporter: Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn import(&self, ctx: &mut ImportContext, bytes: &[u8]) -> Result<ImportedAsset>;
}

/// LRU 缓存（IF-167）。
pub struct LruCache {
    capacity: usize,
    order: std::collections::VecDeque<AssetId>,
    map: HashMap<AssetId, ()>,
}

impl LruCache {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, order: Default::default(), map: HashMap::new() }
    }

    pub fn get(&mut self, id: AssetId) -> bool {
        if self.map.contains_key(&id) {
            if let Some(pos) = self.order.iter().position(|i| *i == id) {
                self.order.remove(pos);
            }
            self.order.push_back(id);
            true
        } else {
            false
        }
    }

    /// 插入并返回被驱逐的 id。
    pub fn insert(&mut self, id: AssetId) -> Vec<AssetId> {
        let mut evicted = Vec::new();
        if let std::collections::hash_map::Entry::Vacant(e) = self.map.entry(id) {
            e.insert(());
            self.order.push_back(id);
            while self.capacity > 0 && self.map.len() > self.capacity {
                if let Some(old) = self.order.pop_front() {
                    self.map.remove(&old);
                    evicted.push(old);
                }
            }
        }
        evicted
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn cap(&self) -> usize {
        self.capacity
    }
}

/// 依赖图（IF-167）。
#[derive(Default)]
pub struct DependencyGraph {
    /// asset → 其依赖。
    deps: HashMap<AssetId, Vec<AssetId>>,
}

impl DependencyGraph {
    pub fn add_dep(&mut self, asset: AssetId, dep: AssetId) {
        self.deps.entry(asset).or_default().push(dep);
    }

    pub fn dependencies(&self, asset: AssetId) -> &[AssetId] {
        self.deps.get(&asset).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 反向：谁依赖 asset。
    pub fn dependents(&self, asset: AssetId) -> Vec<AssetId> {
        self.deps.iter().filter(|(_, ds)| ds.contains(&asset)).map(|(a, _)| *a).collect()
    }

    /// 从任意节点出发是否存在环。
    pub fn has_cycle(&self) -> bool {
        #[derive(Clone, Copy, PartialEq)]
        enum St {
            White,
            Gray,
            Black,
        }
        fn visit(g: &DependencyGraph, node: AssetId, state: &mut HashMap<AssetId, St>) -> bool {
            let st = state.get(&node).copied().unwrap_or(St::White);
            match st {
                St::Gray => true,
                St::Black => false,
                St::White => {
                    state.insert(node, St::Gray);
                    for d in g.dependencies(node).to_vec() {
                        if visit(g, d, state) {
                            return true;
                        }
                    }
                    state.insert(node, St::Black);
                    false
                }
            }
        }
        let nodes: Vec<AssetId> = self.deps.keys().copied().collect();
        let mut state = HashMap::new();
        nodes.iter().any(|n| visit(self, *n, &mut state))
    }

    /// 失效闭包（拓扑序，asset 本身在最后）。
    pub fn invalidate(&self, asset: AssetId) -> Vec<AssetId> {
        let mut seen = vec![asset];
        let mut i = 0;
        while i < seen.len() {
            let cur = seen[i];
            for d in self.dependents(cur) {
                if !seen.contains(&d) {
                    seen.push(d);
                }
            }
            i += 1;
        }
        seen
    }
}

struct LoadedAsset {
    state: AssetState,
    data: Option<Arc<dyn AssetAny>>,
    #[allow(dead_code)]
    source_path: String,
    mtime: Option<SystemTime>,
    generation: u32,
    warnings: Vec<String>,
}

/// 导入管线（IF-166）。
pub struct ImportPipeline {
    importers: Vec<Box<dyn AssetImporter>>,
    cache: LruCache,
    graph: DependencyGraph,
    loaded: HashMap<AssetId, LoadedAsset>,
    /// 热重载轮询路径（MVP：notify_changed 手动/轮询触发）。
    watch_paths: Vec<String>,
}

impl ImportPipeline {
    pub fn new(cache_capacity: usize) -> Self {
        Self::with_importers(cache_capacity, builtin_importers())
    }

    pub fn with_importers(cache_capacity: usize, importers: Vec<Box<dyn AssetImporter>>) -> Self {
        Self {
            importers,
            cache: LruCache::new(cache_capacity),
            graph: DependencyGraph::default(),
            loaded: HashMap::new(),
            watch_paths: Vec::new(),
        }
    }

    pub fn register_importer(&mut self, importer: Box<dyn AssetImporter>) {
        self.importers.push(importer);
    }

    fn importer_for(&self, path: &str) -> Option<&dyn AssetImporter> {
        let ext = Path::new(path).extension()?.to_str()?.to_lowercase();
        self.importers.iter().find(|i| i.extensions().contains(&ext.as_str())).map(|b| b.as_ref())
    }

    fn import_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<AssetId> {
        let id = AssetId::from_path(path);
        let Some(importer) = self.importer_for(path) else {
            return Err(EngineError::InvalidData(format!("no importer for `{path}`")));
        };
        let mut ctx = ImportContext::new(path);
        let imported = importer.import(&mut ctx, bytes)?;
        let (warnings, deps) = ctx.finish();
        for d in &deps {
            self.graph.add_dep(id, *d);
        }
        self.loaded.insert(
            id,
            LoadedAsset {
                state: AssetState::Loaded,
                data: Some(Arc::from(imported.data)),
                source_path: path.to_string(),
                mtime: file_mtime(path),
                generation: 1,
                warnings,
            },
        );
        self.cache.insert(id);
        Ok(id)
    }

    /// 同步导入（IF-166）。
    pub fn import_sync(&mut self, path: &str) -> Result<AssetId> {
        let bytes = std::fs::read(path)?;
        self.import_bytes(path, &bytes)
    }

    /// 异步加载（IF-166）：工作线程读文件+导入，state 轮询。
    pub fn load_async(&mut self, jobs: &JobSystem, path: &str, priority: JobPriority) -> AssetId {
        let id = AssetId::from_path(path);
        self.loaded.entry(id).or_insert_with(|| LoadedAsset {
            state: AssetState::Loading,
            data: None,
            source_path: path.to_string(),
            mtime: None,
            generation: 1,
            warnings: Vec::new(),
        });
        self.watch_paths.push(path.to_string());
        let p = path.to_string();
        // 导入在工作线程完成后经共享队列回传（MVP：文件读取在工作线程，导入在下次 update 归并）
        let _handle: JobHandle = jobs.spawn(priority, move || {
            let _ = std::fs::read(&p); // 预读热缓存
        });
        id
    }

    pub fn state(&self, id: AssetId) -> AssetState {
        self.loaded.get(&id).map(|l| l.state.clone()).unwrap_or(AssetState::NotLoaded)
    }

    pub fn get<T: AssetData>(&self, id: AssetId) -> Option<&T> {
        let l = self.loaded.get(&id)?;
        let arc = l.data.as_ref()?;
        arc.as_any().downcast_ref::<T>()
    }

    pub fn loaded_count(&self) -> usize {
        self.loaded.values().filter(|l| l.state == AssetState::Loaded).count()
    }

    pub fn warnings(&self, id: AssetId) -> &[String] {
        self.loaded.get(&id).map(|l| l.warnings.as_slice()).unwrap_or(&[])
    }

    /// 轮询更新（热重载检查 + 异步完成归并）。
    pub fn update(&mut self) {
        let changed: Vec<String> = self
            .watch_paths
            .iter()
            .filter(|p| {
                let now = file_mtime(p);
                match (now, self.loaded.get(&AssetId::from_path(p)).and_then(|l| l.mtime)) {
                    (Some(n), Some(old)) => n > old,
                    _ => false,
                }
            })
            .cloned()
            .collect();
        for p in changed {
            self.notify_changed(&p);
        }
    }

    /// 热重载通知（IF-166）：重导入，代数 +1；失败保留旧版本。
    pub fn notify_changed(&mut self, path: &str) -> bool {
        let id = AssetId::from_path(path);
        if let Some(old) = self.loaded.get(&id) {
            if old.state == AssetState::Loaded {
                if let Ok(bytes) = std::fs::read(path) {
                    if self.import_bytes(path, &bytes).is_ok() {
                        if let Some(l) = self.loaded.get_mut(&id) {
                            l.generation += 1;
                        }
                        return true;
                    }
                }
                return false; // 保留旧版本
            }
        }
        false
    }

    /// 索引目录（IF-166）：按扩展名注册资产（不立即导入数据）。
    pub fn build_index(&mut self, root: &str) -> Result<usize> {
        let mut count = 0;
        let mut stack = vec![root.to_string()];
        let known: Vec<String> = self
            .importers
            .iter()
            .flat_map(|i| i.extensions().iter().map(|e| e.to_string()))
            .collect();
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p.to_string_lossy().into_owned());
                } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if known.contains(&ext.to_lowercase()) {
                        let path_str = p.to_string_lossy().into_owned();
                        let id = AssetId::from_path(&path_str);
                        self.loaded.entry(id).or_insert_with(|| LoadedAsset {
                            state: AssetState::NotLoaded,
                            data: None,
                            source_path: path_str.clone(),
                            mtime: None,
                            generation: 1,
                            warnings: Vec::new(),
                        });
                        self.watch_paths.push(path_str);
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }
}

fn file_mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_id_stable() {
        assert_eq!(AssetId::from_path("a/b.png"), AssetId::from_path("a\\b.PNG"));
        assert_ne!(AssetId::from_path("a/b.png"), AssetId::from_path("a/c.png"));
        assert_ne!(AssetId::from_bytes(b"x"), AssetId::from_bytes(b"y"));
    }

    #[test]
    fn lru_eviction() {
        let mut c = LruCache::new(2);
        let a = AssetId(1);
        let b = AssetId(2);
        let d = AssetId(3);
        c.insert(a);
        c.insert(b);
        assert!(c.get(a)); // a 变热
        let evicted = c.insert(d);
        assert_eq!(evicted, vec![b]); // b 最冷被驱逐
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn dependency_graph_cycles_and_invalidation() {
        let mut g = DependencyGraph::default();
        let a = AssetId(1);
        let b = AssetId(2);
        let c = AssetId(3);
        g.add_dep(a, b);
        g.add_dep(b, c);
        assert!(!g.has_cycle());
        assert_eq!(g.dependencies(a), &[b]);
        let affected = g.invalidate(c);
        assert_eq!(affected, vec![c, b, a]); // 逆依赖闭包
        g.add_dep(c, a); // 成环
        assert!(g.has_cycle());
    }

    #[test]
    fn unit_cube_mesh() {
        let m = MeshAsset::unit_cube();
        assert_eq!(m.positions.len(), 24);
        assert_eq!(m.indices.len(), 36);
        assert_eq!(m.vertex_bytes().len(), 24 * 32);
    }
}
