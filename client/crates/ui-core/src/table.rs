//! `QTableWidget` state: headers, sparse cell data, selection and
//! scroll offset.
//!
//! The server drives a table through the same `setProperty` channel as
//! any widget, with table-shaped payloads mirroring `tcs_qtgui`'s
//! `invoke_qt` operations: `tableHeaders`, `tableData`, `selectRows`,
//! `readOnly`, `currentRow`. Cell reads come back through
//! `getProperty` with the special `currentRowCell:<col>` property.
//!
//! Rows are never materialized: `cells` only holds what the server
//! pushed, and [`Table::visible_row_range`] limits rendering to the
//! rows inside the viewport (100k-row tables stay cheap).

use form_xml::PropValue;
use std::collections::HashMap;

/// Header band height in pixels.
pub const HEADER_H: f32 = 24.0;
/// Row height in pixels (Qt default for a 14px font).
pub const ROW_H: f32 = 20.0;
/// Left/right padding of cell text.
pub const CELL_PAD: f32 = 5.0;
/// Column width when the server did not specify one.
pub const DEFAULT_COL_W: f32 = 90.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// Horizontal header labels, index = column.
    pub headers: Vec<String>,
    /// Per-column width in pixels.
    pub widths: Vec<f32>,
    /// Per-column hidden flag (`hiddenColumns`).
    pub hidden: Vec<bool>,
    /// Rows the server declared (`rowCount`); only some have cells.
    pub row_count: usize,
    /// Sparse cells, 0-based `(row, column)`.
    pub cells: HashMap<(usize, usize), String>,
    /// `readOnly` — disables cell editing (Qt `NoEditTriggers`).
    pub read_only: bool,
    /// `selectRows` — a click selects the whole row (Qt `SelectRows`).
    pub select_rows: bool,
    /// 0-based selected row/column (Qt `currentRow`/`currentColumn`).
    pub current_row: Option<usize>,
    pub current_col: Option<usize>,
    /// Vertical scroll offset in pixels.
    pub scroll: f32,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            widths: Vec::new(),
            hidden: Vec::new(),
            row_count: 0,
            cells: HashMap::new(),
            // Qt: editable by default, item selection by default.
            read_only: false,
            select_rows: false,
            current_row: None,
            current_col: None,
            scroll: 0.0,
        }
    }
}

impl Table {
    // -- server operations -------------------------------------------------

    /// `clear` — drop items, labels and the selection but keep the
    /// declared row/column counts, widths and hidden flags, like
    /// `QTableWidget::clear()`.
    pub fn clear(&mut self) {
        self.cells.clear();
        for header in &mut self.headers {
            header.clear();
        }
        self.current_row = None;
        self.current_col = None;
    }

    /// `rowCount` — `setRowCount`: items outside the new range vanish.
    pub fn set_row_count(&mut self, count: usize) {
        self.row_count = count;
        self.cells.retain(|(row, _), _| *row < self.row_count);
        if self.current_row.is_some_and(|row| row >= self.row_count) {
            self.current_row = None;
            self.current_col = None;
        }
    }

    /// `horizontalHeaderLabels` — one label per column; extra labels
    /// grow the column count.
    pub fn set_header_labels(&mut self, labels: &PropValue) {
        let Some(labels) = labels.as_list() else {
            return;
        };
        for (index, label) in labels.iter().enumerate() {
            if index >= self.headers.len() {
                self.set_column_count(index + 1);
            }
            if let Some(text) = label.as_cell_text() {
                self.headers[index] = text;
            }
        }
    }

    /// `columnWidths` — pixel widths; extra entries grow the column count.
    pub fn set_column_widths(&mut self, widths: &PropValue) {
        let Some(widths) = widths.as_list() else {
            return;
        };
        for (index, width) in widths.iter().enumerate() {
            if index >= self.widths.len() {
                self.set_column_count(index + 1);
            }
            if let Some(px) = width.as_i64() {
                self.widths[index] = px.max(1) as f32;
            }
        }
    }

    /// `hideColumn {index, hidden}` — 1-based index, `hidden` defaults
    /// to `true` (see `invoke_qt`).
    pub fn hide_column(&mut self, cfg: &PropValue) {
        let Some(index) = cfg.get("index").and_then(one_based) else {
            return;
        };
        let hidden = cfg
            .get("hidden")
            .and_then(PropValue::as_bool)
            .unwrap_or(true);
        self.set_hidden(index, hidden);
    }

    /// `tableHeaders {clear, columnCount, horizontalHeaderLabels,
    /// columnWidths, hiddenColumns}` — the subset of `invoke_qt`'s
    /// `tableHeaders` the MVP renderer can honor.
    pub fn apply_headers(&mut self, cfg: &PropValue) {
        if cfg.get("clear").and_then(PropValue::as_bool) == Some(true) {
            self.clear();
        }
        if let Some(count) = cfg.get("columnCount").and_then(PropValue::as_i64) {
            self.set_column_count(count.max(0) as usize);
        }
        if let Some(labels) = cfg.get("horizontalHeaderLabels") {
            self.set_header_labels(labels);
        }
        if let Some(widths) = cfg.get("columnWidths") {
            self.set_column_widths(widths);
        }
        match cfg.get("hiddenColumns") {
            // `[1, 3]` — 1-based column numbers.
            Some(PropValue::List(items)) => {
                for item in items {
                    if let Some(col) = one_based(item) {
                        self.set_hidden(col, true);
                    }
                }
            }
            // `{"1": true}` — 1-based keys.
            Some(PropValue::Map(entries)) => {
                for (key, value) in entries {
                    if let Some(col) = key.parse::<usize>().ok().filter(|n| *n >= 1) {
                        self.set_hidden(col - 1, value.as_bool().unwrap_or(false));
                    }
                }
            }
            _ => {}
        }
    }

    /// `tableData {rowCount, data}` — `data` is either a map of
    /// 1-based row keys or a list of rows; a row is either a map of
    /// 1-based column keys or a list of cells. A missing `rowCount`
    /// means zero, like `invoke_qt`'s `cfg.value("rowCount", 0)`.
    pub fn apply_data(&mut self, cfg: &PropValue) {
        let count = cfg.get("rowCount").and_then(PropValue::as_i64).unwrap_or(0);
        self.set_row_count(count.max(0) as usize);
        let Some(data) = cfg.get("data") else {
            return;
        };
        match data {
            PropValue::Map(rows) => {
                for (key, row_cells) in rows {
                    if let Some(row) = one_based_str(key) {
                        self.apply_row(row, row_cells);
                    }
                }
            }
            PropValue::List(rows) => {
                for (index, row_cells) in rows.iter().enumerate() {
                    self.apply_row(index, row_cells);
                }
            }
            _ => {}
        }
    }

    fn apply_row(&mut self, row: usize, row_cells: &PropValue) {
        match row_cells {
            PropValue::Map(cols) => {
                for (key, value) in cols {
                    if let Some(col) = one_based_str(key) {
                        self.write_cell(row, col, value);
                    }
                }
            }
            PropValue::List(cols) => {
                for (col, value) in cols.iter().enumerate() {
                    self.write_cell(row, col, value);
                }
            }
            _ => {}
        }
    }

    fn write_cell(&mut self, row: usize, col: usize, value: &PropValue) {
        match value.as_cell_text() {
            Some(text) if !text.is_empty() => {
                self.cells.insert((row, col), text);
            }
            _ => {
                // Empty text: no item at all (keeps 100k-row tables sparse).
                self.cells.remove(&(row, col));
            }
        }
    }

    /// `columnCount` resize: labels/widths follow, cells past the new
    /// width are dropped (Qt removes those items).
    pub fn set_column_count(&mut self, count: usize) {
        self.headers.resize(count, String::new());
        self.widths.resize(count, DEFAULT_COL_W);
        self.hidden.resize(count, false);
        self.cells.retain(|(_, col), _| *col < count);
    }

    fn set_hidden(&mut self, col: usize, hidden: bool) {
        if col >= self.hidden.len() {
            self.set_column_count(col + 1);
        }
        // Hiding is a view-only change: cell data stays for unhide.
        self.hidden[col] = hidden;
    }

    /// `selectRows` / `readOnly` flags.
    pub fn set_flag(&mut self, property: &str, value: &PropValue) -> bool {
        let Some(flag) = value.as_bool() else {
            return false;
        };
        match property {
            "selectRows" => {
                self.select_rows = flag;
                if flag {
                    // Qt: `SelectRows` keeps the current column at 0.
                    self.current_col = self.current_row.map(|_| 0);
                }
                true
            }
            "readOnly" => {
                self.read_only = flag;
                true
            }
            _ => false,
        }
    }

    /// `currentRow` set — 1-based on the wire, like `invoke_qt`.
    pub fn set_current_row_1based(&mut self, row: i64) -> bool {
        if self.row_count == 0 {
            return true;
        }
        let row = (row - 1).clamp(0, self.row_count as i64 - 1) as usize;
        self.current_row = Some(row);
        // Qt `setCurrentCell(row, 0)` always lands on the first column.
        self.current_col = Some(0);
        true
    }

    // -- reads (getProperty) ----------------------------------------------

    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.cells.get(&(row, col)).map(String::as_str)
    }

    /// `currentRow` — Qt's property, 0-based; `-1` with no selection.
    pub fn current_row_property(&self) -> PropValue {
        PropValue::Int(self.current_row.map(|r| r as i64).unwrap_or(-1))
    }

    /// `currentRowCell:<col>` — 1-based column, cell text of the
    /// selected row (`""` for an empty cell, `None` with no selection).
    pub fn current_row_cell(&self, col_1based: i64) -> Option<PropValue> {
        let row = self.current_row?;
        let col = col_1based.checked_sub(1)?;
        if col < 0 || col as usize >= self.column_count_hint() {
            return None;
        }
        Some(PropValue::Str(
            self.cell(row, col as usize).unwrap_or("").to_string(),
        ))
    }

    fn column_count_hint(&self) -> usize {
        self.widths.len().max(self.headers.len())
    }

    /// Number of columns the table currently has (labels or widths).
    pub fn column_count(&self) -> usize {
        self.column_count_hint()
    }

    // -- geometry (rendering / hit-testing) --------------------------------

    pub fn column_width(&self, col: usize) -> f32 {
        self.widths
            .get(col)
            .copied()
            .filter(|w| *w > 0.0)
            .unwrap_or(DEFAULT_COL_W)
    }

    pub fn is_hidden(&self, col: usize) -> bool {
        self.hidden.get(col).copied().unwrap_or(false)
    }

    /// X offset of `col` inside the table (hidden columns take no space).
    pub fn col_offset(&self, col: usize) -> f32 {
        (0..col)
            .filter(|c| !self.is_hidden(*c))
            .map(|c| self.column_width(c))
            .sum()
    }

    /// Visible column under a local x coordinate.
    pub fn col_at(&self, x: f32) -> Option<usize> {
        let mut cursor = 0.0;
        for col in 0..self.headers.len().max(self.column_count_hint()) {
            if self.is_hidden(col) {
                continue;
            }
            let w = self.column_width(col);
            if x >= cursor && x < cursor + w {
                return Some(col);
            }
            cursor += w;
        }
        None
    }

    /// Visible row under a local y coordinate (`None` over the header).
    pub fn row_at(&self, y: f32) -> Option<usize> {
        if y < HEADER_H {
            return None;
        }
        let row = ((y - HEADER_H + self.scroll) / ROW_H) as usize;
        (row < self.row_count).then_some(row)
    }

    /// Rows intersecting a viewport of `view_h` pixels — the whole
    /// point of virtualization: only these are rendered.
    pub fn visible_row_range(&self, view_h: f32) -> std::ops::Range<usize> {
        let body = (view_h - HEADER_H).max(0.0);
        let first = (self.scroll / ROW_H).floor().max(0.0) as usize;
        let last = ((self.scroll + body) / ROW_H).ceil() as usize;
        first..last.min(self.row_count)
    }

    /// Maximum scroll offset for a viewport of `view_h` pixels.
    pub fn scroll_max(&self, view_h: f32) -> f32 {
        (self.row_count as f32 * ROW_H - (view_h - HEADER_H)).max(0.0)
    }

    /// Scroll by `dy` pixels (positive = content moves up), clamped.
    pub fn scroll_by(&mut self, dy: f32, view_h: f32) {
        self.scroll = (self.scroll + dy).clamp(0.0, self.scroll_max(view_h));
    }

    /// Keep the scroll offset valid after the row count shrinks.
    pub fn clamp_scroll(&mut self, view_h: f32) {
        self.scroll = self.scroll.clamp(0.0, self.scroll_max(view_h));
    }

    /// Scroll so the selected row is inside a viewport of `view_h`
    /// pixels (`QTableView::scrollTo` after `setCurrentCell`).
    pub fn reveal_current_row(&mut self, view_h: f32) {
        let Some(row) = self.current_row else {
            return;
        };
        let top = row as f32 * ROW_H;
        let bottom = top + ROW_H;
        let body = (view_h - HEADER_H).max(0.0);
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + body {
            self.scroll = bottom - body;
        }
        self.scroll = self.scroll.clamp(0.0, self.scroll_max(view_h));
    }

    /// Y of `row`'s top edge in local coordinates (clipped to the
    /// viewport is the caller's job).
    pub fn row_y(&self, row: usize) -> f32 {
        HEADER_H + row as f32 * ROW_H - self.scroll
    }
}

/// 1-based wire index → 0-based (`"3"`/`3` → `2`).
fn one_based(value: &PropValue) -> Option<usize> {
    value
        .as_i64()
        .and_then(|n| n.checked_sub(1))
        .filter(|n| *n >= 0)
        .map(|n| n as usize)
}

fn one_based_str(key: &str) -> Option<usize> {
    key.parse::<usize>().ok().filter(|n| *n >= 1).map(|n| n - 1)
}
