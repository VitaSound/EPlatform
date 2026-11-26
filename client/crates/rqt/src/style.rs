use crate::gui::{QColor, QFont, QPalette, QPaletteRole, QPointF, QRectF, QSizeF};
use crate::scroll::{QOrientation, QScrollBar, QScrollBarSubControl};

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
    ScrollBarSliderMin,
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
    /// Qt `QStyle::drawComplexControl`. `bar` supplies sub-control geometry
    /// and hover/press state (like `QStyleOptionSlider` + activeSubControls).
    fn draw_complex_control(
        &self,
        control: QComplexControl,
        option: &QStyleOption,
        bar: &QScrollBar,
    ) -> Vec<QPaintCommand>;
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

/// Integer-aligned indicator box inside a check/radio control rect.
fn indicator_box(option: &QStyleOption) -> QRectF {
    let size = option
        .font
        .pixel_size
        .min(option.rect.height - 4.0)
        .max(9.0)
        .round();
    QRectF::new(
        option.rect.x.round(),
        (option.rect.y + (option.rect.height - size) / 2.0).round(),
        size,
        size,
    )
}

/// Fusion indicator ink (`text.darker(120)` @ alpha) baked in sRGB onto Base
/// so the GPU linear blend matches QPainter's gamma-space composite.
fn fusion_indicator_ink(palette: &QPalette, alpha_u8: u8) -> QColor {
    let ink = palette.color(QPaletteRole::Text).darker(120);
    let bg = palette.color(QPaletteRole::Base);
    let a = alpha_u8 as f32 / 255.0;
    QColor::rgba(
        ink.red * a + bg.red * (1.0 - a),
        ink.green * a + bg.green * (1.0 - a),
        ink.blue * a + bg.blue * (1.0 - a),
        1.0,
    )
}

fn draw_indicator_radio(option: &QStyleOption) -> Vec<QPaintCommand> {
    let palette = &option.palette;
    let enabled = option.state.contains(QStyleState::ENABLED);
    let outer = if option.rect.width == option.rect.height {
        // Already an indicator box (from draw_control).
        QRectF::new(
            option.rect.x.round(),
            option.rect.y.round(),
            option.rect.width.round().max(9.0),
            option.rect.height.round().max(9.0),
        )
    } else {
        indicator_box(option)
    };
    let outline = if !enabled {
        palette.color(QPaletteRole::Mid)
    } else if option.state.contains(QStyleState::FOCUSED) {
        palette.color(QPaletteRole::Highlight).darker(110)
    } else {
        palette.color(QPaletteRole::Window).darker(150)
    };
    let mut commands = vec![
        QPaintCommand::FillEllipse {
            rect: outer,
            color: if enabled {
                palette.color(QPaletteRole::Base)
            } else {
                palette.color(QPaletteRole::Window)
            },
        },
        QPaintCommand::StrokeEllipse {
            rect: outer,
            width: 1.0,
            color: outline,
        },
    ];
    if option.state.contains(QStyleState::SELECTED) {
        // QFusionStyle::PE_IndicatorRadioButton — shared pixel center.
        let size = outer.width;
        let outline_radius = (size + ((size as i32 + 1) % 2) as f32) * 0.5 - 1.0;
        let mut inner = (outline_radius / 2.32 * 2.0).round().max(4.0);
        if (inner as i32) % 2 != (size as i32) % 2 {
            inner += 1.0;
        }
        let inset = ((size - inner) * 0.5).round();
        commands.push(QPaintCommand::FillEllipse {
            rect: QRectF::new(outer.x + inset, outer.y + inset, inner, inner),
            color: if enabled {
                fusion_indicator_ink(palette, 180)
            } else {
                palette.color(QPaletteRole::Mid)
            },
        });
    }
    commands
}

fn draw_indicator_checkbox(option: &QStyleOption) -> Vec<QPaintCommand> {
    let palette = &option.palette;
    let enabled = option.state.contains(QStyleState::ENABLED);
    let box_rect = if (option.rect.width - option.rect.height).abs() < 0.5 {
        QRectF::new(
            option.rect.x.round(),
            option.rect.y.round(),
            option.rect.width.round().max(9.0),
            option.rect.height.round().max(9.0),
        )
    } else {
        indicator_box(option)
    };
    let mut commands = vec![
        QPaintCommand::FillRect {
            rect: box_rect,
            color: if enabled {
                palette.color(QPaletteRole::Base)
            } else {
                palette.color(QPaletteRole::Window)
            },
        },
        QPaintCommand::StrokeRect {
            rect: box_rect,
            width: 1.0,
            color: palette.color(QPaletteRole::Window).darker(150),
        },
    ];
    if option.state.contains(QStyleState::SELECTED) {
        // QFusionStyle::PE_IndicatorCheckBox checkmark.
        let width = (box_rect.width - 1.0).max(1.0);
        let padding = 1.0 + width * 0.13;
        let pen_width = 1.5_f32.max(width * 0.13).min(width * 0.20);
        let color = if enabled {
            fusion_indicator_ink(palette, 210)
        } else {
            palette.color(QPaletteRole::Mid)
        };
        let x = box_rect.x - 0.8;
        let y = box_rect.y + 0.5;
        let first = QPointF::new(x + padding + width * 0.11, y + width * 0.47);
        let second = QPointF::new(x + width * 0.5, y + width - padding);
        let third = QPointF::new(x + width - padding, y + padding);
        commands.push(QPaintCommand::DrawLine {
            from: first,
            to: second,
            width: pen_width,
            color,
        });
        commands.push(QPaintCommand::DrawLine {
            from: second,
            to: third,
            width: pen_width,
            color,
        });
    }
    commands
}

fn draw_indicator_arrow_down(option: &QStyleOption) -> Vec<QPaintCommand> {
    let area = option.rect;
    let width = 8.0_f32.min(area.width).min(area.height);
    let height = 5.5_f32.min(width);
    let center_x = area.x + area.width / 2.0;
    let center_y = area.y + area.height / 2.0;
    let color = if option.state.contains(QStyleState::ENABLED) {
        option
            .palette
            .color(QPaletteRole::ButtonText)
            .with_alpha(0.63)
    } else {
        option.palette.color(QPaletteRole::Mid)
    };
    vec![QPaintCommand::FillTriangle {
        a: QPointF::new(center_x - width / 2.0, center_y - height / 2.0),
        b: QPointF::new(center_x + width / 2.0, center_y - height / 2.0),
        c: QPointF::new(center_x, center_y + height / 2.0),
        color,
    }]
}

fn push_fill(out: &mut Vec<QPaintCommand>, rect: QRectF, color: QColor) {
    if rect.width > 0.0 && rect.height > 0.0 {
        out.push(QPaintCommand::FillRect { rect, color });
    }
}

fn push_gradient(
    out: &mut Vec<QPaintCommand>,
    rect: QRectF,
    start: QColor,
    end: QColor,
    horizontal: bool,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    out.push(QPaintCommand::FillLinearGradientRect {
        rect,
        radius: 0.0,
        start,
        end,
        horizontal,
    });
}

fn scroll_arrow_commands(
    area: QRectF,
    orientation: QOrientation,
    up_or_left: bool,
    color: QColor,
) -> Vec<QPaintCommand> {
    let center_x = area.x + area.width / 2.0;
    let center_y = area.y + area.height / 2.0;
    let tri = |a: QPointF, b: QPointF, c: QPointF| QPaintCommand::FillTriangle { a, b, c, color };
    match orientation {
        QOrientation::Vertical => {
            let half = 3.0_f32.min(area.width / 3.0).max(1.0);
            let height = 3.0_f32.min(area.height / 3.0).max(1.0);
            if up_or_left {
                vec![tri(
                    QPointF::new(center_x - half, center_y + height / 2.0),
                    QPointF::new(center_x + half, center_y + height / 2.0),
                    QPointF::new(center_x, center_y - height / 2.0),
                )]
            } else {
                vec![tri(
                    QPointF::new(center_x - half, center_y - height / 2.0),
                    QPointF::new(center_x + half, center_y - height / 2.0),
                    QPointF::new(center_x, center_y + height / 2.0),
                )]
            }
        }
        QOrientation::Horizontal => {
            let half = 3.0_f32.min(area.height / 3.0).max(1.0);
            let width = 3.0_f32.min(area.width / 3.0).max(1.0);
            if up_or_left {
                vec![tri(
                    QPointF::new(center_x + width / 2.0, center_y - half),
                    QPointF::new(center_x + width / 2.0, center_y + half),
                    QPointF::new(center_x - width / 2.0, center_y),
                )]
            } else {
                vec![tri(
                    QPointF::new(center_x - width / 2.0, center_y - half),
                    QPointF::new(center_x - width / 2.0, center_y + half),
                    QPointF::new(center_x + width / 2.0, center_y),
                )]
            }
        }
    }
}

/// Fusion `CC_ScrollBar` — same probe fidelity as the former `paint_scroll_bar`.
fn draw_scroll_bar(option: &QStyleOption, bar: &QScrollBar) -> Vec<QPaintCommand> {
    let rect = option.rect;
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Vec::new();
    }
    let palette = &option.palette;
    let enabled = option.state.contains(QStyleState::ENABLED);
    let base = palette.color(QPaletteRole::Window);
    let disabled = palette.color(QPaletteRole::Mid);
    let border = if enabled { base.darker(124) } else { disabled };
    let arrow_color = if enabled {
        palette.color(QPaletteRole::ButtonText).with_alpha(0.70)
    } else {
        disabled
    };
    let mut out = Vec::new();
    push_fill(&mut out, rect, base);

    let groove = bar.groove_rect(rect);
    // Qt groove: light gradient 253→242 along the cross axis.
    match bar.orientation {
        QOrientation::Horizontal => {
            push_gradient(
                &mut out,
                groove,
                base.lighter(106),
                base.lighter(101),
                false,
            );
        }
        QOrientation::Vertical => {
            push_gradient(
                &mut out,
                groove,
                base.lighter(106),
                base.lighter(101),
                true,
            );
        }
    }

    let mut paint_arrow = |control: QScrollBarSubControl, up: bool| {
        let area = bar.sub_control_rect(rect, control);
        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }
        let pressed = bar.pressed == Some(control);
        let hovered = bar.hovered == Some(control);
        let fill = if !enabled {
            base
        } else if pressed {
            base.darker(112)
        } else if hovered {
            base.lighter(105)
        } else {
            base
        };
        match bar.orientation {
            QOrientation::Horizontal => {
                push_gradient(&mut out, area, fill.lighter(106), fill.lighter(101), false);
            }
            QOrientation::Vertical => {
                push_gradient(&mut out, area, fill.lighter(106), fill.lighter(101), true);
            }
        }
        out.extend(scroll_arrow_commands(
            area,
            bar.orientation,
            up,
            arrow_color,
        ));
        push_fill(
            &mut out,
            QRectF::new(area.x, area.y, area.width, 1.0),
            QColor::rgba8(255, 255, 255, 30),
        );
    };
    paint_arrow(QScrollBarSubControl::SubLine, true);
    paint_arrow(QScrollBarSubControl::AddLine, false);

    let page_fill = if enabled { base.darker(104) } else { base };
    for page in [QScrollBarSubControl::SubPage, QScrollBarSubControl::AddPage] {
        let area = bar.sub_control_rect(rect, page);
        if area.width <= 0.0 || area.height <= 0.0 {
            continue;
        }
        push_fill(&mut out, area, page_fill);
        if enabled
            && bar.orientation == QOrientation::Horizontal
            && area.height >= 4.0
            && area.width > 2.0
        {
            push_fill(
                &mut out,
                QRectF::new(area.x + 1.0, area.bottom() - 2.0, area.width - 2.0, 1.0),
                base.darker(105),
            );
            push_fill(
                &mut out,
                QRectF::new(area.x + 1.0, area.bottom() - 1.0, area.width - 2.0, 1.0),
                base.darker(106),
            );
        }
    }

    for control in [
        QScrollBarSubControl::SubLine,
        QScrollBarSubControl::AddLine,
    ] {
        let area = bar.sub_control_rect(rect, control);
        if area.width <= 0.0 || area.height <= 0.0 {
            continue;
        }
        let inner = match (bar.orientation, control) {
            (QOrientation::Horizontal, QScrollBarSubControl::SubLine) => {
                Some(QRectF::new(area.right(), area.y, 1.0, area.height))
            }
            (QOrientation::Horizontal, QScrollBarSubControl::AddLine) => {
                Some(QRectF::new(area.x - 1.0, area.y, 1.0, area.height))
            }
            (QOrientation::Vertical, QScrollBarSubControl::SubLine) => {
                Some(QRectF::new(area.x, area.bottom(), area.width, 1.0))
            }
            (QOrientation::Vertical, QScrollBarSubControl::AddLine) => {
                Some(QRectF::new(area.x, area.y - 1.0, area.width, 1.0))
            }
            _ => None,
        };
        if let Some(line) = inner {
            push_fill(&mut out, line, border);
        }
    }

    let slider = bar.slider_rect(rect);
    if slider.width <= 0.0 || slider.height <= 0.0 {
        return out;
    }
    let pressed = bar.pressed == Some(QScrollBarSubControl::Slider);
    let hovered = bar.hovered == Some(QScrollBarSubControl::Slider);
    if enabled && !pressed && !hovered {
        match bar.orientation {
            QOrientation::Horizontal => {
                push_gradient(
                    &mut out,
                    slider,
                    base.lighter(106),
                    base.lighter(101),
                    false,
                );
            }
            QOrientation::Vertical => {
                push_gradient(
                    &mut out,
                    slider,
                    base.lighter(106),
                    base.lighter(101),
                    true,
                );
            }
        }
    } else {
        let fill = if !enabled {
            base
        } else if pressed {
            base.darker(112)
        } else {
            base.lighter(106)
        };
        push_fill(&mut out, slider, fill);
    }
    match bar.orientation {
        QOrientation::Horizontal => {
            push_fill(
                &mut out,
                QRectF::new(slider.x, slider.y, 1.0, slider.height),
                border,
            );
            push_fill(
                &mut out,
                QRectF::new(slider.right() - 1.0, slider.y, 1.0, slider.height),
                border,
            );
        }
        QOrientation::Vertical => {
            push_fill(
                &mut out,
                QRectF::new(slider.x, slider.y, slider.width, 1.0),
                border,
            );
            push_fill(
                &mut out,
                QRectF::new(slider.x, slider.bottom() - 1.0, slider.width, 1.0),
                border,
            );
        }
    }
    match bar.orientation {
        QOrientation::Horizontal => {
            push_fill(
                &mut out,
                QRectF::new(rect.x, rect.y, rect.width, 1.0),
                border,
            );
        }
        QOrientation::Vertical => {
            push_fill(
                &mut out,
                QRectF::new(rect.x, rect.y, 1.0, rect.height),
                border,
            );
        }
    }
    out
}

impl QStyle for QFusionStyle {
    fn pixel_metric(&self, metric: QPixelMetric) -> f32 {
        match metric {
            QPixelMetric::ButtonMargin => 6.0,
            QPixelMetric::ButtonDefaultIndicator => 1.0,
            QPixelMetric::DefaultFrameWidth => 1.0,
            QPixelMetric::FocusFrameWidth => 1.0,
            QPixelMetric::ScrollBarExtent => 14.0,
            QPixelMetric::ScrollBarSliderMin => 26.0,
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
            QPrimitiveElement::IndicatorArrowDown => draw_indicator_arrow_down(option),
            QPrimitiveElement::IndicatorCheckBox => draw_indicator_checkbox(option),
            QPrimitiveElement::IndicatorRadioButton => draw_indicator_radio(option),
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
                            horizontal: false,
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
                    // Qt fills disabled line edits with the window colour
                    // instead of white (gallery/hbox/88_disabled_mix).
                    color: if option.state.contains(QStyleState::ENABLED) {
                        palette.color(crate::gui::QPaletteRole::Base)
                    } else {
                        palette.color(crate::gui::QPaletteRole::Window)
                    },
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
                    // Disabled views are gray, not white (gallery/lists/188,
                    // gallery/states/155_disabled_table).
                    color: if option.state.contains(QStyleState::ENABLED) {
                        palette.color(crate::gui::QPaletteRole::Base)
                    } else {
                        palette.color(crate::gui::QPaletteRole::Window)
                    },
                },
                QPaintCommand::StrokeRect {
                    rect: option.rect,
                    width: self.pixel_metric(QPixelMetric::DefaultFrameWidth),
                    color: palette.color(crate::gui::QPaletteRole::Mid),
                },
            ],
            QControlElement::CheckBox => {
                let mut indicator = option.clone();
                indicator.rect = indicator_box(option);
                self.draw_primitive(QPrimitiveElement::IndicatorCheckBox, &indicator)
            }
            QControlElement::RadioButton => {
                let mut indicator = option.clone();
                indicator.rect = indicator_box(option);
                self.draw_primitive(QPrimitiveElement::IndicatorRadioButton, &indicator)
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
                        horizontal: false,
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

    fn draw_complex_control(
        &self,
        control: QComplexControl,
        option: &QStyleOption,
        bar: &QScrollBar,
    ) -> Vec<QPaintCommand> {
        match control {
            QComplexControl::ScrollBar => draw_scroll_bar(option, bar),
            // Slider/ComboBox/SpinBox complex paint lands in later plans.
            QComplexControl::Slider
            | QComplexControl::ComboBox
            | QComplexControl::SpinBox => Vec::new(),
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
        assert_eq!(style.pixel_metric(QPixelMetric::ScrollBarExtent), 14.0);
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

    #[test]
    fn radio_indicator_is_ellipses_not_a_square() {
        let style = QFusionStyle::default();
        let mut option = option();
        option.state = option.state | QStyleState::SELECTED;
        let commands = style.draw_control(QControlElement::RadioButton, &option);
        assert!(
            commands.iter().any(|c| matches!(c, QPaintCommand::FillEllipse { .. })),
            "radio must draw a circle fill, not a square"
        );
        assert!(
            commands.iter().any(|c| matches!(c, QPaintCommand::StrokeEllipse { .. })),
            "radio must stroke a circle"
        );
        assert!(
            !commands.iter().any(|c| matches!(c, QPaintCommand::FillRect { .. })),
            "radio must not paint a square indicator"
        );
        // Checked → inner dot as a second fill ellipse.
        let fills = commands
            .iter()
            .filter(|c| matches!(c, QPaintCommand::FillEllipse { .. }))
            .count();
        assert_eq!(fills, 2);
    }

    #[test]
    fn checkbox_indicator_draws_square_and_checkmark_when_on() {
        let style = QFusionStyle::default();
        let mut option = option();
        option.state = option.state | QStyleState::SELECTED;
        let commands = style.draw_control(QControlElement::CheckBox, &option);
        assert!(matches!(commands[0], QPaintCommand::FillRect { .. }));
        assert!(matches!(commands[1], QPaintCommand::StrokeRect { .. }));
        assert!(
            commands
                .iter()
                .filter(|c| matches!(c, QPaintCommand::DrawLine { .. }))
                .count()
                >= 2
        );
    }

    #[test]
    fn unchecked_checkbox_has_no_checkmark_lines() {
        let style = QFusionStyle::default();
        let commands = style.draw_control(QControlElement::CheckBox, &option());
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, QPaintCommand::DrawLine { .. }))
        );
    }

    #[test]
    fn arrow_down_primitive_emits_a_triangle() {
        let style = QFusionStyle::default();
        let commands =
            style.draw_primitive(QPrimitiveElement::IndicatorArrowDown, &option());
        assert!(matches!(commands[0], QPaintCommand::FillTriangle { .. }));
    }

    #[test]
    fn scroll_bar_complex_control_emits_groove_and_arrows() {
        let style = QFusionStyle::default();
        let mut option = option();
        option.rect = QRectF::new(10.0, 10.0, 14.0, 200.0);
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        bar.set_range(0.0, 100.0, 20.0);
        bar.set_value(40.0);
        let commands =
            style.draw_complex_control(QComplexControl::ScrollBar, &option, &bar);
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, QPaintCommand::FillLinearGradientRect { .. })),
            "groove/handle use cross-axis gradients"
        );
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, QPaintCommand::FillTriangle { .. })),
            "arrow buttons draw triangles"
        );
        assert!(commands.len() > 8);
    }
}
