use crate::core::{QEventType, QVariant};
use crate::gui::{QFont, QFontMetrics, QMargins, QPalette, QPixmap, QPointF, QRectF, QSizeF};
use crate::item_model::{QEditTrigger, QSelectionMode, QTableWidget};
use crate::item_view::{QListWidget, QTreeWidget, QVIEW_HEADER_H};
use crate::qss::QStyleSheet;
use crate::scroll::{QScrollAreaLayout, QScrollBar, QScrollBarPolicy, QScrollBarSubControl};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QBoxLayoutDirection {
    #[default]
    LeftToRight,
    TopToBottom,
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
    Widget(QWidgetId),
    Spacer(QSpacerItem),
    Layout(Box<QBoxLayout>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct QBoxLayout {
    pub direction: QBoxLayoutDirection,
    pub spacing: f32,
    pub margins: QMargins,
    pub items: Vec<QLayoutItem>,
}

impl QBoxLayout {
    pub fn new(direction: QBoxLayoutDirection) -> Self {
        Self {
            direction,
            spacing: 6.0,
            margins: QMargins::default(),
            items: Vec::new(),
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.direction == QBoxLayoutDirection::LeftToRight
    }
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
    pub size_policy: QSizePolicy,
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
    pub layout: Option<QBoxLayout>,
    pub table: Option<Box<QTableWidget>>,
    pub list: Option<Box<QListWidget>>,
    pub tree: Option<Box<QTreeWidget>>,
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
        let focus_policy = match class_name.as_str() {
            "QLineEdit" | "QPushButton" | "QTableWidget" | "QListWidget" | "QTreeWidget"
            | "QComboBox" | "QCheckBox" | "QRadioButton" => QFocusPolicy::StrongFocus,
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

    fn size_hint_from_props(&self) -> QSizeF {
        let metrics = QFontMetrics::new(&self.font);
        let text = self.text();
        let text_width = metrics.horizontal_advance(&text);
        let font_height = metrics.height();
        match self.class_name.as_str() {
            "QLabel" => QSizeF::new(
                metrics.bounding_rect(&text).width.ceil(),
                font_height.round(),
            ),
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
                QSizeF::new(width + 24.0, (font_height + 7.0).round())
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
            "QGroupBox" => QSizeF::new(text_width + 16.0, font_height + 16.0),
            _ => QSizeF::new(100.0, font_height + 8.0),
        }
    }
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
        if name == "styleSheet" {
            widget.style_sheet = match &value {
                QVariant::String(text) if !text.trim().is_empty() => Some(QStyleSheet::parse(text)),
                _ => None,
            };
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
        widget.set_property(name, value);
        !is_item_view || handled
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
            let width = 4.0 + metrics.horizontal_advance(&prefix);
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
        if target.has_focus {
            return true;
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
        if !node.visible || !node.enabled {
            return None;
        }
        for child in node.children.iter().rev() {
            if let Some(hit) = self.widget_at_inner(*child, point) {
                return Some(hit);
            }
        }
        if node.geometry.contains(point) {
            Some(widget)
        } else {
            None
        }
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
                        if let Some(row) = tree.row_at(local_y) {
                            tree.set_current(Some(row));
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
        let Some(node) = self.widgets.get_mut(widget) else {
            return QSizeF::default();
        };
        let layout = node.layout.take();
        let hint = if let Some(layout) = &layout {
            self.measure_layout(layout)
        } else {
            self.widgets[widget].size_hint_from_props()
        };
        let node = &mut self.widgets[widget];
        node.size_hint = hint;
        node.layout = layout;
        hint
    }

    fn measure_layout(&mut self, layout: &QBoxLayout) -> QSizeF {
        for item in &layout.items {
            if let QLayoutItem::Widget(widget) = item {
                self.measure(*widget);
            } else if let QLayoutItem::Layout(inner) = item {
                self.measure_layout(inner);
            }
        }
        self.layout_size_hint(layout)
    }

    fn layout_size_hint(&self, layout: &QBoxLayout) -> QSizeF {
        let mut main = 0.0f32;
        let mut perp = 0.0f32;
        for item in &layout.items {
            let size = match item {
                QLayoutItem::Widget(widget) => self.widget_hint(*widget),
                QLayoutItem::Spacer(spacer) => QSizeF::new(spacer.width, spacer.height),
                QLayoutItem::Layout(inner) => self.layout_size_hint(inner),
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

    fn apply_explicit_positions(&mut self, widget: QWidgetId) {
        let Some(parent) = self.widgets.get(widget) else {
            return;
        };
        let parent_origin = (parent.geometry.x, parent.geometry.y);
        let children = parent.children.clone();
        for child in children {
            if let Some(relative) = self.widgets[child].explicit_position {
                self.widgets[child].geometry.x = parent_origin.0 + relative.x;
                self.widgets[child].geometry.y = parent_origin.1 + relative.y;
            }
            self.apply_explicit_positions(child);
        }
    }

    fn layout_widget(&mut self, widget: QWidgetId, rect: QRectF) {
        if let Some(node) = self.widgets.get_mut(widget) {
            node.geometry = rect;
        }
        let Some(node) = self.widgets.get_mut(widget) else {
            return;
        };
        let layout = node.layout.take();
        if let Some(layout) = &layout {
            self.layout_box(layout, rect);
        }
        self.widgets[widget].layout = layout;
    }

    fn layout_box(&mut self, layout: &QBoxLayout, rect: QRectF) {
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
                .filter(|index| matches!(layout.items[*index], QLayoutItem::Spacer(_)))
                .collect();
            let has_widget_expanders = expanders
                .iter()
                .any(|index| !matches!(layout.items[*index], QLayoutItem::Spacer(_)));
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
                    let share = slack / growers.len() as f32;
                    for index in growers {
                        sizes[index] += share;
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
                QLayoutItem::Widget(widget) => self.layout_widget(*widget, slot),
                QLayoutItem::Layout(inner_layout) => self.layout_box(inner_layout, slot),
                QLayoutItem::Spacer(_) => {}
            }
            cursor += main + if main > 0.0 { layout.spacing } else { 0.0 } + gap_extra_value;
        }
    }

    fn item_hint(&self, item: &QLayoutItem) -> QSizeF {
        match item {
            QLayoutItem::Widget(widget) => self.widget_hint(*widget),
            QLayoutItem::Spacer(spacer) => QSizeF::new(spacer.width, spacer.height),
            QLayoutItem::Layout(layout) => self.layout_size_hint(layout),
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
        let QLayoutItem::Widget(widget) = item else {
            return 0.0;
        };
        let Some(widget) = self.widgets.get(*widget) else {
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
            QLayoutItem::Widget(widget) => self.widget_expands(*widget, horizontal),
            QLayoutItem::Spacer(spacer) => {
                if horizontal {
                    spacer.horizontal_policy == QSizePolicy::Expanding
                } else {
                    spacer.vertical_policy == QSizePolicy::Expanding
                }
            }
            QLayoutItem::Layout(layout) => layout
                .items
                .iter()
                .any(|item| self.item_expands(item, horizontal)),
        }
    }

    fn item_can_grow(&self, item: &QLayoutItem, horizontal: bool) -> bool {
        let QLayoutItem::Widget(widget) = item else {
            return false;
        };
        let Some(widget) = self.widgets.get(*widget) else {
            return false;
        };
        if !matches!(
            widget.class_name.as_str(),
            "QLabel" | "QCheckBox" | "QRadioButton" | "QGroupBox" | "QWidget"
        ) {
            return false;
        }
        let (available, hint) = if horizontal {
            (widget.maximum_size.width, widget.size_hint.width)
        } else {
            (widget.maximum_size.height, widget.size_hint.height)
        };
        available > hint + 0.5
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
            widget.size_policy == QSizePolicy::Expanding
                || widget.size_policy == QSizePolicy::MinimumExpanding
                || widget.class_name == "QTableWidget"
                || widget.class_name == "QListWidget"
                || widget.class_name == "QTreeWidget"
        }
    }
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
    id
}

fn load_pixmap(path: &str) -> Option<QPixmap> {
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
) -> QBoxLayout {
    let direction = if spec.class.contains('H') {
        QBoxLayoutDirection::LeftToRight
    } else {
        QBoxLayoutDirection::TopToBottom
    };
    let margins = match spec.margins {
        Some((left, top, right, bottom)) => {
            QMargins::new(left as f32, top as f32, right as f32, bottom as f32)
        }
        None if is_root => QMargins::uniform(11.0),
        None => QMargins::default(),
    };
    let mut layout = QBoxLayout {
        direction,
        spacing: spec.spacing.map(|value| value as f32).unwrap_or(6.0),
        margins,
        items: Vec::new(),
    };
    for item in &spec.items {
        match item {
            form_xml::LayoutItem::Widget(widget) => {
                let id = load_widget(tree, widget, Some(parent), false, false);
                layout.items.push(QLayoutItem::Widget(id));
            }
            form_xml::LayoutItem::Layout(inner) => {
                layout.items.push(QLayoutItem::Layout(Box::new(load_layout(
                    tree, inner, parent, false,
                ))));
            }
            form_xml::LayoutItem::Spacer(spacer) => {
                layout.items.push(QLayoutItem::Spacer(QSpacerItem {
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
                }));
            }
        }
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
