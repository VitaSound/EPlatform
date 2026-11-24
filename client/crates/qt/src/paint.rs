use crate::core::QVariant;
use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QPointF, QRectF};
use crate::item_model::{QSortOrder, QTABLE_HEADER_H, QTABLE_ROW_H};
use crate::qss::{QStyleDeclarations, QStyleSheet};
use crate::style::{QControlElement, QPrimitiveElement, QStyle, QStyleOption, QStyleState};
use crate::widgets::{QWidget, QWidgetId, QWidgetTree};

pub fn paint_widget_tree(
    tree: &QWidgetTree,
    root: QWidgetId,
    style: &dyn QStyle,
    painter: &mut QPainter,
) {
    paint_widget(tree, root, style, painter);
}

fn paint_widget(
    tree: &QWidgetTree,
    widget_id: QWidgetId,
    style: &dyn QStyle,
    painter: &mut QPainter,
) {
    let Some(widget) = tree.widget(widget_id) else {
        return;
    };
    if !widget.visible {
        return;
    }
    painter.save();
    painter.set_clip_rect(Some(widget.geometry));
    painter.set_font(widget.font.clone());
    painter.set_palette(widget.palette.clone());

    let mut state = if widget.enabled {
        QStyleState::ENABLED
    } else {
        QStyleState::DISABLED
    };
    if widget.has_focus {
        state = state | QStyleState::FOCUSED | QStyleState::ACTIVE;
    }
    if widget.pressed {
        state = state | QStyleState::SUNKEN;
    }
    if widget.under_mouse {
        state = state | QStyleState::HOVER;
    }
    if property_bool(widget, "checked") {
        state = state | QStyleState::SELECTED;
    }
    if widget.default_button {
        state = state | QStyleState::DEFAULT;
    }
    let mut option =
        QStyleOption::new(widget.geometry, widget.palette.clone(), widget.font.clone());
    option.state = state;
    let qss = resolved_style(tree, widget_id);

    paint_widget_background(widget, style, &option, painter, &qss);
    paint_table(widget, painter);
    paint_item_views(widget, painter);
    paint_control_indicator(widget, painter);
    paint_line_edit_selection(widget, painter, &qss);
    paint_widget_text(widget, painter, &qss);
    paint_line_edit_caret(widget, painter, &qss);
    painter.restore();

    for child in &widget.children {
        paint_widget(tree, *child, style, painter);
    }
}

fn resolved_style(tree: &QWidgetTree, widget_id: QWidgetId) -> QStyleDeclarations {
    let mut sheets: Vec<&QStyleSheet> = Vec::new();
    let mut current = Some(widget_id);
    while let Some(id) = current {
        let Some(widget) = tree.widget(id) else {
            break;
        };
        if let Some(sheet) = &widget.style_sheet {
            sheets.push(sheet);
        }
        current = widget.parent;
    }
    sheets.reverse();
    let Some(widget) = tree.widget(widget_id) else {
        return QStyleDeclarations::default();
    };
    let mut declarations = QStyleDeclarations::default();
    for sheet in sheets {
        declarations.merge(&sheet.resolve(widget));
    }
    declarations
}

fn paint_widget_background(
    widget: &QWidget,
    style: &dyn QStyle,
    option: &QStyleOption,
    painter: &mut QPainter,
    qss: &QStyleDeclarations,
) {
    match widget.class_name.as_str() {
        "QDialog" | "QWidget" => {
            painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Panel, option));
        }
        "QGroupBox" => {
            let metrics = QFontMetrics::new(&widget.font);
            let top_margin = metrics.height().max(14.0) + 3.0;
            let mut frame_option = option.clone();
            frame_option.rect = QRectF::new(
                widget.geometry.x,
                widget.geometry.y + top_margin,
                widget.geometry.width,
                (widget.geometry.height - top_margin).max(0.0),
            );
            painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Frame, &frame_option));
        }
        "QLabel" => {}
        "QLineEdit" => {
            painter.draw_commands(&style.draw_control(QControlElement::LineEdit, option));
        }
        "QPushButton" | "QToolButton" => {
            painter.draw_commands(&style.draw_control(QControlElement::PushButton, option));
        }
        "QCheckBox" => {
            painter.draw_commands(&style.draw_control(QControlElement::CheckBox, option));
        }
        "QRadioButton" => {}
        "QComboBox" => {
            painter.draw_commands(&style.draw_control(QControlElement::ComboBox, option));
        }
        "QTableWidget" | "QListWidget" | "QTreeWidget" => {
            painter.draw_commands(&style.draw_control(QControlElement::TableView, option));
        }
        _ => {}
    }
    if let Some(background) = qss.background {
        let radius = qss.border_radius.unwrap_or(0.0);
        if radius > 0.0 {
            painter.fill_rounded_rect(widget.geometry, radius, background);
        } else {
            painter.fill_rect(widget.geometry, background);
        }
    }
    if let Some(width) = qss.border_width {
        if width > 0.0 {
            let color = qss
                .border_color
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::WindowText));
            if let Some(radius) = qss.border_radius {
                painter.stroke_rounded_rect(widget.geometry, radius, width, color);
            } else {
                painter.stroke_rect(widget.geometry, width, color);
            }
        }
    }
}

fn property_bool(widget: &QWidget, key: &str) -> bool {
    matches!(widget.property(key), Some(QVariant::Bool(true)))
}

fn property_int(widget: &QWidget, key: &str) -> Option<i64> {
    match widget.property(key) {
        Some(QVariant::Int(value)) => Some(*value),
        Some(QVariant::Double(value)) => Some(*value as i64),
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

fn property_texts(widget: &QWidget, key: &str) -> Vec<String> {
    match widget.property(key) {
        Some(QVariant::List(items)) => items.iter().filter_map(variant_text).collect(),
        _ => Vec::new(),
    }
}

fn combo_text(widget: &QWidget) -> Option<String> {
    let items = property_texts(widget, "items");
    if !items.is_empty() {
        let index = property_int(widget, "currentIndex").unwrap_or(0).max(0) as usize;
        return items.get(index).cloned().or_else(|| items.first().cloned());
    }
    widget_text(widget)
}

fn paint_item_views(widget: &QWidget, painter: &mut QPainter) {
    match widget.class_name.as_str() {
        "QListWidget" => paint_list_widget(widget, painter),
        "QTreeWidget" => paint_tree_widget(widget, painter),
        _ => {}
    }
}

fn paint_list_widget(widget: &QWidget, painter: &mut QPainter) {
    let items = property_texts(widget, "items");
    if items.is_empty() {
        return;
    }
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let row_h = (line_height + 4.0).max(QTABLE_ROW_H);
    let current = property_int(widget, "currentIndex").unwrap_or(-1);
    let text_color = widget.palette.color(QPaletteRole::Text);
    let selected_text = widget.palette.color(QPaletteRole::HighlightedText);
    for (index, item) in items.iter().enumerate() {
        let y = widget.geometry.y + 2.0 + index as f32 * row_h;
        if y + row_h > widget.geometry.bottom() {
            break;
        }
        let selected = current == index as i64;
        if selected {
            painter.fill_rect(
                QRectF::new(
                    widget.geometry.x + 1.0,
                    y,
                    widget.geometry.width - 2.0,
                    row_h,
                ),
                widget.palette.color(QPaletteRole::Highlight),
            );
        }
        painter.draw_text(
            QRectF::new(
                widget.geometry.x + 4.0,
                y + ((row_h - line_height) / 2.0).max(0.0),
                (widget.geometry.width - 8.0).max(1.0),
                line_height,
            ),
            item,
            if selected { selected_text } else { text_color },
        );
    }
}

fn flatten_tree(items: &[QVariant], depth: usize, out: &mut Vec<(usize, String)>) {
    for item in items {
        let QVariant::Map(entries) = item else {
            continue;
        };
        let text = entries
            .iter()
            .find(|(key, _)| key == "text")
            .and_then(|(_, value)| variant_text(value))
            .unwrap_or_default();
        let columns = entries
            .iter()
            .find(|(key, _)| key == "columns")
            .map(|(_, value)| match value {
                QVariant::List(items) => items.iter().filter_map(variant_text).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .unwrap_or_default();
        let mut row = text;
        for column in columns {
            if !column.is_empty() {
                if !row.is_empty() {
                    row.push_str("  ");
                }
                row.push_str(&column);
            }
        }
        out.push((depth, row));
        if let Some(QVariant::List(children)) = entries
            .iter()
            .find(|(key, _)| key == "children")
            .map(|(_, value)| value)
        {
            flatten_tree(children, depth + 1, out);
        }
    }
}

fn fusion_button_color(palette: &crate::gui::QPalette) -> QColor {
    let button = palette.color(QPaletteRole::Button);
    button
        .lighter(100 + ((180_i32 - button.q_gray() as i32) / 6).max(1))
        .with_saturation_factor(75)
}

fn paint_header_gradient(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    let base = fusion_button_color(&widget.palette);
    painter.fill_linear_gradient_rect(rect, 0.0, base.lighter(104), base.darker(102));
}

fn paint_header_section(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    paint_header_gradient(widget, painter, rect);
    if rect.width > 0.0 && rect.height > 0.0 {
        painter.fill_rect(
            QRectF::new(rect.x, rect.y, rect.width, 1.0),
            QColor::rgba8(255, 255, 255, 30),
        );
        painter.fill_rect(
            QRectF::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0),
            widget.palette.color(QPaletteRole::Window).darker(140),
        );
    }
}

fn paint_tree_widget(widget: &QWidget, painter: &mut QPainter) {
    let data = match widget.property("data") {
        Some(QVariant::List(data)) => data.as_slice(),
        _ => &[],
    };
    let mut headers = property_texts(widget, "headerLabels");
    if headers.is_empty() && widget.class_name == "QTreeWidget" {
        headers.push("1".into());
    }
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let header_h = QTABLE_HEADER_H;
    let header_color = widget.palette.color(QPaletteRole::ButtonText);
    paint_header_section(
        widget,
        painter,
        QRectF::new(
            widget.geometry.x,
            widget.geometry.y,
            widget.geometry.width,
            header_h,
        ),
    );
    painter.stroke_rect(
        widget.geometry,
        1.0,
        widget.palette.color(QPaletteRole::Mid),
    );
    if !headers.is_empty() {
        painter.draw_text(
            QRectF::new(
                widget.geometry.x + 4.0,
                widget.geometry.y + ((header_h - line_height) / 2.0).max(0.0),
                (widget.geometry.width - 8.0).max(1.0),
                line_height,
            ),
            headers.join("  "),
            header_color,
        );
    }
    let mut rows = Vec::new();
    flatten_tree(data, 0, &mut rows);
    let text_color = widget.palette.color(QPaletteRole::Text);
    let row_h = (line_height + 4.0).max(QTABLE_ROW_H);
    for (index, (depth, text)) in rows.iter().enumerate() {
        let y = widget.geometry.y + header_h + index as f32 * row_h;
        if y + row_h > widget.geometry.bottom() {
            break;
        }
        painter.draw_text(
            QRectF::new(
                widget.geometry.x + 4.0 + *depth as f32 * 14.0,
                y + ((row_h - line_height) / 2.0).max(0.0),
                (widget.geometry.width - 8.0 - *depth as f32 * 14.0).max(1.0),
                line_height,
            ),
            text,
            text_color,
        );
    }
}

fn paint_control_indicator(widget: &QWidget, painter: &mut QPainter) {
    let checked = property_bool(widget, "checked");
    if widget.class_name == "QRadioButton" {
        let size = widget
            .font
            .pixel_size
            .min(widget.geometry.height - 4.0)
            .max(9.0);
        let x = widget.geometry.x.round();
        let y = (widget.geometry.y + (widget.geometry.height - size) / 2.0).round();
        let outline = if widget.enabled {
            widget.palette.color(QPaletteRole::Window).darker(150)
        } else {
            widget.palette.color(QPaletteRole::Mid)
        };
        painter.fill_ellipse(
            QRectF::new(x, y, size, size),
            widget.palette.color(QPaletteRole::Base),
        );
        painter.stroke_ellipse(QRectF::new(x, y, size, size), 1.0, outline);
        if checked {
            let parity = ((size as i32 + 1) % 2) as f32;
            let outline_radius = (size + parity) / 2.0 - 1.0;
            let radius = outline_radius / 1.8;
            let center_x = x + size / 2.0;
            let center_y = y + size / 2.0;
            painter.fill_ellipse(
                QRectF::new(
                    center_x - radius,
                    center_y - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                widget
                    .palette
                    .color(QPaletteRole::Text)
                    .darker(120)
                    .with_alpha(180.0 / 255.0),
            );
        }
        return;
    }
    if !checked || widget.class_name != "QCheckBox" {
        return;
    }
    let size = widget
        .font
        .pixel_size
        .min(widget.geometry.height - 4.0)
        .max(9.0);
    let x = widget.geometry.x;
    let y = widget.geometry.y + (widget.geometry.height - size) / 2.0;
    let width = (size - 1.0).max(1.0);
    let padding = 1.0 + width * 0.13;
    let pen_width = (width * 0.13).max(1.5).min(width * 0.20);
    let color = if widget.enabled {
        widget
            .palette
            .color(QPaletteRole::Text)
            .darker(120)
            .with_alpha(210.0 / 255.0)
    } else {
        widget.palette.color(QPaletteRole::Mid)
    };
    let first = QPointF::new(x + padding + width * 0.11 - 0.8, y + width * 0.47 + 0.5);
    let second = QPointF::new(x + width * 0.5 - 0.8, y + width - padding + 0.5);
    let third = QPointF::new(x + width - padding - 0.8, y + padding + 0.5);
    painter.draw_line(first, second, pen_width, color);
    painter.draw_line(second, third, pen_width, color);
}

fn paint_table(widget: &QWidget, painter: &mut QPainter) {
    if widget.class_name != "QTableWidget" {
        return;
    }
    let Some(table) = widget.table.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let frame = 1.0;
    let left = rect.x + frame;
    let top = rect.y + frame;
    let right = rect.right() - frame;
    let bottom = rect.bottom() - frame;
    let header_h = QTABLE_HEADER_H;
    let row_header_w = table.row_header_width(&widget.font);
    let body_x = left + row_header_w;
    let body_top = top + header_h;
    let rows_h = (bottom - body_top).max(0.0);
    let content_h =
        (table.model.row_count() as f32 * QTABLE_ROW_H - table.scroll).clamp(0.0, rows_h);
    let content_bottom = body_top + content_h;
    let rows = table.visible_row_range(rect.height);
    let palette = &widget.palette;
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let visible_width: f32 = (0..table.model.column_count())
        .filter(|column| !table.model.is_column_hidden(*column))
        .map(|column| table.model.column_width(column))
        .sum();

    paint_header_section(
        widget,
        painter,
        QRectF::new(left, top, row_header_w, header_h),
    );
    let mut header_x = body_x;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        paint_header_section(widget, painter, QRectF::new(header_x, top, width, header_h));
        header_x += width;
    }
    if header_x < right {
        painter.fill_rect(
            QRectF::new(header_x, top, (right - header_x).max(0.0), header_h),
            palette.color(QPaletteRole::Window),
        );
    }
    if body_top < bottom {
        painter.fill_rect(
            QRectF::new(left, body_top, row_header_w, (bottom - body_top).max(0.0)),
            palette.color(QPaletteRole::Button),
        );
        for row in rows.clone() {
            let y = rect.y + table.row_y(row);
            let row_top = y.max(body_top);
            let row_bottom = (y + QTABLE_ROW_H).min(bottom);
            if row_bottom > row_top {
                paint_header_gradient(
                    widget,
                    painter,
                    QRectF::new(left, row_top, row_header_w, row_bottom - row_top),
                );
            }
        }
    }

    let mut focus_border: Option<(QRectF, QColor)> = None;
    for (row, column) in table.selection.selected_cells() {
        if row < rows.start
            || row >= rows.end
            || column >= table.model.column_count()
            || table.model.is_column_hidden(column)
        {
            continue;
        }
        let top_y = (rect.y + table.row_y(row)).max(body_top);
        let bottom_y = (rect.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
        if bottom_y <= top_y {
            continue;
        }
        painter.fill_rect(
            QRectF::new(
                body_x + table.model.column_offset(column),
                top_y,
                table.model.column_width(column),
                bottom_y - top_y,
            ),
            palette.color(QPaletteRole::Highlight),
        );
    }

    if widget.has_focus {
        if let Some((row, column)) = table.current() {
            if row >= rows.start && row < rows.end && !table.model.is_column_hidden(column) {
                let top_y = (rect.y + table.row_y(row)).max(body_top);
                let bottom_y = (rect.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
                if bottom_y > top_y {
                    let cell = QRectF::new(
                        body_x + table.model.column_offset(column),
                        top_y,
                        table.model.column_width(column),
                        bottom_y - top_y,
                    );
                    let mut fill = palette.color(QPaletteRole::Highlight).darker(125);
                    if fill.value() > 160 {
                        fill = fill.with_value(160);
                    }
                    fill = fill.with_alpha(30.0 / 255.0);
                    painter.fill_linear_gradient_rect(cell, 0.0, fill.lighter(160), fill);
                    focus_border = Some((
                        QRectF::new(cell.x, cell.y, (cell.width - 1.0).max(1.0), cell.height),
                        QColor::rgb(0xb1, 0xc9, 0xd7),
                    ));
                }
            }
        }
    }

    let grid = palette.color(QPaletteRole::Window).darker(120);
    let header_grid = palette.color(QPaletteRole::Window).darker(140);
    if content_h > 0.0 {
        let mut horizontal: Vec<f32> = rows
            .clone()
            .filter_map(|row| {
                let y = rect.y + table.row_y(row);
                (y >= body_top - 0.5 && y < content_bottom).then_some(y)
            })
            .collect();
        if let Some(last) = rows.clone().last() {
            horizontal.push(rect.y + table.row_y(last) + QTABLE_ROW_H);
        }
        for y in horizontal {
            painter.fill_rect(QRectF::new(body_x, y, visible_width, 1.0), grid);
            painter.fill_rect(QRectF::new(left, y, row_header_w, 1.0), header_grid);
        }
        let mut column_x = body_x - 1.0;
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            painter.fill_rect(QRectF::new(column_x, body_top, 1.0, content_h), grid);
            column_x += table.model.column_width(column);
        }
        painter.fill_rect(QRectF::new(column_x, body_top, 1.0, content_h), grid);
        painter.fill_rect(
            QRectF::new(body_x - 1.0, body_top, 1.0, content_h),
            header_grid,
        );
    }

    if let Some((rect, color)) = focus_border {
        painter.stroke_rounded_rect(rect, 1.0, 1.0, color);
    }

    painter.fill_rect(
        QRectF::new(left, body_top - 1.0, (right - left).max(0.0), 1.0),
        header_grid,
    );
    let mut column_x = body_x - 1.0;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        painter.fill_rect(QRectF::new(column_x, top, 1.0, header_h), header_grid);
        column_x += table.model.column_width(column);
    }
    painter.fill_rect(QRectF::new(column_x, top, 1.0, header_h), header_grid);

    let header_color = palette.color(QPaletteRole::ButtonText);
    let mut column_x = body_x;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        let label = table
            .model
            .header(column)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| (column + 1).to_string());
        let label_width = metrics.horizontal_advance(&label);
        painter.draw_text(
            QRectF::new(
                column_x + ((width - label_width) / 2.0).max(2.0),
                top + ((header_h - line_height) / 2.0).max(0.0),
                (width - 4.0).max(1.0),
                line_height,
            ),
            label,
            header_color,
        );
        if let Some((sort_column, order)) = table.sort_indicator {
            if sort_column == column {
                painter.draw_text(
                    QRectF::new(
                        column_x + width - 12.0,
                        top + ((header_h - line_height) / 2.0).max(0.0),
                        10.0,
                        line_height,
                    ),
                    if order == QSortOrder::Ascending {
                        "▲"
                    } else {
                        "▼"
                    },
                    header_color,
                );
            }
        }
        column_x += width;
    }

    let text_color = palette.color(QPaletteRole::Text);
    let selected_text = palette.color(QPaletteRole::HighlightedText);
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        body_top,
        (right - left).max(0.0),
        (content_bottom - body_top).max(0.0),
    )));
    for row in rows {
        let raw_y = rect.y + table.row_y(row);
        let row_top = raw_y.max(body_top);
        let row_bottom = (raw_y + QTABLE_ROW_H).min(content_bottom);
        if row_bottom <= row_top {
            continue;
        }
        let row_number = (row + 1).to_string();
        let number_width = metrics.horizontal_advance(&row_number);
        painter.draw_text(
            QRectF::new(
                left + ((row_header_w - number_width) / 2.0).max(2.0),
                raw_y + ((QTABLE_ROW_H - line_height) / 2.0).max(0.0),
                (row_header_w - 4.0).max(1.0),
                line_height,
            ),
            row_number,
            text_color,
        );
        let mut column_x = body_x;
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            let width = table.model.column_width(column);
            let editing = table
                .editor
                .as_ref()
                .filter(|editor| editor.row == row && editor.column == column);
            let text = editing
                .map(|editor| editor.text.as_str())
                .or_else(|| table.model.cell(row, column));
            let is_selected = table.is_selected(row, column);
            if let Some(text) = text {
                painter.draw_text(
                    QRectF::new(
                        column_x + 3.0,
                        raw_y + ((QTABLE_ROW_H - line_height) / 2.0).max(0.0),
                        (width - 6.0).max(1.0),
                        line_height,
                    ),
                    text,
                    if is_selected {
                        selected_text
                    } else {
                        text_color
                    },
                );
            }
            if let Some(editor) = editing {
                let caret_y = raw_y + 3.0;
                let caret_h = (QTABLE_ROW_H - 6.0).min(row_bottom - caret_y).max(0.0);
                if caret_h > 0.0 {
                    painter.fill_rect(
                        QRectF::new(
                            (column_x + 3.0 + metrics.horizontal_advance(&editor.text) + 1.0)
                                .min(column_x + width - 2.0),
                            caret_y.max(body_top),
                            1.0,
                            caret_h,
                        ),
                        text_color,
                    );
                }
            }
            column_x += width;
        }
    }
    painter.restore();
}

fn paint_line_edit_selection(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if widget.class_name != "QLineEdit" {
        return;
    }
    let Some(anchor) = widget.selection_anchor else {
        return;
    };
    let text = widget_text(widget).unwrap_or_default();
    let length = text.chars().count();
    let start = anchor.min(widget.cursor_position).min(length);
    let end = anchor.max(widget.cursor_position).min(length);
    if start >= end {
        return;
    }
    let metrics = QFontMetrics::new(&widget.font);
    let prefix: String = text.chars().take(start).collect();
    let selected: String = text.chars().take(end).skip(start).collect();
    let padding = qss.padding.map(|padding| padding.0).unwrap_or(4.0);
    let x = widget.geometry.x + padding + metrics.horizontal_advance(&prefix);
    let width = metrics.horizontal_advance(&selected);
    let y = widget.geometry.y + 2.0;
    let height = (widget.geometry.height - 4.0).max(0.0);
    if width > 0.0 && height > 0.0 {
        painter.fill_rect(
            QRectF::new(x, y, width, height),
            qss.selection_background
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::Highlight)),
        );
    }
}

fn paint_line_edit_caret(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if widget.class_name != "QLineEdit"
        || !widget.has_focus
        || !widget.enabled
        || !widget.cursor_visible
    {
        return;
    }
    let text = widget_text(widget).unwrap_or_default();
    let prefix: String = text.chars().take(widget.cursor_position).collect();
    let metrics = QFontMetrics::new(&widget.font);
    let padding = qss.padding.map(|padding| padding.0).unwrap_or(4.0);
    let gap = if widget.cursor_position > 0 { 1.0 } else { 0.0 };
    let caret_x = (widget.geometry.x + padding + metrics.horizontal_advance(&prefix) + gap)
        .min(widget.geometry.right() - 2.0);
    let caret_y = widget.geometry.y + 3.0;
    let caret_h = (widget.geometry.height - 6.0).max(0.0);
    if caret_h > 0.0 {
        painter.fill_rect(
            QRectF::new(caret_x, caret_y, 1.0, caret_h),
            text_color(widget, qss),
        );
    }
}

fn paint_line_edit_text(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    let Some(text) = widget_text(widget) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let y = widget.geometry.y + ((widget.geometry.height - line_height) / 2.0).max(0.0);
    let padding = qss.padding.map(|padding| padding.0).unwrap_or(4.0);
    let base_x = widget.geometry.x + padding;
    let color = text_color(widget, qss);
    let (start, end) = match widget.selection_anchor {
        Some(anchor) => {
            let length = text.chars().count();
            let start = anchor.min(widget.cursor_position).min(length);
            let end = anchor.max(widget.cursor_position).min(length);
            (start, end)
        }
        None => (0, 0),
    };
    if start >= end {
        painter.draw_text(
            QRectF::new(
                base_x,
                y,
                (widget.geometry.width - 8.0).max(1.0),
                line_height,
            ),
            text,
            color,
        );
        return;
    }
    let prefix: String = text.chars().take(start).collect();
    let selected: String = text.chars().take(end).skip(start).collect();
    let suffix: String = text.chars().skip(end).collect();
    let prefix_width = metrics.horizontal_advance(&prefix);
    let selected_width = metrics.horizontal_advance(&selected);
    if !prefix.is_empty() {
        painter.draw_text(
            QRectF::new(base_x, y, prefix_width.max(1.0), line_height),
            prefix,
            color,
        );
    }
    if !selected.is_empty() {
        painter.draw_text(
            QRectF::new(
                base_x + prefix_width,
                y,
                selected_width.max(1.0),
                line_height,
            ),
            selected,
            qss.selection_color
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::HighlightedText)),
        );
    }
    if !suffix.is_empty() {
        painter.draw_text(
            QRectF::new(
                base_x + prefix_width + selected_width,
                y,
                (widget.geometry.width - 8.0 - prefix_width - selected_width).max(1.0),
                line_height,
            ),
            suffix,
            color,
        );
    }
}

fn paint_widget_text(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if widget.class_name == "QLineEdit" {
        paint_line_edit_text(widget, painter, qss);
        return;
    }
    if widget.class_name == "QGroupBox" {
        if let Some(title) = widget_text(widget) {
            if !title.is_empty() {
                let metrics = QFontMetrics::new(&widget.font);
                let line_height = metrics.height();
                painter.draw_text(
                    QRectF::new(
                        widget.geometry.x + 8.0,
                        widget.geometry.y + 2.0,
                        (widget.geometry.width - 16.0).max(1.0),
                        line_height,
                    ),
                    title,
                    text_color(widget, qss),
                );
            }
        }
        return;
    }
    if matches!(widget.class_name.as_str(), "QPushButton" | "QToolButton") {
        paint_button_label(widget, painter, qss);
        return;
    }
    if widget.class_name == "QComboBox" {
        let metrics = QFontMetrics::new(&widget.font);
        let line_height = metrics.height();
        if let Some(text) = combo_text(widget) {
            painter.draw_text(
                QRectF::new(
                    widget.geometry.x + 3.0,
                    widget.geometry.y + ((widget.geometry.height - line_height) / 2.0).max(0.0),
                    (widget.geometry.width - 26.0).max(1.0),
                    line_height,
                ),
                text,
                text_color(widget, qss),
            );
        }
        let area = QRectF::new(
            widget.geometry.right() - 20.5,
            widget.geometry.y - 2.0,
            19.0,
            widget.geometry.height + 4.0,
        );
        let width = 8.0_f32.min(area.width).min(area.height);
        let height = 5.5_f32.min(width);
        let center_x = area.x + area.width / 2.0;
        let center_y = area.y + area.height / 2.0;
        painter.fill_triangle(
            QPointF::new(center_x - width / 2.0, center_y - height / 2.0),
            QPointF::new(center_x + width / 2.0, center_y - height / 2.0),
            QPointF::new(center_x, center_y + height / 2.0),
            text_color(widget, qss).with_alpha(0.63),
        );
        return;
    }
    if matches!(widget.class_name.as_str(), "QListWidget" | "QTreeWidget") {
        return;
    }
    let Some(text) = widget_text(widget) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    if widget.class_name == "QTableWidget" {
        return;
    }
    let metrics = QFontMetrics::new(&widget.font);
    let text_width = metrics.horizontal_advance(&text);
    let line_height = metrics.height();
    let mut rect = QRectF::new(
        widget.geometry.x,
        widget.geometry.y + ((widget.geometry.height - line_height) / 2.0).max(0.0),
        text_width,
        line_height,
    );
    match widget.class_name.as_str() {
        "QPushButton" | "QToolButton" => {
            rect.x += ((widget.geometry.width - text_width) / 2.0).max(0.0);
        }
        "QLineEdit" => {
            rect.x += 4.0;
        }
        "QCheckBox" | "QRadioButton" => {
            rect.x += widget.font.pixel_size + 6.0;
        }
        "QLabel" => {}
        _ => {
            rect.x += qss.padding.map(|padding| padding.0).unwrap_or(4.0);
        }
    }
    let color = text_color(widget, qss);
    painter.draw_text(rect, text, color);
}

fn paint_button_label(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    let text = widget_text(widget).unwrap_or_default();
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let down = widget.pressed || property_bool(widget, "checked");
    let shift = if down { 1.0 } else { 0.0 };
    let color = text_color(widget, qss);
    let contents = QRectF::new(
        widget.geometry.x + 1.0,
        widget.geometry.y + 1.0,
        (widget.geometry.width - 2.0).max(1.0),
        (widget.geometry.height - 2.0).max(1.0),
    );
    let icon = if widget.enabled {
        widget.icon.as_ref()
    } else {
        widget.icon_disabled.as_ref().or(widget.icon.as_ref())
    };
    if let Some(pixmap) = icon {
        let icon_width = if widget.icon_size.width > 0.0 {
            widget.icon_size.width
        } else {
            pixmap.width as f32
        };
        let icon_height = if widget.icon_size.height > 0.0 {
            widget.icon_size.height
        } else {
            pixmap.height as f32
        };
        let text_width = if text.is_empty() {
            0.0
        } else {
            metrics.horizontal_advance(&text)
        };
        let label_width = icon_width
            + if text.is_empty() {
                0.0
            } else {
                text_width + 4.0
            };
        let start = contents.x + ((contents.width - label_width) / 2.0).max(0.0) + shift;
        let icon_y = contents.y + ((contents.height - icon_height) / 2.0).max(0.0) + shift;
        let opacity = if widget.enabled || widget.icon_disabled.is_some() {
            1.0
        } else {
            0.45
        };
        painter.draw_pixmap(
            QRectF::new(start, icon_y, icon_width, icon_height),
            pixmap.clone(),
            opacity,
        );
        if !text.is_empty() {
            let text_x = start + icon_width + 2.0;
            let text_y = contents.y + ((contents.height - line_height) / 2.0).max(0.0) + shift;
            painter.draw_text(
                QRectF::new(
                    text_x,
                    text_y,
                    (contents.right() - text_x).max(1.0),
                    line_height,
                ),
                text,
                color,
            );
        }
        return;
    }
    if text.is_empty() {
        return;
    }
    let text_width = metrics.horizontal_advance(&text);
    let text_y =
        widget.geometry.y + ((widget.geometry.height - line_height) / 2.0).max(0.0) + shift;
    painter.draw_text(
        QRectF::new(
            widget.geometry.x + ((widget.geometry.width - text_width) / 2.0).max(0.0) + shift,
            text_y,
            text_width.max(1.0),
            line_height,
        ),
        text,
        color,
    );
}

fn widget_text(widget: &QWidget) -> Option<String> {
    if widget.class_name == "QGroupBox" {
        return match widget.property("title") {
            Some(QVariant::String(text)) | Some(QVariant::Enum(text)) => Some(text.clone()),
            _ => None,
        };
    }
    match widget.property("text") {
        Some(crate::core::QVariant::String(text)) | Some(crate::core::QVariant::Enum(text)) => {
            Some(text.clone())
        }
        Some(crate::core::QVariant::Int(value)) => Some(value.to_string()),
        Some(crate::core::QVariant::Double(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn text_color(widget: &QWidget, qss: &QStyleDeclarations) -> QColor {
    if let Some(color) = qss.color {
        return color;
    }
    let role = match widget.class_name.as_str() {
        "QLineEdit" => QPaletteRole::Text,
        "QPushButton" | "QToolButton" | "QComboBox" => QPaletteRole::ButtonText,
        _ => QPaletteRole::WindowText,
    };
    let color = widget.palette.color(role);
    if widget.enabled {
        color
    } else {
        widget.palette.color(QPaletteRole::Mid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::QPaintCommand;
    use crate::style::QFusionStyle;
    use crate::widgets::load_ui;

    const FIXTURE: &str = include_str!("../../form-xml/tests/fixtures/test_dialog01.ui");
    const DIALOG02: &str = include_str!("../../../../server/priv/qt_forms/test_dialog02.ui");
    const TABLE_UI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>320</width><height>200</height></rect></property>
  <property name="windowTitle"><string>Table</string></property>
  <layout class="QVBoxLayout" name="verticalLayout">
   <item><widget class="QTableWidget" name="tableWidget"/></item>
  </layout>
 </widget>
</ui>"#;

    #[test]
    fn paints_dialog_widgets_in_qt_order() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            [
                "TextLabel",
                "TextLabel",
                "TextLabel",
                "PushButton",
                "PushButton"
            ]
        );

        let fills = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::FillRect { .. }))
            .count();
        assert!(fills >= 4);
        assert!(painter
            .commands()
            .iter()
            .any(|command| matches!(command, QPaintCommand::SetClip { .. })));
    }

    #[test]
    fn paints_table_headers_and_cells() {
        let (mut tree, root) = load_ui(TABLE_UI).expect("loads ui");
        let table = tree.find_widget("tableWidget").expect("table");
        tree.set_property(table, "rowCount", crate::core::QVariant::Int(2));
        tree.set_property(table, "columnCount", crate::core::QVariant::Int(2));
        tree.set_property(
            table,
            "horizontalHeaderLabels",
            crate::core::QVariant::List(vec![
                crate::core::QVariant::String("Col 1".into()),
                crate::core::QVariant::String("Col 2".into()),
            ]),
        );
        tree.set_property(
            table,
            "data",
            crate::core::QVariant::List(vec![
                crate::core::QVariant::List(vec![
                    crate::core::QVariant::String("A".into()),
                    crate::core::QVariant::String("B".into()),
                ]),
                crate::core::QVariant::List(vec![
                    crate::core::QVariant::String("C".into()),
                    crate::core::QVariant::String("D".into()),
                ]),
            ]),
        );
        tree.layout(root, tree.widget(root).unwrap().geometry);

        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Col 1"));
        assert!(texts.contains(&"Col 2"));
        assert!(texts.contains(&"A"));
        assert!(texts.contains(&"D"));
    }

    #[test]
    fn paints_checkbox_combo_list_and_tree_values() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let checkbox = tree.find_widget("checkBox_2").unwrap();
        tree.set_property(checkbox, "checked", QVariant::Bool(true));
        let combo = tree.find_widget("comboBox_1").unwrap();
        tree.set_property(
            combo,
            "items",
            QVariant::List(vec![
                QVariant::String("Option 1".into()),
                QVariant::String("Option 2".into()),
                QVariant::String("Option 3".into()),
            ]),
        );
        tree.set_property(combo, "currentIndex", QVariant::Int(2));
        let list = tree.find_widget("listWidget").unwrap();
        tree.set_property(
            list,
            "items",
            QVariant::List(vec![
                QVariant::String("Item 1".into()),
                QVariant::String("Item 2".into()),
            ]),
        );
        tree.set_property(list, "currentIndex", QVariant::Int(1));
        let tree_widget = tree.find_widget("treeWidget").unwrap();
        tree.set_property(
            tree_widget,
            "headerLabels",
            QVariant::List(vec![QVariant::String("Column 1".into())]),
        );
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![QVariant::Map(vec![
                ("text".into(), QVariant::String("Root 1".into())),
                (
                    "columns".into(),
                    QVariant::List(vec![QVariant::String("Root Column".into())]),
                ),
            ])]),
        );
        tree.layout(root, tree.widget(root).unwrap().geometry);

        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Option 3"));
        assert!(texts.contains(&"Item 2"));
        assert!(texts.contains(&"Root 1  Root Column"));
        let check_lines = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::DrawLine { .. }))
            .count();
        assert!(check_lines >= 2);
    }

    #[test]
    fn clicking_table_selects_a_cell() {
        let (mut tree, root) = load_ui(TABLE_UI).expect("loads ui");
        let table = tree.find_widget("tableWidget").expect("table");
        tree.set_property(table, "rowCount", crate::core::QVariant::Int(2));
        tree.set_property(table, "columnCount", crate::core::QVariant::Int(2));
        tree.layout(root, tree.widget(root).unwrap().geometry);
        let geometry = tree.widget(table).unwrap().geometry;
        let point =
            crate::gui::QPointF::new(geometry.x + 14.0 + 5.0, geometry.y + QTABLE_HEADER_H + 5.0);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        let table = tree.table(table).expect("table");
        assert_eq!(table.current_row(), Some(0));
        assert_eq!(table.current_column(), Some(0));
        assert!(table.is_selected(0, 0));
    }

    #[test]
    fn button_text_is_centered() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let button = tree.find_widget("pushButton_1").unwrap();
        let geometry = tree.widget(button).unwrap().geometry;
        let metrics = QFontMetrics::new(&tree.widget(button).unwrap().font);
        let expected =
            geometry.x + (geometry.width - metrics.horizontal_advance("PushButton")) / 2.0;
        let command = painter.commands().iter().find_map(|command| match command {
            QPaintCommand::DrawText { rect, text, .. } if text == "PushButton" => Some(*rect),
            _ => None,
        });
        let rect = command.expect("button text");
        assert!((rect.x - expected).abs() < 0.5);
    }
}
