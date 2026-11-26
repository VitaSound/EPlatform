use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::qss::QStyleDeclarations;
use crate::widgets::QWidget;

use super::{qss_content_rect, qss_font, qss_frame_rect, widget_text};

/// Qt's focus indicators are style-dependent: Windows draws a dotted text
/// rectangle for check/radio and a dotted inner rectangle for buttons, while
/// Fusion fills the check/radio text area and keeps only the button frame.
/// EPlatform uses the dotted variant for check/radio/button so the focus is
/// obvious on its flat palette; line edits and combos keep the Fusion blue
/// frame.
pub(crate) fn paint_focus_indicators(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if !widget.has_focus || !widget.enabled {
        return;
    }
    let color = widget
        .palette
        .color(QPaletteRole::WindowText)
        .with_alpha(0.72);
    match widget.class_name.as_str() {
        "QPushButton" | "QToolButton" => {
            let frame = qss_frame_rect(widget, qss);
            paint_dotted_rect(
                painter,
                QRectF::new(
                    frame.x + 3.0,
                    frame.y + 3.0,
                    (frame.width - 6.0).max(1.0),
                    (frame.height - 6.0).max(1.0),
                ),
                color,
            );
        }
        "QCheckBox" | "QRadioButton" => {
            let Some(text) = widget_text(widget) else {
                return;
            };
            if text.is_empty() {
                return;
            }
            let content = qss_content_rect(widget, qss, (0.0, 0.0, 0.0, 0.0));
            let font = qss_font(widget, qss);
            let metrics = QFontMetrics::new(&font);
            let line_height = metrics.height();
            let text_width = metrics.horizontal_advance(&text);
            let text_x = content.x + widget.font.pixel_size + 6.0;
            let text_y = content.y + ((content.height - line_height) / 2.0).max(0.0);
            paint_dotted_rect(
                painter,
                QRectF::new(
                    text_x - 2.0,
                    text_y - 1.0,
                    text_width + 4.0,
                    line_height + 2.0,
                ),
                color,
            );
        }
        _ => {}
    }
}

pub(crate) fn paint_dotted_rect(painter: &mut QPainter, rect: QRectF, color: QColor) {
    let left = rect.x.floor();
    let top = rect.y.floor();
    let right = rect.right().floor();
    let bottom = rect.bottom().floor();
    if right <= left || bottom <= top {
        return;
    }
    let mut x = left;
    while x <= right {
        painter.fill_rect(QRectF::new(x, top, 1.0, 1.0), color);
        painter.fill_rect(QRectF::new(x, bottom, 1.0, 1.0), color);
        x += 2.0;
    }
    let mut y = top + 2.0;
    while y < bottom {
        painter.fill_rect(QRectF::new(left, y, 1.0, 1.0), color);
        painter.fill_rect(QRectF::new(right, y, 1.0, 1.0), color);
        y += 2.0;
    }
}

