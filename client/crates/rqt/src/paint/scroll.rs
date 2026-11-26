use crate::gui::{
    QColor, QFont, QFontMetrics, QPainter, QPalette, QPaletteRole, QPointF, QRectF, QSizeF,
};
use crate::item_view::{QVIEW_BRANCH_W, QVIEW_HEADER_H, QVIEW_ROW_H};
use crate::qss::QStyleDeclarations;
use crate::scroll::QScrollBar;
use crate::style::{QComplexControl, QStyle, QStyleOption, QStyleState};
use crate::widgets::QWidget;

use super::focus::paint_dotted_rect;

pub(crate) fn paint_item_views(
    widget: &QWidget,
    painter: &mut QPainter,
    style: &dyn QStyle,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    match widget.class_name.as_str() {
        "QListWidget" => paint_list_widget(widget, painter, style, item_qss),
        "QTreeWidget" => paint_tree_widget(widget, painter, style, item_qss, header_qss),
        _ => {}
    }
}

fn paint_list_widget(
    widget: &QWidget,
    painter: &mut QPainter,
    style: &dyn QStyle,
    item_qss: &QStyleDeclarations,
) {
    let Some(list) = widget.list.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let layout = list.scroll_area(rect);
    let viewport = layout.viewport;
    // Disabled lists paint gray (gallery/lists/188_list_disabled).
    let list_fill = if widget.enabled {
        palette.color(QPaletteRole::Base)
    } else {
        palette.color(QPaletteRole::Window)
    };
    painter.fill_rect(rect, list_fill);
    painter.stroke_rect(rect, 1.0, palette.color(QPaletteRole::Mid));

    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let text_color = item_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(viewport));
    for index in visible_rows(list.scroll, viewport.height, list.items.len()) {
        let y = viewport.y + list.row_y(index);
        let selected = list.selected.contains(&index);
        let row_rect = QRectF::new(viewport.x, y, viewport.width, QVIEW_ROW_H);
        if selected {
            painter.fill_rect(
                row_rect,
                item_qss
                    .selection_background
                    .unwrap_or_else(|| palette.color(QPaletteRole::Highlight)),
            );
        } else if let Some(background) = item_qss.background {
            painter.fill_rect(row_rect, background);
        }
        if widget.has_focus && list.current == Some(index) {
            paint_dotted_rect(
                painter,
                QRectF::new(
                    row_rect.x + 1.0,
                    row_rect.y + 1.0,
                    (row_rect.width - 2.0).max(1.0),
                    (row_rect.height - 2.0).max(1.0),
                ),
                palette.color(QPaletteRole::WindowText).with_alpha(0.72),
            );
        }
        painter.draw_text(
            QRectF::new(
                viewport.x + 4.0 - list.horizontal,
                y + ((QVIEW_ROW_H - line_height) / 2.0).max(0.0),
                (list.content_width + 8.0).max(1.0),
                line_height,
            ),
            &list.items[index],
            if selected { selected_text } else { text_color },
        );
    }
    painter.restore();

    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, style, bar, &list.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, style, bar, &list.hbar, palette, widget.enabled);
    }
    if let Some(corner) = layout.corner {
        paint_scroll_corner(painter, corner, palette);
    }
    // Qt paints the widget frame above the scroll bars (probe on tables/179:
    // the 184 border stays visible over the arrow fill at x11 and x328);
    // the bars now reach the widget edge like Qt's, so redraw the frame.
    painter.stroke_rect(
        widget.geometry,
        1.0,
        palette.color(QPaletteRole::Mid),
    );
}

pub(crate) fn fusion_button_color(palette: &crate::gui::QPalette) -> QColor {
    let button = palette.color(QPaletteRole::Button);
    button
        .lighter(100 + ((180_i32 - button.q_gray() as i32) / 6).max(1))
        .with_saturation_factor(75)
}

/// Thin wrapper: scrollbar chrome lives in `QStyle::draw_complex_control`.
pub fn paint_scroll_bar(
    painter: &mut QPainter,
    style: &dyn QStyle,
    rect: QRectF,
    bar: &QScrollBar,
    palette: &QPalette,
    enabled: bool,
) {
    let mut option = QStyleOption::new(rect, palette.clone(), QFont::default());
    option.state = if enabled {
        QStyleState::ENABLED | QStyleState::ACTIVE
    } else {
        QStyleState::DISABLED
    };
    painter.draw_commands(&style.draw_complex_control(
        QComplexControl::ScrollBar,
        &option,
        bar,
    ));
}

/// Junction square where vertical and horizontal bars meet (Qt paints this
/// Window-colored with a darker top+left edge — those edges are the missing
/// outer strokes of the bottom/right arrow buttons on #098 / tables/183).
pub(crate) fn paint_scroll_corner(painter: &mut QPainter, rect: QRectF, palette: &QPalette) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let base = palette.color(QPaletteRole::Window);
    let border = base.darker(124);
    painter.fill_rect(rect, base);
    painter.fill_rect(QRectF::new(rect.x, rect.y, rect.width, 1.0), border);
    painter.fill_rect(QRectF::new(rect.x, rect.y, 1.0, rect.height), border);
}

pub(crate) fn paint_header_gradient(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    let base = fusion_button_color(&widget.palette);
    painter.fill_linear_gradient_rect(rect, 0.0, base.lighter(104), base.darker(102));
}

pub(crate) fn paint_header_section(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    // Qt's header gradient runs from 251 to 230 over the 20 visible rows
    // (probed on the offscreen renders: start Window lighter 105 %, end
    // darker 104 %); the bottom pixel is the section border.
    let base = widget.palette.color(QPaletteRole::Window);
    // Qt's section: a slow 251 → 236 gradient over the visible rows, then a
    // single 230 edge row above the 171 bottom border (probed y35..54).
    let gradient = QRectF::new(rect.x, rect.y, rect.width, (rect.height - 2.0).max(1.0));
    painter.fill_linear_gradient_rect(gradient, 0.0, base.lighter(105), base.darker(101));
    if rect.height >= 3.0 {
        painter.fill_rect(
            QRectF::new(rect.x, rect.bottom() - 2.0, rect.width, 1.0),
            base.darker(104),
        );
    }
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

fn paint_tree_widget(
    widget: &QWidget,
    painter: &mut QPainter,
    style: &dyn QStyle,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    let Some(tree) = widget.tree.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let layout = crate::scroll::scroll_area_layout(
        rect,
        QSizeF::new(tree.content_width, QVIEW_HEADER_H + tree.content_height()),
        crate::scroll::QScrollBarPolicy::AsNeeded,
        crate::scroll::QScrollBarPolicy::AsNeeded,
        crate::scroll::QSCROLLBAR_EXTENT,
    );
    let viewport = layout.viewport;
    let header = QRectF::new(viewport.x, viewport.y, viewport.width, QVIEW_HEADER_H);
    let body = QRectF::new(
        viewport.x,
        viewport.y + QVIEW_HEADER_H,
        viewport.width,
        (viewport.height - QVIEW_HEADER_H).max(0.0),
    );
    // Disabled trees paint gray (gallery/lists/189_tree_disabled).
    let tree_fill = if widget.enabled {
        palette.color(QPaletteRole::Base)
    } else {
        palette.color(QPaletteRole::Window)
    };
    painter.fill_rect(rect, tree_fill);
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let header_color = header_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::ButtonText));
    painter.save();
    painter.set_clip_rect(Some(viewport));
    paint_header_section(widget, painter, header);
    if let Some(background) = header_qss.background {
        painter.fill_rect(header, background);
    }
    let headers = if tree.headers.is_empty() {
        vec!["1".to_string()]
    } else {
        tree.headers.clone()
    };
    painter.draw_text(
        QRectF::new(
            viewport.x + 4.0 - tree.horizontal,
            viewport.y + ((QVIEW_HEADER_H - line_height) / 2.0).max(0.0),
            (viewport.width + tree.horizontal).max(1.0),
            line_height,
        ),
        headers.join("  "),
        header_color,
    );
    painter.restore();

    let text_color = item_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(body));
    for index in visible_rows(tree.scroll, body.height, tree.rows.len()) {
        let row = &tree.rows[index];
        let y = body.y + tree.row_y(index);
        let selected = tree.selected.contains(&index);
        let row_rect = QRectF::new(body.x, y, body.width, QVIEW_ROW_H);
        if selected {
            painter.fill_rect(
                row_rect,
                item_qss
                    .selection_background
                    .unwrap_or_else(|| palette.color(QPaletteRole::Highlight)),
            );
        } else if let Some(background) = item_qss.background {
            painter.fill_rect(row_rect, background);
        }
        if widget.has_focus && tree.current == Some(index) {
            paint_dotted_rect(
                painter,
                QRectF::new(
                    row_rect.x + 1.0,
                    row_rect.y + 1.0,
                    (row_rect.width - 2.0).max(1.0),
                    (row_rect.height - 2.0).max(1.0),
                ),
                palette.color(QPaletteRole::WindowText).with_alpha(0.72),
            );
        }
        let text_x = body.x + 4.0 + row.indent() + QVIEW_BRANCH_W - tree.horizontal;
        if row.has_children {
            let expanded = tree
                .items
                .get(row.path[0])
                .and_then(|_| {
                    // Walk the path to read the item's expanded flag.
                    let mut items = &tree.items;
                    let mut item = None;
                    for index in &row.path {
                        item = items.get(*index);
                        items = &item?.children;
                    }
                    item
                })
                .is_some_and(|item| item.expanded);
            let center_x = body.x + 2.0 + row.indent() + 4.0 - tree.horizontal;
            let center_y = y + QVIEW_ROW_H / 2.0;
            let half = 3.0;
            let color = text_color.with_alpha(0.75);
            if expanded {
                painter.fill_triangle(
                    QPointF::new(center_x - half, center_y - half / 2.0),
                    QPointF::new(center_x + half, center_y - half / 2.0),
                    QPointF::new(center_x, center_y + half / 2.0 + 1.0),
                    color,
                );
            } else {
                painter.fill_triangle(
                    QPointF::new(center_x - half / 2.0, center_y - half),
                    QPointF::new(center_x - half / 2.0, center_y + half),
                    QPointF::new(center_x + half / 2.0 + 1.0, center_y),
                    color,
                );
            }
        }
        painter.draw_text(
            QRectF::new(
                text_x,
                y + ((QVIEW_ROW_H - line_height) / 2.0).max(0.0),
                (tree.content_width + 8.0).max(1.0),
                line_height,
            ),
            &row.display,
            if selected { selected_text } else { text_color },
        );
    }
    painter.restore();

    painter.stroke_rect(rect, 1.0, palette.color(QPaletteRole::Mid));
    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, style, bar, &tree.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, style, bar, &tree.hbar, palette, widget.enabled);
    }
    if let Some(corner) = layout.corner {
        paint_scroll_corner(painter, corner, palette);
    }
    // Qt paints the widget frame above the scroll bars (probe on tables/179:
    // the 184 border stays visible over the arrow fill at x11 and x328);
    // the bars now reach the widget edge like Qt's, so redraw the frame.
    painter.stroke_rect(
        widget.geometry,
        1.0,
        palette.color(QPaletteRole::Mid),
    );
}

fn visible_rows(scroll: f32, height: f32, count: usize) -> std::ops::Range<usize> {
    if height <= 0.0 || count == 0 {
        return 0..0;
    }
    let first = (scroll / QVIEW_ROW_H).floor().max(0.0) as usize;
    let visible = (height / QVIEW_ROW_H).ceil() as usize + 1;
    first.min(count)..(first + visible).min(count)
}

