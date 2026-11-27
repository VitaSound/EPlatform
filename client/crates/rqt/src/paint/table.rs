use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::item_model::{QSortOrder, QTABLE_HEADER_H, QTABLE_ROW_H};
use crate::qss::QStyleDeclarations;
use crate::style::QStyle;
use crate::widgets::QWidget;

use super::scroll::{
    paint_header_gradient, paint_header_section, paint_scroll_bar, paint_scroll_corner,
};

pub(crate) fn paint_table(
    widget: &QWidget,
    painter: &mut QPainter,
    style: &dyn QStyle,
    qss: &QStyleDeclarations,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    if widget.class_name != "QTableWidget" {
        return;
    }
    let Some(table) = widget.table.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let frame = qss.scroll_frame_width(1.0);
    let layout = table.scroll_area_with_frame(rect, &widget.font, frame);
    let viewport = layout.viewport;
    let left = viewport.x;
    let top = viewport.y;
    let right = viewport.right();
    let bottom = viewport.bottom();
    let header_h = table.header_height();
    let leaf_top = top + table.grouped_header.len() as f32 * QTABLE_HEADER_H;
    let row_header_w = table.row_header_width(&widget.font);
    let body_x = left + row_header_w;
    let body_top = top + header_h;
    let rows_h = (bottom - body_top).max(0.0);
    let content_h =
        (table.model.row_count() as f32 * QTABLE_ROW_H - table.scroll).clamp(0.0, rows_h);
    let content_bottom = body_top + content_h;
    let rows = table.visible_row_range(viewport.height);
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let column_left = |column: usize| body_x + table.model.column_offset(column) - table.horizontal;

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        top,
        (right - left).max(0.0),
        header_h,
    )));
    let row_header_rect = QRectF::new(left, top, row_header_w, header_h);
    paint_header_section(widget, painter, row_header_rect);
    if let Some(background) = header_qss.background {
        painter.fill_rect(row_header_rect, background);
    }
    // Grouped (upper) header rows.
    for (level_index, level) in table.grouped_header.iter().enumerate() {
        let y = top + level_index as f32 * QTABLE_HEADER_H;
        let mut column = 0usize;
        for span in level {
            let start = column;
            let end = (column + span.colspan.max(1)).min(table.model.column_count());
            column = end;
            let mut x0 = None;
            let mut width = 0.0;
            for col in start..end {
                if table.model.is_column_hidden(col) {
                    continue;
                }
                if x0.is_none() {
                    x0 = Some(column_left(col));
                }
                width += table.model.column_width(col);
            }
            let Some(x) = x0 else {
                continue;
            };
            let section = QRectF::new(x, y, width, QTABLE_HEADER_H);
            paint_header_section(widget, painter, section);
            if let Some(background) = header_qss.background {
                painter.fill_rect(section, background);
            }
        }
    }
    // Leaf header cells (single-row height at the bottom of the band).
    let mut header_x = body_x;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        let x = column_left(column);
        let section = QRectF::new(x, leaf_top, width, QTABLE_HEADER_H);
        paint_header_section(widget, painter, section);
        if let Some(background) = header_qss.background {
            painter.fill_rect(section, background);
        }
        header_x = x + width;
    }
    if header_x < right {
        painter.fill_rect(
            QRectF::new(header_x, top, (right - header_x).max(0.0), header_h),
            palette.color(QPaletteRole::Window),
        );
    }
    painter.restore();

    if body_top < bottom {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            left,
            body_top,
            (body_x - left).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        painter.fill_rect(
            QRectF::new(left, body_top, row_header_w, (bottom - body_top).max(0.0)),
            palette.color(QPaletteRole::Button),
        );
        for row in rows.clone() {
            let y = viewport.y + table.row_y(row);
            let row_top = y.max(body_top);
            let row_bottom = (y + QTABLE_ROW_H).min(bottom);
            if row_bottom > row_top {
                let section = QRectF::new(left, row_top, row_header_w, row_bottom - row_top);
                paint_header_gradient(widget, painter, section);
                if let Some(background) = header_qss.background {
                    painter.fill_rect(section, background);
                }
            }
        }
        // Right border of the row-number cells: Qt draws a 1 px line between
        // the vertical header and the cells (color 171), from the header's
        // bottom to the last row line (gallery/tables/33, /172).
        if content_h > 0.0 {
            painter.fill_rect(
                QRectF::new(body_x - 1.0, body_top - 1.0, 1.0, content_h),
                palette.color(QPaletteRole::Window).darker(140),
            );
        }
        painter.restore();
    }

    let mut focus_border: Option<(QRectF, QColor)> = None;
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        body_x,
        body_top,
        (right - body_x).max(0.0),
        (bottom - body_top).max(0.0),
    )));
    if let Some(background) = item_qss.background {
        for row in rows.clone() {
            let top_y = (viewport.y + table.row_y(row)).max(body_top);
            let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
            if bottom_y <= top_y {
                continue;
            }
            for column in 0..table.model.column_count() {
                if table.model.is_column_hidden(column) {
                    continue;
                }
                painter.fill_rect(
                    QRectF::new(
                        column_left(column),
                        top_y,
                        table.model.column_width(column),
                        bottom_y - top_y,
                    ),
                    background,
                );
            }
        }
    }
    for (row, column) in table.selection.selected_cells() {
        if row < rows.start
            || row >= rows.end
            || column >= table.model.column_count()
            || table.model.is_column_hidden(column)
        {
            continue;
        }
        let top_y = (viewport.y + table.row_y(row)).max(body_top);
        let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
        if bottom_y <= top_y {
            continue;
        }
        painter.fill_rect(
            QRectF::new(
                column_left(column),
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
                let top_y = (viewport.y + table.row_y(row)).max(body_top);
                let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
                if bottom_y > top_y {
                    let cell = QRectF::new(
                        column_left(column),
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
                        // Qt's current-cell frame stops one pixel above the
                        // grid line below the row: the bottom edge must stay
                        // as thick as the sides/top (tables/33 probe: frame
                        // rows 33..61, grid line 62).
                        QRectF::new(
                            cell.x,
                            cell.y,
                            (cell.width - 1.0).max(1.0),
                            (cell.height - 1.0).max(1.0),
                        ),
                        QColor::rgb(0xb1, 0xc9, 0xd7),
                    ));
                }
            }
        }
    }
    painter.restore();

    let grid = palette.color(QPaletteRole::Window).darker(120);
    let header_grid = palette.color(QPaletteRole::Window).darker(140);
    if content_h > 0.0 {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            body_x,
            body_top,
            (right - body_x).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        let mut horizontal: Vec<f32> = rows
            .clone()
            .filter_map(|row| {
                let y = viewport.y + table.row_y(row);
                (y >= body_top - 0.5 && y < content_bottom).then_some(y)
            })
            .collect();
        if let Some(last) = rows.clone().last() {
            horizontal.push(viewport.y + table.row_y(last) + QTABLE_ROW_H);
        }
        let last_x = if table.model.column_count() == 0 {
            body_x
        } else {
            column_left(table.model.column_count() - 1)
                + table.model.column_width(table.model.column_count() - 1)
        };
        for y in &horizontal {
            // Qt paints the separator on the row's top edge, i.e. one pixel
            // above the row's content; the first row's top edge is the
            // header's bottom line and is already painted there.
            let line_y = *y - 1.0;
            if line_y <= body_top - 1.0 {
                continue;
            }
            // Lines only span the cells: Qt draws nothing in the empty area
            // right of the last column (gallery/grid/113, tables/172).
            painter.fill_rect(
                QRectF::new(body_x, line_y, (last_x - body_x).max(0.0), 1.0),
                grid,
            );
        }
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            let x = column_left(column) - 1.0;
            painter.fill_rect(QRectF::new(x, body_top, 1.0, content_h), grid);
        }
        if table.model.column_count() > 0 {
            // The cells' right border sits on the last column boundary
            // (x - 1 like the internal borders); drawing at `last_x` left a
            // white gap next to the cell fill — a visible double line
            // (gallery/tables/172, /178).
            painter.fill_rect(QRectF::new(last_x - 1.0, body_top, 1.0, content_h), grid);
        }
        painter.restore();

        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            left,
            body_top,
            (body_x - left).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        for y in &horizontal {
            let line_y = *y - 1.0;
            if line_y <= body_top - 1.0 {
                continue;
            }
            painter.fill_rect(QRectF::new(left, line_y, row_header_w, 1.0), header_grid);
        }
        painter.restore();
    }

    if let Some((rect, color)) = focus_border {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            body_x,
            body_top,
            (right - body_x).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        painter.stroke_rounded_rect(rect, 1.0, 1.0, color);
        painter.restore();
    }

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        top,
        (right - left).max(0.0),
        header_h,
    )));
    // Qt draws the header's bottom line under the sections only: the empty
    // header area right of the last column stays clean (tables/34 probe:
    // the 171 line ends at the last section — 12..425 — and the rest is
    // window colour). The line also overwrites the separators' bottom pixel
    // so it stays continuous like Qt's (no 203 gaps at x125/225).
    let mut headers_end = left;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        headers_end = column_left(column) + table.model.column_width(column);
    }
    // Qt separates neighbouring header sections with one light line and
    // draws nothing at the gutter edge or after the last section: only the
    // internal boundaries (gallery/tables/172 has no header lines at all).
    let header_separator = palette.color(QPaletteRole::Window).darker(118);
    for column in 1..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let x = column_left(column) - 1.0;
        painter.fill_rect(QRectF::new(x, top, 1.0, header_h), header_separator);
    }
    painter.fill_rect(
        QRectF::new(left, body_top - 1.0, (headers_end - left).max(0.0), 1.0),
        header_grid,
    );

    // Disabled headers use Mid (gallery/states/155_disabled_table).
    let header_color = header_qss.color.unwrap_or_else(|| {
        if widget.enabled {
            palette.color(QPaletteRole::ButtonText)
        } else {
            palette.color(QPaletteRole::Mid)
        }
    });
    for (level_index, level) in table.grouped_header.iter().enumerate() {
        let y = top + level_index as f32 * QTABLE_HEADER_H;
        let mut column = 0usize;
        for span in level {
            let start = column;
            let end = (column + span.colspan.max(1)).min(table.model.column_count());
            column = end;
            let mut x0 = None;
            let mut width = 0.0;
            for col in start..end {
                if table.model.is_column_hidden(col) {
                    continue;
                }
                if x0.is_none() {
                    x0 = Some(column_left(col));
                }
                width += table.model.column_width(col);
            }
            let Some(x) = x0 else {
                continue;
            };
            if span.text.is_empty() {
                continue;
            }
            let label_width = metrics.horizontal_advance(&span.text);
            painter.draw_text(
                QRectF::new(
                    x + ((width - label_width) / 2.0).max(2.0),
                    y + ((QTABLE_HEADER_H - line_height) / 2.0).max(0.0),
                    (width - 4.0).max(1.0),
                    line_height,
                ),
                span.text.clone(),
                header_color,
            );
        }
    }
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        let x = column_left(column);
        let label = table
            .model
            .header(column)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| (column + 1).to_string());
        let label_width = metrics.horizontal_advance(&label);
        painter.draw_text(
            QRectF::new(
                x + ((width - label_width) / 2.0).max(2.0),
                leaf_top + ((QTABLE_HEADER_H - line_height) / 2.0).max(0.0),
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
                        x + width - 12.0,
                        leaf_top + ((QTABLE_HEADER_H - line_height) / 2.0).max(0.0),
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
    }
    painter.restore();

    let text_color = item_qss.color.unwrap_or_else(|| {
        if widget.enabled {
            palette.color(QPaletteRole::Text)
        } else {
            palette.color(QPaletteRole::Mid)
        }
    });
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        body_top,
        (body_x - left).max(0.0),
        (content_bottom - body_top).max(0.0),
    )));
    for row in rows.clone() {
        let raw_y = viewport.y + table.row_y(row);
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
    }
    painter.restore();

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        body_x,
        body_top,
        (right - body_x).max(0.0),
        (content_bottom - body_top).max(0.0),
    )));
    for row in rows {
        let raw_y = viewport.y + table.row_y(row);
        let row_bottom = (raw_y + QTABLE_ROW_H).min(content_bottom);
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            let width = table.model.column_width(column);
            let column_x = column_left(column);
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
                            (column_x + 3.0 + metrics.horizontal_advance(&editor.text))
                                .min(column_x + width - 2.0),
                            caret_y.max(body_top),
                            1.0,
                            caret_h,
                        ),
                        text_color,
                    );
                }
            }
        }
    }
    painter.restore();

    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, style, bar, &table.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, style, bar, &table.hbar, palette, widget.enabled);
    }
    if let Some(corner) = layout.corner {
        paint_scroll_corner(painter, corner, palette);
    }
    // Qt paints the widget frame above the scroll bars (probe on tables/179:
    // the 184 border stays visible over the arrow fill at x11 and x328);
    // the bars now reach the widget edge like Qt's, so redraw the frame.
    // A QSS `border` replaces the Fusion Mid chrome (qss/53_table).
    if qss.replaces_native_frame() {
        if let Some(width) = qss.border_width {
            if width > 0.0 {
                let color = qss
                    .border_color
                    .unwrap_or_else(|| palette.color(QPaletteRole::WindowText));
                if let Some(radius) = qss.border_radius {
                    painter.stroke_rounded_rect(widget.geometry, radius, width, color);
                } else {
                    painter.stroke_rect(widget.geometry, width, color);
                }
            }
        }
    } else {
        painter.stroke_rect(
            widget.geometry,
            1.0,
            palette.color(QPaletteRole::Mid),
        );
    }
}

