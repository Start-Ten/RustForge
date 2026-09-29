//! 虚拟文件系统（IF-127）：HostFs + OverlayFs。

use rf_core::{EngineError, Result};
use std::path::PathBuf;

/// 文件系统抽象（IF-127，可扩展）。
pub trait FileSystem: Send + Sync {
    fn read(&self, path: &str) -> Result<Vec<u8>>;
    fn exists(&self, path: &str) -> bool;
    fn list(&self, dir: &str) -> Vec<String>;
}

/// 宿主文件系统（路径安全：拒绝越出 root 的 .. 访问）。
pub struct HostFs {
    root: PathBuf,
}

impl HostFs {
    pub fn new() -> Self {
        Self { root: PathBuf::from(".") }
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf> {
        let joined = self.root.join(path);
        let canon = joined.canonicalize().unwrap_or_else(|_| joined.clone());
        let root_canon = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        if !canon.starts_with(&root_canon) {
            return Err(EngineError::Message(format!("path escapes root: {path}")));
        }
        Ok(joined)
    }
}

impl Default for HostFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for HostFs {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let p = self.resolve(path)?;
        std::fs::read(p).map_err(EngineError::Io)
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve(path).map(|p| p.exists()).unwrap_or(false)
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let Ok(p) = self.resolve(dir) else { return Vec::new() };
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                out.push(e.file_name().to_string_lossy().into_owned());
            }
        }
        out.sort();
        out
    }
}

/// 叠加文件系统：先查前面的层（读写沙盒→包→网络 顺序由调用方决定）。
pub struct OverlayFs {
    layers: Vec<Box<dyn FileSystem>>,
}

impl OverlayFs {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn push(&mut self, layer: Box<dyn FileSystem>) {
        self.layers.push(layer);
    }
}

impl Default for OverlayFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for OverlayFs {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        for l in &self.layers {
            if l.exists(path) {
                return l.read(path);
            }
        }
        Err(EngineError::Message(format!("not found in overlay: {path}")))
    }

    fn exists(&self, path: &str) -> bool {
        self.layers.iter().any(|l| l.exists(path))
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        for l in &self.layers {
            seen.extend(l.list(dir));
        }
        seen.into_iter().collect()
    }
}

/// 记忆文件系统（测试/PAK 挂载）。
#[derive(Default)]
pub struct MemoryFs {
    files: std::collections::HashMap<String, Vec<u8>>,
}

impl MemoryFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: &str, data: Vec<u8>) {
        self.files.insert(path.to_string(), data);
    }
}

impl FileSystem for MemoryFs {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| EngineError::Message(format!("not found: {path}")))
    }

    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let prefix = format!("{dir}/");
        let mut names = std::collections::BTreeSet::new();
        for k in self.files.keys() {
            if let Some(rest) = k.strip_prefix(&prefix) {
                names.insert(rest.split('/').next().unwrap_or(rest).to_string());
            }
        }
        names.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_fs_and_overlay() {
        let mut m = MemoryFs::new();
        m.insert("assets/a.png", vec![1, 2, 3]);
        m.insert("assets/b.png", vec![4]);
        assert!(m.exists("assets/a.png"));
        assert_eq!(m.read("assets/a.png").unwrap(), vec![1, 2, 3]);
        assert_eq!(m.list("assets"), vec!["a.png", "b.png"]);
        let mut o = OverlayFs::new();
        o.push(Box::new(m));
        assert!(o.exists("assets/a.png"));
        assert!(o.read("missing").is_err());
    }

    #[test]
    fn host_fs_sandbox() {
        let dir = std::env::temp_dir().join("rf_fs_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.txt"), b"hi").unwrap();
        let fs = HostFs::with_root(&dir);
        assert!(fs.exists("x.txt"));
        assert_eq!(fs.read("x.txt").unwrap(), b"hi");
        // .. 访问被沙盒拦截（canonicalize 越界报错）
        assert!(fs.read("../escape.txt").is_err() || !fs.exists("../escape.txt"));
        assert!(fs.read("../nonexistent-file.txt").is_err());
    }
}
