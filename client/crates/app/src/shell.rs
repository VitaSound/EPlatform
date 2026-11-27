//! Application shell: menu bar, toolbar and status bar.
//!
//! The shell is deliberately independent from `QMdiArea`: it owns the
//! application chrome, while forms are placed either into the central MDI
//! area (`placement=mdi`) or into a separate top-level layer.

use std::collections::HashMap;

use form_xml::PropValue;
use rqt::gui::{QColor, QFont, QFontMetrics, QPainter, QRectF};
use rqt::icons;

pub const MENU_H: f32 = 22.0;
pub const TOOLBAR_H: f32 = 39.0;
pub const STATUS_H: f32 = 22.0;
const ITEM_H: f32 = 24.0;
const SEPARATOR_H: f32 = 2.0;
const TOOL_ICON: f32 = 24.0;
/// Menubar top-level icon column — our extension over Qt, which renders
/// icon-only menubar items (probed: Qt drops the title entirely).
const MENU_ICON: f32 = 16.0;
/// QMenu popup icon size; Qt draws a 16 px icon inside the item.
const POPUP_ICON: f32 = 16.0;
/// Qt reserves the popup text column unconditionally: probed text starts
/// at `item.x + 28` whether or not the menu has icons (iconless popup
/// width = advance + 51, with icons = + 59 — see `menu_width`).
const POPUP_TEXT_X: f32 = 28.0;

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

/// Static parts extracted from a `QMainWindow` `.ui` file (0.14.8).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MainWindowSpec {
    pub title: Option<String>,
    pub menu: Vec<MenuItem>,
    pub toolbar: Vec<MenuItem>,
    pub has_menu_bar: bool,
    pub has_toolbar: bool,
    pub status_visible: bool,
    pub central_class: Option<String>,
    pub central_is_mdi: bool,
}

/// Parse a Qt Designer `QMainWindow` form: menu bar, toolbar, status bar and
/// the central widget. Actions are resolved by `<addaction name="...">`.
pub fn parse_main_window(xml: &str) -> Result<MainWindowSpec, String> {
    let form = form_xml::parse(xml).map_err(|error| error.to_string())?;
    let root = &form.root;
    if root.class != "QMainWindow" && root.class != "QWidget" && root.class != "QDialog" {
        return Err(format!("root widget is {}, not QMainWindow", root.class));
    }

    let mut actions = HashMap::new();
    collect_actions(root, &mut actions);
    let mut menus = HashMap::new();
    collect_menus(root, &mut menus);

    let menu_bar = root
        .children
        .iter()
        .find(|widget| widget.class == "QMenuBar" || widget.name.eq_ignore_ascii_case("menubar"));
    let status_bar = root.children.iter().find(|widget| {
        widget.class == "QStatusBar" || widget.name.eq_ignore_ascii_case("statusbar")
    });
    let toolbars: Vec<&form_xml::Widget> = root
        .children
        .iter()
        .filter(|widget| widget.class == "QToolBar")
        .collect();

    let menu = menu_bar
        .map(|bar| menu_items_from_bar(bar, &actions, &menus))
        .unwrap_or_default();
    let toolbar = toolbar_items(&toolbars, &actions);

    let chrome = |class: &str| {
        matches!(
            class,
            "QMenuBar" | "QStatusBar" | "QToolBar" | "QMenu" | "QDockWidget"
        )
    };
    let central = root
        .children
        .iter()
        .find(|widget| widget.name.eq_ignore_ascii_case("centralwidget"))
        .or_else(|| root.children.iter().find(|widget| !chrome(&widget.class)));

    Ok(MainWindowSpec {
        title: root
            .property("windowTitle")
            .and_then(PropValue::as_str)
            .map(str::to_string),
        has_menu_bar: menu_bar.is_some(),
        has_toolbar: !toolbars.is_empty(),
        status_visible: status_bar.is_some(),
        central_class: central.map(|widget| widget.class.clone()),
        central_is_mdi: central.is_some_and(contains_mdi_area),
        menu,
        toolbar,
    })
}

fn contains_mdi_area(widget: &form_xml::Widget) -> bool {
    widget.class == "QMdiArea"
        || widget.children.iter().any(contains_mdi_area)
        || widget.layout.as_ref().is_some_and(layout_contains_mdi_area)
}

fn layout_contains_mdi_area(layout: &form_xml::Layout) -> bool {
    layout.items.iter().any(|item| match item {
        form_xml::LayoutItem::Widget(widget, _) => contains_mdi_area(widget),
        form_xml::LayoutItem::Layout(layout, _) => layout_contains_mdi_area(layout),
        form_xml::LayoutItem::Spacer(_, _) => false,
    })
}

fn collect_actions(widget: &form_xml::Widget, actions: &mut HashMap<String, form_xml::Action>) {
    for action in &widget.actions {
        actions
            .entry(action.name.clone())
            .or_insert_with(|| action.clone());
    }
    for child in &widget.children {
        collect_actions(child, actions);
    }
}

fn collect_menus<'a>(
    widget: &'a form_xml::Widget,
    menus: &mut HashMap<String, &'a form_xml::Widget>,
) {
    if widget.class == "QMenu" {
        menus.entry(widget.name.clone()).or_insert(widget);
    }
    for child in &widget.children {
        collect_menus(child, menus);
    }
}

fn menu_items_from_bar(
    menu_bar: &form_xml::Widget,
    actions: &HashMap<String, form_xml::Action>,
    menus: &HashMap<String, &form_xml::Widget>,
) -> Vec<MenuItem> {
    let mut out = Vec::new();
    let references: Vec<String> = if menu_bar.add_actions.is_empty() {
        menu_bar
            .children
            .iter()
            .filter(|widget| widget.class == "QMenu")
            .map(|widget| widget.name.clone())
            .collect()
    } else {
        menu_bar.add_actions.clone()
    };
    for (index, reference) in references.iter().enumerate() {
        if let Some(item) =
            resolve_menu_reference(reference, actions, menus, &index.to_string(), true)
        {
            out.push(item);
        }
    }
    out
}

fn resolve_menu_reference(
    reference: &str,
    actions: &HashMap<String, form_xml::Action>,
    menus: &HashMap<String, &form_xml::Widget>,
    path: &str,
    top_level: bool,
) -> Option<MenuItem> {
    if reference == "separator" {
        return Some(MenuItem {
            id: path.to_string(),
            separator: true,
            ..MenuItem::default()
        });
    }
    if let Some(action) = actions.get(reference) {
        return Some(menu_item_from_action(action, path));
    }
    if let Some(menu) = menus.get(reference) {
        let title = menu
            .property("title")
            .and_then(PropValue::as_str)
            .unwrap_or(&menu.name)
            .to_string();
        let children = if menu.add_actions.is_empty() {
            menu.children
                .iter()
                .filter(|widget| widget.class == "QMenu")
                .enumerate()
                .filter_map(|(index, child)| {
                    resolve_menu_reference(
                        &child.name,
                        actions,
                        menus,
                        &format!("{path}/{index}"),
                        false,
                    )
                })
                .collect()
        } else {
            menu.add_actions
                .iter()
                .enumerate()
                .filter_map(|(index, child)| {
                    resolve_menu_reference(child, actions, menus, &format!("{path}/{index}"), false)
                })
                .collect()
        };
        return Some(MenuItem {
            id: path.to_string(),
            title,
            icon: property_icon(menu),
            enabled: menu
                .property("enabled")
                .and_then(PropValue::as_bool)
                .unwrap_or(true),
            children,
            ..MenuItem::default()
        });
    }
    // Unknown references are rendered by their object name so that a broken
    // `.ui` does not silently drop a whole menu row.
    if top_level {
        Some(MenuItem {
            id: path.to_string(),
            title: reference.to_string(),
            ..MenuItem::default()
        })
    } else {
        None
    }
}

fn menu_item_from_action(action: &form_xml::Action, path: &str) -> MenuItem {
    let text = action
        .property("text")
        .and_then(PropValue::as_str)
        .unwrap_or(&action.name)
        .to_string();
    let action_name = action
        .property("action")
        .and_then(PropValue::as_str)
        .map(str::to_string);
    let lua_file = action
        .property("lua_file")
        .and_then(PropValue::as_str)
        .map(str::to_string);
    let action_name = action_name.or_else(|| lua_file.as_ref().map(|_| "open_lua".to_string()));
    MenuItem {
        id: path.to_string(),
        title: text,
        icon: property_icon_from(&action.properties),
        action: action_name,
        lua_file,
        ui_file: action
            .property("ui_file")
            .and_then(PropValue::as_str)
            .map(str::to_string),
        mdi: action
            .property("mdi")
            .and_then(PropValue::as_bool)
            .unwrap_or(false),
        modal: action
            .property("modal")
            .and_then(PropValue::as_bool)
            .unwrap_or(false),
        modal_scope: action
            .property("modal_scope")
            .and_then(PropValue::as_str)
            .unwrap_or("application")
            .to_string(),
        enabled: action
            .property("enabled")
            .and_then(PropValue::as_bool)
            .unwrap_or(true),
        separator: action.name == "separator"
            || action
                .property("separator")
                .and_then(PropValue::as_bool)
                .unwrap_or(false),
        checkable: action
            .property("checkable")
            .and_then(PropValue::as_bool)
            .unwrap_or(false),
        checked: action
            .property("checked")
            .and_then(PropValue::as_bool)
            .unwrap_or(false),
        children: Vec::new(),
    }
}

fn toolbar_items(
    toolbars: &[&form_xml::Widget],
    actions: &HashMap<String, form_xml::Action>,
) -> Vec<MenuItem> {
    let mut out = Vec::new();
    for toolbar in toolbars {
        if !out.is_empty() {
            out.push(MenuItem {
                id: format!("separator/{}", out.len()),
                separator: true,
                ..MenuItem::default()
            });
        }
        for (index, reference) in toolbar.add_actions.iter().enumerate() {
            let path = format!("{}/{}", out.len(), index);
            if reference == "separator" {
                out.push(MenuItem {
                    id: path,
                    separator: true,
                    ..MenuItem::default()
                });
            } else if let Some(action) = actions.get(reference) {
                out.push(menu_item_from_action(action, &path));
            }
        }
    }
    out
}

fn property_icon(widget: &form_xml::Widget) -> Option<String> {
    property_icon_from(&widget.properties)
}

fn property_icon_from(properties: &[(String, PropValue)]) -> Option<String> {
    let value = properties
        .iter()
        .find(|(key, _)| key == "icon")
        .map(|(_, value)| value)?;
    match value {
        PropValue::Map(entries) => entries
            .iter()
            .find(|(key, _)| key == "normaloff")
            .or_else(|| entries.first())
            .and_then(|(_, value)| value.as_str())
            .map(str::to_string),
        PropValue::Str(spec) => Some(spec.clone()),
        _ => None,
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
    /// Item under the pointer inside an open popup. Qt draws it with the
    /// selected look: highlight fill, darkened-highlight border, white text.
    pub(crate) hovered_path: Option<Vec<usize>>,
}

impl Shell {
    pub fn set_menu(&mut self, value: &PropValue) {
        self.set_menu_items(items_from_value(value));
    }

    pub fn set_menu_items(&mut self, items: Vec<MenuItem>) {
        self.menu = items;
        self.open_path = None;
        self.hovered_path = None;
    }

    pub fn set_toolbar(&mut self, value: &PropValue) {
        self.set_toolbar_items(items_from_value(value));
    }

    pub fn set_toolbar_items(&mut self, items: Vec<MenuItem>) {
        self.toolbar = items;
        self.toolbar_visible = !self.toolbar.is_empty();
    }

    /// Apply static `QMainWindow.ui` parts. Later Lua `menu`/`toolBar`
    /// properties replace them through the normal setters.
    pub fn apply_main_window(&mut self, spec: MainWindowSpec) {
        self.set_menu_items(spec.menu);
        self.set_toolbar_items(spec.toolbar);
        self.status_visible = spec.status_visible;
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
        self.hovered_path = None;
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
        let mut x = viewport.x + 6.0;
        let mut out = Vec::with_capacity(self.menu.len());
        for item in &self.menu {
            if item.separator {
                continue;
            }
            // Qt menubar item = title advance + 8 px (probe: "Файл" = 47 px
            // at advance 39). Top-level icons widen only their own item.
            let mut width = metrics.horizontal_advance(&item.title).ceil() + 8.0;
            if item.icon.is_some() {
                width += MENU_ICON + 4.0;
            }
            out.push(QRectF::new(x, viewport.y, width, MENU_H));
            x += width + 6.0;
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
            // Root popup drops below the menubar item; cascading submenus
            // align with the parent item's top edge (Qt `QMenu` tear-off).
            let mut y = if depth == 0 {
                anchor.bottom()
            } else {
                anchor.y
            };
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
        let icon_extra = if items
            .iter()
            .any(|item| !item.separator && item.icon.is_some())
        {
            8.0
        } else {
            0.0
        };
        items
            .iter()
            .map(|item| {
                if item.separator {
                    8.0
                } else {
                    metrics.horizontal_advance(&item.title) + 51.0 + icon_extra
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
                    self.hovered_path = None;
                    return Some(item);
                }
                if self.open_path.as_ref().and_then(|p| p.first()) == Some(&index) {
                    self.open_path = None;
                } else {
                    self.open_path = Some(vec![index]);
                }
                self.hovered_path = None;
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
                    self.hovered_path = None;
                    return Some(item);
                }
                self.open_path = Some(path);
                self.hovered_path = None;
                None
            }
            ShellHit::Toolbar(index) => self
                .toolbar
                .get(index)
                .filter(|item| item.enabled && !item.separator)
                .cloned(),
        }
    }

    /// Qt menu tracking: with a menu open, moving the pointer over another
    /// menubar item switches the active item; popup items get the selected
    /// look. Without an open menu the menubar hover is deliberately inert
    /// — Qt6 Fusion does not highlight plain hover (probed: `menubar_hover`
    /// is pixel-identical to `menubar_normal`).
    pub fn hover(&mut self, point: (f32, f32), viewport: QRectF) {
        match self.hit(point, viewport) {
            Some(ShellHit::MenuTop(index)) => {
                if self.menu_open() {
                    let active = self.open_path.as_ref().and_then(|p| p.first()).copied();
                    if active != Some(index)
                        && self.menu.get(index).is_some_and(|item| !item.children.is_empty())
                    {
                        self.open_path = Some(vec![index]);
                    }
                }
                self.hovered_path = None;
            }
            Some(ShellHit::MenuItem(path)) => {
                if self.menu_open() {
                    if let Some(item) = self.item_at(&path) {
                        if !item.children.is_empty() && item.enabled {
                            // Cascade: keep this item's submenu open.
                            self.open_path = Some(path.clone());
                        } else if path.len() > 1 {
                            // Leaf (or disabled branch): close deeper cascades.
                            // Path `[root, …, item]` → open parent column only.
                            self.open_path = Some(path[..path.len() - 1].to_vec());
                        }
                    }
                }
                self.hovered_path = Some(path);
            }
            _ => self.hovered_path = None,
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
        let mut x = viewport.x + 13.0;
        let mut out = Vec::new();
        for item in &self.toolbar {
            if item.separator {
                out.push(QRectF::new(x, y + 4.0, 6.0, TOOLBAR_H - 8.0));
                x += 7.0;
                continue;
            }
            let text_width = metrics.horizontal_advance(&item.title);
            let width = (text_width + 44.0).max(32.0);
            out.push(QRectF::new(x, y + 4.0, width, TOOLBAR_H - 8.0));
            x += width + 1.0;
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
        // Qt menubar bottom line (`menubar_normal.png`, row y=21).
        let menu_border = QColor::rgba8(214, 214, 214, 255);
        let highlight = QColor::rgba8(48, 140, 198, 255);
        // Qt active menubar item: 1 px border = highlight darkened to 80 %.
        let highlight_border = QColor::rgba8(38, 112, 158, 255);

        // Menu bar.
        if !self.menu.is_empty() {
            painter.fill_rect(
                QRectF::new(viewport.x, viewport.y, viewport.width, MENU_H),
                button,
            );
            painter.fill_rect(
                QRectF::new(viewport.x, viewport.y + MENU_H - 1.0, viewport.width, 1.0),
                menu_border,
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
                let active = open_root == Some(menu_index);
                if active {
                    // Qt6 `menubar_active.png`: darkened-highlight frame
                    // around the whole item, highlight fill inside.
                    painter.fill_rect(rect, highlight_border);
                    painter.fill_rect(
                        QRectF::new(
                            rect.x + 1.0,
                            rect.y + 1.0,
                            rect.width - 2.0,
                            rect.height - 2.0,
                        ),
                        highlight,
                    );
                }
                // Qt text column: rect.x + 4 (probed: glyph starts at 10 in
                // the item at x=6), vertically 2 px above our old offset.
                let mut text_x = rect.x + 4.0;
                if let Some(icon) = item.icon.as_deref().and_then(icons::pixmap) {
                    painter.draw_pixmap(
                        QRectF::new(
                            rect.x + 4.0,
                            rect.y + (MENU_H - MENU_ICON) / 2.0,
                            MENU_ICON,
                            MENU_ICON,
                        ),
                        icon,
                        1.0,
                    );
                    text_x = rect.x + 4.0 + MENU_ICON + 4.0;
                }
                let color = if active {
                    QColor::rgba8(255, 255, 255, 255)
                } else if item.enabled {
                    text
                } else {
                    disabled
                };
                let text_rect = QRectF::new(
                    text_x,
                    rect.y + 1.0,
                    (rect.right() - text_x - 4.0).max(0.0),
                    MENU_H,
                );
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
                    // Qt6 ground truth (`qt6_mainwindow.png`, QToolBar
                    // separator action at x=178, y=4, 6x31): two crisp
                    // 1 px lines at the rect centre — dark (0xd9) left,
                    // light right — inset by 6 px top and bottom, not a
                    // solid block.
                    let x = rect.x.floor() + (rect.width.floor() / 2.0);
                    let top = rect.y + 6.0;
                    let height = (rect.height - 12.0).max(1.0);
                    painter.fill_rect(
                        QRectF::new(x, top, 1.0, height),
                        QColor::rgb(0xd9, 0xd9, 0xd9),
                    );
                    painter.fill_rect(
                        QRectF::new(x + 1.0, top, 1.0, height),
                        QColor::rgb(0xff, 0xff, 0xff),
                    );
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
                // Align from metrics, draw through the viewport edge — never use
                // advance alone as width (glyphon can clip the last glyph).
                let width = metrics.horizontal_advance(info) + 8.0;
                let x = (viewport.right() - width - 8.0).max(left);
                painter.draw_text(
                    QRectF::new(x, y + 3.0, (viewport.right() - x).max(1.0), STATUS_H),
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
        // Qt6 popup ground truth (`menu_popup.png`): white background,
        // 1 px (149) frame per menu column, selected item = highlight fill
        // with a darkened-highlight border, separator = single 1 px (195)
        // line inset 5 px at the top row of its 2 px band.
        let background = QColor::rgba8(255, 255, 255, 255);
        let text = QColor::rgba8(28, 28, 32, 255);
        let disabled = QColor::rgba8(130, 130, 134, 255);
        let frame = QColor::rgba8(149, 149, 149, 255);
        let highlight = QColor::rgba8(48, 140, 198, 255);
        let highlight_border = QColor::rgba8(38, 112, 158, 255);
        let separator_line = QColor::rgba8(195, 195, 195, 255);
        let open = self.open_path.as_deref();
        let hovered = self.hovered_path.as_deref();

        // One frame per menu column: each depth level is a single popup
        // window in Qt (root menu first, then each open submenu). The old
        // per-entry stroke drew a grid across the popup.
        let mut start = 0usize;
        while start < entries.len() {
            let depth = entries[start].path.len();
            let mut end = start;
            while end < entries.len() && entries[end].path.len() == depth {
                end += 1;
            }
            let column = &entries[start..end];
            let first = column.first().expect("non-empty column");
            let last = column.last().expect("non-empty column");
            let union = QRectF::new(
                first.rect.x,
                first.rect.y,
                first.rect.width,
                last.rect.bottom() - first.rect.y,
            );

            for entry in column {
                painter.fill_rect(entry.rect, background);
            }
            painter.stroke_rect(union, 1.0, frame);

            for entry in column {
                if entry.item.separator {
                    painter.fill_rect(
                        QRectF::new(
                            entry.rect.x + 5.0,
                            entry.rect.y,
                            entry.rect.width - 10.0,
                            1.0,
                        ),
                        separator_line,
                    );
                    continue;
                }
                let selected = entry.item.enabled
                    && (hovered.is_some_and(|path| path == entry.path.as_slice())
                        || open.is_some_and(|path| path == entry.path.as_slice()));
                if selected {
                    painter.fill_rect(entry.rect, highlight_border);
                    painter.fill_rect(
                        QRectF::new(
                            entry.rect.x + 1.0,
                            entry.rect.y + 1.0,
                            entry.rect.width - 2.0,
                            entry.rect.height - 2.0,
                        ),
                        highlight,
                    );
                }
                let color = if selected {
                    QColor::rgba8(255, 255, 255, 255)
                } else if entry.item.enabled {
                    text
                } else {
                    disabled
                };
                if let Some(icon) = entry.item.icon.as_deref().and_then(icons::pixmap) {
                    painter.draw_pixmap(
                        QRectF::new(
                            entry.rect.x + 6.0,
                            entry.rect.y + 4.0,
                            POPUP_ICON,
                            POPUP_ICON,
                        ),
                        icon,
                        1.0,
                    );
                }
                if entry.item.checkable {
                    // Qt keeps the check mark inside the icon column, so
                    // titles stay aligned across rows.
                    painter.draw_text(
                        QRectF::new(entry.rect.x + 6.0, entry.rect.y + 4.0, 16.0, ITEM_H),
                        if entry.item.checked { "✓" } else { " " }.to_string(),
                        color,
                    );
                }
                let text_x = entry.rect.x + POPUP_TEXT_X;
                painter.draw_text(
                    QRectF::new(
                        text_x,
                        entry.rect.y + 4.0,
                        (entry.rect.right() - text_x - 6.0).max(0.0),
                        ITEM_H,
                    ),
                    entry.item.title.clone(),
                    color,
                );
                if !entry.item.children.is_empty() {
                    painter.draw_text(
                        QRectF::new(
                            entry.rect.right() - 18.0,
                            entry.rect.y + 4.0,
                            14.0,
                            ITEM_H,
                        ),
                        "›".to_string(),
                        color,
                    );
                }
            }
            start = end;
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
                    PropValue::List(vec![
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("Nested".into())),
                            (
                                "children".into(),
                                PropValue::List(vec![PropValue::Map(vec![
                                    ("title".into(), PropValue::Str("Deep".into())),
                                    ("action".into(), PropValue::Str("open_lua".into())),
                                    ("lua_file".into(), PropValue::Str("deep.lua".into())),
                                ])]),
                            ),
                        ]),
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("About".into())),
                            ("action".into(), PropValue::Str("about".into())),
                        ]),
                    ]),
                ),
            ]),
        ])
    }

    const MAIN_WINDOW_UI: &str = include_str!("../../../../server/priv/qt_forms/mainwindow.ui");

    #[test]
    fn parses_qmainwindow_ui_static_shell() {
        let spec = parse_main_window(MAIN_WINDOW_UI).expect("main window parses");
        assert_eq!(spec.title.as_deref(), Some("EPlatform main window"));
        assert!(spec.has_menu_bar);
        assert!(spec.has_toolbar);
        assert!(spec.status_visible);
        assert_eq!(spec.central_class.as_deref(), Some("QWidget"));
        assert!(spec.central_is_mdi);
        assert_eq!(spec.menu.len(), 2);
        assert_eq!(spec.menu[0].title, "Файл");
        assert_eq!(spec.menu[0].children.len(), 3);
        assert_eq!(spec.menu[0].children[0].title, "Тестовая форма");
        assert_eq!(
            spec.menu[0].children[0].lua_file.as_deref(),
            Some("test_form_move.lua")
        );
        assert_eq!(spec.menu[0].children[0].action.as_deref(), Some("open_lua"));
        assert_eq!(spec.menu[0].children[0].icon.as_deref(), Some("img24:45"));
        assert!(spec.menu[0].children[1].separator);
        assert_eq!(spec.menu[0].children[2].action.as_deref(), Some("exit"));
        assert_eq!(spec.menu[1].children[0].title, "Окна");
        assert_eq!(spec.toolbar.len(), 3);
        assert_eq!(spec.toolbar[0].title, "Тестовая форма");
        assert!(spec.toolbar[1].separator);

        let mut shell = Shell::default();
        shell.visible = true;
        shell.apply_main_window(spec);
        assert!(shell.menu.len() >= 2);
        assert!(shell.toolbar_visible);
        assert!(shell.status_visible);

        // Dynamic `set_menu` replaces the static model.
        shell.set_menu(&PropValue::List(vec![PropValue::Map(vec![(
            "title".into(),
            PropValue::Str("Dynamic".into()),
        )])]));
        assert_eq!(shell.menu.len(), 1);
        assert_eq!(shell.menu[0].title, "Dynamic");
    }

    const QT6_MAINWINDOW: &str = include_str!("../tests/fixtures/qt6_mainwindow.json");

    #[test]
    fn main_window_shell_geometry_matches_qt6_offscreen() {
        let golden: serde_json::Value =
            serde_json::from_str(QT6_MAINWINDOW).expect("main window golden json");
        let golden = &golden["main_window"];
        let number = |value: &serde_json::Value, key: &str| {
            value[key]
                .as_f64()
                .unwrap_or_else(|| panic!("{key} missing"))
        };
        let near = |context: &str, actual: f32, expected: f64| {
            assert!(
                (actual - expected as f32).abs() <= 1.0,
                "{context}: expected {expected}, got {actual}"
            );
        };

        let spec = parse_main_window(MAIN_WINDOW_UI).expect("main window parses");
        let mut shell = Shell::default();
        shell.visible = true;
        shell.apply_main_window(spec);
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);

        let central = shell.central(viewport);
        let golden_central = &golden["central"];
        near("central.x", central.x, number(golden_central, "x"));
        near("central.y", central.y, number(golden_central, "y"));
        near("central.w", central.width, number(golden_central, "w"));
        near("central.h", central.height, number(golden_central, "h"));
        near(
            "statusbar.y",
            viewport.bottom() - STATUS_H,
            number(&golden["statusbar"], "y"),
        );
        near("menu.h", MENU_H, number(&golden["menubar"], "h"));
        near(
            "toolbar.h",
            TOOLBAR_H,
            number(&golden["toolbars"][0]["chrome"], "h"),
        );

        let actions = golden["menubar_actions"]
            .as_array()
            .expect("menubar actions");
        let rects = shell.top_rects(viewport);
        assert_eq!(rects.len(), 2);
        for (index, expected) in actions.iter().enumerate() {
            let rect = rects[index];
            near(
                &format!("menubar[{index}].x"),
                rect.x,
                number(expected, "x"),
            );
            near(
                &format!("menubar[{index}].y"),
                rect.y,
                number(expected, "y"),
            );
            near(
                &format!("menubar[{index}].w"),
                rect.width,
                number(expected, "w"),
            );
            near(
                &format!("menubar[{index}].h"),
                rect.height,
                number(expected, "h"),
            );
        }

        shell.open_path = Some(vec![0]);
        let entries = shell.menu_entries(viewport);
        let menu = &golden["menus"][0];
        let expected_actions = menu["actions"].as_array().expect("menu actions");
        assert_eq!(entries.len(), expected_actions.len());
        let origin_x = entries[0].rect.x;
        let origin_y = entries[0].rect.y;
        for (index, (entry, expected)) in entries.iter().zip(expected_actions).enumerate() {
            near(
                &format!("menu[{index}].x"),
                entry.rect.x - origin_x,
                number(expected, "x"),
            );
            near(
                &format!("menu[{index}].y"),
                entry.rect.y - origin_y,
                number(expected, "y"),
            );
            near(
                &format!("menu[{index}].w"),
                entry.rect.width,
                number(expected, "w"),
            );
            near(
                &format!("menu[{index}].h"),
                entry.rect.height,
                number(expected, "h"),
            );
        }
        let menu_height: f32 = entries.iter().map(|entry| entry.rect.height).sum();
        near("menu.popup.h", menu_height, number(menu, "hint_h"));
        near(
            "menu.popup.w",
            entries[0].rect.width,
            number(menu, "hint_w"),
        );

        let expected_toolbar = golden["toolbars"][0]["actions"]
            .as_array()
            .expect("toolbar actions");
        let toolbar = shell.toolbar_rects(viewport);
        assert_eq!(toolbar.len(), expected_toolbar.len());
        let toolbar_origin_x = number(&golden["toolbars"][0]["chrome"], "x") as f32;
        let toolbar_origin_y = number(&golden["toolbars"][0]["chrome"], "y") as f32;
        for (index, expected) in expected_toolbar.iter().enumerate() {
            let rect = toolbar[index];
            near(
                &format!("toolbar[{index}].x"),
                rect.x - toolbar_origin_x,
                number(expected, "x"),
            );
            near(
                &format!("toolbar[{index}].y"),
                rect.y - toolbar_origin_y,
                number(expected, "y"),
            );
            near(
                &format!("toolbar[{index}].w"),
                rect.width,
                number(expected, "w"),
            );
            near(
                &format!("toolbar[{index}].h"),
                rect.height,
                number(expected, "h"),
            );
        }
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
        assert!(
            (deep.rect.y - nested.rect.y).abs() < 0.01,
            "cascading submenu must align with the parent item top, nested.y={} deep.y={}",
            nested.rect.y,
            deep.rect.y
        );
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

    #[test]
    fn toolbar_separator_paints_qt_two_line_primitive() {
        use rqt::gui::QPaintCommand;

        let mut shell = Shell::default();
        shell.visible = true;
        shell.toolbar = vec![
            MenuItem {
                title: "First".into(),
                ..Default::default()
            },
            MenuItem {
                separator: true,
                ..Default::default()
            },
            MenuItem {
                title: "Second".into(),
                ..Default::default()
            },
        ];
        shell.toolbar_visible = true;
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let separator = shell.toolbar_rects(viewport)[1];
        assert!((separator.width - 6.0).abs() < 0.01);

        let mut painter = QPainter::new();
        shell.paint_base(&mut painter, viewport, 0.0, None);

        // Qt6 renders the toolbar separator as two crisp 1 px lines
        // (dark left, light right) inset 6 px from the action rect —
        // never as a solid block over the full 6x31 area.
        let mut lines = Vec::new();
        for command in painter.commands() {
            if let QPaintCommand::FillRect { rect, color } = command {
                if (rect.width - 1.0).abs() < 0.01 {
                    lines.push((*rect, *color));
                }
            }
        }
        assert_eq!(lines.len(), 2, "exactly two 1px lines, got {lines:?}");
        let x = separator.x.floor() + (separator.width.floor() / 2.0);
        let y = separator.y + 6.0;
        let height = separator.height - 12.0;
        assert_eq!(
            lines[0].0,
            QRectF::new(x, y, 1.0, height),
            "dark line position"
        );
        assert_eq!(lines[0].1, QColor::rgb(0xd9, 0xd9, 0xd9));
        assert_eq!(
            lines[1].0,
            QRectF::new(x + 1.0, y, 1.0, height),
            "light line position"
        );
        assert_eq!(lines[1].1, QColor::rgb(0xff, 0xff, 0xff));

        // No solid 6-wide block covering the separator.
        let solid = painter.commands().iter().any(|command| {
            matches!(
                command,
                QPaintCommand::FillRect { rect, .. }
                    if (rect.width - separator.width).abs() < 0.01
                        && (rect.height - separator.height).abs() < 0.01
                        && (rect.x - separator.x).abs() < 1.5
                        && (rect.y - separator.y).abs() < 1.5
            )
        });
        assert!(!solid, "separator must not be filled as one block");
    }

    const QT6_MENUBAR_NORMAL: &[u8] =
        include_bytes!("../tests/fixtures/qt6_chrome/menubar_normal.png");
    const QT6_MENUBAR_ACTIVE: &[u8] =
        include_bytes!("../tests/fixtures/qt6_chrome/menubar_active.png");
    const QT6_POPUP_NOICON: &[u8] =
        include_bytes!("../tests/fixtures/qt6_chrome/menu_popup_noicon.png");

    /// Compare our raster against a Qt6 offscreen PNG. `offset` is the
    /// position of the compared region inside our image.
    fn compare_region_to_qt(
        name: &str,
        image: &rqt::raster::QRasterImage,
        offset: (u32, u32),
        fixture: &[u8],
        mean_limit: f64,
        diff_limit: u64,
    ) {
        let reference = rqt::raster::QRasterImage::from_png_bytes(fixture)
            .unwrap_or_else(|error| panic!("{name}: reference png: {error}"));
        let mut total = 0u64;
        let mut count = 0u64;
        let mut different = 0u64;
        for y in 0..reference.height {
            for x in 0..reference.width {
                let left = image.pixel(x + offset.0, y + offset.1);
                let right = reference.pixel(x, y);
                let mut pixel_diff = 0u32;
                for channel in 0..3 {
                    let diff = u32::from(left[channel].abs_diff(right[channel]));
                    total += u64::from(diff);
                    pixel_diff = pixel_diff.max(diff);
                    count += 1;
                }
                if pixel_diff > 30 {
                    different += 1;
                }
            }
        }
        let mean = total as f64 / count as f64;
        eprintln!(
            "{name}: mean channel diff {mean:.3}, pixels >30: {different} / {}",
            reference.width * reference.height
        );
        assert!(mean < mean_limit, "{name}: mean diff {mean}");
        assert!(
            different < diff_limit,
            "{name}: pixels >30 {different} >= {diff_limit}"
        );
    }

    fn menubar_shell() -> Shell {
        let spec = parse_main_window(MAIN_WINDOW_UI).expect("main window parses");
        let mut shell = Shell::default();
        shell.visible = true;
        shell.apply_main_window(spec);
        shell
    }

    fn strip_menu_icons(items: &mut [MenuItem]) {
        for item in items.iter_mut() {
            item.icon = None;
            strip_menu_icons(&mut item.children);
        }
    }

    #[test]
    fn menubar_normal_raster_matches_qt6_offscreen() {
        let shell = menubar_shell();
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let mut painter = QPainter::new();
        shell.paint_base(&mut painter, viewport, 0.0, None);
        let image = rqt::raster::rasterize(
            painter.commands(),
            800,
            MENU_H as u32,
            QColor::rgb(0xef, 0xef, 0xef),
        );
        compare_region_to_qt("menubar_normal", &image, (0, 0), QT6_MENUBAR_NORMAL, 4.0, 900);
    }

    #[test]
    fn menubar_active_raster_matches_qt6_offscreen() {
        let mut shell = menubar_shell();
        shell.open_path = Some(vec![0]);
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let mut painter = QPainter::new();
        shell.paint_base(&mut painter, viewport, 0.0, None);
        let image = rqt::raster::rasterize(
            painter.commands(),
            800,
            MENU_H as u32,
            QColor::rgb(0xef, 0xef, 0xef),
        );
        compare_region_to_qt("menubar_active", &image, (0, 0), QT6_MENUBAR_ACTIVE, 4.0, 900);
    }

    #[test]
    fn popup_without_icons_raster_matches_qt6_offscreen() {
        // Qt fixture: the same three rows ("Тестовая форма", separator,
        // "Выход") without resolvable icons, first action highlighted.
        let mut shell = menubar_shell();
        strip_menu_icons(&mut shell.menu);
        shell.open_path = Some(vec![0]);
        shell.hovered_path = Some(vec![0, 0]);
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);

        let entries = shell.menu_entries(viewport);
        assert_eq!(entries.len(), 3);
        assert!((entries[0].rect.width - 171.0).abs() <= 1.0, "popup w");
        let origin = shell.top_rect(viewport, 0).expect("top rect");
        assert!((origin.x - 6.0).abs() < 0.01);

        let mut painter = QPainter::new();
        shell.paint_overlay(&mut painter, viewport);
        let image = rqt::raster::rasterize(
            painter.commands(),
            800,
            400,
            QColor::rgb(0xef, 0xef, 0xef),
        );
        compare_region_to_qt(
            "popup_noicon",
            &image,
            (origin.x as u32, MENU_H as u32),
            QT6_POPUP_NOICON,
            6.0,
            700,
        );
    }

    #[test]
    fn menubar_hover_tracks_open_menu_like_qt() {
        let mut shell = Shell::default();
        shell.visible = true;
        shell.set_menu(&menu_value());
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);

        // Menu closed: hovering the menubar must not open anything (Qt6
        // Fusion parity — no highlight, no auto-open).
        let file = shell.top_rect(viewport, 0).expect("file rect");
        shell.hover((file.x + 2.0, 5.0), viewport);
        assert_eq!(shell.open_path, None);
        assert_eq!(shell.hovered_path, None);

        // Menu open on "File": moving over "View" switches the active menu.
        shell.open_path = Some(vec![0]);
        let view = shell.top_rect(viewport, 1).expect("view rect");
        shell.hover((view.x + 2.0, 5.0), viewport);
        assert_eq!(shell.open_path, Some(vec![1]));

        // A popup leaf under the pointer becomes the selected item.
        shell.open_path = Some(vec![0]);
        let (open_path, open_rect) = shell
            .menu_entries(viewport)
            .into_iter()
            .find(|entry| entry.item.title == "Open")
            .map(|entry| (entry.path, entry.rect))
            .expect("open entry");
        shell.hover((open_rect.x + 4.0, open_rect.y + 4.0), viewport);
        assert_eq!(shell.hovered_path, Some(open_path));
        assert_eq!(shell.open_path, Some(vec![0]), "leaf hover keeps the menu");

        // Cascading submenu closes when the pointer moves to a leaf sibling.
        shell.open_path = Some(vec![1]);
        let nested = shell
            .menu_entries(viewport)
            .into_iter()
            .find(|entry| entry.item.title == "Nested")
            .expect("nested entry");
        shell.hover((nested.rect.x + 4.0, nested.rect.y + 4.0), viewport);
        assert_eq!(shell.open_path.as_deref(), Some(nested.path.as_slice()));
        let leaf = shell
            .menu_entries(viewport)
            .into_iter()
            .find(|entry| entry.item.title == "About")
            .expect("about entry");
        shell.hover((leaf.rect.x + 4.0, leaf.rect.y + 4.0), viewport);
        assert_eq!(shell.hovered_path.as_deref(), Some(leaf.path.as_slice()));
        assert_eq!(
            shell.open_path,
            Some(vec![1]),
            "leaf sibling closes the cascade"
        );
        assert!(
            shell
                .menu_entries(viewport)
                .iter()
                .all(|entry| entry.item.title != "Deep"),
            "cascading submenu must dismiss"
        );

        // Moving out of the popup clears the selection but keeps the menu.
        shell.open_path = Some(vec![0]);
        shell.hover((400.0, 300.0), viewport);
        assert_eq!(shell.hovered_path, None);
        assert_eq!(shell.open_path, Some(vec![0]), "menu stays open");
    }
}
