use crate::gui::{QColor, QFont, QFontMetrics, QFontWeight, QPainter, QPaletteRole, QPointF, QRectF};
use crate::style::QStyle;
use crate::widgets::QWidget;

/// Paint Fusion / QCommonStyle `CE_ToolBoxTab` headers for a `QToolBox`.
/// Page widgets are painted as normal children when visible; headers are
/// chrome owned by the toolbox (like Qt's private `QToolBoxButton`s).
pub(crate) fn paint_toolbox(widget: &QWidget, _style: &dyn QStyle, painter: &mut QPainter) {
    let Some(toolbox) = widget.toolbox.as_deref() else {
        return;
    };
    let frame = widget.geometry;
    let header_h = QFontMetrics::new(&widget.font).height() + crate::widgets::QTOOLBOX_TAB_V_PAD;
    let page_count = widget.children.len().max(toolbox.labels.len());
    if page_count == 0 {
        return;
    }
    let current = toolbox.current_index.min(page_count.saturating_sub(1));
    let spacing = crate::widgets::QTOOLBOX_SPACING;
    let outline = widget.palette.color(QPaletteRole::Mid).darker(150);
    let highlight = widget.palette.color(QPaletteRole::Light);
    let text_color = if widget.enabled {
        widget.palette.color(QPaletteRole::ButtonText)
    } else {
        QColor::rgba8(160, 160, 160, 255)
    };

    let mut y = frame.y;
    for index in 0..page_count {
        let rect = QRectF::new(frame.x, y, frame.width, header_h);
        // Qt clips `CE_ToolBoxTabShape` to the button; keep the same clip so
        // the polygon's y=h+1 closing edge never paints into the page gap.
        painter.save();
        painter.set_clip_rect(Some(rect));
        paint_toolbox_tab_shape(painter, rect, outline, highlight);
        painter.restore();
        let label = toolbox_label(
            toolbox.labels.get(index).map(String::as_str).unwrap_or(""),
            index,
        );
        let selected = index == current;
        paint_toolbox_tab_label(painter, &widget.font, rect, &label, selected, text_color);
        y += header_h + spacing;
        if index == current {
            let after = page_count.saturating_sub(index + 1);
            let content_h =
                (frame.y + frame.height - y - after as f32 * (header_h + spacing)).max(0.0);
            y += content_h + if after > 0 { spacing } else { 0.0 };
        }
    }
}

fn paint_toolbox_tab_shape(
    painter: &mut QPainter,
    rect: QRectF,
    outline: QColor,
    highlight: QColor,
) {
    // QCommonStyle::CE_ToolBoxTabShape (LTR). Axis-aligned edges use 1px fills
    // (matches Qt's hard outline); only the diagonal uses `draw_line`.
    let w = rect.width;
    let h = rect.height;
    let x = rect.x;
    let y = rect.y;
    let d = 20.0 + h - 3.0;
    let top_w = (w - d).max(0.0);

    // Outline: top run + diagonal + short bottom step (right).
    painter.fill_rect(QRectF::new(x, y + 1.0, top_w, 1.0), outline);
    painter.draw_line(
        QPointF::new(x + w - d, y + 1.0),
        QPointF::new(x + w - 20.0, y + h - 2.0),
        1.0,
        outline,
    );
    painter.fill_rect(
        QRectF::new(x + w - 20.0, y + h - 2.0, 19.0_f32.max(0.0), 1.0),
        outline,
    );

    // Light highlight inset by 1px (QCommonStyle).
    painter.fill_rect(QRectF::new(x, y + 2.0, top_w, 1.0), highlight);
    painter.draw_line(
        QPointF::new(x + w - d - 1.0, y + 2.0),
        QPointF::new(x + w - 21.0, y + h - 1.0),
        1.0,
        highlight,
    );
    painter.fill_rect(
        QRectF::new(x + w - 20.0, y + h - 1.0, 20.0_f32.max(0.0), 1.0),
        highlight,
    );
}

fn paint_toolbox_tab_label(
    painter: &mut QPainter,
    base_font: &QFont,
    rect: QRectF,
    label: &str,
    selected: bool,
    color: QColor,
) {
    let mut font = base_font.clone();
    // Fusion: SH_ToolBox_SelectedPageTitleBold (real Bold face in assets).
    if selected {
        font.weight = QFontWeight::Bold;
    }
    let metrics = QFontMetrics::new(&font);
    let text_h = metrics.height();
    // SE_ToolBoxTabContents ≈ full rect; label inset matches QCommonStyle.
    let text_rect = QRectF::new(
        rect.x + 4.0,
        rect.y + ((rect.height - text_h) / 2.0).max(0.0),
        (rect.width - 12.0).max(1.0),
        text_h,
    );
    painter.set_font(font);
    painter.draw_text(text_rect, label, color);
    painter.set_font(base_font.clone());
}

fn toolbox_label(title: &str, index: usize) -> String {
    if title.is_empty() {
        format!("Page {}", index + 1)
    } else {
        title.to_string()
    }
}
