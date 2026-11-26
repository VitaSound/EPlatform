//! EPlatform browser shell: winit window + wgpu renderer + Phoenix channel.
//! Stages 3–5: Qt `.ui` XML → widget tree → box layout → rect batch
//! render, the event loop (mouse/keyboard → Lua → `setProperty`) and
//! the virtualized `QTableWidget`.

mod gfx;
mod shell;
mod socket;

use rqt::item_model::{QTABLE_HEADER_H, QTABLE_ROW_H};
use rqt::mdi::{QMdiArea, QMdiSubWindow, QMdiSubWindowButton, QMdiSubWindowHit, QMDI_TITLE_H};
use rqt::widgets::{QWidgetId, QWidgetTree};
use shell::Shell;
use socket::{PhoenixSocket, SocketEvent};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{CursorIcon, Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Actions forwarded by the hidden mobile text field. The JS side owns
    /// text composition and key semantics; Rust only sees `(kind, text)`.
    static WEB_TEXT_INPUT: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// In-progress cell edit of a `QTableWidget`: the buffer lives on the
/// client until Enter/blur commits it (only then does it reach Lua).
#[derive(Debug, Clone)]
struct CellEdit {
    form_id: String,
    table: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MdiDragMode {
    Move,
    Resize(QMdiSubWindowHit),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowLayer {
    Mdi,
    Floating,
}

#[derive(Debug, Clone)]
struct Drag {
    layer: WindowLayer,
    index: usize,
    mode: MdiDragMode,
    last: (f32, f32),
    /// Frame and cursor at press time, used for anchored border resizing.
    origin_frame: rqt::gui::QRectF,
    origin_cursor: (f32, f32),
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
    under_mouse_widget: Option<QWidgetId>,
}

#[derive(Debug, Clone)]
struct TablePress {
    form_id: String,
    name: String,
    already_selected: bool,
}

#[derive(Clone, Debug)]
pub struct Params {
    backend: String,
    app: String,
    lua: String,
    ui: String,
    /// Full application shell (`true`) or standalone form (`false`).
    full: bool,
    /// Lua script for the main session when `full` is true.
    main_lua: Option<String>,
    /// Optional `QMainWindow` `.ui` for the main session (0.14.8).
    main_ui: Option<String>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            backend: "ws://127.0.0.1:4000/socket/websocket?vsn=2.0.0".into(),
            app: "demo".into(),
            lua: "test_01.lua".into(),
            ui: "test_dialog01.ui".into(),
            full: false,
            main_lua: None,
            main_ui: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Params {
    fn from_location() -> Self {
        let mut params = Params::default();
        let mut has_app = false;
        let mut ui_value = None;
        let mut main_ui_value = None;
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
                            params.backend = percent_decode(&value);
                        }
                        "app" => {
                            params.app = value;
                            has_app = true;
                        }
                        "lua" => params.lua = value,
                        "ui" => {
                            params.ui = value.clone();
                            ui_value = Some(value);
                        }
                        "mainui" => main_ui_value = Some(value),
                        "main" => {
                            params.main_lua = Some(value);
                            params.full = true;
                        }
                        "mode" if value == "full" => {
                            params.full = true;
                        }
                        "mode" if value == "standalone" => {
                            params.full = false;
                        }
                        _ => {}
                    }
                }
            }
            if !has_app {
                params.app = unique_app_id();
            }
            if params.full && params.main_lua.is_none() {
                params.main_lua = Some(params.lua.clone());
            }
            if params.full {
                params.main_ui = main_ui_value
                    .or(ui_value)
                    .or_else(|| Some("mainwindow.ui".to_string()));
            }
            if params.backend.starts_with("ws://127.0.0.1") {
                // Default backend follows the page host so remote browsers work.
                if let Ok(host) = location.hostname() {
                    if !host.is_empty() && host != "localhost" {
                        params.backend = format!("ws://{host}:4000/socket/websocket?vsn=2.0.0");
                    }
                }
            }
            params.backend = backend_with_version(params.backend);
        }
        params
    }
}

/// Decode `%3A`, `%2F`, `%3F`, … in a URL parameter. Safari may percent-encode
/// characters in `location.search` that Chrome leaves raw.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = (bytes[index + 1] as char).to_digit(16);
            let low = (bytes[index + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (high, low) {
                out.push((high * 16 + low) as u8);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Phoenix needs the JSON serializer version in the WebSocket query. Add it
/// ourselves when the URL has no query: `?backend=.../websocket?...` inside a
/// page URL is fragile in mobile Safari, which may percent-encode the second
/// `?` before handing `location.search` to wasm.
fn backend_with_version(backend: String) -> String {
    if (backend.starts_with("ws://") || backend.starts_with("wss://")) && !backend.contains('?') {
        format!("{backend}?vsn=2.0.0")
    } else {
        backend
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

#[cfg(any(target_arch = "wasm32", test))]
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Data URL is more reliable than a Blob URL for dynamically set favicons:
/// no MIME/ownership issues and no object URLs to revoke.
#[cfg(any(target_arch = "wasm32", test))]
fn png_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64_encode(png))
}

#[cfg(target_arch = "wasm32")]
fn set_favicon(href: &str) {
    use wasm_bindgen::JsCast;

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(head) = document.head() else {
        return;
    };

    // Replace the link so the browser re-fetches the favicon instead of
    // keeping the previously parsed one.
    if let Ok(Some(existing)) = document.query_selector("link[rel~='icon']") {
        existing.remove();
    }

    let Ok(element) = document.create_element("link") else {
        return;
    };
    let Ok(link) = element.dyn_into::<web_sys::HtmlLinkElement>() else {
        return;
    };
    link.set_rel("icon");
    link.set_type("image/png");
    link.set_href(href);
    let _ = head.append_child(&link);
}

/// Browser-chrome state for the application (the old `App:` sugar).
///
/// `windowTitle`/`windowIcon` are applied by the browser client. The
/// application shell applies `menu`, `toolBar`, `statusMessage` and status
/// visibility when running in full mode; native-window geometry
/// (`size`, `position`, `geometry`, `centered`, `windowState`, `visible`) is
/// accepted but intentionally ignored in the browser.
#[derive(Debug, Default)]
struct AppChrome {
    window_title: Option<String>,
    window_icon: Option<String>,
}

#[derive(Debug, PartialEq)]
enum AppChromeAction {
    None,
    Title(String),
    /// URL / `data:` URI that the browser can use directly.
    Icon(String),
    /// Legacy `img24:N` sprite cell, encoded as a PNG data URL.
    IconPng(Vec<u8>),
}

impl AppChrome {
    fn apply(&mut self, property: &str, value: &form_xml::PropValue) -> AppChromeAction {
        match property {
            "windowTitle" => match value.as_str() {
                Some(title) => {
                    self.window_title = Some(title.to_string());
                    AppChromeAction::Title(title.to_string())
                }
                None => AppChromeAction::None,
            },
            "windowIcon" => match value.as_str() {
                Some(icon) => {
                    self.window_icon = Some(icon.to_string());
                    if let Some(url) = icon_url(icon) {
                        AppChromeAction::Icon(url.to_string())
                    } else if let Some(png) = rqt::icons::png_bytes(icon) {
                        AppChromeAction::IconPng(png)
                    } else {
                        AppChromeAction::None
                    }
                }
                None => AppChromeAction::None,
            },
            _ => AppChromeAction::None,
        }
    }
}

/// Icon spec → URL usable as a browser favicon.
///
/// Legacy Qt specs (`img24:N`, `img24d:N`, `:/icons/...`) cannot be resolved
/// in the browser and are left for a desktop host.
fn icon_url(spec: &str) -> Option<&str> {
    let spec = spec.trim();
    if spec.is_empty()
        || spec.starts_with("img24:")
        || spec.starts_with("img24d:")
        || spec.starts_with(":/")
    {
        return None;
    }
    if spec.starts_with("data:") || spec.starts_with("http://") || spec.starts_with("https://") {
        return Some(spec);
    }
    if spec.starts_with('/') || spec.contains('/') {
        return Some(spec);
    }
    None
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
    /// Focused `QLineEdit`: `(form_id, objectName)`. Keyboard focus itself
    /// lives on `QWidget.has_focus`; this mirrors the line edit that
    /// receives text input.
    focused: Option<(String, String)>,
    /// Focused item view: `(form_id, objectName)` — typing starts
    /// editing its current cell (Qt's `AnyKeyPressed` edit trigger).
    table_focus: Option<(String, String)>,
    /// In-progress cell edit of `table_focus`.
    editing: Option<CellEdit>,
    /// Laid-out Qt widget trees by `form_id` (one per open dialog session).
    forms: HashMap<String, QtForm>,
    /// Global painting/hit z-order across MDI and top-level windows. The
    /// last entry is frontmost; activation moves a form there.
    window_z: Vec<(WindowLayer, String)>,
    /// Stack of currently open modal forms. The last one owns input.
    modal_stack: Vec<String>,
    /// Modal scope by form id (`window` / `application`).
    modal_scopes: HashMap<String, String>,
    mdi: QMdiArea<String>,
    /// Top-level forms (`placement=top-level`) live outside the central MDI.
    floating: QMdiArea<String>,
    /// Application chrome (menu/toolbar/status) when running in full mode.
    shell: Shell,
    /// `form_id` of the main session returned by `start_app`.
    main_form_id: Option<String>,
    /// Last activated form, independent of its layer.
    active_form: Option<String>,
    dragging: Option<Drag>,
    column_drag: Option<ColumnDrag>,
    line_select: Option<LineSelect>,
    left_down: bool,
    /// A press was consumed by the menu/toolbar, so its release must not
    /// fall through to the form under the cursor.
    shell_press: bool,
    shift: bool,
    ctrl: bool,
    caret_epoch: f64,
    table_press: Option<TablePress>,
    scroll_click: bool,
    mdi_hover: Option<(WindowLayer, usize, QMdiSubWindowHit)>,
    /// Active resize cursor; `None` means the default arrow.
    cursor_hit: Option<QMdiSubWindowHit>,
    /// Touch contact that owns the pointer; additional contacts drive
    /// two-finger scrolling (pinch zoom is a separate task).
    primary_touch: Option<u64>,
    secondary_touches: Vec<(u64, f32, f32)>,
    /// Start of the active touch, for long-press detection.
    touch_start: Option<(f32, f32, f64)>,
    long_tap_sent: bool,
    /// Time of the last touch event, used to detect a missing `Ended`.
    last_touch_ms: f64,
    /// Last click/tap, for word selection on a double tap.
    last_tap: Option<(String, String, f32, f32, f64)>,
    /// Last rectangle sent to the mobile HTML input overlay.
    web_input_rect: Option<rqt::gui::QRectF>,
    /// Application-level (browser chrome) properties from `setAppProperty`.
    chrome: AppChrome,
    /// Last size reported by `WindowEvent::Resized`.
    ///
    /// winit-web can deliver `Resized` before `Gfx` is ready and then keep
    /// `Window::inner_size()` at the initial 1x1 canvas size, so the pending
    /// size must be applied when the surface appears.
    last_size: Rc<Cell<Option<(u32, u32)>>>,
}

impl App {
    fn new(params: Params) -> Self {
        let (_tx, rx) = channel();
        let full = params.full;
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
            window_z: Vec::new(),
            modal_stack: Vec::new(),
            modal_scopes: HashMap::new(),
            mdi: QMdiArea::new(rqt::gui::QRectF::new(0.0, 0.0, 1280.0, 800.0)),
            floating: QMdiArea::new(rqt::gui::QRectF::new(0.0, 0.0, 1280.0, 800.0)),
            shell: Shell {
                visible: full,
                status_visible: full,
                ..Shell::default()
            },
            main_form_id: None,
            active_form: None,
            dragging: None,
            column_drag: None,
            line_select: None,
            left_down: false,
            shell_press: false,
            shift: false,
            ctrl: false,
            caret_epoch: now_ms(),
            table_press: None,
            scroll_click: false,
            mdi_hover: None,
            cursor_hit: None,
            primary_touch: None,
            secondary_touches: Vec::new(),
            touch_start: None,
            long_tap_sent: false,
            last_touch_ms: 0.0,
            last_tap: None,
            web_input_rect: None,
            chrome: AppChrome::default(),
            last_size: Rc::new(Cell::new(None)),
        }
    }

    fn apply_window_title(&self, title: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                document.set_title(title);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(window) = &self.window {
                window.set_title(title);
            }
        }
    }

    fn apply_window_icon(&self, url: &str) {
        #[cfg(target_arch = "wasm32")]
        set_favicon(url);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = url;
    }

    fn apply_window_icon_png(&self, png: &[u8]) {
        #[cfg(target_arch = "wasm32")]
        set_favicon(&png_data_url(png));
        #[cfg(not(target_arch = "wasm32"))]
        let _ = png;
    }

    /// Surface size in device pixels.
    ///
    /// The browser is the only reliable source of the canvas geometry: winit's
    /// web backend has reported `inner_size()` both as CSS pixels and as device
    /// pixels across versions/browsers (it forwards `ResizeObserver`'s box,
    /// which differs between Firefox and Chromium and between observer modes).
    /// Deriving the size from `getBoundingClientRect() * devicePixelRatio` keeps
    /// the surface, `ui_scale` and the pointer mapping in the same units no
    /// matter what winit decides to report.
    #[cfg(target_arch = "wasm32")]
    fn dom_canvas_size(&self) -> Option<(f64, f64, f64)> {
        use winit::platform::web::WindowExtWebSys;
        let window = self.window.as_ref()?;
        let canvas = window.canvas()?;
        let rect = canvas.get_bounding_client_rect();
        let dpr = window.scale_factor();
        if !(rect.width() > 0.0) || !(rect.height() > 0.0) || !(dpr > 0.0) {
            return None;
        }
        Some((rect.width(), rect.height(), dpr))
    }

    #[allow(dead_code)]
    fn physical_size(&self, size: winit::dpi::PhysicalSize<u32>) -> (u32, u32) {
        #[cfg(target_arch = "wasm32")]
        if let Some((width, height, dpr)) = self.dom_canvas_size() {
            return (
                (width * dpr).round().max(1.0) as u32,
                (height * dpr).round().max(1.0) as u32,
            );
        }
        (size.width.max(1), size.height.max(1))
    }

    /// Device-pixel size after a DPR change; see `physical_size`.
    #[allow(dead_code)]
    fn scaled_size(&self, scale_factor: f64) -> (u32, u32) {
        #[cfg(target_arch = "wasm32")]
        if let Some((width, height, dpr)) = self.dom_canvas_size() {
            let dpr = if dpr > 0.0 { dpr } else { scale_factor };
            return (
                (width * dpr).round().max(1.0) as u32,
                (height * dpr).round().max(1.0) as u32,
            );
        }
        let size = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or(winit::dpi::PhysicalSize::new(0, 0));
        let _ = scale_factor;
        (size.width.max(1), size.height.max(1))
    }

    /// Keep the canvas backing store in device pixels; the CSS size stays
    /// 100% so the browser scales it down (HiDPI stays crisp).
    #[cfg(target_arch = "wasm32")]
    fn sync_canvas_backing(&self, width: u32, height: u32) {
        use winit::platform::web::WindowExtWebSys;
        let Some(canvas) = self.window.as_ref().and_then(|window| window.canvas()) else {
            return;
        };
        if canvas.width() != width || canvas.height() != height {
            canvas.set_width(width);
            canvas.set_height(height);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_canvas_backing(&self, _width: u32, _height: u32) {}

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
                SocketEvent::Opened => {
                    Self::log("websocket opened");
                    connection_error_banner(false, "");
                }
                SocketEvent::Joined => {
                    Self::log("channel joined");
                    self.joined = true;
                }
                SocketEvent::Reply(payload) => {
                    if payload.get("role").and_then(|v| v.as_str()) == Some("main") {
                        self.main_form_id = payload
                            .get("form_id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        Self::log(format!(
                            "main session form_id={}",
                            self.main_form_id.as_deref().unwrap_or("")
                        ));
                    }
                }
                SocketEvent::Command(payload) => {
                    Self::log(format!("command: {payload}"));
                    self.on_command(&payload);
                }
                SocketEvent::Closed => {
                    Self::log("websocket closed");
                    self.joined = false;
                    self.start_sent = false;
                    connection_error_banner(true, CONNECTION_FAILED);
                }
                SocketEvent::Error(err) => {
                    Self::log(format!("websocket error: {err}"));
                    connection_error_banner(true, CONNECTION_FAILED);
                }
            }
        }

        // Once joined, ask the server to start the main session or a
        // standalone form. In full mode the main Lua script owns the menu and
        // opens forms later; it does not create a form itself.
        if self.joined && !self.start_sent {
            if let Some(socket) = &self.socket {
                if self.params.full {
                    let main = self
                        .params
                        .main_lua
                        .clone()
                        .unwrap_or_else(|| self.params.lua.clone());
                    let mut payload = serde_json::json!({
                        "lua_file": main,
                        "mode": "full",
                    });
                    if let Some(ui_file) = &self.params.main_ui {
                        payload["ui_file"] = serde_json::Value::String(ui_file.clone());
                    }
                    socket.send("start_app", payload);
                    Self::log(format!(
                        "start_app sent lua_file={main} ui_file={}",
                        self.params.main_ui.as_deref().unwrap_or("")
                    ));
                } else {
                    socket.send(
                        "start_dialog",
                        serde_json::json!({
                            "lua_file": self.params.lua,
                            "ui_file": self.params.ui,
                            "mode": "standalone",
                            "placement": "top-level",
                        }),
                    );
                    Self::log("start_dialog sent (standalone)");
                }
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
            "loadMainWindow" => self.on_load_main_window(payload),
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
                if let Some((layer, index)) = self.find_window(form_id) {
                    if property == "cascade" && value.as_bool() == Some(true) {
                        // `Form:cascade()`: move this window to the next
                        // cascade slot without touching its size.
                        self.area_mut(layer).cascade_window(index);
                        Self::log(format!("apply form.cascade form_id={form_id}"));
                        return;
                    }
                    let current = self
                        .forms
                        .get(form_id)
                        .and_then(|form| form.tree.widget(form.root))
                        .map(|widget| widget.geometry)
                        .unwrap_or_default();
                    let centered = if property == "centered" && value.as_bool() == Some(true) {
                        let area = self.area(layer);
                        let viewport = area.viewport();
                        area.subwindow(index).map(|subwindow| {
                            let content = subwindow.content_rect();
                            rqt::gui::QRectF::new(
                                viewport.x + ((viewport.width - content.width) / 2.0).max(0.0),
                                viewport.y + ((viewport.height - content.height) / 2.0).max(0.0),
                                content.width,
                                content.height,
                            )
                        })
                    } else {
                        None
                    };
                    let (rect, form_local) = if let Some(centered) = centered {
                        (Some(centered), false)
                    } else {
                        match &value {
                            form_xml::PropValue::Rect { x, y, w, h } => (
                                Some(rqt::gui::QRectF::new(
                                    *x as f32, *y as f32, *w as f32, *h as f32,
                                )),
                                true,
                            ),
                            // `size` keeps the current (already absolute) position.
                            form_xml::PropValue::Size { w, h } if property == "size" => (
                                Some(rqt::gui::QRectF::new(
                                    current.x, current.y, *w as f32, *h as f32,
                                )),
                                false,
                            ),
                            form_xml::PropValue::Map(_) if property == "position" => {
                                let x = value.get("x").and_then(|x| x.as_i64());
                                let y = value.get("y").and_then(|y| y.as_i64());
                                (
                                    Some(rqt::gui::QRectF::new(
                                        x.map(|x| x as f32).unwrap_or(current.x),
                                        y.map(|y| y as f32).unwrap_or(current.y),
                                        current.width,
                                        current.height,
                                    )),
                                    x.is_some() && y.is_some(),
                                )
                            }
                            _ => (None, false),
                        }
                    };
                    if let Some(rect) = rect {
                        // Qt form-level geometry is relative to the window
                        // manager: MDI content coordinates are inside the
                        // central QMdiArea, top-level ones are screen coords.
                        // `size` already carries an absolute current position.
                        let rect = if layer == WindowLayer::Mdi && form_local {
                            let offset = self.mdi.viewport();
                            rqt::gui::QRectF::new(
                                rect.x + offset.x,
                                rect.y + offset.y,
                                rect.width,
                                rect.height,
                            )
                        } else {
                            rect
                        };
                        let area = self.area_mut(layer);
                        area.set_content_geometry(index, rect);
                        let content = area
                            .subwindow(index)
                            .map(|subwindow| subwindow.content_rect())
                            .unwrap_or(rect);
                        if let Some(form) = self.forms.get_mut(form_id) {
                            form.tree.set_geometry(form.root, content);
                        }
                    } else if let Some(form) = self.forms.get_mut(form_id) {
                        form.tree.set_property(
                            form.root,
                            property,
                            rqt::core::QVariant::from_prop(&value),
                        );
                        form.tree.relayout(form.root);
                    }

                    if property == "minimumSize" || property == "maximumSize" {
                        let (min, max) = self
                            .forms
                            .get_mut(form_id)
                            .map(|form| {
                                let root = form.root;
                                let max = form
                                    .tree
                                    .widget(root)
                                    .map(|widget| widget.maximum_size)
                                    .unwrap_or_default();
                                (form.tree.minimum_size_hint(root), max)
                            })
                            .unwrap_or_default();
                        let area = self.area_mut(layer);
                        area.set_size_constraints(index, min, max);
                        if let Some(content) = area
                            .subwindow(index)
                            .map(|subwindow| subwindow.content_rect())
                        {
                            if let Some(form) = self.forms.get_mut(form_id) {
                                form.tree.set_geometry(form.root, content);
                            }
                        }
                    }
                    if property == "resizable" {
                        if let Some(enabled) = value.as_bool() {
                            self.area_mut(layer).set_resizable(index, enabled);
                        }
                    }
                    title = self
                        .forms
                        .get(form_id)
                        .map(|form| form_title(&form.tree, form.root, form_id));
                    Self::log(format!("apply form.{property}={}", prop_brief(&value)));
                }
                if property == "windowTitle" {
                    if let (Some(title), Some((layer, index))) = (title, self.find_window(form_id))
                    {
                        self.area_mut(layer).set_title(index, title);
                    }
                }
            }
            "setAppProperty" => {
                let property = str_field(payload, "property");
                if payload.get("role").and_then(|v| v.as_str()) == Some("main") {
                    if let Some(main_id) = payload.get("source_form_id").and_then(|v| v.as_str()) {
                        self.main_form_id = Some(main_id.to_string());
                    }
                }
                let Some(value) = prop_from_json(payload.get("value")) else {
                    return;
                };
                match property {
                    "menu" => {
                        self.shell.set_menu(&value);
                        Self::log(format!("apply app.menu items={}", self.shell.menu.len()));
                    }
                    "toolBar" | "toolbar" => {
                        self.shell.set_toolbar(&value);
                        Self::log(format!(
                            "apply app.toolBar items={}",
                            self.shell.toolbar.len()
                        ));
                    }
                    "toolBarVisible" | "toolbarVisible" => {
                        if let Some(visible) = value.as_bool() {
                            self.shell.toolbar_visible = visible;
                        }
                    }
                    "statusMessage" | "status" => {
                        let text = value.as_str().map(str::to_string).or_else(|| {
                            value
                                .get("text")
                                .and_then(|v| v.as_str())
                                .map(str::to_string)
                        });
                        let timeout = value
                            .get("timeout")
                            .and_then(|value| value.as_i64())
                            .map(|value| value as f64);
                        self.shell.set_status_message(text, timeout, now_ms());
                    }
                    "statusInfo" => {
                        self.shell
                            .set_status_info(value.as_str().map(str::to_string));
                    }
                    "statusVisible" | "statusbar" => {
                        if let Some(visible) = value.as_bool() {
                            self.shell.status_visible = visible;
                        }
                    }
                    "cascadeWindows" => {
                        if value.as_bool() != Some(false) {
                            self.mdi.cascade_subwindows();
                            self.floating.cascade_subwindows();
                            Self::log("cascade windows".to_string());
                        }
                    }
                    "tileWindows" => {
                        if value.as_bool() != Some(false) {
                            self.mdi.tile_subwindows();
                            self.floating.tile_subwindows();
                            Self::log("tile windows".to_string());
                        }
                    }
                    _ => match self.chrome.apply(property, &value) {
                        AppChromeAction::Title(title) => {
                            self.apply_window_title(&title);
                            Self::log(format!("apply app.windowTitle={title}"));
                        }
                        AppChromeAction::Icon(url) => {
                            self.apply_window_icon(&url);
                            Self::log(format!("apply app.windowIcon={url}"));
                        }
                        AppChromeAction::IconPng(png) => {
                            self.apply_window_icon_png(&png);
                            Self::log(format!("apply app.windowIcon=png:{} bytes", png.len()));
                        }
                        AppChromeAction::None => {
                            Self::log(format!("ignore app.{property}"));
                        }
                    },
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
                self.main_form_id = None;
                self.shell.visible = false;
                self.shell.close_menu();
                Self::log("closeApplication");
            }
            _ => {}
        }
    }

    fn remove_form(&mut self, form_id: &str) {
        self.mdi_hover = None;
        self.forms.remove(form_id);
        self.window_z.retain(|(_, id)| id != form_id);
        let was_modal = self.modal_stack.iter().any(|id| id == form_id);
        self.modal_stack.retain(|id| id != form_id);
        self.modal_scopes.remove(form_id);
        if let Some((layer, index)) = self.find_window(form_id) {
            self.area_mut(layer).remove(index);
        }
        if self.active_form.as_deref() == Some(form_id) {
            self.active_form = None;
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
        if was_modal {
            if let Some(active) = self.active_form_id() {
                self.focus_initial_form(&active);
            }
        }
        self.sync_web_text_input();
    }

    fn remove_all_forms(&mut self) {
        self.mdi_hover = None;
        self.forms.clear();
        self.window_z.clear();
        self.modal_stack.clear();
        self.modal_scopes.clear();
        self.mdi.clear();
        self.floating.clear();
        self.active_form = None;
        self.focused = None;
        self.table_focus = None;
        self.editing = None;
        self.column_drag = None;
        self.line_select = None;
        self.sync_web_text_input();
    }

    fn area(&self, layer: WindowLayer) -> &QMdiArea<String> {
        match layer {
            WindowLayer::Mdi => &self.mdi,
            WindowLayer::Floating => &self.floating,
        }
    }

    fn area_mut(&mut self, layer: WindowLayer) -> &mut QMdiArea<String> {
        match layer {
            WindowLayer::Mdi => &mut self.mdi,
            WindowLayer::Floating => &mut self.floating,
        }
    }

    fn find_window(&self, form_id: &str) -> Option<(WindowLayer, usize)> {
        if let Some(index) = self.mdi.index_of(|payload| payload == form_id) {
            return Some((WindowLayer::Mdi, index));
        }
        self.floating
            .index_of(|payload| payload == form_id)
            .map(|index| (WindowLayer::Floating, index))
    }

    fn active_form_id(&self) -> Option<String> {
        if let Some(active) = &self.active_form {
            if self.forms.contains_key(active) {
                return Some(active.clone());
            }
        }
        self.mdi
            .active_payload()
            .cloned()
            .or_else(|| self.floating.active_payload().cloned())
    }

    /// Parse the Qt `.ui` XML into the Qt widget tree and place it either
    /// into the central MDI area (`placement=mdi`) or into the top-level
    /// layer (`placement=top-level`). Standalone mode coerces every form to
    /// top-level: there is no main window / MDI area to attach to.
    /// 0.14.8: static `QMainWindow.ui` parts (menu/toolbar/status/title).
    ///
    /// Loaded before the main Lua script sends dynamic `menu`/`toolBar`
    /// properties; later those properties replace the static model.
    fn on_load_main_window(&mut self, payload: &serde_json::Value) {
        let xml = payload
            .get("ui_xml")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        match shell::parse_main_window(xml) {
            Ok(spec) => {
                if let Some(title) = &spec.title {
                    self.apply_window_title(title);
                }
                Self::log(format!(
                    "loadMainWindow menu={} toolbar={} status={} central={} mdi={}",
                    spec.menu.len(),
                    spec.toolbar.len(),
                    spec.status_visible,
                    spec.central_class.as_deref().unwrap_or(""),
                    spec.central_is_mdi
                ));
                self.shell.apply_main_window(spec);
            }
            Err(error) => Self::log(format!("loadMainWindow failed: {error}")),
        }
    }

    /// Recompute both window layers' viewports from the current canvas and
    /// shell chrome. Called on every redraw *and before creating a form*:
    /// a form that opens right after `setToolBar` (e.g. the gallery's
    /// auto-open) would otherwise be placed against a stale viewport and
    /// slide under the toolbar.
    fn refresh_viewports(&mut self) {
        let viewport = {
            let gfx = self.gfx.borrow();
            gfx.as_ref().map(|gfx| {
                let scale = gfx.ui_scale();
                let (width, height) = gfx.size();
                rqt::gui::QRectF::new(0.0, 0.0, width as f32 / scale, height as f32 / scale)
            })
        };
        match viewport {
            Some(viewport) => {
                self.floating.set_viewport(viewport);
                self.mdi.set_viewport(self.shell.central(viewport));
            }
            None => {
                // Before the GPU is up the canvas size is unknown, but a
                // form that auto-opens right after the shell gained its
                // menu/toolbar must still be placed below the chrome.
                // Tests (no GPU) are skipped unless the shell chrome is set.
                let chrome = self.shell.visible
                    && (!self.shell.menu.is_empty()
                        || (self.shell.toolbar_visible && !self.shell.toolbar.is_empty()));
                if chrome {
                    let current = self.floating.viewport();
                    self.mdi.set_viewport(self.shell.central(current));
                }
            }
        }
    }

    fn on_create_form(&mut self, payload: &serde_json::Value) {
        // Place against the *current* chrome: the shell may have gained a
        // menu/toolbar since the last redraw (auto-opened forms).
        self.refresh_viewports();
        let form_id = payload
            .get("form_id")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string();
        let xml = payload.get("ui_xml").and_then(|v| v.as_str()).unwrap_or("");
        let modal = payload
            .get("modal")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let modal_scope = payload
            .get("modal_scope")
            .and_then(|v| v.as_str())
            .unwrap_or("application")
            .to_string();
        let standalone = payload.get("mode").and_then(|v| v.as_str()) == Some("standalone");
        let placement = payload.get("placement").and_then(|v| v.as_str());
        let layer = if standalone || placement == Some("top-level") {
            WindowLayer::Floating
        } else {
            WindowLayer::Mdi
        };

        let (mut tree, root) = match rqt::widgets::load_ui(xml) {
            Ok(form) => form,
            Err(err) => {
                Self::log(format!("createForm parse error: {err}"));
                return;
            }
        };
        let form_area = tree
            .widget(root)
            .map(|widget| widget.geometry)
            .unwrap_or_default();
        let title = form_title(&tree, root, &form_id);

        if let Some((old_layer, index)) = self.find_window(&form_id) {
            self.area_mut(old_layer).remove(index);
        }
        self.modal_stack.retain(|id| id != &form_id);
        self.modal_scopes.remove(&form_id);
        if modal {
            self.modal_stack.push(form_id.clone());
            self.modal_scopes.insert(form_id.clone(), modal_scope);
        }

        let index = self.area_mut(layer).add_subwindow(
            title,
            form_id.clone(),
            rqt::gui::QSizeF::new(form_area.width, form_area.height),
        );
        self.raise_window(layer, &form_id);

        let min_size = tree.minimum_size_hint(root);
        let max_size = tree
            .widget(root)
            .map(|widget| widget.maximum_size)
            .unwrap_or_default();
        let size_grip_disabled = matches!(
            tree.widget(root)
                .and_then(|widget| widget.property("sizeGripEnabled")),
            Some(rqt::core::QVariant::Bool(false))
        );
        let fixed_size = window_fixed_size(&tree, root);

        {
            let area = self.area_mut(layer);
            area.set_size_constraints(index, min_size, max_size);
            if size_grip_disabled || fixed_size {
                area.set_resizable(index, false);
            }
            if fixed_size {
                // Qt's MSWindowsFixedSizeDialogHint also removes the maximize button.
                if let Some(subwindow) = area.subwindow_mut(index) {
                    subwindow.window_flags.maximize = false;
                }
            }
        }

        if layer == WindowLayer::Floating {
            let viewport = self.floating.viewport();
            // A designer position of (0,0) means "no position given": the
            // window keeps the cascade slot assigned by `add_subwindow`.
            // Explicit `.ui`/Lua positions are honored as-is — coinciding
            // explicit positions are intentional (user decision, 0.18.1).
            let explicit_position = form_area.x != 0.0 || form_area.y != 0.0;
            if explicit_position {
                let width = form_area.width.clamp(1.0, viewport.width.max(1.0));
                let height = form_area.height.clamp(1.0, viewport.height.max(1.0));
                let x = form_area
                    .x
                    .clamp(viewport.x, (viewport.right() - width).max(viewport.x));
                let y = form_area
                    .y
                    .clamp(viewport.y, (viewport.bottom() - height).max(viewport.y));
                self.floating
                    .set_content_geometry(index, rqt::gui::QRectF::new(x, y, width, height));
            }
        }
        // MDI windows keep the absolute cascade slot from `add_subwindow`
        // (it already contains the viewport origin; no post-shift needed).

        if modal || layer == WindowLayer::Floating {
            let _ = self.area_mut(layer).activate(index);
        }
        self.active_form = Some(form_id.clone());
        let content = self
            .area(layer)
            .subwindow(index)
            .map(|subwindow| subwindow.content_rect())
            .unwrap_or(form_area);
        tree.translate(root, content.x - form_area.x, content.y - form_area.y);
        // Size constraints may have widened/narrowed the subwindow; keep the
        // root widget geometry in sync with its content rect before first paint.
        tree.set_geometry(root, content);

        Self::log(format!(
            "layout form_id={form_id} layer={layer:?} nodes={} area=({:.0},{:.0},{:.0},{:.0})",
            tree.len(),
            form_area.x,
            form_area.y,
            form_area.width,
            form_area.height
        ));
        self.forms.insert(
            form_id.clone(),
            QtForm {
                tree,
                root,
                under_mouse_widget: None,
            },
        );
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
        self.focus_initial_form(&form_id);
    }

    /// Collect the Qt-painted form commands into an ordered display list.
    /// Batches keep the painter order across shell → MDI → top-level →
    /// popup layers; no text is masked or reordered to fake z-order.
    fn collect_paint(&self) -> Vec<gfx::PaintBatch> {
        let mut batches = Vec::new();
        let style = rqt::style::QFusionStyle::default();
        let viewport = self.floating.viewport();
        let technical = format!(
            "{}  ·  app {}  ·  forms {}",
            if self.joined { "online" } else { "offline" },
            self.params.app,
            self.forms.len()
        );

        let mut base = rqt::gui::QPainter::new();
        self.shell
            .paint_base(&mut base, viewport, now_ms(), Some(&technical));
        append_paint_commands(base.commands(), &mut batches);

        let active_form = self.active_form_id();
        for (layer, index) in self.window_order() {
            let Some(subwindow) = self.area(layer).subwindow(index) else {
                continue;
            };
            let Some(form) = self.forms.get(&subwindow.payload) else {
                continue;
            };
            if !subwindow.minimized {
                let mut painter = rqt::gui::QPainter::new();
                rqt::paint::paint_widget_tree_content(&form.tree, form.root, &style, &mut painter);
                append_paint_commands(painter.commands(), &mut batches);
            }
            let active = active_form.as_deref() == Some(subwindow.payload.as_str());
            let hover = self.mdi_hover.and_then(|(hover_layer, hover_index, hit)| {
                (hover_layer == layer && hover_index == index).then_some(hit)
            });
            self.push_subwindow_chrome(subwindow, active, hover, &mut batches);
        }

        // Popups are last: their opaque backgrounds naturally cover menu,
        // toolbar and form text underneath, so no masking is needed.
        let mut overlay = rqt::gui::QPainter::new();
        self.shell.paint_overlay(&mut overlay, viewport);
        append_paint_commands(overlay.commands(), &mut batches);

        // Combo popups are top-level in Qt: paint them after MDI/top-level
        // chrome and shell popups so they cannot be covered by a frame.
        for layer in [WindowLayer::Mdi, WindowLayer::Floating] {
            for subwindow in self.area(layer).subwindows() {
                let Some(form) = self.forms.get(&subwindow.payload) else {
                    continue;
                };
                if subwindow.minimized {
                    continue;
                }
                let mut painter = rqt::gui::QPainter::new();
                rqt::paint::paint_combo_popups(&form.tree, &style, &mut painter);
                append_paint_commands(painter.commands(), &mut batches);
            }
        }
        batches
    }

    fn push_subwindow_chrome(
        &self,
        subwindow: &QMdiSubWindow<String>,
        active: bool,
        hover: Option<QMdiSubWindowHit>,
        batches: &mut Vec<gfx::PaintBatch>,
    ) {
        use rqt::gui::QPainter;

        let mut painter = QPainter::new();
        self.paint_subwindow_chrome(&mut painter, subwindow, active, hover);
        append_paint_commands(painter.commands(), batches);
    }

    fn paint_subwindow_chrome(
        &self,
        painter: &mut rqt::gui::QPainter,
        subwindow: &QMdiSubWindow<String>,
        active: bool,
        hover: Option<QMdiSubWindowHit>,
    ) {
        use rqt::gui::{QColor, QPointF, QRectF};

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
    fn mdi_hit(&self, point: (f32, f32)) -> Option<(usize, QMdiSubWindowHit)> {
        self.mdi.hit_test(rqt::gui::QPointF::new(point.0, point.1))
    }

    fn modal_form(&self) -> Option<&str> {
        self.modal_stack.last().map(String::as_str)
    }

    fn is_application_modal(&self) -> bool {
        self.modal_form().is_some_and(|form_id| {
            self.modal_scopes
                .get(form_id)
                .map(String::as_str)
                .unwrap_or("application")
                == "application"
        })
    }

    /// Like `mdi_hit`, but while a modal form is open only that form's
    /// subwindow may receive mouse input.
    #[cfg(test)]
    fn mdi_hit_modal(&self) -> Option<(usize, QMdiSubWindowHit)> {
        let hit = self.mdi_hit(self.cursor)?;
        if let Some(modal) = self.modal_form() {
            let payload = self.mdi.subwindow(hit.0)?.payload.as_str();
            if payload != modal {
                return None;
            }
        }
        Some(hit)
    }

    /// Topmost window under a point. Top-level windows sit above MDI, like
    /// separate Qt windows; modal filtering is applied separately.
    fn window_hit(&self, point: (f32, f32)) -> Option<(WindowLayer, usize, QMdiSubWindowHit)> {
        let point = rqt::gui::QPointF::new(point.0, point.1);
        let touch = self.primary_touch.is_some();
        for (layer, index) in self.window_order().into_iter().rev() {
            let Some(subwindow) = self.area(layer).subwindow(index) else {
                continue;
            };
            let hit = if touch {
                subwindow.hit_test_touch(point)
            } else {
                subwindow.hit_test(point)
            };
            if let Some(hit) = hit {
                return Some((layer, index, hit));
            }
        }
        None
    }

    fn window_hit_modal(&self) -> Option<(WindowLayer, usize, QMdiSubWindowHit)> {
        let hit = self.window_hit(self.cursor)?;
        if let Some(modal) = self.modal_form() {
            let payload = self.area(hit.0).subwindow(hit.1)?.payload.as_str();
            if payload != modal {
                return None;
            }
        }
        Some(hit)
    }

    fn raise_window(&mut self, layer: WindowLayer, form_id: &str) {
        self.window_z
            .retain(|(window_layer, id)| !(*window_layer == layer && id == form_id));
        self.window_z.push((layer, form_id.to_string()));
    }

    /// Front-to-back list for painting and hit-testing. Known windows use
    /// the global activation order; manually inserted test windows fall back
    /// to the layer/area order.
    fn window_order(&self) -> Vec<(WindowLayer, usize)> {
        let mut entries = Vec::new();
        for layer in [WindowLayer::Mdi, WindowLayer::Floating] {
            for (index, subwindow) in self.area(layer).subwindows().iter().enumerate() {
                let base = entries.len();
                let z = self
                    .window_z
                    .iter()
                    .position(|(l, id)| *l == layer && *id == subwindow.payload)
                    .unwrap_or(usize::MAX / 2 + base);
                entries.push((z, layer, index));
            }
        }
        entries.sort_by_key(|(z, _, _)| *z);
        entries
            .into_iter()
            .map(|(_, layer, index)| (layer, index))
            .collect()
    }

    fn activate_form(&mut self, form_id: &str) -> Option<usize> {
        self.mdi_hover = None;
        let (layer, index) = self.find_window(form_id)?;
        let activated = self.area_mut(layer).activate(index)?;
        self.raise_window(layer, form_id);
        for (id, form) in self.forms.iter_mut() {
            if id != form_id {
                form.tree.close_combo_popups();
            }
        }
        self.active_form = Some(form_id.to_string());
        self.focus_initial_form(form_id);
        Some(activated)
    }

    fn sync_form_geometry(&mut self, form_id: &str) {
        let Some((layer, index)) = self.find_window(form_id) else {
            return;
        };
        let Some(content) = self
            .area(layer)
            .subwindow(index)
            .map(|subwindow| subwindow.content_rect())
        else {
            return;
        };
        if let Some(form) = self.forms.get_mut(form_id) {
            form.tree.set_geometry(form.root, content);
        }
        self.sync_web_text_input();
    }

    /// Keep the browser cursor in sync with the MDI resize border under the
    /// mouse. The cursor is only changed when the resize direction changes.
    fn update_resize_cursor(&mut self, hit: Option<QMdiSubWindowHit>) {
        let next = hit.filter(|hit| hit.is_resize());
        if self.cursor_hit == next {
            return;
        }
        self.cursor_hit = next;
        if let Some(window) = &self.window {
            window.set_cursor(cursor_icon_for_hit(next));
        }
    }

    fn cancel_mouse_grabs(&mut self) {
        for form in self.forms.values_mut() {
            form.tree.cancel_mouse_grab();
        }
    }

    fn dispatch_shell_action(&mut self, item: &shell::MenuItem) {
        let action = item.action.as_deref().unwrap_or("");
        let Some(main_form_id) = self.main_form_id.clone() else {
            Self::log(format!(
                "menu action ignored: no main session (action={action} title={})",
                item.title
            ));
            return;
        };
        let Some(socket) = &self.socket else {
            return;
        };
        socket.send(
            "send_event",
            serde_json::json!({
                "command": "menuAction",
                "form_id": main_form_id,
                "action_id": item.id,
                "action": action,
                "title": item.title,
                "lua_file": item.lua_file,
                "ui_file": item.ui_file,
                "mdi": item.mdi,
                "modal": item.modal,
                "modal_scope": item.modal_scope,
            }),
        );
        Self::log(format!(
            "menuAction {:?} action={action} lua_file={:?}",
            item.title, item.lua_file
        ));
    }

    fn dispatch_mouse_press(&mut self) {
        self.cancel_mouse_grabs();
        self.left_down = true;
        self.shell_press = false;
        if self.shell.visible && !self.is_application_modal() {
            let viewport = self.floating.viewport();
            let shell_hit = self.shell.hit(self.cursor, viewport);
            let floating_hit = self
                .floating
                .hit_test(rqt::gui::QPointF::new(self.cursor.0, self.cursor.1));
            // An open menu popup is above top-level windows. Otherwise a
            // top-level window covers the main-window chrome, as in Qt.
            let popup_open = self.shell.menu_open() && shell_hit.is_some();
            if popup_open || floating_hit.is_none() {
                if shell_hit.is_some() {
                    if let Some(action) = self.shell.click(self.cursor, viewport) {
                        self.dispatch_shell_action(&action);
                    }
                    for form in self.forms.values_mut() {
                        form.tree.close_combo_popups();
                    }
                    self.shell_press = true;
                    return;
                }
            }
            if self.shell.menu_open() {
                self.shell.close_menu();
            }
        }
        self.line_select = None;
        self.caret_epoch = now_ms();
        let Some((layer, index, hit)) = self.window_hit_modal() else {
            for form in self.forms.values_mut() {
                form.tree.close_combo_popups();
            }
            self.line_select = None;
            return;
        };
        if !matches!(hit, QMdiSubWindowHit::Body) {
            for form in self.forms.values_mut() {
                form.tree.close_combo_popups();
            }
        }
        match hit {
            QMdiSubWindowHit::Title => {
                self.commit_edit();
                if let Some(index) = self.area_mut(layer).activate(index) {
                    let origin_frame = self
                        .area(layer)
                        .subwindow(index)
                        .map(|subwindow| subwindow.frame)
                        .unwrap_or_default();
                    self.dragging = Some(Drag {
                        layer,
                        index,
                        mode: MdiDragMode::Move,
                        last: self.cursor,
                        origin_frame,
                        origin_cursor: self.cursor,
                    });
                    self.active_form = self
                        .area(layer)
                        .subwindow(index)
                        .map(|subwindow| subwindow.payload.clone());
                    if let Some(form_id) = self.active_form.clone() {
                        self.raise_window(layer, &form_id);
                    }
                    self.update_resize_cursor(None);
                }
            }
            hit if hit.is_resize() => {
                self.commit_edit();
                if let Some(index) = self.area_mut(layer).activate(index) {
                    let origin_frame = self
                        .area(layer)
                        .subwindow(index)
                        .map(|subwindow| subwindow.frame)
                        .unwrap_or_default();
                    self.dragging = Some(Drag {
                        layer,
                        index,
                        mode: MdiDragMode::Resize(hit),
                        last: self.cursor,
                        origin_frame,
                        origin_cursor: self.cursor,
                    });
                    self.active_form = self
                        .area(layer)
                        .subwindow(index)
                        .map(|subwindow| subwindow.payload.clone());
                    if let Some(form_id) = self.active_form.clone() {
                        self.raise_window(layer, &form_id);
                    }
                    self.update_resize_cursor(Some(hit));
                }
            }
            QMdiSubWindowHit::Close
            | QMdiSubWindowHit::Maximize
            | QMdiSubWindowHit::Minimize
            | QMdiSubWindowHit::SystemMenu => {}
            QMdiSubWindowHit::Body => {
                let Some(index) = self.area_mut(layer).activate(index) else {
                    return;
                };
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.active_form = Some(form_id.clone());
                self.raise_window(layer, &form_id);
                self.clear_inactive_focus(&form_id);
                let Some((_, name, class)) = self.hit_test(self.cursor) else {
                    self.line_select = None;
                    return;
                };
                let point = rqt::gui::QPointF::new(self.cursor.0, self.cursor.1);
                self.scroll_click = self.forms.get(&form_id).is_some_and(|form| {
                    form.tree.combo_popup_hit(point).is_none()
                        && form.tree.scrollbar_at(form.root, point).is_some()
                });
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
            _ => {}
        }
        self.update_resize_cursor(Some(hit));
    }

    fn drag_card(&mut self, dx: f32, dy: f32) {
        let Some(drag) = self.dragging.as_mut() else {
            return;
        };
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        let layer = drag.layer;
        let index = drag.index;
        let mode = drag.mode;
        let origin_frame = drag.origin_frame;
        let origin_cursor = drag.origin_cursor;
        drag.last = self.cursor;
        match mode {
            MdiDragMode::Move => {
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                let before = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| (subwindow.frame.x, subwindow.frame.y));
                self.area_mut(layer).translate(index, dx, dy);
                let after = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| (subwindow.frame.x, subwindow.frame.y));
                if before != after {
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        form.tree.translate(form.root, dx, dy);
                    }
                    self.sync_web_text_input();
                }
            }
            MdiDragMode::Resize(hit) => {
                let Some((form_id, min, max)) =
                    self.area(layer).subwindow(index).map(|subwindow| {
                        (
                            subwindow.payload.clone(),
                            subwindow.min_content_size(),
                            subwindow.max_content_size(),
                        )
                    })
                else {
                    return;
                };
                let frame = resized_frame(
                    origin_frame,
                    hit,
                    self.cursor.0 - origin_cursor.0,
                    self.cursor.1 - origin_cursor.1,
                    min,
                    max,
                );
                self.area_mut(layer).set_frame(index, frame);
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
        let modal = self.modal_form();
        for layer in [WindowLayer::Floating, WindowLayer::Mdi] {
            for subwindow in self.area(layer).subwindows().iter().rev() {
                if let Some(modal) = modal {
                    if subwindow.payload.as_str() != modal {
                        continue;
                    }
                }
                if !subwindow.content_rect().contains(point) {
                    continue;
                }
                let Some(form) = self.forms.get(&subwindow.payload) else {
                    continue;
                };
                let widget = if let Some((popup, _)) = form.tree.combo_popup_hit(point) {
                    Some(popup)
                } else {
                    form.tree.widget_at(form.root, point)
                };
                let Some(widget) = widget else {
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
        if self.shell_press {
            self.shell_press = false;
            return;
        }
        self.update_resize_cursor(self.window_hit_modal().map(|(_, _, hit)| hit));
        self.line_select = None;
        self.commit_edit();
        if self.dragging.take().is_some() || self.column_drag.is_some() {
            self.column_drag = None;
            return;
        }

        match self.window_hit_modal() {
            Some((layer, index, QMdiSubWindowHit::Close)) => {
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.activate_form(&form_id);
                self.close_card(&form_id);
                return;
            }
            Some((layer, index, QMdiSubWindowHit::Maximize)) => {
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                if let Some(index) = self.activate_form(&form_id) {
                    self.area_mut(layer).toggle_maximized(index);
                    self.sync_form_geometry(&form_id);
                }
                return;
            }
            Some((layer, index, QMdiSubWindowHit::Minimize)) => {
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.commit_edit();
                let Some(minimized) = self.area_mut(layer).toggle_minimized(index) else {
                    return;
                };
                if minimized {
                    self.clear_form_focus(&form_id);
                } else {
                    self.sync_form_geometry(&form_id);
                    self.focus_initial_form(&form_id);
                }
                return;
            }
            Some((_, _, QMdiSubWindowHit::SystemMenu)) => return,
            Some((layer, index, QMdiSubWindowHit::Title)) => {
                if let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                {
                    self.activate_form(&form_id);
                }
                return;
            }
            Some((_, _, hit)) if hit.is_resize() => return,
            Some((layer, index, QMdiSubWindowHit::Body)) => {
                let Some(index) = self.area_mut(layer).activate(index) else {
                    return;
                };
                let Some(form_id) = self
                    .area(layer)
                    .subwindow(index)
                    .map(|subwindow| subwindow.payload.clone())
                else {
                    return;
                };
                self.raise_window(layer, &form_id);
                self.active_form = Some(form_id);
            }
            Some((_, _, _)) => return,
            None => {
                if self.modal_form().is_some() {
                    return;
                }
                self.cancel_mouse_grabs();
                if let Some(form_id) = self.active_form_id() {
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        form.tree.clear_focus();
                    }
                }
                self.focused = None;
                self.table_focus = None;
                self.sync_web_text_input();
                return;
            }
        }

        let hit = self.hit_test(self.cursor);

        let Some((form_id, widget_name, class)) = hit else {
            self.cancel_mouse_grabs();
            self.table_focus = None;
            self.table_press = None;
            return;
        };
        self.refresh_keyboard_focus(&form_id);
        if self.focused.is_some() {
            self.caret_epoch = now_ms();
            Self::log(format!("focus {:?}", self.focused));
        }

        let (protocol_event, start_cell_edit, combo_before) = {
            let Some(form) = self.forms.get_mut(&form_id) else {
                self.table_press = None;
                return;
            };
            let Some(widget) = form.tree.find_widget(&widget_name) else {
                self.table_press = None;
                return;
            };
            let combo_before = form.tree.combo_current_index(widget);
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
            (protocol_event, start_cell_edit, combo_before)
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
                "QComboBox" => {
                    let current = form.tree.combo_current_index(widget)?;
                    (Some(current) != combo_before).then(|| {
                        (
                            "CurrentIndexChanged",
                            serde_json::json!({ "index": current }),
                        )
                    })
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
            KeyEdit::Tab => {}
            KeyEdit::SelectAll => {}
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

    /// Show/hide the mobile HTML input and keep its transparent hit area over
    /// the focused `QLineEdit`. Only sends a change when the rect changes;
    /// this runs from focus/layout updates, not every frame.
    fn sync_web_text_input(&mut self) {
        let rect = self.focused.as_ref().and_then(|(form_id, name)| {
            let form = self.forms.get(form_id)?;
            let widget = form.tree.find_widget(name)?;
            form.tree.widget(widget).map(|node| node.geometry)
        });
        if self.web_input_rect == rect {
            return;
        }
        self.web_input_rect = rect;
        set_web_text_input(rect.is_some(), rect);
    }

    /// Apply messages from the hidden mobile field in arrival order.
    #[cfg(target_arch = "wasm32")]
    fn drain_web_text_input(&mut self) {
        let events: Vec<(String, String)> =
            WEB_TEXT_INPUT.with(|queue| queue.borrow_mut().drain(..).collect());
        for (kind, text) in events {
            let (action, shift) = text_input_action(&kind, &text);
            let previous_shift = self.shift;
            self.shift = shift;
            self.apply_key_action(action);
            self.shift = previous_shift;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn drain_web_text_input(&mut self) {}

    /// Keyboard half of the event loop: an open cell edit first, then
    /// typing into a selected table cell, then the focused line edit.
    fn dispatch_key(&mut self, event: &winit::event::KeyEvent) {
        if self.ctrl {
            if let winit::keyboard::Key::Character(text) = &event.logical_key {
                if text.eq_ignore_ascii_case("a") {
                    self.apply_key_action(KeyEdit::SelectAll);
                    return;
                }
            }
        }
        let action = key_edit(event.text.as_deref(), &event.logical_key);
        self.apply_key_action(action);
    }

    /// Common keyboard path for native winit events and the hidden mobile
    /// input field. The modal filter lives here so both sources obey it.
    fn apply_key_action(&mut self, action: KeyEdit) {
        if let Some(modal) = self.modal_form() {
            if self.active_form_id().as_deref() != Some(modal) {
                return;
            }
        }
        self.caret_epoch = now_ms();

        if matches!(action, KeyEdit::Tab) {
            self.tab_focus(self.shift);
            return;
        }

        if self.combo_key(&action) {
            return;
        }

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
            KeyEdit::SelectAll => {
                let mut selected = false;
                if let Some((form_id, name)) = self.focused.clone() {
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        if let Some(widget) = form.tree.find_widget(&name) {
                            form.tree.select_all(widget);
                            selected = true;
                        }
                    }
                }
                if selected {
                    self.caret_epoch = now_ms();
                }
            }
            KeyEdit::ClearFocus => {
                if self.focused.take().is_some() {
                    Self::log("focus cleared");
                }
                self.table_focus = None;
                if let Some(form_id) = self.active_form_id() {
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        form.tree.clear_focus();
                    }
                }
                self.sync_web_text_input();
            }
            KeyEdit::Commit => {
                let Some((form_id, name)) = self.focused.clone() else {
                    return;
                };
                if let Some(socket) = &self.socket {
                    socket.send(
                        "send_event",
                        serde_json::json!({
                            "command": "formEvent",
                            "form_id": form_id,
                            "widget_name": name,
                            "event": "ReturnPressed",
                        }),
                    );
                }
                Self::log(format!("returnPressed {name}"));
            }
            KeyEdit::Up
            | KeyEdit::Down
            | KeyEdit::PageUp
            | KeyEdit::PageDown
            | KeyEdit::Tab
            | KeyEdit::Ignore => {}
        }
    }

    /// Keyboard handling for `QComboBox`: arrows change the current item
    /// when closed and move the popup highlight when open; Enter commits or
    /// opens, Escape closes, Space opens (Qt behavior).
    fn combo_key(&mut self, action: &KeyEdit) -> bool {
        let Some(form_id) = self.active_form_id() else {
            return false;
        };
        let Some((widget, name, open)) = self.forms.get(&form_id).and_then(|form| {
            let widget = form.tree.focus_widget()?;
            let node = form.tree.widget(widget)?;
            (node.class_name == "QComboBox")
                .then(|| (widget, node.object_name.clone(), node.combo_popup.is_some()))
        }) else {
            return false;
        };
        let Some(form) = self.forms.get_mut(&form_id) else {
            return false;
        };
        let mut changed = None;
        match action {
            KeyEdit::ClearFocus if open => {
                form.tree.close_combo_popup(widget);
                return true;
            }
            KeyEdit::Commit => {
                if open {
                    let before = form.tree.combo_current_index(widget);
                    form.tree.commit_combo_popup(widget);
                    let after = form.tree.combo_current_index(widget);
                    if after != before {
                        changed = after;
                    }
                } else {
                    form.tree.open_combo_popup(widget);
                }
            }
            KeyEdit::Insert(text) if !text.is_empty() && text.trim().is_empty() && !open => {
                form.tree.open_combo_popup(widget);
                return true;
            }
            KeyEdit::Up
            | KeyEdit::Down
            | KeyEdit::PageUp
            | KeyEdit::PageDown
            | KeyEdit::Home
            | KeyEdit::End => {
                if open {
                    let items = form
                        .tree
                        .property(widget, "items")
                        .and_then(|value| match value {
                            rqt::core::QVariant::List(items) => Some(items.len()),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let delta = match action {
                        KeyEdit::Up => -1,
                        KeyEdit::Down => 1,
                        KeyEdit::PageUp => -10,
                        KeyEdit::PageDown => 10,
                        KeyEdit::Home => -(items as i64),
                        KeyEdit::End => items as i64,
                        _ => 0,
                    };
                    form.tree.move_combo_highlight(widget, delta);
                    return true;
                }
                let count = form
                    .tree
                    .property(widget, "items")
                    .and_then(|value| match value {
                        rqt::core::QVariant::List(items) => Some(items.len()),
                        _ => None,
                    })
                    .unwrap_or(0);
                if count == 0 {
                    return true;
                }
                let current = form.tree.combo_current_index(widget).unwrap_or(0);
                let next = match action {
                    KeyEdit::Up => current.saturating_sub(1),
                    KeyEdit::Down => (current + 1).min(count - 1),
                    KeyEdit::PageUp => current.saturating_sub(10),
                    KeyEdit::PageDown => (current + 10).min(count - 1),
                    KeyEdit::Home => 0,
                    KeyEdit::End => count - 1,
                    _ => current,
                };
                if next != current {
                    form.tree.select_combo_index(widget, next);
                    changed = Some(next);
                }
            }
            _ => return false,
        }
        if let Some(index) = changed {
            if let Some(socket) = &self.socket {
                socket.send(
                    "send_event",
                    serde_json::json!({
                        "command": "formEvent",
                        "form_id": form_id,
                        "widget_name": name,
                        "event": "CurrentIndexChanged",
                        "index": index,
                    }),
                );
            }
            Self::log(format!("event CurrentIndexChanged -> {name}: {index}"));
        }
        true
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
                    KeyEdit::Left => {
                        if tree.current_has_children() && tree.current_expanded().unwrap_or(false) {
                            tree.set_row_expanded(current, false);
                        } else {
                            tree.current_to_parent();
                        }
                        if let Some(row) = tree.current {
                            tree.ensure_visible(row, body_height);
                        }
                        tree.sync_vertical_bar(body_height);
                    }
                    KeyEdit::Right => {
                        if tree.current_has_children() && !tree.current_expanded().unwrap_or(false)
                        {
                            tree.set_row_expanded(current, true);
                        } else {
                            tree.current_to_first_child();
                        }
                        if let Some(row) = tree.current {
                            tree.ensure_visible(row, body_height);
                        }
                        tree.sync_vertical_bar(body_height);
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

    fn clear_form_focus(&mut self, form_id: &str) {
        if let Some(form) = self.forms.get_mut(form_id) {
            form.tree.clear_focus();
        }
        if self
            .focused
            .as_ref()
            .is_some_and(|(focused_form, _)| focused_form == form_id)
        {
            self.focused = None;
        }
        if self
            .table_focus
            .as_ref()
            .is_some_and(|(focused_form, _)| focused_form == form_id)
        {
            self.table_focus = None;
        }
        self.sync_web_text_input();
    }

    fn clear_inactive_focus(&mut self, active: &str) {
        for (form_id, form) in self.forms.iter_mut() {
            if form_id != active {
                form.tree.clear_focus();
                form.tree.close_combo_popups();
            }
        }
        if self
            .focused
            .as_ref()
            .is_some_and(|(form_id, _)| form_id != active)
        {
            self.focused = None;
        }
        if self
            .table_focus
            .as_ref()
            .is_some_and(|(form_id, _)| form_id != active)
        {
            self.table_focus = None;
        }
        self.sync_web_text_input();
    }

    fn focus_initial_form(&mut self, form_id: &str) {
        self.clear_inactive_focus(form_id);
        if self
            .forms
            .get(form_id)
            .and_then(|form| form.tree.focus_widget())
            .is_some()
        {
            return;
        }
        let Some(widget) = self
            .forms
            .get_mut(form_id)
            .and_then(|form| form.tree.focus_next(form.root, false))
        else {
            return;
        };
        self.sync_focus(form_id, widget, true);
    }

    fn sync_focus(&mut self, form_id: &str, widget: QWidgetId, select_text: bool) {
        let Some((class, name)) = self.forms.get(form_id).and_then(|form| {
            form.tree
                .widget(widget)
                .map(|node| (node.class_name.clone(), node.object_name.clone()))
        }) else {
            return;
        };
        self.caret_epoch = now_ms();
        match class.as_str() {
            "QLineEdit" => {
                self.focused = Some((form_id.to_string(), name.clone()));
                self.table_focus = None;
                if select_text {
                    if let Some(form) = self.forms.get_mut(form_id) {
                        if let Some(widget) = form.tree.find_widget(&name) {
                            form.tree.select_all(widget);
                        }
                    }
                }
            }
            "QTableWidget" | "QListWidget" | "QTreeWidget" | "QComboBox" => {
                self.focused = None;
                self.table_focus = Some((form_id.to_string(), name.clone()));
            }
            _ => {
                self.focused = None;
                self.table_focus = None;
            }
        }
        self.sync_web_text_input();
        Self::log(format!("focus {name} ({class})"));
    }

    fn tab_focus(&mut self, backwards: bool) {
        let Some(form_id) = self.active_form_id() else {
            return;
        };
        self.commit_edit();
        self.clear_inactive_focus(&form_id);
        let focused = self.forms.get(&form_id).and_then(|form| {
            form.tree.focus_widget().and_then(|widget| {
                form.tree
                    .widget(widget)
                    .map(|node| (widget, node.class_name.clone()))
            })
        });
        if let Some((widget, class)) = focused {
            if matches!(
                class.as_str(),
                "QTableWidget" | "QListWidget" | "QTreeWidget"
            ) && self.tab_view(&form_id, widget, &class, backwards)
            {
                return;
            }
        }
        let Some(widget) = self
            .forms
            .get_mut(&form_id)
            .and_then(|form| form.tree.focus_next(form.root, backwards))
        else {
            return;
        };
        self.sync_focus(&form_id, widget, true);
    }

    fn refresh_keyboard_focus(&mut self, form_id: &str) {
        let widget = self
            .forms
            .get(form_id)
            .and_then(|form| form.tree.focus_widget());
        if let Some(widget) = widget {
            self.sync_focus(form_id, widget, false);
        } else {
            self.focused = None;
            self.table_focus = None;
        }
    }

    fn tab_view(&mut self, form_id: &str, widget: QWidgetId, class: &str, backwards: bool) -> bool {
        let Some(form) = self.forms.get_mut(form_id) else {
            return false;
        };
        let Some(node) = form.tree.widget(widget) else {
            return false;
        };
        let rect = node.geometry;
        let font = node.font.clone();
        match class {
            "QTableWidget" => {
                let Some(table) = form.tree.table_mut(widget) else {
                    return false;
                };
                let rows = table.model.row_count();
                let columns = table.model.column_count();
                if rows == 0 || columns == 0 {
                    return false;
                }
                let Some((row, column)) = table.current() else {
                    return false;
                };
                let (row, column) = if backwards {
                    if row == 0 && column == 0 {
                        return false;
                    }
                    if column == 0 {
                        (row - 1, columns - 1)
                    } else {
                        (row, column - 1)
                    }
                } else {
                    if row + 1 >= rows && column + 1 >= columns {
                        return false;
                    }
                    if column + 1 < columns {
                        (row, column + 1)
                    } else {
                        (row + 1, 0)
                    }
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
                let Some(list) = form.tree.list_mut(widget) else {
                    return false;
                };
                let rows = list.row_count();
                if rows == 0 {
                    return false;
                }
                let Some(current) = list.current else {
                    return false;
                };
                let row = if backwards {
                    if current == 0 {
                        return false;
                    }
                    current - 1
                } else {
                    if current + 1 >= rows {
                        return false;
                    }
                    current + 1
                };
                let viewport = list.scroll_area(rect).viewport;
                list.set_current(Some(row));
                list.ensure_visible(row, viewport.height);
                list.sync_vertical_bar(viewport.height);
                true
            }
            "QTreeWidget" => {
                let Some(tree) = form.tree.tree_mut(widget) else {
                    return false;
                };
                let rows = tree.row_count();
                if rows == 0 {
                    return false;
                }
                let Some(current) = tree.current else {
                    return false;
                };
                let row = if backwards {
                    if current == 0 {
                        return false;
                    }
                    current - 1
                } else {
                    if current + 1 >= rows {
                        return false;
                    }
                    current + 1
                };
                let viewport = tree.scroll_area(rect).viewport;
                let body_height = (viewport.height - rqt::item_view::QVIEW_HEADER_H).max(0.0);
                tree.set_current(Some(row));
                tree.ensure_visible(row, body_height);
                tree.sync_vertical_bar(body_height);
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
    /// Tab — move to the next focusable widget (Shift+Tab goes back).
    Tab,
    /// Ctrl+A on the focused line edit.
    SelectAll,
    ClearFocus,
    Ignore,
}

/// Map a winit key event to a tree edit. `text` carries the printable
/// characters — on the web it also contains `"\r"` (Enter) and `"\t"`
/// (Tab); those are mapped to their own actions instead of being typed.
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
        Key::Named(NamedKey::Tab) => KeyEdit::Tab,
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

/// Map one message from the hidden web input field to a local action.
/// `shift` is encoded in `text` as `"1"` for navigation keys.
fn text_input_action(kind: &str, text: &str) -> (KeyEdit, bool) {
    let shift = text == "1";
    let action = match kind {
        "insert" => KeyEdit::Insert(text.to_string()),
        "backspace" => KeyEdit::Backspace,
        "delete" => KeyEdit::Delete,
        "left" => KeyEdit::Left,
        "right" => KeyEdit::Right,
        "up" => KeyEdit::Up,
        "down" => KeyEdit::Down,
        "pageUp" => KeyEdit::PageUp,
        "pageDown" => KeyEdit::PageDown,
        "home" => KeyEdit::Home,
        "end" => KeyEdit::End,
        "commit" => KeyEdit::Commit,
        "tab" => KeyEdit::Tab,
        "selectAll" => KeyEdit::SelectAll,
        "clearFocus" => KeyEdit::ClearFocus,
        _ => KeyEdit::Ignore,
    };
    (action, shift)
}

/// English banner for server connection failures (E1): shown over the
/// canvas whenever the websocket errors or closes without ever opening.
const CONNECTION_FAILED: &str = "Failed to connect to the server. Check that the backend is running, then reload the page.";

#[cfg(not(target_arch = "wasm32"))]
fn connection_error_banner(_show: bool, _message: &str) {}

#[cfg(target_arch = "wasm32")]
fn connection_error_banner(show: bool, message: &str) {
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let name = if show {
        "__epShowConnectionError"
    } else {
        "__epHideConnectionError"
    };
    if let Ok(function) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name)) {
        if let Ok(function) = function.dyn_into::<js_sys::Function>() {
            let _ = function.call1(&JsValue::NULL, &JsValue::from_str(message));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn install_web_text_input() {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut(String, String)>::new(|kind: String, text: String| {
        WEB_TEXT_INPUT.with(|queue| queue.borrow_mut().push((kind, text)));
    });
    let _ = js_sys::Reflect::set(
        window.as_ref(),
        &JsValue::from_str("__epTextInput"),
        callback.as_ref().unchecked_ref(),
    );
    callback.forget();

    // Events typed before the wasm module finished loading are queued by JS.
    if let Ok(flush) =
        js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__epFlushTextQueue"))
    {
        if let Ok(flush) = flush.dyn_into::<js_sys::Function>() {
            let _ = flush.call0(&JsValue::NULL);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_web_text_input(_active: bool, _rect: Option<rqt::gui::QRectF>) {}

#[cfg(target_arch = "wasm32")]
fn set_web_text_input(active: bool, rect: Option<rqt::gui::QRectF>) {
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(rect) = rect {
        if let Ok(setter) =
            js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__epSetTextInputRect"))
        {
            if let Ok(setter) = setter.dyn_into::<js_sys::Function>() {
                let _ = setter.call4(
                    &JsValue::NULL,
                    &JsValue::from_f64(rect.x as f64),
                    &JsValue::from_f64(rect.y as f64),
                    &JsValue::from_f64(rect.width as f64),
                    &JsValue::from_f64(rect.height as f64),
                );
            }
        }
    }
    if let Ok(setter) = js_sys::Reflect::get(
        window.as_ref(),
        &JsValue::from_str("__epSetTextInputActive"),
    ) {
        if let Ok(setter) = setter.dyn_into::<js_sys::Function>() {
            let _ = setter.call1(&JsValue::NULL, &JsValue::from_bool(active));
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

/// Qt's `Qt::MSWindowsFixedSizeDialogHint` (from a `.ui` `windowFlags`
/// property) disables interactive resizing, just like `QDialog`.
fn window_fixed_size(tree: &QWidgetTree, root: QWidgetId) -> bool {
    let Some(widget) = tree.widget(root) else {
        return false;
    };
    let has_hint = |value: &rqt::core::QVariant| match value {
        rqt::core::QVariant::String(flags) | rqt::core::QVariant::Enum(flags) => {
            flags.contains("MSWindowsFixedSizeDialogHint")
        }
        _ => false,
    };
    match widget.property("windowFlags") {
        Some(rqt::core::QVariant::List(values)) => values.iter().any(has_hint),
        Some(value) => has_hint(value),
        None => false,
    }
}

fn cursor_icon_for_hit(hit: Option<QMdiSubWindowHit>) -> CursorIcon {
    match hit {
        Some(QMdiSubWindowHit::ResizeLeft) => CursorIcon::WResize,
        Some(QMdiSubWindowHit::ResizeRight) => CursorIcon::EResize,
        Some(QMdiSubWindowHit::ResizeTop) => CursorIcon::NResize,
        Some(QMdiSubWindowHit::ResizeBottom) => CursorIcon::SResize,
        Some(QMdiSubWindowHit::ResizeTopLeft) => CursorIcon::NwResize,
        Some(QMdiSubWindowHit::ResizeTopRight) => CursorIcon::NeResize,
        Some(QMdiSubWindowHit::ResizeBottomLeft) => CursorIcon::SwResize,
        Some(QMdiSubWindowHit::ResizeBottomRight) => CursorIcon::SeResize,
        _ => CursorIcon::Default,
    }
}

/// New frame for an interactive border drag. The edge opposite to `hit`
/// stays anchored, and `min`/`max` are the form's *content* size limits.
fn resized_frame(
    origin: rqt::gui::QRectF,
    hit: QMdiSubWindowHit,
    dx: f32,
    dy: f32,
    min: rqt::gui::QSizeF,
    max: rqt::gui::QSizeF,
) -> rqt::gui::QRectF {
    use QMdiSubWindowHit::*;

    let min_w = min.width.max(1.0);
    let max_w = max.width.max(min_w);
    let min_h = (min.height + QMDI_TITLE_H).max(QMDI_TITLE_H);
    let max_h = (max.height + QMDI_TITLE_H).max(min_h);
    let mut frame = origin;

    let left = matches!(hit, ResizeLeft | ResizeTopLeft | ResizeBottomLeft);
    let right = matches!(hit, ResizeRight | ResizeTopRight | ResizeBottomRight);
    let top = matches!(hit, ResizeTop | ResizeTopLeft | ResizeTopRight);
    let bottom = matches!(hit, ResizeBottom | ResizeBottomLeft | ResizeBottomRight);

    if left {
        let right_edge = origin.right();
        let width = (origin.width - dx).clamp(min_w, max_w);
        frame.x = right_edge - width;
        frame.width = width;
    } else if right {
        frame.width = (origin.width + dx).clamp(min_w, max_w);
    }

    if top {
        let bottom_edge = origin.bottom();
        let height = (origin.height - dy).clamp(min_h, max_h);
        frame.y = bottom_edge - height;
        frame.height = height;
    } else if bottom {
        frame.height = (origin.height + dy).clamp(min_h, max_h);
    }

    frame
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

fn append_paint_commands(commands: &[rqt::gui::QPaintCommand], batches: &mut Vec<gfx::PaintBatch>) {
    use rqt::gui::{QPaintCommand, QRectF};

    let mut clip: Option<QRectF> = None;
    let mut stack: Vec<Option<QRectF>> = Vec::new();
    let mut rects: Vec<gfx::ColoredRect> = Vec::new();
    let mut texts: Vec<gfx::ColoredText> = Vec::new();
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
        // Keep the command stream ordered: a rect drawn after text belongs
        // in a later batch, not in a global rect pool.
        let draws_rect = matches!(
            command,
            QPaintCommand::FillRect { .. }
                | QPaintCommand::StrokeRect { .. }
                | QPaintCommand::StrokeRoundedRect { .. }
                | QPaintCommand::FillRoundedRect { .. }
                | QPaintCommand::DrawLine { .. }
                | QPaintCommand::FillLinearGradientRect { .. }
                | QPaintCommand::FillEllipse { .. }
                | QPaintCommand::StrokeEllipse { .. }
                | QPaintCommand::FillTriangle { .. }
                | QPaintCommand::DrawPixmap { .. }
        );
        if draws_rect {
            flush_text_batch(batches, &mut texts);
        } else if matches!(command, QPaintCommand::DrawText { .. }) {
            flush_rect_batch(batches, &mut rects);
        }
        match command {
            QPaintCommand::Save => stack.push(clip),
            QPaintCommand::Restore => clip = stack.pop().unwrap_or(None),
            QPaintCommand::SetClip { rect } => clip = *rect,
            QPaintCommand::Translate { .. } => {}
            QPaintCommand::FillRect { rect, color } => push_rect(&mut rects, clip, *rect, *color),
            QPaintCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => push_rounded_rect(&mut rects, clip, *rect, *radius, *color),
            QPaintCommand::FillLinearGradientRect {
                rect, start, end, ..
            } => {
                let top = rect.top().round() as i32;
                let bottom = rect.bottom().round() as i32;
                let span = (bottom - top).max(1) as f32;
                for y in top..bottom {
                    let t = ((y as f32 + 0.5 - top as f32) / span).clamp(0.0, 1.0);
                    push_rect(
                        &mut rects,
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
                                &mut rects,
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
                                &mut rects,
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
                                &mut rects,
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
                                &mut rects,
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
                push_stroke_rect(&mut rects, clip, *rect, *width, *color);
            }
            QPaintCommand::StrokeRoundedRect {
                rect, width, color, ..
            } => {
                push_stroke_rect(&mut rects, clip, *rect, *width, *color);
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
                                &mut rects,
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
    flush_rect_batch(batches, &mut rects);
    flush_text_batch(batches, &mut texts);
}

fn flush_rect_batch(batches: &mut Vec<gfx::PaintBatch>, rects: &mut Vec<gfx::ColoredRect>) {
    if rects.is_empty() {
        return;
    }
    match batches.last_mut() {
        Some(gfx::PaintBatch::Rects(previous)) => previous.append(rects),
        _ => batches.push(gfx::PaintBatch::Rects(std::mem::take(rects))),
    }
}

fn flush_text_batch(batches: &mut Vec<gfx::PaintBatch>, texts: &mut Vec<gfx::ColoredText>) {
    if texts.is_empty() {
        return;
    }
    match batches.last_mut() {
        Some(gfx::PaintBatch::Text(previous)) => previous.append(texts),
        _ => batches.push(gfx::PaintBatch::Text(std::mem::take(texts))),
    }
}

fn push_rounded_rect(
    rects: &mut Vec<gfx::ColoredRect>,
    clip: Option<rqt::gui::QRectF>,
    rect: rqt::gui::QRectF,
    radius: f32,
    color: rqt::gui::QColor,
) {
    use rqt::gui::QRectF;
    let left = rect.left().floor();
    let top = rect.top().floor();
    let right = rect.right().ceil();
    let bottom = rect.bottom().ceil();
    let width = right - left;
    let height = bottom - top;
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let radius = radius.max(0.0).min(width / 2.0).min(height / 2.0);
    if radius <= 0.0 {
        if let Some(rect) = intersect_rect(QRectF::new(left, top, width, height), clip) {
            rects.push(gfx::ColoredRect {
                x: rect.x,
                y: rect.y,
                w: rect.width,
                h: rect.height,
                color: [color.red, color.green, color.blue, color.alpha],
            });
        }
        return;
    }
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
    for y in top as i32..bottom as i32 {
        let py = y as f32 + 0.5;
        let dy = if py < top + radius {
            top + radius - py
        } else if py > bottom - radius {
            py - (bottom - radius)
        } else {
            0.0
        };
        let inset = if dy > 0.0 {
            radius - (radius * radius - dy * dy).max(0.0).sqrt()
        } else {
            0.0
        };
        let x0 = left + inset;
        let x1 = right - inset;
        if x1 > x0 {
            push(QRectF::new(x0, y as f32, x1 - x0, 1.0));
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

fn intersect_rect(
    rect: rqt::gui::QRectF,
    clip: Option<rqt::gui::QRectF>,
) -> Option<rqt::gui::QRectF> {
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

impl App {
    /// Publish the last pointer mapping to the page for on-site diagnostics.
    /// `diag=1` in the URL makes `index.html` render it as a HUD.
    #[cfg(target_arch = "wasm32")]
    fn publish_pointer_debug(&self, x: f64, y: f64, scale: f32) {
        let Some(window) = web_sys::window() else {
            return;
        };
        use wasm_bindgen::JsCast;
        let win: js_sys::Object = window.unchecked_into();
        let (surface_w, surface_h, app_scale, max_dim) = self
            .gfx
            .borrow()
            .as_ref()
            .map(|gfx| (gfx.size().0, gfx.size().1, gfx.ui_scale(), gfx.max_dim()))
            .unwrap_or((0, 0, 1.0, 0));
        let viewport = self.floating.viewport();
        let text = format!(
            "winit=({:.1}, {:.1}) pointer_scale={:.4} logical=({:.1}, {:.1}) app_surface={}x{} app_scale={:.4} gpu_max={} viewport=({:.0},{:.0},{:.0}x{:.0})",
            x,
            y,
            scale,
            x / scale as f64,
            y / scale as f64,
            surface_w,
            surface_h,
            app_scale,
            max_dim,
            viewport.x,
            viewport.y,
            viewport.width,
            viewport.height
        );
        let _ = js_sys::Reflect::set(&win, &"__epPtr".into(), &text.as_str().into());
        let view = format!(
            "{:.1},{:.1},{:.1},{:.1}",
            viewport.x, viewport.y, viewport.width, viewport.height
        );
        let _ = js_sys::Reflect::set(&win, &"__epView".into(), &view.as_str().into());
    }

    /// Reconcile backing store, surface and `ui_scale` with the DOM.
    ///
    /// The browser is the only source of truth here: at load the canvas can
    /// still carry its pre-layout size, `devicePixelRatio` can arrive after the
    /// first resize, and winit's own numbers are unit-ambiguous. Without this
    /// the renderer keeps the stale geometry from start-up until the user
    /// happens to resize the window, which shows up as the whole UI painted
    /// off-origin (and pointer hit-testing along with it).
    #[cfg(target_arch = "wasm32")]
    fn sync_canvas_geometry(&mut self) {
        let Some((css_w, css_h, dpr)) = self.dom_canvas_size() else {
            return;
        };
        let width = (css_w * dpr).round().max(1.0) as u32;
        let height = (css_h * dpr).round().max(1.0) as u32;
        let stale = self
            .gfx
            .borrow()
            .as_ref()
            .map(|gfx| (gfx.dpr() as f64 - dpr).abs() > 1e-3)
            .unwrap_or(false);
        if !stale && self.last_size.get() == Some((width, height)) {
            return;
        }
        self.last_size.set(Some((width, height)));
        // The renderer clamps the surface to the device's maximum texture size
        // (GPUs behind ANGLE/SwiftShader/VMs report 2048). Keep the canvas
        // backing store equal to what was actually configured: a backing store
        // larger than the surface makes the browser letterbox the presented
        // image, which shifts the whole UI off-origin (and only above a window
        // size that depends on the limit and the DPR).
        let mut surface = (width, height);
        if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
            gfx.set_scale_factor(dpr, width, height);
            surface = gfx.size();
        }
        self.sync_canvas_backing(surface.0, surface.1);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Pixels per CSS pixel in the incoming pointer coordinates.
    ///
    /// winit multiplies the browser's CSS offsets by `devicePixelRatio`, which
    /// is not the same as the surface scale as soon as the surface had to be
    /// clamped to the device texture limit; dividing by the wrong one maps
    /// every click `limit`-times away from its widget.
    fn pointer_scale(&self) -> f32 {
        #[cfg(target_arch = "wasm32")]
        if let Some(dpr) = self.window.as_ref().map(|window| window.scale_factor()) {
            if dpr > 0.0 {
                return dpr as f32;
            }
        }
        self.ui_scale()
    }

    /// Pointer position in canvas pixels, shared by mouse and touch input.
    fn pointer_moved(&mut self, x: f64, y: f64) {
        #[cfg(target_arch = "wasm32")]
        self.sync_canvas_geometry();
        let scale = self.pointer_scale();
        #[cfg(target_arch = "wasm32")]
        self.publish_pointer_debug(x, y, scale);
        let previous = self.dragging.as_ref().map(|drag| drag.last);
        self.cursor = (x as f32 / scale, y as f32 / scale);
        if self.shell.visible {
            self.shell.hover(self.cursor, self.floating.viewport());
        }
        self.mdi_hover = self.window_hit_modal().filter(|(_, _, hit)| {
            matches!(
                hit,
                QMdiSubWindowHit::Close
                    | QMdiSubWindowHit::Maximize
                    | QMdiSubWindowHit::Minimize
                    | QMdiSubWindowHit::SystemMenu
            )
        });
        if self.dragging.is_none() {
            let hit = self.window_hit_modal().map(|(_, _, hit)| hit);
            self.update_resize_cursor(hit);
        }
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
                        if let Some(rect) = form.tree.widget(widget).map(|widget| widget.geometry) {
                            form.tree
                                .select_line_edit_to(widget, self.cursor.0 - rect.x);
                            self.caret_epoch = now_ms();
                            return;
                        }
                    }
                }
            }
        }
        if let Some((form_id, name, column, last_x)) = self.column_drag.as_ref().map(|drag| {
            (
                drag.form_id.clone(),
                drag.name.clone(),
                drag.column,
                drag.last_x,
            )
        }) {
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

        self.update_hover(Some(rqt::gui::QPointF::new(self.cursor.0, self.cursor.1)));
    }

    /// Update `:hover` state and send Enter/Leave events. `None` releases
    /// hover after the pointer leaves the canvas or a touch ends.
    fn update_hover(&mut self, point: Option<rqt::gui::QPointF>) {
        let form_ids: Vec<String> = self.forms.keys().cloned().collect();
        for form_id in form_ids {
            let form = self.forms.get_mut(&form_id).expect("form exists");
            let new_widget = point.and_then(|point| form.tree.widget_at_point(point));
            let prev_widget = form.under_mouse_widget;
            if prev_widget == new_widget {
                continue;
            }

            if let Some(old) = prev_widget {
                let mut name = None;
                if let Some(widget) = form.tree.widget_mut(old) {
                    widget.under_mouse = false;
                    name = Some(widget.object_name.clone());
                }
                if let (Some(name), Some(socket)) = (name, &self.socket) {
                    socket.send(
                        "send_event",
                        serde_json::json!({
                            "command": "formEvent",
                            "form_id": form_id.clone(),
                            "event": "Leave",
                            "widget_name": name,
                        }),
                    );
                }
            }

            if let Some(new) = new_widget {
                let mut name = None;
                if let Some(widget) = form.tree.widget_mut(new) {
                    widget.under_mouse = true;
                    name = Some(widget.object_name.clone());
                }
                if let (Some(name), Some(socket)) = (name, &self.socket) {
                    socket.send(
                        "send_event",
                        serde_json::json!({
                            "command": "formEvent",
                            "form_id": form_id.clone(),
                            "event": "Enter",
                            "widget_name": name,
                        }),
                    );
                }
            }

            form.under_mouse_widget = new_widget;
        }
    }

    fn clear_hover(&mut self) {
        self.update_hover(None);
    }

    /// A new primary touch means any previous drag is over, even if the
    /// browser never delivered its `Ended` event (iOS sometimes does this).
    /// Called before `pointer_moved`, otherwise the stale drag would move
    /// the window to the new touch position.
    fn prepare_primary_touch(&mut self, id: u64) {
        if self
            .secondary_touches
            .iter()
            .any(|(secondary, _, _)| *secondary == id)
        {
            return;
        }
        if self.primary_touch.is_none() || self.primary_touch == Some(id) {
            self.dragging = None;
            self.column_drag = None;
            self.left_down = false;
            self.cancel_mouse_grabs();
            self.primary_touch = Some(id);
        }
    }

    /// Long-press context event for the widget under the active touch.
    fn send_context_menu(&mut self) {
        let Some((form_id, widget_name, _class)) = self.hit_test(self.cursor) else {
            return;
        };
        if let Some(socket) = &self.socket {
            socket.send(
                "send_event",
                serde_json::json!({
                    "command": "formEvent",
                    "form_id": form_id,
                    "widget_name": widget_name,
                    "event": "ContextMenu",
                }),
            );
        }
        Self::log(format!("context menu -> {widget_name}"));
    }

    fn pointer_released(&mut self) {
        let now = now_ms();
        let hit = self.hit_test(self.cursor);
        let double_tap = self
            .last_tap
            .as_ref()
            .is_some_and(|(form, widget, x, y, time)| {
                now - *time <= 350.0
                    && (self.cursor.0 - *x).abs() <= 12.0
                    && (self.cursor.1 - *y).abs() <= 12.0
                    && hit
                        .as_ref()
                        .is_some_and(|(form_id, name, _)| form_id == form && name == widget)
            });

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
            self.left_down = false;
            return;
        }
        self.dispatch_mouse_release();

        if let Some((form_id, name, class)) = &hit {
            if double_tap && class == "QLineEdit" {
                if let Some(form) = self.forms.get_mut(form_id) {
                    if let Some(widget) = form.tree.find_widget(name) {
                        if let Some(rect) = form.tree.widget(widget).map(|node| node.geometry) {
                            form.tree.select_word_at(widget, self.cursor.0 - rect.x);
                            self.caret_epoch = now_ms();
                        }
                    }
                }
            }
        }

        self.last_tap =
            hit.map(|(form_id, name, _)| (form_id, name, self.cursor.0, self.cursor.1, now));
    }
}

#[cfg(target_arch = "wasm32")]
fn self_physical(window: &Window, size: winit::dpi::PhysicalSize<u32>) -> (u32, u32) {
    // Same DOM-derived geometry as `App::physical_size`: winit's `inner_size`
    // is not consistently in CSS or device pixels, the canvas rect is.
    use winit::platform::web::WindowExtWebSys;
    if let Some(canvas) = window.canvas() {
        let rect = canvas.get_bounding_client_rect();
        let dpr = window.scale_factor();
        if rect.width() > 0.0 && rect.height() > 0.0 && dpr > 0.0 {
            return (
                (rect.width() * dpr).round().max(1.0) as u32,
                (rect.height() * dpr).round().max(1.0) as u32,
            );
        }
    }
    let dpr = if window.scale_factor() > 0.0 {
        window.scale_factor()
    } else {
        1.0
    };
    (
        ((size.width as f64) * dpr).round().max(1.0) as u32,
        ((size.height as f64) * dpr).round().max(1.0) as u32,
    )
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
                        let _ = style.set_property("position", "fixed");
                        let _ = style.set_property("top", "0");
                        let _ = style.set_property("left", "0");
                        let _ = style.set_property("width", "100%");
                        let _ = style.set_property("height", "100%");
                        let _ = body.set_attribute(
                            "style",
                            "margin:0; overflow:hidden; position:fixed; inset:0; width:100%; height:100%; overscroll-behavior:none; background:#1c1f26;",
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
            let last_size = self.last_size.clone();
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                App::log("gfx task started");
                match gfx::Gfx::new(gfx_window.clone()).await {
                    Ok(gfx_state) => {
                        *gfx.borrow_mut() = Some(gfx_state);
                        App::log("gfx ready");
                        // Apply the size from the earlier `Resized` event:
                        // `inner_size()` may still report the 1x1 canvas.
                        let (width, height) = last_size.get().unwrap_or_else(|| {
                            let size = gfx_window.inner_size();
                            self_physical(&gfx_window, size)
                        });
                        if let Some(state) = gfx.borrow_mut().as_mut() {
                            state.resize(width, height);
                        }
                        App::log(format!("gfx resized to {width}x{height}"));
                        gfx_window.request_redraw();
                    }
                    Err(err) => App::log(format!("gpu init failed: {err}")),
                }
                App::log("gfx task finished");
            });
            #[cfg(not(target_arch = "wasm32"))]
            {
                match pollster::block_on(gfx::Gfx::new(gfx_window.clone())) {
                    Ok(gfx_state) => {
                        *gfx.borrow_mut() = Some(gfx_state);
                        let (width, height) = self.last_size.get().unwrap_or_else(|| {
                            let size = gfx_window.inner_size();
                            (size.width, size.height)
                        });
                        if let Some(state) = gfx.borrow_mut().as_mut() {
                            state.resize(width, height);
                        }
                        gfx_window.request_redraw();
                    }
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
            WindowEvent::CursorLeft { .. } => {
                self.update_resize_cursor(None);
                self.clear_hover();
            }
            WindowEvent::Resized(size) => {
                // The DOM decides the geometry (see `sync_canvas_geometry`);
                // winit's size is only a fallback for a detached canvas.
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = size;
                    self.sync_canvas_geometry();
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let (width, height) = self.physical_size(size);
                    self.last_size.set(Some((width, height)));
                    self.sync_canvas_backing(width, height);
                    if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
                        gfx.resize(width, height);
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.shift = state.shift_key();
                self.ctrl = state.control_key();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = scale_factor;
                    self.sync_canvas_geometry();
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let (width, height) = self.scaled_size(scale_factor);
                    self.last_size.set(Some((width, height)));
                    self.sync_canvas_backing(width, height);
                    if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
                        gfx.set_scale_factor(scale_factor, width, height);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(target_arch = "wasm32")]
                self.sync_canvas_geometry();
                self.drain_socket_events();
                self.refresh_viewports();
                if let Some((form_id, name)) = self.focused.clone() {
                    let visible = (now_ms() - self.caret_epoch) % 1000.0 < 500.0;
                    if let Some(form) = self.forms.get_mut(&form_id) {
                        if let Some(widget) = form.tree.find_widget(&name) {
                            form.tree.set_cursor_visible(widget, visible);
                        }
                    }
                }
                let batches = self.collect_paint();
                if let Some(gfx) = self.gfx.borrow_mut().as_mut() {
                    gfx.render(
                        wgpu::Color {
                            r: 0.11,
                            g: 0.12,
                            b: 0.14,
                            a: 1.0,
                        },
                        &batches,
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.pointer_moved(position.x, position.y),
            WindowEvent::Touch(touch) => {
                // Phones/tablets: winit delivers touch contacts as `Touch` only,
                // never as `CursorMoved`/`MouseInput`. Only the first contact
                // owns the mouse pointer; extra fingers are ignored until they
                // are lifted (pinch/two-finger scroll is a separate task).
                let x = touch.location.x as f32 / self.pointer_scale();
                let y = touch.location.y as f32 / self.pointer_scale();
                let now = now_ms();
                // Some mobile browsers drop `Ended` when a drag finishes near
                // the screen edge. The next tap then arrives as a `Moved` at
                // the new position; without this guard the stale drag would
                // teleport the window there.
                let stale_jump = (self.cursor.0 - x).hypot(self.cursor.1 - y) > 24.0
                    && now - self.last_touch_ms > 120.0;
                if touch.phase == TouchPhase::Moved && self.dragging.is_some() && stale_jump {
                    self.dragging = None;
                    self.column_drag = None;
                    self.left_down = false;
                    self.cancel_mouse_grabs();
                }
                self.last_touch_ms = now;
                if let Some(index) = self
                    .secondary_touches
                    .iter()
                    .position(|(id, _, _)| *id == touch.id)
                {
                    match touch.phase {
                        TouchPhase::Moved => {
                            let (_, last_x, last_y) = self.secondary_touches[index];
                            self.secondary_touches[index] = (touch.id, x, y);
                            let dx = x - last_x;
                            let dy = y - last_y;
                            if dx != 0.0 || dy != 0.0 {
                                self.scroll_view(-dx, -dy);
                            }
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => {
                            self.secondary_touches.remove(index);
                        }
                        TouchPhase::Started => {}
                    }
                    return;
                }
                if let Some(primary) = self.primary_touch {
                    if primary != touch.id {
                        if touch.phase == TouchPhase::Started {
                            self.secondary_touches.push((touch.id, x, y));
                            // A second finger cancels tap/click/long-press and
                            // switches the sequence to scrolling.
                            self.left_down = false;
                            self.touch_start = None;
                            self.long_tap_sent = false;
                            self.cancel_mouse_grabs();
                        }
                        return;
                    }
                }
                if touch.phase == TouchPhase::Started {
                    self.prepare_primary_touch(touch.id);
                }
                self.pointer_moved(touch.location.x, touch.location.y);
                match touch.phase {
                    TouchPhase::Started => {
                        self.touch_start = Some((self.cursor.0, self.cursor.1, now_ms()));
                        self.long_tap_sent = false;
                        self.dispatch_mouse_press();
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        let long_tap = self.long_tap_sent;
                        let two_finger = !self.secondary_touches.is_empty();
                        self.primary_touch = None;
                        self.touch_start = None;
                        self.long_tap_sent = false;
                        if long_tap || two_finger {
                            // Long press / two-finger scroll: do not turn the
                            // release into a click.
                            self.left_down = false;
                            self.cancel_mouse_grabs();
                        } else {
                            self.pointer_released();
                        }
                        self.clear_hover();
                    }
                    TouchPhase::Moved => {}
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.pointer_released(),
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
        if let Some((x, y, started)) = self.touch_start {
            if !self.long_tap_sent && self.primary_touch.is_some() && now_ms() - started >= 500.0 {
                self.long_tap_sent = true;
                self.last_tap = None;
                self.cursor = (x, y);
                self.send_context_menu();
                self.cancel_mouse_grabs();
                self.left_down = false;
            }
        }
        self.sync_web_text_input();
        self.drain_web_text_input();
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
    install_web_text_input();
    run(Params::from_location());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(params: Params) {
    run(params);
}

#[cfg(test)]
mod tests {
    use super::{
        append_paint_commands, backend_with_version, base64_encode, cursor_icon_for_hit, gfx,
        icon_url, key_edit, now_ms, percent_decode, png_data_url, resized_frame, shell,
        text_input_action, App, AppChrome, AppChromeAction, Drag, KeyEdit, MdiDragMode, Params,
        QtForm, WindowLayer, QMDI_TITLE_H,
    };
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
        QtForm {
            tree,
            root,
            under_mouse_widget: None,
        }
    }

    #[test]
    fn app_property_applies_browser_chrome_and_keeps_desktop_sugar_as_noop() {
        use serde_json::json;

        let mut app = App::new(Params::default());

        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "windowTitle",
            "value": "Измерения"
        }));
        assert_eq!(app.chrome.window_title.as_deref(), Some("Измерения"));

        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "windowIcon",
            "value": "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>"
        }));
        assert_eq!(
            app.chrome.window_icon.as_deref(),
            Some("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>")
        );

        // Legacy Qt icon specs are sliced from the bundled sprite.
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "windowIcon",
            "value": "img24:45"
        }));
        assert_eq!(app.chrome.window_icon.as_deref(), Some("img24:45"));

        let mut chrome = AppChrome::default();
        match chrome.apply("windowIcon", &form_xml::PropValue::Str("img24:45".into())) {
            AppChromeAction::IconPng(png) => assert!(!png.is_empty()),
            other => panic!("expected IconPng, got {other:?}"),
        }

        // Desktop-only sugar must be accepted but not disturb browser chrome.
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "geometry",
            "value": {"x": 0, "y": 0, "w": 640, "h": 480}
        }));
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "windowState",
            "value": "maximized"
        }));
        assert_eq!(app.chrome.window_title.as_deref(), Some("Измерения"));

        assert_eq!(icon_url("img24:45"), None);
        assert_eq!(icon_url(":/icons/icon_001.svg"), None);
        assert_eq!(icon_url("icons/app.png"), Some("icons/app.png"));
        assert_eq!(
            icon_url("https://example.com/favicon.ico"),
            Some("https://example.com/favicon.ico")
        );
        assert_eq!(icon_url("ok"), None);
    }

    #[test]
    fn resized_frame_anchors_opposite_edges_and_clamps() {
        use rqt::mdi::QMdiSubWindowHit;
        let origin = rqt::gui::QRectF::new(100.0, 80.0, 300.0, 224.0);

        let frame = resized_frame(
            origin,
            QMdiSubWindowHit::ResizeBottomRight,
            50.0,
            30.0,
            rqt::gui::QSizeF::new(100.0, 80.0),
            rqt::gui::QSizeF::new(400.0, 300.0),
        );
        assert_eq!(
            (frame.x, frame.y, frame.width, frame.height),
            (100.0, 80.0, 350.0, 254.0)
        );

        let frame = resized_frame(
            origin,
            QMdiSubWindowHit::ResizeLeft,
            -40.0,
            0.0,
            rqt::gui::QSizeF::new(100.0, 80.0),
            rqt::gui::QSizeF::new(400.0, 300.0),
        );
        assert_eq!((frame.x, frame.width), (60.0, 340.0));
        assert_eq!(frame.right(), origin.right());

        let frame = resized_frame(
            origin,
            QMdiSubWindowHit::ResizeTop,
            0.0,
            20.0,
            rqt::gui::QSizeF::new(100.0, 80.0),
            rqt::gui::QSizeF::new(400.0, 300.0),
        );
        assert_eq!((frame.y, frame.height), (100.0, 204.0));
        assert_eq!(frame.bottom(), origin.bottom());

        let frame = resized_frame(
            origin,
            QMdiSubWindowHit::ResizeBottomRight,
            1000.0,
            1000.0,
            rqt::gui::QSizeF::new(100.0, 80.0),
            rqt::gui::QSizeF::new(400.0, 300.0),
        );
        assert_eq!((frame.width, frame.height), (400.0, 300.0 + QMDI_TITLE_H));

        let frame = resized_frame(
            origin,
            QMdiSubWindowHit::ResizeLeft,
            1000.0,
            0.0,
            rqt::gui::QSizeF::new(100.0, 80.0),
            rqt::gui::QSizeF::new(400.0, 300.0),
        );
        assert_eq!((frame.x, frame.width), (300.0, 100.0));
    }

    #[test]
    fn resize_cursor_follows_the_border_direction() {
        use rqt::mdi::QMdiSubWindowHit;
        use winit::window::CursorIcon;

        assert_eq!(
            cursor_icon_for_hit(Some(QMdiSubWindowHit::ResizeLeft)),
            CursorIcon::WResize
        );
        assert_eq!(
            cursor_icon_for_hit(Some(QMdiSubWindowHit::ResizeTopLeft)),
            CursorIcon::NwResize
        );
        assert_eq!(
            cursor_icon_for_hit(Some(QMdiSubWindowHit::ResizeBottomRight)),
            CursorIcon::SeResize
        );
        assert_eq!(cursor_icon_for_hit(None), CursorIcon::Default);
        assert_eq!(
            cursor_icon_for_hit(Some(QMdiSubWindowHit::Body)),
            CursorIcon::Default
        );
    }

    #[test]
    fn form_resize_flag_and_constraints_follow_ui_and_commands() {
        use rqt::mdi::QMdiSubWindowHit;
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property>
              <property name="windowTitle"><string>Resize</string></property>
              <property name="minimumSize"><size><width>200</width><height>150</height></size></property>
              <property name="maximumSize"><size><width>400</width><height>300</height></size></property>
            </widget></ui>"#;
        let mut app = App::new(Params::default());
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "resize",
            "ui_xml": xml,
            "modal": false
        }));
        let index = app
            .mdi
            .index_of(|payload| payload == "resize")
            .expect("subwindow");
        assert_eq!(
            app.mdi.subwindow(index).unwrap().content_rect().size(),
            rqt::gui::QSizeF::new(300.0, 200.0)
        );
        assert!(app.mdi.subwindow(index).unwrap().can_resize());

        let frame = app.mdi.subwindow(index).unwrap().frame;
        assert_eq!(
            app.mdi_hit((frame.x + 1.0, frame.y + 100.0))
                .map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::ResizeLeft)
        );

        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "resize",
            "property": "minimumSize",
            "value": {"w": 350, "h": 250}
        }));
        let frame = app.mdi.subwindow(index).unwrap().frame;
        assert_eq!((frame.width, frame.height), (350.0, 250.0 + QMDI_TITLE_H));
        assert_eq!(
            app.forms["resize"]
                .tree
                .widget(app.forms["resize"].root)
                .unwrap()
                .geometry
                .size(),
            rqt::gui::QSizeF::new(350.0, 250.0)
        );

        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "resize",
            "property": "resizable",
            "value": false
        }));
        assert!(!app.mdi.subwindow(index).unwrap().can_resize());
        assert_eq!(
            app.mdi_hit((frame.x + 1.0, frame.y + 100.0))
                .map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Body)
        );

        let fixed_xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property>
              <property name="windowFlags"><set><enum>Qt::WindowType::MSWindowsFixedSizeDialogHint</enum></set></property>
            </widget></ui>"#;
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "fixed",
            "ui_xml": fixed_xml,
            "modal": false
        }));
        let index = app
            .mdi
            .index_of(|payload| payload == "fixed")
            .expect("fixed subwindow");
        assert!(!app.mdi.subwindow(index).unwrap().can_resize());
        assert!(!app.mdi.subwindow(index).unwrap().window_flags.maximize);
    }

    #[test]
    fn standalone_form_uses_top_level_layer_and_full_form_stays_mdi() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>40</x><y>40</y><width>300</width><height>200</height></rect></property>
              <property name="windowTitle"><string>Standalone</string></property>
            </widget></ui>"#;
        let mut app = App::new(Params::default());
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "standalone",
            "mode": "standalone",
            "placement": "top-level",
            "ui_xml": xml,
        }));
        assert!(app.mdi.is_empty());
        assert_eq!(app.floating.len(), 1);
        let index = app
            .floating
            .index_of(|payload| payload == "standalone")
            .unwrap();
        assert_eq!(
            app.floating.subwindow(index).unwrap().content_rect(),
            QRectF::new(40.0, 40.0, 300.0, 200.0)
        );

        let mut full = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        full.on_command(&json!({
            "command": "createForm",
            "form_id": "mdi",
            "mode": "full",
            "placement": "mdi",
            "ui_xml": xml,
        }));
        assert!(full.floating.is_empty());
        assert_eq!(full.mdi.len(), 1);
        assert!(full.shell.visible);
    }

    fn cascade_test_ui(x: i64, y: i64) -> String {
        format!(
            r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>{x}</x><y>{y}</y><width>300</width><height>200</height></rect></property></widget></ui>"#
        )
    }

    #[test]
    fn forms_without_position_cascade_and_explicit_geometry_stays() {
        use serde_json::json;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        app.floating.set_viewport(viewport);
        app.mdi.set_viewport(viewport);

        // Two MDI forms without any explicit position get cascade slots:
        // step (10, title height), so every title stays visible.
        for id in ["m1", "m2"] {
            app.on_command(&json!({
                "command": "createForm",
                "form_id": id,
                "mode": "full",
                "placement": "mdi",
                "ui_xml": cascade_test_ui(0, 0),
            }));
        }
        let mdi_frame = |app: &App, id: &str| {
            let index = app.mdi.index_of(|payload| payload == id).unwrap();
            app.mdi.subwindow(index).unwrap().frame
        };
        let first = mdi_frame(&app, "m1");
        let second = mdi_frame(&app, "m2");
        assert_eq!((first.x, first.y), (32.0, 28.0));
        assert_eq!((second.x - first.x, second.y - first.y), (10.0, 24.0));

        // An explicit position is honored as-is (coinciding explicit
        // positions are intentional and never shifted).
        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "m2",
            "property": "geometry",
            "value": { "x": 100, "y": 100, "w": 300, "h": 200 },
        }));
        let index = app.mdi.index_of(|payload| payload == "m2").unwrap();
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        assert_eq!((content.x, content.y), (100.0, 100.0));

        // Top-level forms without a designer position cascade as well…
        for id in ["t1", "t2"] {
            app.on_command(&json!({
                "command": "createForm",
                "form_id": id,
                "mode": "standalone",
                "placement": "top-level",
                "ui_xml": cascade_test_ui(0, 0),
            }));
        }
        // …while a designer position is explicit and stays put.
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "t3",
            "mode": "standalone",
            "placement": "top-level",
            "ui_xml": cascade_test_ui(120, 80),
        }));
        let floating_frame = |app: &App, id: &str| {
            let index = app.floating.index_of(|payload| payload == id).unwrap();
            app.floating.subwindow(index).unwrap().frame
        };
        let t1 = floating_frame(&app, "t1");
        let t2 = floating_frame(&app, "t2");
        let t3 = floating_frame(&app, "t3");
        assert_eq!((t1.x, t1.y), (32.0, 28.0));
        assert_eq!((t2.x - t1.x, t2.y - t1.y), (10.0, 24.0));
        assert_eq!((t3.x, t3.y), (120.0, 80.0 - rqt::mdi::QMDI_TITLE_H));
    }

    #[test]
    fn cascade_and_tile_app_commands_rearrange_windows() {
        use serde_json::json;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        app.mdi.set_viewport(viewport);
        for id in ["a", "b", "c"] {
            app.on_command(&json!({
                "command": "createForm",
                "form_id": id,
                "mode": "full",
                "placement": "mdi",
                "ui_xml": cascade_test_ui(0, 0),
            }));
        }
        let frames = |app: &App| {
            ["a", "b", "c"]
                .into_iter()
                .map(|id| {
                    let index = app.mdi.index_of(|payload| payload == id).unwrap();
                    app.mdi.subwindow(index).unwrap().frame
                })
                .collect::<Vec<_>>()
        };

        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "cascadeWindows",
            "value": true,
        }));
        let cascaded = frames(&app);
        assert_eq!((cascaded[0].x, cascaded[0].y), (32.0, 28.0));
        assert_eq!(
            (cascaded[1].x - cascaded[0].x, cascaded[1].y - cascaded[0].y),
            (10.0, 24.0)
        );
        assert_eq!(
            (cascaded[2].x - cascaded[1].x, cascaded[2].y - cascaded[1].y),
            (10.0, 24.0)
        );

        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "tileWindows",
            "value": true,
        }));
        let tiled = frames(&app);
        // 3 windows → 2 columns × 2 rows over the 800x600 viewport.
        assert_eq!(tiled[0], QRectF::new(0.0, 0.0, 400.0, 300.0));
        assert_eq!(tiled[1], QRectF::new(400.0, 0.0, 400.0, 300.0));
        assert_eq!(tiled[2], QRectF::new(0.0, 300.0, 400.0, 300.0));
    }

    #[test]
    fn mdi_form_position_is_relative_to_the_central_area() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property>
            </widget></ui>"#;
        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        app.floating.set_viewport(viewport);
        app.shell
            .set_menu(&form_xml::PropValue::List(vec![form_xml::PropValue::Map(
                vec![("title".into(), form_xml::PropValue::Str("File".into()))],
            )]));
        app.shell.toolbar_visible = true;
        app.shell.toolbar = vec![shell::MenuItem {
            title: "Tool".into(),
            ..Default::default()
        }];
        app.mdi.set_viewport(app.shell.central(viewport));
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "f",
            "mode": "full",
            "placement": "mdi",
            "ui_xml": xml,
        }));
        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "f",
            "property": "position",
            "value": { "x": 40, "y": 40 }
        }));
        let index = app.mdi.index_of(|payload| payload == "f").unwrap();
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        let central = app.mdi.viewport();
        assert_eq!(content.x, central.x + 40.0);
        assert_eq!(content.y, central.y + 40.0);

        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "f",
            "property": "size",
            "value": { "w": 320, "h": 200 }
        }));
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        assert_eq!(content.x, central.x + 40.0);
        assert_eq!(content.y, central.y + 40.0);
        assert_eq!(content.size(), rqt::gui::QSizeF::new(320.0, 200.0));
    }

    #[test]
    fn centered_form_property_centers_in_mdi_viewport() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>7</x><y>9</y><width>300</width><height>200</height></rect></property>
            </widget></ui>"#;
        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.mdi.set_viewport(QRectF::new(10.0, 20.0, 800.0, 600.0));
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "f",
            "mode": "full",
            "placement": "mdi",
            "ui_xml": xml,
        }));
        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "f",
            "property": "centered",
            "value": true,
        }));
        let index = app.mdi.index_of(|payload| payload == "f").unwrap();
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        let viewport = app.mdi.viewport();
        let dx = (content.x + content.width / 2.0) - (viewport.x + viewport.width / 2.0);
        let dy = (content.y + content.height / 2.0) - (viewport.y + viewport.height / 2.0);
        assert!(dx.abs() <= 0.5, "x off by {dx}");
        assert!(dy.abs() <= 0.5, "y off by {dy}");
        assert_eq!(content.size(), rqt::gui::QSizeF::new(300.0, 200.0));
    }

    #[test]
    fn load_main_window_command_applies_static_shell() {
        use serde_json::json;

        const MAIN_WINDOW: &str = include_str!("../../../../server/priv/qt_forms/mainwindow.ui");

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            main_ui: Some("mainwindow.ui".into()),
            ..Params::default()
        });
        app.on_command(&json!({
            "command": "loadMainWindow",
            "role": "main",
            "ui_xml": MAIN_WINDOW
        }));
        assert_eq!(app.shell.menu.len(), 2);
        assert_eq!(app.shell.toolbar.len(), 3);
        assert!(app.shell.status_visible);

        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "menu",
            "value": [{ "title": "Dynamic" }]
        }));
        assert_eq!(app.shell.menu.len(), 1);
        assert_eq!(app.shell.menu[0].title, "Dynamic");
    }

    #[test]
    fn app_shell_parses_menu_toolbar_and_status_properties() {
        use serde_json::json;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "menu",
            "value": [
                { "title": "File", "children": [
                    { "title": "Exit", "action": "exit" }
                ] }
            ]
        }));
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "toolBar",
            "value": [
                { "title": "Refresh", "action": "open_lua",
                  "lua_file": "test_form_move.lua", "mdi": true,
                  "icon": "img24:41" }
            ]
        }));
        app.on_command(&json!({
            "command": "setAppProperty",
            "property": "statusMessage",
            "value": { "text": "Ready", "timeout": 1000 }
        }));
        assert_eq!(app.shell.menu.len(), 1);
        assert_eq!(app.shell.toolbar.len(), 1);
        assert_eq!(app.shell.status_text(now_ms()), Some("Ready"));

        let viewport = QRectF::new(0.0, 0.0, 800.0, 600.0);
        let central = app.shell.central(viewport);
        assert!(central.y > viewport.y);
        assert!(central.height < viewport.height);
    }

    #[test]
    fn create_form_clamps_initial_geometry_to_minimum_size() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>0</x><y>0</y><width>120</width><height>80</height></rect></property>
              <property name="minimumSize"><size><width>300</width><height>220</height></size></property>
            </widget></ui>"#;
        let mut app = App::new(Params::default());
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "clamped",
            "ui_xml": xml,
            "modal": false
        }));
        let index = app
            .mdi
            .index_of(|payload| payload == "clamped")
            .expect("subwindow");
        assert_eq!(
            app.mdi.subwindow(index).unwrap().content_rect().size(),
            rqt::gui::QSizeF::new(300.0, 220.0)
        );
        assert_eq!(
            app.forms["clamped"]
                .tree
                .widget(app.forms["clamped"].root)
                .unwrap()
                .geometry
                .size(),
            rqt::gui::QSizeF::new(300.0, 220.0)
        );
    }

    #[test]
    fn shrinking_form_stops_at_root_layout_minimum() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>OverflowDialog</class>
            <widget class="QDialog" name="OverflowDialog">
              <property name="geometry"><rect><x>0</x><y>0</y><width>360</width><height>240</height></rect></property>
              <property name="minimumSize"><size><width>80</width><height>64</height></size></property>
              <layout class="QVBoxLayout" name="verticalLayout">
                <item><widget class="QPushButton" name="first">
                  <property name="text"><string>First</string></property>
                  <property name="minimumSize"><size><width>80</width><height>50</height></size></property>
                </widget></item>
                <item><widget class="QPushButton" name="second">
                  <property name="text"><string>Second</string></property>
                  <property name="minimumSize"><size><width>80</width><height>50</height></size></property>
                </widget></item>
              </layout>
            </widget></ui>"#;
        let mut app = App::new(Params::default());
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "overflow",
            "ui_xml": xml,
            "modal": false
        }));
        let index = app
            .mdi
            .index_of(|payload| payload == "overflow")
            .expect("subwindow");
        let expected = {
            let form = app.forms.get_mut("overflow").unwrap();
            let root = form.root;
            form.tree.minimum_size_hint(root)
        };
        assert_eq!(expected, rqt::gui::QSizeF::new(102.0, 128.0));

        // Ask for less than the layout needs: the frame must stop at the
        // layout minimum instead of letting the button row escape.
        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "overflow",
            "property": "size",
            "value": { "w": 80, "h": 64 }
        }));
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        assert_eq!(content.size(), expected);

        let second_rect = {
            let form = &app.forms["overflow"];
            let second = form.tree.find_widget("second").expect("second");
            form.tree.widget(second).unwrap().geometry
        };
        assert!(content.contains(second_rect.top_left()));
        assert!(content.contains(rqt::gui::QPointF::new(
            second_rect.right() - 0.5,
            second_rect.bottom() - 0.5
        )));

        let point = rqt::gui::QPointF::new(second_rect.x + 5.0, second_rect.y + 5.0);
        let hit = app.hit_test((point.x, point.y)).expect("button hit");
        assert_eq!(hit.1, "second");

        // Below the content rect there is no form child to hit.
        let outside = rqt::gui::QPointF::new(content.x + 5.0, content.bottom() + 2.0);
        assert_eq!(app.hit_test((outside.x, outside.y)), None);
    }

    #[test]
    fn form_position_and_size_keep_the_other_dimension() {
        use serde_json::json;

        let mut app = App::new(Params::default());
        app.forms.insert("a".into(), card_form(32.0, 54.0, "One"));
        let index = app
            .mdi
            .add_subwindow("One", "a".into(), rqt::gui::QSizeF::new(300.0, 200.0));
        let before = app.forms["a"]
            .tree
            .widget(app.forms["a"].root)
            .unwrap()
            .geometry;

        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "a",
            "property": "position",
            "value": {"x": 200, "y": 150}
        }));
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        assert_eq!((content.x, content.y), (200.0, 150.0));
        let geometry = app.forms["a"]
            .tree
            .widget(app.forms["a"].root)
            .unwrap()
            .geometry;
        assert_eq!((geometry.x, geometry.y), (200.0, 150.0));
        assert_eq!(
            (geometry.width, geometry.height),
            (before.width, before.height)
        );

        app.on_command(&json!({
            "command": "setFormProperty",
            "form_id": "a",
            "property": "size",
            "value": {"w": 420, "h": 210}
        }));
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        assert_eq!(
            (content.x, content.y, content.width, content.height),
            (200.0, 150.0, 420.0, 210.0)
        );
        let geometry = app.forms["a"]
            .tree
            .widget(app.forms["a"].root)
            .unwrap()
            .geometry;
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (200.0, 150.0, 420.0, 210.0)
        );
    }

    #[test]
    fn png_data_url_is_base64_and_marked_as_png() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");

        let png = rqt::icons::png_bytes("img24:22").expect("sprite cell");
        let url = png_data_url(&png);
        assert!(url.starts_with("data:image/png;base64,iVBOR"), "{url}");
    }

    #[test]
    fn activating_mdi_window_raises_it_above_floating() {
        let mut app = App::new(Params::default());
        app.forms.insert("mdi".into(), card_form(32.0, 54.0, "Mdi"));
        app.mdi
            .add_subwindow("Mdi", "mdi".into(), rqt::gui::QSizeF::new(300.0, 200.0));
        app.forms
            .insert("float".into(), card_form(40.0, 60.0, "Float"));
        app.floating
            .add_subwindow("Float", "float".into(), rqt::gui::QSizeF::new(300.0, 200.0));
        app.window_z = vec![
            (WindowLayer::Mdi, "mdi".into()),
            (WindowLayer::Floating, "float".into()),
        ];

        app.activate_form("mdi");

        assert_eq!(
            app.window_z.last(),
            Some(&(WindowLayer::Mdi, "mdi".to_string()))
        );
        assert_eq!(
            app.window_order().last(),
            Some(&(
                WindowLayer::Mdi,
                app.mdi.index_of(|id| id == "mdi").unwrap()
            ))
        );
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
            app.mdi_hit((100.0, 40.0)).map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Title)
        );
        assert_eq!(
            app.mdi_hit((315.0, 32.0)).map(|(_, hit)| hit),
            Some(QMdiSubWindowHit::Close)
        );
        // Second window sits one cascade step away: (42, 52).
        assert_eq!(
            app.mdi_hit((325.0, 60.0)).map(|(_, hit)| hit),
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

    fn batch_texts(batches: &[gfx::PaintBatch]) -> Vec<String> {
        batches
            .iter()
            .flat_map(|batch| match batch {
                gfx::PaintBatch::Rects(_) => Vec::new(),
                gfx::PaintBatch::Text(texts) => {
                    texts.iter().map(|text| text.text.clone()).collect()
                }
            })
            .collect()
    }

    fn text_batch_index(batches: &[gfx::PaintBatch], needle: &str) -> Option<usize> {
        batches.iter().position(|batch| match batch {
            gfx::PaintBatch::Rects(_) => false,
            gfx::PaintBatch::Text(texts) => texts.iter().any(|text| text.text == needle),
        })
    }

    fn rasterize_batches(
        batches: &[gfx::PaintBatch],
        width: u32,
        height: u32,
    ) -> rqt::raster::QRasterImage {
        use rqt::gui::{QColor, QFont, QPaintCommand, QRectF};

        let scalar = |color: [f32; 4]| QColor::rgba(color[0], color[1], color[2], color[3]);
        let mut commands = Vec::new();
        for batch in batches {
            match batch {
                gfx::PaintBatch::Rects(rects) => {
                    for rect in rects {
                        commands.push(QPaintCommand::FillRect {
                            rect: QRectF::new(rect.x, rect.y, rect.w, rect.h),
                            color: scalar(rect.color),
                        });
                    }
                }
                gfx::PaintBatch::Text(texts) => {
                    for text in texts {
                        let [left, top, right, bottom] = text.clip;
                        commands.push(QPaintCommand::Save);
                        commands.push(QPaintCommand::SetClip {
                            rect: Some(QRectF::new(
                                left as f32,
                                top as f32,
                                (right - left) as f32,
                                (bottom - top) as f32,
                            )),
                        });
                        commands.push(QPaintCommand::DrawText {
                            rect: QRectF::new(
                                text.x,
                                text.y,
                                text.wrap_w.max(1.0),
                                gfx::LINE_HEIGHT.max(1.0),
                            ),
                            text: text.text.clone(),
                            font: QFont::default(),
                            color: scalar(text.color),
                        });
                        commands.push(QPaintCommand::Restore);
                    }
                }
            }
        }
        rqt::raster::rasterize(&commands, width, height, QColor::rgb(0, 0, 0))
    }

    fn compare_shell_raster(app: &App, name: &str, reference: &[u8], width: u32, height: u32) {
        let reference = rqt::raster::QRasterImage::from_png_bytes(reference)
            .unwrap_or_else(|error| panic!("{name} golden png: {error}"));
        let image = rasterize_batches(&app.collect_paint(), width, height);
        let _ = image.save_png(format!("/tmp/app_{name}.png"));
        assert_eq!(image.width, reference.width, "{name} width");
        assert_eq!(image.height, reference.height, "{name} height");

        let mut total = 0u64;
        let mut count = 0u64;
        let mut different = 0u64;
        for (left, right) in image
            .pixels
            .chunks_exact(4)
            .zip(reference.pixels.chunks_exact(4))
        {
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
        let mean = total as f64 / count.max(1) as f64;
        eprintln!(
            "{name}: mean channel diff {mean:.2}, pixels >30: {different} / {}",
            width * height
        );
        assert!(mean < 2.0, "{name}: mean diff {mean}");
        assert!(
            different < (width * height) as u64 / 50,
            "{name}: pixels >30 {different}"
        );
    }

    fn shell_popup_app() -> App {
        use form_xml::PropValue;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.shell
            .set_menu(&PropValue::List(vec![PropValue::Map(vec![
                ("title".into(), PropValue::Str("File".into())),
                (
                    "children".into(),
                    PropValue::List(vec![
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("Open".into())),
                            ("action".into(), PropValue::Str("open_lua".into())),
                        ]),
                        PropValue::Map(vec![
                            ("title".into(), PropValue::Str("Save".into())),
                            ("action".into(), PropValue::Str("save".into())),
                        ]),
                    ]),
                ),
            ])]));
        app.shell.toolbar = vec![
            shell::MenuItem {
                title: "ToolbarLabel".into(),
                ..Default::default()
            },
            shell::MenuItem {
                separator: true,
                ..Default::default()
            },
            shell::MenuItem {
                title: "ToolbarTail".into(),
                ..Default::default()
            },
        ];
        app.shell.toolbar_visible = true;
        let viewport = QRectF::new(0.0, 0.0, 480.0, 240.0);
        app.floating.set_viewport(viewport);
        app.mdi.set_viewport(app.shell.central(viewport));
        app.shell.open_path = Some(vec![0]);
        app
    }

    fn shell_nested_app() -> App {
        use form_xml::PropValue;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.shell
            .set_menu(&PropValue::List(vec![PropValue::Map(vec![
                ("title".into(), PropValue::Str("View".into())),
                (
                    "children".into(),
                    PropValue::List(vec![PropValue::Map(vec![
                        ("title".into(), PropValue::Str("Submenu".into())),
                        (
                            "children".into(),
                            PropValue::List(vec![PropValue::Map(vec![
                                ("title".into(), PropValue::Str("Nested".into())),
                                ("action".into(), PropValue::Str("open_lua".into())),
                            ])]),
                        ),
                    ])]),
                ),
            ])]));
        let viewport = QRectF::new(0.0, 0.0, 480.0, 260.0);
        app.floating.set_viewport(viewport);
        app.mdi.set_viewport(app.shell.central(viewport));
        app.shell.open_path = Some(vec![0, 0]);
        app
    }

    fn shell_windows_app() -> App {
        let mut app = App::new(Params::default());
        app.forms
            .insert("mdi".into(), card_form(32.0, 54.0, "MdiWindow"));
        app.mdi.add_subwindow(
            "MdiWindow",
            "mdi".into(),
            rqt::gui::QSizeF::new(300.0, 200.0),
        );
        app.forms
            .insert("float".into(), card_form(80.0, 90.0, "FloatingWindow"));
        app.floating.add_subwindow(
            "FloatingWindow",
            "float".into(),
            rqt::gui::QSizeF::new(300.0, 200.0),
        );
        let viewport = QRectF::new(0.0, 0.0, 520.0, 360.0);
        app.floating.set_viewport(viewport);
        app.mdi.set_viewport(viewport);
        app
    }

    #[test]
    #[ignore = "regenerate committed shell raster goldens"]
    fn regenerate_shell_goldens() {
        let fixtures = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
        for (name, app, width, height) in [
            ("shell_popup_toolbar", shell_popup_app(), 480, 240),
            ("shell_nested_menu", shell_nested_app(), 480, 260),
            ("shell_overlapping_windows", shell_windows_app(), 520, 360),
        ] {
            let image = rasterize_batches(&app.collect_paint(), width, height);
            image
                .save_png(format!("{fixtures}/{name}.png"))
                .unwrap_or_else(|error| panic!("write {name}: {error}"));
        }
    }

    #[test]
    fn shell_popup_over_toolbar_raster_matches_golden() {
        compare_shell_raster(
            &shell_popup_app(),
            "shell_popup_toolbar",
            include_bytes!("../tests/fixtures/shell_popup_toolbar.png"),
            480,
            240,
        );
    }

    #[test]
    fn shell_nested_menu_raster_matches_golden() {
        compare_shell_raster(
            &shell_nested_app(),
            "shell_nested_menu",
            include_bytes!("../tests/fixtures/shell_nested_menu.png"),
            480,
            260,
        );
    }

    #[test]
    fn shell_overlapping_windows_raster_matches_golden() {
        compare_shell_raster(
            &shell_windows_app(),
            "shell_overlapping_windows",
            include_bytes!("../tests/fixtures/shell_overlapping_windows.png"),
            520,
            360,
        );
    }

    #[test]
    fn paint_command_stream_keeps_rect_text_rect_text_order() {
        use rqt::gui::{QColor, QPainter, QRectF};

        let mut painter = QPainter::new();
        painter.fill_rect(
            QRectF::new(0.0, 0.0, 10.0, 10.0),
            QColor::rgba8(10, 10, 10, 255),
        );
        painter.draw_text(
            QRectF::new(0.0, 0.0, 10.0, 10.0),
            "first",
            QColor::rgba8(255, 255, 255, 255),
        );
        painter.fill_rect(
            QRectF::new(0.0, 12.0, 10.0, 10.0),
            QColor::rgba8(20, 20, 20, 255),
        );
        painter.draw_text(
            QRectF::new(0.0, 12.0, 10.0, 10.0),
            "second",
            QColor::rgba8(255, 255, 255, 255),
        );

        let mut batches = Vec::new();
        append_paint_commands(painter.commands(), &mut batches);

        assert_eq!(batches.len(), 4);
        assert!(matches!(batches[0], gfx::PaintBatch::Rects(_)));
        assert!(matches!(batches[1], gfx::PaintBatch::Text(_)));
        assert!(matches!(batches[2], gfx::PaintBatch::Rects(_)));
        assert!(matches!(batches[3], gfx::PaintBatch::Text(_)));
        assert_eq!(batch_texts(&batches), vec!["first", "second"]);
    }

    #[test]
    fn popup_background_is_batched_after_base_text_without_masking() {
        use form_xml::PropValue;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.shell
            .set_menu(&PropValue::List(vec![PropValue::Map(vec![
                ("title".into(), PropValue::Str("File".into())),
                (
                    "children".into(),
                    PropValue::List(vec![PropValue::Map(vec![
                        ("title".into(), PropValue::Str("Open".into())),
                        ("action".into(), PropValue::Str("open_lua".into())),
                    ])]),
                ),
            ])]));
        app.shell.toolbar = vec![shell::MenuItem {
            title: "ToolbarLabel".into(),
            ..Default::default()
        }];
        app.shell.toolbar_visible = true;

        let viewport = app.floating.viewport();
        assert!(app.shell.click((25.0, 10.0), viewport).is_none());
        assert_eq!(app.shell.open_path, Some(vec![0]));

        let batches = app.collect_paint();
        let toolbar = text_batch_index(&batches, "ToolbarLabel")
            .expect("toolbar text must not be masked under the popup");
        let popup = text_batch_index(&batches, "Open").expect("popup item text");
        assert!(toolbar < popup, "base text must precede popup text");

        // An opaque popup background must be replayed between the toolbar
        // text and the popup item's own text.
        let popup_bg = batches
            .iter()
            .enumerate()
            .skip(toolbar + 1)
            .take(popup.saturating_sub(toolbar + 1))
            .any(|(_, batch)| match batch {
                gfx::PaintBatch::Rects(rects) => rects.iter().any(|rect| {
                    (rect.color[0] - 1.0).abs() < 0.01
                        && (rect.color[1] - 1.0).abs() < 0.01
                        && (rect.color[2] - 1.0).abs() < 0.01
                        && rect.color[3] > 0.99
                }),
                gfx::PaintBatch::Text(_) => false,
            });
        assert!(
            popup_bg,
            "popup background must be painted after the covered toolbar text"
        );
    }

    #[test]
    fn nested_popup_batches_paint_parent_before_child() {
        use form_xml::PropValue;

        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.shell
            .set_menu(&PropValue::List(vec![PropValue::Map(vec![
                ("title".into(), PropValue::Str("View".into())),
                (
                    "children".into(),
                    PropValue::List(vec![PropValue::Map(vec![
                        ("title".into(), PropValue::Str("Submenu".into())),
                        (
                            "children".into(),
                            PropValue::List(vec![PropValue::Map(vec![
                                ("title".into(), PropValue::Str("Nested".into())),
                                ("action".into(), PropValue::Str("open_lua".into())),
                            ])]),
                        ),
                    ])]),
                ),
            ])]));
        app.shell.open_path = Some(vec![0, 0]);

        let batches = app.collect_paint();
        let parent = text_batch_index(&batches, "Submenu").expect("parent popup item");
        let child = text_batch_index(&batches, "Nested").expect("nested popup item");
        assert!(
            parent < child,
            "nested submenu must be painted after its parent level"
        );
    }

    #[test]
    fn mdi_content_is_painted_before_floating_top_level_windows() {
        let mut app = App::new(Params::default());
        app.forms
            .insert("mdi".into(), card_form(32.0, 54.0, "MdiWindow"));
        app.mdi.add_subwindow(
            "MdiWindow",
            "mdi".into(),
            rqt::gui::QSizeF::new(300.0, 200.0),
        );
        app.forms
            .insert("float".into(), card_form(80.0, 90.0, "FloatingWindow"));
        app.floating.add_subwindow(
            "FloatingWindow",
            "float".into(),
            rqt::gui::QSizeF::new(300.0, 200.0),
        );

        let batches = app.collect_paint();
        let mdi = text_batch_index(&batches, "MdiWindow").expect("mdi title");
        let floating = text_batch_index(&batches, "FloatingWindow").expect("floating title");
        assert!(mdi < floating, "top-level layer must follow the MDI layer");
    }

    #[test]
    fn application_modal_blocks_mdi_and_shell() {
        use serde_json::json;

        let xml = r#"<ui version="4.0"><class>Dialog</class>
            <widget class="QDialog" name="Dialog">
              <property name="geometry"><rect><x>100</x><y>100</y><width>300</width><height>200</height></rect></property>
            </widget></ui>"#;
        let mut app = App::new(Params {
            full: true,
            main_lua: Some("main.lua".into()),
            ..Params::default()
        });
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "normal",
            "mode": "full",
            "placement": "mdi",
            "ui_xml": xml,
        }));
        app.on_command(&json!({
            "command": "createForm",
            "form_id": "modal",
            "mode": "full",
            "placement": "top-level",
            "modal": true,
            "modal_scope": "application",
            "ui_xml": xml,
        }));
        assert!(app.is_application_modal());
        let index = app.mdi.index_of(|payload| payload == "normal").unwrap();
        let content = app.mdi.subwindow(index).unwrap().content_rect();
        app.cursor = (content.x + 10.0, content.y + 10.0);
        assert_eq!(app.window_hit_modal(), None);

        let index = app.floating.index_of(|payload| payload == "modal").unwrap();
        let content = app.floating.subwindow(index).unwrap().content_rect();
        app.cursor = (content.x + 10.0, content.y + 10.0);
        assert_eq!(
            app.window_hit_modal().map(|(layer, _, _)| layer),
            Some(WindowLayer::Floating)
        );
    }

    #[test]
    fn modal_form_filters_mdi_hit_testing() {
        let mut app = App::new(Params::default());
        app.forms.insert("a".into(), card_form(32.0, 54.0, "One"));
        app.mdi
            .add_subwindow("One", "a".into(), rqt::gui::QSizeF::new(300.0, 200.0));
        app.forms.insert("b".into(), card_form(60.0, 82.0, "Two"));
        app.mdi
            .add_subwindow("Two", "b".into(), rqt::gui::QSizeF::new(300.0, 200.0));

        app.modal_stack.push("b".into());
        app.cursor = (100.0, 100.0);
        assert_eq!(app.mdi_hit_modal().map(|(index, _)| index), Some(1));

        app.cursor = (40.0, 30.0);
        assert_eq!(app.mdi_hit_modal(), None);
    }

    fn add_form(app: &mut App, id: &str, xml: &str) {
        let (tree, root) = rqt::widgets::load_ui(xml).expect("ui");
        app.forms.insert(
            id.into(),
            QtForm {
                tree,
                root,
                under_mouse_widget: None,
            },
        );
        app.mdi
            .add_subwindow(id, id.into(), rqt::gui::QSizeF::new(300.0, 200.0));
    }

    fn focus_name(app: &App, form_id: &str) -> Option<String> {
        let form = app.forms.get(form_id)?;
        let widget = form.tree.focus_widget()?;
        form.tree
            .widget(widget)
            .map(|node| node.object_name.clone())
    }

    const TAB_UI: &str = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QLineEdit" name="lineEdit_1"/></item><item><widget class="QLineEdit" name="lineEdit_2"/></item><item><widget class="QPushButton" name="pushButton_1"><property name="text"><string>OK</string></property></widget></item></layout></widget></ui>"#;
    const TABLE_TAB_UI: &str = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>200</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QTableWidget" name="table"><property name="rowCount"><number>2</number></property><property name="columnCount"><number>2</number></property></widget></item><item><widget class="QPushButton" name="pushButton_1"><property name="text"><string>OK</string></property></widget></item></layout></widget></ui>"#;
    const TREE_KEY_UI: &str = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>320</width><height>220</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QTreeWidget" name="tree"><column><property name="text"><string>Name</string></property></column></widget></item></layout></widget></ui>"#;
    const COMBO_UI: &str = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>120</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QComboBox" name="combo"/></item></layout></widget></ui>"#;
    const LIST_TREE_TAB_UI: &str = r#"<ui version="4.0"><class>Dialog</class><widget class="QDialog" name="Dialog"><property name="geometry"><rect><x>0</x><y>0</y><width>300</width><height>240</height></rect></property><layout class="QVBoxLayout" name="l"><item><widget class="QListWidget" name="listWidget"/></item><item><widget class="QTreeWidget" name="treeWidget"><column><property name="text"><string>1</string></property></column></widget></item><item><widget class="QPushButton" name="pushButton_1"><property name="text"><string>OK</string></property></widget></item></layout></widget></ui>"#;

    #[test]
    fn tree_left_and_right_collapse_expand_and_move_to_parent() {
        let mut app = App::new(Params::default());
        app.on_create_form(&serde_json::json!({
            "form_id": "f",
            "ui_xml": TREE_KEY_UI,
        }));
        app.on_command(&serde_json::json!({
            "command": "setProperty",
            "form_id": "f",
            "object_name": "tree",
            "property": "data",
            "value": [{
                "text": "Root",
                "children": [{ "text": "Child" }],
            }],
        }));
        let tree = app
            .forms
            .get("f")
            .unwrap()
            .tree
            .find_widget("tree")
            .unwrap();
        app.forms.get_mut("f").unwrap().tree.set_focus(tree);
        app.sync_focus("f", tree, false);
        assert_eq!(app.forms["f"].tree.tree(tree).unwrap().row_count(), 1);
        assert!(app.navigate_view(&KeyEdit::Right));
        assert_eq!(app.forms["f"].tree.tree(tree).unwrap().row_count(), 2);
        assert!(app.forms["f"]
            .tree
            .tree(tree)
            .unwrap()
            .current_expanded()
            .unwrap());
        assert!(app.navigate_view(&KeyEdit::Down));
        assert_eq!(app.forms["f"].tree.tree(tree).unwrap().current, Some(1));
        assert!(app.navigate_view(&KeyEdit::Left));
        assert_eq!(app.forms["f"].tree.tree(tree).unwrap().current, Some(0));
        assert_eq!(
            app.forms["f"].tree.tree(tree).unwrap().current_text(),
            rqt::core::QVariant::String("Root".into())
        );
        assert!(app.navigate_view(&KeyEdit::Left));
        assert!(!app.forms["f"]
            .tree
            .tree(tree)
            .unwrap()
            .current_expanded()
            .unwrap());
    }

    #[test]
    fn combo_keyboard_navigates_and_popup_commits_current_index() {
        let mut app = App::new(Params::default());
        app.on_create_form(&serde_json::json!({
            "form_id": "f",
            "ui_xml": COMBO_UI,
        }));
        app.on_command(&serde_json::json!({
            "command": "setProperty",
            "form_id": "f",
            "object_name": "combo",
            "property": "items",
            "value": ["one", "two", "three"],
        }));
        let form = app.forms.get_mut("f").unwrap();
        let combo = form.tree.find_widget("combo").unwrap();
        form.tree.set_focus(combo);
        app.sync_focus("f", combo, false);
        assert_eq!(app.table_focus, Some(("f".into(), "combo".into())));

        assert!(app.combo_key(&KeyEdit::Down));
        assert_eq!(app.forms["f"].tree.combo_current_index(combo), Some(1));
        assert!(app.combo_key(&KeyEdit::Commit));
        assert!(app.forms["f"].tree.combo_popup_open(combo));
        assert!(app.combo_key(&KeyEdit::Down));
        assert_eq!(app.forms["f"].tree.combo_popup_highlight(combo), Some(2));
        assert!(app.combo_key(&KeyEdit::Commit));
        assert_eq!(app.forms["f"].tree.combo_current_index(combo), Some(2));
        assert!(!app.forms["f"].tree.combo_popup_open(combo));
        assert!(app.combo_key(&KeyEdit::Home));
        assert_eq!(app.forms["f"].tree.combo_current_index(combo), Some(0));
        assert!(app.combo_key(&KeyEdit::Insert(" ".into())));
        assert!(app.forms["f"].tree.combo_popup_open(combo));
        assert!(app.combo_key(&KeyEdit::ClearFocus));
        assert!(!app.forms["f"].tree.combo_popup_open(combo));
    }

    #[test]
    fn create_form_focuses_first_focusable_widget() {
        let mut app = App::new(Params::default());
        app.on_create_form(&serde_json::json!({
            "form_id": "f",
            "ui_xml": TAB_UI,
        }));
        assert_eq!(app.mdi.active_payload(), Some(&"f".to_string()));
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_1".into())));
        assert_eq!(focus_name(&app, "f").as_deref(), Some("lineEdit_1"));
    }

    #[test]
    fn mouse_release_keeps_tree_focus() {
        let mut app = App::new(Params::default());
        app.on_create_form(&serde_json::json!({
            "form_id": "f",
            "ui_xml": TAB_UI,
        }));
        let form = app.forms.get("f").unwrap();
        let line = form.tree.find_widget("lineEdit_1").unwrap();
        let line_rect = form.tree.widget(line).unwrap().geometry;
        let root_rect = form.tree.widget(form.root).unwrap().geometry;

        app.cursor = (line_rect.x + 4.0, line_rect.y + 4.0);
        app.dispatch_mouse_press();
        app.dispatch_mouse_release();
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_1".into())));

        app.cursor = (root_rect.x + 2.0, root_rect.y + 2.0);
        app.dispatch_mouse_press();
        app.dispatch_mouse_release();
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_1".into())));
        assert_eq!(focus_name(&app, "f").as_deref(), Some("lineEdit_1"));
    }

    #[test]
    fn minimize_button_collapses_and_restores_form() {
        use rqt::mdi::QMdiSubWindowButton;

        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TAB_UI);
        let index = app.mdi.index_of(|payload| payload == "f").unwrap();
        let minimize = app.mdi.subwindow(index).unwrap().minimize_rect();
        app.cursor = (minimize.x + 2.0, minimize.y + 2.0);
        app.dispatch_mouse_press();
        app.dispatch_mouse_release();
        assert!(app.mdi.subwindow(index).unwrap().minimized);

        let restore = app
            .mdi
            .subwindow(index)
            .unwrap()
            .title_buttons()
            .into_iter()
            .find(|(button, _)| *button == QMdiSubWindowButton::Restore)
            .map(|(_, rect)| rect)
            .unwrap();
        app.cursor = (restore.x + 2.0, restore.y + 2.0);
        app.dispatch_mouse_press();
        app.dispatch_mouse_release();
        assert!(!app.mdi.subwindow(index).unwrap().minimized);
    }

    #[test]
    fn tab_cycles_focusable_widgets_in_active_form() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TAB_UI);
        app.focus_initial_form("f");
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_1".into())));

        app.tab_focus(false);
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_2".into())));
        app.tab_focus(false);
        assert_eq!(app.focused, None);
        assert_eq!(app.table_focus, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));
        app.tab_focus(false);
        assert_eq!(app.focused, Some(("f".into(), "lineEdit_1".into())));
        app.tab_focus(true);
        assert_eq!(app.focused, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));
    }

    #[test]
    fn tab_skips_disabled_widgets() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TAB_UI);
        let form = app.forms.get_mut("f").unwrap();
        let widget = form.tree.find_widget("lineEdit_2").unwrap();
        form.tree.set_enabled(widget, false);
        app.focus_initial_form("f");

        app.tab_focus(false);
        assert_eq!(app.focused, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));
    }

    #[test]
    fn tab_moves_table_current_then_leaves_view() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TABLE_TAB_UI);
        app.focus_initial_form("f");
        let table = app
            .forms
            .get("f")
            .unwrap()
            .tree
            .find_widget("table")
            .unwrap();
        assert_eq!(app.table_focus, Some(("f".into(), "table".into())));

        app.tab_focus(false);
        assert_eq!(
            app.forms
                .get("f")
                .unwrap()
                .tree
                .table(table)
                .unwrap()
                .current(),
            Some((0, 1))
        );
        app.tab_focus(false);
        assert_eq!(
            app.forms
                .get("f")
                .unwrap()
                .tree
                .table(table)
                .unwrap()
                .current(),
            Some((1, 0))
        );
        app.tab_focus(false);
        assert_eq!(
            app.forms
                .get("f")
                .unwrap()
                .tree
                .table(table)
                .unwrap()
                .current(),
            Some((1, 1))
        );
        app.tab_focus(false);
        assert_eq!(app.table_focus, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));

        app.tab_focus(true);
        assert_eq!(app.table_focus, Some(("f".into(), "table".into())));
        app.tab_focus(true);
        assert_eq!(
            app.forms
                .get("f")
                .unwrap()
                .tree
                .table(table)
                .unwrap()
                .current(),
            Some((1, 0))
        );
    }

    #[test]
    fn tab_at_first_table_cell_goes_to_previous_widget() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TABLE_TAB_UI);
        app.focus_initial_form("f");
        let table = app
            .forms
            .get("f")
            .unwrap()
            .tree
            .find_widget("table")
            .unwrap();
        app.forms
            .get_mut("f")
            .unwrap()
            .tree
            .table_mut(table)
            .unwrap()
            .set_current_cell(0, 0);

        app.tab_focus(true);
        assert_eq!(app.table_focus, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));
    }

    #[test]
    fn tab_moves_list_and_tree_current_then_leaves_view() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", LIST_TREE_TAB_UI);
        let form = app.forms.get_mut("f").unwrap();
        let list = form.tree.find_widget("listWidget").unwrap();
        let tree = form.tree.find_widget("treeWidget").unwrap();
        form.tree.set_property(
            list,
            "items",
            QVariant::List(vec![
                QVariant::String("one".into()),
                QVariant::String("two".into()),
            ]),
        );
        form.tree.set_property(
            tree,
            "headerLabels",
            QVariant::List(vec![QVariant::String("Name".into())]),
        );
        form.tree.set_property(
            tree,
            "data",
            QVariant::List(vec![QVariant::Map(vec![(
                "text".into(),
                QVariant::String("Root".into()),
            )])]),
        );

        app.focus_initial_form("f");
        assert_eq!(app.table_focus, Some(("f".into(), "listWidget".into())));
        app.tab_focus(false);
        assert_eq!(
            app.forms.get("f").unwrap().tree.list(list).unwrap().current,
            Some(1)
        );
        app.tab_focus(false);
        assert_eq!(app.table_focus, Some(("f".into(), "treeWidget".into())));
        assert_eq!(
            app.forms.get("f").unwrap().tree.tree(tree).unwrap().current,
            Some(0)
        );
        app.tab_focus(false);
        assert_eq!(app.table_focus, None);
        assert_eq!(focus_name(&app, "f").as_deref(), Some("pushButton_1"));
    }

    #[test]
    fn tab_uses_active_form_only() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "a", TAB_UI);
        add_form(&mut app, "b", TAB_UI);
        let form = app.forms.get_mut("a").unwrap();
        if let Some(widget) = form.tree.find_widget("lineEdit_1") {
            form.tree.set_focus(widget);
        }
        assert_eq!(focus_name(&app, "a").as_deref(), Some("lineEdit_1"));
        assert_eq!(app.mdi.active_payload(), Some(&"b".to_string()));

        app.tab_focus(false);
        assert_eq!(focus_name(&app, "a"), None);
        assert_eq!(focus_name(&app, "b").as_deref(), Some("lineEdit_1"));
        assert_eq!(app.focused, Some(("b".into(), "lineEdit_1".into())));
    }

    #[test]
    fn clear_hover_clears_qss_hover_state_and_enter_target() {
        let mut app = App::new(Params::default());
        add_form(&mut app, "f", TAB_UI);
        let edit = app.forms["f"].tree.find_widget("lineEdit_1").unwrap();
        app.forms
            .get_mut("f")
            .unwrap()
            .tree
            .widget_mut(edit)
            .unwrap()
            .under_mouse = true;
        app.forms.get_mut("f").unwrap().under_mouse_widget = Some(edit);

        app.clear_hover();

        let form = &app.forms["f"];
        assert!(!form.tree.widget(edit).unwrap().under_mouse);
        assert_eq!(form.under_mouse_widget, None);
    }

    #[test]
    fn new_primary_touch_discards_stale_drag() {
        let mut app = App::new(Params::default());
        app.primary_touch = Some(7);
        app.left_down = true;
        app.dragging = Some(Drag {
            layer: WindowLayer::Floating,
            index: 0,
            mode: MdiDragMode::Move,
            last: (100.0, 100.0),
            origin_frame: QRectF::new(10.0, 10.0, 300.0, 200.0),
            origin_cursor: (100.0, 100.0),
        });

        app.prepare_primary_touch(7);

        assert!(app.dragging.is_none());
        assert!(!app.left_down);
        assert_eq!(app.primary_touch, Some(7));
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
        // Enter commits a cell edit and Tab moves focus; neither is ever
        // typed into the widget.
        assert_eq!(
            key_edit(Some("\r"), &Key::Named(NamedKey::Enter)),
            KeyEdit::Commit
        );
        assert_eq!(
            key_edit(Some("\t"), &Key::Named(NamedKey::Tab)),
            KeyEdit::Tab
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

    #[test]
    fn backend_url_decodes_safari_percent_escapes_and_adds_version() {
        let encoded = "ws%3A%2F%2F10.0.0.7%3A4010%2Fsocket%2Fwebsocket%3Fvsn%3D2.0.0";
        assert_eq!(
            backend_with_version(percent_decode(encoded)),
            "ws://10.0.0.7:4010/socket/websocket?vsn=2.0.0"
        );
    }

    #[test]
    fn backend_url_gets_phoenix_version_when_query_is_missing() {
        assert_eq!(
            backend_with_version("ws://10.0.0.7:4010/socket/websocket".into()),
            "ws://10.0.0.7:4010/socket/websocket?vsn=2.0.0"
        );
        assert_eq!(
            backend_with_version("ws://10.0.0.7:4010/socket/websocket?vsn=2.0.0".into()),
            "ws://10.0.0.7:4010/socket/websocket?vsn=2.0.0"
        );
        assert_eq!(
            backend_with_version("http://example".into()),
            "http://example"
        );
    }

    #[test]
    fn web_text_input_maps_semantic_actions() {
        assert_eq!(
            text_input_action("insert", "Привет"),
            (KeyEdit::Insert("Привет".into()), false)
        );
        assert_eq!(
            text_input_action("backspace", ""),
            (KeyEdit::Backspace, false)
        );
        assert_eq!(text_input_action("left", "1"), (KeyEdit::Left, true));
        assert_eq!(text_input_action("right", ""), (KeyEdit::Right, false));
        assert_eq!(text_input_action("home", "1"), (KeyEdit::Home, true));
        assert_eq!(text_input_action("tab", "1"), (KeyEdit::Tab, true));
        assert_eq!(
            text_input_action("selectAll", ""),
            (KeyEdit::SelectAll, false)
        );
        assert_eq!(
            text_input_action("clearFocus", ""),
            (KeyEdit::ClearFocus, false)
        );
    }
}
