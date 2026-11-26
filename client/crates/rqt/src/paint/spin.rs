use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QPointF, QRectF};
use crate::style::{QControlElement, QStyle, QStyleOption, QStyleState};
use crate::widgets::{QWidget, QSPIN_BUTTON_W};

/// Paint a Fusion-like `QSpinBox` / `QDoubleSpinBox`: line-edit frame, value
/// text, and up/down buttons with triangle arrows.
pub(crate) fn paint_spin_box(
    widget: &QWidget,
    style: &dyn QStyle,
    option: &QStyleOption,
    painter: &mut QPainter,
) {
    let Some(spin) = widget.spin.as_deref() else {
        return;
    };
    let frame = widget.geometry;
    let mut frame_option = option.clone();
    frame_option.rect = frame;
    painter.draw_commands(&style.draw_control(QControlElement::LineEdit, &frame_option));

    let button = QRectF::new(
        frame.right() - QSPIN_BUTTON_W,
        frame.y + 1.0,
        (QSPIN_BUTTON_W - 1.0).max(1.0),
        (frame.height - 2.0).max(1.0),
    );
    let half = (button.height / 2.0).max(1.0);
    let up = QRectF::new(button.x, button.y, button.width, half);
    let down = QRectF::new(
        button.x,
        button.y + half,
        button.width,
        (button.height - half).max(1.0),
    );
    let window = widget.palette.color(QPaletteRole::Window);
    let outline = window.darker(140);
    let button_fill = widget.palette.color(QPaletteRole::Button);
    painter.fill_rect(up, button_fill.lighter(108));
    painter.fill_rect(down, button_fill.darker(102));
    painter.fill_rect(
        QRectF::new(button.x, button.y + half - 0.5, button.width, 1.0),
        outline,
    );
    painter.fill_rect(
        QRectF::new(button.x, button.y, 1.0, button.height),
        outline,
    );

    let arrow = if widget.enabled {
        widget
            .palette
            .color(QPaletteRole::ButtonText)
            .with_alpha(0.63)
    } else {
        widget.palette.color(QPaletteRole::Mid)
    };
    fill_arrow(painter, up, true, arrow);
    fill_arrow(painter, down, false, arrow);

    let text = spin.display_text();
    let metrics = QFontMetrics::new(&widget.font);
    let text_h = metrics.height();
    let text_color = if widget.enabled {
        widget.palette.color(QPaletteRole::Text)
    } else {
        QColor::rgba8(160, 160, 160, 255)
    };
    let text_rect = QRectF::new(
        frame.x + 4.0,
        frame.y + ((frame.height - text_h) / 2.0).max(0.0),
        (frame.width - QSPIN_BUTTON_W - 8.0).max(1.0),
        text_h,
    );
    painter.draw_text(text_rect, &text, text_color);

    if option.state.contains(QStyleState::FOCUSED) && widget.enabled {
        painter.stroke_rect(frame, 1.0, widget.palette.color(QPaletteRole::Highlight));
    }
}

fn fill_arrow(painter: &mut QPainter, area: QRectF, up: bool, color: QColor) {
    let width = 7.0_f32.min(area.width).min(area.height);
    let height = 4.5_f32.min(width);
    let cx = area.x + area.width / 2.0;
    let cy = area.y + area.height / 2.0;
    let (a, b, c) = if up {
        (
            QPointF::new(cx - width / 2.0, cy + height / 2.0),
            QPointF::new(cx + width / 2.0, cy + height / 2.0),
            QPointF::new(cx, cy - height / 2.0),
        )
    } else {
        (
            QPointF::new(cx - width / 2.0, cy - height / 2.0),
            QPointF::new(cx + width / 2.0, cy - height / 2.0),
            QPointF::new(cx, cy + height / 2.0),
        )
    };
    painter.fill_triangle(a, b, c, color);
}
