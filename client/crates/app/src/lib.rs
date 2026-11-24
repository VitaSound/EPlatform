//! EPlatform browser shell: winit window + wgpu renderer + Phoenix channel.
//! Stages 3–5: Qt `.ui` XML → widget tree → box layout → rect batch
//! render, the event loop (mouse/keyboard → Lua → `setProperty`) and
//! the virtualized `QTableWidget`.

mod gfx;
mod socket;

use rqt::item_model::{QTABLE_HEADER_H, QTABLE_ROW_H};
use rqt::mdi::{QMdiArea, QMdiSubWindow, QMdiSubWindowHit, QMDI_BUTTON, QMDI_TITLE_H};
use rqt::widgets::{QWidgetId, QWidgetTree};
use socket::{PhoenixSocket, SocketEvent};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// In-progress cell edit of a `QTableWidget`: the buffer lives on the
/// client until Enter/blur commits it (only then does it reach Lua).
#[derive(Debug, Clone)]
struct CellEdit {
    form_id: String,
    table: String,
}

const CARD_IDLE: [f32; 4] = [0.76, 0.78, 0.82, 1.0];
const CARD_BORDER: [f32; 4] = [0.58, 0.61, 0.66, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MdiDragMode {
    Move,
    Resize,
}

#[derive(Debug, Clone)]
struct Drag {
    index: usize,
    mode: MdiDragMode,
    last: (f32, f32),
}

#[derive(Debug, Clone)]
struct ColumnDrag {
    form_id: String,
    name: String,
    column: usize,
    last_x: f32,
}

#[derive(Debug, Clone)]
struct LineSelect {
    form_id: String,
    name: String,
}

struct QtForm {
    tree: QWidgetTree,
    root: QWidgetId,
}

#[derive(Debug, Clone)]
struct TablePress {
    form_id: String,
    name: String,
    already_selected: bool,
}

#[derive(Clone, Debug)]
struct Params {
    backend: String,
    app: String,
    lua: String,
    ui: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            backend: "ws://127.0.0.1:4000/socket/websocket?vsn=2.0.0".into(),
            app: "demo".into(),
            lua: "test_01.lua".into(),
            ui: "test_dialog01.ui".into(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Params {
    fn from_location() -> Self {
        let mut params = Params::default();
        let mut has_app = false;
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(search) = location.search() {
                for pair in search.trim_start_matches('?').split('&') {
                    let Some((key, value)) = pair.split_once('=') else {
                        continue;
                    };
                    let value = value.to_string();
                    match key {
                        "backend" => {
                            params.backend = value;
                        }
                        "app" => {
                            params.app = value;
                            has_app = true;
                        }
                        "lua" => params.lua = value,
                        "ui" => params.ui = value,
                        _ => {}
                    }
                }
            }
            if !has_app {
                params.app = unique_app_id();
            }
            if params.backend.starts_with("ws://127.0.0.1") {
                // Default backend follows the page host so remote browsers work.
                if let Ok(host) = location.hostname() {
                    if !host.is_empty() && host != "localhost" {
                        params.backend = format!("ws://{host}:4000/socket/websocket?vsn=2.0.0");
                    }
                }
            }
        }
        params
    }
}

#[cfg(target_arch = "wasm32")]
fn unique_app_id() -> String {
    let mut bytes = [0u8; 16];
    let mut filled = false;
    if let Some(window) = web_sys::window() {
        if let Ok(crypto) = window.crypto() {
            filled = crypto.get_random_values_with_u8_array(&mut bytes).is_ok();
        }
    }
    if !filled {
        for byte in &mut bytes {
            *byte = (js_sys::Math::random() * 256.0) as u8;
        }
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

struct App {
    params: Params,
    window: Option<Arc<Window>>,
    gfx: Rc<RefCell<Option<gfx::Gfx>>>,
    rx: Receiver<SocketEvent>,
    socket: Option<PhoenixSocket>,
    joined: bool,
    start_sent: bool,
    event_count: u32,
    /// Last known cursor position in canvas pixels (for hit-testing;
    /// `winit::MouseInput` does not carry a position).
    cursor: (f32, f32),
    /// Widget with keyboard focus: `(form_id, objectName)`. Only
    /// editable widgets (`QLineEdit`) take focus, like tabbing in Qt.
    focused: Option<(String, String)>,
    /// Last clicked table: `(form_id, objectName)` — typing starts
    /// editing its current cell (Qt's `AnyKeyPressed` edit trigger).
    table_focus: Option<(String, String)>,
    /// In-progress cell edit of `table_focus`.
    editing: Option<CellEdit>,
    /// Laid-out Qt widget trees by `form_id` (one per open dialog session).
    forms: HashMap<String, QtForm>,
    mdi: QMdiArea<String>,
    dragging: Option<Drag>,
    column_drag: Option<ColumnDrag>,
    line_select: Option<LineSelect>,
    left_down: bool,
    shift: bool,
    ctrl: bool,
    caret_epoch: f64,
    table_press: Option<TablePress>,
    scroll_click: bool,
}

impl App {
    fn new(params: Params) -> Self {
        let (_tx, rx) = channel();
        Self {
            params,
            window: None,
            gfx: Rc::new(RefCell::new(None)),
            rx,
            socket: None,
            joined: false,
            start_sent: false,
            event_count: 0,
            cursor: (0.0, 0.0),
            focused: None,
            table_focus: None,
            editing: None,
            forms: HashMap::new(),
            mdi: QMdiArea::new(rqt::gui::QRectF::new(0.0, 0.0, 1280.0, 800.0)),
            dragging: None,
            column_drag: None,
            line_select: None,
            left_down: false,
            shift: false,
            ctrl: false,
            caret_epoch: now_ms(),
            table_press: None,
            scroll_click: false,
        }
    }

    fn ui_scale(&self) -> f32 {
        self.gfx
            .borrow()
            .as_ref()
            .map(|gfx| gfx.ui_scale())
            .or_else(|| self.window.as_ref().map(|w| w.scale_factor() as f32))
            .unwrap_or(1.0)
    }

    fn log(msg: impl AsRef<str>) {
        println!("[app] {}", msg.as_ref());
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&msg.as_ref().into());
            // Mirror logs into window.__epLogs for automated inspection.
            if let Some(window) = web_sys::window() {
                use wasm_bindgen::JsCast;
                let win: wasm_bindgen::JsValue = window.into();
                let logs = match js_sys::Reflect::get(&win, &"__epLogs".into()) {
                    Ok(value) if value.is_array() => value.unchecked_into::<js_sys::Array>(),
                    _ => {
                        let array = js_sys::Array::new();
                        let _ = js_sys::Reflect::set(&win, &"__epLogs".into(), &array);
                        array
                    }
                };
                logs.push(&msg.as_ref().into());
            }
        }
    }

    fn drain_socket_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                SocketEvent::Opened => Self::log("websocket opened"),
                SocketEvent::Joined => {
                    Self::log("channel joined");
                    self.joined = true;
                }
                SocketEvent::Command(payload) => {
                    Self::log(format!("command: {payload}"));
                    self.on_command(&payload);
                }
                SocketEvent::Closed => {
                    Self::log("websocket closed");
                    self.joined = false;
                    self.start_sent = false;
                }
                SocketEvent::Error(err) => Self::log(format!("websocket error: {err}")),
            }
        }

        // Once joined, ask the server to start the demo dialog.
        if self.joined && !self.start_sent {
            if let Some(socket) = &self.socket {
                socket.send(
                    "start_dialog",
                    serde_json::json!({
                        "lua_file": self.params.lua,
                        "ui_file": self.params.ui,
                    }),
                );
                Self::log("start_dialog sent");
                self.start_sent = true;
            }
        }
    }

    /// Command dispatch into the widget tree.
    fn on_command(&mut self, payload: &serde_json::Value) {
        let command = payload
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let form_id = payload
            .get("form_id")
            .and_then(|f| f.as_str())
            .unwrap_or("");
        match command {
            "createForm" => self.on_create_form(payload),
            "setProperty" => {
                let object_name = str_field(payload, "object_name");
                let property = str_field(payload, "property");
                let Some(value) = prop_from_json(payload.get("value")) else {
                    return;
                };
                if let Some(form) = self.forms.get_mut(form_id) {
                    if let Some(widget) = form.tree.find_widget(object_name) {
                        let class = form
                            .tree
                            .widget(widget)
                            .map(|widget| widget.class_name.clone())
                            .unwrap_or_default();
                        form.tree.set_property(
                            widget,
                            property,
                            rqt::core::QVariant::from_prop(&value),
                        );
                        form.tree.relayout(form.root);
                        if class == "QTableWidget" {
                            if let Some(node) = form.tree.widget(widget) {
                                let rect = node.geometry;
                                let font = node.font.clone();
                                if let Some(table) = form.tree.table_mut(widget) {
                                    let viewport = table.scroll_area(rect, &font).viewport;
                                    if property == "currentRow" {
                                        table.reveal_current_row(viewport.height);
                                    }
                                    table.sync_scrollbars(rect, &font);
                                }
                            }
                            Self::log(format!(
                                "table {object_name}.{property}={}",
                                prop_brief(&value)
                            ));
                        } else {
                            if class == "QListWidget" {
                                if let Some(node) = form.tree.widget(widget) {
                                    let rect = node.geometry;
                                    if let Some(list) = form.tree.list_mut(widget) {
                                        let layout = list.scroll_area(rect);
                                        if let Some(current) = list.current {
                                            list.ensure_visible(current, layout.viewport.height);
                                        }
                                        list.sync_bars(layout.viewport.size());
                                    }
                                }
                            } else if class == "QTreeWidget" {
                                if let Some(node) = form.tree.widget(widget) {
                                    let rect = node.geometry;
                                    if let Some(tree) = form.tree.tree_mut(widget) {
                                        let layout = tree.scroll_area(rect);
                                        let body_height = (layout.viewport.height
                                            - rqt::item_view::QVIEW_HEADER_H)
                                            .max(0.0);
                                        if let Some(current) = tree.current {
                                            tree.ensure_visible(current, body_height);
                                        }
                                        tree.sync_bars(rqt::gui::QSizeF::new(
                                            layout.viewport.width,
                                            body_height,
                                        ));
                                    }
                                }
                            }
                            Self::log(format!(
                                "apply {object_name}.{property}={}",
                                prop_brief(&value)
                            ));
                        }
                    }
                }
            }
            "setFormProperty" => {
                let property = str_field(payload, "property");
                let Some(value) = prop_from_json(payload.get("value")) else {
                    return;
                };
                let mut title = None;
                if let Some(form) = self.forms.get_mut(form_id) {
                    if let form_xml::PropValue::Rect { x, y, w, h } = &value {
                        let rect = rqt::gui::QRectF::new(*x as f32, *y as f32, *w as f32, *h as f32);
                        if let Some(index) = self.mdi.index_of(|payload| payload == form_id) {
                            self.mdi.set_content_geometry(index, rect);
                        }
                        form.tree.set_geometry(form.root, rect);
                    } else {
                        form.tree.set_property(
                            form.root,
                            property,
                            rqt::core::QVariant::from_prop(&value),
                        );
                        form.tree.relayout(form.root);
                    }
                    title = Some(form_title(&form.tree, form.root, form_id));
                    Self::log(format!("apply form.{property}={}", prop_brief(&value)));
                }
                if property == "windowTitle" {
                    if let (Some(title), Some(index)) =
                        (title, self.mdi.index_of(|payload| payload == form_id))
                    {
                        self.mdi.set_title(index, title);
                    }
                }
            }
            // Lua is blocked waiting for the current widget value.
            "getProperty" => {
                let ref_id = payload.get("ref").cloned().unwrap_or_default();
                let object_name = str_field(payload, "object_name");
                let property = str_field(payload, "property");
                let value = self
                    .forms
                    .get(form_id)
                    .and_then(|form| {
                        let widget = form.tree.find_widget(object_name)?;
                        form.tree
                            .get_table_property(widget, property)
                            .or_else(|| form.tree.property(widget, property).cloned())
                    })
                    .and_then(|value| value.to_prop())
                    .map(|value| prop_to_json(&value))
                    .unwrap_or(serde_json::Value::Null);
                if let Some(socket) = &self.socket {
                    socket.send(
                        "send_event",
                        serde_json::json!({
                            "command": "getProperty",
                            "form_id": form_id,
                            "ref": ref_id,
                            "value": value,
                        }),
                    );
                }
                Self::log(format!("reply {object_name}.{property}={value}"));
            }
            "getFormProperty" => {
                let ref_id = payload.get("ref").cloned().unwrap_or_default();
                let property = str_field(payload, "property");
                let value = self
                    .forms
                    .get(form_id)
                    .and_then(|form| form.tree.property(form.root, property).cloned())
                    .and_then(|value| value.to_prop())
                    .map(|value| prop_to_json(&value))
                    .unwrap_or(serde_json::Value::Null);
                if let Some(socket) = &self.socket {
                    socket.send(
                        "send_event",
                        serde_json::json!({
                            "command": "getProperty",
                            "form_id": form_id,
                            "ref": ref_id,
                            "value": value,
                        }),
                    );
                }
                Self::log(format!("reply form.{property}={value}"));
            }
            "dialogError" => {
                Self::log(format!(
                    "dialogError: {}",
                    payload.get("error").and_then(|e| e.as_str()).unwrap_or("")
                ));
            }
            "closeForm" => {
                self.remove_form(form_id);
                Self::log(format!("closeForm form_id={form_id}"));
            }
            "closeApplication" => {
                self.remove_all_forms();
                Self::log("closeApplication");
            }
            _ => {}
        }
    }

    fn remove_form(&mut self, form_id: &str) {
        self.forms.remove(form_id);
        if let Some(index) = self.mdi.index_of(|payload| payload == form_id) {
            self.mdi.remove(index);
        }
        if self.focused.as_ref().is_some_and(|(f, _)| f == form_id) {
            self.focused = None;
        }
        if self.table_focus.as_ref().is_some_and(|(f, _)| f == form_id) {
            self.table_focus = None;
        }
        if self.editing.as_ref().is_some_and(|e| e.form_id == form_id) {
            self.editing = None;
        }
        if self
            .column_drag
            .as_ref()
            .is_some_and(|drag| drag.form_id == form_id)
        {
            self.column_drag = None;
        }
        if self
            .line_select
            .as_ref()
            .is_some_and(|select| select.form_id == form_id)
        {
            self.line_select = None;
        }
    }

    fn remove_all_forms(&mut self) {
        self.forms.clear();
        self.mdi.clear();
        self.focused = None;
        self.table_focus = None;
        self.editing = None;
        self.column_drag = None;
        self.line_select = None;
    }

    /// Parse the Qt `.ui` XML into the Qt widget tree, place it in the
    /// MDI area and keep it for rendering and event routing.
    fn on_create_form(&mut self, payload: &serde_json::Value) {
        let form_id = payload
            .get("form_id")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string();
        let xml = payload.get("ui_xml").and_then(|v| v.as_str()).unwrap_or("");

        let (mut tree, root) = match rqt::widgets::load_ui(xml) {
            Ok(form) => form,
            Err(err) => {
                Self::log(format!("createForm parse error: {err}"));
                return;
            }
        };
        let area = tree
            .widget(root)
            .map(|widget| widget.geometry)
            .unwrap_or_default();
        let title = form_title(&tree, root, &form_id);

        if let Some(index) = self.mdi.index_of(|payload| payload == &form_id) {
            self.mdi.remove(index);
        }
        let index = self.mdi.add_subwindow(
            title,
            form_id.clone(),
            rqt::gui::QSizeF::new(area.width, area.height),
        );
        let content = self
            .mdi
            .subwindow(index)
            .map(|subwindow| subwindow.content_rect())
            .unwrap_or(area);
        tree.translate(root, content.x - area.x, content.y - area.y);

        Self::log(format!(
            "layout form_id={form_id} nodes={} area=({:.0},{:.0},{:.0},{:.0})",
            tree.len(),
            area.x,
            area.y,
            area.width,
            area.height
        ));
        self.forms.insert(form_id.clone(), QtForm { tree, root });
        if self.focused.as_ref().is_some_and(|(f, _)| f == &form_id) {
            self.focused = None;
        }
        if self
            .table_focus
            .as_ref()
            .is_some_and(|(f, _)| f == &form_id)
        {
            self.table_focus = None;
        }
        if self.editing.as_ref().is_some_and(|e| e.form_id == form_id) {
            self.editing = None;
        }
    }

    /// Collect the Qt-painted form commands into the flat batches the
    /// wgpu renderer consumes, interleaving MDI card chrome per form.
    fn collect_paint(&self) -> (Vec<gfx::ColoredRect>, Vec<gfx::ColoredText>) {
        let mut rects = Vec::new();
        let mut texts = Vec::new();
        let style = rqt::style::QFusionStyle::default();
        let active = self.mdi.active_index();
        for (index, subwindow) in self.mdi.subwindows().iter().enumerate() {
            let Some(form) = self.forms.get(&subwindow.payload) else {
                continue;
            };
            let mut painter = rqt::gui::QPainter::new();
            rqt::paint::paint_widget_tree(&form.tree, form.root, &style, &mut painter);
            append_paint_commands(painter.commands(), &mut rects, &mut texts);
            self.push_subwindow_chrome(subwindow, active == Some(index), &mut rects, &mut texts);
        }
        (rects, texts)
    }

    fn collect_rects(&self) -> Vec<gfx::ColoredRect> {
        self.collect_paint().0
    }

    fn collect_texts(&self) -> Vec<gfx::ColoredText> {
        self.collect_paint().1
    }

    fn push_subwindow_chrome(
        &self,
        subwindow: &QMdiSubWindow<String>,
        active: bool,
        rects: &mut Vec<gfx::ColoredRect>,
        texts: &mut Vec<gfx::ColoredText>,
    ) {
        let frame = subwindow.frame;
        let title = if active {
            [0.188, 0.549, 0.776, 1.0]
        } else {
            CARD_IDLE
        };
        let text_color = if active {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [0.08, 0.08, 0.10, 1.0]
        };
        rects.push(gfx::ColoredRect {
            x: frame.x,
            y: frame.y,
            w: frame.width,
            h: QMDI_TITLE_H,
            color: title,
        });
        for border in [
            gfx::ColoredRect {
                x: frame.x,
                y: frame.y,
                w: 1.0,
                h: frame.height,
                color: CARD_BORDER,
            },
            gfx::ColoredRect {
                x: frame.right() - 1.0,
                y: frame.y,
                w: 1.0,
                h: frame.height,
                color: CARD_BORDER,
            },
            gfx::ColoredRect {
                x: frame.x,
                y: frame.bottom() - 1.0,
                w: frame.width,
                h: 1.0,
                color: CARD_BORDER,
            },
        ] {
            rects.push(border);
        }
        let close = subwindow.close_rect();
        let maximize = subwindow.maximize_rect();
        rects.push(gfx::ColoredRect {
            x: close.x,
            y: close.y,
            w: close.width,
            h: close.height,
            color: [0.58, 0.26, 0.29, 1.0],
        });
        rects.push(gfx::ColoredRect {
            x: maximize.x,
            y: maximize.y,
            w: maximize.width,
            h: maximize.height,
            color: if active {
                [0.36, 0.55, 0.72, 1.0]
            } else {
                [0.66, 0.68, 0.72, 1.0]
            },
        });
        let resize = subwindow.resize_rect();
        for step in 0..3 {
            let offset = step as f32 * 4.0;
            rects.push(gfx::ColoredRect {
                x: resize.right() - 3.0 - offset,
                y: resize.bottom() - 1.0,
                w: 2.0,
                h: 1.0,
                color: CARD_BORDER,
            });
            rects.push(gfx::ColoredRect {
                x: resize.right() - 1.0,
                y: resize.bottom() - 3.0 - offset,
                w: 1.0,
                h: 2.0,
                color: CARD_BORDER,
            });
        }
        let title_right = (maximize.x - 6.0).max(frame.x + 8.0);
        texts.push(gfx::ColoredText {
            x: frame.x + 8.0,
            y: frame.y + 4.0,
            clip: [
                frame.x as i32,
                frame.y as i32,
                title_right as i32,
                (frame.y + QMDI_TITLE_H) as i32,
            ],
            wrap_w: (title_right - frame.x - 8.0).max(1.0),
            text: subwindow.title.clone(),
            color: text_color,
        });
        for (rect, glyph) in [(close, "×"), (maximize, "□")] {
            texts.push(gfx::ColoredText {
                x: rect.x + (QMDI_BUTTON - 7.0) / 2.0,
                y: rect.y,
                clip: [
                    rect.x as i32,
                    rect.y as i32,
                    rect.right() as i32,
                    rect.bottom() as i32,
                ],
                wrap_w: QMDI_BUTTON,
                text: glyph.into(),
                color: text_color,
            });
        }
    }

    fn mdi_hit(&self, point: (f32, f32)) -> Option<(usize, QMdiSubWindowHit)> {
        self.mdi.hit_test(rqt::gui::QPointF::new(point.0, point.1))
    }

    fn activate_form(&mut self, form_id: &str) -> Option<usize> {
        let index = self.mdi.index_of(|payload| payload == form_id)?;
        self.mdi.activate(index)
    }

    fn sync_form_geometry(&mut self, form_id: &str) {
        let Some(index) = self.mdi.index_of(|payload| payload == form_id) else {
            return;
        };
        let Some(content) = self
            .mdi
            .subwindow(index)
            .map(|subwindow| subwindow.content_rect())
        else {
            return;
        };
        if let Some(form) = self.forms.get_mut(form_id) {
            form.tree.set_geometry(form.root, content);
        }
    }

    fn dispatch_mouse_press(&mut self) {
        self.left_down = true;
        self.line_select = None;
        self.caret_epoch = now_ms();
        let Some((index, hit)) = self.mdi_hit(self.cursor) else {
            self.line_select = None;
            return;
        };
        match hit {
            QMdiSubWindowHit::Title => {
                self.commit_edit();
                if let Some(index) = self.mdi.activate(index) {
                    self.dragging = Some(Drag {
                        index,
                        mode: MdiDragMode::Move,
                        last: self.cursor,
                    });
                }
            }
            QMdiSubWindowHit::ResizeBottomRight => {
                self.commit_edit();
                if let Some(index) = self.mdi.activate(index) {
                    self.dragging = Some(Drag {
                        index,
                        mode: MdiDragMode::Resize,
                        last: self.cursor,
                    });
                }
            }
            QMdiSubWindowHit::Close | QMdiSubWindowHit::Maximize => {}
            QMdiSubWindowHit::Body => {
                let Some(index) = self.mdi.activate(index) else {
                    return;
                };
                let Some(form_id) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                let Some((_, name, class)) = self.hit_test(self.cursor) else {
                    self.line_select = None;
                    return;
                };
                let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                self.scroll_click = self
                    .forms
                    .get(&form_id)
                    .and_then(|form| form.tree.scrollbar_at(form.root, point))
                    .is_some();
                self.line_select = (class == "QLineEdit").then(|| LineSelect {
                    form_id: form_id.clone(),
                    name: name.clone(),
                });
                if class == "QTableWidget" {
                    let action = self.forms.get(&form_id).and_then(|form| {
                        let widget = form.tree.find_widget(&name)?;
                        let node = form.tree.widget(widget)?;
                        let table = form.tree.table(widget)?;
                        let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                        let viewport = table.scroll_area(node.geometry, &node.font).viewport;
                        if !viewport.contains(point) {
                            return None;
                        }
                        let local_x = self.cursor.0 - viewport.x;
                        let local_y = self.cursor.1 - viewport.y;
                        if local_y >= QTABLE_HEADER_H {
                            return None;
                        }
                        let row_header_w = table.row_header_width(&node.font);
                        if let Some(column) =
                            table.column_resize_handle_at(local_x - table.horizontal, row_header_w)
                        {
                            return Some((true, column));
                        }
                        table
                            .column_at(local_x, row_header_w, viewport.width)
                            .map(|column| (false, column))
                    });
                    if let Some((resize, column)) = action {
                        if resize {
                            self.column_drag = Some(ColumnDrag {
                                form_id: form_id.clone(),
                                name: name.clone(),
                                column,
                                last_x: self.cursor.0,
                            });
                        } else if let Some(table) = self.table_mut_for(&form_id, &name) {
                            table.toggle_sort(column);
                        }
                        return;
                    }
                    let already_selected = self.forms.get(&form_id).and_then(|form| {
                        let widget = form.tree.find_widget(&name)?;
                        let node = form.tree.widget(widget)?;
                        let table = form.tree.table(widget)?;
                        let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                        let viewport = table.scroll_area(node.geometry, &node.font).viewport;
                        if !viewport.contains(point) {
                            return None;
                        }
                        let row_header_w = table.row_header_width(&node.font);
                        let row = table.row_at(self.cursor.1 - viewport.y)?;
                        let column = table.column_at(
                            self.cursor.0 - viewport.x,
                            row_header_w,
                            viewport.width,
                        )?;
                        Some(table.is_selected(row, column))
                    });
                    if let Some(already) = already_selected {
                        self.table_press = Some(TablePress {
                            form_id: form_id.clone(),
                            name: name.clone(),
                            already_selected: already,
                        });
                    }
                }
                let Some(form) = self.forms.get_mut(&form_id) else {
                    return;
                };
                form.tree.mouse_press_with_modifiers(
                    form.root,
                    rqt::gui::QPointF::new(self.cursor.0, self.cursor.1),
                    self.shift,
                );
            }
        }
    }

    fn drag_card(&mut self, dx: f32, dy: f32) {
        let Some(drag) = self.dragging.as_mut() else {
            return;
        };
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        let index = drag.index;
        let mode = drag.mode;
        drag.last = self.cursor;
        match mode {
            MdiDragMode::Move => {
                let Some(form_id) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.mdi.translate(index, dx, dy);
                if let Some(form) = self.forms.get_mut(&form_id) {
                    form.tree.translate(form.root, dx, dy);
                }
            }
            MdiDragMode::Resize => {
                let Some((form_id, frame)) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| (subwindow.payload.clone(), subwindow.frame))
                else {
                    return;
                };
                let width = (self.cursor.0 - frame.x).max(80.0);
                let height = (self.cursor.1 - (frame.y + QMDI_TITLE_H)).max(64.0);
                self.mdi
                    .resize_content(index, rqt::gui::QSizeF::new(width, height));
                self.sync_form_geometry(&form_id);
            }
        }
    }

    fn close_card(&mut self, form_id: &str) {
        if let Some(socket) = &self.socket {
            socket.send(
                "send_event",
                serde_json::json!({
                    "command": "formEvent",
                    "form_id": form_id,
                    "widget_name": form_id,
                    "event": "Close",
                }),
            );
        }
        self.remove_form(form_id);
        Self::log(format!("event Close -> {form_id}"));
    }

    /// Topmost leaf widget under the cursor, using the Qt widget tree.
    fn hit_test(&self, point: (f32, f32)) -> Option<(String, String, String)> {
        let point = rqt::gui::QPointF::new(point.0, point.1);
        for subwindow in self.mdi.subwindows().iter().rev() {
            if !subwindow.content_rect().contains(point) {
                continue;
            }
            let Some(form) = self.forms.get(&subwindow.payload) else {
                continue;
            };
            let Some(widget) = form.tree.widget_at(form.root, point) else {
                continue;
            };
            let Some(widget) = form.tree.widget(widget) else {
                continue;
            };
            return Some((
                subwindow.payload.clone(),
                widget.object_name.clone(),
                widget.class_name.clone(),
            ));
        }
        None
    }

    /// Qt `eventFilter` analog: route a mouse release to the Lua side
    /// and move the keyboard focus with the click. A click always ends
    /// a cell edit first.
    fn dispatch_mouse_release(&mut self) {
        Self::log(format!(
            "click at ({:.0},{:.0})",
            self.cursor.0, self.cursor.1
        ));
        self.left_down = false;
        self.line_select = None;
        self.commit_edit();
        if self.dragging.take().is_some() || self.column_drag.is_some() {
            self.column_drag = None;
            return;
        }

        match self.mdi_hit(self.cursor) {
            Some((index, QMdiSubWindowHit::Close)) => {
                let Some(form_id) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.activate_form(&form_id);
                self.close_card(&form_id);
                return;
            }
            Some((index, QMdiSubWindowHit::Maximize)) => {
                let Some(form_id) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                if let Some(index) = self.activate_form(&form_id) {
                    self.mdi.toggle_maximized(index);
                    self.sync_form_geometry(&form_id);
                }
                return;
            }
            Some((index, QMdiSubWindowHit::Title)) => {
                if let Some(form_id) = self
                    .mdi
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                {
                    self.activate_form(&form_id);
                }
                return;
            }
            Some((_, QMdiSubWindowHit::ResizeBottomRight)) => return,
            Some((index, QMdiSubWindowHit::Body)) => {
                self.mdi.activate(index);
            }
            None => {
                self.focused = None;
                self.table_focus = None;
                return;
            }
        }

        let hit = self.hit_test(self.cursor);

        // Focus follows the click; only editable widgets keep it.
        self.focused = match &hit {
            Some((form_id, name, class)) if class == "QLineEdit" => {
                Some((form_id.clone(), name.clone()))
            }
            _ => None,
        };
        if self.focused.is_some() {
            self.caret_epoch = now_ms();
            Self::log(format!("focus {:?}", self.focused));
        }

        let Some((form_id, widget_name, class)) = hit else {
            self.table_focus = None;
            self.table_press = None;
            return;
        };

        let (protocol_event, start_cell_edit) = {
            let Some(form) = self.forms.get_mut(&form_id) else {
                self.table_press = None;
                return;
            };
            let Some(widget) = form.tree.find_widget(&widget_name) else {
                self.table_press = None;
                return;
            };
            let event = form.tree.mouse_release(
                form.root,
                rqt::gui::QPointF::new(self.cursor.0, self.cursor.1),
            );
            let protocol_event = event
                .map(|event| event.protocol_event())
                .unwrap_or("MouseButtonRelease");
            let start_cell_edit = if class == "QTableWidget" {
                let already_selected = self.table_press.as_ref().is_some_and(|press| {
                    press.form_id == form_id && press.name == widget_name && press.already_selected
                });
                if already_selected && self.editing.is_none() {
                    form.tree.table(widget).and_then(|table| {
                        let row = table.current_row()?;
                        let column = table.current_column()?;
                        (!table.read_only).then_some((row, column))
                    })
                } else {
                    None
                }
            } else {
                None
            };
            (protocol_event, start_cell_edit)
        };

        self.table_focus = if matches!(
            class.as_str(),
            "QTableWidget" | "QListWidget" | "QTreeWidget"
        ) {
            Some((form_id.clone(), widget_name.clone()))
        } else {
            None
        };
        self.table_press = None;
        if let Some((row, column)) = start_cell_edit {
            self.start_edit(&form_id, &widget_name, row, column);
        }

        let selection_event = self.forms.get(&form_id).and_then(|form| {
            let widget = form.tree.find_widget(&widget_name)?;
            match class.as_str() {
                "QTableWidget" => {
                    let table = form.tree.table(widget)?;
                    let (row, column) = table.current()?;
                    Some((
                        "CurrentCellChanged",
                        serde_json::json!({
                            "row": row + 1,
                            "col": column + 1,
                        }),
                    ))
                }
                "QListWidget" => {
                    let list = form.tree.list(widget)?;
                    let row = list.current?;
                    Some((
                        "CurrentRowChanged",
                        serde_json::json!({
                            "row": row,
                            "value": list.items.get(row).cloned().unwrap_or_default(),
                        }),
                    ))
                }
                "QTreeWidget" => {
                    let tree = form.tree.tree(widget)?;
                    let row = tree.current?;
                    Some((
                        "CurrentItemChanged",
                        serde_json::json!({
                            "row": row,
                            "value": tree.rows.get(row).map(|row| row.text.clone()).unwrap_or_default(),
                        }),
                    ))
                }
                _ => None,
            }
        });

        let Some(socket) = &self.socket else {
            return;
        };
        socket.send(
            "send_event",
            serde_json::json!({
                "command": "formEvent",
                "form_id": form_id,
                "widget_name": widget_name,
                "event": protocol_event,
            }),
        );
        if let Some((event, payload)) = selection_event {
            socket.send(
                "send_event",
                serde_json::json!({
                    "command": "formEvent",
                    "form_id": form_id,
                    "widget_name": widget_name,
                    "event": event,
                    "row": payload.get("row"),
                    "col": payload.get("col"),
                    "value": payload.get("value"),
                }),
            );
            Self::log(format!("event {event} -> {widget_name}"));
        }
        Self::log(format!("event {protocol_event} -> {widget_name}"));
    }

    fn table_mut_for(
        &mut self,
        form_id: &str,
        name: &str,
    ) -> Option<&mut rqt::item_model::QTableWidget> {
        let form = self.forms.get_mut(form_id)?;
        let widget = form.tree.find_widget(name)?;
        form.tree.table_mut(widget)
    }

    /// Open a cell editor seeded with the current cell text.
    fn start_edit(&mut self, form_id: &str, name: &str, row: usize, col: usize) {
        let Some(form) = self.forms.get_mut(form_id) else {
            return;
        };
        let Some(widget) = form.tree.find_widget(name) else {
            return;
        };
        let Some(table) = form.tree.table_mut(widget) else {
            return;
        };
        if !table.begin_edit(row, col) {
            return;
        }
        self.editing = Some(CellEdit {
            form_id: form_id.to_string(),
            table: name.to_string(),
        });
        Self::log(format!("table {name}: edit r{}c{}", row + 1, col + 1));
    }

    /// Typing on the selected cell of an editable table opens an
    /// editor (Qt: `editTriggers = AnyKeyPressed`). `false` when the
    /// keyboard belongs somewhere else.
    fn begin_edit_on_selection(&mut self) -> bool {
        if self.focused.is_some() {
            return false;
        }
        let Some((form_id, name)) = self.table_focus.clone() else {
            return false;
        };
        let cell = self.forms.get(&form_id).and_then(|form| {
            let widget = form.tree.find_widget(&name)?;
            let table = form.tree.table(widget)?;
            if table.read_only {
                return None;
            }
            Some((table.current_row()?, table.current_column()?))
        });
        let Some((row, col)) = cell else {
            return false;
        };
        self.start_edit(&form_id, &name, row, col);
        true
    }

    /// Apply one key to the open cell editor.
    fn edit_key(&mut self, action: KeyEdit) {
        match action {
            KeyEdit::Insert(text) => {
                let Some((form_id, name)) = self
                    .editing
                    .as_ref()
                    .map(|edit| (edit.form_id.clone(), edit.table.clone()))
                else {
                    return;
                };
                let buffer = self
                    .table_mut_for(&form_id, &name)
                    .map(|table| {
                        table.editor_insert(&text);
                        table
                            .editor
                            .as_ref()
                            .map(|editor| editor.text.clone())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                Self::log(format!("edit buffer {buffer:?}"));
            }
            KeyEdit::Backspace => {
                let Some((form_id, name)) = self
                    .editing
                    .as_ref()
                    .map(|edit| (edit.form_id.clone(), edit.table.clone()))
                else {
                    return;
                };
                if let Some(table) = self.table_mut_for(&form_id, &name) {
                    table.editor_backspace();
                }
            }
            KeyEdit::Left
            | KeyEdit::Right
            | KeyEdit::Up
            | KeyEdit::Down
            | KeyEdit::PageUp
            | KeyEdit::PageDown
            | KeyEdit::Home
            | KeyEdit::End
            | KeyEdit::Delete => {}
            KeyEdit::Commit => self.commit_edit(),
            KeyEdit::ClearFocus => {
                if let Some(edit) = self.editing.take() {
                    if let Some(table) = self.table_mut_for(&edit.form_id, &edit.table) {
                        table.cancel_editor();
                    }
                    Self::log("edit canceled");
                }
            }
            KeyEdit::Ignore => {}
        }
    }

    /// Write the buffer into the Qt table model and notify the Lua side
    /// (`CellChanged`; Lua reads cells back via `currentRowCell`).
    fn commit_edit(&mut self) {
        let Some(edit) = self.editing.take() else {
            return;
        };
        let editor = self
            .table_mut_for(&edit.form_id, &edit.table)
            .and_then(|table| table.commit_editor());
        let Some(editor) = editor else {
            return;
        };
        if let Some(socket) = &self.socket {
            socket.send(
                "send_event",
                serde_json::json!({
                    "command": "formEvent",
                    "form_id": edit.form_id,
                    "widget_name": edit.table,
                    "event": "CellChanged",
                    "row": editor.row + 1,
                    "col": editor.column + 1,
                    "value": editor.text,
                }),
            );
        }
        Self::log(format!(
            "table {}: cell r{}c{} = {:?}",
            edit.table,
            editor.row + 1,
            editor.column + 1,
            editor.text
        ));
    }

    /// Mouse wheel over an item view scrolls it. winit's web backend
    /// negates DOM wheel deltas, so a positive delta means "scroll up".
    fn scroll_view(&mut self, dx: f32, dy: f32) {
        let Some((form_id, name, class)) = self.hit_test(self.cursor) else {
            return;
        };
        if !matches!(
            class.as_str(),
            "QTableWidget" | "QListWidget" | "QTreeWidget"
        ) {
            return;
        }
        let Some(form) = self.forms.get_mut(&form_id) else {
            return;
        };
        let Some(widget) = form.tree.find_widget(&name) else {
            return;
        };
        let Some(node) = form.tree.widget(widget) else {
            return;
        };
        let rect = node.geometry;
        let font = node.font.clone();
        let shift = self.shift;
        if class == "QTableWidget" {
            let Some(table) = form.tree.table_mut(widget) else {
                return;
            };
            let viewport = table.scroll_area(rect, &font).viewport;
            let row_header_w = table.row_header_width(&font);
            if shift {
                table.scroll_horizontal_by(-dy, viewport.width, row_header_w);
            } else {
                table.scroll_by(-dy, viewport.height);
                if dx != 0.0 {
                    table.scroll_horizontal_by(-dx, viewport.width, row_header_w);
                }
            }
            table.sync_scrollbars(rect, &font);
            return;
        }
        if class == "QListWidget" {
            let Some(list) = form.tree.list_mut(widget) else {
                return;
            };
            let layout = list.scroll_area(rect);
            let viewport = layout.viewport;
            if shift {
                list.scroll_horizontal_by(-dy, viewport.width);
            } else {
                list.scroll_by(-dy, viewport.height);
                if dx != 0.0 {
                    list.scroll_horizontal_by(-dx, viewport.width);
                }
            }
            list.sync_bars(rqt::gui::QSizeF::new(viewport.width, viewport.height));
            return;
        }
        let Some(tree) = form.tree.tree_mut(widget) else {
            return;
        };
        let layout = tree.scroll_area(rect);
        let viewport = layout.viewport;
        let body_height = (viewport.height - rqt::item_view::QVIEW_HEADER_H).max(0.0);
        if shift {
            tree.scroll_horizontal_by(-dy, viewport.width);
        } else {
            tree.scroll_by(-dy, body_height);
            if dx != 0.0 {
                tree.scroll_horizontal_by(-dx, viewport.width);
            }
        }
        tree.sync_bars(rqt::gui::QSizeF::new(viewport.width, body_height));
    }

    /// Keyboard half of the event loop: an open cell edit first, then
    /// typing into a selected table cell, then the focused line edit.
    fn dispatch_key(&mut self, event: &winit::event::KeyEvent) {
        if self.ctrl {
            if let winit::keyboard::Key::Character(text) = &event.logical_key {
                if text.eq_ignore_ascii_case("a") {
                    if let Some((form_id, name)) = self.focused.clone() {
                        if let Some(form) = self.forms.get_mut(&form_id) {
                            if let Some(widget) = form.tree.find_widget(&name) {
                                form.tree.select_all(widget);
                                self.caret_epoch = now_ms();
                            }
                        }
                    }
                    return;
                }
            }
        }
        let action = key_edit(event.text.as_deref(), &event.logical_key);
        self.caret_epoch = now_ms();

        if self.editing.is_some() {
            self.edit_key(action);
            return;
        }

        if matches!(action, KeyEdit::Insert(_) | KeyEdit::Backspace)
            && self.begin_edit_on_selection()
        {
            self.edit_key(action);
            return;
        }

        if matches!(
            action,
            KeyEdit::Left
                | KeyEdit::Right
                | KeyEdit::Up
                | KeyEdit::Down
                | KeyEdit::PageUp
                | KeyEdit::PageDown
                | KeyEdit::Home
                | KeyEdit::End
        ) && self.navigate_view(&action)
        {
            return;
        }

        match action {
            KeyEdit::Insert(text) => {
                let Some((form_id, name)) = self.focused.clone() else {
                    return;
                };
                let Some(form) = self.forms.get_mut(&form_id) else {
                    return;
                };
                let Some(widget) = form.tree.find_widget(&name) else {
                    return;
                };
                let mut applied = String::new();
                for ch in text.chars() {
                    if form.tree.insert_text(widget, ch) {
                        applied.push(ch);
                    }
                }
                if !applied.is_empty() {
                    form.tree.relayout(form.root);
                    Self::log(format!("edit {name}: +{applied:?}"));
                }
            }
            KeyEdit::Backspace => {
                let Some((form_id, name)) = self.focused.clone() else {
                    return;
                };
                let Some(form) = self.forms.get_mut(&form_id) else {
                    return;
                };
                let Some(widget) = form.tree.find_widget(&name) else {
                    return;
                };
                if form.tree.backspace_text(widget) {
                    form.tree.relayout(form.root);
                    Self::log(format!("edit {name}: backspace"));
                }
            }
            KeyEdit::Left | KeyEdit::Right | KeyEdit::Home | KeyEdit::End => {
                if self.editing.is_some() {
                    return;
                }
                let Some((form_id, name)) = self.focused.clone() else {
                    return;
                };
                let Some(form) = self.forms.get_mut(&form_id) else {
                    return;
                };
                let Some(widget) = form.tree.find_widget(&name) else {
                    return;
                };
                let current = form.tree.cursor_position(widget);
                let length = form.tree.text(widget).unwrap_or_default().chars().count();
                let position = match action {
                    KeyEdit::Left => current.saturating_sub(1),
                    KeyEdit::Right => (current + 1).min(length),
                    KeyEdit::Home => 0,
                    KeyEdit::End => length,
                    _ => current,
                };
                form.tree
                    .set_cursor_position_ext(widget, position, self.shift);
            }
            KeyEdit::Delete => {
                if self.editing.is_some() {
                    return;
                }
                let Some((form_id, name)) = self.focused.clone() else {
                    return;
                };
                if let Some(form) = self.forms.get_mut(&form_id) {
                    if let Some(widget) = form.tree.find_widget(&name) {
                        if form.tree.delete_text(widget) {
                            form.tree.relayout(form.root);
                        }
                    }
                }
            }
            KeyEdit::ClearFocus => {
                if self.focused.take().is_some() {
                    Self::log("focus cleared");
                }
                self.table_focus = None;
            }
            KeyEdit::Up
            | KeyEdit::Down
            | KeyEdit::PageUp
            | KeyEdit::PageDown
            | KeyEdit::Commit
            | KeyEdit::Ignore => {}
        }
    }

    /// Arrow/Page/Home/End navigation for the focused item view.
    fn navigate_view(&mut self, action: &KeyEdit) -> bool {
        let Some((form_id, name)) = self.table_focus.clone() else {
            return false;
        };
        let Some(form) = self.forms.get_mut(&form_id) else {
            return false;
        };
        let Some(widget) = form.tree.find_widget(&name) else {
            return false;
        };
        let Some(node) = form.tree.widget(widget) else {
            return false;
        };
        let class = node.class_name.clone();
        let rect = node.geometry;
        let font = node.font.clone();
        match class.as_str() {
            "QTableWidget" => {
                let Some(table) = form.tree.table(widget) else {
                    return false;
                };
                let row_count = table.model.row_count();
                let column_count = table.model.column_count();
                if row_count == 0 || column_count == 0 {
                    return true;
                }
                let (row, column) = table.current().unwrap_or((0, 0));
                let viewport = table.scroll_area(rect, &font).viewport;
                let page_rows = ((viewport.height - QTABLE_HEADER_H) / QTABLE_ROW_H)
                    .floor()
                    .max(1.0) as usize;
                let (row, column) = match action {
                    KeyEdit::Up => (row.saturating_sub(1), column),
                    KeyEdit::Down => ((row + 1).min(row_count - 1), column),
                    KeyEdit::PageUp => (row.saturating_sub(page_rows), column),
                    KeyEdit::PageDown => ((row + page_rows).min(row_count - 1), column),
                    KeyEdit::Home => (row, 0),
                    KeyEdit::End => (row, column_count - 1),
                    KeyEdit::Left => (row, column.saturating_sub(1)),
                    KeyEdit::Right => (row, (column + 1).min(column_count - 1)),
                    _ => (row, column),
                };
                let Some(table) = form.tree.table_mut(widget) else {
                    return false;
                };
                table.set_current_cell(row, column);
                table.select_current();
                let viewport = table.scroll_area(rect, &font).viewport;
                table.reveal_current_row(viewport.height);
                let row_header_w = table.row_header_width(&font);
                table.reveal_current_column(viewport.width, row_header_w);
                table.sync_scrollbars(rect, &font);
                true
            }
            "QListWidget" => {
                let Some(list) = form.tree.list(widget) else {
                    return false;
                };
                if list.row_count() == 0 {
                    return true;
                }
                let layout = list.scroll_area(rect);
                let viewport = layout.viewport;
                let page_rows = (viewport.height / rqt::item_view::QVIEW_ROW_H)
                    .floor()
                    .max(1.0) as usize;
                let current = list.current.unwrap_or(0);
                let row = nav_row(action, current, list.row_count(), page_rows);
                let Some(list) = form.tree.list_mut(widget) else {
                    return false;
                };
                match action {
                    KeyEdit::Left | KeyEdit::Right => {
                        let delta = if matches!(action, KeyEdit::Left) {
                            -rqt::item_view::QVIEW_ROW_H
                        } else {
                            rqt::item_view::QVIEW_ROW_H
                        };
                        list.scroll_horizontal_by(delta, viewport.width);
                        list.hbar.set_value(list.horizontal);
                    }
                    _ => {
                        list.set_current(Some(row));
                        list.ensure_visible(row, viewport.height);
                        list.sync_vertical_bar(viewport.height);
                    }
                }
                true
            }
            "QTreeWidget" => {
                let Some(tree) = form.tree.tree(widget) else {
                    return false;
                };
                if tree.row_count() == 0 {
                    return true;
                }
                let layout = tree.scroll_area(rect);
                let viewport = layout.viewport;
                let body_height = (viewport.height - rqt::item_view::QVIEW_HEADER_H).max(0.0);
                let page_rows =
                    (body_height / rqt::item_view::QVIEW_ROW_H).floor().max(1.0) as usize;
                let current = tree.current.unwrap_or(0);
                let row = nav_row(action, current, tree.row_count(), page_rows);
                let Some(tree) = form.tree.tree_mut(widget) else {
                    return false;
                };
                match action {
                    KeyEdit::Left | KeyEdit::Right => {
                        let delta = if matches!(action, KeyEdit::Left) {
                            -rqt::item_view::QVIEW_ROW_H
                        } else {
                            rqt::item_view::QVIEW_ROW_H
                        };
                        tree.scroll_horizontal_by(delta, viewport.width);
                        tree.hbar.set_value(tree.horizontal);
                    }
                    _ => {
                        tree.set_current(Some(row));
                        tree.ensure_visible(row, body_height);
                        tree.sync_vertical_bar(body_height);
                    }
                }
                true
            }
            _ => false,
        }
    }
}

fn nav_row(action: &KeyEdit, current: usize, count: usize, page: usize) -> usize {
    match action {
        KeyEdit::Up => current.saturating_sub(1),
        KeyEdit::Down => (current + 1).min(count - 1),
        KeyEdit::PageUp => current.saturating_sub(page),
        KeyEdit::PageDown => (current + page).min(count - 1),
        KeyEdit::Home => 0,
        KeyEdit::End => count - 1,
        _ => current,
    }
}

/// One local effect of a key press.
#[derive(Debug, PartialEq, Eq)]
enum KeyEdit {
    Insert(String),
    Backspace,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
    /// Enter — commit an open cell editor.
    Commit,
    ClearFocus,
    Ignore,
}

/// Map a winit key event to a tree edit. `text` carries the printable
/// characters — on the web it also contains `"\r"` (Enter) and `"\t"`
/// (Tab), which are filtered out instead of typed into the widget.
fn key_edit(text: Option<&str>, key: &winit::keyboard::Key) -> KeyEdit {
    use winit::keyboard::{Key, NamedKey};
    match key {
        Key::Named(NamedKey::Backspace) => KeyEdit::Backspace,
        Key::Named(NamedKey::ArrowLeft) => KeyEdit::Left,
        Key::Named(NamedKey::ArrowRight) => KeyEdit::Right,
        Key::Named(NamedKey::ArrowUp) => KeyEdit::Up,
        Key::Named(NamedKey::ArrowDown) => KeyEdit::Down,
        Key::Named(NamedKey::PageUp) => KeyEdit::PageUp,
        Key::Named(NamedKey::PageDown) => KeyEdit::PageDown,
        Key::Named(NamedKey::Home) => KeyEdit::Home,
        Key::Named(NamedKey::End) => KeyEdit::End,
        Key::Named(NamedKey::Delete) => KeyEdit::Delete,
        Key::Named(NamedKey::Escape) => KeyEdit::ClearFocus,
        Key::Named(NamedKey::Enter) => KeyEdit::Commit,
        _ => {
            let printable: String = text
                .unwrap_or("")
                .chars()
                .filter(|c| !c.is_control())
                .collect();
            if printable.is_empty() {
                KeyEdit::Ignore
            } else {
                KeyEdit::Insert(printable)
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

fn str_field<'a>(payload: &'a serde_json::Value, key: &str) -> &'a str {
    payload.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// Wire JSON value → typed property (geometry/size objects included).
fn prop_from_json(value: Option<&serde_json::Value>) -> Option<form_xml::PropValue> {
    use form_xml::PropValue;
    use serde_json::Value;
    match value? {
        Value::String(s) => Some(PropValue::Str(s.clone())),
        Value::Bool(b) => Some(PropValue::Bool(*b)),
        Value::Number(n) => Some(match n.as_i64() {
            Some(i) => PropValue::Int(i),
            None => PropValue::Str(n.to_string()),
        }),
        Value::Array(items) => Some(PropValue::List(
            items
                .iter()
                .filter_map(|v| prop_from_json(Some(v)))
                .collect(),
        )),
        Value::Object(map) => {
            let get = |k: &str| map.get(k).and_then(|v| v.as_i64());
            match (get("x"), get("y"), get("w"), get("h")) {
                (Some(x), Some(y), Some(w), Some(h)) => Some(PropValue::Rect { x, y, w, h }),
                _ => match (get("w"), get("h")) {
                    (Some(w), Some(h)) => Some(PropValue::Size { w, h }),
                    _ => Some(PropValue::Map(
                        map.iter()
                            .filter_map(|(k, v)| {
                                prop_from_json(Some(v)).map(|value| (k.clone(), value))
                            })
                            .collect(),
                    )),
                },
            }
        }
        _ => None,
    }
}

/// Typed property → wire JSON (Lua's `getProperty` reply).
fn prop_to_json(value: &form_xml::PropValue) -> serde_json::Value {
    use form_xml::PropValue;
    use serde_json::Value;
    match value {
        PropValue::Str(s) => Value::String(s.clone()),
        PropValue::Int(i) => Value::from(*i),
        PropValue::Bool(b) => Value::from(*b),
        PropValue::Enum(s) => Value::String(s.clone()),
        PropValue::Rect { x, y, w, h } => serde_json::json!({"x": x, "y": y, "w": w, "h": h}),
        PropValue::Size { w, h } => serde_json::json!({"w": w, "h": h}),
        PropValue::List(items) => Value::Array(items.iter().map(prop_to_json).collect()),
        PropValue::Map(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), prop_to_json(v)))
                .collect(),
        ),
    }
}

/// Compact one-token rendering for log lines.
fn prop_brief(value: &form_xml::PropValue) -> String {
    use form_xml::PropValue;
    match value {
        PropValue::Str(s) => s.clone(),
        PropValue::Int(i) => i.to_string(),
        PropValue::Bool(b) => b.to_string(),
        PropValue::Enum(s) => s.clone(),
        PropValue::Rect { x, y, w, h } => format!("({x},{y},{w},{h})"),
        PropValue::Size { w, h } => format!("{w}x{h}"),
        PropValue::List(items) => format!("list[{}]", items.len()),
        PropValue::Map(entries) => format!("map[{}]", entries.len()),
    }
}

fn form_title(tree: &QWidgetTree, root: QWidgetId, fallback: &str) -> String {
    match tree.property(root, "windowTitle") {
        Some(rqt::core::QVariant::String(title)) | Some(rqt::core::QVariant::Enum(title))
            if !title.is_empty() =>
        {
            title.clone()
        }
        _ => fallback.to_string(),
    }
}

fn append_paint_commands(
    commands: &[rqt::gui::QPaintCommand],
    rects: &mut Vec<gfx::ColoredRect>,
    texts: &mut Vec<gfx::ColoredText>,
) {
    use rqt::gui::{QPaintCommand, QRectF};

    let mut clip: Option<QRectF> = None;
    let mut stack: Vec<Option<QRectF>> = Vec::new();
    let push_rect = |rects: &mut Vec<gfx::ColoredRect>,
                     clip: Option<QRectF>,
                     rect: QRectF,
                     color: rqt::gui::QColor| {
        if let Some(rect) = intersect_rect(rect, clip) {
            rects.push(gfx::ColoredRect {
                x: rect.x,
                y: rect.y,
                w: rect.width,
                h: rect.height,
                color: [color.red, color.green, color.blue, color.alpha],
            });
        }
    };
    for command in commands {
        match command {
            QPaintCommand::Save => stack.push(clip),
            QPaintCommand::Restore => clip = stack.pop().unwrap_or(None),
            QPaintCommand::SetClip { rect } => clip = *rect,
            QPaintCommand::Translate { .. } => {}
            QPaintCommand::FillRect { rect, color }
            | QPaintCommand::FillRoundedRect { rect, color, .. } => {
                push_rect(rects, clip, *rect, *color)
            }
            QPaintCommand::FillLinearGradientRect {
                rect, start, end, ..
            } => {
                let top = rect.top().round() as i32;
                let bottom = rect.bottom().round() as i32;
                let span = (bottom - top).max(1) as f32;
                for y in top..bottom {
                    let t = ((y as f32 + 0.5 - top as f32) / span).clamp(0.0, 1.0);
                    push_rect(
                        rects,
                        clip,
                        QRectF::new(rect.x, y as f32, rect.width, 1.0),
                        start.lerp(*end, t),
                    );
                }
            }
            QPaintCommand::StrokeEllipse { rect, width, color } => {
                let left = rect.left().floor() as i32;
                let top = rect.top().floor() as i32;
                let w = rect.width.ceil().max(1.0);
                let h = rect.height.ceil().max(1.0);
                let cx = rect.x + rect.width / 2.0;
                let cy = rect.y + rect.height / 2.0;
                let radius = rect.width.min(rect.height) / 2.0;
                let inner = (radius - *width).max(0.0);
                for y in 0..h as i32 {
                    for x in 0..w as i32 {
                        let dx = left as f32 + x as f32 + 0.5 - cx;
                        let dy = top as f32 + y as f32 + 0.5 - cy;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d <= radius && d >= inner {
                            push_rect(
                                rects,
                                clip,
                                QRectF::new(
                                    left as f32 + x as f32,
                                    top as f32 + y as f32,
                                    1.0,
                                    1.0,
                                ),
                                *color,
                            );
                        }
                    }
                }
            }
            QPaintCommand::FillEllipse { rect, color } => {
                let left = rect.left().floor() as i32;
                let top = rect.top().floor() as i32;
                let width = rect.width.ceil().max(1.0);
                let height = rect.height.ceil().max(1.0);
                let cx = rect.x + rect.width / 2.0;
                let cy = rect.y + rect.height / 2.0;
                let rx = rect.width / 2.0;
                let ry = rect.height / 2.0;
                for y in 0..height as i32 {
                    for x in 0..width as i32 {
                        let dx = (left as f32 + x as f32 + 0.5 - cx) / rx;
                        let dy = (top as f32 + y as f32 + 0.5 - cy) / ry;
                        if dx * dx + dy * dy <= 1.0 {
                            push_rect(
                                rects,
                                clip,
                                QRectF::new(
                                    left as f32 + x as f32,
                                    top as f32 + y as f32,
                                    1.0,
                                    1.0,
                                ),
                                *color,
                            );
                        }
                    }
                }
            }
            QPaintCommand::FillTriangle { a, b, c, color } => {
                let min_x = a.x.min(b.x).min(c.x).floor() as i32;
                let min_y = a.y.min(b.y).min(c.y).floor() as i32;
                let max_x = a.x.max(b.x).max(c.x).ceil() as i32;
                let max_y = a.y.max(b.y).max(c.y).ceil() as i32;
                let edge = |p: rqt::gui::QPointF, q: rqt::gui::QPointF, x: f32, y: f32| {
                    (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x)
                };
                for y in min_y..max_y {
                    for x in min_x..max_x {
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;
                        let e0 = edge(*a, *b, px, py);
                        let e1 = edge(*b, *c, px, py);
                        let e2 = edge(*c, *a, px, py);
                        if (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0)
                            || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0)
                        {
                            push_rect(
                                rects,
                                clip,
                                QRectF::new(x as f32, y as f32, 1.0, 1.0),
                                *color,
                            );
                        }
                    }
                }
            }
            QPaintCommand::DrawPixmap {
                rect,
                pixmap,
                opacity,
            } => {
                let left = rect.left().round() as i32;
                let top = rect.top().round() as i32;
                let width = rect.width.round().max(1.0);
                let height = rect.height.round().max(1.0);
                let scale_x = pixmap.width as f32 / width;
                let scale_y = pixmap.height as f32 / height;
                for y in 0..height as i32 {
                    for x in 0..width as i32 {
                        let source_x = ((x as f32 + 0.5) * scale_x)
                            .floor()
                            .clamp(0.0, pixmap.width.saturating_sub(1) as f32)
                            as u32;
                        let source_y = ((y as f32 + 0.5) * scale_y)
                            .floor()
                            .clamp(0.0, pixmap.height.saturating_sub(1) as f32)
                            as u32;
                        let [red, green, blue, alpha] = pixmap.pixel(source_x, source_y);
                        if alpha > 0 {
                            let color = rqt::gui::QColor::rgba8(red, green, blue, alpha)
                                .with_alpha(alpha as f32 / 255.0 * *opacity);
                            push_rect(
                                rects,
                                clip,
                                QRectF::new(
                                    left as f32 + x as f32,
                                    top as f32 + y as f32,
                                    1.0,
                                    1.0,
                                ),
                                color,
                            );
                        }
                    }
                }
            }
            QPaintCommand::StrokeRect { rect, width, color } => {
                push_stroke_rect(rects, clip, *rect, *width, *color);
            }
            QPaintCommand::StrokeRoundedRect {
                rect, width, color, ..
            } => {
                push_stroke_rect(rects, clip, *rect, *width, *color);
            }
            QPaintCommand::DrawLine {
                from,
                to,
                width,
                color,
            } => {
                let width = (*width).max(1.0);
                let half = width / 2.0;
                let left = from.x.min(to.x) - half;
                let top = from.y.min(to.y) - half;
                let right = from.x.max(to.x) + half;
                let bottom = from.y.max(to.y) + half;
                let dx = to.x - from.x;
                let dy = to.y - from.y;
                let length_squared = dx * dx + dy * dy;
                let mut y = top.floor() as i32;
                while y < bottom.ceil() as i32 {
                    let mut x = left.floor() as i32;
                    while x < right.ceil() as i32 {
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;
                        let t = if length_squared > 0.0 {
                            (((px - from.x) * dx + (py - from.y) * dy) / length_squared)
                                .clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let qx = from.x + dx * t;
                        let qy = from.y + dy * t;
                        let distance = ((px - qx) * (px - qx) + (py - qy) * (py - qy)).sqrt();
                        if distance <= half {
                            push_rect(
                                rects,
                                clip,
                                QRectF::new(x as f32, y as f32, 1.0, 1.0),
                                *color,
                            );
                        }
                        x += 1;
                    }
                    y += 1;
                }
            }
            QPaintCommand::DrawText {
                rect, text, color, ..
            } => {
                let clip_rect = clip.unwrap_or(*rect);
                if clip_rect.width > 0.0 && clip_rect.height > 0.0 {
                    texts.push(gfx::ColoredText {
                        x: rect.x,
                        y: rect.y,
                        clip: [
                            clip_rect.x as i32,
                            clip_rect.y as i32,
                            clip_rect.right() as i32,
                            clip_rect.bottom() as i32,
                        ],
                        wrap_w: rect.width.max(1.0),
                        text: text.clone(),
                        color: [color.red, color.green, color.blue, color.alpha],
                    });
                }
            }
        }
    }
}

fn push_stroke_rect(
    rects: &mut Vec<gfx::ColoredRect>,
    clip: Option<rqt::gui::QRectF>,
    rect: rqt::gui::QRectF,
    width: f32,
    color: rqt::gui::QColor,
) {
    use rqt::gui::QRectF;
    let left = rect.left().round();
    let top = rect.top().round();
    let right = rect.right().round();
    let bottom = rect.bottom().round();
    let width = width.round().max(1.0);
    let mut push = |rect: QRectF| {
        if let Some(rect) = intersect_rect(rect, clip) {
            rects.push(gfx::ColoredRect {
                x: rect.x,
                y: rect.y,
                w: rect.width,
                h: rect.height,
                color: [color.red, color.green, color.blue, color.alpha],
            });
        }
    };
    push(QRectF::new(left, top, right - left, width));
    push(QRectF::new(left, bottom - width, right - left, width));
    push(QRectF::new(left, top, width, bottom - top));
    push(QRectF::new(right - width, top, width, bottom - top));
}

fn intersect_rect(rect: rqt::gui::QRectF, clip: Option<rqt::gui::QRectF>) -> Option<rqt::gui::QRectF> {
    let Some(clip) = clip else {
        return (!rect.is_empty()).then_some(rect);
    };
    let left = rect.left().max(clip.left());
    let top = rect.top().max(clip.top());
    let right = rect.right().min(clip.right());
    let bottom = rect.bottom().min(clip.bottom());
    (right > left && bottom > top).then_some(rqt::gui::QRectF::new(
        left,
        top,
        right - left,
        bottom - top,
    ))
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("EPlatform")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
                )
                .expect("create_window");
            let window = Arc::new(window);

            // winit does not attach its canvas to the page by default
            // (WebWindowAttributes.append = false); do it manually.
            #[cfg(target_arch = "wasm32")]
            {
                use winit::platform::web::WindowExtWebSys;
                if let Some(canvas) = window.canvas() {
                    if let Some(body) = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.body())
                    {
                        let style = canvas.style();
                        let _ = style.set_property("display", "block");
                        let _ = style.set_property("position", "absolute");
                        let _ = style.set_property("top", "0");
                        let _ = style.set_property("left", "0");
                        let _ = style.set_property("width", "100%");
                        let _ = style.set_property("height", "100%");
                        let _ = body.set_attribute(
                            "style",
                            "margin:0; overflow:hidden; background:#1c1f26;",
                        );
                        let _ = body.append_child(&canvas);
                    }
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                let canvas_count = web_sys::window()
                    .map(|w| {
                        w.document().map(|d| {
                            d.query_selector_all("canvas")
                                .map(|n| n.length())
                                .unwrap_or(0)
                        })
                    })
                    .flatten()
                    .map(|c| c as i32)
                    .unwrap_or(-1);
                Self::log(format!("window created, canvas count = {canvas_count}"));
            }

            // Open the Phoenix socket BEFORE spawning the GPU task so any
            // GPU probe failure cannot delay the connection.
            Self::log(format!(
                "opening socket {} app={}",
                self.params.backend, self.params.app
            ));
            let (tx, rx) = channel();
            // Replace the placeholder receiver created in App::new by moving
            // events through a bridge: simplest is to rebuild App fields here.
            self.rx = rx;
            self.socket = Some(PhoenixSocket::open(
                &self.params.backend,
                format!("app:{}", self.params.app),
                tx,
            ));
            Self::log("socket object created");

            let gfx = self.gfx.clone();
            let gfx_window = window.clone();
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                App::log("gfx task started");
                match gfx::Gfx::new(gfx_window).await {
                    Ok(gfx_state) => {
                        *gfx.borrow_mut() = Some(gfx_state);
                        App::log("gfx ready");
                    }
                    Err(err) => App::log(format!("gpu init failed: {err}")),
                }
                App::log("gfx task finished");
            });
            #[cfg(not(target_arch = "wasm32"))]
            {
                match pollster::block_on(gfx::Gfx::new(gfx_window)) {
                    Ok(gfx_state) => *gfx.borrow_mut() = Some(gfx_state),
                    Err(err) => App::log(format!("gpu init failed: {err}")),
                }
            }

            self.window = Some(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.event_count < 8 {
            self.event_count += 1;
            Self::log(format!("window_event: {event:?}"));
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
                    gfx.resize(size.width, size.height);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.shift = state.shift_key();
                self.ctrl = state.control_key();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = self.window.as_ref().map(|w| w.inner_size());
                if let (Some(size), Some(gfx)) = (size, self.gfx.borrow_mut().as_mut()) {
                    gfx.set_scale_factor(scale_factor, size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.drain_socket_events();
                let viewport = {
                    let gfx = self.gfx.borrow();
                    gfx.as_ref().map(|gfx| {
                        let scale = gfx.ui_scale();
                        let (width, height) = gfx.size();
                        rqt::gui::QRectF::new(0.0, 0.0, width as f32 / scale, height as f32 / scale)
                    })
                };
                if let Some(viewport) = viewport {
                    self.mdi.set_viewport(viewport);
                }
                if let Some((form_id, name)) = self.focused.clone() {
                    let visible = (now_ms() - self.caret_epoch) % 1000.0 < 500.0;
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        if let Some(widget) = form.tree.find_widget(&name) {
                            form.tree.set_cursor_visible(widget, visible);
                        }
                    }
                }
                let rects = self.collect_rects();
                let texts = self.collect_texts();
                if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
                    gfx.render(
                        wgpu::Color {
                            r: 0.11,
                            g: 0.12,
                            b: 0.14,
                            a: 1.0,
                        },
                        &rects,
                        &texts,
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.ui_scale();
                let previous = self.dragging.as_ref().map(|drag| drag.last);
                self.cursor = (position.x as f32 / scale, position.y as f32 / scale);
                if self.left_down {
                    let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                    let mut moved = false;
                    for form in self.forms.values_mut() {
                        if form.tree.mouse_move(point) {
                            moved = true;
                            break;
                        }
                    }
                    if moved {
                        return;
                    }
                    if let Some(select) = self.line_select.clone() {
                        if let Some(form) = self.forms.get_mut(&select.form_id) {
                            if let Some(widget) = form.tree.find_widget(&select.name) {
                                if let Some(rect) =
                                    form.tree.widget(widget).map(|widget| widget.geometry)
                                {
                                    form.tree
                                        .select_line_edit_to(widget, self.cursor.0 - rect.x);
                                    self.caret_epoch = now_ms();
                                    return;
                                }
                            }
                        }
                    }
                }
                if let Some((form_id, name, column, last_x)) =
                    self.column_drag.as_ref().map(|drag| {
                        (
                            drag.form_id.clone(),
                            drag.name.clone(),
                            drag.column,
                            drag.last_x,
                        )
                    })
                {
                    let delta = self.cursor.0 - last_x;
                    if let Some(table) = self.table_mut_for(&form_id, &name) {
                        let width = table.model.column_width(column) + delta;
                        table.resize_column(column, width);
                    }
                    if let Some(drag) = self.column_drag.as_mut() {
                        drag.last_x = self.cursor.0;
                    }
                    return;
                }
                if let Some(last) = previous {
                    self.drag_card(self.cursor.0 - last.0, self.cursor.1 - last.1);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.scroll_click {
                    self.scroll_click = false;
                    self.left_down = false;
                    self.table_press = None;
                    let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                    for form in self.forms.values_mut() {
                        form.tree.mouse_release(form.root, point);
                    }
                    return;
                }
                if self.dragging.take().is_some() || self.column_drag.take().is_some() {
                    return;
                }
                self.dispatch_mouse_release();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.dispatch_mouse_press();
            }
            WindowEvent::MouseInput { .. } => {}
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.dispatch_key(&event);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self.ui_scale();
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (x * QTABLE_ROW_H * 3.0, y * QTABLE_ROW_H * 3.0)
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x as f32 / scale, pos.y as f32 / scale)
                    }
                };
                if dx != 0.0 || dy != 0.0 {
                    self.scroll_view(dx, dy);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn run(params: Params) {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new(params);
    event_loop.run_app(&mut app).expect("run_app");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
    run(Params::from_location());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(params: Params) {
    run(params);
}

#[cfg(test)]
mod tests {
    use super::{key_edit, App, KeyEdit, Params, QtForm};
    use rqt::core::QVariant;
    use rqt::gui::QRectF;
    use rqt::widgets::QWidgetTree;
    use winit::keyboard::{Key, NamedKey};

    fn card_form(x: f32, y: f32, title: &str) -> QtForm {
        let mut tree = QWidgetTree::new();
        let root = tree.create_widget("QDialog", "Dialog");
        let widget = tree.widget_mut(root).unwrap();
        widget.geometry = QRectF::new(x, y, 300.0, 200.0);
        widget.size_hint = rqt::gui::QSizeF::new(300.0, 200.0);
        widget.set_property("windowTitle", QVariant::String(title.into()));
        QtForm { tree, root }
    }

    #[test]
    fn mdi_subwindows_hit_test_and_raise() {
        use rqt::mdi::QMdiSubWindowHit;

        let mut app = App::new(Params::default());
        app.forms.insert("a".into(), card_form(32.0, 54.0, "One"));
        app.mdi
            .add_subwindow("One", "a".into(), rqt::gui::QSizeF::new(300.0, 200.0));
        app.forms.insert("b".into(), card_form(60.0, 82.0, "Two"));
        app.mdi
            .add_subwindow("Two", "b".into(), rqt::gui::QSizeF::new(300.0, 200.0));

        assert_eq!(
            app.mdi_hit((40.0, 30.0)).map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Title)
        );
        assert_eq!(
            app.mdi_hit((315.0, 32.0)).map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Close)
        );
        assert_eq!(
            app.mdi_hit((345.0, 60.0)).map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Close)
        );
        assert_eq!(app.mdi_hit((100.0, 100.0)).map(|(index, _)| index), Some(1));

        app.activate_form("a");
        assert_eq!(app.mdi.active_payload(), Some(&"a".to_string()));
        assert_eq!(app.mdi_hit((100.0, 100.0)).map(|(index, _)| index), Some(1));

        app.close_card("a");
        assert!(!app.forms.contains_key("a"));
        assert_eq!(app.mdi.active_payload(), Some(&"b".to_string()));
    }

    #[test]
    fn printable_text_is_inserted() {
        assert_eq!(
            key_edit(Some("abc"), &Key::Character("abc".into())),
            KeyEdit::Insert("abc".into())
        );
        // Space arrives as text on the web, too.
        assert_eq!(
            key_edit(Some(" "), &Key::Named(NamedKey::Space)),
            KeyEdit::Insert(" ".into())
        );
    }

    #[test]
    fn enter_and_tab_are_not_typed_into_widgets() {
        // The web backend maps Enter to "\r" and Tab to "\t" in `text`.
        // Enter is the cell-edit commit (stage 5), Tab stays a no-op —
        // neither is ever typed into the widget.
        assert_eq!(
            key_edit(Some("\r"), &Key::Named(NamedKey::Enter)),
            KeyEdit::Commit
        );
        assert_eq!(
            key_edit(Some("\t"), &Key::Named(NamedKey::Tab)),
            KeyEdit::Ignore
        );
        assert_eq!(
            key_edit(None, &Key::Named(NamedKey::Enter)),
            KeyEdit::Commit
        );
    }

    #[test]
    fn special_keys_map_to_edit_actions() {
        assert_eq!(
            key_edit(None, &Key::Named(NamedKey::Backspace)),
            KeyEdit::Backspace
        );
        assert_eq!(
            key_edit(None, &Key::Named(NamedKey::Escape)),
            KeyEdit::ClearFocus
        );
        // Non-text keys without a printable payload are ignored.
        assert_eq!(
            key_edit(None, &Key::Named(NamedKey::Shift)),
            KeyEdit::Ignore
        );
    }
}
