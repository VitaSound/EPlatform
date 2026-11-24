use std::collections::{BTreeSet, HashMap, HashSet};

use crate::core::QVariant;
use crate::gui::{QFont, QFontMetrics, QFontWeight};

pub const QTABLE_HEADER_H: f32 = 21.0;
pub const QTABLE_ROW_H: f32 = 30.0;
pub const QTABLE_DEFAULT_COL_W: f32 = 100.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QSelectionMode {
    #[default]
    SelectItems,
    SelectRows,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QEditTrigger {
    #[default]
    NoEditTriggers,
    AnyKeyPressed,
    DoubleClicked,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QSortOrder {
    #[default]
    Ascending,
    Descending,
}

pub trait QAbstractItemModel {
    fn row_count(&self) -> usize;
    fn column_count(&self) -> usize;
    fn data(&self, row: usize, column: usize) -> Option<&str>;
    fn set_data(&mut self, row: usize, column: usize, text: String) -> bool;
    fn header_data(&self, column: usize) -> Option<&str>;
    fn set_header_data(&mut self, column: usize, text: String) -> bool;
}

pub trait QAbstractItemDelegate {
    fn editor_text(&self, model: &dyn QAbstractItemModel, row: usize, column: usize) -> String {
        model.data(row, column).unwrap_or("").to_string()
    }

    fn commit_text(
        &self,
        model: &mut dyn QAbstractItemModel,
        row: usize,
        column: usize,
        text: String,
    ) -> bool {
        model.set_data(row, column, text)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct QStyledItemDelegate;

impl QAbstractItemDelegate for QStyledItemDelegate {}

#[derive(Clone, Debug, Default)]
pub struct QTableModel {
    row_count: usize,
    column_count: usize,
    headers: Vec<String>,
    cells: HashMap<(usize, usize), String>,
    widths: Vec<f32>,
    hidden: HashSet<usize>,
}

impl QTableModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn column_count(&self) -> usize {
        self.column_count
    }

    pub fn set_row_count(&mut self, count: usize) {
        self.row_count = count;
        self.cells.retain(|(row, _), _| *row < count);
    }

    pub fn set_column_count(&mut self, count: usize) {
        self.column_count = count;
        self.headers.resize(count, String::new());
        self.widths.resize(count, QTABLE_DEFAULT_COL_W);
        self.hidden.retain(|column| *column < count);
        self.cells.retain(|(_, column), _| *column < count);
    }

    pub fn header(&self, column: usize) -> Option<&str> {
        self.headers.get(column).map(String::as_str)
    }

    pub fn set_header(&mut self, column: usize, text: impl Into<String>) {
        if column >= self.column_count {
            self.set_column_count(column + 1);
        }
        self.headers[column] = text.into();
    }

    pub fn cell(&self, row: usize, column: usize) -> Option<&str> {
        self.cells.get(&(row, column)).map(String::as_str)
    }

    pub fn cells(&self) -> impl Iterator<Item = ((usize, usize), &str)> {
        self.cells
            .iter()
            .map(|(position, text)| (*position, text.as_str()))
    }

    pub fn set_cell(&mut self, row: usize, column: usize, text: impl Into<String>) {
        if row >= self.row_count {
            self.set_row_count(row + 1);
        }
        if column >= self.column_count {
            self.set_column_count(column + 1);
        }
        let text = text.into();
        if text.is_empty() {
            self.cells.remove(&(row, column));
        } else {
            self.cells.insert((row, column), text);
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        for header in &mut self.headers {
            header.clear();
        }
    }

    pub fn column_width(&self, column: usize) -> f32 {
        self.widths
            .get(column)
            .copied()
            .unwrap_or(QTABLE_DEFAULT_COL_W)
    }

    pub fn set_column_width(&mut self, column: usize, width: f32) {
        if column >= self.column_count {
            self.set_column_count(column + 1);
        }
        self.widths[column] = width.max(1.0);
    }

    pub fn is_column_hidden(&self, column: usize) -> bool {
        self.hidden.contains(&column)
    }

    pub fn set_column_hidden(&mut self, column: usize, hidden: bool) {
        if hidden {
            self.hidden.insert(column);
        } else {
            self.hidden.remove(&column);
        }
    }

    pub fn column_offset(&self, column: usize) -> f32 {
        (0..column)
            .filter(|index| !self.is_column_hidden(*index))
            .map(|index| self.column_width(index))
            .sum()
    }

    pub fn column_at(&self, x: f32) -> Option<usize> {
        if x < 0.0 {
            return None;
        }
        let mut left = 0.0;
        for column in 0..self.column_count {
            if self.is_column_hidden(column) {
                continue;
            }
            let width = self.column_width(column);
            if x >= left && x < left + width {
                return Some(column);
            }
            left += width;
        }
        None
    }

    pub fn apply_headers(&mut self, config: &QVariant) {
        if let Some(QVariant::Bool(true)) = map_get(config, "clear") {
            self.clear();
        }
        if let Some(count) = map_get(config, "columnCount").and_then(QVariant::as_i64) {
            self.set_column_count(count.max(0) as usize);
        }
        if let Some(labels) = map_get(config, "horizontalHeaderLabels") {
            self.set_headers(labels);
        }
        if let Some(widths) = map_get(config, "columnWidths") {
            self.set_widths(widths);
        }
        if let Some(hidden) = map_get(config, "hiddenColumns") {
            match hidden {
                QVariant::List(items) => {
                    for item in items {
                        if let Some(column) = one_based(item) {
                            self.set_column_hidden(column, true);
                        }
                    }
                }
                QVariant::Map(entries) => {
                    for (key, value) in entries {
                        if let Ok(column) = key.parse::<usize>() {
                            if column >= 1 {
                                self.set_column_hidden(
                                    column - 1,
                                    value.as_bool().unwrap_or(false),
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn apply_data(&mut self, config: &QVariant) {
        let count = map_get(config, "rowCount")
            .and_then(QVariant::as_i64)
            .unwrap_or(0);
        self.set_row_count(count.max(0) as usize);
        let Some(data) = map_get(config, "data") else {
            return;
        };
        match data {
            QVariant::Map(rows) => {
                for (key, cells) in rows {
                    if let Ok(row) = key.parse::<usize>() {
                        if row >= 1 {
                            self.apply_row(row - 1, cells);
                        }
                    }
                }
            }
            QVariant::List(rows) => {
                for (row, cells) in rows.iter().enumerate() {
                    self.apply_row(row, cells);
                }
            }
            _ => {}
        }
    }

    fn set_headers(&mut self, labels: &QVariant) {
        let Some(items) = variant_list(labels) else {
            return;
        };
        if items.len() > self.column_count {
            self.set_column_count(items.len());
        }
        for (column, label) in items.iter().enumerate() {
            if let Some(text) = variant_text(label) {
                self.headers[column] = text;
            }
        }
    }

    fn set_widths(&mut self, widths: &QVariant) {
        let Some(items) = variant_list(widths) else {
            return;
        };
        if items.len() > self.column_count {
            self.set_column_count(items.len());
        }
        for (column, width) in items.iter().enumerate() {
            if let Some(width) = width.as_i64() {
                self.widths[column] = width.max(1) as f32;
            }
        }
    }

    fn apply_row(&mut self, row: usize, cells: &QVariant) {
        match cells {
            QVariant::Map(columns) => {
                for (key, value) in columns {
                    if let Ok(column) = key.parse::<usize>() {
                        if column >= 1 {
                            if let Some(text) = variant_text(value) {
                                self.set_cell(row, column - 1, text);
                            }
                        }
                    }
                }
            }
            QVariant::List(columns) => {
                for (column, value) in columns.iter().enumerate() {
                    if let Some(text) = variant_text(value) {
                        self.set_cell(row, column, text);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn hide_column(&mut self, config: &QVariant) {
        let Some(index) = map_get(config, "index").and_then(one_based) else {
            return;
        };
        let hidden = map_get(config, "hidden")
            .and_then(QVariant::as_bool)
            .unwrap_or(true);
        self.set_column_hidden(index, hidden);
    }
}

impl QAbstractItemModel for QTableModel {
    fn row_count(&self) -> usize {
        self.row_count()
    }

    fn column_count(&self) -> usize {
        self.column_count()
    }

    fn data(&self, row: usize, column: usize) -> Option<&str> {
        self.cell(row, column)
    }

    fn set_data(&mut self, row: usize, column: usize, text: String) -> bool {
        self.set_cell(row, column, text);
        true
    }

    fn header_data(&self, column: usize) -> Option<&str> {
        self.header(column)
    }

    fn set_header_data(&mut self, column: usize, text: String) -> bool {
        self.set_header(column, text);
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QTableWidgetEditor {
    pub row: usize,
    pub column: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct QItemSelectionModel {
    current: Option<(usize, usize)>,
    selected: BTreeSet<(usize, usize)>,
}

impl QItemSelectionModel {
    pub fn current(&self) -> Option<(usize, usize)> {
        self.current
    }

    pub fn current_row(&self) -> Option<usize> {
        self.current.map(|(row, _)| row)
    }

    pub fn current_column(&self) -> Option<usize> {
        self.current.map(|(_, column)| column)
    }

    pub fn set_current(&mut self, row: usize, column: usize) {
        self.current = Some((row, column));
    }

    pub fn clear_current(&mut self) {
        self.current = None;
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn select(&mut self, row: usize, column: usize) {
        self.selected.insert((row, column));
    }

    pub fn deselect(&mut self, row: usize, column: usize) {
        self.selected.remove(&(row, column));
    }

    pub fn is_selected(&self, row: usize, column: usize) -> bool {
        self.selected.contains(&(row, column))
    }

    pub fn selected_rows(&self) -> BTreeSet<usize> {
        self.selected.iter().map(|(row, _)| *row).collect()
    }

    pub fn selected_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.selected.iter().copied()
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.selected.clear();
    }

    pub fn prune(&mut self, row_count: usize, column_count: usize) {
        if self
            .current
            .is_some_and(|(row, column)| row >= row_count || column >= column_count)
        {
            self.current = None;
        }
        self.selected
            .retain(|(row, column)| *row < row_count && *column < column_count);
    }
}

#[derive(Clone, Debug)]
pub struct QTableWidget {
    pub model: QTableModel,
    pub selection: QItemSelectionModel,
    pub selection_mode: QSelectionMode,
    pub edit_trigger: QEditTrigger,
    pub read_only: bool,
    pub scroll: f32,
    pub editor: Option<QTableWidgetEditor>,
    pub sort_indicator: Option<(usize, QSortOrder)>,
}

impl Default for QTableWidget {
    fn default() -> Self {
        Self {
            model: QTableModel::new(),
            selection: QItemSelectionModel::default(),
            selection_mode: QSelectionMode::SelectItems,
            edit_trigger: QEditTrigger::NoEditTriggers,
            read_only: false,
            scroll: 0.0,
            editor: None,
            sort_indicator: None,
        }
    }
}

impl QTableWidget {
    pub fn row_header_width(&self, font: &QFont) -> f32 {
        let mut header_font = font.clone();
        header_font.weight = QFontWeight::Bold;
        let metrics = QFontMetrics::new(&header_font);
        let label = self.model.row_count().max(1).to_string();
        (metrics.horizontal_advance(&label) + 4.0).ceil().max(14.0)
    }

    pub fn current(&self) -> Option<(usize, usize)> {
        self.selection.current()
    }

    pub fn current_row(&self) -> Option<usize> {
        self.selection.current_row()
    }

    pub fn current_column(&self) -> Option<usize> {
        self.selection.current_column()
    }

    pub fn is_selected(&self, row: usize, column: usize) -> bool {
        self.selection.is_selected(row, column)
    }

    pub fn clear(&mut self) {
        self.model.clear();
        self.selection.clear();
    }

    pub fn set_current_cell(&mut self, row: usize, column: usize) {
        if row >= self.model.row_count() || column >= self.model.column_count() {
            return;
        }
        let column = if self.selection_mode == QSelectionMode::SelectRows {
            0
        } else {
            column
        };
        self.selection.set_current(row, column);
    }

    pub fn select_current(&mut self) {
        let Some((row, column)) = self.current() else {
            return;
        };
        self.selection.clear_selection();
        if self.selection_mode == QSelectionMode::SelectRows {
            for cell_column in 0..self.model.column_count() {
                self.selection.select(row, cell_column);
            }
        } else {
            self.selection.select(row, column);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear_selection();
    }

    pub fn set_current_row_1based(&mut self, row: i64) -> bool {
        if row < 1 {
            self.selection.clear_current();
            return true;
        }
        let row = (row - 1) as usize;
        if row >= self.model.row_count() {
            return false;
        }
        self.selection.set_current(row, 0);
        true
    }

    pub fn current_row_property(&self) -> QVariant {
        QVariant::Int(self.current_row().map(|row| row as i64).unwrap_or(-1))
    }

    pub fn current_row_cell(&self, column_1based: i64) -> Option<QVariant> {
        let row = self.current_row()?;
        if column_1based < 1 {
            return None;
        }
        let column = (column_1based - 1) as usize;
        self.model
            .cell(row, column)
            .map(|text| QVariant::String(text.to_string()))
    }

    pub fn ensure_current(&mut self) {
        if self.current().is_none() && self.model.row_count() > 0 && self.model.column_count() > 0 {
            self.selection.set_current(0, 0);
        }
    }

    pub fn begin_edit(&mut self, row: usize, column: usize) -> bool {
        if self.read_only || row >= self.model.row_count() || column >= self.model.column_count() {
            return false;
        }
        let text = self.model.cell(row, column).unwrap_or("").to_string();
        self.editor = Some(QTableWidgetEditor { row, column, text });
        true
    }

    pub fn begin_edit_current(&mut self) -> bool {
        let Some((row, column)) = self.current() else {
            return false;
        };
        self.begin_edit(row, column)
    }

    pub fn is_editing(&self) -> bool {
        self.editor.is_some()
    }

    pub fn editor_insert(&mut self, text: &str) {
        if let Some(editor) = self.editor.as_mut() {
            editor.text.push_str(text);
        }
    }

    pub fn editor_backspace(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.text.pop();
        }
    }

    pub fn commit_editor(&mut self) -> Option<QTableWidgetEditor> {
        let editor = self.editor.take()?;
        self.model
            .set_cell(editor.row, editor.column, editor.text.clone());
        Some(editor)
    }

    pub fn cancel_editor(&mut self) {
        self.editor = None;
    }

    pub fn column_resize_handle_at(&self, x: f32, row_header_width: f32) -> Option<usize> {
        let mut edge = row_header_width;
        for column in 0..self.model.column_count() {
            if self.model.is_column_hidden(column) {
                continue;
            }
            edge += self.model.column_width(column);
            if (x - edge).abs() <= 3.0 {
                return Some(column);
            }
        }
        None
    }

    pub fn resize_column(&mut self, column: usize, width: f32) {
        self.model.set_column_width(column, width);
    }

    pub fn toggle_sort(&mut self, column: usize) {
        let order = match self.sort_indicator {
            Some((active, QSortOrder::Ascending)) if active == column => QSortOrder::Descending,
            Some((active, QSortOrder::Descending)) if active == column => QSortOrder::Ascending,
            _ => QSortOrder::Ascending,
        };
        self.sort_indicator = Some((column, order));
        self.sort_by_column(column, order);
    }

    pub fn sort_by_column(&mut self, column: usize, order: QSortOrder) {
        if column >= self.model.column_count() {
            return;
        }
        let mut rows: Vec<usize> = (0..self.model.row_count()).collect();
        rows.sort_by(|left, right| {
            let left_text = self.model.cell(*left, column).unwrap_or("");
            let right_text = self.model.cell(*right, column).unwrap_or("");
            let ordering = match (left_text.parse::<f64>(), right_text.parse::<f64>()) {
                (Ok(left), Ok(right)) => left
                    .partial_cmp(&right)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => left_text.cmp(right_text),
            };
            if order == QSortOrder::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        let mut cells = HashMap::new();
        for (new_row, old_row) in rows.into_iter().enumerate() {
            for cell_column in 0..self.model.column_count() {
                if let Some(text) = self.model.cell(old_row, cell_column) {
                    cells.insert((new_row, cell_column), text.to_string());
                }
            }
        }
        self.model.cells = cells;
        self.selection.clear();
        self.editor = None;
    }

    pub fn row_at(&self, y: f32) -> Option<usize> {
        if y < QTABLE_HEADER_H {
            return None;
        }
        let row = ((y - QTABLE_HEADER_H + self.scroll) / QTABLE_ROW_H).floor() as usize;
        (row < self.model.row_count()).then_some(row)
    }

    pub fn visible_row_range(&self, view_height: f32) -> std::ops::Range<usize> {
        let body_height = (view_height - QTABLE_HEADER_H).max(0.0);
        if body_height <= 0.0 || self.model.row_count() == 0 {
            return 0..0;
        }
        let first = (self.scroll / QTABLE_ROW_H).floor() as usize;
        let visible = (body_height / QTABLE_ROW_H).ceil() as usize;
        let last = (first + visible + 1).min(self.model.row_count());
        first.min(self.model.row_count())..last
    }

    pub fn row_y(&self, row: usize) -> f32 {
        QTABLE_HEADER_H + row as f32 * QTABLE_ROW_H - self.scroll
    }

    pub fn scroll_max(&self, view_height: f32) -> f32 {
        let content = self.model.row_count() as f32 * QTABLE_ROW_H;
        (content - (view_height - QTABLE_HEADER_H).max(0.0)).max(0.0)
    }

    pub fn clamp_scroll(&mut self, view_height: f32) {
        self.scroll = self.scroll.clamp(0.0, self.scroll_max(view_height));
    }

    pub fn scroll_by(&mut self, delta: f32, view_height: f32) {
        self.scroll += delta;
        self.clamp_scroll(view_height);
    }

    pub fn reveal_current_row(&mut self, view_height: f32) {
        let Some(row) = self.current_row() else {
            return;
        };
        let top = row as f32 * QTABLE_ROW_H;
        let bottom = top + QTABLE_ROW_H;
        let body = (view_height - QTABLE_HEADER_H).max(0.0);
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + body {
            self.scroll = bottom - body;
        }
        self.clamp_scroll(view_height);
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

fn variant_list(value: &QVariant) -> Option<&[QVariant]> {
    match value {
        QVariant::List(items) => Some(items),
        _ => None,
    }
}

fn variant_text(value: &QVariant) -> Option<String> {
    match value {
        QVariant::String(text) | QVariant::Enum(text) => Some(text.clone()),
        QVariant::Int(value) => Some(value.to_string()),
        QVariant::Double(value) => Some(value.to_string()),
        QVariant::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn one_based(value: &QVariant) -> Option<usize> {
    value
        .as_i64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value >= 1)
        .map(|value| value - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_model_applies_headers_and_data() {
        let mut model = QTableModel::new();
        model.apply_headers(&QVariant::Map(vec![
            ("columnCount".into(), QVariant::Int(2)),
            (
                "horizontalHeaderLabels".into(),
                QVariant::List(vec![
                    QVariant::String("Col 1".into()),
                    QVariant::String("Col 2".into()),
                ]),
            ),
            (
                "columnWidths".into(),
                QVariant::List(vec![QVariant::Int(100), QVariant::Int(120)]),
            ),
        ]));
        model.apply_data(&QVariant::Map(vec![
            ("rowCount".into(), QVariant::Int(2)),
            (
                "data".into(),
                QVariant::List(vec![
                    QVariant::List(vec![
                        QVariant::String("A".into()),
                        QVariant::String("B".into()),
                    ]),
                    QVariant::List(vec![QVariant::Int(1), QVariant::Int(2)]),
                ]),
            ),
        ]));

        assert_eq!(model.row_count(), 2);
        assert_eq!(model.column_count(), 2);
        assert_eq!(model.header(0), Some("Col 1"));
        assert_eq!(model.cell(0, 1), Some("B"));
        assert_eq!(model.cell(1, 0), Some("1"));
        assert_eq!(model.column_width(1), 120.0);
    }

    #[test]
    fn table_editor_uses_delegate_semantics() {
        let mut table = QTableWidget::default();
        table.model.set_row_count(1);
        table.model.set_column_count(1);
        table.model.set_cell(0, 0, "old");
        table.set_current_cell(0, 0);
        assert!(table.begin_edit_current());
        table.editor_insert("X");
        table.editor_backspace();
        table.editor_insert("!");
        let editor = table.commit_editor().expect("committed");
        assert_eq!(editor.text, "old!");
        assert_eq!(table.model.cell(0, 0), Some("old!"));

        let delegate = QStyledItemDelegate;
        assert_eq!(delegate.editor_text(&table.model, 0, 0), "old!");
        assert!(delegate.commit_text(&mut table.model, 0, 0, "new".into()));
        assert_eq!(table.model.cell(0, 0), Some("new"));
    }

    #[test]
    fn table_sorts_rows_by_column() {
        let mut table = QTableWidget::default();
        table.model.set_row_count(3);
        table.model.set_column_count(2);
        table.model.set_cell(0, 0, "3");
        table.model.set_cell(1, 0, "1");
        table.model.set_cell(2, 0, "2");
        table.model.set_cell(0, 1, "c");
        table.model.set_cell(1, 1, "a");
        table.model.set_cell(2, 1, "b");

        table.sort_by_column(0, QSortOrder::Ascending);
        assert_eq!(table.model.cell(0, 0), Some("1"));
        assert_eq!(table.model.cell(1, 0), Some("2"));
        assert_eq!(table.model.cell(2, 0), Some("3"));
        assert_eq!(table.model.cell(0, 1), Some("a"));

        table.sort_by_column(0, QSortOrder::Descending);
        assert_eq!(table.model.cell(0, 0), Some("3"));
        assert_eq!(table.model.cell(2, 0), Some("1"));

        table.toggle_sort(1);
        assert_eq!(table.sort_indicator, Some((1, QSortOrder::Ascending)));
        table.toggle_sort(1);
        assert_eq!(table.sort_indicator, Some((1, QSortOrder::Descending)));
    }

    #[test]
    fn current_and_selection_are_separate_like_qt() {
        let mut table = QTableWidget::default();
        table.model.set_row_count(2);
        table.model.set_column_count(2);

        table.ensure_current();
        assert_eq!(table.current(), Some((0, 0)));
        assert!(!table.is_selected(0, 0));

        table.select_current();
        assert!(table.is_selected(0, 0));

        table.clear_selection();
        table.set_current_cell(1, 1);
        assert_eq!(table.current(), Some((1, 1)));
        assert!(!table.is_selected(1, 1));

        table.selection_mode = QSelectionMode::SelectRows;
        table.select_current();
        assert!(table.is_selected(1, 0));
        assert!(table.is_selected(1, 1));
    }

    #[test]
    fn row_header_width_adapts_to_row_numbers() {
        let mut table = QTableWidget::default();
        let font = QFont::default();
        table.model.set_row_count(5);
        assert_eq!(table.row_header_width(&font), 14.0);
        table.model.set_row_count(10);
        assert_eq!(table.row_header_width(&font), 24.0);
        table.model.set_row_count(100);
        assert_eq!(table.row_header_width(&font), 34.0);
    }

    #[test]
    fn header_resize_handles_visible_column_edges() {
        let mut table = QTableWidget::default();
        table.model.set_column_count(2);
        table.model.set_column_width(0, 50.0);
        table.model.set_column_width(1, 70.0);
        assert_eq!(table.column_resize_handle_at(79.0, 30.0), Some(0));
        assert_eq!(table.column_resize_handle_at(149.0, 30.0), Some(1));
        assert_eq!(table.column_resize_handle_at(100.0, 30.0), None);
        table.model.set_column_hidden(0, true);
        assert_eq!(table.column_resize_handle_at(79.0, 30.0), None);
        assert_eq!(table.column_resize_handle_at(99.0, 30.0), Some(1));
        table.resize_column(1, 100.0);
        assert_eq!(table.model.column_width(1), 100.0);
    }

    #[test]
    fn table_widget_selection_and_scroll() {
        let mut table = QTableWidget::default();
        table.model.set_row_count(100);
        table.model.set_column_count(2);
        table.set_current_cell(50, 1);
        assert_eq!(table.current_row(), Some(50));
        table.selection_mode = QSelectionMode::SelectRows;
        table.set_current_cell(60, 1);
        assert_eq!(table.current_column(), Some(0));

        let view_height = QTABLE_HEADER_H + QTABLE_ROW_H * 5.0;
        table.reveal_current_row(view_height);
        assert!(table.scroll > 0.0);
        table.clamp_scroll(view_height);
        assert!(table.scroll <= table.scroll_max(view_height));
    }
}
