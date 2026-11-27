//! Qt Designer `.ui` XML → widget specification tree.
//!
//! Only the subset used by the platform's forms is parsed: widgets,
//! layouts (`QVBoxLayout`/`QHBoxLayout`/`QGridLayout`/`QFormLayout`),
//! spacers and the common property value tags (`string`, `number`, `double`,
//! `bool`, `enum`, `rect`, `size`, `set`, `sizepolicy`). Unknown constructs are
//! tolerated rather than fatal.

use roxmltree::Node;

/// Single property value, typed by the Qt XML tag that carried it.
///
/// `Map` is produced by `<iconset>` and `<sizepolicy>` and also carries the
/// JSON objects the server pushes (e.g. `tableHeaders`/`tableData`).
/// `List` is produced by `<set>` (Qt window flags) and can also arrive from
/// the server.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Str(String),
    Int(i64),
    Bool(bool),
    Rect { x: i64, y: i64, w: i64, h: i64 },
    Size { w: i64, h: i64 },
    Enum(String),
    Map(Vec<(String, PropValue)>),
    List(Vec<PropValue>),
}

impl PropValue {
    /// Value for `key` when this is a [`PropValue::Map`].
    pub fn get(&self, key: &str) -> Option<&PropValue> {
        match self {
            PropValue::Map(entries) => entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Items when this is a [`PropValue::List`].
    pub fn as_list(&self) -> Option<&[PropValue]> {
        match self {
            PropValue::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            PropValue::Str(s) | PropValue::Enum(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PropValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Scalars rendered as text (table cell values follow Qt's
    /// `QVariant::toString`); `None` for composite values.
    pub fn as_cell_text(&self) -> Option<String> {
        match self {
            PropValue::Str(s) | PropValue::Enum(s) => Some(s.clone()),
            PropValue::Int(i) => Some(i.to_string()),
            PropValue::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
}

/// `<action name="...">` definition, used by `QMainWindow` menus/toolbars.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    pub name: String,
    pub properties: Vec<(String, PropValue)>,
}

impl Action {
    pub fn property(&self, name: &str) -> Option<&PropValue> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    pub class: String,
    pub name: String,
    pub properties: Vec<(String, PropValue)>,
    /// `<attribute name="…">` values (e.g. QTabWidget page `title`,
    /// QToolBox page `label`).
    pub attributes: Vec<(String, PropValue)>,
    /// Direct child widgets placed by explicit geometry (no layout).
    pub children: Vec<Widget>,
    /// The widget's single layout, if it has one (Qt allows at most one).
    pub layout: Option<Layout>,
    /// `<action>` definitions owned by this widget (usually `QMainWindow`).
    pub actions: Vec<Action>,
    /// `<addaction name="...">` references in document order.
    pub add_actions: Vec<String>,
    /// `<column>` captions (QTableWidget/QTreeWidget header sections).
    pub columns: Vec<String>,
}

impl Widget {
    pub fn property(&self, name: &str) -> Option<&PropValue> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn attribute(&self, name: &str) -> Option<&PropValue> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }
}

/// `QFormLayout::FieldGrowthPolicy` (0.13.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldGrowthPolicy {
    #[default]
    AllNonFixedFieldsGrow,
    FieldsStayAtSizeHint,
    ExpandingFieldsGrow,
}

impl FieldGrowthPolicy {
    fn from_name(name: &str) -> Option<Self> {
        match name.rsplit("::").next().unwrap_or(name) {
            "AllNonFixedFieldsGrow" => Some(Self::AllNonFixedFieldsGrow),
            "FieldsStayAtSizeHint" => Some(Self::FieldsStayAtSizeHint),
            "ExpandingFieldsGrow" => Some(Self::ExpandingFieldsGrow),
            _ => None,
        }
    }
}

/// `QFormLayout::RowWrapPolicy` (0.13.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowWrapPolicy {
    #[default]
    DontWrapRows,
    WrapLongRows,
    WrapAllRows,
}

impl RowWrapPolicy {
    fn from_name(name: &str) -> Option<Self> {
        match name.rsplit("::").next().unwrap_or(name) {
            "DontWrapRows" => Some(Self::DontWrapRows),
            "WrapLongRows" => Some(Self::WrapLongRows),
            "WrapAllRows" => Some(Self::WrapAllRows),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// `QVBoxLayout` / `QHBoxLayout` / ...
    pub class: String,
    pub name: String,
    /// `spacing` property if set explicitly; `None` → Qt default (6).
    pub spacing: Option<i64>,
    /// `(left, top, right, bottom)` if set explicitly.
    pub margins: Option<(i64, i64, i64, i64)>,
    /// `QFormLayout` growth policy (default `AllNonFixedFieldsGrow`).
    pub field_growth_policy: FieldGrowthPolicy,
    /// `QFormLayout` row wrap policy (default `DontWrapRows`).
    pub row_wrap_policy: RowWrapPolicy,
    /// `labelAlignment` bitmask; `0` → Qt style default (`AlignLeft`).
    pub label_alignment: u32,
    /// `formAlignment` bitmask; `0` → Qt style default
    /// (`AlignLeft | AlignTop`).
    pub form_alignment: u32,
    /// `horizontalSpacing` if set explicitly; `None` → style default.
    pub horizontal_spacing: Option<i64>,
    /// `verticalSpacing` if set explicitly; `None` → style default.
    pub vertical_spacing: Option<i64>,
    pub items: Vec<LayoutItem>,
}

/// `Qt::AlignLeft` (and the other alignment flags), as written on `<item>`.
pub const ALIGN_LEFT: u32 = 0x1;
pub const ALIGN_RIGHT: u32 = 0x2;
pub const ALIGN_HCENTER: u32 = 0x4;
pub const ALIGN_TOP: u32 = 0x20;
pub const ALIGN_BOTTOM: u32 = 0x40;
pub const ALIGN_VCENTER: u32 = 0x80;
pub const ALIGN_HORIZONTAL_MASK: u32 = 0x0f;
pub const ALIGN_VERTICAL_MASK: u32 = 0xf0;

/// Cell of an item inside a layout.
///
/// Qt Designer writes `row` / `column` / `rowspan` / `colspan` and optionally
/// `alignment` directly on `<item>` (grid and form layouts). Box layouts leave
/// the attributes unset, so the default is `(0, 0)` with a `1x1` span and no
/// alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutCell {
    pub row: i64,
    pub column: i64,
    pub row_span: i64,
    pub column_span: i64,
    /// Qt alignment flags for the item inside its cell (`0` → Qt defaults).
    pub alignment: u32,
}

impl Default for LayoutCell {
    fn default() -> Self {
        Self {
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
            alignment: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutItem {
    Widget(Widget, LayoutCell),
    Layout(Layout, LayoutCell),
    Spacer(Spacer, LayoutCell),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spacer {
    pub name: String,
    /// `true` for `Qt::Orientation::Horizontal`.
    pub horizontal: bool,
    /// `(width, height)` from the `sizeHint` property.
    pub hint: (i64, i64),
}

/// A parsed `.ui` document: form class + root widget.
#[derive(Debug, Clone, PartialEq)]
pub struct UiForm {
    pub class: String,
    pub root: Widget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ui parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse a Qt Designer `.ui` document into a [`UiForm`].
pub fn parse(xml: &str) -> Result<UiForm, ParseError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| ParseError(e.to_string()))?;
    let ui = doc.root_element();
    if ui.tag_name().name() != "ui" {
        return Err(ParseError("root element is not <ui>".into()));
    }

    let class = ui
        .children()
        .find(|n| n.has_tag_name("class"))
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();

    let root = ui
        .children()
        .find(|n| n.has_tag_name("widget"))
        .ok_or_else(|| ParseError("no root <widget> element".into()))?;

    Ok(UiForm {
        class,
        root: parse_widget(root)?,
    })
}

fn parse_widget(node: Node<'_, '_>) -> Result<Widget, ParseError> {
    let class = node
        .attribute("class")
        .ok_or_else(|| ParseError("<widget> without class attribute".into()))?
        .to_string();
    let name = node.attribute("name").unwrap_or("").to_string();

    let mut properties = Vec::new();
    let mut attributes = Vec::new();
    let mut children = Vec::new();
    let mut layout = None;
    let mut actions = Vec::new();
    let mut add_actions = Vec::new();
    let mut columns = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "property" => {
                if let Some(prop) = parse_property(child) {
                    properties.push(prop);
                }
            }
            "attribute" => {
                if let Some(attr) = parse_property(child) {
                    attributes.push(attr);
                }
            }
            "column" => columns.push(parse_column(child)),
            "widget" => {
                children.push(parse_widget(child)?);
            }
            "layout" => {
                layout = Some(parse_layout(child)?);
            }
            "action" => actions.push(parse_action(child)),
            "addaction" => {
                if let Some(name) = child.attribute("name") {
                    add_actions.push(name.to_string());
                }
            }
            _ => {}
        }
    }

    Ok(Widget {
        class,
        name,
        properties,
        attributes,
        children,
        layout,
        actions,
        add_actions,
        columns,
    })
}

/// `<column>` caption: `<column><property name="text"><string>…`.
/// Missing text means an empty section (Qt then shows no label).
fn parse_column(node: Node<'_, '_>) -> String {
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "property" {
            continue;
        }
        if let Some((name, value)) = parse_property(child) {
            if name == "text" {
                return match value {
                    PropValue::Str(text) | PropValue::Enum(text) => text,
                    _ => String::new(),
                };
            }
        }
    }
    String::new()
}

/// `<action>` definition: name plus its `<property>` children.
fn parse_action(node: Node<'_, '_>) -> Action {
    let name = node.attribute("name").unwrap_or("").to_string();
    let properties = node
        .children()
        .filter(|child| child.has_tag_name("property"))
        .filter_map(parse_property)
        .collect();
    Action { name, properties }
}

fn parse_layout(node: Node<'_, '_>) -> Result<Layout, ParseError> {
    let class = node
        .attribute("class")
        .ok_or_else(|| ParseError("<layout> without class attribute".into()))?
        .to_string();
    let name = node.attribute("name").unwrap_or("").to_string();

    let mut spacing = None;
    let mut margins = None;
    let mut field_growth_policy = FieldGrowthPolicy::default();
    let mut row_wrap_policy = RowWrapPolicy::default();
    let mut label_alignment = 0;
    let mut form_alignment = 0;
    let mut horizontal_spacing = None;
    let mut vertical_spacing = None;
    let mut items = Vec::new();

    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "property" => {
                let Some((key, value)) = parse_property(child) else {
                    continue;
                };
                match (key.as_str(), &value) {
                    ("spacing", PropValue::Int(n)) => spacing = Some(*n),
                    ("horizontalSpacing", PropValue::Int(v)) => horizontal_spacing = Some(*v),
                    ("verticalSpacing", PropValue::Int(v)) => vertical_spacing = Some(*v),
                    ("leftMargin", PropValue::Int(v)) => set_margin(&mut margins, 0, *v),
                    ("topMargin", PropValue::Int(v)) => set_margin(&mut margins, 1, *v),
                    ("rightMargin", PropValue::Int(v)) => set_margin(&mut margins, 2, *v),
                    ("bottomMargin", PropValue::Int(v)) => set_margin(&mut margins, 3, *v),
                    ("fieldGrowthPolicy", value) => {
                        if let Some(name) = value.as_str().and_then(FieldGrowthPolicy::from_name) {
                            field_growth_policy = name;
                        }
                    }
                    ("rowWrapPolicy", value) => {
                        if let Some(name) = value.as_str().and_then(RowWrapPolicy::from_name) {
                            row_wrap_policy = name;
                        }
                    }
                    ("labelAlignment", value) => label_alignment = alignment_value(value),
                    ("formAlignment", value) => form_alignment = alignment_value(value),
                    _ => {}
                }
            }
            "item" => {
                let cell = parse_cell(child);
                let inner = child
                    .children()
                    .find(|n| n.is_element())
                    .ok_or_else(|| ParseError("<item> without content".into()))?;
                match inner.tag_name().name() {
                    "widget" => items.push(LayoutItem::Widget(parse_widget(inner)?, cell)),
                    "layout" => items.push(LayoutItem::Layout(parse_layout(inner)?, cell)),
                    "spacer" => items.push(LayoutItem::Spacer(parse_spacer(inner)?, cell)),
                    other => return Err(ParseError(format!("unsupported item type <{other}>"))),
                }
            }
            _ => {}
        }
    }

    Ok(Layout {
        class,
        name,
        spacing,
        margins,
        field_growth_policy,
        row_wrap_policy,
        label_alignment,
        form_alignment,
        horizontal_spacing,
        vertical_spacing,
        items,
    })
}

/// Alignment property written either as `<set>Qt::AlignLeft|...</set>`, as a
/// set of child `<enum>` elements, or (tolerantly) as plain text.
fn alignment_value(value: &PropValue) -> u32 {
    match value {
        PropValue::Str(text) | PropValue::Enum(text) => parse_alignment(text),
        PropValue::List(items) => items
            .iter()
            .filter_map(PropValue::as_str)
            .fold(0, |mask, flag| mask | parse_alignment(flag)),
        _ => 0,
    }
}

/// Read grid cell attributes from `<item>`; box layouts leave them unset.
fn parse_cell(item: Node<'_, '_>) -> LayoutCell {
    let int = |name: &str, default: i64| -> i64 {
        item.attribute(name)
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(default)
    };

    LayoutCell {
        row: int("row", 0).max(0),
        column: int("column", 0).max(0),
        row_span: int("rowspan", 1).max(1),
        column_span: int("colspan", 1).max(1),
        alignment: item
            .attribute("alignment")
            .map(parse_alignment)
            .unwrap_or(0),
    }
}

/// `Qt::AlignRight|Qt::AlignmentFlag::AlignVCenter` → alignment bitmask.
fn parse_alignment(value: &str) -> u32 {
    value
        .split('|')
        .map(str::trim)
        .filter(|flag| !flag.is_empty())
        .fold(0, |mask, flag| {
            let tail = flag.rsplit("::").next().unwrap_or(flag);
            mask | match tail {
                "AlignLeft" | "AlignLeading" => ALIGN_LEFT,
                "AlignRight" | "AlignTrailing" => ALIGN_RIGHT,
                "AlignHCenter" => ALIGN_HCENTER,
                "AlignTop" => ALIGN_TOP,
                "AlignBottom" => ALIGN_BOTTOM,
                "AlignVCenter" => ALIGN_VCENTER,
                _ => 0,
            }
        })
}

fn set_margin(margins: &mut Option<(i64, i64, i64, i64)>, index: usize, value: i64) {
    let entry = margins.get_or_insert((0, 0, 0, 0));
    match index {
        0 => entry.0 = value,
        1 => entry.1 = value,
        2 => entry.2 = value,
        _ => entry.3 = value,
    }
}

fn parse_spacer(node: Node<'_, '_>) -> Result<Spacer, ParseError> {
    let name = node.attribute("name").unwrap_or("").to_string();
    let mut horizontal = true;
    let mut hint = (40i64, 20i64);

    for child in node.children().filter(|n| n.has_tag_name("property")) {
        let Some((key, value)) = parse_property(child) else {
            continue;
        };
        match (key.as_str(), &value) {
            ("orientation", PropValue::Enum(e)) => {
                horizontal = e.ends_with("Horizontal");
            }
            ("sizeHint", PropValue::Size { w, h }) => hint = (*w, *h),
            _ => {}
        }
    }

    Ok(Spacer {
        name,
        horizontal,
        hint,
    })
}

/// `<property name="...">value</property>` → `(name, value)`.
/// Returns `None` for properties without a recognizable value element.
fn parse_property(node: Node<'_, '_>) -> Option<(String, PropValue)> {
    let key = node.attribute("name")?.to_string();
    let value_node = node.children().find(|n| n.is_element())?;
    let value = parse_value(value_node)?;
    Some((key, value))
}

fn parse_value(node: Node<'_, '_>) -> Option<PropValue> {
    match node.tag_name().name() {
        "string" | "cstring" => Some(PropValue::Str(node.text().unwrap_or("").to_string())),
        "number" => {
            let text = node.text().unwrap_or("").trim();
            Some(
                text.parse::<i64>()
                    .map(PropValue::Int)
                    // Non-integer numerics (e.g. `12.5`) stay as text; callers
                    // that need a float parse the string. Prefer `<double>` in
                    // `.ui` so Qt's QUiLoader accepts the same value.
                    .unwrap_or_else(|_| PropValue::Str(text.to_string())),
            )
        }
        // Qt Designer writes QDoubleSpinBox floats as `<double>`, not `<number>`.
        "double" => {
            let text = node.text().unwrap_or("").trim();
            Some(PropValue::Str(text.to_string()))
        }
        "bool" => Some(PropValue::Bool(node.text().unwrap_or("").trim() == "true")),
        "enum" => Some(PropValue::Enum(node.text().unwrap_or("").to_string())),
        "rect" => {
            let mut rect = [0i64; 4];
            for child in node.children().filter(|n| n.is_element()) {
                let n = child
                    .text()
                    .unwrap_or("")
                    .trim()
                    .parse::<i64>()
                    .unwrap_or(0);
                match child.tag_name().name() {
                    "x" => rect[0] = n,
                    "y" => rect[1] = n,
                    "width" => rect[2] = n,
                    "height" => rect[3] = n,
                    _ => {}
                }
            }
            Some(PropValue::Rect {
                x: rect[0],
                y: rect[1],
                w: rect[2],
                h: rect[3],
            })
        }
        // `QWidget::sizePolicy` is written as named type tags plus stretch
        // factors. Geometry code needs the two policy types, so keep them in
        // a map instead of degrading the whole property to text.
        "sizepolicy" => {
            let mut entries = Vec::new();
            for key in ["hsizetype", "vsizetype"] {
                if let Some(text) = node.attribute(key) {
                    entries.push((key.to_string(), PropValue::Str(text.trim().to_string())));
                }
            }
            for child in node.children().filter(|child| child.is_element()) {
                let key = child.tag_name().name().to_string();
                let Some(value) = child
                    .text()
                    .and_then(|text| text.trim().parse::<i64>().ok())
                else {
                    continue;
                };
                entries.push((key, PropValue::Int(value)));
            }
            Some(PropValue::Map(entries))
        }
        "size" => {
            let mut size = [0i64; 2];
            for child in node.children().filter(|n| n.is_element()) {
                let n = child
                    .text()
                    .unwrap_or("")
                    .trim()
                    .parse::<i64>()
                    .unwrap_or(0);
                match child.tag_name().name() {
                    "width" => size[0] = n,
                    "height" => size[1] = n,
                    _ => {}
                }
            }
            Some(PropValue::Size {
                w: size[0],
                h: size[1],
            })
        }
        // Qt Designer writes flag sets as either `<set>Qt::...Flag</set>`
        // with text, or as a `<set>` with child elements. Keep both forms
        // so `windowFlags` can be inspected without a dedicated parser.
        "set" => {
            let items: Vec<PropValue> = node
                .children()
                .filter(|child| child.is_element())
                .map(|child| PropValue::Str(child.text().unwrap_or("").trim().to_string()))
                .filter(|value| !matches!(value, PropValue::Str(text) if text.is_empty()))
                .collect();
            if items.is_empty() {
                Some(PropValue::Str(node.text().unwrap_or("").trim().to_string()))
            } else {
                Some(PropValue::List(items))
            }
        }
        "iconset" => {
            let entries = node
                .children()
                .filter(|child| child.is_element())
                .map(|child| {
                    (
                        child.tag_name().name().to_string(),
                        PropValue::Str(child.text().unwrap_or("").to_string()),
                    )
                })
                .collect();
            Some(PropValue::Map(entries))
        }
        // Unknown value tags (sizepolicy, ...) degrade to their text.
        _ => Some(PropValue::Str(node.text().unwrap_or("").to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/test_dialog01.ui");

    #[test]
    fn parses_dialog_fixture() {
        let form = parse(FIXTURE).expect("fixture parses");
        assert_eq!(form.class, "Dialog");
        assert_eq!(form.root.class, "QDialog");
        assert_eq!(form.root.name, "Dialog");
        assert_eq!(
            form.root.property("geometry"),
            Some(&PropValue::Rect {
                x: 0,
                y: 0,
                w: 388,
                h: 190
            })
        );
        assert_eq!(
            form.root.property("windowTitle"),
            Some(&PropValue::Str("Dialog".into()))
        );
    }

    #[test]
    fn parses_root_layout_structure() {
        let form = parse(FIXTURE).expect("fixture parses");
        let layout = form.root.layout.as_ref().expect("root has layout");
        assert_eq!(layout.class, "QVBoxLayout");
        assert_eq!(layout.name, "verticalLayout");
        // 3 rows + vertical spacer + button row
        assert_eq!(layout.items.len(), 5);

        match &layout.items[0] {
            LayoutItem::Layout(row, _) => {
                assert_eq!(row.class, "QHBoxLayout");
                assert_eq!(row.items.len(), 3);
                match &row.items[0] {
                    LayoutItem::Widget(w, _) => {
                        assert_eq!(w.class, "QLabel");
                        assert_eq!(w.name, "label_1");
                        assert_eq!(
                            w.property("text"),
                            Some(&PropValue::Str("TextLabel".into()))
                        );
                    }
                    other => panic!("expected label, got {other:?}"),
                }
                match &row.items[1] {
                    LayoutItem::Spacer(s, _) => {
                        assert!(s.horizontal);
                        assert_eq!(s.hint, (40, 20));
                    }
                    other => panic!("expected spacer, got {other:?}"),
                }
                match &row.items[2] {
                    LayoutItem::Widget(w, _) => {
                        assert_eq!(w.class, "QLineEdit");
                        assert_eq!(w.name, "lineEdit_1");
                        assert!(w.properties.is_empty());
                    }
                    other => panic!("expected line edit, got {other:?}"),
                }
            }
            other => panic!("expected row layout, got {other:?}"),
        }

        match &layout.items[3] {
            LayoutItem::Spacer(s, _) => {
                assert!(!s.horizontal);
                assert_eq!(s.hint, (20, 71));
            }
            other => panic!("expected vertical spacer, got {other:?}"),
        }

        match &layout.items[4] {
            LayoutItem::Layout(row, _) => {
                assert_eq!(row.items.len(), 3);
                let button_texts: Vec<_> = row
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        LayoutItem::Widget(w, _) => w.property("text").cloned(),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    button_texts,
                    vec![
                        PropValue::Str("PushButton".into()),
                        PropValue::Str("PushButton".into())
                    ]
                );
            }
            other => panic!("expected button row, got {other:?}"),
        }
    }

    #[test]
    fn parses_window_flag_sets() {
        let xml = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog">
            <property name="windowFlags"><set>Qt::WindowType::MSWindowsFixedSizeDialogHint</set></property>
        </widget></ui>"#;
        let form = parse(xml).expect("ui parses");
        assert_eq!(
            form.root.property("windowFlags"),
            Some(&PropValue::Str(
                "Qt::WindowType::MSWindowsFixedSizeDialogHint".into()
            ))
        );

        let xml = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog">
            <property name="windowFlags"><set><enum>Qt::WindowType::MSWindowsFixedSizeDialogHint</enum></set></property>
        </widget></ui>"#;
        let form = parse(xml).expect("ui parses");
        assert_eq!(
            form.root.property("windowFlags"),
            Some(&PropValue::List(vec![PropValue::Str(
                "Qt::WindowType::MSWindowsFixedSizeDialogHint".into()
            )]))
        );
    }

    #[test]
    fn parses_item_cells_from_grid_layouts() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QGridLayout" name="grid">
   <item row="0" column="1">
    <widget class="QLabel" name="a"/>
   </item>
   <item row="1" column="0" rowspan="2" colspan="3">
    <layout class="QVBoxLayout" name="inner">
     <item><widget class="QLabel" name="b"/></item>
    </layout>
   </item>
   <item row="2" column="2" colspan="2">
    <spacer name="s">
     <property name="orientation"><enum>Qt::Orientation::Horizontal</enum></property>
    </spacer>
   </item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("grid parses");
        let layout = form.root.layout.as_ref().expect("root layout");

        match &layout.items[0] {
            LayoutItem::Widget(widget, cell) => {
                assert_eq!(widget.name, "a");
                assert_eq!(
                    *cell,
                    LayoutCell {
                        row: 0,
                        column: 1,
                        row_span: 1,
                        column_span: 1,
                        alignment: 0,
                    }
                );
            }
            other => panic!("expected widget item, got {other:?}"),
        }

        match &layout.items[1] {
            LayoutItem::Layout(inner, cell) => {
                assert_eq!(inner.class, "QVBoxLayout");
                assert_eq!(
                    *cell,
                    LayoutCell {
                        row: 1,
                        column: 0,
                        row_span: 2,
                        column_span: 3,
                        alignment: 0,
                    }
                );
                match &inner.items[0] {
                    LayoutItem::Widget(widget, inner_cell) => {
                        assert_eq!(widget.name, "b");
                        assert_eq!(*inner_cell, LayoutCell::default());
                    }
                    other => panic!("expected nested widget, got {other:?}"),
                }
            }
            other => panic!("expected nested layout, got {other:?}"),
        }

        match &layout.items[2] {
            LayoutItem::Spacer(spacer, cell) => {
                assert!(spacer.horizontal);
                assert_eq!(
                    *cell,
                    LayoutCell {
                        row: 2,
                        column: 2,
                        row_span: 1,
                        column_span: 2,
                        alignment: 0,
                    }
                );
            }
            other => panic!("expected spacer, got {other:?}"),
        }
    }

    #[test]
    fn parses_form_layout_properties() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QFormLayout" name="form">
   <property name="fieldGrowthPolicy"><enum>QFormLayout::ExpandingFieldsGrow</enum></property>
   <property name="rowWrapPolicy"><enum>QFormLayout::WrapLongRows</enum></property>
   <property name="labelAlignment"><set>Qt::AlignmentFlag::AlignRight|Qt::AlignmentFlag::AlignVCenter</set></property>
   <property name="formAlignment"><set><enum>Qt::AlignHCenter</enum><enum>Qt::AlignTop</enum></set></property>
   <property name="horizontalSpacing"><number>9</number></property>
   <property name="verticalSpacing"><number>4</number></property>
   <item row="0" column="0"><widget class="QLabel" name="label"/></item>
   <item row="0" column="1"><widget class="QLineEdit" name="edit"/></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("form parses");
        let layout = form.root.layout.as_ref().expect("root layout");
        assert_eq!(layout.class, "QFormLayout");
        assert_eq!(
            layout.field_growth_policy,
            FieldGrowthPolicy::ExpandingFieldsGrow
        );
        assert_eq!(layout.row_wrap_policy, RowWrapPolicy::WrapLongRows);
        assert_eq!(layout.label_alignment, ALIGN_RIGHT | ALIGN_VCENTER);
        assert_eq!(layout.form_alignment, ALIGN_HCENTER | ALIGN_TOP);
        assert_eq!(layout.horizontal_spacing, Some(9));
        assert_eq!(layout.vertical_spacing, Some(4));
    }

    #[test]
    fn form_layout_defaults_follow_fusion() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QFormLayout" name="form">
   <item row="0" column="0"><widget class="QLabel" name="label"/></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("form parses");
        let layout = form.root.layout.as_ref().expect("root layout");
        assert_eq!(
            layout.field_growth_policy,
            FieldGrowthPolicy::AllNonFixedFieldsGrow
        );
        assert_eq!(layout.row_wrap_policy, RowWrapPolicy::DontWrapRows);
        assert_eq!(layout.label_alignment, 0);
        assert_eq!(layout.form_alignment, 0);
        assert_eq!(layout.horizontal_spacing, None);
        assert_eq!(layout.vertical_spacing, None);
    }

    #[test]
    fn parses_item_alignment_flags() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QGridLayout" name="grid">
   <item row="0" column="0" alignment="Qt::AlignRight|Qt::AlignmentFlag::AlignVCenter">
    <widget class="QPushButton" name="ok"/>
   </item>
   <item row="0" column="1" alignment="Qt::AlignLeft">
    <widget class="QPushButton" name="cancel"/>
   </item>
   <item row="1" column="0">
    <widget class="QPushButton" name="plain"/>
   </item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let layout = form.root.layout.as_ref().expect("root layout");
        let alignment = |item: &LayoutItem| match item {
            LayoutItem::Widget(_, cell) => cell.alignment,
            other => panic!("expected widget, got {other:?}"),
        };
        assert_eq!(alignment(&layout.items[0]), ALIGN_RIGHT | ALIGN_VCENTER);
        assert_eq!(alignment(&layout.items[1]), ALIGN_LEFT);
        assert_eq!(alignment(&layout.items[2]), 0);
    }

    #[test]
    fn box_layout_items_default_to_single_cell() {
        let form = parse(FIXTURE).expect("fixture parses");
        let layout = form.root.layout.as_ref().expect("root layout");

        match &layout.items[0] {
            LayoutItem::Layout(_, cell) => assert_eq!(*cell, LayoutCell::default()),
            other => panic!("expected row layout, got {other:?}"),
        }
        match &layout.items[3] {
            LayoutItem::Spacer(_, cell) => assert_eq!(*cell, LayoutCell::default()),
            other => panic!("expected spacer, got {other:?}"),
        }
    }

    #[test]
    fn invalid_cell_attributes_fall_back_to_defaults() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QGridLayout" name="grid">
   <item row="-5" column="nope" rowspan="0" colspan="-2">
    <widget class="QLabel" name="a"/>
   </item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let layout = form.root.layout.as_ref().expect("root layout");
        match &layout.items[0] {
            LayoutItem::Widget(_, cell) => assert_eq!(*cell, LayoutCell::default()),
            other => panic!("expected widget, got {other:?}"),
        }
    }

    #[test]
    fn parses_size_policy_types() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <layout class="QGridLayout" name="grid">
   <item row="0" column="0">
    <widget class="QPushButton" name="ok">
     <property name="sizePolicy">
      <sizepolicy hsizetype="Fixed" vsizetype="MinimumExpanding">
       <horstretch>0</horstretch>
       <verstretch>2</verstretch>
      </sizepolicy>
     </property>
    </widget>
   </item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("ui parses");
        let layout = form.root.layout.as_ref().expect("root layout");
        let widget = match &layout.items[0] {
            LayoutItem::Widget(widget, _) => widget,
            other => panic!("expected widget, got {other:?}"),
        };
        let policy = widget.property("sizePolicy").expect("sizePolicy");
        assert_eq!(
            policy.get("hsizetype"),
            Some(&PropValue::Str("Fixed".into()))
        );
        assert_eq!(
            policy.get("vsizetype"),
            Some(&PropValue::Str("MinimumExpanding".into()))
        );
        assert_eq!(policy.get("horstretch"), Some(&PropValue::Int(0)));
        assert_eq!(policy.get("verstretch"), Some(&PropValue::Int(2)));
    }

    #[test]
    fn parses_main_window_actions_and_addaction_order() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>MainWindow</class>
 <widget class="QMainWindow" name="MainWindow">
  <widget class="QMenuBar" name="menubar">
   <widget class="QMenu" name="menuFile">
    <property name="title"><string>File</string></property>
    <addaction name="actionOpen"/>
    <addaction name="separator"/>
    <addaction name="actionExit"/>
   </widget>
   <addaction name="menuFile"/>
  </widget>
  <widget class="QToolBar" name="toolBar">
   <addaction name="actionOpen"/>
  </widget>
  <action name="actionOpen">
   <property name="text"><string>Open</string></property>
   <property name="enabled"><bool>false</bool></property>
  </action>
  <action name="actionExit">
   <property name="text"><string>Exit</string></property>
  </action>
 </widget>
</ui>"#;
        let form = parse(xml).expect("main window parses");
        assert_eq!(form.root.class, "QMainWindow");
        assert_eq!(form.root.actions.len(), 2);
        assert_eq!(form.root.actions[0].name, "actionOpen");
        assert_eq!(
            form.root.actions[0].property("enabled"),
            Some(&PropValue::Bool(false))
        );
        let menu_bar = form
            .root
            .children
            .iter()
            .find(|widget| widget.class == "QMenuBar")
            .expect("menu bar");
        assert_eq!(menu_bar.add_actions, vec!["menuFile".to_string()]);
        let menu = &menu_bar.children[0];
        assert_eq!(menu.class, "QMenu");
        assert_eq!(
            menu.add_actions,
            vec![
                "actionOpen".to_string(),
                "separator".to_string(),
                "actionExit".to_string()
            ]
        );
        let toolbar = form
            .root
            .children
            .iter()
            .find(|widget| widget.class == "QToolBar")
            .expect("toolbar");
        assert_eq!(toolbar.add_actions, vec!["actionOpen".to_string()]);
    }

    #[test]
    fn rejects_non_ui_documents() {
        assert!(parse("<html><body/></html>").is_err());
        assert!(parse("not xml at all").is_err());
        assert!(parse("<ui><class>X</class></ui>").is_err()); // no root widget
    }

    #[test]
    fn tolerates_unknown_property_value_tags() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="w">
  <property name="weird"><something>text</something></property>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        assert_eq!(
            form.root.property("weird"),
            Some(&PropValue::Str("text".into()))
        );
    }

    #[test]
    fn parses_tab_widget_page_title_attributes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <widget class="QTabWidget" name="tabs">
   <property name="currentIndex"><number>1</number></property>
   <widget class="QWidget" name="pageA">
    <attribute name="title"><string>Первая</string></attribute>
   </widget>
   <widget class="QWidget" name="pageB">
    <attribute name="title"><string>Вторая</string></attribute>
   </widget>
  </widget>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let tabs = &form.root.children[0];
        assert_eq!(tabs.class, "QTabWidget");
        assert_eq!(
            tabs.property("currentIndex"),
            Some(&PropValue::Int(1))
        );
        assert_eq!(
            tabs.children[0].attribute("title"),
            Some(&PropValue::Str("Первая".into()))
        );
        assert_eq!(
            tabs.children[1].attribute("title"),
            Some(&PropValue::Str("Вторая".into()))
        );
    }

    #[test]
    fn parses_toolbox_page_label_attributes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <widget class="QToolBox" name="toolbox">
   <property name="currentIndex"><number>1</number></property>
   <widget class="QWidget" name="pageA">
    <attribute name="label"><string>Первая</string></attribute>
   </widget>
   <widget class="QWidget" name="pageB">
    <attribute name="label"><string>Вторая</string></attribute>
   </widget>
  </widget>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let toolbox = &form.root.children[0];
        assert_eq!(toolbox.class, "QToolBox");
        assert_eq!(
            toolbox.property("currentIndex"),
            Some(&PropValue::Int(1))
        );
        assert_eq!(
            toolbox.children[0].attribute("label"),
            Some(&PropValue::Str("Первая".into()))
        );
        assert_eq!(
            toolbox.children[1].attribute("label"),
            Some(&PropValue::Str("Вторая".into()))
        );
    }
}
