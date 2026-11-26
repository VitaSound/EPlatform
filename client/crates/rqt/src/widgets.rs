use crate::core::{QEventType, QVariant};
use crate::gui::{QFont, QFontMetrics, QMargins, QPalette, QPixmap, QPointF, QRectF, QSizeF};
use crate::item_model::{QEditTrigger, QSelectionMode, QTableWidget};
use crate::item_view::{QListWidget, QTreeWidget, QVIEW_HEADER_H};
use crate::qss::QStyleSheet;
use crate::scroll::{
    QOrientation, QScrollAreaLayout, QScrollBar, QScrollBarPolicy, QScrollBarSubControl,
};

pub type QWidgetId = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QSizePolicy {
    Fixed,
    Minimum,
    Maximum,
    #[default]
    Preferred,
    Expanding,
    MinimumExpanding,
    Ignored,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QFocusPolicy {
    #[default]
    NoFocus,
    TabFocus,
    ClickFocus,
    StrongFocus,
    WheelFocus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QWidgetEvent {
    pub widget: QWidgetId,
    pub event_type: QEventType,
    pub clicked: bool,
}

impl QWidgetEvent {
    pub fn protocol_event(&self) -> &'static str {
        if self.clicked {
            "Clicked"
        } else {
            self.event_type.qt_name()
        }
    }
}

/// Layout flavour, selected by the `.ui` layout class name.
///
/// Before 0.12.2 the flavour was inferred from `class.contains('H')`, which
/// mis-detected `QGridLayout` as a horizontal box. The kind is explicit now,
/// so new layout engines can be added without touching box layouts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QLayoutKind {
    #[default]
    VBox,
    HBox,
    Grid,
    /// `QFormLayout`; laid out as a vertical stack until 0.13 lands.
    Form,
}

impl QLayoutKind {
    pub fn from_class(class: &str) -> Self {
        match class {
            "QHBoxLayout" => Self::HBox,
            "QGridLayout" => Self::Grid,
            "QFormLayout" => Self::Form,
            _ => Self::VBox,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QSpacerItem {
    pub width: f32,
    pub height: f32,
    pub horizontal_policy: QSizePolicy,
    pub vertical_policy: QSizePolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QLayoutItem {
    Widget {
        id: QWidgetId,
        /// Cell from `<item row= column= rowspan= colspan=>`.
        cell: form_xml::LayoutCell,
    },
    Spacer {
        item: QSpacerItem,
        cell: form_xml::LayoutCell,
    },
    Layout {
        layout: Box<QLayout>,
        cell: form_xml::LayoutCell,
    },
}

impl QLayoutItem {
    pub fn cell(&self) -> form_xml::LayoutCell {
        match self {
            QLayoutItem::Widget { cell, .. }
            | QLayoutItem::Spacer { cell, .. }
            | QLayoutItem::Layout { cell, .. } => *cell,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QLayout {
    pub kind: QLayoutKind,
    pub spacing: f32,
    pub margins: QMargins,
    /// `QFormLayout::fieldGrowthPolicy`; ignored by other layout kinds.
    pub field_growth_policy: form_xml::FieldGrowthPolicy,
    /// `QFormLayout::rowWrapPolicy`; ignored by other layout kinds.
    pub row_wrap_policy: form_xml::RowWrapPolicy,
    /// `QFormLayout` label alignment; `0` uses the Fusion default
    /// (`AlignLeft`).
    pub label_alignment: u32,
    /// `QFormLayout` form alignment; `0` uses the Fusion default
    /// (`AlignLeft | AlignTop`).
    pub form_alignment: u32,
    /// Explicit `QFormLayout::horizontalSpacing`; `None` falls back to
    /// `spacing` and then to the style default (6).
    pub horizontal_spacing: Option<f32>,
    /// Explicit `QFormLayout::verticalSpacing`; `None` falls back to
    /// `spacing` and then to the style default (6).
    pub vertical_spacing: Option<f32>,
    pub items: Vec<QLayoutItem>,
}

impl QLayout {
    pub fn new(kind: QLayoutKind) -> Self {
        Self {
            kind,
            spacing: 6.0,
            margins: QMargins::default(),
            field_growth_policy: form_xml::FieldGrowthPolicy::default(),
            row_wrap_policy: form_xml::RowWrapPolicy::default(),
            label_alignment: 0,
            form_alignment: 0,
            horizontal_spacing: None,
            vertical_spacing: None,
            items: Vec::new(),
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.kind == QLayoutKind::HBox
    }

    pub fn is_grid(&self) -> bool {
        self.kind == QLayoutKind::Grid
    }

    pub fn is_form(&self) -> bool {
        self.kind == QLayoutKind::Form
    }

    fn form_horizontal_spacing(&self) -> f32 {
        match self.horizontal_spacing {
            Some(value) if value >= 0.0 => value,
            _ if self.spacing >= 0.0 => self.spacing,
            _ => 6.0,
        }
    }

    fn form_vertical_spacing(&self) -> f32 {
        match self.vertical_spacing {
            Some(value) if value >= 0.0 => value,
            _ if self.spacing >= 0.0 => self.spacing,
            _ => 6.0,
        }
    }
}

/// `QSplitter` state. Pane sizes are kept in pixels so a handle drag and a
/// later window resize (which scales the ratios) behave like Qt.
#[derive(Clone, Debug)]
pub struct QSplitter {
    pub orientation: QOrientation,
    pub handle_width: f32,
    pub sizes: Vec<f32>,
}

impl QSplitter {
    pub fn new() -> Self {
        Self {
            orientation: QOrientation::Horizontal,
            handle_width: 4.0,
            sizes: Vec::new(),
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.orientation == QOrientation::Horizontal
    }
}

impl Default for QSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// `QTabWidget` state. Pages are direct children; titles come from each
/// page's `<attribute name="title">`. The bar height matches
/// `QPixelMetric::TabBarHeight` (Fusion).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QTabWidget {
    pub current_index: usize,
    pub titles: Vec<String>,
}

impl QTabWidget {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Horizontal padding around a tab label (Fusion `PM_TabBarTabHSpace`).
pub const QTAB_H_SPACE: f32 = 24.0;

/// `QComboBox` popup view state. Item strings live in the `items` property
/// (the same source the closed combo paints); only the transient open state
/// is kept here, like `QComboBoxPrivateContainer` in Qt.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QComboBoxPopup {
    pub highlight: usize,
    pub scroll: usize,
}

/// Height of one popup row in Fusion. Qt's `QComboBox` view returns 24 px for
/// the default 14 px DejaVu Sans font (`QListView::sizeHintForRow`).
pub const QCOMBO_POPUP_ROW_H: f32 = 24.0;

#[derive(Clone, Debug)]
struct QSplitterDrag {
    widget: QWidgetId,
    handle: usize,
    last: f32,
}

#[derive(Clone, Debug)]
pub struct QWidget {
    pub class_name: String,
    pub object_name: String,
    pub parent: Option<QWidgetId>,
    pub children: Vec<QWidgetId>,
    pub geometry: QRectF,
    pub size_hint: QSizeF,
    pub minimum_size: QSizeF,
    pub maximum_size: QSizeF,
    /// Explicit `sizePolicy` from `.ui` (horizontal part); `Preferred`
    /// when the form does not override the class default.
    pub size_policy: QSizePolicy,
    /// Explicit `sizePolicy` from `.ui` (vertical part).
    pub size_policy_v: QSizePolicy,
    /// Whether the form set `sizePolicy` explicitly (overrides class defaults).
    pub has_size_policy: bool,
    pub font: QFont,
    pub palette: QPalette,
    pub properties: Vec<(String, QVariant)>,
    pub enabled: bool,
    pub visible: bool,
    pub focus_policy: QFocusPolicy,
    pub has_focus: bool,
    pub pressed: bool,
    pub under_mouse: bool,
    pub cursor_position: usize,
    pub selection_anchor: Option<usize>,
    pub cursor_visible: bool,
    pub default_button: bool,
    pub icon: Option<QPixmap>,
    pub icon_disabled: Option<QPixmap>,
    pub icon_size: QSizeF,
    pub explicit_position: Option<QPointF>,
    pub style_sheet: Option<QStyleSheet>,
    pub layout: Option<QLayout>,
    pub table: Option<Box<QTableWidget>>,
    pub list: Option<Box<QListWidget>>,
    pub tree: Option<Box<QTreeWidget>>,
    pub splitter: Option<Box<QSplitter>>,
    pub tabs: Option<Box<QTabWidget>>,
    pub combo_popup: Option<QComboBoxPopup>,
}

impl QWidget {
    fn new(class_name: String, object_name: String) -> Self {
        let table = if class_name == "QTableWidget" {
            Some(Box::new(QTableWidget::default()))
        } else {
            None
        };
        let list = if class_name == "QListWidget" {
            Some(Box::new(QListWidget::new()))
        } else {
            None
        };
        let tree = if class_name == "QTreeWidget" {
            Some(Box::new(QTreeWidget::new()))
        } else {
            None
        };
        let splitter = if class_name == "QSplitter" {
            Some(Box::new(QSplitter::new()))
        } else {
            None
        };
        let tabs = if class_name == "QTabWidget" {
            Some(Box::new(QTabWidget::new()))
        } else {
            None
        };
        let focus_policy = match class_name.as_str() {
            "QLineEdit" | "QPushButton" | "QTableWidget" | "QListWidget" | "QTreeWidget"
            | "QComboBox" | "QCheckBox" | "QRadioButton" | "QTabWidget" => {
                QFocusPolicy::StrongFocus
            }
            "QAbstractButton" | "QToolButton" => QFocusPolicy::StrongFocus,
            _ => QFocusPolicy::NoFocus,
        };
        Self {
            class_name,
            object_name,
            parent: None,
            children: Vec::new(),
            geometry: QRectF::default(),
            size_hint: QSizeF::default(),
            minimum_size: QSizeF::default(),
            maximum_size: QSizeF::new(f32::MAX, f32::MAX),
            size_policy: QSizePolicy::Preferred,
            size_policy_v: QSizePolicy::Preferred,
            has_size_policy: false,
            font: QFont::default(),
            palette: QPalette::fusion(),
            properties: Vec::new(),
            enabled: true,
            visible: true,
            focus_policy,
            has_focus: false,
            pressed: false,
            under_mouse: false,
            cursor_position: 0,
            selection_anchor: None,
            cursor_visible: true,
            default_button: false,
            icon: None,
            icon_disabled: None,
            icon_size: QSizeF::new(16.0, 16.0),
            explicit_position: None,
            style_sheet: None,
            layout: None,
            table,
            list,
            tree,
            splitter,
            tabs,
            combo_popup: None,
        }
    }

    pub fn property(&self, name: &str) -> Option<&QVariant> {
        self.properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    pub fn set_property(&mut self, name: impl Into<String>, value: QVariant) {
        let name = name.into();
        if let Some(entry) = self.properties.iter_mut().find(|(key, _)| key == &name) {
            entry.1 = value;
        } else {
            self.properties.push((name, value));
        }
    }

    fn text(&self) -> String {
        match self.property("text") {
            Some(QVariant::String(text)) | Some(QVariant::Enum(text)) => text.clone(),
            Some(QVariant::Int(value)) => value.to_string(),
            Some(QVariant::Double(value)) => value.to_string(),
            _ => String::new(),
        }
    }

    /// QLabel `wordWrap` property — the label wraps and sizes like Qt's
    /// heightForWidth label (0.18/E3).
    fn word_wrap(&self) -> bool {
        matches!(self.property("wordWrap"), Some(QVariant::Bool(true)))
    }

    fn size_hint_from_props(&self) -> QSizeF {
        let metrics = QFontMetrics::new(&self.font);
        let text = self.text();
        let text_width = metrics.horizontal_advance(&text);
        let font_height = metrics.height();
        match self.class_name.as_str() {
            "QLabel" => {
                if self.word_wrap() {
                    // Qt's minimum for a wrapped label is its longest word,
                    // so long captions no longer widen the window.
                    let longest = text
                        .split_whitespace()
                        .map(|word| metrics.horizontal_advance(word))
                        .fold(0.0_f32, f32::max);
                    QSizeF::new(longest.ceil(), font_height.ceil())
                } else {
                    QSizeF::new(metrics.bounding_rect(&text).width.ceil(), font_height.ceil())
                }
            }
            "QLineEdit" => QSizeF::new(
                (17.0 * metrics.horizontal_advance("x")).ceil() + 5.0,
                (font_height + 8.0).round(),
            ),
            "QPushButton" => {
                let icon_extra = self
                    .icon
                    .as_ref()
                    .map(|_| self.icon_size.width + if text.is_empty() { 0.0 } else { 4.0 })
                    .unwrap_or(0.0);
                QSizeF::new(
                    (text_width.round() + icon_extra + 9.0).max(80.0),
                    (font_height + 9.0).round(),
                )
            }
            "QCheckBox" | "QRadioButton" => {
                let width = metrics.bounding_rect(&text).width.ceil();
                QSizeF::new(width + 26.0, (font_height + 7.0).round())
            }
            "QComboBox" => QSizeF::new(
                if text.is_empty() {
                    86.0
                } else {
                    text_width.max(80.0) + 28.0
                },
                (font_height + 8.7).round(),
            ),
            "QTableWidget" => QSizeF::new(256.0, 192.0),
            "QListWidget" | "QTreeWidget" => QSizeF::new(256.0, 192.0),
            "QSplitter" => QSizeF::new(256.0, 192.0),
            "QTabWidget" => QSizeF::new(256.0, 192.0),
            "QGroupBox" => {
                // QGroupBox stores its caption in `title`, not `text`.
                let title = self
                    .property("title")
                    .and_then(variant_text)
                    .unwrap_or_default();
                let title_width = metrics.horizontal_advance(&title);
                // Qt expands the group minimum by title + 2*framewidth;
                // probed empirically: Qt window 508 = title 456 + 30 +
                // 22 (root margins) for gallery/text/group_long_title.
                QSizeF::new(title_width + 30.0, font_height + 16.0)
            }
            _ => QSizeF::new(100.0, font_height + 8.0),
        }
    }
}

/// Greedy word wrap shared by QLabel layout (`wordWrap`) and painting:
/// breaks `text` into lines no wider than `width` (0.18/E3).
pub(crate) fn wrap_text_lines(text: &str, width: f32, metrics: &QFontMetrics) -> Vec<String> {
    let width = width.max(1.0);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        // A single word longer than the line still starts its own line.
        if current.is_empty() || metrics.horizontal_advance(&candidate) <= width {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Height of `text` wrapped at `width` — the heightForWidth substitute for
/// QVBoxLayout (Qt computes exactly this for word-wrapped labels).
pub(crate) fn word_wrapped_height(text: &str, width: f32, metrics: &QFontMetrics) -> f32 {
    wrap_text_lines(text, width, metrics).len() as f32 * metrics.height()
}

fn variant_text(value: &QVariant) -> Option<String> {
    match value {
        QVariant::String(text) | QVariant::Enum(text) => Some(text.clone()),
        QVariant::Int(value) => Some(value.to_string()),
        QVariant::Double(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_as_i64(value: &QVariant) -> Option<i64> {
    match value {
        QVariant::Int(value) => Some(*value),
        QVariant::Double(value) => Some(*value as i64),
        QVariant::String(text) | QVariant::Enum(text) => text.parse().ok(),
        _ => None,
    }
}

fn combo_items(widget: &QWidget) -> Vec<String> {
    match widget.property("items") {
        Some(QVariant::List(items)) => items.iter().filter_map(variant_text).collect(),
        _ => Vec::new(),
    }
}

fn combo_visible_rows(widget: &QWidget, count: usize) -> usize {
    let max_visible = widget
        .property("maxVisibleItems")
        .and_then(value_as_i64)
        .unwrap_or(10)
        .max(1) as usize;
    count.min(max_visible).max(1)
}

#[derive(Clone, Copy, Debug)]
struct QScrollDrag {
    widget: QWidgetId,
    vertical: bool,
    offset: f32,
}

#[derive(Clone, Debug, Default)]
pub struct QWidgetTree {
    widgets: Vec<QWidget>,
    mouse_grab: Option<QWidgetId>,
    scroll_drag: Option<QScrollDrag>,
    splitter_drag: Option<QSplitterDrag>,
}

impl QWidgetTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_widget(
        &mut self,
        class_name: impl Into<String>,
        object_name: impl Into<String>,
    ) -> QWidgetId {
        let id = self.widgets.len();
        self.widgets
            .push(QWidget::new(class_name.into(), object_name.into()));
        id
    }

    pub fn add_child(&mut self, parent: QWidgetId, child: QWidgetId) -> bool {
        if parent == child
            || parent >= self.widgets.len()
            || child >= self.widgets.len()
            || self.is_ancestor(child, parent)
        {
            return false;
        }
        if let Some(previous) = self.widgets[child].parent {
            if let Some(previous) = self.widgets.get_mut(previous) {
                previous.children.retain(|id| id != &child);
            }
        }
        self.widgets[child].parent = Some(parent);
        for child_id in &mut self.widgets[parent].children {
            if *child_id == child {
                return true;
            }
        }
        self.widgets[parent].children.push(child);
        true
    }

    pub fn is_ancestor(&self, ancestor: QWidgetId, widget: QWidgetId) -> bool {
        let mut current = Some(widget);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.widgets.get(id).and_then(|widget| widget.parent);
        }
        false
    }

    pub fn widget(&self, id: QWidgetId) -> Option<&QWidget> {
        self.widgets.get(id)
    }

    pub fn widget_mut(&mut self, id: QWidgetId) -> Option<&mut QWidget> {
        self.widgets.get_mut(id)
    }

    pub fn iter_widgets(&self) -> impl Iterator<Item = (QWidgetId, &QWidget)> {
        self.widgets.iter().enumerate()
    }

    pub fn set_property(
        &mut self,
        id: QWidgetId,
        name: impl Into<String>,
        value: QVariant,
    ) -> bool {
        let name = name.into();
        let class = self
            .widgets
            .get(id)
            .map(|widget| widget.class_name.clone())
            .unwrap_or_default();
        let font = self
            .widgets
            .get(id)
            .map(|widget| widget.font.clone())
            .unwrap_or_default();
        if name == "icon" {
            if let Some(spec) = value.as_str() {
                let icon = crate::icons::pixmap(spec);
                let disabled = crate::icons::spec_index(spec)
                    .filter(|(disabled, _)| !disabled)
                    .and_then(|(_, index)| crate::icons::pixmap(&format!("img24d:{}", index + 1)));
                if let Some(widget) = self.widgets.get_mut(id) {
                    widget.icon = icon;
                    widget.icon_disabled = disabled;
                }
                return true;
            }
        }
        if name == "checked"
            && matches!(class.as_str(), "QRadioButton" | "QCheckBox")
            && value.as_bool().is_some()
        {
            return self.set_checked(id, value.as_bool().unwrap_or(false));
        }
        let handled = match class.as_str() {
            "QTableWidget" => self
                .widgets
                .get_mut(id)
                .and_then(|widget| widget.table.as_mut())
                .is_some_and(|table| set_table_property(table, &name, &value)),
            "QListWidget" => self
                .widgets
                .get_mut(id)
                .and_then(|widget| widget.list.as_mut())
                .is_some_and(|list| set_list_property(list, &name, &value, &font)),
            "QTreeWidget" => self
                .widgets
                .get_mut(id)
                .and_then(|widget| widget.tree.as_mut())
                .is_some_and(|tree| set_tree_property(tree, &name, &value, &font)),
            _ => true,
        };
        let is_item_view = matches!(
            class.as_str(),
            "QTableWidget" | "QListWidget" | "QTreeWidget"
        );
        let (ok, tab_geometry) = {
        let Some(widget) = self.widgets.get_mut(id) else {
            return false;
        };
        if name == "enabled" {
            if let Some(enabled) = value.as_bool() {
                widget.enabled = enabled;
                if !enabled {
                    if widget.class_name == "QLineEdit" {
                        widget.selection_anchor = None;
                    }
                    widget.has_focus = false;
                }
            }
        }
        if name == "visible" {
            if let Some(visible) = value.as_bool() {
                widget.visible = visible;
            }
        }
        if name == "minimumSize" {
            if let QVariant::Size(size) = &value {
                widget.minimum_size = *size;
            }
        }
        if name == "maximumSize" {
            if let QVariant::Size(size) = &value {
                widget.maximum_size = *size;
            }
        }
        if name == "sizePolicy" {
            if let QVariant::Map(entries) = &value {
                for (key, value) in entries {
                    let Some(text) = value.as_str() else {
                        continue;
                    };
                    let Some(policy) = parse_size_policy(text) else {
                        continue;
                    };
                    match key.as_str() {
                        "hsizetype" => {
                            widget.size_policy = policy;
                            widget.has_size_policy = true;
                        }
                        "vsizetype" => {
                            widget.size_policy_v = policy;
                            widget.has_size_policy = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        if name == "styleSheet" {
            widget.style_sheet = match &value {
                QVariant::String(text) if !text.trim().is_empty() => Some(QStyleSheet::parse(text)),
                _ => None,
            };
        }
        if widget.class_name == "QSplitter" {
            let Some(splitter) = widget.splitter.as_mut() else {
                return false;
            };
            if name == "orientation" {
                let text = variant_text(&value).unwrap_or_default();
                splitter.orientation = if text.to_ascii_lowercase().contains("vertical") {
                    QOrientation::Vertical
                } else if text.to_ascii_lowercase().contains("horizontal") {
                    QOrientation::Horizontal
                } else {
                    splitter.orientation
                };
                splitter.sizes.clear();
            } else if name == "handleWidth" {
                if let Some(width) = value_as_i64(&value) {
                    splitter.handle_width = (width as f32).max(0.0);
                    splitter.sizes.clear();
                }
            } else if name == "sizes" {
                if let QVariant::List(items) = &value {
                    splitter.sizes = items
                        .iter()
                        .filter_map(value_as_i64)
                        .map(|size| (size as f32).max(0.0))
                        .collect();
                }
            }
        }
        let tab_index_changed = widget.class_name == "QTabWidget" && name == "currentIndex";
        if tab_index_changed {
            if let Some(tabs) = widget.tabs.as_mut() {
                if let Some(index) = value_as_i64(&value) {
                    tabs.current_index = index.max(0) as usize;
                }
            }
        }
        if widget.class_name == "QComboBox" && name == "text" {
            // `setCurrentText` style updates are represented by `text`; keep
            // the current index in sync with the item list when possible.
            if let Some(text) = value.as_str() {
                let items = combo_items(widget);
                if let Some(index) = items.iter().position(|item| item == text) {
                    widget.set_property("currentIndex", QVariant::Int(index as i64));
                }
            }
        }
        if widget.class_name == "QLineEdit" && name == "text" {
            let length = match &value {
                QVariant::String(text) | QVariant::Enum(text) => text.chars().count(),
                QVariant::Int(value) => value.to_string().chars().count(),
                _ => 0,
            };
            widget.cursor_position = length;
            widget.selection_anchor = None;
            widget.cursor_visible = true;
        }
        let property_name = name.clone();
        let tab_geometry = if tab_index_changed {
            Some(widget.geometry)
        } else {
            None
        };
        widget.set_property(name, value);
        if widget.class_name == "QComboBox" {
            if matches!(
                property_name.as_str(),
                "items" | "currentIndex" | "maxVisibleItems" | "enabled" | "visible"
            ) {
                let items = combo_items(widget);
                if items.is_empty() || !widget.visible || !widget.enabled {
                    widget.combo_popup = None;
                } else {
                    let current = widget
                        .property("currentIndex")
                        .and_then(|value| value_as_i64(value))
                        .unwrap_or(0)
                        .max(0) as usize;
                    let visible = combo_visible_rows(widget, items.len());
                    if let Some(popup) = widget.combo_popup.as_mut() {
                        popup.highlight = popup.highlight.min(items.len() - 1);
                        popup.scroll = popup
                            .scroll
                            .min(current.saturating_sub(visible.saturating_sub(1)))
                            .min(items.len() - visible);
                        popup.highlight = popup.highlight.max(popup.scroll);
                    }
                }
            }
        }
        (!is_item_view || handled, tab_geometry)
        };
        if let Some(geometry) = tab_geometry {
            self.sync_tab_pages(id);
            self.layout_tab_widget(id, geometry);
        }
        ok
    }

    pub fn translate(&mut self, widget: QWidgetId, dx: f32, dy: f32) {
        self.translate_inner(widget, dx, dy);
    }

    fn translate_inner(&mut self, widget: QWidgetId, dx: f32, dy: f32) {
        let Some(node) = self.widgets.get_mut(widget) else {
            return;
        };
        node.geometry.x += dx;
        node.geometry.y += dy;
        let children = node.children.clone();
        for child in children {
            self.translate_inner(child, dx, dy);
        }
    }

    pub fn set_geometry(&mut self, widget: QWidgetId, rect: QRectF) {
        self.layout(widget, rect);
    }

    pub fn relayout(&mut self, widget: QWidgetId) {
        let rect = self.widgets[widget].geometry;
        self.layout(widget, rect);
    }

    pub fn text(&self, widget: QWidgetId) -> Option<String> {
        let widget = self.widgets.get(widget)?;
        match widget.property("text") {
            Some(QVariant::String(text)) | Some(QVariant::Enum(text)) => Some(text.clone()),
            Some(QVariant::Int(value)) => Some(value.to_string()),
            Some(QVariant::Double(value)) => Some(value.to_string()),
            _ => None,
        }
    }

    pub fn set_text(&mut self, widget: QWidgetId, text: String) -> bool {
        let length = text.chars().count();
        let Some(widget) = self.widgets.get_mut(widget) else {
            return false;
        };
        widget.set_property("text", QVariant::String(text));
        widget.cursor_position = length;
        widget.selection_anchor = None;
        widget.cursor_visible = true;
        true
    }

    pub fn insert_text(&mut self, widget: QWidgetId, ch: char) -> bool {
        let text = self.text(widget).unwrap_or_default();
        let selection = self.line_edit_selection(widget);
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let mut chars: Vec<char> = text.chars().collect();
        let (start, end) = selection.unwrap_or_else(|| {
            let cursor = node.cursor_position.min(chars.len());
            (cursor, cursor)
        });
        if start < end {
            chars.drain(start..end);
        }
        chars.insert(start, ch);
        let text: String = chars.into_iter().collect();
        node.set_property("text", QVariant::String(text));
        node.cursor_position = start + 1;
        node.selection_anchor = None;
        node.cursor_visible = true;
        true
    }

    pub fn backspace_text(&mut self, widget: QWidgetId) -> bool {
        let text = self.text(widget).unwrap_or_default();
        let selection = self.line_edit_selection(widget);
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let mut chars: Vec<char> = text.chars().collect();
        let cursor = node.cursor_position.min(chars.len());
        let (start, end) = match selection {
            Some(selection) => selection,
            None if cursor == 0 => return false,
            None => (cursor - 1, cursor),
        };
        chars.drain(start..end);
        let text: String = chars.into_iter().collect();
        node.set_property("text", QVariant::String(text));
        node.cursor_position = start;
        node.selection_anchor = None;
        node.cursor_visible = true;
        true
    }

    pub fn delete_text(&mut self, widget: QWidgetId) -> bool {
        let text = self.text(widget).unwrap_or_default();
        let selection = self.line_edit_selection(widget);
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let mut chars: Vec<char> = text.chars().collect();
        let cursor = node.cursor_position.min(chars.len());
        let (start, end) = match selection {
            Some(selection) => selection,
            None if cursor >= chars.len() => return false,
            None => (cursor, cursor + 1),
        };
        chars.drain(start..end);
        let text: String = chars.into_iter().collect();
        node.set_property("text", QVariant::String(text));
        node.cursor_position = start;
        node.selection_anchor = None;
        node.cursor_visible = true;
        true
    }

    pub fn cursor_position(&self, widget: QWidgetId) -> usize {
        self.widgets
            .get(widget)
            .map(|widget| widget.cursor_position)
            .unwrap_or(0)
    }

    pub fn cursor_visible(&self, widget: QWidgetId) -> bool {
        self.widgets
            .get(widget)
            .map(|widget| widget.cursor_visible)
            .unwrap_or(false)
    }

    pub fn set_cursor_visible(&mut self, widget: QWidgetId, visible: bool) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        node.cursor_visible = visible;
        true
    }

    pub fn set_cursor_position(&mut self, widget: QWidgetId, position: usize) -> bool {
        self.set_cursor_position_ext(widget, position, false)
    }

    pub fn set_cursor_position_ext(
        &mut self,
        widget: QWidgetId,
        position: usize,
        extend_selection: bool,
    ) -> bool {
        let length = self.text(widget).unwrap_or_default().chars().count();
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let previous = node.cursor_position.min(length);
        let position = position.min(length);
        if extend_selection {
            if node.selection_anchor.is_none() {
                node.selection_anchor = Some(previous);
            }
        } else {
            node.selection_anchor = None;
        }
        node.cursor_position = position;
        node.cursor_visible = true;
        true
    }

    pub fn select_line_edit_to(&mut self, widget: QWidgetId, local_x: f32) -> bool {
        let Some(position) = self.cursor_at_x(widget, local_x) else {
            return false;
        };
        self.set_cursor_position_ext(widget, position, true)
    }

    pub fn checked(&self, widget: QWidgetId) -> bool {
        matches!(self.property(widget, "checked"), Some(QVariant::Bool(true)))
    }

    fn radio_auto_exclusive(&self, widget: QWidgetId) -> bool {
        let Some(node) = self.widgets.get(widget) else {
            return false;
        };
        node.class_name == "QRadioButton"
            && !matches!(node.property("autoExclusive"), Some(QVariant::Bool(false)))
    }

    pub fn set_checked(&mut self, widget: QWidgetId, checked: bool) -> bool {
        let Some(node) = self.widgets.get(widget) else {
            return false;
        };
        let parent = node.parent;
        if checked && self.radio_auto_exclusive(widget) {
            if let Some(parent) = parent {
                let siblings = self.widgets[parent].children.clone();
                for sibling in siblings {
                    if sibling != widget
                        && self.radio_auto_exclusive(sibling)
                        && self.checked(sibling)
                    {
                        self.widgets[sibling].set_property("checked", QVariant::Bool(false));
                    }
                }
            }
        }
        self.widgets[widget].set_property("checked", QVariant::Bool(checked));
        true
    }

    pub fn toggle_checked(&mut self, widget: QWidgetId) -> bool {
        let checked = self.checked(widget);
        if checked && self.radio_auto_exclusive(widget) {
            return true;
        }
        self.set_checked(widget, !checked);
        !checked
    }

    pub fn select_all(&mut self, widget: QWidgetId) -> bool {
        let length = self.text(widget).unwrap_or_default().chars().count();
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        node.selection_anchor = Some(0);
        node.cursor_position = length;
        node.cursor_visible = true;
        true
    }

    /// Select the word under `local_x` (double tap/click in a line edit).
    /// Word characters follow Qt closely enough for UI text: Unicode
    /// alphanumerics plus `_`; punctuation/whitespace selects one character.
    pub fn select_word_at(&mut self, widget: QWidgetId, local_x: f32) -> bool {
        if self
            .widgets
            .get(widget)
            .map(|node| node.class_name.as_str())
            != Some("QLineEdit")
        {
            return false;
        }
        let Some(position) = self.cursor_at_x(widget, local_x) else {
            return false;
        };
        let text: Vec<char> = self.text(widget).unwrap_or_default().chars().collect();
        if text.is_empty() {
            return false;
        }
        let word = |ch: char| ch.is_alphanumeric() || ch == '_';
        let (start, end) = if position < text.len() && word(text[position]) {
            let mut start = position;
            while start > 0 && word(text[start - 1]) {
                start -= 1;
            }
            let mut end = position;
            while end < text.len() && word(text[end]) {
                end += 1;
            }
            (start, end)
        } else if position > 0 && word(text[position - 1]) {
            let mut start = position - 1;
            while start > 0 && word(text[start - 1]) {
                start -= 1;
            }
            (start, position)
        } else {
            let start = position.min(text.len().saturating_sub(1));
            (start, (start + 1).min(text.len()))
        };
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        node.selection_anchor = Some(start);
        node.cursor_position = end;
        node.cursor_visible = true;
        true
    }

    pub fn clear_line_edit_selection(&mut self, widget: QWidgetId) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        node.selection_anchor = None;
        true
    }

    pub fn line_edit_selection(&self, widget: QWidgetId) -> Option<(usize, usize)> {
        let node = self.widgets.get(widget)?;
        let anchor = node.selection_anchor?;
        let cursor = node.cursor_position;
        let start = anchor.min(cursor);
        let end = anchor.max(cursor);
        (start < end).then_some((start, end))
    }

    fn cursor_at_x(&self, widget: QWidgetId, local_x: f32) -> Option<usize> {
        let node = self.widgets.get(widget)?;
        if node.class_name != "QLineEdit" {
            return None;
        }
        let text = self.text(widget).unwrap_or_default();
        let metrics = QFontMetrics::new(&node.font);
        let mut best = 0;
        let mut best_distance = f32::MAX;
        for index in 0..=text.chars().count() {
            let prefix: String = text.chars().take(index).collect();
            let width = 3.0 + metrics.horizontal_advance(&prefix);
            let distance = (local_x - width).abs();
            if distance < best_distance {
                best = index;
                best_distance = distance;
            }
        }
        Some(best)
    }

    pub fn get_table_property(&self, widget: QWidgetId, property: &str) -> Option<QVariant> {
        let node = self.widgets.get(widget)?;
        if let Some(table) = node.table.as_deref() {
            return match property {
                "rowCount" => Some(QVariant::Int(table.model.row_count() as i64)),
                "columnCount" => Some(QVariant::Int(table.model.column_count() as i64)),
                "currentRow" => Some(table.current_row_property()),
                "currentColumn" => Some(QVariant::Int(
                    table
                        .current_column()
                        .map(|column| column as i64)
                        .unwrap_or(-1),
                )),
                "verticalScrollBarPolicy" => Some(scroll_policy_variant(table.vbar.policy)),
                "horizontalScrollBarPolicy" => Some(scroll_policy_variant(table.hbar.policy)),
                _ => {
                    let column = property.strip_prefix("currentRowCell:")?;
                    let column = column.parse::<i64>().ok()?;
                    table.current_row_cell(column)
                }
            };
        }
        if let Some(list) = node.list.as_deref() {
            return match property {
                "rowCount" => Some(QVariant::Int(list.row_count() as i64)),
                "currentRow" | "currentIndex" => Some(list.current_row_property()),
                "currentText" => Some(list.current_text()),
                "verticalScrollBarPolicy" => Some(scroll_policy_variant(list.vbar.policy)),
                "horizontalScrollBarPolicy" => Some(scroll_policy_variant(list.hbar.policy)),
                _ => None,
            };
        }
        let tree = node.tree.as_deref()?;
        match property {
            "rowCount" => Some(QVariant::Int(tree.row_count() as i64)),
            "currentRow" | "currentIndex" => Some(tree.current_row_property()),
            "currentText" => Some(tree.current_text()),
            "verticalScrollBarPolicy" => Some(scroll_policy_variant(tree.vbar.policy)),
            "horizontalScrollBarPolicy" => Some(scroll_policy_variant(tree.hbar.policy)),
            _ => None,
        }
    }

    pub fn table(&self, id: QWidgetId) -> Option<&QTableWidget> {
        self.widgets.get(id)?.table.as_deref()
    }

    pub fn table_mut(&mut self, id: QWidgetId) -> Option<&mut QTableWidget> {
        self.widgets.get_mut(id)?.table.as_deref_mut()
    }

    pub fn list(&self, id: QWidgetId) -> Option<&QListWidget> {
        self.widgets.get(id)?.list.as_deref()
    }

    pub fn list_mut(&mut self, id: QWidgetId) -> Option<&mut QListWidget> {
        self.widgets.get_mut(id)?.list.as_deref_mut()
    }

    pub fn tree(&self, id: QWidgetId) -> Option<&QTreeWidget> {
        self.widgets.get(id)?.tree.as_deref()
    }

    pub fn tree_mut(&mut self, id: QWidgetId) -> Option<&mut QTreeWidget> {
        self.widgets.get_mut(id)?.tree.as_deref_mut()
    }

    fn root_of(&self, widget: QWidgetId) -> Option<QWidgetId> {
        let mut current = Some(widget);
        let mut root = None;
        while let Some(id) = current {
            root = Some(id);
            current = self.widgets.get(id).and_then(|node| node.parent);
        }
        root
    }

    /// Popup rectangle in form coordinates, mirroring the way Qt places the
    /// combo container: at least as wide as the combo, below it when there is
    /// room, otherwise above, and clamped inside the form.
    pub fn combo_popup_rect(&self, widget: QWidgetId) -> Option<QRectF> {
        let node = self.widgets.get(widget)?;
        if node.class_name != "QComboBox" || node.combo_popup.is_none() {
            return None;
        }
        let items = combo_items(node);
        if items.is_empty() {
            return None;
        }
        let metrics = QFontMetrics::new(&node.font);
        let visible = combo_visible_rows(node, items.len());
        let content_width = items
            .iter()
            .map(|item| metrics.horizontal_advance(item))
            .fold(0.0_f32, f32::max)
            + 10.0;
        let root = self.root_of(widget)?;
        let root_rect = self.widgets.get(root)?.geometry;
        let width = node
            .geometry
            .width
            .max(content_width)
            .min(root_rect.width.max(node.geometry.width));
        let height = (visible as f32 * QCOMBO_POPUP_ROW_H + 2.0).min(root_rect.height.max(1.0));
        let mut x = node.geometry.x;
        let mut y = node.geometry.bottom();
        if y + height > root_rect.bottom() {
            let above = node.geometry.y - height;
            y = if above >= root_rect.y {
                above
            } else {
                (root_rect.bottom() - height).max(root_rect.y)
            };
        }
        x = x.min(root_rect.right() - width).max(root_rect.x);
        Some(QRectF::new(x, y, width, height))
    }

    pub fn combo_popup_widget(&self) -> Option<QWidgetId> {
        self.widgets
            .iter()
            .position(|widget| widget.combo_popup.is_some())
    }

    /// Hit-test an open popup. `Some((combo, Some(row)))` is an item hit;
    /// `Some((combo, None))` is the popup frame (a click that only dismisses
    /// the popup must not fall through to the widget below).
    pub fn combo_popup_hit(&self, point: QPointF) -> Option<(QWidgetId, Option<usize>)> {
        for (id, widget) in self.widgets.iter().enumerate().rev() {
            if widget.combo_popup.is_none() {
                continue;
            }
            let rect = self.combo_popup_rect(id)?;
            if !rect.contains(point) {
                continue;
            }
            let popup = widget.combo_popup.as_ref()?;
            let items = combo_items(widget);
            let row = ((point.y - rect.y - 1.0) / QCOMBO_POPUP_ROW_H).floor();
            let row = if row < 0.0 {
                None
            } else {
                let index = popup.scroll + row as usize;
                (index < items.len()).then_some(index)
            };
            return Some((id, row));
        }
        None
    }

    pub fn combo_popup_open(&self, widget: QWidgetId) -> bool {
        self.widgets
            .get(widget)
            .is_some_and(|node| node.combo_popup.is_some())
    }

    pub fn open_combo_popup(&mut self, widget: QWidgetId) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        if node.class_name != "QComboBox" || !node.enabled || !node.visible {
            return false;
        }
        let items = combo_items(node);
        if items.is_empty() {
            return false;
        }
        let current = node
            .property("currentIndex")
            .and_then(value_as_i64)
            .unwrap_or(0)
            .clamp(0, items.len() as i64 - 1) as usize;
        let visible = combo_visible_rows(node, items.len());
        node.combo_popup = Some(QComboBoxPopup {
            highlight: current,
            scroll: current.saturating_sub(visible.saturating_sub(1)),
        });
        true
    }

    pub fn close_combo_popup(&mut self, widget: QWidgetId) {
        if let Some(node) = self.widgets.get_mut(widget) {
            node.combo_popup = None;
        }
    }

    pub fn close_combo_popups(&mut self) {
        for widget in &mut self.widgets {
            widget.combo_popup = None;
        }
    }

    pub fn combo_current_index(&self, widget: QWidgetId) -> Option<usize> {
        let node = self.widgets.get(widget)?;
        let items = combo_items(node);
        if items.is_empty() {
            return None;
        }
        Some(
            node.property("currentIndex")
                .and_then(value_as_i64)
                .unwrap_or(0)
                .clamp(0, items.len() as i64 - 1) as usize,
        )
    }

    /// Set `currentIndex` and close the popup, like selecting an item in the
    /// QComboBox popup. Returns `true` when the index changed.
    pub fn select_combo_index(&mut self, widget: QWidgetId, index: usize) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let items = combo_items(node);
        if index >= items.len() {
            return false;
        }
        let changed = self
            .widgets
            .get(widget)
            .and_then(|node| node.property("currentIndex"))
            .and_then(value_as_i64)
            .unwrap_or(0)
            != index as i64;
        self.widgets[widget].set_property("currentIndex", QVariant::Int(index as i64));
        self.widgets[widget].combo_popup = None;
        changed
    }

    pub fn combo_popup_highlight(&self, widget: QWidgetId) -> Option<usize> {
        self.widgets
            .get(widget)
            .and_then(|node| node.combo_popup.as_ref())
            .map(|popup| popup.highlight)
    }

    pub fn move_combo_highlight(&mut self, widget: QWidgetId, delta: i64) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let items = combo_items(node);
        if items.is_empty() {
            return false;
        }
        if node.combo_popup.is_none() {
            return false;
        }
        let visible = combo_visible_rows(node, items.len());
        let Some(popup) = node.combo_popup.as_mut() else {
            return false;
        };
        let next = (popup.highlight as i64 + delta).clamp(0, items.len() as i64 - 1);
        popup.highlight = next as usize;
        if popup.highlight < popup.scroll {
            popup.scroll = popup.highlight;
        } else if popup.highlight >= popup.scroll + visible {
            popup.scroll = popup.highlight + 1 - visible;
        }
        true
    }

    pub fn set_combo_highlight(&mut self, widget: QWidgetId, index: usize) -> bool {
        let Some(node) = self.widgets.get_mut(widget) else {
            return false;
        };
        let items = combo_items(node);
        if index >= items.len() {
            return false;
        }
        if node.combo_popup.is_none() {
            return false;
        }
        let visible = combo_visible_rows(node, items.len());
        let Some(popup) = node.combo_popup.as_mut() else {
            return false;
        };
        popup.highlight = index;
        if index < popup.scroll {
            popup.scroll = index;
        } else if index >= popup.scroll + visible {
            popup.scroll = index + 1 - visible;
        }
        true
    }

    pub fn commit_combo_popup(&mut self, widget: QWidgetId) -> bool {
        let Some(index) = self.combo_popup_highlight(widget) else {
            return false;
        };
        if self.combo_current_index(widget) == Some(index) {
            self.close_combo_popup(widget);
            return false;
        }
        self.select_combo_index(widget, index)
    }

    /// Splitter containing the point, plus the handle index (between panes
    /// `handle` and `handle + 1`).
    pub fn splitter_handle_at(&self, point: QPointF) -> Option<(QWidgetId, usize)> {
        for (id, widget) in self.widgets.iter().enumerate().rev() {
            let Some(splitter) = widget.splitter.as_deref() else {
                continue;
            };
            if !widget.visible || !widget.geometry.contains(point) {
                continue;
            }
            let horizontal = splitter.is_horizontal();
            for (index, pair) in widget.children.windows(2).enumerate() {
                let Some(a) = self.widgets.get(pair[0]) else {
                    continue;
                };
                let Some(b) = self.widgets.get(pair[1]) else {
                    continue;
                };
                let (start, end) = if horizontal {
                    (a.geometry.right(), b.geometry.x)
                } else {
                    (a.geometry.bottom(), b.geometry.y)
                };
                if end <= start {
                    continue;
                }
                let axis = if horizontal { point.x } else { point.y };
                if axis >= start && axis < end {
                    return Some((id, index));
                }
            }
        }
        None
    }

    fn splitter_minimum(&mut self, widget: QWidgetId) -> QSizeF {
        let Some(node) = self.widgets.get(widget) else {
            return QSizeF::default();
        };
        let Some(splitter) = node.splitter.as_deref() else {
            return QSizeF::default();
        };
        let horizontal = splitter.is_horizontal();
        let children = node.children.clone();
        let handle = splitter.handle_width;
        let mut main = handle * children.len().saturating_sub(1) as f32;
        let mut perp = 0.0_f32;
        for child in children {
            let min = self.minimum_size_hint(child);
            if horizontal {
                main += min.width;
                perp = perp.max(min.height);
            } else {
                main += min.height;
                perp = perp.max(min.width);
            }
        }
        if horizontal {
            QSizeF::new(main, perp)
        } else {
            QSizeF::new(perp, main)
        }
    }

    fn tab_bar_height() -> f32 {
        // Matches `QFusionStyle::pixel_metric(TabBarHeight)` / Qt probe (~28).
        28.0
    }

    /// Sync page visibility to `tabs.current_index` (only the current page
    /// is visible, like `QStackedWidget` inside `QTabWidget`).
    fn sync_tab_pages(&mut self, widget: QWidgetId) {
        let Some(node) = self.widgets.get(widget) else {
            return;
        };
        let Some(tabs) = node.tabs.as_deref() else {
            return;
        };
        let current = tabs.current_index;
        let children = node.children.clone();
        for (index, child) in children.into_iter().enumerate() {
            if let Some(page) = self.widgets.get_mut(child) {
                page.visible = index == current;
            }
        }
    }

    /// Absolute tab label rectangles left-to-right inside the tab bar.
    pub fn tab_rects(&self, widget: QWidgetId) -> Vec<QRectF> {
        let Some(node) = self.widgets.get(widget) else {
            return Vec::new();
        };
        let Some(tabs) = node.tabs.as_deref() else {
            return Vec::new();
        };
        let frame = node.geometry;
        let bar_h = Self::tab_bar_height();
        let metrics = QFontMetrics::new(&node.font);
        // Flush with the tab widget's left edge (Qt probe: no 2px inset).
        let mut x = frame.x;
        let mut rects = Vec::with_capacity(tabs.titles.len());
        for (index, title) in tabs.titles.iter().enumerate() {
            let label = if title.is_empty() {
                format!("Tab {}", index + 1)
            } else {
                title.clone()
            };
            let width = (metrics.horizontal_advance(&label) + QTAB_H_SPACE).max(40.0);
            let selected = index == tabs.current_index;
            let y = if selected {
                frame.y
            } else {
                frame.y + 2.0
            };
            let height = if selected { bar_h } else { bar_h - 2.0 };
            rects.push(QRectF::new(x, y, width, height));
            x += width - 1.0;
        }
        rects
    }

    pub fn tab_at(&self, point: QPointF) -> Option<(QWidgetId, usize)> {
        for (id, widget) in self.widgets.iter().enumerate().rev() {
            if widget.tabs.is_none() || !widget.visible || !widget.enabled {
                continue;
            }
            let bar = QRectF::new(
                widget.geometry.x,
                widget.geometry.y,
                widget.geometry.width,
                Self::tab_bar_height(),
            );
            if !bar.contains(point) {
                continue;
            }
            for (index, rect) in self.tab_rects(id).into_iter().enumerate() {
                if rect.contains(point) {
                    return Some((id, index));
                }
            }
        }
        None
    }

    pub fn set_tab_current_index(&mut self, widget: QWidgetId, index: usize) -> bool {
        let geometry = {
            let Some(node) = self.widgets.get_mut(widget) else {
                return false;
            };
            let page_count = node.children.len();
            let Some(tabs) = node.tabs.as_mut() else {
                return false;
            };
            if index >= tabs.titles.len().max(page_count) {
                return false;
            }
            if tabs.current_index == index {
                return false;
            }
            tabs.current_index = index;
            node.set_property("currentIndex", QVariant::Int(index as i64));
            node.geometry
        };
        self.sync_tab_pages(widget);
        self.layout_tab_widget(widget, geometry);
        true
    }

    fn measure_tab_widget(&mut self, widget: QWidgetId) -> QSizeF {
        let Some(node) = self.widgets.get(widget) else {
            return QSizeF::default();
        };
        let children = node.children.clone();
        let titles = node
            .tabs
            .as_deref()
            .map(|tabs| tabs.titles.clone())
            .unwrap_or_default();
        let font = node.font.clone();
        let mut page_w = 0.0_f32;
        let mut page_h = 0.0_f32;
        for child in &children {
            let hint = self.measure(*child);
            page_w = page_w.max(hint.width);
            page_h = page_h.max(hint.height);
        }
        let metrics = QFontMetrics::new(&font);
        let mut bar_w = 0.0_f32;
        for (index, title) in titles.iter().enumerate() {
            let label = if title.is_empty() {
                format!("Tab {}", index + 1)
            } else {
                title.clone()
            };
            bar_w += (metrics.horizontal_advance(&label) + QTAB_H_SPACE).max(40.0) - 1.0;
        }
        bar_w = bar_w.max(0.0) + 1.0;
        let hint = QSizeF::new(page_w.max(bar_w), page_h + Self::tab_bar_height());
        if let Some(node) = self.widgets.get_mut(widget) {
            node.size_hint = hint;
        }
        hint
    }

    fn layout_tab_widget(&mut self, widget: QWidgetId, rect: QRectF) {
        if let Some(node) = self.widgets.get_mut(widget) {
            node.geometry = rect;
        }
        let children = self
            .widgets
            .get(widget)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        let bar_h = Self::tab_bar_height();
        // Inset like Qt's stacked widget (≈2px) so the pane outline stays
        // visible after the page fills Base.
        let inset = 1.0;
        let page = QRectF::new(
            rect.x + inset,
            rect.y + bar_h,
            (rect.width - inset * 2.0).max(0.0),
            (rect.height - bar_h - inset).max(0.0),
        );
        for child in children {
            self.layout_widget(child, page);
        }
        self.sync_tab_pages(widget);
    }

    fn measure_splitter(&mut self, widget: QWidgetId) -> QSizeF {
        let Some(node) = self.widgets.get(widget) else {
            return QSizeF::default();
        };
        let Some(splitter) = node.splitter.as_deref() else {
            return QSizeF::default();
        };
        let horizontal = splitter.is_horizontal();
        let handle = splitter.handle_width;
        let children = node.children.clone();
        let mut main = handle * children.len().saturating_sub(1) as f32;
        let mut perp = 0.0_f32;
        for child in children {
            let hint = self.measure(child);
            if horizontal {
                main += hint.width;
                perp = perp.max(hint.height);
            } else {
                main += hint.height;
                perp = perp.max(hint.width);
            }
        }
        let hint = if horizontal {
            QSizeF::new(main, perp)
        } else {
            QSizeF::new(perp, main)
        };
        if let Some(node) = self.widgets.get_mut(widget) {
            node.size_hint = hint;
        }
        hint
    }

    fn layout_splitter(&mut self, widget: QWidgetId, rect: QRectF) {
        let (horizontal, handle, children, mut desired) = {
            let Some(node) = self.widgets.get(widget) else {
                return;
            };
            let Some(splitter) = node.splitter.as_deref() else {
                return;
            };
            (
                splitter.is_horizontal(),
                splitter.handle_width,
                node.children.clone(),
                splitter.sizes.clone(),
            )
        };
        let count = children.len();
        if count == 0 {
            return;
        }
        let total = ((if horizontal { rect.width } else { rect.height })
            - handle * count.saturating_sub(1) as f32)
            .max(0.0);
        if desired.len() != count {
            desired = children
                .iter()
                .map(|child| {
                    let hint = self
                        .widgets
                        .get(*child)
                        .map(|node| node.size_hint)
                        .unwrap_or_default();
                    if horizontal {
                        hint.width
                    } else {
                        hint.height
                    }
                })
                .collect();
        }
        let desired_sum: f32 = desired.iter().sum();
        let mut sizes: Vec<f32> = if desired_sum > 0.0 {
            desired
                .iter()
                .map(|size| size / desired_sum * total)
                .collect()
        } else {
            vec![total / count as f32; count]
        };
        let minimums: Vec<f32> = children
            .iter()
            .map(|child| {
                let min = self.minimum_size_hint(*child);
                (if horizontal { min.width } else { min.height }).max(0.0)
            })
            .collect();
        for _ in 0..count {
            let deficit: f32 = sizes
                .iter()
                .zip(&minimums)
                .map(|(size, min)| (min - size).max(0.0))
                .sum();
            if deficit <= 0.01 {
                break;
            }
            let free: f32 = sizes
                .iter()
                .zip(&minimums)
                .map(|(size, min)| (size - min).max(0.0))
                .sum();
            if free <= 0.01 {
                break;
            }
            for (size, min) in sizes.iter_mut().zip(&minimums) {
                let removable = (*size - *min).max(0.0);
                if removable > 0.0 {
                    *size -= deficit * removable / free;
                }
            }
        }
        for (size, min) in sizes.iter_mut().zip(&minimums) {
            *size = size.max(*min).max(0.0);
        }
        let mut cursor = if horizontal { rect.x } else { rect.y };
        let end = if horizontal {
            rect.right()
        } else {
            rect.bottom()
        };
        for (index, child) in children.iter().enumerate() {
            let mut length = sizes[index];
            if index + 1 == count {
                length = (end - cursor).max(0.0);
            }
            let child_rect = if horizontal {
                QRectF::new(cursor, rect.y, length, rect.height)
            } else {
                QRectF::new(rect.x, cursor, rect.width, length)
            };
            self.layout_widget(*child, child_rect);
            cursor += length + handle;
        }
        if let Some(node) = self.widgets.get_mut(widget) {
            if let Some(splitter) = node.splitter.as_mut() {
                splitter.sizes = sizes;
            }
        }
    }

    fn drag_splitter(&mut self, delta: f32) -> bool {
        let Some(drag) = self.splitter_drag.clone() else {
            return false;
        };
        let (horizontal, mut sizes, children) = {
            let Some(node) = self.widgets.get(drag.widget) else {
                return false;
            };
            let Some(splitter) = node.splitter.as_deref() else {
                return false;
            };
            (
                splitter.is_horizontal(),
                splitter.sizes.clone(),
                node.children.clone(),
            )
        };
        if sizes.len() != children.len() || drag.handle + 1 >= sizes.len() {
            return false;
        }
        let (first_min, second_min) = {
            let a = self.minimum_size_hint(children[drag.handle]);
            let b = self.minimum_size_hint(children[drag.handle + 1]);
            (
                (if horizontal { a.width } else { a.height }).max(0.0),
                (if horizontal { b.width } else { b.height }).max(0.0),
            )
        };
        let total = sizes[drag.handle] + sizes[drag.handle + 1];
        let maximum_first = (total - second_min).max(first_min);
        let first = (sizes[drag.handle] + delta).clamp(first_min, maximum_first);
        let second = (total - first).max(second_min);
        sizes[drag.handle] = first;
        sizes[drag.handle + 1] = second;
        if let Some(node) = self.widgets.get_mut(drag.widget) {
            if let Some(splitter) = node.splitter.as_mut() {
                splitter.sizes = sizes;
            }
        }
        let rect = self
            .widgets
            .get(drag.widget)
            .map(|node| node.geometry)
            .unwrap_or_default();
        self.layout_splitter(drag.widget, rect);
        self.sync_views();
        true
    }

    pub fn sync_views(&mut self) {
        for widget in &mut self.widgets {
            let rect = widget.geometry;
            let font = widget.font.clone();
            if let Some(table) = widget.table.as_mut() {
                table.sync_scrollbars(rect, &font);
            }
            if let Some(list) = widget.list.as_mut() {
                let layout = list.scroll_area(rect);
                list.sync_bars(layout.viewport.size());
            }
            if let Some(tree) = widget.tree.as_mut() {
                let layout = tree.scroll_area(rect);
                tree.sync_bars(QSizeF::new(
                    layout.viewport.width,
                    (layout.viewport.height - QVIEW_HEADER_H).max(0.0),
                ));
            }
        }
    }

    pub fn property(&self, id: QWidgetId, name: &str) -> Option<&QVariant> {
        self.widgets.get(id)?.property(name)
    }

    pub fn find_widget(&self, object_name: &str) -> Option<QWidgetId> {
        self.widgets
            .iter()
            .position(|widget| widget.object_name == object_name)
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Find the topmost widget at the given point (iterating from last to first for z-order)
    pub fn widget_at_point(&self, point: QPointF) -> Option<QWidgetId> {
        // Iterate in reverse order (last added = topmost) to find the top widget at this point
        for i in (0..self.widgets.len()).rev() {
            let widget = &self.widgets[i];
            if widget.visible && widget.enabled && widget.geometry.contains(point) {
                return Some(i);
            }
        }
        None
    }

    pub fn set_enabled(&mut self, widget: QWidgetId, enabled: bool) -> bool {
        let Some(widget) = self.widgets.get_mut(widget) else {
            return false;
        };
        widget.enabled = enabled;
        if !enabled && widget.has_focus {
            if widget.class_name == "QLineEdit" {
                widget.selection_anchor = None;
            }
            widget.has_focus = false;
        }
        true
    }

    pub fn set_visible(&mut self, widget: QWidgetId, visible: bool) -> bool {
        let Some(widget) = self.widgets.get_mut(widget) else {
            return false;
        };
        widget.visible = visible;
        true
    }

    pub fn focus_first(&mut self, root: QWidgetId) -> Option<QWidgetId> {
        let mut order = Vec::new();
        self.collect_focusable(root, &mut order);
        let first = *order
            .iter()
            .find(|id| self.widgets[**id].class_name == "QLineEdit")
            .or_else(|| {
                order
                    .iter()
                    .find(|id| self.widgets[**id].class_name == "QTableWidget")
            })?;
        self.set_focus(first);
        if self.widgets[first].class_name == "QLineEdit" {
            let length = self.text(first).unwrap_or_default().chars().count();
            self.set_cursor_position(first, length);
        }
        Some(first)
    }

    pub fn focus_widget(&self) -> Option<QWidgetId> {
        self.widgets.iter().position(|widget| widget.has_focus)
    }

    pub fn clear_focus(&mut self) {
        for widget in &mut self.widgets {
            if widget.has_focus && widget.class_name == "QLineEdit" {
                widget.selection_anchor = None;
            }
            widget.has_focus = false;
        }
    }

    pub fn set_focus(&mut self, widget: QWidgetId) -> bool {
        let Some(target) = self.widgets.get(widget) else {
            return false;
        };
        if !target.visible || !target.enabled || target.focus_policy == QFocusPolicy::NoFocus {
            return false;
        }
        self.clear_focus();
        self.widgets[widget].has_focus = true;
        match self.widgets[widget].class_name.as_str() {
            "QTableWidget" => {
                if let Some(table) = self.widgets[widget].table.as_mut() {
                    table.ensure_current();
                }
            }
            "QListWidget" => {
                if let Some(list) = self.widgets[widget].list.as_mut() {
                    if list.current.is_none() && list.row_count() > 0 {
                        list.set_current(Some(0));
                    }
                }
            }
            "QTreeWidget" => {
                if let Some(tree) = self.widgets[widget].tree.as_mut() {
                    if tree.current.is_none() && tree.row_count() > 0 {
                        tree.set_current(Some(0));
                    }
                }
            }
            _ => {}
        }
        true
    }

    pub fn focus_next(&mut self, root: QWidgetId, backwards: bool) -> Option<QWidgetId> {
        let mut order = Vec::new();
        self.collect_focusable(root, &mut order);
        if order.is_empty() {
            return None;
        }
        let current = self
            .focus_widget()
            .and_then(|widget| order.iter().position(|id| *id == widget));
        let index = match (current, backwards) {
            (Some(index), false) => (index + 1) % order.len(),
            (Some(index), true) => (index + order.len() - 1) % order.len(),
            (None, false) => 0,
            (None, true) => order.len() - 1,
        };
        let next = order[index];
        self.set_focus(next);
        Some(next)
    }

    fn collect_focusable(&self, widget: QWidgetId, order: &mut Vec<QWidgetId>) {
        let Some(node) = self.widgets.get(widget) else {
            return;
        };
        if !node.visible || !node.enabled {
            return;
        }
        if node.focus_policy != QFocusPolicy::NoFocus {
            order.push(widget);
        }
        for child in &node.children {
            self.collect_focusable(*child, order);
        }
    }

    pub fn widget_at(&self, root: QWidgetId, point: QPointF) -> Option<QWidgetId> {
        self.widget_at_inner(root, point)
    }

    fn widget_at_inner(&self, widget: QWidgetId, point: QPointF) -> Option<QWidgetId> {
        let node = self.widgets.get(widget)?;
        // Qt delivers mouse events only inside the parent; a child that
        // overflows a shrunken form is clipped and is not hit outside it.
        if !node.visible || !node.enabled || !node.geometry.contains(point) {
            return None;
        }
        for child in node.children.iter().rev() {
            if let Some(hit) = self.widget_at_inner(*child, point) {
                return Some(hit);
            }
        }
        Some(widget)
    }

    pub fn scrollbar_at(&self, root: QWidgetId, point: QPointF) -> Option<QWidgetId> {
        let widget = self.widget_at(root, point)?;
        let layout = self.scroll_layout(widget)?;
        if layout.vertical.is_some_and(|rect| rect.contains(point))
            || layout.horizontal.is_some_and(|rect| rect.contains(point))
        {
            Some(widget)
        } else {
            None
        }
    }

    pub fn mouse_press(&mut self, root: QWidgetId, point: QPointF) -> Option<QWidgetEvent> {
        self.mouse_press_with_modifiers(root, point, false)
    }

    pub fn mouse_press_with_modifiers(
        &mut self,
        root: QWidgetId,
        point: QPointF,
        shift: bool,
    ) -> Option<QWidgetEvent> {
        // A popup is a top-level overlay: it receives the click before any
        // widget below it, and a click elsewhere dismisses it first.
        let open_popup = self.combo_popup_widget();
        if let Some((popup, row)) = self.combo_popup_hit(point) {
            if let Some(row) = row {
                self.set_combo_highlight(popup, row);
            }
            self.mouse_grab = Some(popup);
            return Some(QWidgetEvent {
                widget: popup,
                event_type: QEventType::MouseButtonPress,
                clicked: false,
            });
        }
        if open_popup.is_some() {
            self.close_combo_popups();
        }
        if let Some((splitter, handle)) = self.splitter_handle_at(point) {
            let horizontal = self
                .widgets
                .get(splitter)
                .and_then(|node| node.splitter.as_deref())
                .map(QSplitter::is_horizontal)
                .unwrap_or(true);
            self.splitter_drag = Some(QSplitterDrag {
                widget: splitter,
                handle,
                last: if horizontal { point.x } else { point.y },
            });
            self.mouse_grab = Some(splitter);
            return Some(QWidgetEvent {
                widget: splitter,
                event_type: QEventType::MouseButtonPress,
                clicked: false,
            });
        }
        if let Some((tabs, index)) = self.tab_at(point) {
            self.set_focus(tabs);
            self.set_tab_current_index(tabs, index);
            self.mouse_grab = Some(tabs);
            return Some(QWidgetEvent {
                widget: tabs,
                event_type: QEventType::MouseButtonPress,
                clicked: false,
            });
        }
        let widget = self.widget_at(root, point)?;
        if self.handle_scrollbar_press(widget, point) {
            self.mouse_grab = Some(widget);
            return Some(QWidgetEvent {
                widget,
                event_type: QEventType::MouseButtonPress,
                clicked: false,
            });
        }
        let policy = self.widgets[widget].focus_policy;
        if matches!(
            policy,
            QFocusPolicy::ClickFocus | QFocusPolicy::StrongFocus | QFocusPolicy::WheelFocus
        ) {
            self.set_focus(widget);
        }
        if self.widgets[widget].class_name == "QLineEdit" {
            let geometry = self.widgets[widget].geometry;
            if let Some(position) = self.cursor_at_x(widget, point.x - geometry.x) {
                self.set_cursor_position_ext(widget, position, shift);
            }
        }
        let class = self.widgets[widget].class_name.clone();
        let geometry = self.widgets[widget].geometry;
        let font = self.widgets[widget].font.clone();
        if class == "QComboBox" {
            if open_popup != Some(widget) {
                self.open_combo_popup(widget);
            }
            self.mouse_grab = Some(widget);
            return Some(QWidgetEvent {
                widget,
                event_type: QEventType::MouseButtonPress,
                clicked: false,
            });
        }
        match class.as_str() {
            "QTableWidget" => {
                if let Some(table) = self.widgets[widget].table.as_mut() {
                    let viewport = table.scroll_area(geometry, &font).viewport;
                    if viewport.contains(point) {
                        let local_x = point.x - viewport.x;
                        let local_y = point.y - viewport.y;
                        let row_header_w = table.row_header_width(&font);
                        if let (Some(row), Some(column)) = (
                            table.row_at(local_y),
                            table.column_at(local_x, row_header_w, viewport.width),
                        ) {
                            table.set_current_cell(row, column);
                            table.select_current();
                        }
                    }
                }
            }
            "QListWidget" => {
                if let Some(list) = self.widgets[widget].list.as_mut() {
                    let layout = list.scroll_area(geometry);
                    let viewport = layout.viewport;
                    if viewport.contains(point) {
                        let local_y = point.y - viewport.y;
                        if let Some(row) = list.row_at(local_y) {
                            list.set_current(Some(row));
                        }
                    }
                }
            }
            "QTreeWidget" => {
                if let Some(tree) = self.widgets[widget].tree.as_mut() {
                    let layout = tree.scroll_area(geometry);
                    let viewport = layout.viewport;
                    let body = QRectF::new(
                        viewport.x,
                        viewport.y + QVIEW_HEADER_H,
                        viewport.width,
                        (viewport.height - QVIEW_HEADER_H).max(0.0),
                    );
                    if body.contains(point) {
                        let local_y = point.y - body.y;
                        let local_x = point.x - body.x + tree.horizontal;
                        if let Some(row) = tree.row_at(local_y) {
                            let branch = tree.rows.get(row).is_some_and(|row| {
                                row.has_children
                                    && local_x >= row.indent() + 2.0
                                    && local_x
                                        <= row.indent() + 2.0 + crate::item_view::QVIEW_BRANCH_W
                            });
                            tree.set_current(Some(row));
                            if branch {
                                tree.toggle_row_expanded(row);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.widgets[widget].pressed = self.is_abstract_button(widget);
        self.mouse_grab = Some(widget);
        Some(QWidgetEvent {
            widget,
            event_type: QEventType::MouseButtonPress,
            clicked: false,
        })
    }

    pub fn mouse_move(&mut self, point: QPointF) -> bool {
        if let Some(grab) = self.mouse_grab {
            if self.is_abstract_button(grab) {
                let inside = self.widgets.get(grab).is_some_and(|widget| {
                    widget.visible && widget.enabled && widget.geometry.contains(point)
                });
                self.widgets[grab].pressed = inside;
                return true;
            }
        }
        if let Some(drag) = self.splitter_drag.clone() {
            let horizontal = self
                .widgets
                .get(drag.widget)
                .and_then(|node| node.splitter.as_deref())
                .map(QSplitter::is_horizontal)
                .unwrap_or(true);
            let axis = if horizontal { point.x } else { point.y };
            let delta = axis - drag.last;
            if let Some(drag) = self.splitter_drag.as_mut() {
                drag.last = axis;
            }
            return self.drag_splitter(delta);
        }
        let Some(drag) = self.scroll_drag else {
            return false;
        };
        let Some(layout) = self.scroll_layout(drag.widget) else {
            self.scroll_drag = None;
            return false;
        };
        let bar_rect = if drag.vertical {
            layout.vertical
        } else {
            layout.horizontal
        };
        let Some(bar_rect) = bar_rect else {
            return false;
        };
        let value = {
            let Some(bar) = self.bar(drag.widget, drag.vertical) else {
                return false;
            };
            let groove = bar.groove_rect(bar_rect);
            let track = if drag.vertical {
                groove.height
            } else {
                groove.width
            };
            let length = bar.slider_length(track);
            let movable = (track - length).max(0.0);
            let span = bar.maximum - bar.minimum;
            if movable <= 0.0 || span <= 0.0 {
                bar.minimum
            } else {
                let axis = if drag.vertical { point.y } else { point.x };
                let groove_start = if drag.vertical { groove.y } else { groove.x };
                let start = axis - drag.offset;
                let fraction = ((start - groove_start) / movable).clamp(0.0, 1.0);
                bar.minimum + fraction * span
            }
        };
        self.set_view_scroll(drag.widget, drag.vertical, value);
        true
    }

    fn handle_scrollbar_press(&mut self, widget: QWidgetId, point: QPointF) -> bool {
        let Some(layout) = self.scroll_layout(widget) else {
            return false;
        };
        if let Some(rect) = layout.vertical {
            if rect.contains(point) {
                return self.press_scrollbar(widget, true, rect, point);
            }
        }
        if let Some(rect) = layout.horizontal {
            if rect.contains(point) {
                return self.press_scrollbar(widget, false, rect, point);
            }
        }
        false
    }

    fn press_scrollbar(
        &mut self,
        widget: QWidgetId,
        vertical: bool,
        rect: QRectF,
        point: QPointF,
    ) -> bool {
        let hit = {
            let Some(bar) = self.bar_mut(widget, vertical) else {
                return false;
            };
            let Some(hit) = bar.hit_test(rect, point) else {
                return false;
            };
            bar.pressed = Some(hit);
            match hit {
                QScrollBarSubControl::SubLine => {
                    bar.step_by(-1.0);
                }
                QScrollBarSubControl::AddLine => {
                    bar.step_by(1.0);
                }
                QScrollBarSubControl::SubPage => {
                    bar.page_by(-1.0);
                }
                QScrollBarSubControl::AddPage => {
                    bar.page_by(1.0);
                }
                QScrollBarSubControl::First
                | QScrollBarSubControl::Last
                | QScrollBarSubControl::Slider => {}
            }
            hit
        };
        let (value, slider_start) = {
            let Some(bar) = self.bar(widget, vertical) else {
                return false;
            };
            let slider = bar.slider_rect(rect);
            (bar.value, if vertical { slider.y } else { slider.x })
        };
        if hit == QScrollBarSubControl::Slider {
            let axis = if vertical { point.y } else { point.x };
            self.scroll_drag = Some(QScrollDrag {
                widget,
                vertical,
                offset: axis - slider_start,
            });
        }
        self.set_view_scroll(widget, vertical, value);
        true
    }

    fn scroll_layout(&self, widget: QWidgetId) -> Option<QScrollAreaLayout> {
        let node = self.widgets.get(widget)?;
        let rect = node.geometry;
        let font = &node.font;
        if let Some(table) = node.table.as_deref() {
            return Some(table.scroll_area(rect, font));
        }
        if let Some(list) = node.list.as_deref() {
            return Some(list.scroll_area(rect));
        }
        let tree = node.tree.as_deref()?;
        Some(tree.scroll_area(rect))
    }

    fn bar(&self, widget: QWidgetId, vertical: bool) -> Option<&QScrollBar> {
        let node = self.widgets.get(widget)?;
        if let Some(table) = node.table.as_deref() {
            return Some(if vertical { &table.vbar } else { &table.hbar });
        }
        if let Some(list) = node.list.as_deref() {
            return Some(if vertical { &list.vbar } else { &list.hbar });
        }
        let tree = node.tree.as_deref()?;
        Some(if vertical { &tree.vbar } else { &tree.hbar })
    }

    fn bar_mut(&mut self, widget: QWidgetId, vertical: bool) -> Option<&mut QScrollBar> {
        let node = self.widgets.get_mut(widget)?;
        if let Some(table) = node.table.as_mut() {
            return Some(if vertical {
                &mut table.vbar
            } else {
                &mut table.hbar
            });
        }
        if let Some(list) = node.list.as_mut() {
            return Some(if vertical {
                &mut list.vbar
            } else {
                &mut list.hbar
            });
        }
        let tree = node.tree.as_mut()?;
        Some(if vertical {
            &mut tree.vbar
        } else {
            &mut tree.hbar
        })
    }

    fn set_view_scroll(&mut self, widget: QWidgetId, vertical: bool, value: f32) {
        let Some(node) = self.widgets.get_mut(widget) else {
            return;
        };
        if let Some(table) = node.table.as_mut() {
            if vertical {
                table.scroll = value * crate::item_model::QTABLE_ROW_H;
                table.vbar.set_value(value);
            } else {
                table.horizontal = table.model.column_offset(value.max(0.0) as usize);
                table.hbar.set_value(value);
            }
            return;
        }
        if let Some(list) = node.list.as_mut() {
            if vertical {
                list.scroll = value * crate::item_view::QVIEW_ROW_H;
                list.vbar.set_value(value);
            } else {
                list.horizontal = value;
                list.hbar.set_value(value);
            }
            return;
        }
        if let Some(tree) = node.tree.as_mut() {
            if vertical {
                tree.scroll = value * crate::item_view::QVIEW_ROW_H;
                tree.vbar.set_value(value);
            } else {
                tree.horizontal = value;
                tree.hbar.set_value(value);
            }
        }
    }

    pub fn cancel_mouse_grab(&mut self) {
        self.mouse_grab = None;
        self.scroll_drag = None;
        self.splitter_drag = None;
        for widget in &mut self.widgets {
            widget.pressed = false;
            if let Some(table) = widget.table.as_mut() {
                table.vbar.pressed = None;
                table.hbar.pressed = None;
            }
            if let Some(list) = widget.list.as_mut() {
                list.vbar.pressed = None;
                list.hbar.pressed = None;
            }
            if let Some(tree) = widget.tree.as_mut() {
                tree.vbar.pressed = None;
                tree.hbar.pressed = None;
            }
        }
    }

    pub fn mouse_release(&mut self, root: QWidgetId, point: QPointF) -> Option<QWidgetEvent> {
        let grabbed = self.mouse_grab.take();
        self.scroll_drag = None;
        let splitter_drag = self.splitter_drag.take();
        if let Some(widget) = grabbed {
            if let Some(node) = self.widgets.get_mut(widget) {
                if let Some(table) = node.table.as_mut() {
                    table.vbar.pressed = None;
                    table.hbar.pressed = None;
                }
                if let Some(list) = node.list.as_mut() {
                    list.vbar.pressed = None;
                    list.hbar.pressed = None;
                }
                if let Some(tree) = node.tree.as_mut() {
                    tree.vbar.pressed = None;
                    tree.hbar.pressed = None;
                }
            }
        }
        if let Some(drag) = splitter_drag {
            return Some(QWidgetEvent {
                widget: drag.widget,
                event_type: QEventType::MouseButtonRelease,
                clicked: false,
            });
        }
        if let Some(widget) = grabbed {
            if self.widgets[widget].class_name == "QComboBox" {
                if let Some((popup, row)) = self.combo_popup_hit(point) {
                    if popup == widget {
                        if let Some(row) = row {
                            self.select_combo_index(widget, row);
                        } else {
                            self.close_combo_popup(widget);
                        }
                    }
                } else {
                    let inside_combo = self.widgets[widget].geometry.contains(point);
                    if !inside_combo {
                        self.close_combo_popup(widget);
                    }
                }
                return Some(QWidgetEvent {
                    widget,
                    event_type: QEventType::MouseButtonRelease,
                    clicked: false,
                });
            }
        }
        let hit = self.widget_at(root, point);
        let widget = grabbed.or(hit)?;
        let hit_started_here = grabbed.is_none() || hit == Some(widget);
        let clicked = self.is_abstract_button(widget)
            && self.widgets[widget].geometry.contains(point)
            && (self.widgets[widget].pressed || hit_started_here);
        if clicked
            && matches!(
                self.widgets[widget].class_name.as_str(),
                "QCheckBox" | "QRadioButton"
            )
        {
            self.toggle_checked(widget);
        }
        self.widgets[widget].pressed = false;
        Some(QWidgetEvent {
            widget,
            event_type: QEventType::MouseButtonRelease,
            clicked,
        })
    }

    fn is_abstract_button(&self, widget: QWidgetId) -> bool {
        matches!(
            self.widgets[widget].class_name.as_str(),
            "QPushButton" | "QToolButton" | "QCheckBox" | "QRadioButton"
        )
    }

    pub fn measure(&mut self, widget: QWidgetId) -> QSizeF {
        if self
            .widgets
            .get(widget)
            .is_some_and(|node| node.splitter.is_some())
        {
            return self.measure_splitter(widget);
        }
        if self
            .widgets
            .get(widget)
            .is_some_and(|node| node.tabs.is_some())
        {
            return self.measure_tab_widget(widget);
        }
        let Some(node) = self.widgets.get_mut(widget) else {
            return QSizeF::default();
        };
        let layout = node.layout.take();
        let hint = if let Some(layout) = &layout {
            let layout_hint = self.measure_layout(layout);
            if self.widgets[widget].class_name == "QGroupBox" {
                // The title contributes to the group's hint in Qt: a caption
                // wider than the children grows the whole window.
                let title = self.widgets[widget].size_hint_from_props();
                QSizeF::new(
                    layout_hint.width.max(title.width),
                    layout_hint.height.max(title.height),
                )
            } else {
                layout_hint
            }
        } else {
            self.widgets[widget].size_hint_from_props()
        };
        let node = &mut self.widgets[widget];
        node.size_hint = hint;
        node.layout = layout;
        hint
    }

    fn measure_layout(&mut self, layout: &QLayout) -> QSizeF {
        for item in &layout.items {
            if let QLayoutItem::Widget { id, .. } = item {
                self.measure(*id);
            } else if let QLayoutItem::Layout { layout: inner, .. } = item {
                self.measure_layout(inner);
            }
        }
        self.layout_size_hint(layout)
    }

    fn layout_size_hint(&self, layout: &QLayout) -> QSizeF {
        if layout.is_grid() {
            return self.grid_layout_size_hint(layout);
        }
        if layout.is_form() {
            return self.form_size_hint(layout);
        }
        let mut main = 0.0f32;
        let mut perp = 0.0f32;
        for item in &layout.items {
            let size = match item {
                QLayoutItem::Widget { id, .. } => self.widget_hint(*id),
                QLayoutItem::Spacer { item: spacer, .. } => {
                    QSizeF::new(spacer.width, spacer.height)
                }
                QLayoutItem::Layout { layout, .. } => self.layout_size_hint(layout),
            };
            let (item_main, item_perp) = if layout.is_horizontal() {
                (size.width, size.height)
            } else {
                (size.height, size.width)
            };
            main += item_main;
            perp = perp.max(item_perp);
        }
        if layout.items.len() > 1 {
            main += layout.spacing * (layout.items.len() as f32 - 1.0);
        }
        if layout.is_horizontal() {
            QSizeF::new(
                main + layout.margins.left + layout.margins.right,
                perp + layout.margins.top + layout.margins.bottom,
            )
        } else {
            QSizeF::new(
                perp + layout.margins.left + layout.margins.right,
                main + layout.margins.top + layout.margins.bottom,
            )
        }
    }

    pub fn layout(&mut self, root: QWidgetId, rect: QRectF) {
        self.measure(root);
        self.layout_widget(root, rect);
        self.apply_explicit_positions(root);
        self.sync_views();
    }

    /// `QWidget::minimumSizeHint()` as used by a window manager: the explicit
    /// `minimumSize` raised to the root layout's total minimum. Qt's default
    /// layout constraint pushes this value into the form's minimum size, so a
    /// dialog cannot be squeezed until its bottom button row escapes the
    /// content rect.
    pub fn minimum_size_hint(&mut self, widget: QWidgetId) -> QSizeF {
        self.measure(widget);
        let Some(node) = self.widgets.get(widget) else {
            return QSizeF::default();
        };
        let layout_min = node
            .layout
            .as_ref()
            .map(|layout| self.layout_minimum_size(layout))
            .unwrap_or_default();
        let splitter = node.splitter.is_some();
        let minimum = node.minimum_size;
        let splitter_min = if splitter {
            self.splitter_minimum(widget)
        } else {
            QSizeF::default()
        };
        QSizeF::new(
            minimum.width.max(layout_min.width).max(splitter_min.width),
            minimum
                .height
                .max(layout_min.height)
                .max(splitter_min.height),
        )
    }

    fn apply_explicit_positions(&mut self, widget: QWidgetId) {
        let Some(parent) = self.widgets.get(widget) else {
            return;
        };
        let parent_origin = (parent.geometry.x, parent.geometry.y);
        let children = parent.children.clone();
        // QSplitter / QTabWidget own their children geometry; Designer may
        // still store a `<property name="geometry">` on them, which must not
        // override the owner layout.
        let owned_parent = matches!(
            parent.class_name.as_str(),
            "QSplitter" | "QTabWidget"
        );
        for child in children {
            if !owned_parent {
                if let Some(relative) = self.widgets[child].explicit_position {
                    self.widgets[child].geometry.x = parent_origin.0 + relative.x;
                    self.widgets[child].geometry.y = parent_origin.1 + relative.y;
                }
            }
            self.apply_explicit_positions(child);
        }
    }

    fn layout_widget(&mut self, widget: QWidgetId, rect: QRectF) {
        if let Some(node) = self.widgets.get_mut(widget) {
            node.geometry = rect;
        }
        let splitter = self
            .widgets
            .get(widget)
            .is_some_and(|node| node.splitter.is_some());
        if splitter {
            self.layout_splitter(widget, rect);
            return;
        }
        let tabs = self
            .widgets
            .get(widget)
            .is_some_and(|node| node.tabs.is_some());
        if tabs {
            self.layout_tab_widget(widget, rect);
            return;
        }
        let Some(node) = self.widgets.get_mut(widget) else {
            return;
        };
        let layout = node.layout.take();
        if let Some(layout) = &layout {
            self.layout_engine(layout, rect);
        }
        self.widgets[widget].layout = layout;
    }

    /// Dispatch a layout to its geometry engine.
    fn layout_engine(&mut self, layout: &QLayout, rect: QRectF) {
        match layout.kind {
            QLayoutKind::Grid => self.layout_grid(layout, rect),
            QLayoutKind::Form => self.layout_form(layout, rect),
            QLayoutKind::HBox | QLayoutKind::VBox => self.layout_box(layout, rect),
        }
    }

    fn layout_box(&mut self, layout: &QLayout, rect: QRectF) {
        let inner = QRectF::new(
            rect.x + layout.margins.left,
            rect.y + layout.margins.top,
            (rect.width - layout.margins.left - layout.margins.right).max(0.0),
            (rect.height - layout.margins.top - layout.margins.bottom).max(0.0),
        );
        let horizontal = layout.is_horizontal();
        let count = layout.items.len();
        if count == 0 {
            return;
        }

        let mut sizes: Vec<f32> = layout
            .items
            .iter()
            .map(|item| {
                let size = self.item_hint(item);
                if horizontal {
                    size.width
                } else if let QLayoutItem::Widget { id, .. } = item {
                    // Qt sizes a word-wrapped QLabel through heightForWidth:
                    // in a vertical box its height follows from the row
                    // width (0.18/E3).
                    let widget = &self.widgets[*id];
                    if widget.class_name == "QLabel" && widget.word_wrap() {
                        let metrics = QFontMetrics::new(&widget.font);
                        return word_wrapped_height(&widget.text(), inner.width, &metrics);
                    }
                    size.height
                } else {
                    size.height
                }
            })
            .collect();
        let available = if horizontal {
            inner.width
        } else {
            inner.height
        };
        let mut gaps = layout.spacing * (count as f32 - 1.0);
        let slack = available - sizes.iter().sum::<f32>() - gaps;
        let mut cursor_extra = 0.0;
        let mut gap_extra_value = 0.0;

        if slack < 0.0 {
            let expanders: Vec<usize> = layout
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| self.item_expands(item, horizontal))
                .map(|(index, _)| index)
                .collect();
            let spacer_expanders: Vec<usize> = expanders
                .iter()
                .copied()
                .filter(|index| matches!(layout.items[*index], QLayoutItem::Spacer { .. }))
                .collect();
            let has_widget_expanders = expanders
                .iter()
                .any(|index| !matches!(layout.items[*index], QLayoutItem::Spacer { .. }));
            let mut candidates = if expanders.is_empty() {
                (0..count).collect::<Vec<_>>()
            } else {
                expanders
            };
            if has_widget_expanders {
                for index in &spacer_expanders {
                    sizes[*index] = 0.0;
                }
                gaps -= layout.spacing * spacer_expanders.len() as f32;
                candidates.retain(|index| !spacer_expanders.contains(index));
            }
            let deficit = (sizes.iter().sum::<f32>() + gaps - available).max(0.0);
            let total: f32 = candidates.iter().map(|index| sizes[*index]).sum();
            if total > 0.0 {
                for index in candidates {
                    let minimum = self.item_min_size(&layout.items[index], horizontal);
                    sizes[index] = (sizes[index] - sizes[index] / total * deficit).max(minimum);
                }
            }
        } else if slack > 0.0 {
            let expanders: Vec<usize> = layout
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| self.item_expands(item, horizontal))
                .map(|(index, _)| index)
                .collect();
            if expanders.is_empty() {
                let growers: Vec<usize> = layout
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| self.item_can_grow(item, horizontal))
                    .map(|(index, _)| index)
                    .collect();
                if !growers.is_empty() {
                    // `qGeomCalc()`'s growth branch: items that can grow get
                    // an equal share of the free space (not `hint + share`),
                    // giving up on items that hit their hint or maximum.
                    let fixed: f32 = (0..count)
                        .filter(|index| !growers.contains(index))
                        .map(|index| sizes[index])
                        .sum();
                    let mut remaining = available - fixed - gaps;
                    let mut active = growers;
                    loop {
                        if active.is_empty() {
                            break;
                        }
                        let share = remaining / active.len() as f32;
                        let mut fixed_any = false;
                        let mut next_active = Vec::new();
                        for index in &active {
                            let maximum = self.item_max_size(&layout.items[*index], horizontal);
                            if sizes[*index] > share + 0.01 {
                                // Keep the item at its hint: it is wider than
                                // the equal share and would only shrink.
                                remaining -= sizes[*index];
                                fixed_any = true;
                            } else if share > maximum + 0.01 {
                                // Clamp at the maximum and redistribute the
                                // surplus among the other growers.
                                sizes[*index] = maximum;
                                remaining -= maximum;
                                fixed_any = true;
                            } else {
                                next_active.push(*index);
                            }
                        }
                        if !fixed_any {
                            let share = remaining / active.len() as f32;
                            for index in &active {
                                sizes[*index] = share;
                            }
                            break;
                        }
                        active = next_active;
                    }
                    // If every grower stopped at its maximum, the remaining
                    // space goes back into the gaps, as Qt does for spacers.
                    let leftover = available - sizes.iter().sum::<f32>() - gaps;
                    if leftover > 0.01 {
                        let gap_extra = leftover / (count as f32 + 1.0);
                        cursor_extra = gap_extra;
                        gap_extra_value = gap_extra;
                    }
                } else {
                    let gap_extra = slack / (count as f32 + 1.0);
                    cursor_extra = gap_extra;
                    gap_extra_value = gap_extra;
                }
            } else {
                let fixed: f32 = (0..count)
                    .filter(|index| !expanders.contains(index))
                    .map(|index| sizes[index])
                    .sum();
                let mut remaining = available - fixed - gaps;
                let mut active = expanders;
                loop {
                    if active.is_empty() {
                        break;
                    }
                    let share = remaining / active.len() as f32;
                    let mut fixed_any = false;
                    let mut next_active = Vec::new();
                    for index in &active {
                        if sizes[*index] > share {
                            remaining -= sizes[*index];
                            fixed_any = true;
                        } else {
                            next_active.push(*index);
                        }
                    }
                    if !fixed_any {
                        for index in &active {
                            sizes[*index] = share;
                        }
                        break;
                    }
                    active = next_active;
                }
            }
        }

        let mut cursor = if horizontal { inner.x } else { inner.y } + cursor_extra;
        for (index, item) in layout.items.iter().enumerate() {
            let main = sizes[index];
            let perp = if horizontal {
                inner.height
            } else {
                inner.width
            };
            let slot = if horizontal {
                QRectF::new(cursor, inner.y, main, perp)
            } else {
                QRectF::new(inner.x, cursor, perp, main)
            };
            match item {
                QLayoutItem::Widget { id, cell } => {
                    // `QWidgetItem::setGeometry`: clamp to the maximum size
                    // and apply the item alignment (vertical centring by
                    // default), like the grid engine does.
                    let rect = self.grid_widget_rect(*id, slot, cell.alignment);
                    self.layout_widget(*id, rect);
                }
                QLayoutItem::Layout {
                    layout: inner_layout,
                    ..
                } => self.layout_engine(inner_layout, slot),
                QLayoutItem::Spacer { .. } => {}
            }
            cursor += main + if main > 0.0 { layout.spacing } else { 0.0 } + gap_extra_value;
        }
    }

    // ---- QFormLayout geometry (0.13.2) ----------------------------------
    //
    // Port of Qt's `QFormLayoutPrivate::updateSizes`, `setupVerticalLayoutData`,
    // `setupHorizontalLayoutData` and `arrangeWidgets` for the non-height-for-
    // width case used by the platform's forms. Row/column sizes go through the
    // same `qGeomCalc` implementation as the grid engine.

    fn layout_form(&mut self, layout: &QLayout, rect: QRectF) {
        let inner = QRectF::new(
            rect.x + layout.margins.left,
            rect.y + layout.margins.top,
            (rect.width - layout.margins.left - layout.margins.right).max(0.0),
            (rect.height - layout.margins.top - layout.margins.bottom).max(0.0),
        );
        let width = inner.width.round() as i32;
        let mut data = self.form_data(layout, width);
        if data.tracks.is_empty() {
            return;
        }

        // `setupHorizontalLayoutData`.
        let wrap_all = layout.row_wrap_policy == form_xml::RowWrapPolicy::WrapAllRows;
        let mut field_max_width = 0;
        for row in 0..data.labels.len() {
            let label = data.labels[row];
            let field = data.fields[row];
            if let Some(index) = label {
                let item = &mut data.items[index];
                item.layout_width = if field.is_some() && item.side_by_side {
                    data.max_label_width
                } else {
                    item.hint.w
                };
                item.layout_pos = 0;
            }
            if let Some(index) = field {
                let full_row = data.items[index].full_row;
                let sbs_hspace = data.items[index].sbs_hspace;
                let hint_w = data.items[index].hint.w;
                let mut layout_width = width;
                let mut layout_pos = 0;
                if !data.items[index].side_by_side {
                    let fldwidth = width - data.max_label_width - sbs_hspace;
                    if !(wrap_all || (label.is_none() && full_row) || hint_w > fldwidth) {
                        layout_width = fldwidth;
                        layout_pos = width - fldwidth;
                    }
                } else {
                    let fldwidth = width - data.max_label_width - sbs_hspace;
                    layout_width = fldwidth;
                    layout_pos = width - fldwidth;
                }
                let item = &mut data.items[index];
                item.layout_width = layout_width;
                item.layout_pos = layout_pos;
                field_max_width = field_max_width.max(item.max.w);
            }
        }
        data.form_max_width = data.max_label_width + field_max_width;

        let count = data.tracks.len();
        q_geom_calc(
            &mut data.tracks,
            0,
            count,
            inner.y.round() as i32,
            inner.height.round() as i32,
        );

        let form_alignment = if layout.form_alignment == 0 {
            form_xml::ALIGN_LEFT | form_xml::ALIGN_TOP
        } else {
            layout.form_alignment
        };
        let label_alignment = if layout.label_alignment == 0 {
            form_xml::ALIGN_LEFT
        } else {
            layout.label_alignment
        };
        let delta = width - data.form_max_width;
        let mut left_offset = 0;
        if form_alignment & (form_xml::ALIGN_HCENTER | form_xml::ALIGN_RIGHT) != 0 && delta > 0 {
            left_offset = if form_alignment & form_xml::ALIGN_HCENTER != 0 {
                delta >> 1
            } else {
                delta
            };
        }

        let mut targets: Vec<(usize, QRectF)> = Vec::new();
        for row in 0..data.labels.len() {
            if let Some(index) = data.labels[row] {
                let item = &data.items[index];
                let track = &data.tracks[item.v_layout_index];
                let mut height = track.size;
                if !item.expansive_v {
                    height = height.min(item.hint.h * 7 / 4).min(item.max.h);
                }
                let w = item.layout_width.min(item.hint.w);
                let mut x = left_offset + inner.x.round() as i32 + item.layout_pos;
                if label_alignment & form_xml::ALIGN_RIGHT != 0 {
                    x += item.layout_width - w;
                } else if label_alignment & form_xml::ALIGN_HCENTER != 0 {
                    x += item.layout_width / 2 - w / 2;
                }
                targets.push((
                    item.item,
                    QRectF::new(x as f32, track.pos as f32, w as f32, height as f32),
                ));
            }
            if let Some(index) = data.fields[row] {
                let item = &data.items[index];
                let track = &data.tracks[item.v_layout_index];
                let w = item.layout_width.min(item.max.w);
                let h = track.size.min(item.max.h);
                let x = item.layout_pos + left_offset + inner.x.round() as i32;
                targets.push((
                    item.item,
                    QRectF::new(x as f32, track.pos as f32, w as f32, h as f32),
                ));
            }
        }

        for (index, target) in targets {
            match &layout.items[index] {
                QLayoutItem::Widget { id, .. } => self.layout_widget(*id, target),
                QLayoutItem::Layout {
                    layout: inner_layout,
                    ..
                } => self.layout_engine(inner_layout, target),
                QLayoutItem::Spacer { .. } => {}
            }
        }
    }

    /// Item metrics for the form engine. Qt builds a `QWidgetItem` hint from
    /// integer `QFontMetrics`; our `QLineEdit` hint uses the raw advance, so
    /// correct the 4 px difference here without touching box-layout geometry.
    fn form_item_metrics(&self, item: &QLayoutItem) -> QGridItemMetrics {
        let mut metrics = self.grid_item_metrics(item);
        if let QLayoutItem::Widget { id, .. } = item {
            let widget = &self.widgets[*id];
            if widget.class_name == "QLineEdit" {
                let text_metrics = QFontMetrics::new(&widget.font);
                let hint = 17.0 * text_metrics.horizontal_advance("x").round() + 6.0;
                let hint = (hint.round() as i32).max(widget.minimum_size.width.round() as i32);
                if metrics.max_w < QLAYOUTSIZE_MAX {
                    metrics.max_w = metrics.max_w.min(hint);
                }
                metrics.hint_w = hint;
            }
        }
        metrics
    }

    /// Shared size/row bookkeeping for a `QFormLayout`, equivalent to Qt's
    /// `updateSizes` + `setupVerticalLayoutData(width)`.
    fn form_data(&self, layout: &QLayout, width: i32) -> QFormData {
        let row_count = layout
            .items
            .iter()
            .map(|item| item.cell().row.max(0) as usize + 1)
            .max()
            .unwrap_or(0);
        let mut labels: Vec<Option<usize>> = vec![None; row_count];
        let mut fields: Vec<Option<usize>> = vec![None; row_count];
        let mut full_rows: Vec<bool> = vec![false; layout.items.len()];
        for (index, item) in layout.items.iter().enumerate() {
            let cell = item.cell();
            let row = cell.row.max(0) as usize;
            if row >= row_count {
                continue;
            }
            if cell.column_span >= 2 {
                full_rows[index] = true;
                fields[row] = Some(index);
            } else if cell.column == 0 {
                labels[row] = Some(index);
            } else {
                fields[row] = Some(index);
            }
        }

        let wrap_all = layout.row_wrap_policy == form_xml::RowWrapPolicy::WrapAllRows;
        let user_v_spacing = layout.form_vertical_spacing().round() as i32;
        let user_h_spacing = if wrap_all {
            0
        } else {
            layout.form_horizontal_spacing().round() as i32
        };

        let mut items: Vec<QFormItem> = layout
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let metrics = self.form_item_metrics(item);
                let full_row = full_rows[index];
                let effective_full_row = full_row
                    && !labels
                        .get(item.cell().row.max(0) as usize)
                        .is_some_and(Option::is_some);
                let mut max_w = metrics.max_w;
                if !effective_full_row
                    && (layout.field_growth_policy
                        == form_xml::FieldGrowthPolicy::FieldsStayAtSizeHint
                        || (layout.field_growth_policy
                            == form_xml::FieldGrowthPolicy::ExpandingFieldsGrow
                            && !metrics.expansive_h))
                {
                    max_w = metrics.hint_w;
                }
                QFormItem {
                    item: index,
                    full_row,
                    expansive_v: metrics.expansive_v,
                    side_by_side: false,
                    min: QFormSize {
                        w: metrics.min_w,
                        h: metrics.min_h,
                    },
                    hint: QFormSize {
                        w: metrics.hint_w,
                        h: metrics.hint_h,
                    },
                    max: QFormSize {
                        w: max_w,
                        h: metrics.max_h,
                    },
                    sbs_hspace: if effective_full_row {
                        0
                    } else {
                        user_h_spacing
                    },
                    v_layout_index: 0,
                    layout_pos: 0,
                    layout_width: 0,
                }
            })
            .collect();

        // `updateSizes` width accounting.
        let (mut max_min_lbl, mut max_min_fld, mut max_min_ifld) = (0, 0, 0);
        let (mut max_sh_lbl, mut max_sh_fld, mut max_sh_ifld) = (0, 0, 0);
        for row in 0..row_count {
            let label = labels[row];
            let field = fields[row];
            if label.is_none() && field.is_none() {
                continue;
            }
            if let Some(index) = label {
                let item = &items[index];
                max_min_lbl = max_min_lbl.max(item.min.w);
                max_sh_lbl = max_sh_lbl.max(item.hint.w);
            }
            if let Some(index) = field {
                let item = &items[index];
                if item.full_row {
                    max_min_ifld = max_min_ifld.max(item.min.w);
                    max_sh_ifld = max_sh_ifld.max(item.hint.w);
                } else {
                    max_min_fld = max_min_fld.max(item.min.w + item.sbs_hspace);
                    max_sh_fld = max_sh_fld.max(item.hint.w + item.sbs_hspace);
                }
            }
        }
        let (sh_width, min_width, _thresh_width) = match layout.row_wrap_policy {
            form_xml::RowWrapPolicy::WrapAllRows => (
                max_sh_lbl.max(max_sh_ifld.max(max_sh_fld)),
                max_min_lbl.max(max_min_ifld.max(max_min_fld)),
                0,
            ),
            form_xml::RowWrapPolicy::WrapLongRows => (
                (max_sh_lbl + max_sh_fld).max(max_sh_ifld),
                max_min_lbl.max(max_min_ifld.max(max_min_fld)),
                max_sh_lbl + max_min_fld,
            ),
            form_xml::RowWrapPolicy::DontWrapRows => (
                (max_sh_lbl + max_sh_fld).max(max_sh_ifld),
                (max_min_lbl + max_min_fld).max(max_min_ifld),
                QLAYOUTSIZE_MAX,
            ),
        };

        // `setupVerticalLayoutData`.
        let mut tracks: Vec<QGridTrack> = Vec::with_capacity(2 * row_count + 2);
        tracks.push(form_track_init(0, 0));
        let mut max_label_width = 0;
        if !wrap_all {
            for row in 0..row_count {
                let Some(index) = labels[row] else {
                    continue;
                };
                let field_min = fields[row].map_or(0, |field| items[field].min.w);
                if items[index].hint.w + field_min <= width {
                    max_label_width = max_label_width.max(items[index].hint.w);
                }
            }
        } else {
            max_label_width = width;
        }

        let mut vidx = 1usize;
        let mut add_top_bottom_stretch = true;
        for row in 0..row_count {
            let label = labels[row];
            let field = fields[row];
            if label.is_none() && field.is_none() {
                continue;
            }
            let split_side_by_side = layout.row_wrap_policy
                == form_xml::RowWrapPolicy::WrapLongRows
                && match label {
                    Some(index) => {
                        max_label_width < items[index].hint.w
                            || width < max_label_width + field.map_or(0, |field| items[field].min.w)
                    }
                    None => width < max_label_width + field.map_or(0, |field| items[field].min.w),
                };

            if wrap_all || split_side_by_side {
                if let Some(index) = label {
                    push_form_track(
                        &mut tracks,
                        &mut items[index],
                        &mut vidx,
                        user_v_spacing,
                        false,
                    );
                    if items[index].expansive_v {
                        add_top_bottom_stretch = false;
                    }
                }
                if let Some(index) = field {
                    push_form_track(
                        &mut tracks,
                        &mut items[index],
                        &mut vidx,
                        user_v_spacing,
                        false,
                    );
                    if items[index].expansive_v {
                        add_top_bottom_stretch = false;
                    }
                }
            } else {
                let label_data = label.map(|index| items[index]);
                let field_data = field.map(|index| items[index]);
                let mut track = form_track_init(
                    0,
                    label_data
                        .map_or(0, |item| item.min.h)
                        .max(field_data.map_or(0, |item| item.min.h)),
                );
                track.hint = label_data
                    .map_or(0, |item| item.hint.h)
                    .max(field_data.map_or(0, |item| item.hint.h));
                track.maximum = label_data
                    .map_or(QLAYOUTSIZE_MAX, |item| item.max.h)
                    .min(field_data.map_or(QLAYOUTSIZE_MAX, |item| item.max.h));
                track.expansive = label_data.is_some_and(|item| item.expansive_v)
                    || field_data.is_some_and(|item| item.expansive_v);
                track.empty = false;
                if track.expansive {
                    add_top_bottom_stretch = false;
                }
                tracks.push(track);
                if vidx > 1 {
                    tracks[vidx - 1].spacing = user_v_spacing;
                }
                if let Some(index) = label {
                    items[index].side_by_side = field.is_some();
                    items[index].v_layout_index = vidx;
                }
                if let Some(index) = field {
                    items[index].side_by_side = label.is_some() || !items[index].full_row;
                    items[index].v_layout_index = vidx;
                }
                vidx += 1;
            }
        }

        let form_alignment = if layout.form_alignment == 0 {
            form_xml::ALIGN_LEFT | form_xml::ALIGN_TOP
        } else {
            layout.form_alignment
        };
        if add_top_bottom_stretch {
            if form_alignment & form_xml::ALIGN_BOTTOM == 0 {
                let mut bottom = form_track_init(0, 0);
                bottom.expansive = true;
                tracks.push(bottom);
            }
            if form_alignment & (form_xml::ALIGN_VCENTER | form_xml::ALIGN_BOTTOM) != 0 {
                let mut top = form_track_init(0, 0);
                top.expansive = true;
                tracks[0] = top;
            }
        }

        QFormData {
            items,
            labels,
            fields,
            tracks,
            max_label_width,
            sh_width,
            min_width,
            form_max_width: 0,
        }
    }

    fn form_size_hint(&self, layout: &QLayout) -> QSizeF {
        let data = self.form_data(layout, QLAYOUTSIZE_MAX);
        let height: i32 = data
            .tracks
            .iter()
            .map(|track| track.hint + track.spacing)
            .sum();
        QSizeF::new(
            (data.sh_width + layout.margins.left as i32 + layout.margins.right as i32) as f32,
            (height + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
        )
    }

    fn form_minimum_size(&self, layout: &QLayout) -> QSizeF {
        let data = self.form_data(layout, QLAYOUTSIZE_MAX);
        let height: i32 = data
            .tracks
            .iter()
            .map(|track| track.minimum + track.spacing)
            .sum();
        QSizeF::new(
            (data.min_width + layout.margins.left as i32 + layout.margins.right as i32) as f32,
            (height + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
        )
    }

    // ---- grid geometry (0.12.3/0.12.4) ----------------------------------
    //
    // Port of Qt's `QGridLayoutPrivate::setupLayoutData` / `distribute` and
    // of `qGeomCalc` (qlayoutengine.cpp). Qt computes row/column sizes in
    // integers and uses fixed point for the proportional passes; the same
    // arithmetic is reproduced here so geometry matches Qt6 pixel for pixel.

    fn layout_grid(&mut self, layout: &QLayout, rect: QRectF) {
        let inner = QRectF::new(
            rect.x + layout.margins.left,
            rect.y + layout.margins.top,
            (rect.width - layout.margins.left - layout.margins.right).max(0.0),
            (rect.height - layout.margins.top - layout.margins.bottom).max(0.0),
        );
        let (mut rows, mut columns) = self.grid_tracks(layout);
        if rows.is_empty() || columns.is_empty() {
            return;
        }
        let row_count = rows.len();
        let column_count = columns.len();
        q_geom_calc(
            &mut rows,
            0,
            row_count,
            inner.y.round() as i32,
            inner.height.round() as i32,
        );
        q_geom_calc(
            &mut columns,
            0,
            column_count,
            inner.x.round() as i32,
            inner.width.round() as i32,
        );

        for item in &layout.items {
            let cell = item.cell();
            let row = (cell.row as usize).min(rows.len() - 1);
            let column = (cell.column as usize).min(columns.len() - 1);
            let last_row = ((cell.row + cell.row_span - 1) as usize).min(rows.len() - 1);
            let last_column =
                ((cell.column + cell.column_span - 1) as usize).min(columns.len() - 1);
            let x = columns[column].pos;
            let y = rows[row].pos;
            let w = columns[last_column].pos + columns[last_column].size - x;
            let h = rows[last_row].pos + rows[last_row].size - y;
            let cell_rect = QRectF::new(x as f32, y as f32, w as f32, h as f32);
            match item {
                QLayoutItem::Widget { id, .. } => {
                    // `QWidgetItem::setGeometry`: clamp to the widget's
                    // maximum size and place it inside the cell.
                    let rect = self.grid_widget_rect(*id, cell_rect, cell.alignment);
                    self.layout_widget(*id, rect);
                }
                QLayoutItem::Layout {
                    layout: inner_layout,
                    ..
                } => self.layout_engine(inner_layout, cell_rect),
                QLayoutItem::Spacer { .. } => {}
            }
        }
    }

    /// Widget geometry inside a grid cell, mirroring Qt's
    /// `QWidgetItem::setGeometry` (maximum-size clamp + item alignment).
    fn grid_widget_rect(&self, id: QWidgetId, cell: QRectF, alignment: u32) -> QRectF {
        let metrics = self.widget_metrics(id);
        let hint = self.widget_hint(id);
        let mut width = cell.width.min(metrics.max_w as f32);
        let mut height = cell.height.min(metrics.max_h as f32);
        let horizontal = alignment & form_xml::ALIGN_HORIZONTAL_MASK;
        let vertical = alignment & form_xml::ALIGN_VERTICAL_MASK;
        if horizontal != 0 {
            width = width.min(hint.width);
        }
        if vertical != 0 {
            height = height.min(hint.height);
        }
        // Qt's `visualAlignment()` implies AlignLeft when no horizontal bit is
        // set; vertically the default is "not AlignTop" → centered.
        let horizontal = if horizontal == 0 {
            form_xml::ALIGN_LEFT
        } else {
            horizontal
        };
        let mut x = cell.x;
        let mut y = cell.y;
        if horizontal & form_xml::ALIGN_RIGHT != 0 {
            x += cell.width - width;
        } else if horizontal & form_xml::ALIGN_LEFT == 0 {
            x += ((cell.width - width) / 2.0).floor();
        }
        if vertical & form_xml::ALIGN_BOTTOM != 0 {
            y += cell.height - height;
        } else if vertical & form_xml::ALIGN_TOP == 0 {
            y += ((cell.height - height) / 2.0).floor();
        }
        QRectF::new(x, y, width, height)
    }

    fn grid_layout_size_hint(&self, layout: &QLayout) -> QSizeF {
        let (rows, columns) = self.grid_tracks(layout);
        let height: i32 = rows.iter().map(|track| track.hint + track.spacing).sum();
        let width: i32 = columns.iter().map(|track| track.hint + track.spacing).sum();
        QSizeF::new(
            (width + layout.margins.left as i32 + layout.margins.right as i32) as f32,
            (height + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
        )
    }

    /// Rows and columns as Qt's `QLayoutStruct` chains, including the
    /// contribution of spanning items (`distributeMultiBox`) and the
    /// per-track spacing used between neighbouring items.
    fn grid_tracks(&self, layout: &QLayout) -> (Vec<QGridTrack>, Vec<QGridTrack>) {
        let mut row_count = 0usize;
        let mut column_count = 0usize;
        for item in &layout.items {
            let cell = item.cell();
            row_count = row_count.max((cell.row + cell.row_span) as usize);
            column_count = column_count.max((cell.column + cell.column_span) as usize);
        }
        if row_count == 0 || column_count == 0 {
            return (Vec::new(), Vec::new());
        }
        let spacing = layout.spacing.round() as i32;
        let mut rows = vec![QGridTrack::new(spacing); row_count];
        let mut columns = vec![QGridTrack::new(spacing); column_count];
        for track in rows.iter_mut().chain(columns.iter_mut()) {
            track.spacing = 0;
        }

        let metrics: Vec<QGridItemMetrics> = layout
            .items
            .iter()
            .map(|item| self.grid_item_metrics(item))
            .collect();

        let mut has_span = false;
        for (item, metric) in layout.items.iter().zip(&metrics) {
            let cell = item.cell();
            let row = cell.row as usize;
            let column = cell.column as usize;
            let last_row = (cell.row + cell.row_span - 1) as usize;
            let last_column = (cell.column + cell.column_span - 1) as usize;
            if last_row == row {
                add_grid_track(
                    &mut rows[row],
                    metric.min_h,
                    metric.hint_h,
                    metric.max_h,
                    metric.expansive_v,
                );
            } else {
                init_empty_multi_box(&mut rows, row, last_row);
                has_span = true;
            }
            if last_column == column {
                add_grid_track(
                    &mut columns[column],
                    metric.min_w,
                    metric.hint_w,
                    metric.max_w,
                    metric.expansive_h,
                );
            } else {
                init_empty_multi_box(&mut columns, column, last_column);
                has_span = true;
            }
        }

        setup_grid_spacings(layout, &mut rows, &mut columns, spacing);

        if has_span {
            for (item, metric) in layout.items.iter().zip(&metrics) {
                let cell = item.cell();
                if cell.row_span > 1 {
                    distribute_multi_box(
                        &mut rows,
                        cell.row as usize,
                        (cell.row + cell.row_span - 1) as usize,
                        metric.min_h,
                        metric.hint_h,
                    );
                }
                if cell.column_span > 1 {
                    distribute_multi_box(
                        &mut columns,
                        cell.column as usize,
                        (cell.column + cell.column_span - 1) as usize,
                        metric.min_w,
                        metric.hint_w,
                    );
                }
            }
        }

        (rows, columns)
    }

    /// Minimum/hint/maximum size of one grid item, following Qt's
    /// `QWidgetItem`/`QSpacerItem`/`QLayoutItem` contracts.
    fn grid_item_metrics(&self, item: &QLayoutItem) -> QGridItemMetrics {
        match item {
            QLayoutItem::Widget { id, .. } => self.widget_metrics(*id),
            QLayoutItem::Spacer { item: spacer, .. } => {
                let hint_w = spacer.width.round() as i32;
                let hint_h = spacer.height.round() as i32;
                let expansive_h = spacer.horizontal_policy == QSizePolicy::Expanding;
                let expansive_v = spacer.vertical_policy == QSizePolicy::Expanding;
                QGridItemMetrics {
                    min_w: 0,
                    min_h: 0,
                    hint_w,
                    hint_h,
                    max_w: if expansive_h { QLAYOUTSIZE_MAX } else { hint_w },
                    max_h: if expansive_v { QLAYOUTSIZE_MAX } else { hint_h },
                    expansive_h,
                    expansive_v,
                }
            }
            QLayoutItem::Layout { layout, .. } => {
                let hint = self.layout_size_hint(layout);
                let min = self.layout_minimum_size(layout);
                let max = self.layout_maximum_size(layout);
                QGridItemMetrics {
                    min_w: min.width.round() as i32,
                    min_h: min.height.round() as i32,
                    hint_w: hint.width.round() as i32,
                    hint_h: hint.height.round() as i32,
                    max_w: max.width.round() as i32,
                    max_h: max.height.round() as i32,
                    expansive_h: layout
                        .items
                        .iter()
                        .any(|item| self.grid_item_metrics(item).expansive_h),
                    expansive_v: layout
                        .items
                        .iter()
                        .any(|item| self.grid_item_metrics(item).expansive_v),
                }
            }
        }
    }

    fn layout_minimum_size(&self, layout: &QLayout) -> QSizeF {
        if layout.is_grid() {
            let (rows, columns) = self.grid_tracks(layout);
            let height: i32 = rows.iter().map(|track| track.minimum + track.spacing).sum();
            let width: i32 = columns
                .iter()
                .map(|track| track.minimum + track.spacing)
                .sum();
            return QSizeF::new(
                (width + layout.margins.left as i32 + layout.margins.right as i32) as f32,
                (height + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
            );
        }
        if layout.is_form() {
            return self.form_minimum_size(layout);
        }
        let mut main = 0.0f32;
        let mut perp = 0.0f32;
        for item in &layout.items {
            let metric = self.grid_item_metrics(item);
            let (item_min_w, item_min_h) = (metric.min_w as f32, metric.min_h as f32);
            let (item_main, item_perp) = if layout.is_horizontal() {
                (item_min_w, item_min_h)
            } else {
                (item_min_h, item_min_w)
            };
            main += item_main;
            perp = perp.max(item_perp);
        }
        if layout.items.len() > 1 {
            main += layout.spacing * (layout.items.len() as f32 - 1.0);
        }
        if layout.is_horizontal() {
            QSizeF::new(
                main + layout.margins.left + layout.margins.right,
                perp + layout.margins.top + layout.margins.bottom,
            )
        } else {
            QSizeF::new(
                perp + layout.margins.left + layout.margins.right,
                main + layout.margins.top + layout.margins.bottom,
            )
        }
    }

    /// `QLayout::maximumSize()` per axis, needed when a sub-layout sits in a
    /// grid cell: a fixed-height button row caps the cell height at its hint.
    fn layout_maximum_size(&self, layout: &QLayout) -> QSizeF {
        // `QFormLayout` does not override `QLayout::maximumSize()`.
        if layout.is_form() {
            return QSizeF::new(QLAYOUTSIZE_MAX as f32, QLAYOUTSIZE_MAX as f32);
        }
        let clamp = |value: i64| value.min(QLAYOUTSIZE_MAX as i64) as i32;
        if layout.is_grid() {
            let (rows, columns) = self.grid_tracks(layout);
            let height = clamp(
                rows.iter()
                    .map(|track| track.maximum as i64 + track.spacing as i64)
                    .sum(),
            );
            let width = clamp(
                columns
                    .iter()
                    .map(|track| track.maximum as i64 + track.spacing as i64)
                    .sum(),
            );
            return QSizeF::new(
                (width + layout.margins.left as i32 + layout.margins.right as i32) as f32,
                (height + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
            );
        }
        // `QBoxLayoutPrivate::setupGeom`: main axis maxima are summed, the
        // perpendicular maximum folds item maxima through `qMaxExpCalc`
        // (a fixed-height item caps the whole box height).
        let horizontal = layout.is_horizontal();
        let mut main: i64 = 0;
        let mut perp = QLAYOUTSIZE_MAX;
        let mut perp_expansive = false;
        let mut previous = false;
        for item in &layout.items {
            let metric = self.grid_item_metrics(item);
            let (item_main, item_perp, expansive_perp) = if horizontal {
                (metric.max_w as i64, metric.max_h, metric.expansive_v)
            } else {
                (metric.max_h as i64, metric.max_w, metric.expansive_h)
            };
            if previous {
                main += layout.spacing.round() as i64;
            }
            main += item_main;
            let mut empty = true;
            q_max_exp_calc(
                &mut perp,
                &mut perp_expansive,
                &mut empty,
                item_perp,
                expansive_perp,
                false,
            );
            previous = true;
        }
        let main = clamp(main);
        let perp = clamp(perp as i64);
        if layout.is_horizontal() {
            QSizeF::new(
                (main + layout.margins.left as i32 + layout.margins.right as i32) as f32,
                (perp + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
            )
        } else {
            QSizeF::new(
                (perp + layout.margins.left as i32 + layout.margins.right as i32) as f32,
                (main + layout.margins.top as i32 + layout.margins.bottom as i32) as f32,
            )
        }
    }

    fn widget_metrics(&self, id: QWidgetId) -> QGridItemMetrics {
        let Some(node) = self.widgets.get(id) else {
            return QGridItemMetrics::default();
        };
        let hint = self.widget_hint(id);
        let hint_w = hint.width.round() as i32;
        let hint_h = hint.height.round() as i32;
        let (minimum_hint_w, minimum_hint_h) = self.widget_minimum_hint(id);
        let policy_h = self.widget_axis_policy(id, true);
        let policy_v = self.widget_axis_policy(id, false);
        QGridItemMetrics {
            min_w: grid_smart_min(
                policy_h,
                hint_w,
                minimum_hint_w,
                node.minimum_size.width.round() as i32,
                node.maximum_size.width.round() as i32,
            ),
            min_h: grid_smart_min(
                policy_v,
                hint_h,
                minimum_hint_h,
                node.minimum_size.height.round() as i32,
                node.maximum_size.height.round() as i32,
            ),
            hint_w,
            hint_h,
            max_w: grid_smart_max(
                policy_h,
                hint_w,
                node.minimum_size.width.round() as i32,
                node.maximum_size.width.round() as i32,
            ),
            max_h: grid_smart_max(
                policy_v,
                hint_h,
                node.minimum_size.height.round() as i32,
                node.maximum_size.height.round() as i32,
            ),
            expansive_h: policy_expands(policy_h),
            expansive_v: policy_expands(policy_v),
        }
    }

    /// Qt class defaults for `QWidget::sizePolicy`; an explicit
    /// `<property name="sizePolicy">` in the form always wins.
    fn widget_axis_policy(&self, id: QWidgetId, horizontal: bool) -> QSizePolicy {
        let Some(node) = self.widgets.get(id) else {
            return QSizePolicy::Preferred;
        };
        if node.has_size_policy {
            return if horizontal {
                node.size_policy
            } else {
                node.size_policy_v
            };
        }
        match node.class_name.as_str() {
            "QLabel" | "QGroupBox" | "QDialog" | "QWidget" | "QMdiSubWindow" => {
                QSizePolicy::Preferred
            }
            "QLineEdit" => {
                if horizontal {
                    QSizePolicy::Expanding
                } else {
                    QSizePolicy::Fixed
                }
            }
            "QComboBox" => {
                if horizontal {
                    QSizePolicy::Preferred
                } else {
                    QSizePolicy::Fixed
                }
            }
            "QTableWidget" | "QListWidget" | "QTreeWidget" => QSizePolicy::Expanding,
            "QPushButton" | "QCheckBox" | "QRadioButton" => {
                if horizontal {
                    QSizePolicy::Minimum
                } else {
                    QSizePolicy::Fixed
                }
            }
            "QToolButton" => QSizePolicy::Fixed,
            _ => QSizePolicy::Preferred,
        }
    }

    /// Approximation of `minimumSizeHint()` for the widget classes we draw;
    /// used by the shrinking branch of the Qt geometry engine.
    fn widget_minimum_hint(&self, id: QWidgetId) -> (i32, i32) {
        let hint = self.widget_hint(id);
        let fallback = (hint.width.round() as i32, hint.height.round() as i32);
        let Some(node) = self.widgets.get(id) else {
            return fallback;
        };
        if let Some(splitter) = node.splitter.as_deref() {
            let horizontal = splitter.is_horizontal();
            let handle = splitter.handle_width;
            let children = node.children.clone();
            let mut main = handle * children.len().saturating_sub(1) as f32;
            let mut perp = 0.0_f32;
            for child in children {
                let (width, height) = self.widget_minimum_hint(child);
                if horizontal {
                    main += width as f32;
                    perp = perp.max(height as f32);
                } else {
                    main += height as f32;
                    perp = perp.max(width as f32);
                }
            }
            return if horizontal {
                (main.round() as i32, perp.round() as i32)
            } else {
                (perp.round() as i32, main.round() as i32)
            };
        }
        match node.class_name.as_str() {
            "QLineEdit" => (32, fallback.1.max(24)),
            "QTableWidget" | "QListWidget" | "QTreeWidget" => (70, 70),
            // The title is part of the minimum: a long group caption makes
            // Qt grow the whole window (gallery/text/group_long_title).
            "QGroupBox" => (fallback.0.max(73), fallback.1.max(35)),
            _ => fallback,
        }
    }

    fn item_hint(&self, item: &QLayoutItem) -> QSizeF {
        match item {
            QLayoutItem::Widget { id, .. } => self.widget_hint(*id),
            QLayoutItem::Spacer { item: spacer, .. } => QSizeF::new(spacer.width, spacer.height),
            QLayoutItem::Layout { layout, .. } => self.layout_size_hint(layout),
        }
    }

    fn widget_hint(&self, widget: QWidgetId) -> QSizeF {
        let node = &self.widgets[widget];
        QSizeF::new(
            node.size_hint
                .width
                .max(node.minimum_size.width)
                .min(node.maximum_size.width),
            node.size_hint
                .height
                .max(node.minimum_size.height)
                .min(node.maximum_size.height),
        )
    }

    fn item_min_size(&self, item: &QLayoutItem, horizontal: bool) -> f32 {
        let QLayoutItem::Widget { id, .. } = item else {
            return 0.0;
        };
        let Some(widget) = self.widgets.get(*id) else {
            return 0.0;
        };
        if horizontal {
            widget.minimum_size.width
        } else {
            widget.minimum_size.height
        }
    }

    fn item_expands(&self, item: &QLayoutItem, horizontal: bool) -> bool {
        match item {
            QLayoutItem::Widget { id, .. } => self.widget_expands(*id, horizontal),
            QLayoutItem::Spacer { item: spacer, .. } => {
                if horizontal {
                    spacer.horizontal_policy == QSizePolicy::Expanding
                } else {
                    spacer.vertical_policy == QSizePolicy::Expanding
                }
            }
            // A nested layout is not an expander for `qGeomCalc`: Qt lets an
            // expanding spacer take all the slack while the nested layout
            // keeps its hint (gallery/lists/190: the grid rows grew and the
            // list became 83 px tall instead of Qt's ~60). When there is no
            // spacer the layout still grows through `item_can_grow` (its
            // maximum exceeds the hint).
            QLayoutItem::Layout { .. } => false,
        }
    }

    /// Qt's `qGeomCalc()` grows every item whose maximum size exceeds its
    /// hint, regardless of class: a row of `QPushButton`s (policy `Minimum`)
    /// or a `QComboBox` (`Preferred`) is stretched just like a label.
    fn item_can_grow(&self, item: &QLayoutItem, horizontal: bool) -> bool {
        let metrics = self.grid_item_metrics(item);
        let (maximum, hint) = if horizontal {
            (metrics.max_w, metrics.hint_w)
        } else {
            (metrics.max_h, metrics.hint_h)
        };
        maximum > hint
    }

    fn item_max_size(&self, item: &QLayoutItem, horizontal: bool) -> f32 {
        let metrics = self.grid_item_metrics(item);
        if horizontal {
            metrics.max_w as f32
        } else {
            metrics.max_h as f32
        }
    }

    fn widget_expands(&self, widget: QWidgetId, horizontal: bool) -> bool {
        let Some(widget) = self.widgets.get(widget) else {
            return false;
        };
        if horizontal {
            widget.size_policy == QSizePolicy::Expanding
                || widget.size_policy == QSizePolicy::MinimumExpanding
                || widget.class_name == "QLineEdit"
                || widget.class_name == "QTableWidget"
                || widget.class_name == "QListWidget"
                || widget.class_name == "QTreeWidget"
        } else {
            widget.size_policy_v == QSizePolicy::Expanding
                || widget.size_policy_v == QSizePolicy::MinimumExpanding
                || widget.class_name == "QTableWidget"
                || widget.class_name == "QListWidget"
                || widget.class_name == "QTreeWidget"
        }
    }
}

/// Largest track size, same value as Qt's `QLAYOUTSIZE_MAX`.
const QLAYOUTSIZE_MAX: i32 = 16_777_215;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct QFormSize {
    w: i32,
    h: i32,
}

/// One cell of a `QFormLayout`, mirroring Qt's `QFormLayoutItem`.
#[derive(Clone, Copy, Debug)]
struct QFormItem {
    /// Index into `QLayout::items`.
    item: usize,
    /// Spanning item (`colspan >= 2`).
    full_row: bool,
    expansive_v: bool,
    side_by_side: bool,
    min: QFormSize,
    hint: QFormSize,
    max: QFormSize,
    sbs_hspace: i32,
    v_layout_index: usize,
    layout_pos: i32,
    layout_width: i32,
}

/// Result of joining Qt's `updateSizes` and `setupVerticalLayoutData`.
struct QFormData {
    items: Vec<QFormItem>,
    /// Row → index into `items`.
    labels: Vec<Option<usize>>,
    fields: Vec<Option<usize>>,
    tracks: Vec<QGridTrack>,
    max_label_width: i32,
    sh_width: i32,
    min_width: i32,
    form_max_width: i32,
}

/// `QLayoutStruct::init(stretch, minSize)` in Qt: hint starts at `minimum`.
fn form_track_init(_stretch: i32, minimum: i32) -> QGridTrack {
    let mut track = QGridTrack::new(0);
    track.minimum = minimum;
    track.hint = minimum;
    track.maximum = QLAYOUTSIZE_MAX;
    track.empty = true;
    track
}

fn push_form_track(
    tracks: &mut Vec<QGridTrack>,
    item: &mut QFormItem,
    vidx: &mut usize,
    spacing: i32,
    side_by_side: bool,
) {
    let mut track = form_track_init(0, item.min.h);
    track.hint = item.hint.h;
    track.maximum = item.max.h;
    track.expansive = item.expansive_v;
    track.empty = false;
    tracks.push(track);
    if *vidx > 1 {
        tracks[*vidx - 1].spacing = spacing;
    }
    item.v_layout_index = *vidx;
    item.side_by_side = side_by_side;
    *vidx += 1;
}

/// One row or column of a grid layout: a port of Qt's `QLayoutStruct`.
#[derive(Clone, Copy, Debug)]
struct QGridTrack {
    minimum: i32,
    hint: i32,
    maximum: i32,
    spacing: i32,
    expansive: bool,
    empty: bool,
    done: bool,
    pos: i32,
    size: i32,
}

impl QGridTrack {
    fn new(spacing: i32) -> Self {
        Self {
            minimum: 0,
            hint: 0,
            maximum: 0,
            spacing,
            expansive: false,
            empty: true,
            done: false,
            pos: 0,
            size: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct QGridItemMetrics {
    min_w: i32,
    min_h: i32,
    hint_w: i32,
    hint_h: i32,
    max_w: i32,
    max_h: i32,
    expansive_h: bool,
    expansive_v: bool,
}

fn policy_grows(policy: QSizePolicy) -> bool {
    !matches!(policy, QSizePolicy::Fixed | QSizePolicy::Maximum)
}

fn policy_shrinks(policy: QSizePolicy) -> bool {
    matches!(
        policy,
        QSizePolicy::Maximum
            | QSizePolicy::Preferred
            | QSizePolicy::Expanding
            | QSizePolicy::Ignored
    )
}

fn policy_expands(policy: QSizePolicy) -> bool {
    matches!(
        policy,
        QSizePolicy::Expanding | QSizePolicy::MinimumExpanding
    )
}

/// `qSmartMinSize()` for one axis.
fn grid_smart_min(
    policy: QSizePolicy,
    hint: i32,
    minimum_hint: i32,
    explicit_minimum: i32,
    maximum: i32,
) -> i32 {
    if policy == QSizePolicy::Ignored {
        return 0;
    }
    let mut size = if policy_shrinks(policy) {
        minimum_hint
    } else {
        hint.max(minimum_hint)
    };
    if maximum > 0 && maximum < QLAYOUTSIZE_MAX {
        size = size.min(maximum);
    }
    if explicit_minimum > 0 {
        size = explicit_minimum;
    }
    size.max(0)
}

/// `qSmartMaxSize()` for one axis.
fn grid_smart_max(policy: QSizePolicy, hint: i32, explicit_minimum: i32, maximum: i32) -> i32 {
    let mut size = if maximum >= QLAYOUTSIZE_MAX {
        QLAYOUTSIZE_MAX
    } else {
        maximum
    };
    if size == QLAYOUTSIZE_MAX && !policy_grows(policy) {
        size = hint.max(explicit_minimum);
    }
    if size == 0 {
        size = hint.max(explicit_minimum);
    }
    size
}

/// `qMaxExpCalc()`: merge one box into the track's maximum size and flags.
fn add_grid_track(track: &mut QGridTrack, minimum: i32, hint: i32, maximum: i32, expansive: bool) {
    track.hint = track.hint.max(hint);
    track.minimum = track.minimum.max(minimum);
    q_max_exp_calc(
        &mut track.maximum,
        &mut track.expansive,
        &mut track.empty,
        maximum,
        expansive,
        false,
    );
}

/// `qMaxExpCalc()`: fold one box maximum into the running maximum/flags.
fn q_max_exp_calc(
    maximum: &mut i32,
    expansive: &mut bool,
    empty: &mut bool,
    box_maximum: i32,
    box_expansive: bool,
    box_empty: bool,
) {
    if *expansive {
        if box_expansive {
            *maximum = (*maximum).max(box_maximum);
        }
    } else if box_expansive || (*empty && (!box_empty || *maximum == 0)) {
        *maximum = box_maximum;
    } else if *empty == box_empty {
        *maximum = (*maximum).min(box_maximum);
    }
    *expansive |= box_expansive;
    *empty &= box_empty;
}

fn init_empty_multi_box(chain: &mut [QGridTrack], start: usize, end: usize) {
    for track in &mut chain[start..=end] {
        if track.empty && track.maximum == 0 {
            track.maximum = QLAYOUTSIZE_MAX;
        }
        track.empty = false;
    }
}

/// `distributeMultiBox()`: make the spanned rows/columns fit one item.
#[allow(clippy::needless_range_loop)]
fn distribute_multi_box(
    chain: &mut [QGridTrack],
    start: usize,
    end: usize,
    min_size: i32,
    hint: i32,
) {
    let mut minimum_sum = 0;
    let mut hint_sum = 0;
    let mut maximum_sum = 0;
    for index in start..=end {
        let track = &chain[index];
        minimum_sum += track.minimum;
        hint_sum += track.hint;
        maximum_sum += track.maximum;
        if index != end {
            minimum_sum += track.spacing;
            hint_sum += track.spacing;
            maximum_sum += track.spacing;
        }
    }

    if maximum_sum < min_size {
        // Not even the maximum sizes fit: qGeomCalc() puts the extra space
        // between the tracks, recover it and grow the minimum sizes.
        q_geom_calc(chain, start, end - start + 1, 0, min_size);
        let mut pos = 0;
        for index in start..=end {
            let next = if index == end {
                min_size
            } else {
                chain[index + 1].pos
            };
            let mut real_size = next - pos;
            if index != end {
                real_size -= chain[index].spacing;
            }
            let track = &mut chain[index];
            if track.minimum < real_size {
                track.minimum = real_size;
            }
            if track.maximum < track.minimum {
                track.maximum = track.minimum;
            }
            pos = next;
        }
    } else if minimum_sum < min_size {
        q_geom_calc(chain, start, end - start + 1, 0, min_size);
        for index in start..=end {
            let track = &mut chain[index];
            if track.minimum < track.size {
                track.minimum = track.size;
            }
        }
    }

    if hint_sum < hint {
        q_geom_calc(chain, start, end - start + 1, 0, hint);
        for index in start..=end {
            let track = &mut chain[index];
            if track.hint < track.size {
                track.hint = track.size;
            }
        }
    }
}

/// Per-track spacing: Qt only inserts a gap between two *different* items.
fn setup_grid_spacings(
    layout: &QLayout,
    rows: &mut [QGridTrack],
    columns: &mut [QGridTrack],
    spacing: i32,
) {
    if spacing <= 0 || rows.is_empty() || columns.is_empty() {
        return;
    }
    let row_count = rows.len();
    let column_count = columns.len();
    let mut grid: Vec<Option<usize>> = vec![None; row_count * column_count];
    for (index, item) in layout.items.iter().enumerate() {
        let cell = item.cell();
        for row in cell.row..cell.row + cell.row_span {
            for column in cell.column..cell.column + cell.column_span {
                let (row, column) = (row as usize, column as usize);
                if row < row_count && column < column_count {
                    grid[row * column_count + column] = Some(index);
                }
            }
        }
    }

    // Horizontal spacing: walk every row, gap belongs to the left track.
    for row in 0..row_count {
        let mut previous: Option<(usize, usize)> = None;
        for column in 0..column_count {
            if columns[column].empty {
                continue;
            }
            let owner = grid[row * column_count + column];
            if let Some((previous_column, previous_owner)) = previous {
                if owner != Some(previous_owner) {
                    let track = &mut columns[previous_column];
                    track.spacing = track.spacing.max(spacing);
                }
            }
            previous = owner.map(|owner| (column, owner));
        }
    }

    // Vertical spacing: walk every column, gap belongs to the upper track.
    for column in 0..column_count {
        let mut previous: Option<(usize, usize)> = None;
        for row in 0..row_count {
            if rows[row].empty {
                continue;
            }
            let owner = grid[row * column_count + column];
            if let Some((previous_row, previous_owner)) = previous {
                if owner != Some(previous_owner) {
                    let track = &mut rows[previous_row];
                    track.spacing = track.spacing.max(spacing);
                }
            }
            previous = owner.map(|owner| (row, owner));
        }
    }
}

fn to_fixed(value: i32) -> i64 {
    value as i64 * 256
}

fn f_round(value: i64) -> i32 {
    if value % 256 < 128 {
        (value / 256) as i32
    } else {
        (1 + value / 256) as i32
    }
}

/// Port of Qt's `qGeomCalc()` (qlayoutengine.cpp) for a stretchless chain.
#[allow(clippy::needless_range_loop)]
fn q_geom_calc(chain: &mut [QGridTrack], start: usize, count: usize, pos: i32, space: i32) {
    if count == 0 {
        return;
    }
    let mut c_hint = 0;
    let mut c_min = 0;
    let mut sum_spacing = 0;
    let mut expanding_count = 0i32;
    let mut all_empty_nonstretch = true;
    let mut pending_spacing: Option<i32> = None;
    let mut spacer_count = 0i32;

    for index in start..start + count {
        let track = &mut chain[index];
        track.done = false;
        c_hint += track.hint;
        c_min += track.minimum;
        if !track.empty {
            if let Some(spacing) = pending_spacing {
                sum_spacing += spacing;
                spacer_count += 1;
            }
            pending_spacing = Some(track.spacing);
        }
        if track.expansive {
            expanding_count += 1;
        }
        all_empty_nonstretch = all_empty_nonstretch && track.empty && !track.expansive;
    }

    let mut extraspace = 0;

    if space < c_min + sum_spacing {
        // Less space than the minimum size: take from the biggest first.
        let mut minimums: Vec<i32> = (start..start + count)
            .map(|index| chain[index].minimum)
            .collect();
        minimums.sort_unstable();
        let space_left = space - sum_spacing;
        let mut sum = 0;
        let mut index = 0i64;
        let mut space_used = 0;
        let mut current = 0;
        while (index as usize) < count && space_used < space_left {
            current = minimums[index as usize];
            space_used = sum + current * (count - index as usize) as i32;
            sum += current;
            index += 1;
        }
        // Qt keeps a signed index here: when `space_left <= 0` the loop above
        // never runs and `--idx` leaves -1, which yields `items = count + 1`.
        index -= 1;
        let deficit = space_used - space_left;
        let items = count as i64 - index;
        let items = items.max(1) as i32;
        let deficit_per_item = deficit / items;
        let remainder = deficit % items;
        let maxval = current - deficit_per_item;
        let mut rest = 0;
        for track in &mut chain[start..start + count] {
            let mut maxv = maxval;
            rest += remainder;
            if rest >= items {
                maxv -= 1;
                rest -= items;
            }
            track.size = track.minimum.min(maxv);
            track.done = true;
        }
    } else if space < c_hint + sum_spacing {
        // Less than the hinted size but more than the minimum: take from
        // each track equally, giving up on tracks that hit their minimum.
        let mut remaining = count as i32;
        let space_left = space - sum_spacing;
        let mut overdraft = c_hint - space_left;

        for track in &mut chain[start..start + count] {
            if !track.done && track.minimum >= track.hint {
                track.size = track.hint;
                track.done = true;
                remaining -= 1;
            }
        }
        let mut finished = remaining == 0;
        while !finished {
            finished = true;
            let fp_over = to_fixed(overdraft);
            let mut fp_w: i64 = 0;
            for index in start..start + count {
                if chain[index].done {
                    continue;
                }
                fp_w += fp_over / remaining as i64;
                let width = f_round(fp_w);
                chain[index].size = chain[index].hint - width;
                fp_w -= to_fixed(width);
                if chain[index].size < chain[index].minimum {
                    let track = &mut chain[index];
                    track.done = true;
                    track.size = track.minimum;
                    finished = false;
                    overdraft -= track.hint - track.minimum;
                    remaining -= 1;
                    break;
                }
            }
        }
    } else {
        // Extra space: give it to expanders, otherwise split it evenly.
        let mut remaining = count as i32;
        let mut space_left = space - sum_spacing;
        for index in start..start + count {
            let track = &mut chain[index];
            if !track.done
                && (track.maximum <= track.hint
                    || (!all_empty_nonstretch && track.empty && !track.expansive))
            {
                track.size = track.hint;
                track.done = true;
                space_left -= track.size;
                if track.expansive {
                    expanding_count -= 1;
                }
                remaining -= 1;
            }
        }
        extraspace = space_left;

        let mut surplus;
        let mut deficit;
        loop {
            surplus = 0;
            deficit = 0;
            let fp_space = to_fixed(space_left);
            let mut fp_w: i64 = 0;
            for index in start..start + count {
                if chain[index].done {
                    continue;
                }
                extraspace = 0;
                if expanding_count > 0 {
                    let weight = if chain[index].expansive { 1 } else { 0 };
                    fp_w += (fp_space * weight) / expanding_count as i64;
                } else {
                    fp_w += fp_space / remaining as i64;
                }
                let width = f_round(fp_w);
                chain[index].size = width;
                fp_w -= to_fixed(width);
                if width < chain[index].hint {
                    deficit += chain[index].hint - width;
                } else if width > chain[index].maximum {
                    surplus += width - chain[index].maximum;
                }
            }
            if deficit > 0 && surplus <= deficit {
                for index in start..start + count {
                    let track = &mut chain[index];
                    if !track.done && track.size < track.hint {
                        track.size = track.hint;
                        track.done = true;
                        space_left -= track.hint;
                        if track.expansive {
                            expanding_count -= 1;
                        }
                        remaining -= 1;
                    }
                }
            }
            if surplus > 0 && surplus >= deficit {
                for index in start..start + count {
                    let track = &mut chain[index];
                    if !track.done && track.size > track.maximum {
                        track.size = track.maximum;
                        track.done = true;
                        space_left -= track.maximum;
                        if track.expansive {
                            expanding_count -= 1;
                        }
                        remaining -= 1;
                    }
                }
            }
            if remaining <= 0 || surplus == deficit {
                break;
            }
        }
        if remaining == 0 {
            extraspace = space_left;
        }
    }

    // Remaining space goes to the spacers, counting both ends of the chain.
    let extra = extraspace / (spacer_count + 2);
    let mut cursor = pos + extra;
    for track in &mut chain[start..start + count] {
        track.pos = cursor;
        cursor += track.size;
        if !track.empty {
            cursor += track.spacing + extra;
        }
    }
}

fn parse_size_policy(name: &str) -> Option<QSizePolicy> {
    let tail = name.rsplit("::").next().unwrap_or(name);
    Some(match tail {
        "Fixed" => QSizePolicy::Fixed,
        "Minimum" => QSizePolicy::Minimum,
        "Maximum" => QSizePolicy::Maximum,
        "Preferred" => QSizePolicy::Preferred,
        "Expanding" => QSizePolicy::Expanding,
        "MinimumExpanding" => QSizePolicy::MinimumExpanding,
        "Ignored" => QSizePolicy::Ignored,
        _ => return None,
    })
}

fn parse_scroll_policy(value: &QVariant) -> Option<QScrollBarPolicy> {
    let name = match value {
        QVariant::Int(value) => {
            return Some(match value {
                1 => QScrollBarPolicy::AlwaysOff,
                2 => QScrollBarPolicy::AlwaysOn,
                _ => QScrollBarPolicy::AsNeeded,
            });
        }
        QVariant::String(text) | QVariant::Enum(text) => text.as_str(),
        _ => return None,
    };
    let tail = name.rsplit("::").next().unwrap_or(name);
    Some(match tail {
        "ScrollBarAlwaysOff" | "AlwaysOff" | "Off" => QScrollBarPolicy::AlwaysOff,
        "ScrollBarAlwaysOn" | "AlwaysOn" | "On" => QScrollBarPolicy::AlwaysOn,
        "ScrollBarAsNeeded" | "AsNeeded" => QScrollBarPolicy::AsNeeded,
        _ => return None,
    })
}

fn scroll_policy_variant(policy: QScrollBarPolicy) -> QVariant {
    QVariant::Enum(
        match policy {
            QScrollBarPolicy::AsNeeded => "Qt::ScrollBarAsNeeded",
            QScrollBarPolicy::AlwaysOff => "Qt::ScrollBarAlwaysOff",
            QScrollBarPolicy::AlwaysOn => "Qt::ScrollBarAlwaysOn",
        }
        .to_string(),
    )
}

fn set_scroll_policy(
    _vertical: bool,
    bar: &mut crate::scroll::QScrollBar,
    value: &QVariant,
) -> bool {
    let Some(policy) = parse_scroll_policy(value) else {
        return false;
    };
    bar.policy = policy;
    true
}

fn set_list_property(list: &mut QListWidget, name: &str, value: &QVariant, font: &QFont) -> bool {
    match name {
        "items" => {
            list.set_items_variant(value, font);
            true
        }
        "currentIndex" => value
            .as_i64()
            .is_some_and(|index| list.set_current_index(index)),
        "currentRow" => match value.as_i64() {
            Some(row) if row >= 1 => list.set_current_index(row - 1),
            Some(_) => {
                list.set_current(None);
                true
            }
            None => false,
        },
        "verticalScrollBarPolicy" => set_scroll_policy(true, &mut list.vbar, value),
        "horizontalScrollBarPolicy" => set_scroll_policy(false, &mut list.hbar, value),
        _ => false,
    }
}

fn set_tree_property(tree: &mut QTreeWidget, name: &str, value: &QVariant, font: &QFont) -> bool {
    match name {
        "data" => {
            tree.set_data(value, font);
            true
        }
        "headerLabels" => {
            tree.set_headers_variant(value, font);
            true
        }
        "currentIndex" => value
            .as_i64()
            .is_some_and(|index| tree.set_current_index(index)),
        "currentRow" => match value.as_i64() {
            Some(row) if row >= 1 => tree.set_current_index(row - 1),
            Some(_) => {
                tree.set_current(None);
                true
            }
            None => false,
        },
        "verticalScrollBarPolicy" => set_scroll_policy(true, &mut tree.vbar, value),
        "horizontalScrollBarPolicy" => set_scroll_policy(false, &mut tree.hbar, value),
        "expandAll" => {
            if value.as_bool() == Some(true) {
                tree.expand_all();
            }
            true
        }
        "collapseAll" => {
            if value.as_bool() == Some(true) {
                tree.collapse_all();
            }
            true
        }
        _ => false,
    }
}

fn set_table_property(table: &mut QTableWidget, name: &str, value: &QVariant) -> bool {
    match name {
        "clear" => {
            if value.as_bool() == Some(true) {
                table.clear();
            }
            true
        }
        "rowCount" => {
            if let Some(count) = value.as_i64() {
                table.model.set_row_count(count.max(0) as usize);
                table
                    .selection
                    .prune(table.model.row_count(), table.model.column_count());
                true
            } else {
                false
            }
        }
        "columnCount" => {
            if let Some(count) = value.as_i64() {
                table.model.set_column_count(count.max(0) as usize);
                table
                    .selection
                    .prune(table.model.row_count(), table.model.column_count());
                true
            } else {
                false
            }
        }
        "horizontalHeaderLabels" | "columnWidths" | "hiddenColumns" => {
            table
                .model
                .apply_headers(&QVariant::Map(vec![(name.to_string(), value.clone())]));
            true
        }
        "hideColumn" => {
            table.model.hide_column(value);
            true
        }
        "tableHeaders" => {
            table.model.apply_headers(value);
            true
        }
        "tableLayout" | "groupedHeader" => {
            table.apply_table_layout(value);
            true
        }
        "tableData" => {
            table.model.apply_data(value);
            table
                .selection
                .prune(table.model.row_count(), table.model.column_count());
            true
        }
        "data" => {
            let row_count = table.model.row_count() as i64;
            table.model.apply_data(&QVariant::Map(vec![
                ("rowCount".into(), QVariant::Int(row_count)),
                ("data".into(), value.clone()),
            ]));
            table
                .selection
                .prune(table.model.row_count(), table.model.column_count());
            true
        }
        "selectRows" => {
            if let Some(rows) = value.as_bool() {
                table.selection_mode = if rows {
                    QSelectionMode::SelectRows
                } else {
                    QSelectionMode::SelectItems
                };
                true
            } else {
                false
            }
        }
        "readOnly" => {
            if let Some(read_only) = value.as_bool() {
                table.read_only = read_only;
                true
            } else {
                false
            }
        }
        "editTriggers" => {
            table.edit_trigger = QEditTrigger::AnyKeyPressed;
            true
        }
        "currentRow" => value
            .as_i64()
            .is_some_and(|row| table.set_current_row_1based(row)),
        "verticalScrollBarPolicy" => set_scroll_policy(true, &mut table.vbar, value),
        "horizontalScrollBarPolicy" => set_scroll_policy(false, &mut table.hbar, value),
        _ => false,
    }
}

pub fn load_ui(xml: &str) -> Result<(QWidgetTree, QWidgetId), String> {
    let form = form_xml::parse(xml).map_err(|error| error.to_string())?;
    let mut tree = QWidgetTree::new();
    let root = load_widget(&mut tree, &form.root, None, true, false);
    let rect = match form.root.property("geometry") {
        Some(form_xml::PropValue::Rect { x, y, w, h }) => {
            QRectF::new(*x as f32, *y as f32, *w as f32, *h as f32)
        }
        _ => QRectF::new(0.0, 0.0, 400.0, 300.0),
    };
    tree.layout(root, rect);
    let explicit_default = tree.widgets.iter().any(|widget| {
        widget.class_name == "QPushButton"
            && matches!(widget.property("default"), Some(QVariant::Bool(_)))
    });
    if !explicit_default {
        if let Some(button) = tree
            .widgets
            .iter()
            .position(|widget| widget.class_name == "QPushButton")
        {
            tree.widgets[button].default_button = true;
        }
    }
    Ok((tree, root))
}

fn load_widget(
    tree: &mut QWidgetTree,
    spec: &form_xml::Widget,
    parent: Option<QWidgetId>,
    is_root: bool,
    explicit: bool,
) -> QWidgetId {
    let id = tree.create_widget(spec.class.clone(), spec.name.clone());
    for (name, value) in &spec.properties {
        tree.set_property(id, name.clone(), QVariant::from_prop(value));
    }
    if let Some(QVariant::Size(size)) = tree.widgets[id].property("iconSize").cloned() {
        tree.widgets[id].icon_size = size;
    }
    if let Some(QVariant::Bool(default)) = tree.widgets[id].property("default").cloned() {
        tree.widgets[id].default_button = default;
    }
    if let Some(QVariant::Map(entries)) = tree.widgets[id].property("icon").cloned() {
        let icon_path = entries
            .iter()
            .find(|(key, _)| key == "normaloff")
            .and_then(|(_, value)| value.as_str());
        let disabled_path = entries
            .iter()
            .find(|(key, _)| key == "disabledoff")
            .and_then(|(_, value)| value.as_str());
        tree.widgets[id].icon = icon_path.and_then(load_pixmap);
        tree.widgets[id].icon_disabled = disabled_path.and_then(load_pixmap);
    }
    // `<column>` header captions: Qt shows them in the horizontal header;
    // without them we fell back to section numbers (gallery/mixed/206
    // rendered "1" where Qt shows "Имя").
    if !spec.columns.is_empty() {
        match spec.class.as_str() {
            "QTreeWidget" => {
                if let Some(tree) = tree.widgets[id].tree.as_mut() {
                    tree.headers = spec.columns.clone();
                }
            }
            "QTableWidget" => {
                if let Some(table) = tree.widgets[id].table.as_mut() {
                    if table.model.column_count() < spec.columns.len() {
                        table.model.set_column_count(spec.columns.len());
                    }
                    for (index, text) in spec.columns.iter().enumerate() {
                        table.model.set_header(index, text.clone());
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(parent) = parent {
        tree.add_child(parent, id);
    }
    if let Some(form_xml::PropValue::Rect { x, y, w, h }) = spec.property("geometry") {
        tree.widgets[id].geometry = QRectF::new(*x as f32, *y as f32, *w as f32, *h as f32);
        if explicit {
            tree.widgets[id].explicit_position = Some(QPointF::new(*x as f32, *y as f32));
        }
    }
    if let Some(layout) = &spec.layout {
        let explicit_margins = layout.margins.is_some();
        let mut layout = load_layout(tree, layout, id, is_root);
        if !is_root && !explicit_margins {
            layout.margins = QMargins::uniform(9.0);
        }
        if spec.class == "QGroupBox" {
            let metrics = QFontMetrics::new(&tree.widgets[id].font);
            layout.margins.left += 3.0;
            layout.margins.top += metrics.height() + 6.0;
            layout.margins.right += 3.0;
            layout.margins.bottom += 3.0;
        }
        tree.widgets[id].layout = Some(layout);
    }
    for child in &spec.children {
        load_widget(tree, child, Some(id), false, true);
    }
    if spec.class == "QTabWidget" {
        let titles: Vec<String> = spec
            .children
            .iter()
            .map(|page| {
                page.attribute("title")
                    .and_then(|value| match value {
                        form_xml::PropValue::Str(text) | form_xml::PropValue::Enum(text) => {
                            Some(text.clone())
                        }
                        _ => value.as_str().map(str::to_string),
                    })
                    .unwrap_or_else(|| page.name.clone())
            })
            .collect();
        let current = tree
            .widgets
            .get(id)
            .and_then(|node| node.property("currentIndex"))
            .and_then(value_as_i64)
            .unwrap_or(0)
            .max(0) as usize;
        if let Some(node) = tree.widgets.get_mut(id) {
            if let Some(tabs) = node.tabs.as_mut() {
                tabs.titles = titles;
                tabs.current_index = current.min(node.children.len().saturating_sub(1));
            }
        }
        tree.sync_tab_pages(id);
    }
    id
}

fn load_pixmap(path: &str) -> Option<QPixmap> {
    if let Some(pixmap) = crate::icons::pixmap(path) {
        return Some(pixmap);
    }
    if path.starts_with(":/") || path.starts_with("data:") || path.is_empty() {
        return None;
    }
    let image = crate::raster::QRasterImage::from_png(path).ok()?;
    Some(QPixmap::new(image.width, image.height, image.pixels))
}

fn load_layout(
    tree: &mut QWidgetTree,
    spec: &form_xml::Layout,
    parent: QWidgetId,
    is_root: bool,
) -> QLayout {
    let kind = QLayoutKind::from_class(&spec.class);
    let margins = match spec.margins {
        Some((left, top, right, bottom)) => {
            QMargins::new(left as f32, top as f32, right as f32, bottom as f32)
        }
        None if is_root => QMargins::uniform(11.0),
        None => QMargins::default(),
    };
    let mut layout = QLayout {
        kind,
        spacing: spec.spacing.map(|value| value as f32).unwrap_or(6.0),
        margins,
        field_growth_policy: spec.field_growth_policy,
        row_wrap_policy: spec.row_wrap_policy,
        label_alignment: spec.label_alignment,
        form_alignment: spec.form_alignment,
        horizontal_spacing: spec.horizontal_spacing.map(|value| value as f32),
        vertical_spacing: spec.vertical_spacing.map(|value| value as f32),
        items: Vec::new(),
    };

    for item in &spec.items {
        match item {
            form_xml::LayoutItem::Widget(widget, cell) => {
                let id = load_widget(tree, widget, Some(parent), false, false);
                layout.items.push(QLayoutItem::Widget { id, cell: *cell });
            }
            form_xml::LayoutItem::Layout(inner, cell) => {
                layout.items.push(QLayoutItem::Layout {
                    layout: Box::new(load_layout(tree, inner, parent, false)),
                    cell: *cell,
                });
            }
            form_xml::LayoutItem::Spacer(spacer, cell) => {
                layout.items.push(QLayoutItem::Spacer {
                    item: QSpacerItem {
                        width: spacer.hint.0 as f32,
                        height: spacer.hint.1 as f32,
                        horizontal_policy: if spacer.horizontal {
                            QSizePolicy::Expanding
                        } else {
                            QSizePolicy::Fixed
                        },
                        vertical_policy: if spacer.horizontal {
                            QSizePolicy::Fixed
                        } else {
                            QSizePolicy::Expanding
                        },
                    },
                    cell: *cell,
                });
            }
        }
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID_2X2: &str = include_str!("../tests/fixtures/grid_2x2.ui");
    const GRID_SPANS: &str = include_str!("../tests/fixtures/grid_spans.ui");
    const GRID_NESTED: &str = include_str!("../tests/fixtures/grid_nested.ui");
    const GRID_POLICIES: &str = include_str!("../tests/fixtures/grid_policies.ui");
    const BOX_BUTTONS: &str = include_str!("../tests/fixtures/box_buttons.ui");
    const QT6_GRID_2X2: &str = include_str!("../tests/fixtures/qt6_grid_2x2.json");
    const QT6_GRID_SPANS: &str = include_str!("../tests/fixtures/qt6_grid_spans.json");
    const QT6_GRID_NESTED: &str = include_str!("../tests/fixtures/qt6_grid_nested.json");
    const QT6_GRID_POLICIES: &str = include_str!("../tests/fixtures/qt6_grid_policies.json");
    const QT6_BOX_BUTTONS: &str = include_str!("../tests/fixtures/qt6_box_buttons.json");
    const FORM_BASIC: &str = include_str!("../tests/fixtures/form_basic.ui");
    const FORM_WRAP_LONG: &str = include_str!("../tests/fixtures/form_wrap_long.ui");
    const FORM_WRAP_ALL: &str = include_str!("../tests/fixtures/form_wrap_all.ui");
    const FORM_GROWTH: &str = include_str!("../tests/fixtures/form_growth.ui");
    const FORM_ALIGNMENT: &str = include_str!("../tests/fixtures/form_alignment.ui");
    const FORM_GROUP: &str = include_str!("../tests/fixtures/form_group.ui");
    const SPLITTER_H: &str = include_str!("../tests/fixtures/splitter_h.ui");
    const SPLITTER_V: &str = include_str!("../tests/fixtures/splitter_v.ui");
    const QT6_SPLITTER_H: &str = include_str!("../tests/fixtures/qt6_splitter_h.json");
    const QT6_SPLITTER_V: &str = include_str!("../tests/fixtures/qt6_splitter_v.json");
    const QT6_FORM_BASIC: &str = include_str!("../tests/fixtures/qt6_form_basic.json");
    const QT6_FORM_WRAP_LONG: &str = include_str!("../tests/fixtures/qt6_form_wrap_long.json");
    const QT6_FORM_WRAP_ALL: &str = include_str!("../tests/fixtures/qt6_form_wrap_all.json");
    const QT6_FORM_GROWTH: &str = include_str!("../tests/fixtures/qt6_form_growth.json");
    const QT6_FORM_ALIGNMENT: &str = include_str!("../tests/fixtures/qt6_form_alignment.json");
    const QT6_FORM_GROUP: &str = include_str!("../tests/fixtures/qt6_form_group.json");

    /// Compare every named widget against the Qt6 offscreen reference.
    fn assert_geometry_matches_qt6(ui: &str, reference: &str, tolerance: f32) {
        let (tree, _) = load_ui(ui).expect("loads ui");
        let reference: serde_json::Value = serde_json::from_str(reference).expect("reference json");
        let widgets = reference["widgets"].as_array().expect("widgets array");
        let mut failures = Vec::new();
        for expected in widgets {
            let name = expected["name"].as_str().expect("name");
            if name.starts_with("qt_") {
                // Internal Qt children of scroll areas (viewports, bars).
                continue;
            }
            let Some(id) = tree.find_widget(name) else {
                failures.push(format!("{name}: missing widget"));
                continue;
            };
            let actual = tree.widget(id).unwrap();
            let class = expected["class"].as_str().expect("class");
            if actual.class_name != class {
                failures.push(format!(
                    "{name}.class: expected {class}, got {}",
                    actual.class_name
                ));
                continue;
            }
            for (key, actual) in [
                ("x", actual.geometry.x),
                ("y", actual.geometry.y),
                ("w", actual.geometry.width),
                ("h", actual.geometry.height),
            ] {
                let expected = expected[key]
                    .as_f64()
                    .unwrap_or_else(|| panic!("{key} missing"));
                if (actual - expected as f32).abs() > tolerance {
                    failures.push(format!("{name}.{key}: expected {expected}, got {actual}"));
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn form_basic_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_BASIC, QT6_FORM_BASIC, 1.0);
    }

    #[test]
    fn form_wrap_long_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_WRAP_LONG, QT6_FORM_WRAP_LONG, 1.0);
    }

    #[test]
    fn form_wrap_all_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_WRAP_ALL, QT6_FORM_WRAP_ALL, 1.0);
    }

    #[test]
    fn form_growth_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_GROWTH, QT6_FORM_GROWTH, 1.0);
    }

    #[test]
    fn form_alignment_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_ALIGNMENT, QT6_FORM_ALIGNMENT, 1.0);
    }

    #[test]
    fn form_group_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(FORM_GROUP, QT6_FORM_GROUP, 1.0);
    }

    #[test]
    fn splitter_horizontal_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(SPLITTER_H, QT6_SPLITTER_H, 1.0);
    }

    #[test]
    fn splitter_vertical_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(SPLITTER_V, QT6_SPLITTER_V, 1.0);
    }

    #[test]
    fn splitter_contributes_child_minimums_to_form_minimum() {
        let (mut tree, root) = load_ui(SPLITTER_H).expect("loads ui");
        assert_eq!(tree.minimum_size_hint(root), QSizeF::new(166.0, 92.0));
        let (mut tree, root) = load_ui(SPLITTER_V).expect("loads ui");
        assert_eq!(tree.minimum_size_hint(root), QSizeF::new(92.0, 166.0));
    }

    #[test]
    fn grid_2x2_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(GRID_2X2, QT6_GRID_2X2, 1.0);
    }

    #[test]
    fn grid_spans_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(GRID_SPANS, QT6_GRID_SPANS, 1.0);
    }

    #[test]
    fn grid_nested_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(GRID_NESTED, QT6_GRID_NESTED, 1.0);
    }

    #[test]
    fn grid_policies_geometry_matches_qt6_offscreen() {
        assert_geometry_matches_qt6(GRID_POLICIES, QT6_GRID_POLICIES, 1.0);
    }

    #[test]
    fn box_buttons_grow_without_spacer_matches_qt6_offscreen() {
        // No expander: Qt's `qGeomCalc` gives every button an equal share of
        // the extra width instead of pushing the surplus into the gaps.
        assert_geometry_matches_qt6(BOX_BUTTONS, QT6_BOX_BUTTONS, 1.0);
    }

    const OVERFLOW_FORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>OverflowDialog</class>
 <widget class="QDialog" name="OverflowDialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>120</width><height>60</height></rect></property>
  <layout class="QVBoxLayout" name="verticalLayout">
   <item><widget class="QPushButton" name="first">
    <property name="text"><string>First</string></property>
    <property name="minimumSize"><size><width>80</width><height>50</height></size></property>
   </widget></item>
   <item><widget class="QPushButton" name="second">
    <property name="text"><string>Second</string></property>
    <property name="minimumSize"><size><width>80</width><height>50</height></size></property>
   </widget></item>
  </layout>
 </widget>
</ui>"#;

    #[test]
    fn layout_minimum_raises_form_minimum_size() {
        let (mut tree, root) = load_ui(OVERFLOW_FORM).expect("loads ui");
        // Two 50 px buttons + 6 px spacing + 2 × 11 px margins.
        assert_eq!(tree.minimum_size_hint(root), QSizeF::new(102.0, 128.0));
    }

    #[test]
    fn overflowing_child_is_clipped_and_not_hit_outside_root() {
        use crate::gui::QPaintCommand;

        let (tree, root) = load_ui(OVERFLOW_FORM).expect("loads ui");
        let root_rect = tree.widget(root).unwrap().geometry;
        let second = tree.find_widget("second").expect("second button");
        let second_rect = tree.widget(second).unwrap().geometry;
        assert!(
            second_rect.bottom() > root_rect.bottom(),
            "fixture must overflow: {second_rect:?} vs {root_rect:?}"
        );

        // A point inside the overflowing button but outside the form root is
        // clipped by Qt and must not reach the button.
        let outside = QPointF::new(second_rect.x + 5.0, second_rect.y + 5.0);
        assert!(!root_rect.contains(outside));
        assert_eq!(tree.widget_at(root, outside), None);

        let mut painter = crate::gui::QPainter::new();
        crate::paint::paint_widget_tree(
            &tree,
            root,
            &crate::style::QFusionStyle::default(),
            &mut painter,
        );
        for command in painter.commands() {
            if let QPaintCommand::SetClip { rect: Some(rect) } = command {
                if rect.is_empty() {
                    continue;
                }
                assert!(
                    rect.left() >= root_rect.left() - 0.01
                        && rect.top() >= root_rect.top() - 0.01
                        && rect.right() <= root_rect.right() + 0.01
                        && rect.bottom() <= root_rect.bottom() + 0.01,
                    "clip escapes root: {rect:?} vs {root_rect:?}"
                );
            }
        }
    }

    const FIXTURE: &str = include_str!("../../form-xml/tests/fixtures/test_dialog01.ui");
    const DIALOG02: &str = include_str!("../../../../server/priv/qt_forms/test_dialog02.ui");
    const QT6_DIALOG01: &str = include_str!("../tests/fixtures/qt6_test_dialog01.json");
    const QT6_DIALOG02: &str = include_str!("../tests/fixtures/qt6_test_dialog02.json");

    #[test]
    fn dialog01_geometry_matches_qt6_offscreen() {
        let (tree, _) = load_ui(FIXTURE).expect("loads ui");
        let reference: serde_json::Value =
            serde_json::from_str(QT6_DIALOG01).expect("reference json");
        let widgets = reference["widgets"].as_array().expect("widgets array");

        let number = |value: &serde_json::Value, key: &str| {
            value[key]
                .as_f64()
                .unwrap_or_else(|| panic!("{key} missing"))
        };
        let near = |context: &str, left: f32, right: f64| {
            assert!(
                (left - right as f32).abs() <= 1.0,
                "{context}: expected {right}, got {left}"
            );
        };

        for expected in widgets {
            let name = expected["name"].as_str().expect("name");
            let id = tree
                .find_widget(name)
                .unwrap_or_else(|| panic!("missing widget {name}"));
            let actual = tree.widget(id).unwrap();
            assert_eq!(actual.class_name, expected["class"].as_str().unwrap());
            near(
                &format!("{name}.x"),
                actual.geometry.x,
                number(expected, "x"),
            );
            near(
                &format!("{name}.y"),
                actual.geometry.y,
                number(expected, "y"),
            );
            near(
                &format!("{name}.w"),
                actual.geometry.width,
                number(expected, "w"),
            );
            near(
                &format!("{name}.h"),
                actual.geometry.height,
                number(expected, "h"),
            );
        }
    }

    #[test]
    fn dialog02_geometry_matches_qt6_offscreen() {
        let (tree, _) = load_ui(DIALOG02).expect("loads ui");
        let reference: serde_json::Value =
            serde_json::from_str(QT6_DIALOG02).expect("reference json");
        let widgets = reference["widgets"].as_array().expect("widgets array");

        let number = |value: &serde_json::Value, key: &str| {
            value[key]
                .as_f64()
                .unwrap_or_else(|| panic!("{key} missing"))
        };
        let mut failures = Vec::new();
        for expected in widgets {
            let name = expected["name"].as_str().expect("name");
            let id = tree
                .find_widget(name)
                .unwrap_or_else(|| panic!("missing widget {name}"));
            let actual = tree.widget(id).unwrap();
            if actual.class_name != expected["class"].as_str().unwrap() {
                failures.push(format!("{name}.class"));
                continue;
            }
            for (key, actual) in [
                ("x", actual.geometry.x),
                ("y", actual.geometry.y),
                ("w", actual.geometry.width),
                ("h", actual.geometry.height),
            ] {
                let expected = number(expected, key);
                if (actual - expected as f32).abs() > 3.0 {
                    failures.push(format!("{name}.{key}: expected {expected}, got {actual}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn grid_with_spacers_in_tight_rect_does_not_panic() {
        // Only spacers: minimum sizes are 0, so a rect that is smaller than
        // the spacings drives `qGeomCalc` into its shrinking branch with a
        // non-positive `space_left` (a signed index in Qt, guarded here).
        const ONLY_SPACERS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>12</width><height>8</height></rect></property>
  <layout class="QGridLayout" name="grid">
   <item row="0" column="0"><spacer name="s1">
    <property name="orientation"><enum>Qt::Orientation::Horizontal</enum></property>
    <property name="sizeHint" stdset="0"><size><width>40</width><height>20</height></size></property>
   </spacer></item>
   <item row="1" column="1"><spacer name="s2">
    <property name="orientation"><enum>Qt::Orientation::Vertical</enum></property>
    <property name="sizeHint" stdset="0"><size><width>20</width><height>40</height></size></property>
   </spacer></item>
  </layout>
 </widget>
</ui>"#;
        let (mut tree, root) = load_ui(ONLY_SPACERS).expect("loads ui");
        tree.layout(root, QRectF::new(0.0, 0.0, 4.0, 3.0));
        let dialog = tree.widget(root).unwrap().geometry;
        assert_eq!((dialog.width, dialog.height), (4.0, 3.0));
    }

    #[test]
    fn loader_builds_qt_widget_tree_and_layout() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let label = tree.find_widget("label_1").expect("label");
        let line = tree.find_widget("lineEdit_1").expect("line edit");
        let button = tree.find_widget("pushButton_2").expect("button");

        assert_eq!(tree.widget(root).unwrap().class_name, "QDialog");
        assert_eq!(tree.widget(label).unwrap().class_name, "QLabel");
        assert_eq!(tree.widget(line).unwrap().class_name, "QLineEdit");
        assert_eq!(tree.widget(button).unwrap().geometry.bottom(), 179.0);
        assert_eq!(tree.widget(button).unwrap().geometry.right(), 377.0);
        assert_eq!(tree.widget(label).unwrap().geometry.width.round(), 67.0);
        assert_eq!(tree.widget(line).unwrap().geometry.width.round(), 146.0);
        assert_eq!(tree.widget(root).unwrap().geometry.width, 388.0);
    }

    #[test]
    fn label_size_hint_uses_real_font_metrics() {
        let (tree, _) = load_ui(FIXTURE).expect("loads ui");
        let label = tree.find_widget("label_1").expect("label");
        let metrics = QFontMetrics::new(&QFont::default());
        let width = tree.widget(label).unwrap().size_hint.width;
        assert!((width - metrics.horizontal_advance("TextLabel").ceil()).abs() < 0.01);
    }

    fn center(tree: &QWidgetTree, widget: QWidgetId) -> QPointF {
        let rect = tree.widget(widget).unwrap().geometry;
        QPointF::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    #[test]
    fn focus_first_uses_line_edit_or_table_and_current_not_selected() {
        let table_only = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QTableWidget" name="table"><property name="rowCount"><number>2</number></property><property name="columnCount"><number>2</number></property></widget></item></layout></widget></ui>"#;
        let (mut tree, root) = load_ui(table_only).unwrap();
        let table = tree.find_widget("table").unwrap();
        tree.layout(root, tree.widget(root).unwrap().geometry);
        assert_eq!(tree.focus_first(root), Some(table));
        let model = tree.table(table).unwrap();
        assert_eq!(model.current(), Some((0, 0)));
        assert!(!model.is_selected(0, 0));

        let form = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>240</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QLineEdit" name="edit"/></item><item><widget class="QTableWidget" name="table"><property name="rowCount"><number>2</number></property><property name="columnCount"><number>2</number></property></widget></item></layout></widget></ui>"#;
        let (mut tree, root) = load_ui(form).unwrap();
        let edit = tree.find_widget("edit").unwrap();
        let table = tree.find_widget("table").unwrap();
        tree.layout(root, tree.widget(root).unwrap().geometry);
        assert_eq!(tree.focus_first(root), Some(edit));
        assert_eq!(tree.table(table).unwrap().current(), None);
    }

    #[test]
    fn double_tap_selects_the_word_under_the_cursor() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let line = tree.find_widget("lineEdit_1").expect("line edit");
        tree.set_property(line, "text", QVariant::String("hello world".into()));
        tree.layout(root, tree.widget(root).unwrap().geometry);
        let font = tree.widget(line).unwrap().font.clone();
        let metrics = QFontMetrics::new(&font);
        let local_x = 3.0 + metrics.horizontal_advance("hello wor");
        assert!(tree.select_word_at(line, local_x));
        assert_eq!(tree.line_edit_selection(line), Some((6, 11)));
    }

    #[test]
    fn focus_follows_click_and_tab_order() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let line_1 = tree.find_widget("lineEdit_1").unwrap();
        let line_2 = tree.find_widget("lineEdit_2").unwrap();
        let line_3 = tree.find_widget("lineEdit_3").unwrap();
        let button_1 = tree.find_widget("pushButton_1").unwrap();
        let button_2 = tree.find_widget("pushButton_2").unwrap();

        let point = center(&tree, line_1);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert_eq!(tree.focus_widget(), Some(line_1));

        assert_eq!(tree.focus_next(root, false), Some(line_2));
        assert_eq!(tree.focus_next(root, false), Some(line_3));
        assert_eq!(tree.focus_next(root, false), Some(button_1));
        assert_eq!(tree.focus_next(root, false), Some(button_2));
        assert_eq!(tree.focus_next(root, false), Some(line_1));
        assert_eq!(tree.focus_next(root, true), Some(button_2));
    }

    #[test]
    fn line_edit_selection_clears_on_focus_out() {
        let (mut tree, _) = load_ui(FIXTURE).expect("loads ui");
        let line = tree.find_widget("lineEdit_1").expect("line edit");
        let button = tree.find_widget("pushButton_1").expect("button");
        tree.set_text(line, "abcdef".into());
        tree.set_focus(line);
        tree.select_all(line);
        assert_eq!(tree.line_edit_selection(line), Some((0, 6)));

        tree.set_focus(button);
        assert!(!tree.widget(line).unwrap().has_focus);
        assert_eq!(tree.line_edit_selection(line), None);
        assert_eq!(tree.cursor_position(line), 6);

        tree.set_focus(line);
        assert_eq!(tree.line_edit_selection(line), None);
    }

    #[test]
    fn mouse_events_match_qt_button_semantics() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let label = tree.find_widget("label_1").unwrap();
        let button = tree.find_widget("pushButton_1").unwrap();

        let label_point = center(&tree, label);
        tree.mouse_press(root, label_point);
        let event = tree.mouse_release(root, label_point).unwrap();
        assert_eq!(event.widget, label);
        assert_eq!(event.protocol_event(), "MouseButtonRelease");
        assert!(!event.clicked);

        let button_point = center(&tree, button);
        tree.mouse_press(root, button_point);
        let event = tree.mouse_release(root, button_point).unwrap();
        assert_eq!(event.widget, button);
        assert_eq!(event.protocol_event(), "Clicked");
        assert!(event.clicked);
        assert_eq!(tree.focus_widget(), Some(button));
    }

    #[test]
    fn button_pressed_visual_follows_pointer_while_held() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let button = tree.find_widget("pushButton_1").unwrap();
        let rect = tree.widget(button).unwrap().geometry;
        let center = QPointF::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let outside = QPointF::new(rect.x - 10.0, center.y);

        tree.mouse_press(root, center);
        assert!(tree.widget(button).unwrap().pressed);
        assert!(tree.mouse_move(outside));
        assert!(!tree.widget(button).unwrap().pressed);
        assert!(tree.mouse_move(center));
        assert!(tree.widget(button).unwrap().pressed);
        let event = tree.mouse_release(root, center).unwrap();
        assert!(event.clicked);
        assert_eq!(event.protocol_event(), "Clicked");
        assert!(!tree.widget(button).unwrap().pressed);
    }

    #[test]
    fn button_release_outside_is_not_a_click() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let button = tree.find_widget("pushButton_1").unwrap();
        let rect = tree.widget(button).unwrap().geometry;
        let center = QPointF::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let outside = QPointF::new(rect.x - 10.0, center.y);

        tree.mouse_press(root, center);
        tree.mouse_move(outside);
        let event = tree.mouse_release(root, outside).unwrap();
        assert_eq!(event.widget, button);
        assert!(!event.clicked);
        assert!(!tree.widget(button).unwrap().pressed);
    }

    #[test]
    fn line_edit_cursor_inserts_in_the_middle() {
        let (mut tree, _) = load_ui(FIXTURE).expect("loads ui");
        let line = tree.find_widget("lineEdit_1").unwrap();
        tree.set_text(line, "abc".into());
        assert_eq!(tree.cursor_position(line), 3);
        tree.set_cursor_position(line, 1);
        assert!(tree.insert_text(line, 'X'));
        assert_eq!(tree.text(line).as_deref(), Some("aXbc"));
        assert_eq!(tree.cursor_position(line), 2);
        assert!(tree.backspace_text(line));
        assert_eq!(tree.text(line).as_deref(), Some("abc"));
        assert_eq!(tree.cursor_position(line), 1);
    }

    #[test]
    fn button_release_without_seen_press_still_clicks() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let button = tree.find_widget("pushButton_1").unwrap();
        let point = center(&tree, button);
        let event = tree.mouse_release(root, point).expect("release event");
        assert_eq!(event.widget, button);
        assert_eq!(event.protocol_event(), "Clicked");
    }

    #[test]
    fn checkboxes_and_radios_toggle_like_qt() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let checkbox = tree.find_widget("checkBox_2").unwrap();
        let radio_1 = tree.find_widget("radioButton_1").unwrap();
        let radio_2 = tree.find_widget("radioButton_2").unwrap();

        assert!(!tree.checked(checkbox));
        let point = center(&tree, checkbox);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert!(tree.checked(checkbox));
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert!(!tree.checked(checkbox));

        tree.set_checked(radio_2, true);
        assert!(tree.checked(radio_2));
        tree.set_checked(radio_1, true);
        assert!(tree.checked(radio_1));
        assert!(!tree.checked(radio_2));

        let radio_2_point = center(&tree, radio_2);
        tree.mouse_press(root, radio_2_point);
        tree.mouse_release(root, radio_2_point);
        assert!(tree.checked(radio_2));
        assert!(!tree.checked(radio_1));
        tree.mouse_press(root, radio_2_point);
        tree.mouse_release(root, radio_2_point);
        assert!(tree.checked(radio_2));

        tree.set_property(radio_1, "autoExclusive", QVariant::Bool(false));
        tree.set_checked(radio_1, true);
        assert!(tree.checked(radio_1));
        assert!(tree.checked(radio_2));
        let radio_1_point = center(&tree, radio_1);
        tree.mouse_press(root, radio_1_point);
        tree.mouse_release(root, radio_1_point);
        assert!(!tree.checked(radio_1));
        assert!(tree.checked(radio_2));
    }

    #[test]
    fn checked_property_routes_through_radio_group() {
        let (mut tree, _) = load_ui(DIALOG02).expect("loads ui");
        let radio_1 = tree.find_widget("radioButton_1").unwrap();
        let radio_2 = tree.find_widget("radioButton_2").unwrap();

        tree.set_property(radio_2, "checked", QVariant::Bool(true));
        assert!(tree.checked(radio_2));
        assert!(!tree.checked(radio_1));
        tree.set_property(radio_1, "checked", QVariant::Bool(true));
        assert!(tree.checked(radio_1));
        assert!(!tree.checked(radio_2));
    }

    #[test]
    fn disabled_widgets_do_not_take_focus_or_clicks() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let button = tree.find_widget("pushButton_1").unwrap();
        tree.set_enabled(button, false);
        let event = tree.mouse_press(root, center(&tree, button));
        assert_ne!(event.map(|event| event.widget), Some(button));
        if let Some(event) = tree.mouse_release(root, center(&tree, button)) {
            assert_ne!(event.protocol_event(), "Clicked");
        }
        assert_eq!(
            tree.focus_next(root, false),
            Some(tree.find_widget("lineEdit_1").unwrap())
        );
    }

    #[test]
    fn focusing_list_and_tree_selects_first_current() {
        let (mut tree, _root) = load_ui(DIALOG02).expect("loads ui");
        let list = tree.find_widget("listWidget").expect("list");
        let tree_widget = tree.find_widget("treeWidget").expect("tree");
        tree.set_property(
            list,
            "items",
            QVariant::List(vec![
                QVariant::String("one".into()),
                QVariant::String("two".into()),
            ]),
        );
        tree.set_property(
            tree_widget,
            "headerLabels",
            QVariant::List(vec![QVariant::String("Name".into())]),
        );
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![QVariant::Map(vec![(
                "text".into(),
                QVariant::String("Root".into()),
            )])]),
        );

        assert!(tree.set_focus(list));
        assert_eq!(
            tree.get_table_property(list, "currentRow"),
            Some(QVariant::Int(0))
        );
        assert!(tree.set_focus(tree_widget));
        assert_eq!(
            tree.get_table_property(tree_widget, "currentRow"),
            Some(QVariant::Int(0))
        );
    }

    #[test]
    fn list_click_selects_current_row() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let list = tree.find_widget("listWidget").expect("list");
        tree.set_property(
            list,
            "items",
            QVariant::List(vec![
                QVariant::String("one".into()),
                QVariant::String("two".into()),
                QVariant::String("three".into()),
            ]),
        );
        tree.relayout(root);
        let rect = tree.widget(list).unwrap().geometry;
        let point = QPointF::new(rect.x + 8.0, rect.y + 40.0);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert_eq!(
            tree.get_table_property(list, "currentRow"),
            Some(QVariant::Int(1))
        );
        assert_eq!(
            tree.get_table_property(list, "currentText"),
            Some(QVariant::String("two".into()))
        );
    }

    #[test]
    fn tree_click_selects_current_item() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let tree_widget = tree.find_widget("treeWidget").expect("tree");
        tree.set_property(
            tree_widget,
            "headerLabels",
            QVariant::List(vec![QVariant::String("Name".into())]),
        );
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![
                QVariant::Map(vec![
                    ("text".into(), QVariant::String("Root".into())),
                    (
                        "children".into(),
                        QVariant::List(vec![QVariant::Map(vec![(
                            "text".into(),
                            QVariant::String("Child".into()),
                        )])]),
                    ),
                ]),
                QVariant::Map(vec![("text".into(), QVariant::String("Other".into()))]),
            ]),
        );
        tree.tree_mut(tree_widget).unwrap().expand_all();
        tree.relayout(root);
        let rect = tree.widget(tree_widget).unwrap().geometry;
        let point = QPointF::new(rect.x + 8.0, rect.y + QVIEW_HEADER_H + 35.0);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert_eq!(
            tree.get_table_property(tree_widget, "currentRow"),
            Some(QVariant::Int(1))
        );
        assert_eq!(
            tree.get_table_property(tree_widget, "currentText"),
            Some(QVariant::String("Child".into()))
        );
    }

    fn combos_form() -> (QWidgetTree, QWidgetId, QWidgetId) {
        let mut tree = QWidgetTree::new();
        let root = tree.create_widget("QDialog", "Dialog");
        tree.set_geometry(root, QRectF::new(0.0, 0.0, 300.0, 200.0));
        let combo = tree.create_widget("QComboBox", "combo");
        tree.add_child(root, combo);
        tree.set_geometry(combo, QRectF::new(10.0, 10.0, 120.0, 25.0));
        tree.set_property(
            combo,
            "items",
            QVariant::List(vec![
                QVariant::String("one".into()),
                QVariant::String("two".into()),
                QVariant::String("three".into()),
            ]),
        );
        tree.set_property(combo, "currentIndex", QVariant::Int(0));
        (tree, root, combo)
    }

    #[test]
    fn combo_popup_opens_and_selects_an_item_on_release() {
        let (mut tree, root, combo) = combos_form();
        assert!(!tree.combo_popup_open(combo));
        let combo_rect = tree.widget(combo).unwrap().geometry;
        tree.mouse_press(root, QPointF::new(combo_rect.x + 5.0, combo_rect.y + 5.0));
        assert!(tree.combo_popup_open(combo));
        let popup = tree.combo_popup_rect(combo).expect("popup rect");
        assert_eq!(popup.x, combo_rect.x);
        assert_eq!(popup.y, combo_rect.bottom());
        assert_eq!(popup.height, 3.0 * QCOMBO_POPUP_ROW_H + 2.0);
        // Press highlights the row, release commits it and closes the popup.
        let row_point = QPointF::new(popup.x + 5.0, popup.y + 1.0 + QCOMBO_POPUP_ROW_H * 1.5);
        tree.mouse_press(root, row_point);
        assert_eq!(tree.combo_popup_highlight(combo), Some(1));
        tree.mouse_release(root, row_point);
        assert_eq!(
            tree.property(combo, "currentIndex"),
            Some(&QVariant::Int(1))
        );
        assert!(!tree.combo_popup_open(combo));
    }

    #[test]
    fn combo_popup_closes_on_outside_click_and_click_on_combo() {
        let (mut tree, root, combo) = combos_form();
        let combo_rect = tree.widget(combo).unwrap().geometry;
        tree.mouse_press(root, QPointF::new(combo_rect.x + 5.0, combo_rect.y + 5.0));
        assert!(tree.combo_popup_open(combo));
        tree.mouse_press(root, QPointF::new(250.0, 150.0));
        assert!(!tree.combo_popup_open(combo));

        tree.mouse_press(root, QPointF::new(combo_rect.x + 5.0, combo_rect.y + 5.0));
        assert!(tree.combo_popup_open(combo));
        tree.mouse_press(root, QPointF::new(combo_rect.x + 5.0, combo_rect.y + 5.0));
        assert!(!tree.combo_popup_open(combo));
    }

    #[test]
    fn combo_popup_above_when_no_room_below() {
        let (mut tree, root, combo) = combos_form();
        tree.set_geometry(combo, QRectF::new(10.0, 170.0, 120.0, 25.0));
        tree.mouse_press(root, QPointF::new(15.0, 175.0));
        let popup = tree.combo_popup_rect(combo).expect("popup rect");
        assert!(popup.bottom() <= 170.0);
        assert!(popup.y >= 0.0);
    }

    fn splitter_form(orientation: &str) -> (QWidgetTree, QWidgetId, QWidgetId, QWidgetId) {
        let mut tree = QWidgetTree::new();
        let root = tree.create_widget("QDialog", "Dialog");
        tree.set_geometry(root, QRectF::new(0.0, 0.0, 400.0, 200.0));
        let splitter = tree.create_widget("QSplitter", "splitter");
        tree.add_child(root, splitter);
        tree.set_property(splitter, "orientation", QVariant::Enum(orientation.into()));
        let left = tree.create_widget("QListWidget", "left");
        let right = tree.create_widget("QListWidget", "right");
        tree.add_child(splitter, left);
        tree.add_child(splitter, right);
        tree.set_geometry(splitter, QRectF::new(0.0, 0.0, 400.0, 200.0));
        (tree, root, left, right)
    }

    #[test]
    fn splitter_splits_evenly_and_drags_handles() {
        let (mut tree, root, left, right) = splitter_form("Qt::Orientation::Horizontal");
        assert_eq!(tree.widget(left).unwrap().geometry.width, 198.0);
        assert_eq!(tree.widget(left).unwrap().geometry.x, 0.0);
        assert_eq!(tree.widget(right).unwrap().geometry.x, 202.0);
        assert_eq!(tree.widget(right).unwrap().geometry.width, 198.0);

        let handle = QPointF::new(200.0, 10.0);
        tree.mouse_press(root, handle);
        assert!(tree.mouse_move(QPointF::new(250.0, 10.0)));
        tree.mouse_release(root, QPointF::new(250.0, 10.0));
        let near = |actual: f32, expected: f32| (actual - expected).abs() < 0.01;
        assert!(near(tree.widget(left).unwrap().geometry.width, 248.0));
        assert!(near(tree.widget(right).unwrap().geometry.x, 252.0));
        assert!(near(tree.widget(right).unwrap().geometry.width, 148.0));

        let vertical = splitter_form("Qt::Orientation::Vertical");
        let (mut tree, _root, top, bottom) = vertical;
        tree.set_geometry(
            tree.find_widget("splitter").unwrap(),
            QRectF::new(0.0, 0.0, 400.0, 200.0),
        );
        assert_eq!(tree.widget(top).unwrap().geometry.y, 0.0);
        assert_eq!(tree.widget(top).unwrap().geometry.height, 98.0);
        assert_eq!(tree.widget(bottom).unwrap().geometry.y, 102.0);
        assert_eq!(tree.widget(bottom).unwrap().geometry.height, 98.0);
    }

    #[test]
    fn splitter_sizes_property_sets_pane_widths() {
        let (mut tree, _root, left, right) = splitter_form("Qt::Orientation::Horizontal");
        let splitter = tree.find_widget("splitter").unwrap();
        tree.set_property(
            splitter,
            "sizes",
            QVariant::List(vec![QVariant::Int(80), QVariant::Int(316)]),
        );
        tree.relayout(splitter);
        assert_eq!(tree.widget(left).unwrap().geometry.width, 80.0);
        assert_eq!(tree.widget(right).unwrap().geometry.x, 84.0);
        assert_eq!(tree.widget(right).unwrap().geometry.width, 316.0);
    }

    #[test]
    fn splitter_honors_child_minimum_sizes() {
        let (mut tree, root, left, _right) = splitter_form("Qt::Orientation::Horizontal");
        tree.set_property(left, "minimumSize", QVariant::Size(QSizeF::new(300.0, 0.0)));
        let splitter = tree.find_widget("splitter").unwrap();
        tree.relayout(splitter);
        assert!(tree.widget(left).unwrap().geometry.width >= 300.0);
        let handle_x = tree.widget(left).unwrap().geometry.right() + 1.0;
        tree.mouse_press(root, QPointF::new(handle_x, 10.0));
        tree.mouse_move(QPointF::new(10.0, 10.0));
        tree.mouse_release(root, QPointF::new(10.0, 10.0));
        assert!(tree.widget(left).unwrap().geometry.width >= 300.0);
    }

    #[test]
    fn tree_expand_all_and_collapse_all_properties_rebuild_rows() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let tree_widget = tree.find_widget("treeWidget").expect("tree");
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![QVariant::Map(vec![
                ("text".into(), QVariant::String("Root".into())),
                (
                    "children".into(),
                    QVariant::List(vec![QVariant::Map(vec![(
                        "text".into(),
                        QVariant::String("Child".into()),
                    )])]),
                ),
            ])]),
        );
        tree.relayout(root);
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 1);
        tree.set_property(tree_widget, "expandAll", QVariant::Bool(true));
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 2);
        tree.set_property(tree_widget, "collapseAll", QVariant::Bool(true));
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 1);
    }

    #[test]
    fn tree_branch_indicator_click_toggles_expansion() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let tree_widget = tree.find_widget("treeWidget").expect("tree");
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![QVariant::Map(vec![
                ("text".into(), QVariant::String("Root".into())),
                (
                    "children".into(),
                    QVariant::List(vec![QVariant::Map(vec![(
                        "text".into(),
                        QVariant::String("Child".into()),
                    )])]),
                ),
            ])]),
        );
        tree.relayout(root);
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 1);
        let rect = tree.widget(tree_widget).unwrap().geometry;
        let branch = QPointF::new(rect.x + 6.0, rect.y + QVIEW_HEADER_H + 10.0);
        tree.mouse_press(root, branch);
        tree.mouse_release(root, branch);
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 2);
        tree.mouse_press(root, branch);
        tree.mouse_release(root, branch);
        assert_eq!(tree.tree(tree_widget).unwrap().row_count(), 1);
    }

    #[test]
    fn scrollbar_policy_property_hides_and_forces_bars() {
        const SCROLL_UI: &str = include_str!("../tests/fixtures/scroll_table.ui");
        let (mut tree, root) = load_ui(SCROLL_UI).expect("loads ui");
        let table = tree.find_widget("table").expect("table");
        tree.set_property(
            table,
            "verticalScrollBarPolicy",
            QVariant::Enum("Qt::ScrollBarAlwaysOff".into()),
        );
        tree.set_property(
            table,
            "horizontalScrollBarPolicy",
            QVariant::Enum("Qt::ScrollBarAlwaysOff".into()),
        );
        tree.relayout(root);
        let (geometry, font) = {
            let node = tree.widget(table).unwrap();
            (node.geometry, node.font.clone())
        };
        let layout = tree.table(table).unwrap().scroll_area(geometry, &font);
        assert!(layout.vertical.is_none());
        assert!(layout.horizontal.is_none());
        assert_eq!(
            tree.get_table_property(table, "verticalScrollBarPolicy"),
            Some(QVariant::Enum("Qt::ScrollBarAlwaysOff".into()))
        );

        tree.set_property(
            table,
            "verticalScrollBarPolicy",
            QVariant::Enum("Qt::ScrollBarAlwaysOn".into()),
        );
        tree.set_property(
            table,
            "horizontalScrollBarPolicy",
            QVariant::Enum("Qt::ScrollBarAlwaysOn".into()),
        );
        tree.relayout(root);
        let layout = tree.table(table).unwrap().scroll_area(geometry, &font);
        assert!(layout.vertical.is_some());
        assert!(layout.horizontal.is_some());

        tree.set_property(
            table,
            "verticalScrollBarPolicy",
            QVariant::String("AsNeeded".into()),
        );
        tree.relayout(root);
        assert_eq!(
            tree.get_table_property(table, "verticalScrollBarPolicy"),
            Some(QVariant::Enum("Qt::ScrollBarAsNeeded".into()))
        );
    }

    #[test]
    fn table_scroll_geometry_matches_qt6_offscreen() {
        const SCROLL_UI: &str = include_str!("../tests/fixtures/scroll_table.ui");
        const SCROLL_QT: &str = include_str!("../tests/fixtures/qt6_scroll_table.json");
        let (tree, _) = load_ui(SCROLL_UI).expect("loads ui");
        let table = tree.find_widget("table").expect("table");
        let node = tree.widget(table).unwrap();
        let layout = tree
            .table(table)
            .unwrap()
            .scroll_area(node.geometry, &node.font);
        let reference: serde_json::Value = serde_json::from_str(SCROLL_QT).expect("reference json");
        let area = &reference["scroll_areas"][0];
        let number = |value: &serde_json::Value, key: &str| {
            value[key]
                .as_f64()
                .unwrap_or_else(|| panic!("{key} missing"))
        };
        let near = |context: &str, left: f32, right: f64| {
            assert!(
                (left - right as f32).abs() <= 1.0,
                "{context}: expected {right}, got {left}"
            );
        };
        let viewport = &area["viewport"];
        let body = tree
            .table(table)
            .unwrap()
            .body_rect(node.geometry, &node.font);
        near("viewport.x", body.x, number(viewport, "x"));
        near("viewport.y", body.y, number(viewport, "y"));
        near("viewport.w", body.width, number(viewport, "w"));
        near("viewport.h", body.height, number(viewport, "h"));
        let vbar = layout.vertical.expect("vertical");
        let ref_vbar = &area["vbar"];
        let ref_rect = &ref_vbar["rect"];
        near("vbar.x", vbar.x, number(ref_rect, "x"));
        near("vbar.y", vbar.y, number(ref_rect, "y"));
        near("vbar.w", vbar.width, number(ref_rect, "w"));
        near("vbar.h", vbar.height, number(ref_rect, "h"));
        let table = tree.table(table).unwrap();
        near("vbar.max", table.vbar.maximum, number(ref_vbar, "max"));
        near("vbar.page", table.vbar.page_step, number(ref_vbar, "page"));
        near("vbar.value", table.vbar.value, number(ref_vbar, "value"));
        let hbar = layout.horizontal.expect("horizontal");
        let ref_hbar = &area["hbar"];
        let ref_rect = &ref_hbar["rect"];
        near("hbar.x", hbar.x, number(ref_rect, "x"));
        near("hbar.y", hbar.y, number(ref_rect, "y"));
        near("hbar.w", hbar.width, number(ref_rect, "w"));
        near("hbar.h", hbar.height, number(ref_rect, "h"));
        near("hbar.max", table.hbar.maximum, number(ref_hbar, "max"));
        near("hbar.page", table.hbar.page_step, number(ref_hbar, "page"));
        near("hbar.value", table.hbar.value, number(ref_hbar, "value"));
    }

    #[test]
    fn table_scrollbar_click_scrolls_rows() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let table = tree.find_widget("tableWidget").expect("table");
        tree.set_property(table, "rowCount", QVariant::Int(50));
        tree.set_property(table, "columnCount", QVariant::Int(2));
        tree.relayout(root);
        let rect = tree.widget(table).unwrap().geometry;
        let bar = tree
            .table(table)
            .unwrap()
            .scroll_area(rect, &tree.widget(table).unwrap().font);
        let vertical = bar.vertical.expect("vertical bar");
        let point = QPointF::new(
            vertical.x + vertical.width / 2.0,
            vertical.y + vertical.height - 30.0,
        );
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        assert!(tree.table(table).unwrap().scroll > 0.0);
    }

    #[test]
    fn tree_links_layout_children_to_parent() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let label = tree.find_widget("label_1").expect("label");
        assert!(tree.is_ancestor(root, label));
        assert_eq!(tree.widget(label).unwrap().parent, Some(root));
        assert!(tree.widget(root).unwrap().children.contains(&label));
    }

    #[test]
    fn set_property_icon_slices_img24_sprite() {
        let mut tree = QWidgetTree::new();
        let root = tree.create_widget("QDialog", "Dialog");
        let button = tree.create_widget("QPushButton", "ok");
        tree.add_child(root, button);

        assert!(tree.set_property(button, "icon", QVariant::String("img24:22".into())));
        let icon = tree
            .widget(button)
            .and_then(|widget| widget.icon.as_ref())
            .expect("sliced icon");
        assert_eq!(
            (icon.width, icon.height),
            (crate::icons::ICON_SIZE, crate::icons::ICON_SIZE)
        );
        assert!(tree
            .widget(button)
            .and_then(|widget| widget.icon_disabled.as_ref())
            .is_some());

        assert!(tree.set_property(button, "icon", QVariant::String("img24:99999".into())));
        assert!(tree
            .widget(button)
            .and_then(|widget| widget.icon.as_ref())
            .is_none());
    }

    #[test]
    fn word_wrap_label_wraps_height_and_keeps_narrow_minimum() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>360</width><height>240</height></rect></property>
  <layout class="QVBoxLayout" name="verticalLayout">
   <item>
    <widget class="QLabel" name="label_1">
     <property name="wordWrap"><bool>true</bool></property>
     <property name="text"><string>Это очень длинная подпись, которая переносится по словам, чтобы проверить вертикальные метрики компоновки</string></property>
    </widget>
   </item>
   <item>
    <widget class="QPushButton" name="ok">
     <property name="text"><string>OK</string></property>
    </widget>
   </item>
  </layout>
</widget></ui>"#;
        let (mut tree, root) = load_ui(xml).expect("ui loads");
        let label = tree.find_widget("label_1").expect("label");
        let widget = tree.widget(label).expect("label widget");
        let text = widget.property("text").and_then(variant_text).unwrap_or_default();

        // 1) The minimum follows the longest word, not the full caption:
        //    the window must not widen to the unwrapped text.
        let metrics = QFontMetrics::new(&widget.font);
        let longest = text
            .split_whitespace()
            .map(|word| metrics.horizontal_advance(word))
            .fold(0.0_f32, f32::max);
        let min = tree.minimum_size_hint(root);
        assert!(
            min.width >= longest && min.width < metrics.horizontal_advance(&text),
            "min {} should be [longest {}, full {}]",
            min.width,
            longest,
            metrics.horizontal_advance(&text)
        );

        // 2) In the vertical box the label is allocated the wrapped height
        //    (heightForWidth substitute), so several lines fit.
        let geometry = tree.widget(label).expect("label widget").geometry;
        let lines = wrap_text_lines(&text, 360.0 - 22.0, &metrics).len();
        assert!(lines >= 2, "text must wrap into several lines: {lines}");
        // The label may absorb the box's slack (Preferred grows), but it
        // must be allocated at least the wrapped block, not a single line.
        assert!(
            geometry.height + 1.0 >= lines as f32 * metrics.height(),
            "label height {} < wrapped {}",
            geometry.height,
            lines as f32 * metrics.height()
        );

    }

    #[test]
    fn tab_widget_switches_pages_on_click() {
        const UI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>420</width><height>280</height></rect></property>
  <layout class="QVBoxLayout" name="verticalLayout">
   <item>
    <widget class="QTabWidget" name="tabs">
     <property name="currentIndex"><number>0</number></property>
     <widget class="QWidget" name="pageA">
      <attribute name="title"><string>Первая</string></attribute>
      <layout class="QVBoxLayout" name="layA">
       <item><widget class="QLabel" name="labelA">
        <property name="text"><string>Page A</string></property>
       </widget></item>
      </layout>
     </widget>
     <widget class="QWidget" name="pageB">
      <attribute name="title"><string>Вторая</string></attribute>
      <layout class="QVBoxLayout" name="layB">
       <item><widget class="QLabel" name="labelB">
        <property name="text"><string>Page B</string></property>
       </widget></item>
      </layout>
     </widget>
    </widget>
   </item>
  </layout>
 </widget>
</ui>"#;
        let (mut tree, root) = load_ui(UI).expect("loads ui");
        tree.layout(root, tree.widget(root).unwrap().geometry);
        let tabs = tree.find_widget("tabs").expect("tabs");
        let page_a = tree.find_widget("pageA").expect("pageA");
        let page_b = tree.find_widget("pageB").expect("pageB");
        assert!(tree.widget(page_a).unwrap().visible);
        assert!(!tree.widget(page_b).unwrap().visible);
        assert_eq!(
            tree.widget(tabs).unwrap().tabs.as_ref().unwrap().titles,
            vec!["Первая".to_string(), "Вторая".to_string()]
        );

        let rects = tree.tab_rects(tabs);
        assert_eq!(rects.len(), 2);
        let click = crate::gui::QPointF::new(
            rects[1].x + rects[1].width / 2.0,
            rects[1].y + rects[1].height / 2.0,
        );
        tree.mouse_press(root, click);
        tree.mouse_release(root, click);
        assert!(!tree.widget(page_a).unwrap().visible);
        assert!(tree.widget(page_b).unwrap().visible);
        assert_eq!(
            tree.property(tabs, "currentIndex"),
            Some(&QVariant::Int(1))
        );

        tree.set_property(tabs, "currentIndex", QVariant::Int(0));
        assert!(tree.widget(page_a).unwrap().visible);
        assert!(!tree.widget(page_b).unwrap().visible);
    }
}
