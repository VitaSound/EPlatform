use crate::gui::QPainter;
use crate::style::{QComplexControl, QComplexControlData, QStyle, QStyleOption, QStyleState};
use crate::widgets::QWidget;

/// Thin wrapper: slider chrome lives in `QStyle::draw_complex_control(CC_Slider)`.
pub(crate) fn paint_slider(
    widget: &QWidget,
    style: &dyn QStyle,
    option: &QStyleOption,
    painter: &mut QPainter,
) {
    let Some(slider) = widget.slider.as_deref() else {
        return;
    };
    let mut frame_option = option.clone();
    frame_option.rect = widget.geometry;
    frame_option.state = if widget.enabled {
        let mut state = QStyleState::ENABLED | QStyleState::ACTIVE;
        if widget.has_focus {
            state = state | QStyleState::FOCUSED;
        }
        if slider.pressed {
            state = state | QStyleState::SUNKEN;
        }
        state
    } else {
        QStyleState::DISABLED
    };
    painter.draw_commands(&style.draw_complex_control(
        QComplexControl::Slider,
        &frame_option,
        QComplexControlData::Slider(slider),
    ));
}
