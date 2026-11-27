use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QPointF, QRectF};
use crate::style::{QControlElement, QStyle, QStyleOption};
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

    // Buttons sit inside the inset LineEdit frame (geometry − 1px chrome).
    let button = QRectF::new(
        frame.right() - QSPIN_BUTTON_W,
        frame.y + 2.0,
        (QSPIN_BUTTON_W - 2.0).max(1.0),
        (frame.height - 4.0).max(1.0),
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
    // Fusion probe (gallery/spin): button column shades ~254 → ~238 top→bottom.
    painter.fill_linear_gradient_rect(
        button,
        0.0,
        QColor::rgba8(254, 254, 254, 255),
        QColor::rgba8(238, 238, 238, 255),
    );
    painter.fill_rect(
        QRectF::new(button.x, button.y, 1.0, button.height),
        outline,
    );

    // Arrow paint area excludes the 1px divider so the tip sits in the
    // content column (Qt probe mid ≈ 340.5 with div=334 / right=348).
    let arrow_up = QRectF::new(up.x + 1.0, up.y, (up.width - 1.0).max(1.0), up.height);
    let arrow_down = QRectF::new(
        down.x + 1.0,
        down.y,
        (down.width - 1.0).max(1.0),
        down.height,
    );

    // Translucent ButtonText over the shaded column → solid ~90 gray (Fusion).
    let enabled_arrow = widget
        .palette
        .color(QPaletteRole::ButtonText)
        .with_alpha(0.63);
    let disabled_arrow = widget.palette.color(QPaletteRole::Mid);
    let up_arrow = if widget.enabled && spin.can_step_up() {
        enabled_arrow
    } else {
        disabled_arrow
    };
    let down_arrow = if widget.enabled && spin.can_step_down() {
        enabled_arrow
    } else {
        disabled_arrow
    };
    fill_arrow(painter, arrow_up, true, up_arrow);
    fill_arrow(painter, arrow_down, false, down_arrow);

    let text = spin.display_text();
    let metrics = QFontMetrics::new(&widget.font);
    let text_h = metrics.height();
    let text_color = if widget.enabled {
        widget.palette.color(QPaletteRole::Text)
    } else {
        QColor::rgba8(160, 160, 160, 255)
    };
    // Qt Fusion line-edit content is inset ~5px from the outer geometry.
    let text_x = frame.x + 5.0;
    let text_rect = QRectF::new(
        text_x,
        frame.y + ((frame.height - text_h) / 2.0).max(0.0),
        (frame.width - QSPIN_BUTTON_W - 9.0).max(1.0),
        text_h,
    );
    painter.draw_text(text_rect, &text, text_color);
    // Embedded QLineEdit caret (gallery/focus/168–169). Same vertical band as
    // `paint_line_edit_caret` (geometry +4 / −8); x follows spin text inset.
    if widget.has_focus && widget.enabled && widget.cursor_visible {
        let cursor = widget.cursor_position.min(text.chars().count());
        let prefix: String = text.chars().take(cursor).collect();
        let caret_x = (text_x + metrics.horizontal_advance(&prefix)).min(text_rect.right());
        let caret_y = frame.y + 4.0;
        let caret_h = (frame.height - 8.0).max(0.0);
        if caret_h > 0.0 {
            painter.fill_rect(QRectF::new(caret_x, caret_y, 1.0, caret_h), text_color);
        }
    }
    // Focus outline comes from LineEdit above (Highlight when FOCUSED).
    // Do not stroke_rect again — that stacked a second 1px ring (double thickness).
}

fn fill_arrow(painter: &mut QPainter, area: QRectF, up: bool, color: QColor) {
    // Fusion spin indicators: flat 7px base, 4px tall. Probe places the up
    // base on y≈113 and the down base on y≈119 inside a 25px spin.
    let width = 7.0_f32.min(area.width).min(area.height);
    let height = 4.0_f32.min(width);
    let cx = area.x + area.width / 2.0;
    let cy = if up {
        area.y + 5.5
    } else {
        area.y + area.height - 5.5
    };
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
    // Flat base row (Fusion probe: solid ~92 across 6px). Triangle AA alone
    // leaves this thinner than Qt. Use opaque gray — reusing the translucent
    // arrow color here would darken over the already-blended triangle.
    let base_fill = if color.alpha < 0.99 {
        QColor::rgba8(92, 92, 92, 255)
    } else {
        color
    };
    let base_y = if up {
        cy + height / 2.0 - 1.0
    } else {
        cy - height / 2.0
    };
    painter.fill_rect(QRectF::new(cx - 3.0, base_y, 6.0, 1.0), base_fill);
}
