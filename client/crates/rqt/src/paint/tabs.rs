use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::style::{QPixelMetric, QStyle};
use crate::widgets::{QWidget, QTAB_H_SPACE};

/// Paint Fusion-like north tab bar + pane frame for a `QTabWidget`.
/// Pages are painted as normal children when visible (inset so this frame
/// stays visible — Qt's stacked widget sits 1–2px inside the outline).
pub(crate) fn paint_tab_widget(widget: &QWidget, style: &dyn QStyle, painter: &mut QPainter) {
    let Some(tabs) = widget.tabs.as_deref() else {
        return;
    };
    let frame = widget.geometry;
    let bar_h = style.pixel_metric(QPixelMetric::TabBarHeight);
    let palette = &widget.palette;
    let window = palette.color(QPaletteRole::Window);
    let base = palette.color(QPaletteRole::Base);
    // Fusion outline ≈ 171.
    let outline = window.darker(140);
    let tab_frame = base.darker(101);
    let tab_idle = window.darker(103);
    let text = palette.color(QPaletteRole::WindowText);

    let pane = QRectF::new(
        frame.x,
        frame.y + bar_h - 1.0,
        frame.width,
        (frame.height - bar_h + 1.0).max(0.0),
    );
    painter.fill_rect(pane, tab_frame);
    // Use 1px fills — `stroke_rect` is easy to clip/cover; Qt's frame is
    // a hard outline the page must not paint over (page is laid out inset).
    stroke_outline(painter, pane, outline);

    let metrics = QFontMetrics::new(&widget.font);
    let mut x = frame.x;
    let mut tab_rects = Vec::with_capacity(tabs.titles.len());
    for (index, title) in tabs.titles.iter().enumerate() {
        let label = tab_label(title, index);
        let width = (metrics.horizontal_advance(&label) + QTAB_H_SPACE).max(40.0);
        let selected = index == tabs.current_index;
        let y = if selected {
            frame.y
        } else {
            frame.y + 2.0
        };
        let height = if selected { bar_h } else { bar_h - 2.0 };
        let rect = QRectF::new(x, y, width, height);
        tab_rects.push((rect, selected, label));
        x += width - 1.0;
    }

    // Fills first, then borders, so shared edges stay dark.
    for (rect, selected, _) in &tab_rects {
        let fill = if *selected { tab_frame } else { tab_idle };
        painter.fill_rect(*rect, fill);
    }
    for (rect, selected, _) in &tab_rects {
        if *selected {
            painter.fill_rect(QRectF::new(rect.x, rect.y, rect.width, 1.0), outline);
            painter.fill_rect(QRectF::new(rect.x, rect.y, 1.0, rect.height), outline);
            painter.fill_rect(
                QRectF::new(rect.right() - 1.0, rect.y, 1.0, rect.height),
                outline,
            );
            // Open into the pane.
            painter.fill_rect(
                QRectF::new(rect.x + 1.0, pane.y, (rect.width - 2.0).max(0.0), 1.0),
                tab_frame,
            );
        } else {
            stroke_outline(painter, *rect, outline);
        }
    }

    for (rect, _, label) in &tab_rects {
        let text_h = metrics.height();
        painter.draw_text(
            QRectF::new(
                rect.x + 4.0,
                rect.y + ((rect.height - text_h) / 2.0).max(0.0),
                (rect.width - 8.0).max(1.0),
                text_h,
            ),
            label,
            if widget.enabled {
                text
            } else {
                QColor::rgba8(160, 160, 160, 255)
            },
        );
    }
}

fn stroke_outline(painter: &mut QPainter, rect: QRectF, color: QColor) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    painter.fill_rect(QRectF::new(rect.x, rect.y, rect.width, 1.0), color);
    painter.fill_rect(
        QRectF::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0),
        color,
    );
    painter.fill_rect(QRectF::new(rect.x, rect.y, 1.0, rect.height), color);
    painter.fill_rect(
        QRectF::new(rect.right() - 1.0, rect.y, 1.0, rect.height),
        color,
    );
}

fn tab_label(title: &str, index: usize) -> String {
    if title.is_empty() {
        format!("Tab {}", index + 1)
    } else {
        title.to_string()
    }
}
