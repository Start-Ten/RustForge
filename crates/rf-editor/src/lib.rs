//! RustForge 编辑器（IF-280 ~ IF-290）：面板模型、撤销重做、命令面板、主题、布局。

use rf_asset::AssetId;
use rf_core::{Entity, Result};
use rf_ecs::World;
use rf_math::Color;
use std::collections::HashMap;

/// 编辑模式（IF-280）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    TwoD,
    ThreeD,
    Mixed,
}

/// 面板类型（IF-280，可扩展）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PanelKind {
    Viewport,
    Hierarchy,
    Properties,
    AssetBrowser,
    Console,
    Profiler,
    CommandPalette,
    Custom(&'static str),
}

/// 选择集（IF-281）。
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub entities: Vec<Entity>,
    pub assets: Vec<AssetId>,
}

impl Selection {
    pub fn is_entity_selected(&self, e: Entity) -> bool {
        self.entities.contains(&e)
    }

    pub fn toggle_entity(&mut self, e: Entity) {
        if let Some(i) = self.entities.iter().position(|x| *x == e) {
            self.entities.remove(i);
        } else {
            self.entities.push(e);
        }
    }

    pub fn select_one(&mut self, e: Entity) {
        self.entities.clear();
        self.entities.push(e);
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.assets.clear();
    }
}

/// 编辑器上下文（IF-282）。
pub struct EditorContext<'a> {
    pub world: &'a mut World,
    pub mode: &'a mut EditorMode,
    pub selection: &'a mut Selection,
    pub undo: &'a mut UndoStack,
    pub log: &'a mut Vec<String>,
}

/// 面板 trait（IF-283，可扩展）。
pub trait EditorPanel: Send {
    fn kind(&self) -> PanelKind;
    fn title(&self) -> String;
    fn update(&mut self, ctx: &mut EditorContext) -> Result<()>;
}

// ---- 撤销重做（IF-284） ----

type WorldOp = Box<dyn Fn(&mut World) + Send>;

/// 撤销条目：label + (do, undo) 闭包对。
struct UndoEntry {
    label: &'static str,
    do_fn: WorldOp,
    undo_fn: WorldOp,
}

/// 撤销栈（IF-284）。
pub struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    pub fn new() -> Self {
        Self { undo: Vec::new(), redo: Vec::new() }
    }

    /// 推入编辑（do_fn 由调用方先行应用，此处仅记录）。
    pub fn push(&mut self, label: &'static str, do_fn: WorldOp, undo_fn: WorldOp) {
        self.undo.push(UndoEntry { label, do_fn, undo_fn });
        self.redo.clear();
    }

    pub fn undo(&mut self, world: &mut World) -> Option<&'static str> {
        let entry = self.undo.pop()?;
        (entry.undo_fn)(world);
        let label = entry.label;
        self.redo.push(entry);
        Some(label)
    }

    pub fn redo(&mut self, world: &mut World) -> Option<&'static str> {
        let entry = self.redo.pop()?;
        (entry.do_fn)(world);
        let label = entry.label;
        self.undo.push(entry);
        Some(label)
    }

    pub fn depth(&self) -> usize {
        self.undo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

// ---- 命令面板（IF-285） ----

type PaletteFn = Box<dyn Fn(&mut EditorContext) + Send>;

struct PaletteCommand {
    name: String,
    #[allow(dead_code)]
    shortcut: Option<String>,
    run: PaletteFn,
}

/// 命令面板：模糊搜索 + 执行。
pub struct CommandPalette {
    commands: Vec<PaletteCommand>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn register(&mut self, name: &str, shortcut: Option<&str>, run: PaletteFn) {
        self.commands.push(PaletteCommand {
            name: name.to_string(),
            shortcut: shortcut.map(str::to_string),
            run,
        });
    }

    /// 模糊匹配评分：子序列匹配（连续加分）。
    fn score(name: &str, query: &str) -> i32 {
        let query = query.to_lowercase();
        if query.is_empty() {
            return 0;
        }
        let name_lower = name.to_lowercase();
        if name_lower.contains(&query) {
            return 1000 - name.len() as i32;
        }
        // 子序列
        let mut qi = query.chars().peekable();
        let mut score = 0;
        for c in name_lower.chars() {
            if qi.peek() == Some(&c) {
                qi.next();
                score += 10;
            }
        }
        if qi.peek().is_none() {
            score
        } else {
            -1
        }
    }

    pub fn search(&self, query: &str) -> Vec<String> {
        let mut scored: Vec<(i32, &str)> = self
            .commands
            .iter()
            .map(|c| (Self::score(&c.name, query), c.name.as_str()))
            .filter(|(s, _)| *s >= 0)
            .collect();
        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
        scored.into_iter().map(|(_, n)| n.to_string()).collect()
    }

    pub fn execute(&mut self, name: &str, ctx: &mut EditorContext) -> bool {
        let Some(idx) = self.commands.iter().position(|c| c.name == name) else { return false };
        let cmd = &mut self.commands[idx];
        (cmd.run)(ctx);
        true
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

// ---- 主题（IF-286） ----

/// 主题。
pub struct Theme {
    pub name: String,
    pub colors: HashMap<String, Color>,
}

impl Theme {
    pub fn dark() -> Self {
        let mut colors = HashMap::new();
        colors.insert("background".into(), Color::rgb(0.11, 0.11, 0.13));
        colors.insert("panel".into(), Color::rgb(0.16, 0.16, 0.19));
        colors.insert("text".into(), Color::rgb(0.9, 0.9, 0.92));
        colors.insert("accent".into(), Color::rgb(0.26, 0.59, 0.98));
        colors.insert("warning".into(), Color::rgb(0.95, 0.77, 0.26));
        Self { name: "dark".into(), colors }
    }

    pub fn light() -> Self {
        let mut colors = HashMap::new();
        colors.insert("background".into(), Color::rgb(0.94, 0.94, 0.95));
        colors.insert("panel".into(), Color::rgb(1.0, 1.0, 1.0));
        colors.insert("text".into(), Color::rgb(0.12, 0.12, 0.14));
        colors.insert("accent".into(), Color::rgb(0.15, 0.46, 0.85));
        colors.insert("warning".into(), Color::rgb(0.83, 0.63, 0.05));
        Self { name: "light".into(), colors }
    }

    pub fn color(&self, name: &str) -> Color {
        self.colors.get(name).copied().unwrap_or(Color::MAGENTA)
    }
}

// ---- 布局（IF-287） ----

/// 停靠布局（列分组，简化模型）。
#[derive(Debug, Clone, Default)]
pub struct DockLayout {
    pub columns: Vec<Vec<PanelKind>>,
}

impl DockLayout {
    pub fn set_columns(&mut self, columns: Vec<Vec<PanelKind>>) {
        self.columns = columns;
    }

    pub fn columns(&self) -> &[Vec<PanelKind>] {
        &self.columns
    }

    /// 序列化（自定义紧凑格式；JSON 走 rf-serialization）。
    pub fn to_json(&self) -> String {
        let names: Vec<String> = self
            .columns
            .iter()
            .map(|col| format!("[{}]", col.iter().map(kind_name).collect::<Vec<_>>().join(",")))
            .collect();
        format!("{{\"columns\":[{}]}}", names.join(","))
    }

    /// 损坏输入回退默认布局（边界条件，规格 J1）。
    pub fn from_json(json: &str) -> Self {
        let mut out = Self::default();
        let mut columns = Vec::new();
        let mut current: Option<Vec<PanelKind>> = None;
        let mut chars = json.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '[' => current = Some(Vec::new()),
                ']' => {
                    if let Some(col) = current.take() {
                        columns.push(col);
                    }
                }
                c if c.is_ascii_alphabetic() => {
                    let mut word = String::new();
                    word.push(c);
                    while let Some(&n) = chars.peek() {
                        if n.is_ascii_alphabetic() {
                            word.push(n);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Some(k) = kind_from_name(&word) {
                        if let Some(col) = current.as_mut() {
                            col.push(k);
                        }
                    }
                }
                _ => {}
            }
        }
        if !columns.is_empty() {
            out.columns = columns;
        } else {
            out.reset();
        }
        out
    }

    pub fn reset(&mut self) {
        self.columns = vec![
            vec![PanelKind::Hierarchy],
            vec![PanelKind::Viewport],
            vec![PanelKind::Properties, PanelKind::AssetBrowser],
        ];
    }
}

fn kind_name(k: &PanelKind) -> &'static str {
    match k {
        PanelKind::Viewport => "Viewport",
        PanelKind::Hierarchy => "Hierarchy",
        PanelKind::Properties => "Properties",
        PanelKind::AssetBrowser => "AssetBrowser",
        PanelKind::Console => "Console",
        PanelKind::Profiler => "Profiler",
        PanelKind::CommandPalette => "CommandPalette",
        PanelKind::Custom(_) => "Custom",
    }
}

fn kind_from_name(s: &str) -> Option<PanelKind> {
    match s {
        "Viewport" => Some(PanelKind::Viewport),
        "Hierarchy" => Some(PanelKind::Hierarchy),
        "Properties" => Some(PanelKind::Properties),
        "AssetBrowser" => Some(PanelKind::AssetBrowser),
        "Console" => Some(PanelKind::Console),
        "Profiler" => Some(PanelKind::Profiler),
        "CommandPalette" => Some(PanelKind::CommandPalette),
        _ => None,
    }
}

// ---- 编辑器模型（IF-288） ----

/// 编辑器状态模型。
pub struct EditorModel {
    pub mode: EditorMode,
    pub selection: Selection,
    pub undo: UndoStack,
    pub palette: CommandPalette,
    pub theme: Theme,
    pub layout: DockLayout,
    pub dirty: bool,
    pub autosave_path: Option<String>,
}

impl EditorModel {
    pub fn new(mode: EditorMode) -> Self {
        Self {
            mode,
            selection: Selection::default(),
            undo: UndoStack::new(),
            palette: CommandPalette::new(),
            theme: Theme::dark(),
            layout: DockLayout::default(),
            dirty: false,
            autosave_path: None,
        }
    }

    /// 一键切换 2D/3D 模式（J1 验收）。
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditorMode::TwoD => EditorMode::ThreeD,
            EditorMode::ThreeD | EditorMode::Mixed => EditorMode::TwoD,
        };
    }

    pub fn update(&mut self, panels: &mut [Box<dyn EditorPanel>], world: &mut World) -> Result<()> {
        let mut log = Vec::new();
        {
            let mut ctx = EditorContext {
                world,
                mode: &mut self.mode,
                selection: &mut self.selection,
                undo: &mut self.undo,
                log: &mut log,
            };
            for p in panels.iter_mut() {
                p.update(&mut ctx)?;
            }
        }
        if !log.is_empty() {
            self.dirty = true;
        }
        Ok(())
    }

    pub fn save_state(&self) -> String {
        format!(
            "{{\"mode\":\"{:?}\",\"dirty\":{},\"layout\":{}}}",
            self.mode,
            self.dirty,
            self.layout.to_json()
        )
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

// ---- 属性网格（IF-289） ----

/// 属性行（反射驱动）。
pub struct PropertyRow {
    pub label: String,
    pub value: rf_reflection::FieldValue,
    pub meta: rf_reflection::FieldMeta,
}

/// 由反射描述符生成属性网格。
pub fn build_property_grid(value: &dyn rf_reflection::Reflect) -> Vec<PropertyRow> {
    let desc = value.reflect_descriptor();
    desc.fields
        .iter()
        .filter(|f| !f.meta.hidden)
        .map(|f| PropertyRow {
            label: if f.meta.display_name.is_empty() {
                f.name.to_string()
            } else {
                f.meta.display_name.to_string()
            },
            value: rf_reflection::read_field(value, f.name)
                .unwrap_or(rf_reflection::FieldValue::Opaque),
            meta: f.meta.clone(),
        })
        .collect()
}

// ---- 内置面板（IF-289） ----

/// 层级面板：列出实体与父子关系（写入 log 供 UI 层读取）。
pub struct HierarchyPanel;

impl EditorPanel for HierarchyPanel {
    fn kind(&self) -> PanelKind {
        PanelKind::Hierarchy
    }
    fn title(&self) -> String {
        "Hierarchy".into()
    }
    fn update(&mut self, ctx: &mut EditorContext) -> Result<()> {
        for e in ctx.world.entities() {
            let label = match ctx.world.get::<crate::NameComponent>(e) {
                Some(n) => format!("{e} {}", n.0),
                None => format!("{e}"),
            };
            ctx.log.push(format!("HIERARCHY {label}"));
        }
        Ok(())
    }
}

/// 属性面板：反射网格（所选实体）。
pub struct PropertiesPanel;

impl EditorPanel for PropertiesPanel {
    fn kind(&self) -> PanelKind {
        PanelKind::Properties
    }
    fn title(&self) -> String {
        "Properties".into()
    }
    fn update(&mut self, ctx: &mut EditorContext) -> Result<()> {
        for e in ctx.selection.entities.clone() {
            if let Some(p) = ctx.world.get::<crate::PositionComponent>(e) {
                ctx.log.push(format!("PROPS {e} position=({:.2}, {:.2})", p.0.x, p.0.y));
            }
        }
        Ok(())
    }
}

/// 资产浏览器面板（IF-289：已加载资产列表）。
pub struct AssetBrowserPanel;

impl EditorPanel for AssetBrowserPanel {
    fn kind(&self) -> PanelKind {
        PanelKind::AssetBrowser
    }
    fn title(&self) -> String {
        "Assets".into()
    }
    fn update(&mut self, _ctx: &mut EditorContext) -> Result<()> {
        Ok(())
    }
}

/// 控制台面板。
pub struct ConsolePanel;

impl EditorPanel for ConsolePanel {
    fn kind(&self) -> PanelKind {
        PanelKind::Console
    }
    fn title(&self) -> String {
        "Console".into()
    }
    fn update(&mut self, _ctx: &mut EditorContext) -> Result<()> {
        Ok(())
    }
}

/// 性能面板。
pub struct ProfilerPanel;

impl EditorPanel for ProfilerPanel {
    fn kind(&self) -> PanelKind {
        PanelKind::Profiler
    }
    fn title(&self) -> String {
        "Profiler".into()
    }
    fn update(&mut self, _ctx: &mut EditorContext) -> Result<()> {
        Ok(())
    }
}

/// 视口面板（2D/3D 由 mode 决定，标题联动）。
pub struct ViewportPanel {
    last_mode: Option<EditorMode>,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewportPanel {
    pub fn new() -> Self {
        Self { last_mode: None }
    }
}

impl EditorPanel for ViewportPanel {
    fn kind(&self) -> PanelKind {
        PanelKind::Viewport
    }
    fn title(&self) -> String {
        match self.last_mode {
            Some(m) => format!("Viewport ({m:?})"),
            None => "Viewport".into(),
        }
    }
    fn update(&mut self, ctx: &mut EditorContext) -> Result<()> {
        self.last_mode = Some(*ctx.mode);
        Ok(())
    }
}

/// 命名组件（层级面板显示）。
#[derive(rf_ecs::Component, Debug)]
pub struct NameComponent(pub String);

/// 位置组件（属性面板演示）。
#[derive(rf_ecs::Component, Debug)]
pub struct PositionComponent(pub rf_math::Vec2);

/// 编辑器应用（IF-290）：面板集合 + 模型 + headless 可测帧循环。
pub struct EditorApp {
    pub model: EditorModel,
    panels: Vec<Box<dyn EditorPanel>>,
    frame_count: u64,
}

impl EditorApp {
    pub fn new(mode: EditorMode) -> Self {
        let mut model = EditorModel::new(mode);
        model.layout.reset();
        model.palette.register(
            "toggle_mode",
            Some("Ctrl+M"),
            Box::new(|ctx| {
                *ctx.mode = match *ctx.mode {
                    EditorMode::TwoD => EditorMode::ThreeD,
                    EditorMode::ThreeD | EditorMode::Mixed => EditorMode::TwoD,
                };
            }),
        );
        model.palette.register(
            "clear_selection",
            Some("Ctrl+D"),
            Box::new(|ctx| ctx.selection.clear()),
        );
        Self {
            model,
            panels: vec![
                Box::new(HierarchyPanel),
                Box::new(PropertiesPanel),
                Box::new(AssetBrowserPanel),
                Box::new(ConsolePanel),
                Box::new(ProfilerPanel),
                Box::new(ViewportPanel::new()),
            ],
            frame_count: 0,
        }
    }

    /// headless 帧推进（测试/CI）。
    pub fn run_frame(&mut self, world: &mut World) -> Result<u64> {
        self.model.update(&mut self.panels, world)?;
        self.frame_count += 1;
        Ok(self.frame_count)
    }

    pub fn model(&self) -> &EditorModel {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut EditorModel {
        &mut self.model
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_roundtrip() {
        let mut world = World::new();
        let e = world.spawn((PositionComponent(rf_math::Vec2::new(1.0, 2.0)),));
        let mut undo = UndoStack::new();
        let old = world.get::<PositionComponent>(e).unwrap().0;
        world.get_mut::<PositionComponent>(e).unwrap().0 = rf_math::Vec2::new(5.0, 5.0);
        undo.push(
            "move",
            Box::new(move |w| {
                w.get_mut::<PositionComponent>(e).unwrap().0 = rf_math::Vec2::new(5.0, 5.0);
            }),
            Box::new(move |w| {
                w.get_mut::<PositionComponent>(e).unwrap().0 = old;
            }),
        );
        assert_eq!(world.get::<PositionComponent>(e).unwrap().0, rf_math::Vec2::new(5.0, 5.0));
        assert_eq!(undo.undo(&mut world), Some("move"));
        assert_eq!(world.get::<PositionComponent>(e).unwrap().0, rf_math::Vec2::new(1.0, 2.0));
        assert!(undo.can_redo());
        assert_eq!(undo.redo(&mut world), Some("move"));
        assert_eq!(world.get::<PositionComponent>(e).unwrap().0, rf_math::Vec2::new(5.0, 5.0));
    }

    #[test]
    fn palette_and_layout() {
        let mut world = World::new();
        let mut app = EditorApp::new(EditorMode::TwoD);
        let results = app.model.palette.search("tog");
        assert!(results.contains(&"toggle_mode".to_string()));
        let mut log = Vec::new();
        {
            let mut sel = Selection::default();
            let mut mode = app.model.mode;
            let mut undo = UndoStack::new();
            let mut ctx = EditorContext {
                world: &mut world,
                mode: &mut mode,
                selection: &mut sel,
                undo: &mut undo,
                log: &mut log,
            };
            assert!(app.model.palette.execute("toggle_mode", &mut ctx));
            assert_eq!(mode, EditorMode::ThreeD);
        }
        let json = app.model.layout.to_json();
        let restored = DockLayout::from_json(&json);
        assert_eq!(restored.columns.len(), app.model.layout.columns.len());
        let bad = DockLayout::from_json("(((garbage");
        assert!(!bad.columns.is_empty());
    }

    #[test]
    fn editor_app_frames_and_panels() {
        let mut world = World::new();
        let e = world.spawn((
            NameComponent("Player".into()),
            PositionComponent(rf_math::Vec2::new(3.0, 4.0)),
        ));
        let mut app = EditorApp::new(EditorMode::ThreeD);
        app.model.selection.select_one(e);
        let n = app.run_frame(&mut world).unwrap();
        assert_eq!(n, 1);
        assert_eq!(app.frame_count(), 1);
        app.model.toggle_mode();
        assert_eq!(app.model.mode, EditorMode::TwoD);
    }

    #[test]
    fn themes_contrast() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert!(dark.color("background").r < light.color("background").r);
    }

    #[test]
    fn selection_ops() {
        let mut s = Selection::default();
        let e1 = Entity::new(1, 1);
        let e2 = Entity::new(2, 1);
        s.toggle_entity(e1);
        assert!(s.is_entity_selected(e1));
        s.toggle_entity(e1);
        assert!(!s.is_entity_selected(e1));
        s.select_one(e2);
        assert!(!s.is_entity_selected(e1));
        assert!(s.is_entity_selected(e2));
        s.clear();
        assert!(s.entities.is_empty());
    }
}
