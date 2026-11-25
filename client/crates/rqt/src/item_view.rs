use std::collections::BTreeSet;

use crate::core::QVariant;
use crate::gui::{QFont, QFontMetrics, QSizeF};
use crate::scroll::{
    scroll_area_layout, QOrientation, QScrollAreaLayout, QScrollBar, QSCROLLBAR_EXTENT,
};

pub const QVIEW_ROW_H: f32 = 30.0;
pub const QVIEW_HEADER_H: f32 = 21.0;
pub const QVIEW_INDENT: f32 = 14.0;
pub const QVIEW_BRANCH_W: f32 = 10.0;

fn variant_text(value: &QVariant) -> Option<String> {
    match value {
        QVariant::String(text) | QVariant::Enum(text) => Some(text.clone()),
        QVariant::Int(value) => Some(value.to_string()),
        QVariant::Double(value) => Some(value.to_string()),
        _ => None,
    }
}

fn variant_texts(value: &QVariant) -> Vec<String> {
    match value {
        QVariant::List(items) => items.iter().filter_map(variant_text).collect(),
        _ => Vec::new(),
    }
}

fn map_get<'a>(value: &'a QVariant, key: &str) -> Option<&'a QVariant> {
    match value {
        QVariant::Map(entries) => entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

#[derive(Clone, Debug, Default)]
pub struct QListWidget {
    pub items: Vec<String>,
    pub current: Option<usize>,
    pub selected: BTreeSet<usize>,
    pub scroll: f32,
    pub horizontal: f32,
    pub content_width: f32,
    pub vbar: QScrollBar,
    pub hbar: QScrollBar,
}

impl QListWidget {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current: None,
            selected: BTreeSet::new(),
            scroll: 0.0,
            horizontal: 0.0,
            content_width: 0.0,
            vbar: QScrollBar::new(QOrientation::Vertical),
            hbar: QScrollBar::new(QOrientation::Horizontal),
        }
    }

    pub fn row_count(&self) -> usize {
        self.items.len()
    }

    pub fn content_height(&self) -> f32 {
        self.items.len() as f32 * QVIEW_ROW_H
    }

    pub fn set_items(&mut self, items: Vec<String>, font: &QFont) {
        self.items = items;
        self.prune();
        let metrics = QFontMetrics::new(font);
        self.content_width = self
            .items
            .iter()
            .map(|item| metrics.horizontal_advance(item))
            .fold(0.0_f32, f32::max)
            + 8.0;
    }

    pub fn set_items_variant(&mut self, value: &QVariant, font: &QFont) {
        self.set_items(variant_texts(value), font);
    }

    pub fn set_current(&mut self, row: Option<usize>) {
        self.current = row.filter(|row| *row < self.items.len());
        self.selected = self.current.iter().copied().collect();
    }

    pub fn set_current_index(&mut self, index: i64) -> bool {
        if index < 0 || index as usize >= self.items.len() {
            return false;
        }
        self.set_current(Some(index as usize));
        true
    }

    pub fn current_row_property(&self) -> QVariant {
        QVariant::Int(self.current.map(|row| row as i64).unwrap_or(-1))
    }

    pub fn current_text(&self) -> QVariant {
        QVariant::String(
            self.current
                .and_then(|row| self.items.get(row))
                .cloned()
                .unwrap_or_default(),
        )
    }

    pub fn prune(&mut self) {
        if self.current.is_some_and(|row| row >= self.items.len()) {
            self.current = None;
        }
        self.selected.retain(|row| *row < self.items.len());
    }

    pub fn row_y(&self, row: usize) -> f32 {
        row as f32 * QVIEW_ROW_H - self.scroll
    }

    pub fn row_at(&self, local_y: f32) -> Option<usize> {
        let row = ((local_y + self.scroll) / QVIEW_ROW_H).floor() as usize;
        (row < self.items.len()).then_some(row)
    }

    pub fn scroll_max(&self, view_height: f32) -> f32 {
        (self.content_height() - view_height.max(0.0)).max(0.0)
    }

    pub fn horizontal_max(&self, view_width: f32) -> f32 {
        (self.content_width - view_width.max(0.0)).max(0.0)
    }

    pub fn clamp_scroll(&mut self, view_height: f32) {
        self.scroll = self.scroll.clamp(0.0, self.scroll_max(view_height));
    }

    pub fn clamp_horizontal(&mut self, view_width: f32) {
        self.horizontal = self.horizontal.clamp(0.0, self.horizontal_max(view_width));
    }

    pub fn scroll_by(&mut self, delta: f32, view_height: f32) {
        self.scroll += delta;
        self.clamp_scroll(view_height);
    }

    pub fn scroll_horizontal_by(&mut self, delta: f32, view_width: f32) {
        self.horizontal += delta;
        self.clamp_horizontal(view_width);
    }

    pub fn ensure_visible(&mut self, row: usize, view_height: f32) {
        let top = row as f32 * QVIEW_ROW_H;
        let bottom = top + QVIEW_ROW_H;
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + view_height.max(0.0) {
            self.scroll = bottom - view_height.max(0.0);
        }
        self.clamp_scroll(view_height);
    }

    pub fn content_size(&self) -> QSizeF {
        QSizeF::new(self.content_width, self.content_height())
    }

    pub fn scroll_area(&self, rect: crate::gui::QRectF) -> QScrollAreaLayout {
        scroll_area_layout(
            rect,
            self.content_size(),
            self.vbar.policy,
            self.hbar.policy,
            QSCROLLBAR_EXTENT,
        )
    }

    pub fn sync_bars(&mut self, view: QSizeF) {
        self.sync_vertical_bar(view.height);
        let hmax = self.horizontal_max(view.width);
        self.hbar.set_range(0.0, hmax, view.width.max(0.0));
        self.clamp_horizontal(view.width);
        self.hbar.set_value(self.horizontal);
    }

    pub fn sync_vertical_bar(&mut self, view_height: f32) {
        let visible = ((view_height / QVIEW_ROW_H).floor() as usize)
            .max(1)
            .min(self.items.len().max(1));
        let max_row = self.items.len().saturating_sub(visible);
        self.vbar
            .set_range(0.0, max_row as f32, visible.min(self.items.len()) as f32);
        self.clamp_scroll(view_height);
        self.vbar.set_value(self.scroll / QVIEW_ROW_H);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QTreeItem {
    pub text: String,
    pub columns: Vec<String>,
    pub children: Vec<QTreeItem>,
    /// `QTreeWidgetItem::isExpanded()`. Qt keeps items collapsed when they
    /// are inserted; children become visible only after `setExpanded(true)`.
    pub expanded: bool,
}

impl Default for QTreeItem {
    fn default() -> Self {
        Self {
            text: String::new(),
            columns: Vec::new(),
            children: Vec::new(),
            expanded: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QTreeRow {
    pub path: Vec<usize>,
    pub depth: usize,
    pub text: String,
    pub columns: Vec<String>,
    pub display: String,
    pub has_children: bool,
}

impl QTreeRow {
    pub fn indent(&self) -> f32 {
        self.depth as f32 * QVIEW_INDENT
    }
}

#[derive(Clone, Debug, Default)]
pub struct QTreeWidget {
    pub items: Vec<QTreeItem>,
    pub rows: Vec<QTreeRow>,
    pub headers: Vec<String>,
    pub current: Option<usize>,
    pub selected: BTreeSet<usize>,
    pub scroll: f32,
    pub horizontal: f32,
    pub content_width: f32,
    pub vbar: QScrollBar,
    pub hbar: QScrollBar,
}

impl QTreeWidget {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            rows: Vec::new(),
            headers: Vec::new(),
            current: None,
            selected: BTreeSet::new(),
            scroll: 0.0,
            horizontal: 0.0,
            content_width: 0.0,
            vbar: QScrollBar::new(QOrientation::Vertical),
            hbar: QScrollBar::new(QOrientation::Horizontal),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn content_height(&self) -> f32 {
        self.rows.len() as f32 * QVIEW_ROW_H
    }

    pub fn set_headers(&mut self, headers: Vec<String>, font: &QFont) {
        self.headers = headers;
        self.update_content_width(font);
    }

    pub fn set_headers_variant(&mut self, value: &QVariant, font: &QFont) {
        self.set_headers(variant_texts(value), font);
    }

    pub fn set_data(&mut self, value: &QVariant, font: &QFont) {
        self.items = match value {
            QVariant::List(items) => items.iter().filter_map(parse_tree_item).collect(),
            _ => Vec::new(),
        };
        self.rebuild_rows();
        self.prune();
        self.update_content_width(font);
    }

    pub fn current_row_property(&self) -> QVariant {
        QVariant::Int(self.current.map(|row| row as i64).unwrap_or(-1))
    }

    pub fn current_text(&self) -> QVariant {
        QVariant::String(
            self.current
                .and_then(|row| self.rows.get(row))
                .map(|row| row.text.clone())
                .unwrap_or_default(),
        )
    }

    pub fn set_current(&mut self, row: Option<usize>) {
        self.current = row.filter(|row| *row < self.rows.len());
        self.selected = self.current.iter().copied().collect();
    }

    pub fn set_current_index(&mut self, index: i64) -> bool {
        if index < 0 || index as usize >= self.rows.len() {
            return false;
        }
        self.set_current(Some(index as usize));
        true
    }

    pub fn prune(&mut self) {
        if self.current.is_some_and(|row| row >= self.rows.len()) {
            self.current = None;
        }
        self.selected.retain(|row| *row < self.rows.len());
    }

    pub fn row_y(&self, row: usize) -> f32 {
        row as f32 * QVIEW_ROW_H - self.scroll
    }

    pub fn row_at(&self, local_y: f32) -> Option<usize> {
        let row = ((local_y + self.scroll) / QVIEW_ROW_H).floor() as usize;
        (row < self.rows.len()).then_some(row)
    }

    pub fn scroll_max(&self, view_height: f32) -> f32 {
        (self.content_height() - view_height.max(0.0)).max(0.0)
    }

    pub fn horizontal_max(&self, view_width: f32) -> f32 {
        (self.content_width - view_width.max(0.0)).max(0.0)
    }

    pub fn clamp_scroll(&mut self, view_height: f32) {
        self.scroll = self.scroll.clamp(0.0, self.scroll_max(view_height));
    }

    pub fn clamp_horizontal(&mut self, view_width: f32) {
        self.horizontal = self.horizontal.clamp(0.0, self.horizontal_max(view_width));
    }

    pub fn scroll_by(&mut self, delta: f32, view_height: f32) {
        self.scroll += delta;
        self.clamp_scroll(view_height);
    }

    pub fn scroll_horizontal_by(&mut self, delta: f32, view_width: f32) {
        self.horizontal += delta;
        self.clamp_horizontal(view_width);
    }

    pub fn ensure_visible(&mut self, row: usize, view_height: f32) {
        let top = row as f32 * QVIEW_ROW_H;
        let bottom = top + QVIEW_ROW_H;
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + view_height.max(0.0) {
            self.scroll = bottom - view_height.max(0.0);
        }
        self.clamp_scroll(view_height);
    }

    pub fn content_size(&self) -> QSizeF {
        QSizeF::new(self.content_width, QVIEW_HEADER_H + self.content_height())
    }

    pub fn scroll_area(&self, rect: crate::gui::QRectF) -> QScrollAreaLayout {
        scroll_area_layout(
            rect,
            self.content_size(),
            self.vbar.policy,
            self.hbar.policy,
            QSCROLLBAR_EXTENT,
        )
    }

    pub fn sync_bars(&mut self, view: QSizeF) {
        self.sync_vertical_bar(view.height);
        let hmax = self.horizontal_max(view.width);
        self.hbar.set_range(0.0, hmax, view.width.max(0.0));
        self.clamp_horizontal(view.width);
        self.hbar.set_value(self.horizontal);
    }

    pub fn sync_vertical_bar(&mut self, view_height: f32) {
        let visible = ((view_height / QVIEW_ROW_H).floor() as usize)
            .max(1)
            .min(self.rows.len().max(1));
        let max_row = self.rows.len().saturating_sub(visible);
        self.vbar
            .set_range(0.0, max_row as f32, visible.min(self.rows.len()) as f32);
        self.clamp_scroll(view_height);
        self.vbar.set_value(self.scroll / QVIEW_ROW_H);
    }

    pub fn rebuild_rows(&mut self) {
        let current_path = self
            .current
            .and_then(|row| self.rows.get(row))
            .map(|row| row.path.clone());
        let selected_paths: Vec<Vec<usize>> = self
            .selected
            .iter()
            .filter_map(|row| self.rows.get(*row))
            .map(|row| row.path.clone())
            .collect();
        self.rows.clear();
        for (index, item) in self.items.iter().enumerate() {
            push_tree_row(item, &mut vec![index], 0, &mut self.rows);
        }
        self.current = current_path
            .as_ref()
            .and_then(|path| self.rows.iter().position(|row| &row.path == path))
            .or_else(|| {
                // The current item may now live under a collapsed parent.
                // Qt keeps the item current but hidden; for keyboard and
                // hit-test purposes fall back to the nearest visible row.
                current_path
                    .as_ref()
                    .and_then(|path| self.rows.iter().position(|row| path.starts_with(&row.path)))
            });
        self.selected = selected_paths
            .iter()
            .filter_map(|path| self.rows.iter().position(|row| row.path == *path))
            .collect();
    }

    /// Expand or collapse the item at a flattened row. Returns `false`
    /// when the row has no children or does not exist.
    pub fn set_row_expanded(&mut self, row: usize, expanded: bool) -> bool {
        let Some(path) = self.rows.get(row).map(|row| row.path.clone()) else {
            return false;
        };
        let Some(item) = tree_item_mut(&mut self.items, &path) else {
            return false;
        };
        if item.children.is_empty() || item.expanded == expanded {
            return false;
        }
        item.expanded = expanded;
        self.rebuild_rows();
        true
    }

    pub fn toggle_row_expanded(&mut self, row: usize) -> bool {
        let Some(path) = self.rows.get(row).map(|row| row.path.clone()) else {
            return false;
        };
        let Some(item) = tree_item_mut(&mut self.items, &path) else {
            return false;
        };
        if item.children.is_empty() {
            return false;
        }
        let expanded = !item.expanded;
        self.set_row_expanded(row, expanded)
    }

    pub fn expand_all(&mut self) {
        set_all_expanded(&mut self.items, true);
        self.rebuild_rows();
    }

    pub fn collapse_all(&mut self) {
        set_all_expanded(&mut self.items, false);
        self.rebuild_rows();
    }

    /// Move current to the parent row, as QTreeView does on Left.
    pub fn current_to_parent(&mut self) -> bool {
        let Some(row) = self.current else {
            return false;
        };
        let path = self.rows[row].path.clone();
        if path.len() < 2 {
            return false;
        }
        let parent = &path[..path.len() - 1];
        if let Some(index) = self.rows.iter().position(|row| row.path == parent) {
            self.set_current(Some(index));
            true
        } else {
            false
        }
    }

    /// Move current to the first child, as QTreeView does on Right after
    /// expanding.
    pub fn current_to_first_child(&mut self) -> bool {
        let Some(row) = self.current else {
            return false;
        };
        let path = self.rows[row].path.clone();
        let child_path = {
            let mut path = path;
            path.push(0);
            path
        };
        if let Some(index) = self.rows.iter().position(|row| row.path == child_path) {
            self.set_current(Some(index));
            true
        } else {
            false
        }
    }

    /// `QTreeWidgetItem::isExpanded()` for the flattened current row.
    pub fn current_expanded(&self) -> Option<bool> {
        let row = self.current?;
        let path = &self.rows.get(row)?.path;
        tree_item(&self.items, path).map(|item| item.expanded)
    }

    pub fn current_has_children(&self) -> bool {
        self.current
            .and_then(|row| self.rows.get(row))
            .is_some_and(|row| row.has_children)
    }

    fn update_content_width(&mut self, font: &QFont) {
        let metrics = QFontMetrics::new(font);
        let headers_width = if self.headers.is_empty() {
            0.0
        } else {
            metrics.horizontal_advance(&self.headers.join("  ")) + 8.0
        };
        let rows_width = self
            .rows
            .iter()
            .map(|row| {
                row.indent() + QVIEW_BRANCH_W + metrics.horizontal_advance(&row.display) + 8.0
            })
            .fold(0.0_f32, f32::max);
        self.content_width = headers_width.max(rows_width);
    }
}

fn parse_tree_item(value: &QVariant) -> Option<QTreeItem> {
    let text = map_get(value, "text")
        .and_then(variant_text)
        .unwrap_or_default();
    let columns = map_get(value, "columns")
        .map(variant_texts)
        .unwrap_or_default();
    let children = map_get(value, "children")
        .map(|value| match value {
            QVariant::List(items) => items.iter().filter_map(parse_tree_item).collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default();
    let expanded = matches!(map_get(value, "expanded"), Some(QVariant::Bool(true)));
    Some(QTreeItem {
        text,
        columns,
        children,
        expanded,
    })
}

fn push_tree_row(item: &QTreeItem, path: &mut Vec<usize>, depth: usize, out: &mut Vec<QTreeRow>) {
    let mut display = item.text.clone();
    for column in &item.columns {
        if !column.is_empty() {
            if !display.is_empty() {
                display.push_str("  ");
            }
            display.push_str(column);
        }
    }
    out.push(QTreeRow {
        path: path.clone(),
        depth,
        text: item.text.clone(),
        columns: item.columns.clone(),
        display,
        has_children: !item.children.is_empty(),
    });
    if !item.expanded {
        return;
    }
    for (index, child) in item.children.iter().enumerate() {
        path.push(index);
        push_tree_row(child, path, depth + 1, out);
        path.pop();
    }
}

fn tree_item<'a>(items: &'a [QTreeItem], path: &[usize]) -> Option<&'a QTreeItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        tree_item(&item.children, rest)
    }
}

fn tree_item_mut<'a>(items: &'a mut [QTreeItem], path: &[usize]) -> Option<&'a mut QTreeItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get_mut(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        tree_item_mut(&mut item.children, rest)
    }
}

fn set_all_expanded(items: &mut [QTreeItem], expanded: bool) {
    for item in items {
        item.expanded = expanded;
        set_all_expanded(&mut item.children, expanded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: Vec<(&str, QVariant)>) -> QVariant {
        QVariant::Map(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn tree_data() -> QVariant {
        QVariant::List(vec![
            map(vec![
                ("text", QVariant::String("Root 1".into())),
                (
                    "columns",
                    QVariant::List(vec![QVariant::String("Root Column".into())]),
                ),
                (
                    "children",
                    QVariant::List(vec![
                        map(vec![
                            ("text", QVariant::String("Child 1.1".into())),
                            (
                                "columns",
                                QVariant::List(vec![QVariant::String("Data 1.1".into())]),
                            ),
                        ]),
                        map(vec![
                            ("text", QVariant::String("Child 1.2".into())),
                            (
                                "columns",
                                QVariant::List(vec![QVariant::String("Data 1.2".into())]),
                            ),
                        ]),
                    ]),
                ),
            ]),
            map(vec![
                ("text", QVariant::String("Root 2".into())),
                (
                    "children",
                    QVariant::List(vec![map(vec![
                        ("text", QVariant::String("Child 2.1".into())),
                        (
                            "columns",
                            QVariant::List(vec![QVariant::String("Data 2.1".into())]),
                        ),
                    ])]),
                ),
            ]),
        ])
    }

    #[test]
    fn list_items_current_and_scroll() {
        let font = QFont::default();
        let mut list = QListWidget::new();
        list.set_items(
            vec!["one".into(), "two".into(), "three".into(), "four".into()],
            &font,
        );
        assert_eq!(list.row_count(), 4);
        assert!(list.content_width > 0.0);
        list.set_current_index(2);
        assert_eq!(list.current_row_property(), QVariant::Int(2));
        assert_eq!(list.current_text(), QVariant::String("three".into()));
        list.sync_bars(QSizeF::new(100.0, 60.0));
        assert!(list.vbar.is_needed());
        list.ensure_visible(3, 60.0);
        assert_eq!(list.scroll, 60.0);
        list.scroll_by(-100.0, 60.0);
        assert_eq!(list.scroll, 0.0);
    }

    #[test]
    fn list_horizontal_scroll_is_clamped() {
        let font = QFont::default();
        let mut list = QListWidget::new();
        list.set_items(vec!["short".into(), "a very long list item".into()], &font);
        list.sync_bars(QSizeF::new(80.0, 100.0));
        assert!(list.hbar.is_needed());
        list.scroll_horizontal_by(10_000.0, 80.0);
        assert_eq!(list.horizontal, list.horizontal_max(80.0));
        list.scroll_horizontal_by(-10_000.0, 80.0);
        assert_eq!(list.horizontal, 0.0);
    }

    #[test]
    fn tree_flattens_children_and_headers() {
        let font = QFont::default();
        let mut tree = QTreeWidget::new();
        tree.set_headers(vec!["Column 1".into(), "Column 2".into()], &font);
        tree.set_data(&tree_data(), &font);
        // Qt inserts items collapsed; expanding a parent reveals its children.
        assert_eq!(tree.row_count(), 2);
        assert_eq!(tree.rows[0].text, "Root 1");
        assert_eq!(tree.rows[1].text, "Root 2");
        assert!(tree.toggle_row_expanded(0));
        assert!(tree.toggle_row_expanded(3));
        assert_eq!(tree.row_count(), 5);
        assert_eq!(tree.rows[0].text, "Root 1");
        assert_eq!(tree.rows[0].depth, 0);
        assert_eq!(tree.rows[1].text, "Child 1.1");
        assert_eq!(tree.rows[1].depth, 1);
        assert_eq!(tree.rows[2].text, "Child 1.2");
        assert_eq!(tree.rows[3].text, "Root 2");
        assert_eq!(tree.rows[4].text, "Child 2.1");
        assert_eq!(tree.rows[0].display, "Root 1  Root Column");
        assert!(tree.rows[0].has_children);
        assert!(!tree.rows[1].has_children);
        assert!(tree.content_width > 0.0);
    }

    #[test]
    fn tree_current_prune_and_scroll() {
        let font = QFont::default();
        let mut tree = QTreeWidget::new();
        tree.set_data(&tree_data(), &font);
        tree.expand_all();
        tree.set_current_index(4);
        assert_eq!(tree.current_text(), QVariant::String("Child 2.1".into()));
        tree.sync_bars(QSizeF::new(100.0, 60.0));
        tree.ensure_visible(4, 60.0);
        assert!(tree.scroll > 0.0);
        tree.set_data(&QVariant::List(Vec::new()), &font);
        assert_eq!(tree.current, None);
        assert!(tree.selected.is_empty());
    }

    #[test]
    fn tree_collapse_keeps_current_on_the_visible_parent() {
        let font = QFont::default();
        let mut tree = QTreeWidget::new();
        tree.set_data(&tree_data(), &font);
        tree.expand_all();
        tree.set_current_index(2);
        assert_eq!(tree.current_text(), QVariant::String("Child 1.2".into()));
        assert!(tree.current_has_children() == false);
        // Collapse "Root 1" from its child row: the current item becomes the
        // nearest visible ancestor instead of an invalid row index.
        assert!(tree.set_row_expanded(0, false));
        // Root 2 is still expanded, so only Root 1's children disappear.
        assert_eq!(tree.row_count(), 3);
        assert_eq!(tree.current_text(), QVariant::String("Root 1".into()));
        assert_eq!(tree.current, Some(0));
        assert!(!tree.current_expanded().unwrap());
    }

    #[test]
    fn scroll_policy_controls_visibility() {
        use crate::gui::QRectF;
        use crate::scroll::QScrollBarPolicy;

        let font = QFont::default();
        let mut list = QListWidget::new();
        list.set_items((0..50).map(|i| i.to_string()).collect(), &font);
        let rect = QRectF::new(0.0, 0.0, 200.0, 100.0);
        assert!(list.scroll_area(rect).vertical.is_some());
        list.vbar.policy = QScrollBarPolicy::AlwaysOff;
        let layout = list.scroll_area(rect);
        assert!(layout.vertical.is_none());
        assert_eq!(layout.viewport, rect);
        list.vbar.policy = QScrollBarPolicy::AlwaysOn;
        assert!(list.scroll_area(rect).vertical.is_some());

        let mut tree = QTreeWidget::new();
        tree.set_data(&tree_data(), &font);
        tree.hbar.policy = QScrollBarPolicy::AlwaysOn;
        let layout = tree.scroll_area(rect);
        assert!(layout.horizontal.is_some());
        assert!(layout.viewport.height < rect.height);
    }

    #[test]
    fn row_at_respects_scroll() {
        let font = QFont::default();
        let mut list = QListWidget::new();
        list.set_items((0..10).map(|i| i.to_string()).collect(), &font);
        list.scroll = QVIEW_ROW_H * 2.0;
        assert_eq!(list.row_at(0.0), Some(2));
        assert_eq!(list.row_at(QVIEW_ROW_H + 1.0), Some(3));
    }
}
