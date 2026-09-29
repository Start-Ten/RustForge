//! RustForge 插件系统（IF-110 ~ IF-115）。

use rf_core::{Result, Version, ENGINE_VERSION};
use std::any::Any;

/// 插件元数据（IF-111）。
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: Version,
    pub author: String,
    pub description: String,
    pub license: String,
    pub dependencies: Vec<String>,
}

impl PluginMetadata {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: Version::new(0, 1, 0),
            author: String::new(),
            description: String::new(),
            license: "MIT".into(),
            dependencies: Vec::new(),
        }
    }
}

/// 插件（IF-110）。
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn register(&self, registry: &mut Registry) -> Result<()>;
    /// 卸载前保存状态（热重载往返）。
    fn save_state(&self) -> Option<String> {
        None
    }
    /// 重载后恢复状态。
    fn restore_state(&mut self, _state: &str) {}
}

/// 扩展点（IF-112，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtensionPoint {
    RhiBackend,
    AssetImporter,
    EditorPanel,
    DebuggerProvider,
    ScriptHost,
    RenderPass,
    PhysicsBackend,
    AudioBackend,
    Renderer2DBackend,
    Physics2DBackend,
    Distribution,
}

/// 注册表（IF-113）：类型擦除槽位。
#[derive(Default)]
pub struct Registry {
    slots: Vec<(ExtensionPoint, Box<dyn Any + Send + Sync>)>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, point: ExtensionPoint, item: Box<dyn Any + Send + Sync>) -> SlotId {
        self.slots.push((point, item));
        SlotId(self.slots.len() as u64 - 1)
    }

    pub fn items(&self, point: ExtensionPoint) -> Vec<&(dyn Any + Send + Sync)> {
        self.slots.iter().filter(|(p, _)| *p == point).map(|(_, i)| i.as_ref()).collect()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// 槽位 ID（IF-113）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotId(pub u64);

struct LoadedPlugin {
    plugin: Box<dyn Plugin>,
}

/// 插件宿主（IF-114）：加载/卸载/依赖与版本校验/状态往返。
pub struct PluginHost {
    registry: Registry,
    loaded: Vec<LoadedPlugin>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost {
    pub fn new() -> Self {
        Self { registry: Registry::new(), loaded: Vec::new() }
    }

    /// 加载插件：依赖必须已加载；版本 ABI 兼容（同 major）。
    pub fn load(&mut self, plugin: Box<dyn Plugin>) -> Result<String> {
        let meta = plugin.metadata();
        if self.loaded.iter().any(|p| p.plugin.metadata().name == meta.name) {
            return Err(rf_core::EngineError::Message(format!(
                "plugin {} already loaded",
                meta.name
            )));
        }
        for dep in &meta.dependencies {
            if !self.loaded.iter().any(|p| &p.plugin.metadata().name == dep) {
                return Err(rf_core::EngineError::Message(format!(
                    "missing dependency `{dep}` for {}",
                    meta.name
                )));
            }
        }
        if meta.version.major > ENGINE_VERSION.major {
            return Err(rf_core::EngineError::Message(format!(
                "plugin {} ABI {} incompatible with engine {}",
                meta.name, meta.version, ENGINE_VERSION
            )));
        }
        plugin.register(&mut self.registry)?;
        let name = meta.name.clone();
        self.loaded.push(LoadedPlugin { plugin });
        Ok(name)
    }

    /// 卸载：取回状态字符串（热重载用）。
    pub fn unload(&mut self, name: &str) -> Result<Option<String>> {
        let idx =
            self.loaded.iter().position(|p| p.plugin.metadata().name == name).ok_or_else(|| {
                rf_core::EngineError::Message(format!("plugin {name} not loaded"))
            })?;
        let lp = self.loaded.remove(idx);
        Ok(lp.plugin.save_state())
    }

    pub fn registry(&mut self) -> &mut Registry {
        &mut self.registry
    }

    pub fn registry_ref(&self) -> &Registry {
        &self.registry
    }

    pub fn loaded(&self) -> Vec<String> {
        self.loaded.iter().map(|p| p.plugin.metadata().name.clone()).collect()
    }

    pub fn restore(&mut self, name: &str, state: &str) -> Result<()> {
        let lp =
            self.loaded.iter_mut().find(|p| p.plugin.metadata().name == name).ok_or_else(|| {
                rf_core::EngineError::Message(format!("plugin {name} not loaded"))
            })?;
        lp.plugin.restore_state(state);
        Ok(())
    }
}

/// 动态库加载（IF-115，feature `dynamic` 需 libloading，P1）。
pub fn load_library_plugin(_path: &str) -> Result<Box<dyn Plugin>> {
    Err(rf_core::EngineError::NotYetSupported {
        what: "dynamic library plugin (feature `dynamic` + libloading)",
        priority: "P1",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        state: String,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self { state: "fresh".into() }
        }
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "test-plugin".into(),
                version: Version::new(0, 1, 0),
                dependencies: vec![],
                ..PluginMetadata::new("test-plugin")
            }
        }
        fn register(&self, registry: &mut Registry) -> Result<()> {
            registry.add(ExtensionPoint::AssetImporter, Box::new(42u32));
            Ok(())
        }
        fn save_state(&self) -> Option<String> {
            Some(self.state.clone())
        }
        fn restore_state(&mut self, state: &str) {
            self.state = state.to_string();
        }
    }

    struct DepPlugin;
    impl Plugin for DepPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                dependencies: vec!["test-plugin".into()],
                ..PluginMetadata::new("dep-plugin")
            }
        }
        fn register(&self, _registry: &mut Registry) -> Result<()> {
            Ok(())
        }
    }

    struct FuturePlugin;
    impl Plugin for FuturePlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata { version: Version::new(99, 0, 0), ..PluginMetadata::new("future") }
        }
        fn register(&self, _registry: &mut Registry) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn load_register_unload() {
        let mut host = PluginHost::new();
        let name = host.load(Box::new(TestPlugin::new())).unwrap();
        assert_eq!(name, "test-plugin");
        assert_eq!(host.registry_ref().items(ExtensionPoint::AssetImporter).len(), 1);
        // downcast 验证
        let item = host.registry_ref().items(ExtensionPoint::AssetImporter)[0];
        assert_eq!(item.downcast_ref::<u32>(), Some(&42));
        // 重复加载拒绝
        assert!(host.load(Box::new(TestPlugin::new())).is_err());
        // 卸载取状态
        let state = host.unload("test-plugin").unwrap();
        assert_eq!(state.as_deref(), Some("fresh"));
        // 槽位保留策略：卸载后槽位不回滚（文档化行为）
    }

    #[test]
    fn dependency_check() {
        let mut host = PluginHost::new();
        assert!(host.load(Box::new(DepPlugin)).is_err()); // 缺依赖
        host.load(Box::new(TestPlugin::new())).unwrap();
        assert!(host.load(Box::new(DepPlugin)).is_ok()); // 依赖满足
    }

    #[test]
    fn abi_version_check() {
        let mut host = PluginHost::new();
        assert!(host.load(Box::new(FuturePlugin)).is_err()); // ABI 不兼容
    }

    #[test]
    fn hot_reload_state_roundtrip() {
        let mut host = PluginHost::new();
        host.load(Box::new(TestPlugin::new())).unwrap();
        let state = host.unload("test-plugin").unwrap().unwrap();
        host.load(Box::new(TestPlugin { state: String::new() })).unwrap();
        host.restore("test-plugin", &state).unwrap();
        // 状态已恢复（经 save_state 再次验证）
        let again = host.unload("test-plugin").unwrap().unwrap();
        assert_eq!(again, "fresh");
    }
}
