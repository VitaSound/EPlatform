use crate::gui::{QColor, QPainter, QPointF, QRectF, QSizeF};

pub const QMDI_TITLE_H: f32 = 24.0;
pub const QMDI_BUTTON: f32 = 18.0;
pub const QMDI_BUTTON_TOP: f32 = 3.0;
pub const QMDI_BUTTON_RIGHT: f32 = 6.0;
pub const QMDI_BUTTON_GAP: f32 = 2.0;
pub const QMDI_SYSMENU_LEFT: f32 = 5.0;
pub const QMDI_RESIZE: f32 = 12.0;
/// Width of the interactive resize border around a resizable subwindow.
pub const QMDI_BORDER: f32 = 4.0;
/// Size of the corner resize zones (the bottom-right grip uses `QMDI_RESIZE`).
pub const QMDI_CORNER: f32 = 8.0;
/// Fallback minimum content size when the form does not set `minimumSize`.
pub const QMDI_MIN_W: f32 = 80.0;
pub const QMDI_MIN_H: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QMdiSubWindowHit {
    Title,
    SystemMenu,
    Minimize,
    Maximize,
    Close,
    Body,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
}

impl QMdiSubWindowHit {
    /// True for every interactive resize border/corner.
    pub fn is_resize(self) -> bool {
        matches!(
            self,
            Self::ResizeTop
                | Self::ResizeBottom
                | Self::ResizeLeft
                | Self::ResizeRight
                | Self::ResizeTopLeft
                | Self::ResizeTopRight
                | Self::ResizeBottomLeft
                | Self::ResizeBottomRight
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QMdiSubWindowButton {
    SystemMenu,
    Minimize,
    Maximize,
    Restore,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QMdiWindowFlags {
    pub system_menu: bool,
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
}

impl Default for QMdiWindowFlags {
    fn default() -> Self {
        Self {
            system_menu: false,
            minimize: true,
            maximize: true,
            close: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QMdiSubWindow<T> {
    pub payload: T,
    pub title: String,
    pub frame: QRectF,
    pub maximized: bool,
    pub minimized: bool,
    pub visible: bool,
    pub window_flags: QMdiWindowFlags,
    /// Whether the user may resize the subwindow (grip/borders).
    pub resizable: bool,
    /// Form-provided content minimum; `0` means the QMDI_MIN_* fallback.
    pub min_content: QSizeF,
    /// Form-provided content maximum; values `<= 0` mean unbounded.
    pub max_content: QSizeF,
    restore_frame: Option<QRectF>,
}

impl<T> QMdiSubWindow<T> {
    pub fn content_rect(&self) -> QRectF {
        let height = if self.minimized {
            0.0
        } else {
            (self.frame.height - QMDI_TITLE_H).max(0.0)
        };
        QRectF::new(
            self.frame.x,
            self.frame.y + QMDI_TITLE_H,
            self.frame.width,
            height,
        )
    }

    fn title_button_rect(&self, offset: f32) -> QRectF {
        QRectF::new(
            self.frame.right() - QMDI_BUTTON_RIGHT - QMDI_BUTTON - offset,
            self.frame.y + QMDI_BUTTON_TOP,
            QMDI_BUTTON,
            QMDI_BUTTON,
        )
    }

    pub fn system_menu_rect(&self) -> QRectF {
        QRectF::new(
            self.frame.x + QMDI_SYSMENU_LEFT,
            self.frame.y + QMDI_BUTTON_TOP,
            QMDI_BUTTON,
            QMDI_BUTTON,
        )
    }

    pub fn close_rect(&self) -> QRectF {
        self.title_button_rect(0.0)
    }

    pub fn maximize_rect(&self) -> QRectF {
        self.title_button_rect(QMDI_BUTTON + QMDI_BUTTON_GAP)
    }

    pub fn minimize_rect(&self) -> QRectF {
        self.title_button_rect((QMDI_BUTTON + QMDI_BUTTON_GAP) * 2.0)
    }

    pub fn title_buttons(&self) -> Vec<(QMdiSubWindowButton, QRectF)> {
        let mut buttons = Vec::new();
        let mut offset = 0.0;
        if self.window_flags.close {
            buttons.push((QMdiSubWindowButton::Close, self.title_button_rect(offset)));
            offset += QMDI_BUTTON + QMDI_BUTTON_GAP;
        }
        if !self.minimized {
            if self.window_flags.maximize {
                let button = if self.maximized {
                    QMdiSubWindowButton::Restore
                } else {
                    QMdiSubWindowButton::Maximize
                };
                buttons.push((button, self.title_button_rect(offset)));
                offset += QMDI_BUTTON + QMDI_BUTTON_GAP;
            }
            if self.window_flags.minimize {
                buttons.push((
                    QMdiSubWindowButton::Minimize,
                    self.title_button_rect(offset),
                ));
            }
        } else if self.window_flags.minimize {
            buttons.push((QMdiSubWindowButton::Restore, self.title_button_rect(offset)));
        }
        if self.window_flags.system_menu {
            buttons.push((QMdiSubWindowButton::SystemMenu, self.system_menu_rect()));
        }
        buttons
    }

    pub fn resize_rect(&self) -> QRectF {
        QRectF::new(
            self.frame.right() - QMDI_RESIZE,
            self.frame.bottom() - QMDI_RESIZE,
            QMDI_RESIZE,
            QMDI_RESIZE,
        )
    }

    /// Effective content minimum: explicit form value or the UI fallback.
    pub fn min_content_size(&self) -> QSizeF {
        QSizeF::new(
            if self.min_content.width > 0.0 {
                self.min_content.width
            } else {
                QMDI_MIN_W
            },
            if self.min_content.height > 0.0 {
                self.min_content.height
            } else {
                QMDI_MIN_H
            },
        )
    }

    /// Effective content maximum; `0`/infinite values are unbounded.
    pub fn max_content_size(&self) -> QSizeF {
        let min = self.min_content_size();
        let unbounded = |value: f32| value <= 0.0 || !value.is_finite();
        QSizeF::new(
            if unbounded(self.max_content.width) {
                f32::MAX
            } else {
                self.max_content.width.max(min.width)
            },
            if unbounded(self.max_content.height) {
                f32::MAX
            } else {
                self.max_content.height.max(min.height)
            },
        )
    }

    /// Interactive resizing is available only for visible, non-maximized
    /// windows with a non-degenerate size range.
    pub fn can_resize(&self) -> bool {
        if !self.visible || !self.resizable || self.maximized || self.minimized {
            return false;
        }
        let min = self.min_content_size();
        let max = self.max_content_size();
        max.width > min.width || max.height > min.height
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    /// Apply form-level `minimumSize`/`maximumSize` (content size) and clamp
    /// the current frame. Dragging a border keeps the opposite edge anchored;
    /// this method only enforces the size range.
    pub fn set_size_constraints(&mut self, min: QSizeF, max: QSizeF) {
        self.min_content = min;
        self.max_content = max;
        if let Some(frame) = self.restore_frame {
            self.restore_frame = Some(self.constrained_frame(frame));
        }
        if !self.maximized && !self.minimized {
            self.frame = self.constrained_frame(self.frame);
        }
    }

    fn constrained_frame(&self, frame: QRectF) -> QRectF {
        let min = self.min_content_size();
        let max = self.max_content_size();
        let width = frame.width.clamp(min.width, max.width);
        let height = frame
            .height
            .clamp(min.height + QMDI_TITLE_H, max.height + QMDI_TITLE_H);
        QRectF::new(frame.x, frame.y, width, height)
    }

    fn resize_hit(&self, point: QPointF) -> Option<QMdiSubWindowHit> {
        if !self.can_resize() || !self.frame.contains(point) {
            return None;
        }
        let frame = self.frame;
        // The visible grip is intentionally larger than the normal corners.
        if point.x >= frame.right() - QMDI_RESIZE && point.y >= frame.bottom() - QMDI_RESIZE {
            return Some(QMdiSubWindowHit::ResizeBottomRight);
        }
        let corner_l = point.x < frame.left() + QMDI_CORNER;
        let corner_r = point.x >= frame.right() - QMDI_CORNER;
        let corner_t = point.y < frame.top() + QMDI_CORNER;
        let corner_b = point.y >= frame.bottom() - QMDI_CORNER;
        if corner_t && corner_l {
            return Some(QMdiSubWindowHit::ResizeTopLeft);
        }
        if corner_t && corner_r {
            return Some(QMdiSubWindowHit::ResizeTopRight);
        }
        if corner_b && corner_l {
            return Some(QMdiSubWindowHit::ResizeBottomLeft);
        }
        if corner_b && corner_r {
            return Some(QMdiSubWindowHit::ResizeBottomRight);
        }
        if point.x < frame.left() + QMDI_BORDER {
            return Some(QMdiSubWindowHit::ResizeLeft);
        }
        if point.x >= frame.right() - QMDI_BORDER {
            return Some(QMdiSubWindowHit::ResizeRight);
        }
        if point.y < frame.top() + QMDI_BORDER {
            return Some(QMdiSubWindowHit::ResizeTop);
        }
        if point.y >= frame.bottom() - QMDI_BORDER {
            return Some(QMdiSubWindowHit::ResizeBottom);
        }
        None
    }

    fn title_hit(&self, point: QPointF) -> Option<QMdiSubWindowHit> {
        if point.y >= self.frame.y + QMDI_TITLE_H {
            return None;
        }
        for (button, rect) in self.title_buttons() {
            if !rect.contains(point) {
                continue;
            }
            return Some(match button {
                QMdiSubWindowButton::SystemMenu => QMdiSubWindowHit::SystemMenu,
                QMdiSubWindowButton::Minimize => QMdiSubWindowHit::Minimize,
                QMdiSubWindowButton::Maximize => QMdiSubWindowHit::Maximize,
                QMdiSubWindowButton::Restore => {
                    if self.minimized {
                        QMdiSubWindowHit::Minimize
                    } else {
                        QMdiSubWindowHit::Maximize
                    }
                }
                QMdiSubWindowButton::Close => QMdiSubWindowHit::Close,
            });
        }
        Some(QMdiSubWindowHit::Title)
    }

    pub fn hit_test(&self, point: QPointF) -> Option<QMdiSubWindowHit> {
        if !self.visible || !self.frame.contains(point) {
            return None;
        }
        if let Some(hit) = self.resize_hit(point) {
            return Some(hit);
        }
        if let Some(hit) = self.title_hit(point) {
            return Some(hit);
        }
        Some(QMdiSubWindowHit::Body)
    }

    /// Touch hit-testing: the title bar wins over the narrow resize border.
    /// Fingers are much less precise than a mouse, and resizing a window when
    /// the user tried to drag it by the title is the most common failure.
    pub fn hit_test_touch(&self, point: QPointF) -> Option<QMdiSubWindowHit> {
        if !self.visible || !self.frame.contains(point) {
            return None;
        }
        if let Some(hit) = self.title_hit(point) {
            return Some(hit);
        }
        if let Some(hit) = self.resize_hit(point) {
            return Some(hit);
        }
        Some(QMdiSubWindowHit::Body)
    }
}

#[derive(Clone, Debug)]
pub struct QMdiArea<T> {
    subwindows: Vec<QMdiSubWindow<T>>,
    active: Option<usize>,
    cascade: usize,
    viewport: QRectF,
}

impl<T> QMdiArea<T> {
    pub fn new(viewport: QRectF) -> Self {
        Self {
            subwindows: Vec::new(),
            active: None,
            cascade: 0,
            viewport,
        }
    }

    pub fn viewport(&self) -> QRectF {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: QRectF) {
        self.viewport = viewport;
        if let Some(index) = self.active {
            if self.subwindows[index].maximized {
                self.subwindows[index].frame = viewport;
            }
        }
    }

    pub fn subwindows(&self) -> &[QMdiSubWindow<T>] {
        &self.subwindows
    }

    pub fn subwindows_mut(&mut self) -> &mut [QMdiSubWindow<T>] {
        &mut self.subwindows
    }

    pub fn subwindow(&self, index: usize) -> Option<&QMdiSubWindow<T>> {
        self.subwindows.get(index)
    }

    pub fn subwindow_mut(&mut self, index: usize) -> Option<&mut QMdiSubWindow<T>> {
        self.subwindows.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.subwindows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.subwindows.is_empty()
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn active_payload(&self) -> Option<&T> {
        self.active
            .and_then(|index| self.subwindows.get(index).map(|sw| &sw.payload))
    }

    pub fn index_of(&self, predicate: impl Fn(&T) -> bool) -> Option<usize> {
        self.subwindows
            .iter()
            .position(|subwindow| predicate(&subwindow.payload))
    }

    pub fn add_subwindow(
        &mut self,
        title: impl Into<String>,
        payload: T,
        content_size: QSizeF,
    ) -> usize {
        // Windows-style auto cascade: every new window gets the next slot
        // of the Qt-probed cascade pattern (see `cascade_pos`). Positions
        // are absolute inside the viewport — callers no longer shift them.
        let size = QSizeF::new(
            content_size.width.max(80.0),
            (content_size.height + QMDI_TITLE_H).max(64.0),
        );
        let pos = Self::cascade_pos(self.viewport, self.cascade, size);
        self.cascade += 1;
        let frame = QRectF::new(pos.x, pos.y, size.width, size.height);
        self.subwindows.push(QMdiSubWindow {
            payload,
            title: title.into(),
            frame,
            maximized: false,
            minimized: false,
            visible: true,
            window_flags: QMdiWindowFlags::default(),
            resizable: true,
            min_content: QSizeF::default(),
            max_content: QSizeF::new(f32::MAX, f32::MAX),
            restore_frame: None,
        });
        let index = self.subwindows.len() - 1;
        self.active = Some(index);
        index
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.subwindows.len() {
            return None;
        }
        let removed = self.subwindows.remove(index);
        match self.active {
            Some(active) if active == index => {
                self.active = self.subwindows.len().checked_sub(1);
            }
            Some(active) if active > index => {
                self.active = Some(active - 1);
            }
            _ => {}
        }
        Some(removed.payload)
    }

    pub fn clear(&mut self) {
        self.subwindows.clear();
        self.active = None;
        self.cascade = 0;
    }

    pub fn activate(&mut self, index: usize) -> Option<usize> {
        if index >= self.subwindows.len() {
            return None;
        }
        if self.active == Some(index) {
            return Some(index);
        }
        let mut subwindow = self.subwindows.remove(index);
        subwindow.visible = true;
        self.subwindows.push(subwindow);
        let index = self.subwindows.len() - 1;
        self.active = Some(index);
        Some(index)
    }

    pub fn activate_payload(&mut self, predicate: impl Fn(&T) -> bool) -> Option<usize> {
        self.index_of(predicate)
            .and_then(|index| self.activate(index))
    }

    pub fn hit_test(&self, point: QPointF) -> Option<(usize, QMdiSubWindowHit)> {
        for (index, subwindow) in self.subwindows.iter().enumerate().rev() {
            if let Some(hit) = subwindow.hit_test(point) {
                return Some((index, hit));
            }
        }
        None
    }

    /// Hit-test for touch input; title bar is preferred over resize borders.
    pub fn hit_test_touch(&self, point: QPointF) -> Option<(usize, QMdiSubWindowHit)> {
        for (index, subwindow) in self.subwindows.iter().enumerate().rev() {
            if let Some(hit) = subwindow.hit_test_touch(point) {
                return Some((index, hit));
            }
        }
        None
    }

    pub fn translate(&mut self, index: usize, dx: f32, dy: f32) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            if !subwindow.maximized && !subwindow.minimized {
                subwindow.frame = subwindow.frame.translated(dx, dy);
            }
        }
    }

    pub fn set_content_geometry(&mut self, index: usize, rect: QRectF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            let frame = QRectF::new(
                rect.x,
                rect.y - QMDI_TITLE_H,
                rect.width,
                rect.height + QMDI_TITLE_H,
            );
            subwindow.frame = subwindow.constrained_frame(frame);
            subwindow.maximized = false;
            subwindow.minimized = false;
            subwindow.restore_frame = None;
        }
    }

    pub fn resize_content(&mut self, index: usize, size: QSizeF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            if !subwindow.maximized && !subwindow.minimized {
                let frame = QRectF::new(
                    subwindow.frame.x,
                    subwindow.frame.y,
                    size.width,
                    size.height + QMDI_TITLE_H,
                );
                subwindow.frame = subwindow.constrained_frame(frame);
            }
        }
    }

    /// Absolute cascade position for `slot` inside `viewport`.
    ///
    /// Qt6 ground truth (`qt_reference --mdi`): `cascadeSubWindows` steps
    /// (10, 22) — the y step equals Qt's title height, so every following
    /// title bar lands right below the previous one, and Qt's columns are
    /// overlapping halves of the area. We keep a 32/28 inset from the
    /// viewport edge (titles do not touch the frame) and use our own
    /// `QMDI_TITLE_H` as the y step; columns advance by half a viewport.
    /// When the slot leaves the viewport, the cascade wraps into the next
    /// column, and slots beyond the visible grid wrap around.
    fn cascade_pos(viewport: QRectF, slot: usize, frame: QSizeF) -> QPointF {
        const STEP_X: f32 = 10.0;
        const INSET_X: f32 = 32.0;
        const INSET_Y: f32 = 28.0;

        let step_y = QMDI_TITLE_H;
        let rows = (((viewport.height - INSET_Y - frame.height) / step_y)
            .floor()
            .max(0.0) as usize
            + 1)
        .max(1);
        let col_step = (viewport.width / 2.0).max(STEP_X);
        let cols = (((viewport.width - INSET_X - 40.0) / col_step)
            .floor()
            .max(0.0) as usize
            + 1)
        .max(1);
        let slots = rows.saturating_mul(cols).max(1);
        let slot = slot % slots;
        let col = slot / rows;
        let row = slot % rows;
        QPointF::new(
            viewport.x + INSET_X + col as f32 * col_step + row as f32 * STEP_X,
            viewport.y + INSET_Y + row as f32 * step_y,
        )
    }

    /// Move one subwindow to the next cascade slot (`Form:cascade`).
    pub fn cascade_window(&mut self, index: usize) {
        let Some(subwindow) = self.subwindows.get(index) else {
            return;
        };
        if !subwindow.visible || subwindow.maximized || subwindow.minimized {
            return;
        }
        let size = QSizeF::new(subwindow.frame.width, subwindow.frame.height);
        let pos = Self::cascade_pos(self.viewport, self.cascade, size);
        self.cascade += 1;
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            subwindow.frame = QRectF::new(
                pos.x,
                pos.y,
                subwindow.frame.width,
                subwindow.frame.height,
            );
        }
    }

    /// Qt `QMdiArea::cascadeSubWindows`: re-arrange every normal-state
    /// subwindow from the cascade origin (probed step: x 10, y = title
    /// height). Maximised/minimized windows are left alone, like Qt.
    pub fn cascade_subwindows(&mut self) {
        let mut placed = 0usize;
        for index in 0..self.subwindows.len() {
            let (visible, frame) = {
                let subwindow = &self.subwindows[index];
                (
                    subwindow.visible && !subwindow.maximized && !subwindow.minimized,
                    QRectF::new(
                        subwindow.frame.x,
                        subwindow.frame.y,
                        subwindow.frame.width,
                        subwindow.frame.height,
                    ),
                )
            };
            if !visible {
                continue;
            }
            let pos = Self::cascade_pos(
                self.viewport,
                placed,
                QSizeF::new(frame.width, frame.height),
            );
            let subwindow = &mut self.subwindows[index];
            subwindow.frame = QRectF::new(pos.x, pos.y, frame.width, frame.height);
            placed += 1;
        }
        // Follow-up `add_subwindow` calls continue after the arranged set.
        self.cascade = placed;
    }

    /// Qt `QMdiArea::tileSubWindows`: `ceil(sqrt(n))` columns × rows over
    /// the viewport with an integer cell split; frames shrink to their cell
    /// but never below the subwindow's size constraints (Qt keeps the
    /// minimum too, letting tiles overflow a too-small area).
    pub fn tile_subwindows(&mut self) {
        let viewport = self.viewport;
        let indices: Vec<usize> = (0..self.subwindows.len())
            .filter(|&index| {
                let subwindow = &self.subwindows[index];
                subwindow.visible && !subwindow.maximized && !subwindow.minimized
            })
            .collect();
        if indices.is_empty() {
            return;
        }
        let count = indices.len();
        let cols = ((count as f32).sqrt().ceil() as usize).max(1);
        let rows = count.div_ceil(cols);
        for (position, &index) in indices.iter().enumerate() {
            let col = position % cols;
            let row = position / cols;
            let cell = QRectF::new(
                viewport.x + viewport.width * col as f32 / cols as f32,
                viewport.y + viewport.height * row as f32 / rows as f32,
                viewport.width / cols as f32,
                viewport.height / rows as f32,
            );
            let subwindow = &mut self.subwindows[index];
            subwindow.frame = subwindow.constrained_frame(cell);
        }
    }

    /// Set the whole subwindow frame (used by interactive border resizing).
    /// Size constraints are enforced on width/height; the caller anchors
    /// the opposite edge itself.
    pub fn set_size_constraints(&mut self, index: usize, min: QSizeF, max: QSizeF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            subwindow.set_size_constraints(min, max);
        }
    }

    pub fn set_resizable(&mut self, index: usize, resizable: bool) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            subwindow.set_resizable(resizable);
        }
    }

    pub fn set_frame(&mut self, index: usize, frame: QRectF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            if !subwindow.maximized && !subwindow.minimized {
                subwindow.frame = subwindow.constrained_frame(frame);
            }
        }
    }

    pub fn toggle_maximized(&mut self, index: usize) -> Option<bool> {
        let subwindow = self.subwindows.get_mut(index)?;
        if subwindow.minimized {
            return None;
        }
        if subwindow.maximized {
            if let Some(frame) = subwindow.restore_frame.take() {
                subwindow.frame = frame;
            }
            subwindow.maximized = false;
            Some(false)
        } else {
            subwindow.restore_frame = Some(subwindow.frame);
            subwindow.frame = self.viewport;
            subwindow.maximized = true;
            Some(true)
        }
    }

    pub fn toggle_minimized(&mut self, index: usize) -> Option<bool> {
        let viewport = self.viewport;
        let slot = self
            .subwindows
            .iter()
            .enumerate()
            .filter(|(i, subwindow)| *i != index && subwindow.minimized)
            .count();
        let subwindow = self.subwindows.get_mut(index)?;
        if subwindow.minimized {
            subwindow.minimized = false;
            if let Some(frame) = subwindow.restore_frame.take() {
                subwindow.frame = frame;
            }
            return Some(false);
        }
        if subwindow.maximized {
            if let Some(frame) = subwindow.restore_frame.take() {
                subwindow.frame = frame;
            }
            subwindow.maximized = false;
        }
        subwindow.restore_frame = Some(subwindow.frame);
        subwindow.minimized = true;
        let icon_w = 180.0_f32.min((viewport.width - 16.0).max(80.0));
        let x = (viewport.x + 8.0 + slot as f32 * (icon_w + 4.0))
            .min((viewport.right() - icon_w - 8.0).max(viewport.x + 8.0));
        let y = (viewport.bottom() - QMDI_TITLE_H - 8.0).max(viewport.y + 8.0);
        subwindow.frame = QRectF::new(x, y, icon_w, QMDI_TITLE_H);
        Some(true)
    }

    pub fn set_title(&mut self, index: usize, title: impl Into<String>) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            subwindow.title = title.into();
        }
    }
}

/// Paint title bar, border, window buttons and resize grip for an MDI subwindow.
pub fn paint_subwindow_chrome<T>(
    painter: &mut QPainter,
    subwindow: &QMdiSubWindow<T>,
    active: bool,
    hover: Option<QMdiSubWindowHit>,
) {
    let frame = subwindow.frame;
    let (title_start, title_end) = if active {
        (
            QColor::rgba8(74, 159, 220, 255),
            QColor::rgba8(39, 121, 180, 255),
        )
    } else {
        (
            QColor::rgba8(235, 237, 241, 255),
            QColor::rgba8(202, 206, 214, 255),
        )
    };
    let border = QColor::rgba8(148, 156, 168, 255);
    let text_color = if active {
        QColor::rgba8(255, 255, 255, 255)
    } else {
        QColor::rgba8(24, 24, 30, 255)
    };
    let icon_color = if active {
        QColor::rgba8(255, 255, 255, 255)
    } else {
        QColor::rgba8(38, 38, 44, 255)
    };
    let hover_fill = if active {
        QColor::rgba8(126, 190, 238, 255)
    } else {
        QColor::rgba8(214, 218, 226, 255)
    };

    painter.fill_linear_gradient_rect(
        QRectF::new(frame.x, frame.y, frame.width, QMDI_TITLE_H),
        0.0,
        title_start,
        title_end,
    );
    for border_rect in [
        QRectF::new(frame.x, frame.y, 1.0, frame.height),
        QRectF::new(frame.right() - 1.0, frame.y, 1.0, frame.height),
        QRectF::new(frame.x, frame.bottom() - 1.0, frame.width, 1.0),
    ] {
        painter.fill_rect(border_rect, border);
    }

    let buttons = subwindow.title_buttons();
    let mut title_right = frame.right() - 8.0;
    for (button, rect) in &buttons {
        if *button != QMdiSubWindowButton::SystemMenu {
            title_right = title_right.min(rect.x - 4.0);
        }
        let button_hit = match button {
            QMdiSubWindowButton::SystemMenu => QMdiSubWindowHit::SystemMenu,
            QMdiSubWindowButton::Minimize => QMdiSubWindowHit::Minimize,
            QMdiSubWindowButton::Maximize => QMdiSubWindowHit::Maximize,
            QMdiSubWindowButton::Restore => {
                if subwindow.minimized {
                    QMdiSubWindowHit::Minimize
                } else {
                    QMdiSubWindowHit::Maximize
                }
            }
            QMdiSubWindowButton::Close => QMdiSubWindowHit::Close,
        };
        if hover == Some(button_hit) {
            painter.fill_rounded_rect(
                QRectF::new(
                    rect.x + 1.0,
                    rect.y + 1.0,
                    rect.width - 2.0,
                    rect.height - 2.0,
                ),
                4.0,
                hover_fill,
            );
        }
        let cx = rect.x + rect.width / 2.0;
        let cy = rect.y + rect.height / 2.0;
        match button {
            QMdiSubWindowButton::Close => {
                let d = 4.0;
                painter.draw_line(
                    QPointF::new(cx - d, cy - d),
                    QPointF::new(cx + d, cy + d),
                    1.0,
                    icon_color,
                );
                painter.draw_line(
                    QPointF::new(cx + d, cy - d),
                    QPointF::new(cx - d, cy + d),
                    1.0,
                    icon_color,
                );
            }
            QMdiSubWindowButton::Minimize => {
                painter.draw_line(
                    QPointF::new(rect.x + 5.0, rect.y + 12.5),
                    QPointF::new(rect.x + 13.0, rect.y + 12.5),
                    1.0,
                    icon_color,
                );
            }
            QMdiSubWindowButton::Maximize => {
                painter.stroke_rect(
                    QRectF::new(rect.x + 5.0, rect.y + 6.0, 8.0, 6.0),
                    1.0,
                    icon_color,
                );
                painter.fill_rect(
                    QRectF::new(rect.x + 5.0, rect.y + 6.0, 8.0, 2.0),
                    icon_color,
                );
            }
            QMdiSubWindowButton::Restore => {
                painter.stroke_rect(
                    QRectF::new(rect.x + 6.0, rect.y + 4.0, 7.0, 5.0),
                    1.0,
                    icon_color,
                );
                painter.fill_rect(
                    QRectF::new(rect.x + 6.0, rect.y + 4.0, 7.0, 1.0),
                    icon_color,
                );
                painter.stroke_rect(
                    QRectF::new(rect.x + 4.0, rect.y + 7.0, 7.0, 5.0),
                    1.0,
                    icon_color,
                );
                painter.fill_rect(
                    QRectF::new(rect.x + 4.0, rect.y + 7.0, 7.0, 1.0),
                    icon_color,
                );
            }
            QMdiSubWindowButton::SystemMenu => {
                painter.stroke_rect(
                    QRectF::new(rect.x + 4.0, rect.y + 4.0, 10.0, 10.0),
                    1.0,
                    icon_color,
                );
                painter.fill_rect(
                    QRectF::new(rect.x + 4.0, rect.y + 4.0, 10.0, 2.0),
                    icon_color,
                );
            }
        }
    }

    let label_left = frame.x + 8.0;
    let label_width = (title_right - label_left).max(1.0);
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        label_left,
        frame.y,
        label_width,
        QMDI_TITLE_H,
    )));
    painter.draw_text(
        QRectF::new(label_left, frame.y + 4.0, label_width, QMDI_TITLE_H),
        subwindow.title.clone(),
        text_color,
    );
    painter.restore();

    if subwindow.can_resize() {
        let resize = subwindow.resize_rect();
        for step in 0..3 {
            let offset = step as f32 * 4.0;
            painter.draw_line(
                QPointF::new(resize.right() - 3.0 - offset, resize.bottom() - 1.0),
                QPointF::new(resize.right() - 1.0, resize.bottom() - 3.0 - offset),
                1.0,
                border,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> QMdiArea<String> {
        QMdiArea::new(QRectF::new(0.0, 0.0, 1280.0, 800.0))
    }

    #[test]
    fn resize_borders_follow_resizable_flag_and_constraints() {
        let mut area = area();
        let index = area.add_subwindow("One", "a".into(), QSizeF::new(300.0, 200.0));
        let frame = area.subwindow(index).unwrap().frame;

        // Borders and corners are interactive resize zones.
        assert_eq!(
            area.hit_test(QPointF::new(frame.x + 1.0, frame.y + 100.0)),
            Some((index, QMdiSubWindowHit::ResizeLeft))
        );
        assert_eq!(
            area.hit_test(QPointF::new(frame.right() - 1.0, frame.y + 100.0)),
            Some((index, QMdiSubWindowHit::ResizeRight))
        );
        assert_eq!(
            area.hit_test(QPointF::new(frame.x + 100.0, frame.bottom() - 1.0)),
            Some((index, QMdiSubWindowHit::ResizeBottom))
        );
        assert_eq!(
            area.hit_test(QPointF::new(frame.x + 1.0, frame.y + 1.0)),
            Some((index, QMdiSubWindowHit::ResizeTopLeft))
        );
        assert_eq!(
            area.hit_test(QPointF::new(frame.right() - 1.0, frame.bottom() - 1.0)),
            Some((index, QMdiSubWindowHit::ResizeBottomRight))
        );
        assert!(area.subwindow(index).unwrap().can_resize());

        // A non-resizable form keeps the same points for title/body hits.
        area.subwindows_mut()[index].set_resizable(false);
        assert!(!area.subwindow(index).unwrap().can_resize());
        assert_eq!(
            area.hit_test(QPointF::new(frame.x + 1.0, frame.y + 100.0)),
            Some((index, QMdiSubWindowHit::Body))
        );
        assert_eq!(
            area.hit_test(QPointF::new(frame.x + 1.0, frame.y + 1.0)),
            Some((index, QMdiSubWindowHit::Title))
        );

        // Minimum/maximum content sizes clamp programmatic and interactive
        // resizes; the title bar adds QMDI_TITLE_H.
        area.subwindows_mut()[index].set_resizable(true);
        area.subwindows_mut()[index]
            .set_size_constraints(QSizeF::new(200.0, 150.0), QSizeF::new(400.0, 300.0));
        area.resize_content(index, QSizeF::new(1000.0, 1000.0));
        let frame = area.subwindow(index).unwrap().frame;
        assert_eq!((frame.width, frame.height), (400.0, 300.0 + QMDI_TITLE_H));
        assert_eq!(
            area.subwindow(index).unwrap().content_rect().size(),
            QSizeF::new(400.0, 300.0)
        );

        area.set_content_geometry(index, QRectF::new(50.0, 60.0, 10.0, 10.0));
        let frame = area.subwindow(index).unwrap().frame;
        assert_eq!((frame.x, frame.y), (50.0, 60.0 - QMDI_TITLE_H));
        assert_eq!((frame.width, frame.height), (200.0, 150.0 + QMDI_TITLE_H));

        // Width and height fixed to the same value means no interactive grip.
        area.subwindows_mut()[index]
            .set_size_constraints(QSizeF::new(250.0, 180.0), QSizeF::new(250.0, 180.0));
        assert!(!area.subwindow(index).unwrap().can_resize());
    }

    #[test]
    fn subwindow_layout_and_hit_test() {
        let mut area = area();
        let index = area.add_subwindow("One", "a".into(), QSizeF::new(300.0, 200.0));
        let subwindow = area.subwindow(index).unwrap();
        assert_eq!(subwindow.frame.width, 300.0);
        assert_eq!(subwindow.frame.height, 200.0 + QMDI_TITLE_H);
        assert_eq!(
            subwindow.content_rect(),
            QRectF::new(32.0, 28.0 + QMDI_TITLE_H, 300.0, 200.0)
        );
        assert_eq!(
            area.hit_test(QPointF::new(100.0, 40.0)),
            Some((index, QMdiSubWindowHit::Title))
        );
        assert_eq!(
            area.hit_test(QPointF::new(100.0, 100.0)),
            Some((index, QMdiSubWindowHit::Body))
        );
        let close = subwindow.close_rect();
        assert_eq!(
            area.hit_test(QPointF::new(close.x + 2.0, close.y + 2.0)),
            Some((index, QMdiSubWindowHit::Close))
        );

        // Touch prefers the title bar over the top resize border.
        let frame = subwindow.frame;
        let top_border = QPointF::new(frame.x + 20.0, frame.y + 1.0);
        assert_eq!(
            area.hit_test(top_border),
            Some((index, QMdiSubWindowHit::ResizeTop))
        );
        assert_eq!(
            area.hit_test_touch(top_border),
            Some((index, QMdiSubWindowHit::Title))
        );
        let bottom_border = QPointF::new(frame.x + 20.0, frame.bottom() - 1.0);
        assert_eq!(
            area.hit_test_touch(bottom_border),
            Some((index, QMdiSubWindowHit::ResizeBottom))
        );
    }

    #[test]
    fn title_buttons_minimize_and_restore_like_qt() {
        let mut area = area();
        let index = area.add_subwindow("One", "a".into(), QSizeF::new(300.0, 200.0));
        let frame = area.subwindow(index).unwrap().frame;
        let buttons = area.subwindow(index).unwrap().title_buttons();
        assert!(buttons
            .iter()
            .any(|(button, _)| *button == QMdiSubWindowButton::Close));
        assert!(buttons
            .iter()
            .any(|(button, _)| *button == QMdiSubWindowButton::Maximize));
        assert!(buttons
            .iter()
            .any(|(button, _)| *button == QMdiSubWindowButton::Minimize));

        let minimize = area.subwindow(index).unwrap().minimize_rect();
        assert_eq!(
            area.hit_test(QPointF::new(minimize.x + 2.0, minimize.y + 2.0)),
            Some((index, QMdiSubWindowHit::Minimize))
        );
        assert_eq!(area.toggle_minimized(index), Some(true));
        assert!(area.subwindow(index).unwrap().minimized);
        assert_eq!(area.subwindow(index).unwrap().content_rect().height, 0.0);
        assert!(area
            .subwindow(index)
            .unwrap()
            .title_buttons()
            .iter()
            .any(|(button, _)| *button == QMdiSubWindowButton::Restore));

        let restore = area
            .subwindow(index)
            .unwrap()
            .title_buttons()
            .into_iter()
            .find(|(button, _)| *button == QMdiSubWindowButton::Restore)
            .map(|(_, rect)| rect)
            .unwrap();
        assert_eq!(
            area.hit_test(QPointF::new(restore.x + 2.0, restore.y + 2.0)),
            Some((index, QMdiSubWindowHit::Minimize))
        );
        assert_eq!(area.toggle_minimized(index), Some(false));
        assert_eq!(area.subwindow(index).unwrap().frame, frame);
    }

    #[test]
    fn activation_raises_and_maximize_restores() {
        let mut area = area();
        let a = area.add_subwindow("A", "a".into(), QSizeF::new(300.0, 200.0));
        let b = area.add_subwindow("B", "b".into(), QSizeF::new(300.0, 200.0));
        assert_eq!(area.active_index(), Some(b));
        let _ = b;
        let a = area.activate(a).unwrap();
        assert_eq!(area.active_payload(), Some(&"a".to_string()));
        assert_eq!(area.subwindows()[a].payload, "a");

        let frame = area.subwindow(a).unwrap().frame;
        assert_eq!(area.toggle_maximized(a), Some(true));
        assert_eq!(area.subwindow(a).unwrap().frame, area.viewport());
        assert_eq!(area.toggle_maximized(a), Some(false));
        assert_eq!(area.subwindow(a).unwrap().frame, frame);
    }

    #[test]
    fn content_geometry_places_title_above_content() {
        let mut area = area();
        let index = area.add_subwindow("A", "a".into(), QSizeF::new(300.0, 200.0));
        area.set_content_geometry(index, QRectF::new(100.0, 100.0, 400.0, 400.0));
        let subwindow = area.subwindow(index).unwrap();
        assert_eq!(
            subwindow.content_rect(),
            QRectF::new(100.0, 100.0, 400.0, 400.0)
        );
        assert_eq!(subwindow.frame.y, 100.0 - QMDI_TITLE_H);
    }

    #[test]
    fn remove_adjusts_active_window() {
        let mut area = area();
        let a = area.add_subwindow("A", "a".into(), QSizeF::new(300.0, 200.0));
        let _b = area.add_subwindow("B", "b".into(), QSizeF::new(300.0, 200.0));
        assert_eq!(area.remove(a), Some("a".into()));
        assert_eq!(area.len(), 1);
        assert_eq!(area.active_index(), Some(0));
        assert_eq!(area.active_payload(), Some(&"b".to_string()));
        assert_eq!(area.remove(0), Some("b".into()));
        assert_eq!(area.active_index(), None);
    }

    #[test]
    fn cascade_slots_step_title_height_and_wrap_into_columns() {
        // Qt6 probe (`qt_reference --mdi`): cascade step is (10, title h).
        // 400x300 viewport with 200x100 content → frame 200x124:
        // rows = (300-28-124)/24 + 1 = 7, col_step = 400/2 = 200,
        // cols = (400-32-40)/200 + 1 = 2 → 14 slots before wrapping.
        let mut area = QMdiArea::new(QRectF::new(0.0, 0.0, 400.0, 300.0));
        let mut frames = Vec::new();
        for i in 0..15 {
            let index =
                area.add_subwindow(format!("W{i}"), i, QSizeF::new(200.0, 100.0));
            frames.push(area.subwindow(index).unwrap().frame);
        }
        // Origin keeps the 32/28 inset; x step 10, y step = title height,
        // so every following title bar sits directly below the previous.
        assert_eq!((frames[0].x, frames[0].y), (32.0, 28.0));
        assert_eq!((frames[1].x, frames[1].y), (42.0, 52.0));
        assert_eq!((frames[2].x, frames[2].y), (52.0, 76.0));
        // Slot 7 wraps into the second column at the top…
        assert_eq!((frames[7].x, frames[7].y), (232.0, 28.0));
        // …and slot 14 exceeds the visible grid and wraps to the first.
        assert_eq!((frames[14].x, frames[14].y), (32.0, 28.0));
    }

    #[test]
    fn cascade_and_tile_rearrange_normal_windows() {
        let mut area = area();
        let mut added = Vec::new();
        for i in 0..4 {
            let index = area.add_subwindow(
                format!("W{i}"),
                i.to_string(),
                QSizeF::new(300.0, 150.0),
            );
            added.push(index);
        }
        // Scatter so the cascade has visible work to do.
        for (step, &index) in added.iter().enumerate() {
            area.translate(index, step as f32 * 150.0, 0.0);
        }
        area.cascade_subwindows();
        for (slot, &index) in added.iter().enumerate() {
            let frame = area.subwindow(index).unwrap().frame;
            assert_eq!(
                (frame.x, frame.y),
                (
                    32.0 + slot as f32 * 10.0,
                    28.0 + slot as f32 * QMDI_TITLE_H
                )
            );
        }
        area.tile_subwindows();
        let expected = [
            QRectF::new(0.0, 0.0, 640.0, 400.0),
            QRectF::new(640.0, 0.0, 640.0, 400.0),
            QRectF::new(0.0, 400.0, 640.0, 400.0),
            QRectF::new(640.0, 400.0, 640.0, 400.0),
        ];
        for (slot, &index) in added.iter().enumerate() {
            assert_eq!(area.subwindow(index).unwrap().frame, expected[slot]);
        }
    }
}
