use crate::gui::{QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::plain_text_edit::line_index_for_cursor;
use crate::qss::QStyleDeclarations;
use crate::style::{QStyle, QStyleOption};
use crate::widgets::QWidget;

use super::draw_text_line;
use super::qss_font;
use super::scroll::paint_scroll_bar;

/// Paint Fusion-like `QPlainTextEdit`: QAbstractScrollArea frame (Base + Mid
/// stroke), multiline text, selection, caret, optional scrollbars.
pub(crate) fn paint_plain_text_edit(
    widget: &QWidget,
    style: &dyn QStyle,
    _option: &QStyleOption,
    painter: &mut QPainter,
    qss: &QStyleDeclarations,
) {
    let Some(edit) = widget.plain_text.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let fill = if widget.enabled {
        widget.palette.color(QPaletteRole::Base)
    } else {
        widget.palette.color(QPaletteRole::Window)
    };
    painter.fill_rect(rect, fill);
    painter.stroke_rect(rect, 1.0, widget.palette.color(QPaletteRole::Mid));

    let font = qss_font(widget, qss);
    let layout = edit.scroll_area(rect, &font);
    let viewport = edit.framed_viewport(rect, &font);
    let doc = edit.document_rect(viewport);
    let metrics = QFontMetrics::new(&font);
    let line_h = metrics.height().max(1.0);
    let lines = edit.visual_lines(&font, viewport.width);
    let scroll_y = edit.scroll_y;
    let scroll_x = edit.hbar.value;
    let color = text_color_for_plain(widget, qss);
    let sel_bg = qss
        .selection_background
        .unwrap_or_else(|| widget.palette.color(QPaletteRole::Highlight));
    let sel_fg = qss
        .selection_color
        .unwrap_or_else(|| widget.palette.color(QPaletteRole::HighlightedText));

    let (sel_start, sel_end) = match widget.selection_anchor {
        Some(anchor) => {
            let len = edit.plain_text.chars().count();
            let a = anchor.min(widget.cursor_position).min(len);
            let b = anchor.max(widget.cursor_position).min(len);
            (a, b)
        }
        None => (0, 0),
    };

    painter.save();
    painter.set_clip_rect(Some(viewport));
    painter.set_font(font.clone());

    for (index, line) in lines.iter().enumerate() {
        let y = doc.y + index as f32 * line_h - scroll_y;
        if y + line_h < viewport.y || y > viewport.bottom() {
            continue;
        }
        let base_x = doc.x - scroll_x;
        let drawn_end = line.start + line.text.chars().count();

        if sel_start < sel_end && sel_start < drawn_end && sel_end > line.start {
            let local_start = sel_start.max(line.start).min(drawn_end);
            let local_end = sel_end.max(line.start).min(drawn_end);
            if local_start < local_end {
                let prefix: String = line
                    .text
                    .chars()
                    .take(local_start.saturating_sub(line.start))
                    .collect();
                let selected: String = line
                    .text
                    .chars()
                    .skip(local_start.saturating_sub(line.start))
                    .take(local_end.saturating_sub(local_start))
                    .collect();
                let x = base_x + metrics.horizontal_advance(&prefix);
                let w = metrics.horizontal_advance(&selected);
                if w > 0.0 {
                    painter.fill_rect(QRectF::new(x, y, w, line_h), sel_bg);
                }
            }
        }

        if line.text.is_empty() {
            continue;
        }

        if sel_start >= sel_end
            || sel_end <= line.start
            || sel_start >= drawn_end
        {
            draw_text_line(
                painter,
                base_x,
                y,
                doc.right(),
                line_h,
                line.text.clone(),
                color,
            );
            continue;
        }

        let local_start = sel_start.max(line.start).min(drawn_end);
        let local_end = sel_end.max(line.start).min(drawn_end);
        let prefix: String = line
            .text
            .chars()
            .take(local_start.saturating_sub(line.start))
            .collect();
        let selected: String = line
            .text
            .chars()
            .skip(local_start.saturating_sub(line.start))
            .take(local_end.saturating_sub(local_start))
            .collect();
        let suffix: String = line
            .text
            .chars()
            .skip(local_end.saturating_sub(line.start))
            .collect();
        let prefix_w = metrics.horizontal_advance(&prefix);
        let selected_w = metrics.horizontal_advance(&selected);
        if !prefix.is_empty() {
            draw_text_line(
                painter,
                base_x,
                y,
                doc.right(),
                line_h,
                prefix,
                color,
            );
        }
        if !selected.is_empty() {
            draw_text_line(
                painter,
                base_x + prefix_w,
                y,
                doc.right(),
                line_h,
                selected,
                sel_fg,
            );
        }
        if !suffix.is_empty() {
            draw_text_line(
                painter,
                base_x + prefix_w + selected_w,
                y,
                doc.right(),
                line_h,
                suffix,
                color,
            );
        }
    }

    if widget.has_focus && widget.enabled && widget.cursor_visible {
        let cursor = widget.cursor_position.min(edit.plain_text.chars().count());
        let line_index = line_index_for_cursor(&lines, cursor);
        let line = &lines[line_index];
        let offset = cursor.saturating_sub(line.start).min(line.text.chars().count());
        let prefix: String = line.text.chars().take(offset).collect();
        let caret_x = (doc.x - scroll_x + metrics.horizontal_advance(&prefix)).min(doc.right());
        let caret_y = doc.y + line_index as f32 * line_h - scroll_y;
        if caret_y + line_h >= viewport.y && caret_y <= viewport.bottom() {
            painter.fill_rect(QRectF::new(caret_x, caret_y, 1.0, line_h), color);
        }
    }

    painter.restore();

    if let Some(bar) = layout.vertical {
        paint_scroll_bar(
            painter,
            style,
            bar,
            &edit.vbar,
            &widget.palette,
            widget.enabled,
        );
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(
            painter,
            style,
            bar,
            &edit.hbar,
            &widget.palette,
            widget.enabled,
        );
    }
    // Frame above bars (same as item views).
    painter.stroke_rect(rect, 1.0, widget.palette.color(QPaletteRole::Mid));
}

fn text_color_for_plain(widget: &QWidget, qss: &QStyleDeclarations) -> crate::gui::QColor {
    if let Some(color) = qss.color {
        return color;
    }
    let color = widget.palette.color(QPaletteRole::Text);
    if widget.enabled {
        color
    } else {
        crate::gui::QColor::rgba8(160, 160, 160, 255)
    }
}

