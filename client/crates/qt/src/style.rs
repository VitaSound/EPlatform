use crate::gui::{QFont, QPalette, QRectF, QSizeF};

pub use crate::gui::QPaintCommand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QPrimitiveElement {
    Panel,
    Frame,
    FrameLineEdit,
    FrameDefaultButton,
    FrameFocusRect,
    IndicatorArrowDown,
    IndicatorCheckBox,
    IndicatorRadioButton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QControlElement {
    PushButton,
    LineEdit,
    Label,
    CheckBox,
    RadioButton,
    ComboBox,
    TableView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QComplexControl {
    ScrollBar,
    Slider,
    ComboBox,
    SpinBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QPixelMetric {
    ButtonMargin,
    ButtonDefaultIndicator,
    DefaultFrameWidth,
    FocusFrameWidth,
    ScrollBarExtent,
    TabBarHeight,
    MenuItemHeight,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QStyleState(u32);

impl QStyleState {
    pub const NONE: Self = Self(0);
    pub const ENABLED: Self = Self(1 << 0);
    pub const DISABLED: Self = Self(1 << 1);
    pub const ACTIVE: Self = Self(1 << 2);
    pub const SUNKEN: Self = Self(1 << 3);
    pub const RAISED: Self = Self(1 << 4);
    pub const SELECTED: Self = Self(1 << 5);
    pub const FOCUSED: Self = Self(1 << 6);
    pub const HOVER: Self = Self(1 << 7);
    pub const DEFAULT: Self = Self(1 << 8);

    pub const fn new(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

impl std::ops::BitOr for QStyleState {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QStyleOption {
    pub rect: QRectF,
    pub state: QStyleState,
    pub palette: QPalette,
    pub font: QFont,
}

impl QStyleOption {
    pub fn new(rect: QRectF, palette: QPalette, font: QFont) -> Self {
        Self {
            rect,
            state: QStyleState::ENABLED | QStyleState::ACTIVE,
            palette,
            font,
        }
    }
}

pub trait QStyle {
    fn pixel_metric(&self, metric: QPixelMetric) -> f32;
    fn size_from_contents(
        &self,
        contents_size: QSizeF,
        element: QControlElement,
        option: &QStyleOption,
    ) -> QSizeF;
    fn draw_primitive(
        &self,
        element: QPrimitiveElement,
        option: &QStyleOption,
    ) -> Vec<QPaintCommand>;
    fn draw_control(&self, element: QControlElement, option: &QStyleOption) -> Vec<QPaintCommand>;
}

#[derive(Clone, Debug)]
pub struct QFusionStyle {
    palette: QPalette,
}

impl Default for QFusionStyle {
    fn default() -> Self {
        Self::new(QPalette::fusion())
    }
}

impl QFusionStyle {
    pub fn new(palette: QPalette) -> Self {
        Self { palette }
    }

    pub fn palette(&self) -> &QPalette {
        &self.palette
    }
}

impl QStyle for QFusionStyle {
    fn pixel_metric(&self, metric: QPixelMetric) -> f32 {
        match metric {
            QPixelMetric::ButtonMargin => 6.0,
            QPixelMetric::ButtonDefaultIndicator => 1.0,
            QPixelMetric::DefaultFrameWidth => 1.0,
            QPixelMetric::FocusFrameWidth => 1.0,
            QPixelMetric::ScrollBarExtent => 12.0,
            QPixelMetric::TabBarHeight => 22.0,
            QPixelMetric::MenuItemHeight => 20.0,
        }
    }

    fn size_from_contents(
        &self,
        contents_size: QSizeF,
        element: QControlElement,
        option: &QStyleOption,
    ) -> QSizeF {
        match element {
            QControlElement::PushButton => QSizeF::new(
                contents_size.width + self.pixel_metric(QPixelMetric::ButtonMargin) * 2.0 + 16.0,
                contents_size.height.max(option.font.pixel_size + 12.0),
            ),
            QControlElement::LineEdit => QSizeF::new(
                contents_size.width.max(100.0),
                contents_size.height.max(option.font.pixel_size + 6.0),
            ),
            _ => contents_size,
        }
    }

    fn draw_primitive(
        &self,
        element: QPrimitiveElement,
        option: &QStyleOption,
    ) -> Vec<QPaintCommand> {
        let palette = &option.palette;
        match element {
            QPrimitiveElement::Panel => vec![QPaintCommand::FillRect {
                rect: option.rect,
                color: palette.color(crate::gui::QPaletteRole::Window),
            }],
            QPrimitiveElement::Frame | QPrimitiveElement::FrameLineEdit => {
                vec![QPaintCommand::StrokeRect {
                    rect: option.rect,
                    width: self.pixel_metric(QPixelMetric::DefaultFrameWidth),
                    color: palette.color(crate::gui::QPaletteRole::Mid),
                }]
            }
            QPrimitiveElement::FrameDefaultButton => vec![QPaintCommand::StrokeRect {
                rect: option.rect,
                width: self.pixel_metric(QPixelMetric::ButtonDefaultIndicator),
                color: palette.color(crate::gui::QPaletteRole::Dark),
            }],
            QPrimitiveElement::FrameFocusRect => vec![QPaintCommand::StrokeRect {
                rect: option.rect,
                width: self.pixel_metric(QPixelMetric::FocusFrameWidth),
                color: palette.color(crate::gui::QPaletteRole::Highlight),
            }],
            _ => Vec::new(),
        }
    }

    fn draw_control(&self, element: QControlElement, option: &QStyleOption) -> Vec<QPaintCommand> {
        let palette = &option.palette;
        match element {
            QControlElement::PushButton => {
                let enabled = option.state.contains(QStyleState::ENABLED);
                let has_focus = option.state.contains(QStyleState::FOCUSED);
                let is_default = option.state.contains(QStyleState::DEFAULT) && enabled;
                let hovered = option.state.contains(QStyleState::HOVER) && enabled;
                let down = option.state.contains(QStyleState::SUNKEN)
                    || option.state.contains(QStyleState::SELECTED);
                let outline = palette.color(crate::gui::QPaletteRole::Window).darker(140);
                let highlighted = palette
                    .color(crate::gui::QPaletteRole::Highlight)
                    .darker(125);
                let highlighted = if highlighted.value() > 160 {
                    highlighted.with_value(160)
                } else {
                    highlighted
                };
                let dark_outline = if has_focus || is_default {
                    highlighted
                } else {
                    outline
                };
                let base = palette
                    .color(crate::gui::QPaletteRole::Button)
                    .lighter(
                        100 + ((180_i32
                            - palette.color(crate::gui::QPaletteRole::Button).q_gray() as i32)
                            / 6)
                        .max(1),
                    )
                    .with_saturation_factor(75);
                let base = if is_default {
                    base.blend(highlighted.lighter(130), 90)
                } else {
                    base
                };
                let gradient_base = if hovered { base } else { base.darker(104) };
                let start = gradient_base.lighter(124);
                let stop = gradient_base.lighter(102);
                let r = QRectF::new(
                    option.rect.x,
                    option.rect.y,
                    option.rect.width.max(1.0),
                    option.rect.height.max(1.0),
                );
                let inner = QRectF::new(
                    r.x + 1.0,
                    r.y + 1.0,
                    (r.width - 2.0).max(1.0),
                    (r.height - 2.0).max(1.0),
                );
                vec![
                    if down {
                        QPaintCommand::FillRoundedRect {
                            rect: r,
                            radius: 2.0,
                            color: base.darker(110),
                        }
                    } else {
                        QPaintCommand::FillLinearGradientRect {
                            rect: r,
                            radius: 2.0,
                            start,
                            end: stop,
                        }
                    },
                    QPaintCommand::StrokeRoundedRect {
                        rect: r,
                        radius: 2.0,
                        width: 1.0,
                        color: if enabled {
                            dark_outline
                        } else {
                            dark_outline.lighter(115)
                        },
                    },
                    QPaintCommand::StrokeRoundedRect {
                        rect: inner,
                        radius: 2.0,
                        width: 1.0,
                        color: crate::gui::QColor::rgba8(255, 255, 255, 30),
                    },
                ]
            }
            QControlElement::LineEdit => vec![
                QPaintCommand::FillRect {
                    rect: option.rect,
                    color: palette.color(crate::gui::QPaletteRole::Base),
                },
                QPaintCommand::StrokeRoundedRect {
                    rect: option.rect,
                    radius: 2.0,
                    width: self.pixel_metric(QPixelMetric::DefaultFrameWidth),
                    color: if option.state.contains(QStyleState::FOCUSED) {
                        palette.color(crate::gui::QPaletteRole::Highlight)
                    } else {
                        palette.color(crate::gui::QPaletteRole::Mid)
                    },
                },
            ],
            QControlElement::Label => Vec::new(),
            QControlElement::TableView => vec![
                QPaintCommand::FillRect {
                    rect: option.rect,
                    color: palette.color(crate::gui::QPaletteRole::Base),
                },
                QPaintCommand::StrokeRect {
                    rect: option.rect,
                    width: self.pixel_metric(QPixelMetric::DefaultFrameWidth),
                    color: palette.color(crate::gui::QPaletteRole::Mid),
                },
            ],
            QControlElement::CheckBox | QControlElement::RadioButton => {
                let size = option
                    .font
                    .pixel_size
                    .min(option.rect.height - 4.0)
                    .max(9.0);
                let indicator = QRectF::new(
                    option.rect.x,
                    option.rect.y + (option.rect.height - size) / 2.0,
                    size,
                    size,
                );
                vec![
                    QPaintCommand::FillRect {
                        rect: indicator,
                        color: palette.color(crate::gui::QPaletteRole::Base),
                    },
                    QPaintCommand::StrokeRect {
                        rect: indicator,
                        width: 1.0,
                        color: palette.color(crate::gui::QPaletteRole::Window).darker(150),
                    },
                ]
            }
            QControlElement::ComboBox => {
                let enabled = option.state.contains(QStyleState::ENABLED);
                let hovered = option.state.contains(QStyleState::HOVER) && enabled;
                let has_focus = option.state.contains(QStyleState::FOCUSED);
                let outline = palette.color(crate::gui::QPaletteRole::Window).darker(140);
                let highlighted = palette
                    .color(crate::gui::QPaletteRole::Highlight)
                    .darker(125);
                let base = palette
                    .color(crate::gui::QPaletteRole::Button)
                    .lighter(
                        100 + ((180_i32
                            - palette.color(crate::gui::QPaletteRole::Button).q_gray() as i32)
                            / 6)
                        .max(1),
                    )
                    .with_saturation_factor(75);
                let gradient_base = if hovered { base } else { base.darker(104) };
                let start = gradient_base.lighter(124);
                let stop = gradient_base.lighter(102);
                let r = QRectF::new(
                    option.rect.x,
                    option.rect.y,
                    option.rect.width.max(1.0),
                    option.rect.height.max(1.0),
                );
                let inner = QRectF::new(
                    r.x + 1.0,
                    r.y + 1.0,
                    (r.width - 2.0).max(1.0),
                    (r.height - 2.0).max(1.0),
                );
                vec![
                    QPaintCommand::FillLinearGradientRect {
                        rect: r,
                        radius: 2.0,
                        start,
                        end: stop,
                    },
                    QPaintCommand::StrokeRoundedRect {
                        rect: r,
                        radius: 2.0,
                        width: 1.0,
                        color: if has_focus { highlighted } else { outline },
                    },
                    QPaintCommand::StrokeRoundedRect {
                        rect: inner,
                        radius: 2.0,
                        width: 1.0,
                        color: crate::gui::QColor::rgba8(255, 255, 255, 30),
                    },
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option() -> QStyleOption {
        QStyleOption::new(
            QRectF::new(1.0, 2.0, 80.0, 24.0),
            QPalette::fusion(),
            QFont::default(),
        )
    }

    #[test]
    fn fusion_pixel_metrics_match_qt_shape() {
        let style = QFusionStyle::default();
        assert_eq!(style.pixel_metric(QPixelMetric::DefaultFrameWidth), 1.0);
        assert_eq!(style.pixel_metric(QPixelMetric::ScrollBarExtent), 12.0);
        assert!(style.pixel_metric(QPixelMetric::ButtonMargin) > 0.0);
    }

    #[test]
    fn push_button_is_drawn_from_palette() {
        let style = QFusionStyle::default();
        let option = option();
        let commands = style.draw_control(QControlElement::PushButton, &option);
        assert!(matches!(
            commands[0],
            QPaintCommand::FillLinearGradientRect { .. }
        ));
        assert!(matches!(
            commands[1],
            QPaintCommand::StrokeRoundedRect { .. }
        ));
        assert!(matches!(
            commands[2],
            QPaintCommand::StrokeRoundedRect { .. }
        ));
    }

    #[test]
    fn labels_do_not_paint_a_background_by_default() {
        let style = QFusionStyle::default();
        assert!(style
            .draw_control(QControlElement::Label, &option())
            .is_empty());
    }

    #[test]
    fn style_state_is_a_bitmask() {
        assert!((QStyleState::ENABLED | QStyleState::FOCUSED).contains(QStyleState::FOCUSED));
        assert!(!QStyleState::ENABLED.contains(QStyleState::SUNKEN));
    }
}
