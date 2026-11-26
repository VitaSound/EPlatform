use std::cell::RefCell;
use std::sync::Arc;

/// Single embedded copy of the bundled UI font — wasm has no system fonts
/// and `include_bytes!` here and in `app::gfx` used to embed it twice
/// (+1.5 MB of wasm data).
pub static DEJA_VU_SANS: &[u8] = include_bytes!("../../../assets/DejaVuSans.ttf");

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QPointF {
    pub x: f32,
    pub y: f32,
}

impl QPointF {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QSizeF {
    pub width: f32,
    pub height: f32,
}

impl QSizeF {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QMargins {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl QMargins {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QRectF {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl QRectF {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn top_left(&self) -> QPointF {
        QPointF::new(self.x, self.y)
    }

    pub fn bottom_right(&self) -> QPointF {
        QPointF::new(self.right(), self.bottom())
    }

    pub fn size(&self) -> QSizeF {
        QSizeF::new(self.width, self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn contains(&self, point: QPointF) -> bool {
        point.x >= self.left()
            && point.x < self.right()
            && point.y >= self.top()
            && point.y < self.bottom()
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Overlap of two rectangles, `None` when they do not touch.
    pub fn intersected(&self, other: &Self) -> Option<Self> {
        let left = self.left().max(other.left());
        let top = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= left || bottom <= top {
            None
        } else {
            Some(Self::new(left, top, right - left, bottom - top))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Default for QColor {
    fn default() -> Self {
        Self::rgba(0.0, 0.0, 0.0, 1.0)
    }
}

impl QColor {
    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        )
    }

    pub fn rgba8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self::rgba(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            alpha as f32 / 255.0,
        )
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#').unwrap_or(value);
        let parse = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
        match hex.len() {
            3 => {
                let expand = |index: usize| {
                    let value = parse(index..index + 1)?;
                    Some(value * 16 + value)
                };
                Some(Self::rgb(expand(0)?, expand(1)?, expand(2)?))
            }
            4 => {
                let expand = |index: usize| {
                    let value = parse(index..index + 1)?;
                    Some(value * 16 + value)
                };
                Some(Self::rgba8(expand(1)?, expand(2)?, expand(3)?, expand(0)?))
            }
            6 => Some(Self::rgb(parse(0..2)?, parse(2..4)?, parse(4..6)?)),
            // Qt style sheets interpret 4/8 digit values as ARGB, unlike
            // CSS (#RRGGBBAA).
            8 => Some(Self::rgba8(
                parse(2..4)?,
                parse(4..6)?,
                parse(6..8)?,
                parse(0..2)?,
            )),
            _ => None,
        }
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            ..self
        }
    }

    fn to_rgb8(self) -> (u8, u8, u8) {
        (
            (self.red * 255.0).round().clamp(0.0, 255.0) as u8,
            (self.green * 255.0).round().clamp(0.0, 255.0) as u8,
            (self.blue * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    fn from_rgb8(red: u8, green: u8, blue: u8, alpha: f32) -> Self {
        Self::rgba(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            alpha,
        )
    }

    fn to_hsv(self) -> (u8, u8, u8) {
        let (red, green, blue) = self.to_rgb8();
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let delta = max.saturating_sub(min);
        let value = max;
        let saturation = if max == 0 {
            0
        } else {
            ((delta as u32 * 255) / max as u32) as u8
        };
        let hue = if delta == 0 {
            0
        } else {
            let delta = delta as i32;
            let (r, g, b) = (red as i32, green as i32, blue as i32);
            let mut h = if max == red {
                (g - b) * 60 / delta
            } else if max == green {
                (b - r) * 60 / delta + 120
            } else {
                (r - g) * 60 / delta + 240
            };
            if h < 0 {
                h += 360;
            }
            (h / 2) as u8
        };
        (hue, saturation, value)
    }

    fn from_hsv(hue: u8, saturation: u8, value: u8, alpha: f32) -> Self {
        if saturation == 0 {
            return Self::from_rgb8(value, value, value, alpha);
        }
        let h = hue as u32 * 2;
        let s = saturation as u32;
        let v = value as u32;
        let region = h / 60;
        let remainder = (h % 60) * 255 / 60;
        let p = (v * (255 - s)) / 255;
        let q = (v * (255 - ((s * remainder) / 255))) / 255;
        let t = (v * (255 - ((s * (255 - remainder)) / 255))) / 255;
        let (r, g, b) = match region {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self::from_rgb8(r as u8, g as u8, b as u8, alpha)
    }

    pub fn lighter(self, factor: i32) -> Self {
        let (red, green, blue) = self.to_rgb8();
        let scale = |channel: u8| {
            (channel as f32 * factor as f32 / 100.0)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Self::from_rgb8(scale(red), scale(green), scale(blue), self.alpha)
    }

    pub fn darker(self, factor: i32) -> Self {
        let (red, green, blue) = self.to_rgb8();
        let factor = factor.max(1);
        let scale = |channel: u8| {
            (channel as f32 * 100.0 / factor as f32)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Self::from_rgb8(scale(red), scale(green), scale(blue), self.alpha)
    }

    pub fn with_saturation_factor(self, factor: i32) -> Self {
        let (hue, saturation, value) = self.to_hsv();
        let saturation = (saturation as i32 * factor / 100).clamp(0, 255) as u8;
        Self::from_hsv(hue, saturation, value, self.alpha)
    }

    pub fn with_value(self, value: u8) -> Self {
        let (hue, saturation, _) = self.to_hsv();
        Self::from_hsv(hue, saturation, value, self.alpha)
    }

    pub fn value(self) -> u8 {
        self.to_hsv().2
    }

    pub fn blend(self, other: Self, self_percent: i32) -> Self {
        let percent = self_percent.clamp(0, 100) as u32;
        let (left_r, left_g, left_b) = self.to_rgb8();
        let (right_r, right_g, right_b) = other.to_rgb8();
        let lerp = |left: u8, right: u8| {
            ((left as u32 * percent + right as u32 * (100 - percent)) / 100) as u8
        };
        Self::from_rgb8(
            lerp(left_r, right_r),
            lerp(left_g, right_g),
            lerp(left_b, right_b),
            self.alpha,
        )
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::rgba(
            self.red + (other.red - self.red) * t,
            self.green + (other.green - self.green) * t,
            self.blue + (other.blue - self.blue) * t,
            self.alpha + (other.alpha - self.alpha) * t,
        )
    }

    pub fn q_gray(self) -> u8 {
        let (red, green, blue) = self.to_rgb8();
        ((red as u32 * 11 + green as u32 * 16 + blue as u32 * 5) / 32) as u8
    }

    pub fn to_linear(self) -> [f32; 4] {
        [
            srgb_to_linear(self.red),
            srgb_to_linear(self.green),
            srgb_to_linear(self.blue),
            self.alpha,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QPixmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl QPixmap {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let index = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ]
    }
}

pub fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QPaletteRole {
    Window = 0,
    WindowText,
    Base,
    AlternateBase,
    ToolTipBase,
    ToolTipText,
    Text,
    Button,
    ButtonText,
    BrightText,
    Highlight,
    HighlightedText,
    Link,
    PlaceholderText,
    Mid,
    Dark,
    Light,
    Shadow,
}

pub const QPALETTE_ROLE_COUNT: usize = QPaletteRole::Shadow as usize + 1;

#[derive(Clone, Debug, PartialEq)]
pub struct QPalette {
    colors: [QColor; QPALETTE_ROLE_COUNT],
}

impl Default for QPalette {
    fn default() -> Self {
        Self::fusion()
    }
}

impl QPalette {
    pub fn fusion() -> Self {
        let mut palette = Self {
            colors: [QColor::default(); QPALETTE_ROLE_COUNT],
        };
        palette.set_color(QPaletteRole::Window, QColor::rgb(0xef, 0xef, 0xef));
        palette.set_color(QPaletteRole::WindowText, QColor::rgb(0x00, 0x00, 0x00));
        palette.set_color(QPaletteRole::Base, QColor::rgb(0xff, 0xff, 0xff));
        palette.set_color(QPaletteRole::AlternateBase, QColor::rgb(0xf7, 0xf7, 0xf7));
        palette.set_color(QPaletteRole::ToolTipBase, QColor::rgb(0xff, 0xff, 0xdc));
        palette.set_color(QPaletteRole::ToolTipText, QColor::rgb(0x00, 0x00, 0x00));
        palette.set_color(QPaletteRole::Text, QColor::rgb(0x00, 0x00, 0x00));
        palette.set_color(QPaletteRole::Button, QColor::rgb(0xef, 0xef, 0xef));
        palette.set_color(QPaletteRole::ButtonText, QColor::rgb(0x00, 0x00, 0x00));
        palette.set_color(QPaletteRole::BrightText, QColor::rgb(0xff, 0xff, 0xff));
        palette.set_color(QPaletteRole::Highlight, QColor::rgb(0x30, 0x8c, 0xc6));
        palette.set_color(QPaletteRole::HighlightedText, QColor::rgb(0xff, 0xff, 0xff));
        palette.set_color(QPaletteRole::Link, QColor::rgb(0x00, 0x00, 0xff));
        palette.set_color(QPaletteRole::PlaceholderText, QColor::rgb(0x80, 0x80, 0x80));
        palette.set_color(QPaletteRole::Mid, QColor::rgb(0xb8, 0xb8, 0xb8));
        palette.set_color(QPaletteRole::Dark, QColor::rgb(0x9f, 0x9f, 0x9f));
        palette.set_color(QPaletteRole::Light, QColor::rgb(0xff, 0xff, 0xff));
        palette.set_color(QPaletteRole::Shadow, QColor::rgb(0x76, 0x76, 0x76));
        palette
    }

    pub fn color(&self, role: QPaletteRole) -> QColor {
        self.colors[role as usize]
    }

    pub fn set_color(&mut self, role: QPaletteRole, color: QColor) {
        self.colors[role as usize] = color;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QFontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    #[default]
    Normal = 400,
    Medium = 500,
    DemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QFont {
    pub family: String,
    pub pixel_size: f32,
    pub weight: QFontWeight,
    pub italic: bool,
}

impl QFont {
    pub fn new(family: impl Into<String>, pixel_size: f32) -> Self {
        Self {
            family: family.into(),
            pixel_size,
            weight: QFontWeight::Normal,
            italic: false,
        }
    }
}

impl Default for QFont {
    fn default() -> Self {
        Self::new("DejaVu Sans", 14.0)
    }
}

thread_local! {
    static FONT_SYSTEM: RefCell<cosmic_text::FontSystem> = RefCell::new(
        cosmic_text::FontSystem::new_with_fonts([
            cosmic_text::fontdb::Source::Binary(Arc::new(DEJA_VU_SANS)),
        ]),
    );
}

pub(crate) fn with_font_system<R>(f: impl FnOnce(&mut cosmic_text::FontSystem) -> R) -> R {
    FONT_SYSTEM.with(|system| f(&mut system.borrow_mut()))
}

fn font_weight(font: &QFont) -> cosmic_text::Weight {
    cosmic_text::Weight(font.weight as u16)
}

fn font_style(font: &QFont) -> cosmic_text::Style {
    if font.italic {
        cosmic_text::Style::Italic
    } else {
        cosmic_text::Style::Normal
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QFontMetrics {
    font: QFont,
    ascent: f32,
    descent: f32,
    leading: f32,
    height: f32,
}

impl QFontMetrics {
    pub fn new(font: &QFont) -> Self {
        let pixel_size = font.pixel_size.max(1.0);
        let family = font.family.clone();
        let weight = font_weight(font);
        let style = font_style(font);

        let metrics = FONT_SYSTEM.with(|system| {
            let mut system = system.borrow_mut();
            let id = system
                .db_mut()
                .query(&cosmic_text::fontdb::Query {
                    families: &[cosmic_text::Family::Name(&family)],
                    weight,
                    stretch: cosmic_text::Stretch::Normal,
                    style,
                })
                .or_else(|| system.db_mut().faces().next().map(|face| face.id));
            id.and_then(|id| system.get_font(id, weight)).map(|face| {
                let metrics = face.metrics();
                let scale = pixel_size / metrics.units_per_em as f32;
                (
                    metrics.ascent * scale,
                    metrics.descent.abs() * scale,
                    metrics.leading.max(0.0) * scale,
                )
            })
        });

        let (ascent, descent, leading) =
            metrics.unwrap_or((pixel_size * 0.8, pixel_size * 0.2, 0.0));
        Self {
            font: font.clone(),
            ascent,
            descent,
            leading,
            height: ascent + descent + leading,
        }
    }

    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    pub fn descent(&self) -> f32 {
        self.descent
    }

    pub fn leading(&self) -> f32 {
        self.leading
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn horizontal_advance(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let family = self.font.family.clone();
        let weight = font_weight(&self.font);
        let pixel_size = self.font.pixel_size.max(1.0);
        FONT_SYSTEM.with(|system| {
            let mut system = system.borrow_mut();
            let mut buffer = cosmic_text::Buffer::new(
                &mut system,
                cosmic_text::Metrics::new(pixel_size, self.height),
            );
            buffer.set_size(Some(1_000_000.0), Some(self.height));
            buffer.set_text(
                text,
                &cosmic_text::Attrs::new()
                    .family(cosmic_text::Family::Name(&family))
                    .weight(weight)
                    .style(font_style(&self.font)),
                cosmic_text::Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut system, false);
            buffer
                .layout_runs()
                .map(|run| run.line_w)
                .fold(0.0, f32::max)
        })
    }

    pub fn ink_width(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let family = self.font.family.clone();
        let weight = font_weight(&self.font);
        let pixel_size = self.font.pixel_size.max(1.0);
        FONT_SYSTEM.with(|system| {
            let mut system = system.borrow_mut();
            let mut buffer = cosmic_text::Buffer::new(
                &mut system,
                cosmic_text::Metrics::new(pixel_size, self.height),
            );
            buffer.set_size(Some(1_000_000.0), Some(self.height));
            buffer.set_text(
                text,
                &cosmic_text::Attrs::new()
                    .family(cosmic_text::Family::Name(&family))
                    .weight(weight)
                    .style(font_style(&self.font)),
                cosmic_text::Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut system, false);
            let mut swash = cosmic_text::SwashCache::new();
            let mut right = 0.0_f32;
            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((0.0, 0.0), 1.0);
                    swash.with_pixels(
                        &mut system,
                        physical.cache_key,
                        cosmic_text::Color(0xff000000),
                        |x, _y, _color| {
                            right = right.max((physical.x + x + 1) as f32);
                        },
                    );
                }
            }
            right
        })
    }

    pub fn bounding_rect(&self, text: &str) -> QRectF {
        QRectF::new(
            0.0,
            0.0,
            self.horizontal_advance(text).max(self.ink_width(text)),
            self.height,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum QPaintCommand {
    Save,
    Restore,
    Translate {
        dx: f32,
        dy: f32,
    },
    SetClip {
        rect: Option<QRectF>,
    },
    FillRect {
        rect: QRectF,
        color: QColor,
    },
    StrokeRect {
        rect: QRectF,
        width: f32,
        color: QColor,
    },
    StrokeRoundedRect {
        rect: QRectF,
        radius: f32,
        width: f32,
        color: QColor,
    },
    FillRoundedRect {
        rect: QRectF,
        radius: f32,
        color: QColor,
    },
    DrawLine {
        from: QPointF,
        to: QPointF,
        width: f32,
        color: QColor,
    },
    FillLinearGradientRect {
        rect: QRectF,
        radius: f32,
        start: QColor,
        end: QColor,
        /// `true` ramps left to right (Qt's vertical scroll bar gradients run
        /// across the bar), `false` ramps top to bottom.
        horizontal: bool,
    },
    FillEllipse {
        rect: QRectF,
        color: QColor,
    },
    StrokeEllipse {
        rect: QRectF,
        width: f32,
        color: QColor,
    },
    FillTriangle {
        a: QPointF,
        b: QPointF,
        c: QPointF,
        color: QColor,
    },
    DrawPixmap {
        rect: QRectF,
        pixmap: QPixmap,
        opacity: f32,
    },
    DrawText {
        rect: QRectF,
        text: String,
        font: QFont,
        color: QColor,
    },
}

#[derive(Clone, Debug)]
struct QPainterState {
    clip: Option<QRectF>,
    font: QFont,
    palette: QPalette,
    opacity: f32,
}

#[derive(Clone, Debug)]
pub struct QPainter {
    current: QPainterState,
    stack: Vec<QPainterState>,
    commands: Vec<QPaintCommand>,
}

impl Default for QPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl QPainter {
    pub fn new() -> Self {
        Self {
            current: QPainterState {
                clip: None,
                font: QFont::default(),
                palette: QPalette::fusion(),
                opacity: 1.0,
            },
            stack: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn commands(&self) -> &[QPaintCommand] {
        &self.commands
    }

    pub fn into_commands(self) -> Vec<QPaintCommand> {
        self.commands
    }

    pub fn save(&mut self) {
        self.stack.push(self.current.clone());
        self.commands.push(QPaintCommand::Save);
    }

    pub fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.current = state;
            self.commands.push(QPaintCommand::Restore);
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.commands.push(QPaintCommand::Translate { dx, dy });
    }

    pub fn set_clip_rect(&mut self, rect: Option<QRectF>) {
        // Qt clips children to their parent. Our widget painter relies on
        // `set_clip_rect` to restrict painting, so intersect with the clip
        // inherited from the enclosing `save()` instead of replacing it.
        let rect = match (rect, self.current.clip) {
            (Some(rect), Some(parent)) => Some(
                rect.intersected(&parent)
                    .unwrap_or(QRectF::new(rect.x, rect.y, 0.0, 0.0)),
            ),
            (Some(rect), None) => Some(rect),
            (None, _) => None,
        };
        self.current.clip = rect;
        self.commands.push(QPaintCommand::SetClip { rect });
    }

    pub fn clip_rect(&self) -> Option<QRectF> {
        self.current.clip
    }

    pub fn set_font(&mut self, font: QFont) {
        self.current.font = font;
    }

    pub fn font(&self) -> &QFont {
        &self.current.font
    }

    pub fn set_palette(&mut self, palette: QPalette) {
        self.current.palette = palette;
    }

    pub fn palette(&self) -> &QPalette {
        &self.current.palette
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.current.opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn opacity(&self) -> f32 {
        self.current.opacity
    }

    pub fn draw_commands(&mut self, commands: &[QPaintCommand]) {
        self.commands.extend_from_slice(commands);
    }

    pub fn fill_rect(&mut self, rect: QRectF, color: QColor) {
        self.commands.push(QPaintCommand::FillRect {
            rect,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn stroke_rect(&mut self, rect: QRectF, width: f32, color: QColor) {
        self.commands.push(QPaintCommand::StrokeRect {
            rect,
            width,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn stroke_rounded_rect(&mut self, rect: QRectF, radius: f32, width: f32, color: QColor) {
        self.commands.push(QPaintCommand::StrokeRoundedRect {
            rect,
            radius,
            width,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn fill_rounded_rect(&mut self, rect: QRectF, radius: f32, color: QColor) {
        self.commands.push(QPaintCommand::FillRoundedRect {
            rect,
            radius,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn draw_line(&mut self, from: QPointF, to: QPointF, width: f32, color: QColor) {
        self.commands.push(QPaintCommand::DrawLine {
            from,
            to,
            width,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn stroke_ellipse(&mut self, rect: QRectF, width: f32, color: QColor) {
        self.commands.push(QPaintCommand::StrokeEllipse {
            rect,
            width,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn fill_triangle(&mut self, a: QPointF, b: QPointF, c: QPointF, color: QColor) {
        self.commands.push(QPaintCommand::FillTriangle {
            a,
            b,
            c,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn fill_ellipse(&mut self, rect: QRectF, color: QColor) {
        self.commands.push(QPaintCommand::FillEllipse {
            rect,
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }

    pub fn fill_linear_gradient_rect(
        &mut self,
        rect: QRectF,
        radius: f32,
        start: QColor,
        end: QColor,
    ) {
        self.commands.push(QPaintCommand::FillLinearGradientRect {
            rect,
            radius,
            start: start.with_alpha(start.alpha * self.current.opacity),
            end: end.with_alpha(end.alpha * self.current.opacity),
            horizontal: false,
        });
    }

    /// Qt paints a vertical scroll bar's gradients across the bar, not along
    /// it (probe: 253 -> 242 left to right).
    pub fn fill_horizontal_gradient_rect(
        &mut self,
        rect: QRectF,
        radius: f32,
        start: QColor,
        end: QColor,
    ) {
        self.commands.push(QPaintCommand::FillLinearGradientRect {
            rect,
            radius,
            start: start.with_alpha(start.alpha * self.current.opacity),
            end: end.with_alpha(end.alpha * self.current.opacity),
            horizontal: true,
        });
    }

    pub fn draw_pixmap(&mut self, rect: QRectF, pixmap: QPixmap, opacity: f32) {
        self.commands.push(QPaintCommand::DrawPixmap {
            rect,
            pixmap,
            opacity: opacity.clamp(0.0, 1.0) * self.current.opacity,
        });
    }

    pub fn draw_text(&mut self, rect: QRectF, text: impl Into<String>, color: QColor) {
        self.commands.push(QPaintCommand::DrawText {
            rect,
            text: text.into(),
            font: self.current.font.clone(),
            color: color.with_alpha(color.alpha * self.current.opacity),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_uses_qt_half_open_geometry() {
        let rect = QRectF::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!((rect.left(), rect.top()), (10.0, 20.0));
        assert_eq!((rect.right(), rect.bottom()), (40.0, 60.0));
        assert_eq!(rect.size(), QSizeF::new(30.0, 40.0));
        assert!(rect.contains(QPointF::new(10.0, 20.0)));
        assert!(rect.contains(QPointF::new(39.9, 59.9)));
        assert!(!rect.contains(QPointF::new(40.0, 60.0)));
    }

    #[test]
    fn color_lighter_and_darker_match_qt() {
        assert_eq!(
            QColor::rgb(48, 140, 198).darker(125).to_rgb8(),
            (38, 112, 158)
        );
        assert_eq!(
            QColor::rgb(50, 146, 206).lighter(124).to_rgb8(),
            (62, 181, 255)
        );
        assert_eq!(
            QColor::rgb(239, 239, 239).darker(140).to_rgb8(),
            (171, 171, 171)
        );
    }

    #[test]
    fn color_converts_srgb_to_linear() {
        assert_eq!(QColor::rgb(0, 0, 0).to_linear()[0], 0.0);
        assert!((QColor::rgb(255, 255, 255).to_linear()[0] - 1.0).abs() < 1e-5);
        assert_eq!(
            QColor::from_hex("#308cc6"),
            Some(QColor::rgb(0x30, 0x8c, 0xc6))
        );
        assert_eq!(QColor::from_hex("ba"), None);
    }

    #[test]
    fn fusion_palette_has_qt_default_roles() {
        let palette = QPalette::fusion();
        assert_eq!(
            palette.color(QPaletteRole::Window),
            QColor::rgb(239, 239, 239)
        );
        assert_eq!(
            palette.color(QPaletteRole::Base),
            QColor::rgb(255, 255, 255)
        );
        assert_eq!(
            palette.color(QPaletteRole::Highlight),
            QColor::rgb(0x30, 0x8c, 0xc6)
        );
    }

    #[test]
    fn font_metrics_uses_real_font_vertical_metrics() {
        let metrics = QFontMetrics::new(&QFont::default());
        assert!(metrics.ascent() > 8.0);
        assert!(metrics.descent() > 0.0);
        assert!(metrics.height() >= metrics.ascent() + metrics.descent());
        assert!(metrics.leading() >= 0.0);
    }

    #[test]
    fn font_metrics_shapes_text_advance() {
        let metrics = QFontMetrics::new(&QFont::default());
        assert_eq!(metrics.horizontal_advance(""), 0.0);
        let narrow = metrics.horizontal_advance("iiii");
        let wide = metrics.horizontal_advance("MMMM");
        assert!(wide > narrow);
        assert!(metrics.bounding_rect("A=").width > 0.0);
    }

    #[test]
    fn painter_records_qt_state_and_primitives() {
        let mut painter = QPainter::new();
        painter.translate(10.0, 20.0);
        painter.set_clip_rect(Some(QRectF::new(0.0, 0.0, 100.0, 50.0)));
        painter.fill_rect(QRectF::new(1.0, 2.0, 3.0, 4.0), QColor::rgb(255, 0, 0));
        painter.save();
        painter.set_opacity(0.5);
        painter.draw_text(
            QRectF::new(0.0, 0.0, 50.0, 20.0),
            "A=",
            QColor::rgb(0, 0, 0),
        );
        painter.restore();

        assert_eq!(painter.commands().len(), 6);
        assert!(matches!(
            painter.commands()[0],
            QPaintCommand::Translate { .. }
        ));
        assert!(matches!(
            painter.commands()[1],
            QPaintCommand::SetClip { .. }
        ));
        assert!(matches!(
            painter.commands()[2],
            QPaintCommand::FillRect { .. }
        ));
        assert_eq!(painter.commands()[3], QPaintCommand::Save);
        match &painter.commands()[4] {
            QPaintCommand::DrawText { color, text, .. } => {
                assert_eq!(text, "A=");
                assert!((color.alpha - 0.5).abs() < 1e-6);
            }
            other => panic!("expected DrawText, got {other:?}"),
        }
        assert_eq!(painter.commands()[5], QPaintCommand::Restore);
        assert_eq!(painter.opacity(), 1.0);
    }
}
