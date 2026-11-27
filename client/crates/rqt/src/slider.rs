use crate::gui::{QPointF, QRectF};
use crate::scroll::QOrientation;
use crate::style::QPixelMetric;

/// Fusion `PM_SliderThickness` / `PM_SliderLength` (15).
pub const QSLIDER_THICKNESS: f32 = 15.0;
/// Fusion groove thickness (`QStyleHelper::dpiScaled(7)` at 96 DPI).
pub const QSLIDER_GROOVE: f32 = 7.0;
/// Qt `QSlider::sizeHint` long axis (~84).
pub const QSLIDER_HINT_LENGTH: f32 = 84.0;

/// `QSlider` range / orientation / interaction state.
#[derive(Clone, Debug, PartialEq)]
pub struct QSlider {
    pub orientation: QOrientation,
    pub minimum: i32,
    pub maximum: i32,
    pub value: i32,
    pub single_step: i32,
    pub page_step: i32,
    /// Matches Qt `QStyleOptionSlider::upsideDown` (vertical default true).
    pub upside_down: bool,
    pub pressed: bool,
}

impl Default for QSlider {
    fn default() -> Self {
        Self::new(QOrientation::Horizontal)
    }
}

impl QSlider {
    pub fn new(orientation: QOrientation) -> Self {
        Self {
            orientation,
            minimum: 0,
            maximum: 99,
            value: 0,
            single_step: 1,
            page_step: 10,
            // Qt: vertical sliders are upside-down so maximum sits at the top.
            upside_down: matches!(orientation, QOrientation::Vertical),
            pressed: false,
        }
    }

    pub fn set_orientation(&mut self, orientation: QOrientation) {
        self.orientation = orientation;
        self.upside_down = matches!(orientation, QOrientation::Vertical);
    }

    pub fn clamp_value(&mut self) {
        if self.maximum < self.minimum {
            self.maximum = self.minimum;
        }
        self.value = self.value.clamp(self.minimum, self.maximum);
    }

    pub fn set_value(&mut self, value: i32) -> bool {
        let value = value.clamp(self.minimum, self.maximum);
        let changed = self.value != value;
        self.value = value;
        changed
    }

    pub fn step_by(&mut self, steps: i32) -> bool {
        let step = self.single_step.max(1);
        self.set_value(self.value.saturating_add(steps.saturating_mul(step)))
    }

    pub fn page_by(&mut self, pages: i32) -> bool {
        let step = self.page_step.max(1);
        self.set_value(self.value.saturating_add(pages.saturating_mul(step)))
    }

    pub fn groove_rect(&self, rect: QRectF) -> QRectF {
        match self.orientation {
            QOrientation::Horizontal => {
                let y = rect.y + ((rect.height - QSLIDER_GROOVE) / 2.0).floor();
                QRectF::new(rect.x, y, rect.width, QSLIDER_GROOVE)
            }
            QOrientation::Vertical => {
                let x = rect.x + ((rect.width - QSLIDER_GROOVE) / 2.0).floor();
                QRectF::new(x, rect.y, QSLIDER_GROOVE, rect.height)
            }
        }
    }

    pub fn handle_length(&self) -> f32 {
        QSLIDER_THICKNESS
    }

    pub fn handle_rect(&self, rect: QRectF) -> QRectF {
        let len = self.handle_length();
        let pos = self.handle_position(rect);
        match self.orientation {
            QOrientation::Horizontal => {
                let y = rect.y + ((rect.height - QSLIDER_THICKNESS) / 2.0).floor();
                QRectF::new(rect.x + pos, y, len, QSLIDER_THICKNESS)
            }
            QOrientation::Vertical => {
                let x = rect.x + ((rect.width - QSLIDER_THICKNESS) / 2.0).floor();
                QRectF::new(x, rect.y + pos, QSLIDER_THICKNESS, len)
            }
        }
    }

    pub fn handle_position(&self, rect: QRectF) -> f32 {
        let track = self.track_length(rect);
        let len = self.handle_length();
        let movable = (track - len).max(0.0);
        let span = self.maximum - self.minimum;
        if movable <= 0.0 || span <= 0 {
            return 0.0;
        }
        let fraction = (self.value - self.minimum) as f32 / span as f32;
        let fraction = if self.upside_down {
            1.0 - fraction
        } else {
            fraction
        };
        (fraction * movable).round()
    }

    pub fn value_from_pos(&self, rect: QRectF, point: QPointF) -> i32 {
        let track = self.track_length(rect);
        let len = self.handle_length();
        let movable = (track - len).max(0.0);
        let span = self.maximum - self.minimum;
        if movable <= 0.0 || span <= 0 {
            return self.minimum;
        }
        let axis = match self.orientation {
            QOrientation::Horizontal => point.x - rect.x - len / 2.0,
            QOrientation::Vertical => point.y - rect.y - len / 2.0,
        };
        let fraction = (axis / movable).clamp(0.0, 1.0);
        let fraction = if self.upside_down {
            1.0 - fraction
        } else {
            fraction
        };
        let raw = self.minimum as f32 + fraction * span as f32;
        raw.round() as i32
    }

    pub fn set_value_from_pos(&mut self, rect: QRectF, point: QPointF) -> bool {
        self.set_value(self.value_from_pos(rect, point))
    }

    /// Active (highlighted) groove clip — from the minimum end up to the handle.
    pub fn active_groove_clip(&self, rect: QRectF) -> QRectF {
        let groove = self.groove_rect(rect);
        let handle = self.handle_rect(rect);
        match self.orientation {
            QOrientation::Horizontal => {
                if self.upside_down {
                    let x = handle.right();
                    QRectF::new(x, groove.y, (groove.right() - x).max(0.0), groove.height)
                } else {
                    QRectF::new(
                        groove.x,
                        groove.y,
                        (handle.x - groove.x).max(0.0),
                        groove.height,
                    )
                }
            }
            QOrientation::Vertical => {
                if self.upside_down {
                    let y = handle.bottom();
                    QRectF::new(groove.x, y, groove.width, (groove.bottom() - y).max(0.0))
                } else {
                    QRectF::new(
                        groove.x,
                        groove.y,
                        groove.width,
                        (handle.y - groove.y).max(0.0),
                    )
                }
            }
        }
    }

    pub fn size_hint(&self) -> QRectF {
        match self.orientation {
            QOrientation::Horizontal => QRectF::new(0.0, 0.0, QSLIDER_HINT_LENGTH, QSLIDER_THICKNESS),
            QOrientation::Vertical => QRectF::new(0.0, 0.0, QSLIDER_THICKNESS, QSLIDER_HINT_LENGTH),
        }
    }

    pub fn thickness_metric(metric: QPixelMetric) -> Option<f32> {
        match metric {
            QPixelMetric::SliderThickness | QPixelMetric::SliderLength => Some(QSLIDER_THICKNESS),
            _ => None,
        }
    }

    fn track_length(&self, rect: QRectF) -> f32 {
        match self.orientation {
            QOrientation::Horizontal => rect.width,
            QOrientation::Vertical => rect.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_handle_tracks_value() {
        let mut slider = QSlider::new(QOrientation::Horizontal);
        slider.maximum = 100;
        slider.set_value(40);
        let rect = QRectF::new(0.0, 0.0, 338.0, 15.0);
        let handle = slider.handle_rect(rect);
        assert_eq!(handle.width, 15.0);
        assert_eq!(handle.height, 15.0);
        assert_eq!(handle.x, 129.0);
        assert_eq!(handle.y, 0.0);
        let groove = slider.groove_rect(rect);
        assert_eq!(groove.y, 4.0);
        assert_eq!(groove.height, 7.0);
    }

    #[test]
    fn vertical_upside_down_puts_max_at_top() {
        let mut slider = QSlider::new(QOrientation::Vertical);
        slider.maximum = 100;
        let rect = QRectF::new(0.0, 0.0, 15.0, 258.0);
        slider.set_value(0);
        assert_eq!(slider.handle_rect(rect).y, 243.0);
        slider.set_value(100);
        assert_eq!(slider.handle_rect(rect).y, 0.0);
        slider.set_value(70);
        assert_eq!(slider.handle_rect(rect).y, 73.0);
    }
}
