//! Application shell: menu bar, toolbar and status bar.
//!
//! The shell is deliberately independent from `QMdiArea`: it owns the
//! application chrome, while forms are placed either into the central MDI
//! area (`placement=mdi`) or into a separate top-level layer.

use form_xml::PropValue;
use rqt::gui::{QColor, QFont, QFontMetrics, QPainter, QRectF};
use rqt::icons;

pub const MENU_H: f32 = 22.0;
pub const TOOLBAR_H: f32 = 30.0;
pub const STATUS_H: f32 = 22.0;
const ITEM_H: f32 = 24.0;
const SEPARATOR_H: f32 = 9.0;
const ICON: f32 = 18.0;
const TOOL_ICON: f32 = 24.0;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub action: Option<String>,
    pub lua_file: Option<String>,
    pub ui_file: Option<String>,
    pub mdi: bool,
    pub modal: bool,
    pub modal_scope: String,
    pub enabled: bool,
    pub separator: bool,
    pub checkable: bool,
    pub checked: bool,
    pub children: Vec<MenuItem>,
}

impl Default for MenuItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            icon: None,
            action: None,
            lua_file: None,
            ui_file: None,
            mdi: false,
            modal: false,
            modal_scope: "application".into(),
            enabled: true,
            separator: false,
            checkable: false,
            checked: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShellHit {
    MenuTop(usize),
    /// Path of indices: `[top]` for a top menu, `[top, child, ...]` for an item.
    MenuItem(Vec<usize>),
    Toolbar(usize),
}

#[derive(Clone, Debug)]
struct MenuEntry {
    path: Vec<usize>,
    rect: QRectF,
    item: MenuItem,
}

#[derive(Default)]
pub struct Shell {
    pub visible: bool,
    pub menu: Vec<MenuItem>,
    pub toolbar: Vec<MenuItem>,
    pub toolbar_visible: bool,
    pub status_visible: bool,
    pub(crate) status_message: Option<String>,
    /// Absolute timestamp in `now_ms` units; `None` means persistent.
    pub(crate) status_until: Option<f64>,
    pub(crate) status_info: Option<String>,
    pub(crate) open_path: Option<Vec<usize>>,
}

impl Shell {
    pub fn set_menu(&mut self, value: &PropValue) {
        self.menu = items_from_value(value);
        self.open_path = None;
    }

    pub fn set_toolbar(&mut self, value: &PropValue) {
        self.toolbar = items_from_value(value);
        self.toolbar_visible = !self.toolbar.is_empty();
    }

    pub fn set_status_message(&mut self, text: Option<String>, timeout_ms: Option<f64>, now: f64) {
        self.status_message = text;
        self.status_until = timeout_ms.map(|ms| now + ms.max(0.0));
    }

    pub fn set_status_info(&mut self, text: Option<String>) {
        self.status_info = text;
    }

    pub fn status_text(&self, now: f64) -> Option<&str> {
        if let Some(until) = self.status_until {
            if now >= until {
                return None;
            }
        }
        self.status_message.as_deref()
    }

    pub fn menu_open(&self) -> bool {
        self.open_path.is_some()
    }

    pub fn close_menu(&mut self) {
        self.open_path = None;
    }

    pub fn central(&self, viewport: QRectF) -> QRectF {
        if !self.visible {
            return viewport;
        }
        let mut top = viewport.y;
        if !self.menu.is_empty() {
            top += MENU_H;
        }
        if self.toolbar_visible && !self.toolbar.is_empty() {
            top += TOOLBAR_H;
        }
        let bottom = if self.status_visible {
            (viewport.bottom() - STATUS_H).max(top)
        } else {
            viewport.bottom()
        };
        QRectF::new(viewport.x, top, viewport.width, (bottom - top).max(0.0))
    }

    fn menu_height(&self) -> f32 {
        if self.menu.is_empty() {
            0.0
        } else {
            MENU_H
        }
    }

    fn top_rects(&self, viewport: QRectF) -> Vec<QRectF> {
        let font = QFont::default();
        let metrics = QFontMetrics::new(&font);
        let mut x = viewport.x;
        let mut out = Vec::with_capacity(self.menu.len());
        for item in &self.menu {
            if item.separator {
                continue;
            }
            let width = metrics.horizontal_advance(&item.title) + 24.0;
            out.push(QRectF::new(x, viewport.y, width, MENU_H));
            x += width;
        }
        out
    }

    fn top_rect(&self, viewport: QRectF, index: usize) -> Option<QRectF> {
        let mut visible_index = 0;
        for (i, item) in self.menu.iter().enumerate() {
            if item.separator {
                continue;
            }
            if i == index {
                return self.top_rects(viewport).get(visible_index).copied();
            }
            visible_index += 1;
        }
        None
    }

    fn menu_entries(&self, viewport: QRectF) -> Vec<MenuEntry> {
        let Some(path) = &self.open_path else {
            return Vec::new();
        };
        let Some(root_index) = path.first().copied() else {
            return Vec::new();
        };
        let Some(root) = self.menu.get(root_index) else {
            return Vec::new();
        };
        let Some(top) = self.top_rect(viewport, root_index) else {
            return Vec::new();
        };

        let mut out = Vec::new();
        let mut item = root;
        let mut anchor = top;
        for depth in 0..path.len() {
            let mut y = anchor.bottom();
            let mut last: Option<QRectF> = None;
            for (child_index, child) in item.children.iter().enumerate() {
                let height = if child.separator { SEPARATOR_H } else { ITEM_H };
                let x = if depth == 0 {
                    anchor.x
                } else {
                    anchor.right() - 2.0
                };
                let rect = QRectF::new(x, y, self.menu_width(&item.children), height);
                let mut child_path = path[..=depth].to_vec();
                child_path.push(child_index);
                out.push(MenuEntry {
                    path: child_path,
                    rect,
                    item: child.clone(),
                });
                if depth + 1 < path.len() && path[depth + 1] == child_index {
                    last = Some(rect);
                }
                y += height;
            }
            if depth + 1 >= path.len() {
                break;
            }
            let Some(next_index) = path.get(depth + 1).copied() else {
                break;
            };
            let Some(next) = item.children.get(next_index) else {
                break;
            };
            let Some(next_rect) = last else {
                break;
            };
            item = next;
            anchor = next_rect;
        }
        out
    }

    fn menu_width(&self, items: &[MenuItem]) -> f32 {
        let font = QFont::default();
        let metrics = QFontMetrics::new(&font);
        items
            .iter()
            .map(|item| {
                if item.separator {
                    8.0
                } else {
                    metrics.horizontal_advance(&item.title) + 64.0
                }
            })
            .fold(120.0, f32::max)
    }

    pub fn hit(&self, point: (f32, f32), viewport: QRectF) -> Option<ShellHit> {
        if !self.visible {
            return None;
        }
        let point = rqt::gui::QPointF::new(point.0, point.1);
        for entry in self.menu_entries(viewport) {
            if entry.rect.contains(point) && !entry.item.separator {
                return Some(ShellHit::MenuItem(entry.path));
            }
        }
        for (index, rect) in self.top_rects(viewport).into_iter().enumerate() {
            if rect.contains(point) {
                // Convert the visible index back to the menu vector index.
                let mut visible = 0;
                for (menu_index, item) in self.menu.iter().enumerate() {
                    if item.separator {
                        continue;
                    }
                    if visible == index {
                        return Some(ShellHit::MenuTop(menu_index));
                    }
                    visible += 1;
                }
            }
        }
        if self.toolbar_visible {
            for (index, rect) in self.toolbar_rects(viewport).into_iter().enumerate() {
                if rect.contains(point) {
                    return Some(ShellHit::Toolbar(index));
                }
            }
        }
        None
    }

    /// Handle a click on the shell. Returns the action to execute, if any.
    pub fn click(&mut self, point: (f32, f32), viewport: QRectF) -> Option<MenuItem> {
        let hit = self.hit(point, viewport)?;
        match hit {
            ShellHit::MenuTop(index) => {
                let Some(item) = self.menu.get(index).cloned() else {
                    return None;
                };
                if item.children.is_empty() {
                    self.open_path = None;
                    return Some(item);
                }
                if self.open_path.as_ref().and_then(|p| p.first()) == Some(&index) {
                    self.open_path = None;
                } else {
                    self.open_path = Some(vec![index]);
                }
                None
            }
            ShellHit::MenuItem(path) => {
                let Some(item) = self.item_at(&path).cloned() else {
                    return None;
                };
                if !item.enabled {
                    return None;
                }
                if item.children.is_empty() {
                    self.open_path = None;
                    return Some(item);
                }
                self.open_path = Some(path);
                None
            }
            ShellHit::Toolbar(index) => self
                .toolbar
                .get(index)
                .filter(|item| item.enabled && !item.separator)
                .cloned(),
        }
    }

    /// Open a submenu when the cursor hovers over it (Qt-like menu tracking).
    pub fn hover(&mut self, point: (f32, f32), viewport: QRectF) {
        if !self.menu_open() {
            return;
        }
        let Some(ShellHit::MenuItem(path)) = self.hit(point, viewport) else {
            return;
        };
        let Some(item) = self.item_at(&path) else {
            return;
        };
        if !item.children.is_empty() && item.enabled {
            self.open_path = Some(path);
        }
    }

    fn item_at(&self, path: &[usize]) -> Option<&MenuItem> {
        let (root, rest) = path.split_first()?;
        let mut item = self.menu.get(*root)?;
        for index in rest {
            item = item.children.get(*index)?;
        }
        Some(item)
    }

    fn toolbar_rects(&self, viewport: QRectF) -> Vec<QRectF> {
        if !self.toolbar_visible || self.toolbar.is_empty() {
            return Vec::new();
        }
        let font = QFont::default();
        let metrics = QFontMetrics::new(&font);
        let y = viewport.y + self.menu_height();
        let mut x = viewport.x + 4.0;
        let mut out = Vec::new();
        for item in &self.toolbar {
            if item.separator {
                out.push(QRectF::new(x, y + 4.0, 8.0, TOOLBAR_H - 8.0));
                x += 8.0;
                continue;
            }
            let text_width = metrics.horizontal_advance(&item.title);
            let width = (text_width + 36.0).max(32.0);
            out.push(QRectF::new(x, y + 1.0, width, TOOLBAR_H - 2.0));
            x += width;
        }
        out
    }

    pub fn paint_base(
        &self,
        painter: &mut QPainter,
        viewport: QRectF,
        now: f64,
        technical: Option<&str>,
    ) {
        if !self.visible {
            return;
        }
        let font = QFont::default();
        let metrics = QFontMetrics::new(&font);
        let palette = rqt::gui::QPalette::fusion();
        let window = palette.color(rqt::gui::QPaletteRole::Window);
        let button = palette.color(rqt::gui::QPaletteRole::Button);
        let text = palette.color(rqt::gui::QPaletteRole::WindowText);
        let disabled = palette.color(rqt::gui::QPaletteRole::PlaceholderText);
        let border = QColor::rgba8(160, 160, 160, 255);
        let highlight = QColor::rgba8(48, 140, 198, 255);

        // Menu bar.
        if !self.menu.is_empty() {
            painter.fill_rect(
                QRectF::new(viewport.x, viewport.y, viewport.width, MENU_H),
                button,
            );
            painter.fill_rect(
                QRectF::new(viewport.x, viewport.y + MENU_H - 1.0, viewport.width, 1.0),
                border,
            );
            let open_root = self.open_path.as_ref().and_then(|p| p.first()).copied();
            for (index, rect) in self.top_rects(viewport).into_iter().enumerate() {
                let mut visible = 0;
                let mut menu_index = 0;
                for (i, item) in self.menu.iter().enumerate() {
                    if item.separator {
                        continue;
                    }
                    if visible == index {
                        menu_index = i;
                        break;
                    }
                    visible += 1;
                }
                let Some(item) = self.menu.get(menu_index) else {
                    continue;
                };
                if open_root == Some(menu_index) {
                    painter.fill_rect(rect, highlight);
                }
                let color = if open_root == Some(menu_index) {
                    QColor::rgba8(255, 255, 255, 255)
                } else if item.enabled {
                    text
                } else {
                    disabled
                };
                let text_rect = QRectF::new(rect.x + 8.0, rect.y + 3.0, rect.width - 12.0, ITEM_H);
                painter.draw_text(text_rect, item.title.clone(), color);
            }
        }

        // Toolbar.
        if self.toolbar_visible && !self.toolbar.is_empty() {
            let y = viewport.y + self.menu_height();
            painter.fill_linear_gradient_rect(
                QRectF::new(viewport.x, y, viewport.width, TOOLBAR_H),
                0.0,
                button,
                window,
            );
            painter.fill_rect(
                QRectF::new(viewport.x, y + TOOLBAR_H - 1.0, viewport.width, 1.0),
                border,
            );
            for (index, rect) in self.toolbar_rects(viewport).into_iter().enumerate() {
                let Some(item) = self.toolbar.get(index) else {
                    continue;
                };
                if item.separator {
                    painter.fill_rect(rect, border);
                    continue;
                }
                let color = if item.enabled { text } else { disabled };
                let mut x = rect.x + 4.0;
                if let Some(icon) = item.icon.as_deref().and_then(icons::pixmap) {
                    painter.draw_pixmap(
                        QRectF::new(x, rect.y + 2.0, TOOL_ICON, TOOL_ICON),
                        icon,
                        1.0,
                    );
                    x += TOOL_ICON + 2.0;
                }
                let label = if item.icon.is_some() {
                    item.title.clone()
                } else {
                    item.title.clone()
                };
                let width = (rect.right() - x - 4.0).max(0.0);
                painter.draw_text(
                    QRectF::new(x, rect.y + 4.0, width, TOOLBAR_H - 6.0),
                    label,
                    color,
                );
            }
        }

        // Status bar.
        if self.status_visible {
            let y = viewport.bottom() - STATUS_H;
            painter.fill_rect(QRectF::new(viewport.x, y, viewport.width, STATUS_H), button);
            painter.fill_rect(QRectF::new(viewport.x, y, viewport.width, 1.0), border);
            let mut left = viewport.x + 8.0;
            if let Some(message) = self.status_text(now) {
                painter.draw_text(
                    QRectF::new(left, y + 3.0, viewport.width * 0.62, STATUS_H),
                    message.to_string(),
                    text,
                );
                left += metrics.horizontal_advance(message);
            }
            if let Some(info) = self.status_info.as_deref().or(technical) {
                let width = metrics.horizontal_advance(info) + 8.0;
                painter.draw_text(
                    QRectF::new(
                        (viewport.right() - width - 8.0).max(left),
                        y + 3.0,
                        width,
                        STATUS_H,
                    ),
                    info.to_string(),
                    disabled,
                );
            }
        }
    }

    pub fn paint_overlay(&self, painter: &mut QPainter, viewport: QRectF) {
        if !self.visible {
            return;
        }
        let entries = self.menu_entries(viewport);
        if entries.is_empty() {
            return;
        }
        let font = QFont::default();
        let window = QColor::rgba8(250, 250, 250, 255);
        let text = QColor::rgba8(28, 28, 32, 255);
        let disabled = QColor::rgba8(130, 130, 134, 255);
        let border = QColor::rgba8(140, 140, 140, 255);
        let highlight = QColor::rgba8(48, 140, 198, 255);
        let metrics = QFontMetrics::new(&font);
        let open = self.open_path.as_deref();
        for entry in entries {
            if entry.item.separator {
                painter.fill_rect(entry.rect, window);
                painter.fill_rect(
                    QRectF::new(
                        entry.rect.x + 4.0,
                        entry.rect.y + 4.0,
                        entry.rect.width - 8.0,
                        1.0,
                    ),
                    border,
                );
                continue;
            }
            painter.fill_rect(entry.rect, window);
            let selected = open.is_some_and(|path| path == entry.path.as_slice());
            if selected && entry.item.enabled {
                painter.fill_rect(entry.rect, highlight);
            }
            let mut x = entry.rect.x + 6.0;
            if let Some(icon) = entry.item.icon.as_deref().and_then(icons::pixmap) {
                painter.draw_pixmap(QRectF::new(x, entry.rect.y + 3.0, ICON, ICON), icon, 1.0);
                x += ICON + 4.0;
            }
            let color = if selected {
                QColor::rgba8(255, 255, 255, 255)
            } else if entry.item.enabled {
                text
            } else {
                disabled
            };
            let label = if entry.item.checkable {
                format!(
                    "{} {}",
                    if entry.item.checked { "✓" } else { " " },
                    entry.item.title
                )
            } else {
                entry.item.title.clone()
            };
            painter.draw_text(
                QRectF::new(
                    x,
                    entry.rect.y + 3.0,
                    entry.rect.width - (x - entry.rect.x) - 20.0,
                    ITEM_H,
                ),
                label,
                color,
            );
            if !entry.item.children.is_empty() {
                painter.draw_text(
                    QRectF::new(entry.rect.right() - 18.0, entry.rect.y + 3.0, 14.0, ITEM_H),
                    "›".to_string(),
                    color,
                );
            }
            // The popup background is painted before its entries. Doing it
            // per entry would hide the text; draw only the outer border here.
            painter.stroke_rect(entry.rect, 1.0, border);
            let _ = metrics;
        }
    }
}

fn items_from_value(value: &PropValue) -> Vec<MenuItem> {
    let values = match value {
        PropValue::List(items) => items.clone(),
        PropValue::Map(entries) => {
            let mut entries = entries.clone();
            entries.sort_by(|(a, _), (b, _)| {
                let ai = a.parse::<i64>().ok();
                let bi = b.parse::<i64>().ok();
                match (ai, bi) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    _ => a.cmp(b),
                }
            });
            entries.into_iter().map(|(_, value)| value).collect()
        }
        _ => return Vec::new(),
    };
    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| item_from_value(value, &index.to_string()))
        .collect()
}

fn item_from_value(value: &PropValue, path: &str) -> Option<MenuItem> {
    let map = match value {
        PropValue::Map(_) => value,
        _ => return None,
    };
    let string = |key: &str| map.get(key).and_then(PropValue::as_str).map(str::to_string);
    let bool_value =
        |key: &str, default: bool| map.get(key).and_then(PropValue::as_bool).unwrap_or(default);
    let title = string("title").unwrap_or_default();
    let separator = bool_value("separator", false)
        || bool_value("is_separator", false)
        || string("type").is_some_and(|value| value.eq_ignore_ascii_case("separator"))
        || title == "-";
    let children = map
        .get("children")
        .map(|value| {
            items_from_value(value)
                .into_iter()
                .enumerate()
                .map(|(index, child)| {
                    let mut child = child;
                    child.id = format!("{path}/{index}");
                    child
                })
                .collect()
        })
        .unwrap_or_default();
    Some(MenuItem {
        id: path.to_string(),
        title,
        icon: string("icon"),
        action: string("action"),
        lua_file: string("lua_file"),
        ui_file: string("ui_file"),
        mdi: bool_value("mdi", false),
        modal: bool_value("modal", false),
        modal_scope: string("modal_scope").unwrap_or_else(|| "application".into()),
        enabled: bool_value("enabled", true),
        separator,
        checkable: bool_value("checkable", false),
        checked: bool_value("checked", false),
        children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu_value() -> PropValue {
        PropValue::List(vec![
            PropValue::Map(vec![
                ("title".into(), PropValue::Str("File".into())),
                (
                    "children".into(),
                    PropValue::List(vec![
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("Open".into())),
                            ("action".into(), PropValue::Str("open_lua".into())),
                            ("lua_file".into(), PropValue::Str("form.lua".into())),
                            ("mdi".into(), PropValue::Bool(true)),
                        ]),
                        PropValue::Map(vec![("separator".into(), PropValue::Bool(true))]),
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("Exit".into())),
                            ("action".into(), PropValue::Str("exit".into())),
                        ]),
                    ]),
                ),
            ]),
            PropValue::Map(vec![
                ("title".into(), PropValue::Str("View".into())),
                (
                    "children".into(),
                    PropValue::List(vec![PropValue::Map(vec![
                        ("title".into(), PropValue::Str("Nested".into())),
                        (
                            "children".into(),
                            PropValue::List(vec![PropValue::Map(vec![
                                ("title".into(), PropValue::Str("Deep".into())),
                                ("action".into(), PropValue::Str("open_lua".into())),
                                ("lua_file".into(), PropValue::Str("deep.lua".into())),
                            ])]),
                        ),
                    ])]),
                ),
            ]),
        ])
    }

    #[test]
    fn parses_qt_menu_spec_and_opens_popup() {
        let mut shell = Shell::default();
        shell.visible = true;
        shell.set_menu(&menu_value());
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let top = shell.top_rect(viewport, 0).unwrap();
        assert_eq!(
            shell.hit((top.x + 2.0, 10.0), viewport),
            Some(ShellHit::MenuTop(0))
        );
        assert!(shell.click((top.x + 2.0, 10.0), viewport).is_none());
        let entries = shell.menu_entries(viewport);
        assert!(entries.iter().any(|entry| entry.item.title == "Open"));
        let open = entries
            .iter()
            .find(|entry| entry.item.title == "Open")
            .unwrap()
            .rect;
        let action = shell.click((open.x + 2.0, open.y + 2.0), viewport);
        assert_eq!(action.unwrap().lua_file.as_deref(), Some("form.lua"));
    }

    #[test]
    fn nested_submenu_is_hit_testable() {
        let mut shell = Shell::default();
        shell.visible = true;
        shell.set_menu(&menu_value());
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        shell.open_path = Some(vec![1]);
        let nested = shell
            .menu_entries(viewport)
            .into_iter()
            .find(|entry| entry.item.title == "Nested")
            .unwrap();
        shell.open_path = Some(nested.path.clone());
        let deep = shell
            .menu_entries(viewport)
            .into_iter()
            .find(|entry| entry.item.title == "Deep")
            .unwrap();
        assert_eq!(
            shell.hit((deep.rect.x + 2.0, deep.rect.y + 2.0), viewport),
            Some(ShellHit::MenuItem(deep.path.clone()))
        );
    }

    #[test]
    fn toolbar_actions_are_hit_testable() {
        let mut shell = Shell::default();
        shell.visible = true;
        shell.set_toolbar(&PropValue::List(vec![PropValue::Map(vec![
            ("title".into(), PropValue::Str("Refresh".into())),
            ("action".into(), PropValue::Str("open_lua".into())),
            ("lua_file".into(), PropValue::Str("form.lua".into())),
        ])]));
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let rect = shell.toolbar_rects(viewport)[0];
        let action = shell
            .click((rect.x + 2.0, rect.y + 2.0), viewport)
            .expect("toolbar action");
        assert_eq!(action.lua_file.as_deref(), Some("form.lua"));
    }

    #[test]
    fn status_message_respects_timeout() {
        let mut shell = Shell::default();
        shell.set_status_message(Some("ready".into()), Some(500.0), 1000.0);
        assert_eq!(shell.status_text(1400.0), Some("ready"));
        assert_eq!(shell.status_text(1500.0), None);
    }
}
