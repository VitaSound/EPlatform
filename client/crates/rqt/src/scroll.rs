use crate::gui::{QPointF, QRectF, QSizeF};

pub const QSCROLLBAR_EXTENT: f32 = 14.0;
pub const QSCROLLBAR_SLIDER_MIN: f32 = 26.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QOrientation {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QScrollBarPolicy {
    #[default]
    AsNeeded,
    AlwaysOff,
    AlwaysOn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QScrollBarSubControl {
    SubLine,
    AddLine,
    SubPage,
    AddPage,
    First,
    Last,
    Slider,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QScrollBar {
    pub orientation: QOrientation,
    pub minimum: f32,
    pub maximum: f32,
    pub value: f32,
    pub page_step: f32,
    pub single_step: f32,
    pub policy: QScrollBarPolicy,
    pub hovered: Option<QScrollBarSubControl>,
    pub pressed: Option<QScrollBarSubControl>,
}

impl Default for QScrollBar {
    fn default() -> Self {
        Self::new(QOrientation::Vertical)
    }
}

impl QScrollBar {
    pub fn new(orientation: QOrientation) -> Self {
        Self {
            orientation,
            minimum: 0.0,
            maximum: 0.0,
            value: 0.0,
            page_step: 0.0,
            single_step: 1.0,
            policy: QScrollBarPolicy::AsNeeded,
            hovered: None,
            pressed: None,
        }
    }

    pub fn set_range(&mut self, minimum: f32, maximum: f32, page_step: f32) -> bool {
        let minimum = minimum.min(maximum);
        let maximum = maximum.max(minimum);
        let page_step = page_step.max(0.0);
        let changed =
            self.minimum != minimum || self.maximum != maximum || self.page_step != page_step;
        self.minimum = minimum;
        self.maximum = maximum;
        self.page_step = page_step;
        self.clamp_value();
        changed
    }

    pub fn is_needed(&self) -> bool {
        self.maximum > self.minimum
    }

    pub fn visible(&self) -> bool {
        match self.policy {
            QScrollBarPolicy::AlwaysOn => true,
            QScrollBarPolicy::AlwaysOff => false,
            QScrollBarPolicy::AsNeeded => self.is_needed(),
        }
    }

    pub fn set_value(&mut self, value: f32) -> bool {
        let value = value.clamp(self.minimum, self.maximum);
        let changed = self.value != value;
        self.value = value;
        changed
    }

    pub fn step_by(&mut self, steps: f32) -> bool {
        self.set_value(self.value + steps * self.single_step)
    }

    pub fn page_by(&mut self, pages: f32) -> bool {
        self.set_value(self.value + pages * self.page_step)
    }

    pub fn clamp_value(&mut self) {
        self.value = self.value.clamp(self.minimum, self.maximum);
    }

    pub fn groove_rect(&self, rect: QRectF) -> QRectF {
        let extent = self.cross_extent(rect);
        // The groove (and therefore the slider) sits inside the scrollbar's
        // frame: Qt insets it by 1 px on the cross axis top/left, while the
        // arrow buttons keep the full extent (gallery/tables/35).
        match self.orientation {
            QOrientation::Vertical => {
                let length = (rect.height - extent * 2.0).max(0.0);
                QRectF::new(rect.x + 1.0, rect.y + extent, (rect.width - 1.0).max(0.0), length)
            }
            QOrientation::Horizontal => {
                let length = (rect.width - extent * 2.0).max(0.0);
                QRectF::new(rect.x + extent, rect.y + 1.0, length, (rect.height - 1.0).max(0.0))
            }
        }
    }

    pub fn slider_length(&self, track_length: f32) -> f32 {
        let track_length = track_length.max(0.0);
        if !self.is_needed() || track_length <= 0.0 {
            return track_length;
        }
        let span = self.maximum - self.minimum;
        let raw = if self.page_step > 0.0 {
            track_length * self.page_step / (span + self.page_step)
        } else {
            QSCROLLBAR_SLIDER_MIN
        };
        raw.round()
            .clamp(QSCROLLBAR_SLIDER_MIN.min(track_length), track_length)
    }

    pub fn slider_rect(&self, rect: QRectF) -> QRectF {
        let groove = self.groove_rect(rect);
        let track = self.track_length(groove);
        let length = self.slider_length(track);
        let position = self.slider_position(track, length);
        match self.orientation {
            QOrientation::Vertical => {
                QRectF::new(groove.x, groove.y + position, groove.width, length)
            }
            QOrientation::Horizontal => {
                QRectF::new(groove.x + position, groove.y, length, groove.height)
            }
        }
    }

    pub fn sub_control_rect(&self, rect: QRectF, control: QScrollBarSubControl) -> QRectF {
        let extent = self.cross_extent(rect);
        let groove = self.groove_rect(rect);
        match (self.orientation, control) {
            (QOrientation::Vertical, QScrollBarSubControl::SubLine) => {
                QRectF::new(rect.x, rect.y, rect.width, extent)
            }
            (QOrientation::Vertical, QScrollBarSubControl::AddLine) => {
                QRectF::new(rect.x, rect.bottom() - extent, rect.width, extent)
            }
            (QOrientation::Vertical, QScrollBarSubControl::First) => {
                QRectF::new(rect.x, rect.y, rect.width, extent)
            }
            (QOrientation::Vertical, QScrollBarSubControl::Last) => {
                QRectF::new(rect.x, rect.bottom() - extent, rect.width, extent)
            }
            (QOrientation::Vertical, QScrollBarSubControl::SubPage) => {
                let slider = self.slider_rect(rect);
                QRectF::new(
                    groove.x,
                    groove.y,
                    groove.width,
                    (slider.y - groove.y).max(0.0),
                )
            }
            (QOrientation::Vertical, QScrollBarSubControl::AddPage) => {
                let slider = self.slider_rect(rect);
                let top = slider.bottom();
                QRectF::new(
                    groove.x,
                    top,
                    groove.width,
                    (groove.bottom() - top).max(0.0),
                )
            }
            (QOrientation::Vertical, QScrollBarSubControl::Slider) => self.slider_rect(rect),
            (QOrientation::Horizontal, QScrollBarSubControl::SubLine)
            | (QOrientation::Horizontal, QScrollBarSubControl::First) => {
                QRectF::new(rect.x, rect.y, extent, rect.height)
            }
            (QOrientation::Horizontal, QScrollBarSubControl::AddLine)
            | (QOrientation::Horizontal, QScrollBarSubControl::Last) => {
                QRectF::new(rect.right() - extent, rect.y, extent, rect.height)
            }
            (QOrientation::Horizontal, QScrollBarSubControl::SubPage) => {
                let slider = self.slider_rect(rect);
                QRectF::new(
                    groove.x,
                    groove.y,
                    (slider.x - groove.x).max(0.0),
                    groove.height,
                )
            }
            (QOrientation::Horizontal, QScrollBarSubControl::AddPage) => {
                let slider = self.slider_rect(rect);
                let left = slider.right();
                QRectF::new(
                    left,
                    groove.y,
                    (groove.right() - left).max(0.0),
                    groove.height,
                )
            }
            (QOrientation::Horizontal, QScrollBarSubControl::Slider) => self.slider_rect(rect),
        }
    }

    pub fn hit_test(&self, rect: QRectF, point: QPointF) -> Option<QScrollBarSubControl> {
        if !rect.contains(point) {
            return None;
        }
        if self.slider_rect(rect).contains(point) {
            return Some(QScrollBarSubControl::Slider);
        }
        let groove = self.groove_rect(rect);
        let extent = self.cross_extent(rect);
        match self.orientation {
            QOrientation::Vertical => {
                if point.y < groove.y {
                    Some(QScrollBarSubControl::SubLine)
                } else if point.y >= groove.bottom() {
                    Some(QScrollBarSubControl::AddLine)
                } else if point.y < groove.y + extent {
                    Some(QScrollBarSubControl::SubPage)
                } else if point.y < self.slider_rect(rect).y {
                    Some(QScrollBarSubControl::SubPage)
                } else {
                    Some(QScrollBarSubControl::AddPage)
                }
            }
            QOrientation::Horizontal => {
                if point.x < groove.x {
                    Some(QScrollBarSubControl::SubLine)
                } else if point.x >= groove.right() {
                    Some(QScrollBarSubControl::AddLine)
                } else if point.x < self.slider_rect(rect).x {
                    Some(QScrollBarSubControl::SubPage)
                } else {
                    Some(QScrollBarSubControl::AddPage)
                }
            }
        }
    }

    pub fn value_from_pos(&self, rect: QRectF, point: QPointF) -> f32 {
        let groove = self.groove_rect(rect);
        let track = self.track_length(groove);
        let length = self.slider_length(track);
        let movable = (track - length).max(0.0);
        let span = self.maximum - self.minimum;
        if movable <= 0.0 || span <= 0.0 {
            return self.minimum;
        }
        let position = match self.orientation {
            QOrientation::Vertical => point.y - groove.y - length / 2.0,
            QOrientation::Horizontal => point.x - groove.x - length / 2.0,
        };
        (self.minimum + position.clamp(0.0, movable) / movable * span)
            .clamp(self.minimum, self.maximum)
    }

    pub fn set_value_from_pos(&mut self, rect: QRectF, point: QPointF) -> bool {
        self.set_value(self.value_from_pos(rect, point))
    }

    fn track_length(&self, groove: QRectF) -> f32 {
        match self.orientation {
            QOrientation::Vertical => groove.height,
            QOrientation::Horizontal => groove.width,
        }
    }

    fn slider_position(&self, track: f32, length: f32) -> f32 {
        let movable = (track - length).max(0.0);
        let span = self.maximum - self.minimum;
        if movable <= 0.0 || span <= 0.0 {
            return 0.0;
        }
        ((self.value - self.minimum) / span * movable).round()
    }

    fn cross_extent(&self, rect: QRectF) -> f32 {
        match self.orientation {
            QOrientation::Vertical => rect.width.max(0.0),
            QOrientation::Horizontal => rect.height.max(0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QScrollAreaLayout {
    pub viewport: QRectF,
    pub vertical: Option<QRectF>,
    pub horizontal: Option<QRectF>,
    pub corner: Option<QRectF>,
}

pub fn scroll_area_layout(
    rect: QRectF,
    content: QSizeF,
    vertical_policy: QScrollBarPolicy,
    horizontal_policy: QScrollBarPolicy,
    extent: f32,
) -> QScrollAreaLayout {
    let extent = extent.max(0.0);
    let mut vertical = matches!(vertical_policy, QScrollBarPolicy::AlwaysOn);
    let mut horizontal = matches!(horizontal_policy, QScrollBarPolicy::AlwaysOn);

    for _ in 0..3 {
        let inner_width = (rect.width - if vertical { extent } else { 0.0 }).max(0.0);
        let inner_height = (rect.height - if horizontal { extent } else { 0.0 }).max(0.0);
        let next_vertical = match vertical_policy {
            QScrollBarPolicy::AlwaysOn => true,
            QScrollBarPolicy::AlwaysOff => false,
            QScrollBarPolicy::AsNeeded => content.height > inner_height,
        };
        let next_horizontal = match horizontal_policy {
            QScrollBarPolicy::AlwaysOn => true,
            QScrollBarPolicy::AlwaysOff => false,
            QScrollBarPolicy::AsNeeded => content.width > inner_width,
        };
        if next_vertical == vertical && next_horizontal == horizontal {
            break;
        }
        vertical = next_vertical;
        horizontal = next_horizontal;
    }

    let viewport = QRectF::new(
        rect.x,
        rect.y,
        (rect.width - if vertical { extent } else { 0.0 }).max(0.0),
        (rect.height - if horizontal { extent } else { 0.0 }).max(0.0),
    );
    let vertical = vertical.then(|| {
        // Qt lays the bar out over the full widget height (probe on
        // tables/181: bar rows 41..377 = the table's rect, while the inner
        // viewport starts at 42): the long axis of a bar reaches one pixel
        // beyond the inner rectangle on both ends.
        let y = rect.y - 1.0;
        let bottom = if horizontal {
            rect.bottom() - extent
        } else {
            rect.bottom() + 1.0
        };
        QRectF::new(
            rect.right() - extent,
            y,
            extent,
            (bottom - y).max(0.0),
        )
    });
    let horizontal = horizontal.then(|| {
        // Mirror of the vertical rule: the horizontal bar spans the full
        // widget width (tables/179 probe: bar columns 11..328 inclusive,
        // arrows framed at x25 — one pixel beyond the inner rect on each
        // side).
        let x = rect.x - 1.0;
        let right = if vertical.is_some() {
            rect.right() - extent
        } else {
            rect.right() + 1.0
        };
        QRectF::new(
            x,
            rect.bottom() - extent,
            (right - x).max(0.0),
            extent,
        )
    });
    let corner = (vertical.is_some() && horizontal.is_some()).then(|| {
        QRectF::new(
            rect.right() - extent,
            rect.bottom() - extent,
            extent,
            extent,
        )
    });

    QScrollAreaLayout {
        viewport,
        vertical,
        horizontal,
        corner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_needed_policy_hides_when_content_fits() {
        let rect = QRectF::new(0.0, 0.0, 200.0, 100.0);
        let layout = scroll_area_layout(
            rect,
            QSizeF::new(200.0, 100.0),
            QScrollBarPolicy::AsNeeded,
            QScrollBarPolicy::AsNeeded,
            QSCROLLBAR_EXTENT,
        );
        assert_eq!(layout.viewport, rect);
        assert!(layout.vertical.is_none());
        assert!(layout.horizontal.is_none());
        assert!(layout.corner.is_none());
    }

    #[test]
    fn always_on_reserves_both_extents() {
        let rect = QRectF::new(5.0, 7.0, 200.0, 100.0);
        let layout = scroll_area_layout(
            rect,
            QSizeF::new(10.0, 10.0),
            QScrollBarPolicy::AlwaysOn,
            QScrollBarPolicy::AlwaysOn,
            QSCROLLBAR_EXTENT,
        );
        assert_eq!(layout.viewport, QRectF::new(5.0, 7.0, 186.0, 86.0));
        // Bars reach one pixel beyond the inner rect on both ends of their
        // long axis, like Qt's (probe tables/179 and tables/181).
        assert_eq!(layout.vertical, Some(QRectF::new(191.0, 6.0, 14.0, 87.0)));
        assert_eq!(layout.horizontal, Some(QRectF::new(4.0, 93.0, 187.0, 14.0)));
        assert_eq!(layout.corner, Some(QRectF::new(191.0, 93.0, 14.0, 14.0)));
    }

    #[test]
    fn horizontal_scrollbar_can_force_vertical_one() {
        let rect = QRectF::new(0.0, 0.0, 200.0, 100.0);
        let layout = scroll_area_layout(
            rect,
            QSizeF::new(205.0, 100.0),
            QScrollBarPolicy::AsNeeded,
            QScrollBarPolicy::AsNeeded,
            QSCROLLBAR_EXTENT,
        );
        assert!(layout.horizontal.is_some());
        assert!(layout.vertical.is_some());
        assert_eq!(layout.viewport, QRectF::new(0.0, 0.0, 186.0, 86.0));
    }

    #[test]
    fn always_off_keeps_full_viewport() {
        let rect = QRectF::new(0.0, 0.0, 200.0, 100.0);
        let layout = scroll_area_layout(
            rect,
            QSizeF::new(500.0, 500.0),
            QScrollBarPolicy::AlwaysOff,
            QScrollBarPolicy::AlwaysOff,
            QSCROLLBAR_EXTENT,
        );
        assert_eq!(layout.viewport, rect);
        assert!(layout.vertical.is_none());
        assert!(layout.horizontal.is_none());
    }

    #[test]
    fn vertical_scrollbar_has_arrows_and_groove() {
        let bar = QScrollBar::new(QOrientation::Vertical);
        let rect = QRectF::new(10.0, 20.0, 12.0, 212.0);
        assert_eq!(
            bar.sub_control_rect(rect, QScrollBarSubControl::SubLine),
            QRectF::new(10.0, 20.0, 12.0, 12.0)
        );
        assert_eq!(
            bar.sub_control_rect(rect, QScrollBarSubControl::AddLine),
            QRectF::new(10.0, 220.0, 12.0, 12.0)
        );
        // The groove is inset 1 px on the cross axis (left edge) like Qt's
        // scrollbar frame; the arrows keep the full width.
        assert_eq!(bar.groove_rect(rect), QRectF::new(11.0, 32.0, 11.0, 188.0));
    }

    #[test]
    fn slider_length_and_position_follow_range() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        let rect = QRectF::new(0.0, 0.0, 12.0, 212.0);
        bar.set_range(0.0, 400.0, 100.0);
        assert_eq!(bar.slider_length(188.0), 38.0);
        assert_eq!(bar.slider_rect(rect), QRectF::new(1.0, 12.0, 11.0, 38.0));

        bar.set_value(200.0);
        assert_eq!(bar.slider_rect(rect), QRectF::new(1.0, 87.0, 11.0, 38.0));

        bar.set_value(400.0);
        assert_eq!(bar.slider_rect(rect), QRectF::new(1.0, 162.0, 11.0, 38.0));
    }

    #[test]
    fn slider_never_shrinks_below_minimum() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        bar.set_range(0.0, 10_000.0, 10.0);
        assert_eq!(bar.slider_length(188.0), QSCROLLBAR_SLIDER_MIN);
    }

    #[test]
    fn empty_range_slider_fills_groove() {
        let bar = QScrollBar::new(QOrientation::Vertical);
        let rect = QRectF::new(0.0, 0.0, 12.0, 212.0);
        assert_eq!(bar.slider_rect(rect), bar.groove_rect(rect));
        assert!(!bar.is_needed());
    }

    #[test]
    fn hit_test_maps_sub_controls() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        let rect = QRectF::new(0.0, 0.0, 12.0, 212.0);
        bar.set_range(0.0, 400.0, 100.0);
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 5.0)),
            Some(QScrollBarSubControl::SubLine)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 207.0)),
            Some(QScrollBarSubControl::AddLine)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 20.0)),
            Some(QScrollBarSubControl::Slider)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 100.0)),
            Some(QScrollBarSubControl::AddPage)
        );
        bar.set_value(400.0);
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 100.0)),
            Some(QScrollBarSubControl::SubPage)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(6.0, 180.0)),
            Some(QScrollBarSubControl::Slider)
        );
        assert_eq!(bar.hit_test(rect, QPointF::new(20.0, 100.0)), None);
    }

    #[test]
    fn horizontal_hit_test_maps_x_axis() {
        let mut bar = QScrollBar::new(QOrientation::Horizontal);
        let rect = QRectF::new(0.0, 0.0, 212.0, 12.0);
        bar.set_range(0.0, 400.0, 100.0);
        assert_eq!(
            bar.hit_test(rect, QPointF::new(5.0, 6.0)),
            Some(QScrollBarSubControl::SubLine)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(207.0, 6.0)),
            Some(QScrollBarSubControl::AddLine)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(20.0, 6.0)),
            Some(QScrollBarSubControl::Slider)
        );
        assert_eq!(
            bar.hit_test(rect, QPointF::new(100.0, 6.0)),
            Some(QScrollBarSubControl::AddPage)
        );
    }

    #[test]
    fn dragging_slider_center_sets_value() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        let rect = QRectF::new(0.0, 0.0, 12.0, 212.0);
        bar.set_range(0.0, 400.0, 100.0);
        bar.set_value_from_pos(rect, QPointF::new(6.0, 106.0));
        assert!((bar.value - 200.0).abs() <= 1.0);
    }

    #[test]
    fn value_step_page_and_clamp() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        bar.set_range(0.0, 100.0, 20.0);
        assert!(bar.step_by(5.0));
        assert_eq!(bar.value, 5.0);
        assert!(bar.page_by(2.0));
        assert_eq!(bar.value, 45.0);
        assert!(bar.page_by(100.0));
        assert_eq!(bar.value, 100.0);
        assert!(!bar.page_by(100.0));
        assert!(bar.set_value(-10.0));
        assert_eq!(bar.value, 0.0);
    }

    #[test]
    fn policies_control_visibility() {
        let mut bar = QScrollBar::new(QOrientation::Vertical);
        assert!(!bar.visible());
        bar.policy = QScrollBarPolicy::AlwaysOn;
        assert!(bar.visible());
        bar.set_range(0.0, 10.0, 5.0);
        bar.policy = QScrollBarPolicy::AlwaysOff;
        assert!(!bar.visible());
        bar.policy = QScrollBarPolicy::AsNeeded;
        assert!(bar.visible());
    }
}
