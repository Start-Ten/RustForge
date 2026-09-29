//! RustForge UI（IF-250 ~ IF-255）：控件树、确定性布局、显示列表、9-Slice、绑定、虚拟化。

use rf_math::Color;

/// 控件类型（IF-250，可扩展）。
#[derive(Debug, Clone)]
pub enum WidgetKind {
    Container,
    Button,
    Label { text: String },
    Image { uv: [f32; 4] },
    NineSlice { insets: (f32, f32, f32, f32) },
    Slider { min: f32, max: f32, value: f32 },
    Checkbox { checked: bool },
    TextField { placeholder: String },
    Panel,
    List { items: usize, item_h: f32, scroll: f32 },
}

/// 布局（IF-251，可扩展）。
#[derive(Debug, Clone)]
pub enum Layout {
    StackV { spacing: f32, padding: f32 },
    StackH { spacing: f32, padding: f32 },
    Grid { cols: usize, spacing: f32 },
    Overlay,
    Absolute,
}

/// 样式（IF-251，可扩展）。
#[derive(Debug, Clone)]
pub struct Style {
    pub background: Option<Color>,
    pub border: Option<(f32, Color)>,
    pub text_color: Color,
    pub font_size: f32,
    pub corner: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            border: None,
            text_color: Color::WHITE,
            font_size: 14.0,
            corner: 0.0,
        }
    }
}

/// UI 节点（IF-250）。
#[derive(Debug, Clone)]
pub struct UiNode {
    pub id: u64,
    pub kind: WidgetKind,
    pub layout: Layout,
    pub style: Style,
    pub bind: Option<String>,
    pub children: Vec<UiNode>,
    /// 布局产物（像素矩形 x,y,w,h）。
    pub rect: (f32, f32, f32, f32),
}

impl UiNode {
    pub fn new(id: u64, kind: WidgetKind, layout: Layout) -> Self {
        Self {
            id,
            kind,
            layout,
            style: Style::default(),
            bind: None,
            children: Vec::new(),
            rect: (0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }
}

/// 确定性布局（IF-252）。
pub fn layout_tree(node: &mut UiNode, avail: (f32, f32)) {
    node.rect = (0.0, 0.0, avail.0, avail.1);
    layout_children(node, avail);
}

fn measure(node: &UiNode) -> (f32, f32) {
    match &node.kind {
        WidgetKind::Label { text } => {
            (text.chars().count() as f32 * node.style.font_size * 0.6, node.style.font_size * 1.4)
        }
        WidgetKind::Button => (80.0, 28.0),
        WidgetKind::Slider { .. } => (120.0, 20.0),
        WidgetKind::Checkbox { .. } => (18.0, 18.0),
        WidgetKind::TextField { .. } => (140.0, 24.0),
        _ => (node.rect.2, node.rect.3),
    }
}

fn layout_children(node: &mut UiNode, avail: (f32, f32)) {
    match &node.layout {
        Layout::StackV { spacing, padding } => {
            let mut y = *padding;
            let inner_w = avail.0 - padding * 2.0;
            let _ = inner_w;
            for child in &mut node.children {
                let size = measure(child);
                child.rect = (*padding, y, (avail.0 - padding * 2.0).max(0.0), size.1);
                y += size.1 + spacing;
            }
        }
        Layout::StackH { spacing, padding } => {
            let mut x = *padding;
            for child in &mut node.children {
                let size = measure(child);
                child.rect = (x, *padding, size.0, (avail.1 - padding * 2.0).max(0.0));
                x += size.0 + spacing;
            }
        }
        Layout::Grid { cols, spacing } => {
            let cols = (*cols).max(1);
            let cw = (avail.0 - spacing * (cols - 1) as f32) / cols as f32;
            for (i, child) in node.children.iter_mut().enumerate() {
                let col = (i % cols) as f32;
                let row = (i / cols) as f32;
                let size = measure(child);
                child.rect = (col * (cw + spacing), row * (size.1 + spacing), cw, size.1);
            }
        }
        Layout::Overlay => {
            for child in &mut node.children {
                child.rect = (0.0, 0.0, avail.0, avail.1);
            }
        }
        Layout::Absolute => {
            // 保留 children 已有 rect（绝对定位）
        }
    }
    for child in &mut node.children {
        layout_children(child, (child.rect.2, child.rect.3));
    }
}

/// 命中测试（IF-252）：最深命中节点 id。
pub fn hit_test(node: &UiNode, x: f32, y: f32) -> Option<u64> {
    let (rx, ry, rw, rh) = node.rect;
    if x < rx || y < ry || x > rx + rw || y > ry + rh {
        return None;
    }
    for child in &node.children {
        if let Some(id) = hit_test(child, x - rx, y - ry) {
            return Some(id);
        }
    }
    Some(node.id)
}

/// 显示列表命令（IF-253）。
#[derive(Debug, Clone)]
pub enum UiCommand {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color,
        corner: f32,
    },
    NineSlice {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv: [f32; 4],
        insets: (f32, f32, f32, f32),
        color: Color,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: Color,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        thick: f32,
    },
}

/// 构建显示列表（IF-253）。
pub fn build_draw_list(node: &UiNode, out: &mut Vec<UiCommand>) {
    let (x, y, w, h) = node.rect;
    if let Some(bg) = node.style.background {
        out.push(UiCommand::Rect { x, y, w, h, color: bg, corner: node.style.corner });
    }
    if let Some((thick, color)) = node.style.border {
        out.push(UiCommand::Line { x1: x, y1: y, x2: x + w, y2: y, color, thick });
        out.push(UiCommand::Line { x1: x, y1: y + h, x2: x + w, y2: y + h, color, thick });
        out.push(UiCommand::Line { x1: x, y1: y, x2: x, y2: y + h, color, thick });
        out.push(UiCommand::Line { x1: x + w, y1: y, x2: x + w, y2: y + h, color, thick });
    }
    match &node.kind {
        WidgetKind::Label { text } => out.push(UiCommand::Text {
            x: x + 4.0,
            y: y + 4.0,
            text: text.clone(),
            size: node.style.font_size,
            color: node.style.text_color,
        }),
        WidgetKind::Button => out.push(UiCommand::Rect {
            x: x + 2.0,
            y: y + 2.0,
            w: (w - 4.0).max(0.0),
            h: (h - 4.0).max(0.0),
            color: node.style.background.unwrap_or(Color::rgb(0.3, 0.4, 0.6)),
            corner: 4.0,
        }),
        WidgetKind::NineSlice { insets } => out.push(UiCommand::NineSlice {
            x,
            y,
            w,
            h,
            uv: [0.0, 0.0, 1.0, 1.0],
            insets: *insets,
            color: Color::WHITE,
        }),
        _ => {}
    }
    for child in &node.children {
        build_draw_list(child, out);
    }
}

/// 9-Slice 目标矩形集（绘制辅助，L5）。
pub type Rect4 = (f32, f32, f32, f32);

#[allow(clippy::type_complexity)]
pub fn nine_slice_rects(dst: Rect4, insets: (f32, f32, f32, f32)) -> [(Rect4, Rect4); 9] {
    let (x, y, w, h) = dst;
    let (il, ir, it, ib) = insets;
    let cx = (w - il - ir).max(0.0);
    let cy = (h - it - ib).max(0.0);
    let iu = 1.0 / 3.0;
    [
        ((x, y, il, it), (0.0, 0.0, iu, iu)),
        ((x + il, y, cx, it), (iu, 0.0, iu, iu)),
        ((x + il + cx, y, ir, it), (2.0 * iu, 0.0, iu, iu)),
        ((x, y + it, il, cy), (0.0, iu, iu, iu)),
        ((x + il, y + it, cx, cy), (iu, iu, iu, iu)),
        ((x + il + cx, y + it, ir, cy), (2.0 * iu, iu, iu, iu)),
        ((x, y + it + cy, il, ib), (0.0, 2.0 * iu, iu, iu)),
        ((x + il, y + it + cy, cx, ib), (iu, 2.0 * iu, iu, iu)),
        ((x + il + cx, y + it + cy, ir, ib), (2.0 * iu, 2.0 * iu, iu, iu)),
    ]
}

/// UI 绑定值（IF-254；独立于 rf-ai 黑板以保持依赖方向冻结）。
#[derive(Debug, Clone, PartialEq)]
pub enum UiBindValue {
    Bool(bool),
    F32(f32),
    I64(i64),
    Str(String),
}

/// 绑定存储（IF-254）。
pub struct BindingStore {
    values: std::collections::HashMap<String, UiBindValue>,
    snapshot: std::collections::HashMap<String, UiBindValue>,
}

impl Default for BindingStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BindingStore {
    pub fn new() -> Self {
        Self { values: Default::default(), snapshot: Default::default() }
    }

    pub fn set(&mut self, name: &str, v: UiBindValue) {
        self.values.insert(name.to_string(), v);
    }

    pub fn get(&self, name: &str) -> Option<&UiBindValue> {
        self.values.get(name)
    }

    /// 自上次调用以来变更的绑定名。
    pub fn changed(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, v) in self.values.iter() {
            if self.snapshot.get(k) != Some(v) {
                out.push(k.clone());
            }
        }
        self.snapshot = self.values.clone();
        out
    }
}

/// 应用绑定：Label 文本跟随绑定值。
pub fn apply_bindings(node: &mut UiNode, store: &BindingStore) {
    if let Some(name) = &node.bind {
        if let Some(v) = store.get(name) {
            if let WidgetKind::Label { text } = &mut node.kind {
                *text = match v {
                    UiBindValue::Str(s) => s.clone(),
                    other => format!("{other:?}"),
                };
            }
        }
    }
    for child in &mut node.children {
        apply_bindings(child, store);
    }
}

/// 列表虚拟化窗口（IF-255）。
pub fn list_visible_range(items: usize, item_h: f32, scroll: f32, viewport: f32) -> (usize, usize) {
    if items == 0 || item_h <= 0.0 {
        return (0, 0);
    }
    let start = ((scroll / item_h).floor().max(0.0) as usize).min(items);
    let visible = (viewport / item_h).ceil() as usize + 1;
    let end = (start + visible).min(items);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_layout_deterministic() {
        let mut root =
            UiNode::new(1, WidgetKind::Container, Layout::StackV { spacing: 4.0, padding: 8.0 })
                .with_child(UiNode::new(2, WidgetKind::Button, Layout::Overlay))
                .with_child(UiNode::new(3, WidgetKind::Button, Layout::Overlay));
        layout_tree(&mut root, (200.0, 100.0));
        assert_eq!(root.children[0].rect, (8.0, 8.0, 184.0, 28.0));
        assert_eq!(root.children[1].rect, (8.0, 40.0, 184.0, 28.0)); // y 累进
                                                                     // 布局确定性：重复布局结果一致
        let first = format!("{:?}", root.children[0].rect);
        layout_tree(&mut root, (200.0, 100.0));
        assert_eq!(format!("{:?}", root.children[0].rect), first);
    }

    #[test]
    fn horizontal_and_grid() {
        let mut root =
            UiNode::new(1, WidgetKind::Container, Layout::StackH { spacing: 2.0, padding: 0.0 })
                .with_child(UiNode::new(2, WidgetKind::Button, Layout::Overlay))
                .with_child(UiNode::new(3, WidgetKind::Button, Layout::Overlay));
        layout_tree(&mut root, (300.0, 100.0));
        assert!((root.children[0].rect.0 - 0.0).abs() < 1e-4);
        assert!((root.children[1].rect.0 - 82.0).abs() < 1e-4);
        let mut grid =
            UiNode::new(1, WidgetKind::Container, Layout::Grid { cols: 2, spacing: 0.0 });
        for i in 0..4 {
            grid = grid.with_child(UiNode::new(
                10 + i,
                WidgetKind::Checkbox { checked: false },
                Layout::Overlay,
            ));
        }
        layout_tree(&mut grid, (100.0, 100.0));
        assert!((grid.children[0].rect.0 - 0.0).abs() < 1e-4);
        assert!((grid.children[1].rect.0 - 50.0).abs() < 1e-4);
        assert!((grid.children[2].rect.0 - 0.0).abs() < 1e-4);
    }

    #[test]
    fn hit_test_deep() {
        let mut root =
            UiNode::new(1, WidgetKind::Panel, Layout::StackV { spacing: 0.0, padding: 0.0 })
                .with_child(UiNode::new(2, WidgetKind::Button, Layout::Overlay));
        layout_tree(&mut root, (100.0, 100.0));
        // 按钮区域（第一行 0..28）
        assert_eq!(hit_test(&root, 50.0, 10.0), Some(2));
        // 面板空白区
        assert_eq!(hit_test(&root, 50.0, 90.0), Some(1));
        // 界外
        assert_eq!(hit_test(&root, 150.0, 10.0), None);
    }

    #[test]
    fn draw_list_contains_commands() {
        let mut root = UiNode::new(1, WidgetKind::Container, Layout::Overlay)
            .with_style(Style { background: Some(Color::rgb(0.2, 0.2, 0.2)), ..Default::default() })
            .with_child(UiNode::new(2, WidgetKind::Label { text: "Hi".into() }, Layout::Overlay));
        layout_tree(&mut root, (10.0, 10.0));
        let mut out = Vec::new();
        build_draw_list(&root, &mut out);
        assert!(out.iter().any(|c| matches!(c, UiCommand::Rect { .. })));
        assert!(out.iter().any(|c| matches!(c, UiCommand::Text { text, .. } if text == "Hi")));
    }

    #[test]
    fn nine_slice_geometry() {
        let rects = nine_slice_rects((0.0, 0.0, 100.0, 60.0), (10.0, 10.0, 8.0, 8.0));
        // 角块
        assert_eq!((rects[0].0 .2, rects[0].0 .3), (10.0, 8.0));
        // 中心块扩展
        assert_eq!((rects[4].0 .2, rects[4].0 .3), (80.0, 44.0));
        // uv 总和为 1
        assert!((rects[8].1 .0 + rects[8].1 .2 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn bindings_update_labels() {
        let mut store = BindingStore::new();
        store.set("score", UiBindValue::Str("42".into()));
        let mut ui = UiNode::new(1, WidgetKind::Container, Layout::Overlay);
        let mut label = UiNode::new(2, WidgetKind::Label { text: "-".into() }, Layout::Overlay);
        label.bind = Some("score".into());
        ui.children.push(label);
        apply_bindings(&mut ui, &store);
        match &ui.children[0].kind {
            WidgetKind::Label { text } => assert_eq!(text, "42"),
            _ => panic!(),
        }
        // 变更检测
        let changed = store.changed();
        assert!(changed.contains(&"score".to_string()));
        assert!(store.changed().is_empty()); // 第二次无变化
        store.set("score", UiBindValue::Str("43".into()));
        assert!(store.changed().contains(&"score".to_string()));
    }

    #[test]
    fn list_virtualization_window() {
        assert_eq!(list_visible_range(1000, 20.0, 0.0, 100.0), (0, 6)); // 视口 5 行 + 1
        assert_eq!(list_visible_range(1000, 20.0, 200.0, 100.0), (10, 16));
        assert_eq!(list_visible_range(10, 20.0, 999.0, 100.0), (10, 10)); // 越界钳制
        assert_eq!(list_visible_range(0, 20.0, 0.0, 100.0), (0, 0));
    }
}
