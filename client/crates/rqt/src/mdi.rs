use crate::gui::{QPointF, QRectF, QSizeF};

pub const QMDI_TITLE_H: f32 = 24.0;
pub const QMDI_BUTTON: f32 = 18.0;
pub const QMDI_BUTTON_TOP: f32 = 3.0;
pub const QMDI_BUTTON_RIGHT: f32 = 6.0;
pub const QMDI_BUTTON_GAP: f32 = 2.0;
pub const QMDI_SYSMENU_LEFT: f32 = 5.0;
pub const QMDI_RESIZE: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QMdiSubWindowHit {
    Title,
    SystemMenu,
    Minimize,
    Maximize,
    Close,
    Body,
    ResizeBottomRight,
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

    pub fn hit_test(&self, point: QPointF) -> Option<QMdiSubWindowHit> {
        if !self.visible || !self.frame.contains(point) {
            return None;
        }
        if !self.minimized && self.resize_rect().contains(point) {
            return Some(QMdiSubWindowHit::ResizeBottomRight);
        }
        if point.y < self.frame.y + QMDI_TITLE_H {
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
            return Some(QMdiSubWindowHit::Title);
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
        let offset = (self.cascade % 8) as f32 * 28.0;
        self.cascade += 1;
        let frame = QRectF::new(
            32.0 + offset,
            28.0 + offset,
            content_size.width.max(80.0),
            (content_size.height + QMDI_TITLE_H).max(64.0),
        );
        self.subwindows.push(QMdiSubWindow {
            payload,
            title: title.into(),
            frame,
            maximized: false,
            minimized: false,
            visible: true,
            window_flags: QMdiWindowFlags::default(),
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

    pub fn translate(&mut self, index: usize, dx: f32, dy: f32) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            if !subwindow.maximized && !subwindow.minimized {
                subwindow.frame = subwindow.frame.translated(dx, dy);
            }
        }
    }

    pub fn set_content_geometry(&mut self, index: usize, rect: QRectF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            subwindow.frame = QRectF::new(
                rect.x,
                rect.y - QMDI_TITLE_H,
                rect.width.max(80.0),
                (rect.height + QMDI_TITLE_H).max(64.0),
            );
            subwindow.maximized = false;
            subwindow.minimized = false;
            subwindow.restore_frame = None;
        }
    }

    pub fn resize_content(&mut self, index: usize, size: QSizeF) {
        if let Some(subwindow) = self.subwindows.get_mut(index) {
            if !subwindow.maximized && !subwindow.minimized {
                subwindow.frame.width = size.width.max(80.0);
                subwindow.frame.height = (size.height + QMDI_TITLE_H).max(64.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> QMdiArea<String> {
        QMdiArea::new(QRectF::new(0.0, 0.0, 1280.0, 800.0))
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
            area.hit_test(QPointF::new(40.0, 30.0)),
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
}
