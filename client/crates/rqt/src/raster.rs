use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, SwashCache};
use std::path::Path;

use crate::gui::{with_font_system, QColor, QFont, QPaintCommand, QPointF, QRectF};

#[derive(Clone, Debug, PartialEq)]
pub struct QRasterImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl QRasterImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    pub fn fill(&mut self, color: QColor) {
        let rgba = color_to_rgba(color);
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ]
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: QColor, clip: Option<QRectF>) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if let Some(clip) = clip {
            if x as f32 + 0.5 < clip.left()
                || y as f32 + 0.5 < clip.top()
                || x as f32 + 0.5 >= clip.right()
                || y as f32 + 0.5 >= clip.bottom()
            {
                return;
            }
        }
        let source = color_to_rgba(color);
        let alpha = source[3] as f32 / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let index = ((y as u32 * self.width + x as u32) * 4) as usize;
        for channel in 0..3 {
            let dst = self.pixels[index + channel] as f32;
            let src = source[channel] as f32;
            self.pixels[index + channel] = (src * alpha + dst * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        let dst_a = self.pixels[index + 3] as f32 / 255.0;
        let out_a = alpha + dst_a * (1.0 - alpha);
        self.pixels[index + 3] = (out_a * 255.0).round() as u8;
    }

    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), String> {
        std::fs::write(path, self.to_png_bytes()?).map_err(|error| error.to_string())
    }

    pub fn to_png_bytes(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buffer, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|error| error.to_string())?;
        }
        Ok(buffer)
    }

    pub fn from_png(path: impl AsRef<Path>) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        Self::read_png(file)
    }

    /// Decode PNG from memory (works on `wasm32` where `std::fs` is unusable).
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, String> {
        Self::read_png(std::io::Cursor::new(bytes))
    }

    fn read_png<R: std::io::Read>(reader: R) -> Result<Self, String> {
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| error.to_string())?;
        let mut pixels = vec![0; (info.width * info.height * 4) as usize];
        match info.color_type {
            png::ColorType::Rgba => {
                let length = pixels.len();
                pixels.copy_from_slice(&buffer[..length])
            }
            png::ColorType::Rgb => {
                for (index, source) in buffer.chunks_exact(3).enumerate() {
                    let target = index * 4;
                    pixels[target] = source[0];
                    pixels[target + 1] = source[1];
                    pixels[target + 2] = source[2];
                    pixels[target + 3] = 255;
                }
            }
            png::ColorType::Grayscale => {
                for (index, value) in buffer.iter().enumerate() {
                    let target = index * 4;
                    pixels[target] = *value;
                    pixels[target + 1] = *value;
                    pixels[target + 2] = *value;
                    pixels[target + 3] = 255;
                }
            }
            _ => return Err("unsupported PNG color type".into()),
        }
        Ok(Self {
            width: info.width,
            height: info.height,
            pixels,
        })
    }
}

fn color_to_rgba(color: QColor) -> [u8; 4] {
    [
        (color.red * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.green * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.blue * 255.0).round().clamp(0.0, 255.0) as u8,
        (color.alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

pub fn rasterize(
    commands: &[QPaintCommand],
    width: u32,
    height: u32,
    clear: QColor,
) -> QRasterImage {
    let mut image = QRasterImage::new(width, height);
    image.fill(clear);
    let mut clip: Option<QRectF> = None;
    let mut stack: Vec<Option<QRectF>> = Vec::new();

    for command in commands {
        match command {
            QPaintCommand::Save => stack.push(clip),
            QPaintCommand::Restore => clip = stack.pop().unwrap_or(None),
            QPaintCommand::SetClip { rect } => clip = *rect,
            QPaintCommand::Translate { .. } => {}
            QPaintCommand::FillRect { rect, color } => {
                fill_rect(&mut image, *rect, *color, clip);
            }
            QPaintCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => {
                fill_rounded(&mut image, *rect, *radius, *color, clip);
            }
            QPaintCommand::FillLinearGradientRect {
                rect,
                radius,
                start,
                end,
                horizontal,
            } => {
                fill_linear_gradient(
                    &mut image, *rect, *radius, *start, *end, *horizontal, clip,
                );
            }
            QPaintCommand::StrokeRect { rect, width, color } => {
                fill_rounded_border(&mut image, *rect, 0.0, *width, *color, clip);
            }
            QPaintCommand::StrokeRoundedRect {
                rect,
                radius,
                width,
                color,
            } => {
                fill_rounded_border(&mut image, *rect, *radius, *width, *color, clip);
            }
            QPaintCommand::DrawLine {
                from,
                to,
                width,
                color,
            } => {
                draw_line(&mut image, *from, *to, *width, *color, clip);
            }
            QPaintCommand::FillEllipse { rect, color } => {
                fill_ellipse(&mut image, *rect, *color, clip);
            }
            QPaintCommand::StrokeEllipse { rect, width, color } => {
                stroke_ellipse(&mut image, *rect, *width, *color, clip);
            }
            QPaintCommand::FillTriangle { a, b, c, color } => {
                fill_triangle(&mut image, *a, *b, *c, *color, clip);
            }
            QPaintCommand::DrawPixmap {
                rect,
                pixmap,
                opacity,
            } => draw_pixmap(&mut image, *rect, pixmap, *opacity, clip),
            QPaintCommand::DrawText {
                rect,
                text,
                font,
                color,
            } => draw_text(&mut image, *rect, text, font, *color, clip),
        }
    }
    image
}

fn fill_rounded_border(
    image: &mut QRasterImage,
    rect: QRectF,
    radius: f32,
    width: f32,
    color: QColor,
    clip: Option<QRectF>,
) {
    let left = rect.left().round() as i32;
    let top = rect.top().round() as i32;
    let right = rect.right().round() as i32;
    let bottom = rect.bottom().round() as i32;
    if right <= left || bottom <= top {
        return;
    }
    let stroke = width.round().max(1.0) as i32;
    let radius = radius
        .round()
        .max(0.0)
        .min((right - left).min(bottom - top) as f32 / 2.0);
    let inner_left = left + stroke;
    let inner_top = top + stroke;
    let inner_right = right - stroke;
    let inner_bottom = bottom - stroke;
    let inner_radius = (radius - stroke as f32).max(0.0);
    let bounds = intersect(
        QRectF::new(
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ),
        clip,
    );
    let Some(bounds) = bounds else {
        return;
    };
    let x0 = bounds.left().floor() as i32;
    let y0 = bounds.top().floor() as i32;
    let x1 = bounds.right().ceil() as i32;
    let y1 = bounds.bottom().ceil() as i32;
    for y in y0..y1 {
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let outer = inside_rounded_rect(px, py, left, top, right, bottom, radius);
            let inner = inside_rounded_rect(
                px,
                py,
                inner_left,
                inner_top,
                inner_right,
                inner_bottom,
                inner_radius,
            );
            if outer && !inner {
                image.blend_pixel(x, y, color, None);
            }
        }
    }
}

fn inside_rounded_rect(
    x: f32,
    y: f32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    radius: f32,
) -> bool {
    if x < left as f32 || x >= right as f32 || y < top as f32 || y >= bottom as f32 {
        return false;
    }
    if radius <= 0.0 {
        return true;
    }
    let cx = (left as f32 + radius).max((right as f32 - radius).min(x));
    let cy = (top as f32 + radius).max((bottom as f32 - radius).min(y));
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= radius * radius
}

fn fill_triangle(
    image: &mut QRasterImage,
    a: QPointF,
    b: QPointF,
    c: QPointF,
    color: QColor,
    clip: Option<QRectF>,
) {
    let min_x = a.x.min(b.x).min(c.x).floor() as i32;
    let min_y = a.y.min(b.y).min(c.y).floor() as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil() as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil() as i32;
    let Some(bounds) = intersect(
        QRectF::new(
            min_x as f32,
            min_y as f32,
            (max_x - min_x) as f32,
            (max_y - min_y) as f32,
        ),
        clip,
    ) else {
        return;
    };
    let edge =
        |p: QPointF, q: QPointF, x: f32, y: f32| (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x);
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let e0 = edge(a, b, px, py);
                    let e1 = edge(b, c, px, py);
                    let e2 = edge(c, a, px, py);
                    if (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0)
                        || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0)
                    {
                        covered += 1;
                    }
                }
            }
            if covered > 0 {
                let coverage = covered as f32 / 16.0;
                image.blend_pixel(x, y, color.with_alpha(color.alpha * coverage), None);
            }
        }
    }
}

fn stroke_ellipse(
    image: &mut QRasterImage,
    rect: QRectF,
    width: f32,
    color: QColor,
    clip: Option<QRectF>,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let Some(bounds) = intersect(rect, clip) else {
        return;
    };
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let rx = rect.width / 2.0;
    let ry = rect.height / 2.0;
    let inner = (rx.min(ry) - width).max(0.0);
    let outer = rx.min(ry);
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let distance = (dx * dx + dy * dy).sqrt();
            let outer_coverage = (outer - distance + 0.5).clamp(0.0, 1.0);
            let inner_coverage = (distance - inner + 0.5).clamp(0.0, 1.0);
            let coverage = outer_coverage.min(inner_coverage);
            if coverage > 0.0 {
                image.blend_pixel(x, y, color.with_alpha(color.alpha * coverage), None);
            }
        }
    }
}

fn draw_line(
    image: &mut QRasterImage,
    from: QPointF,
    to: QPointF,
    width: f32,
    color: QColor,
    clip: Option<QRectF>,
) {
    let width = width.max(1.0);
    let half = width / 2.0;
    let left = from.x.min(to.x) - half;
    let top = from.y.min(to.y) - half;
    let right = from.x.max(to.x) + half;
    let bottom = from.y.max(to.y) + half;
    let Some(bounds) = intersect(QRectF::new(left, top, right - left, bottom - top), clip) else {
        return;
    };
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length_squared = dx * dx + dy * dy;
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = if length_squared > 0.0 {
                (((px - from.x) * dx + (py - from.y) * dy) / length_squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let qx = from.x + dx * t;
            let qy = from.y + dy * t;
            let distance = ((px - qx) * (px - qx) + (py - qy) * (py - qy)).sqrt();
            let coverage = (half + 0.5 - distance).clamp(0.0, 1.0);
            if coverage > 0.0 {
                image.blend_pixel(x, y, color.with_alpha(color.alpha * coverage), None);
            }
        }
    }
}

fn fill_ellipse(image: &mut QRasterImage, rect: QRectF, color: QColor, clip: Option<QRectF>) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let Some(bounds) = intersect(rect, clip) else {
        return;
    };
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let rx = rect.width / 2.0;
    let ry = rect.height / 2.0;
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            let distance = (dx * dx + dy * dy).sqrt();
            let coverage = ((1.0 - distance) * rx.min(ry) + 0.5).clamp(0.0, 1.0);
            if coverage > 0.0 {
                image.blend_pixel(x, y, color.with_alpha(color.alpha * coverage), None);
            }
        }
    }
}

fn fill_rounded(
    image: &mut QRasterImage,
    rect: QRectF,
    radius: f32,
    color: QColor,
    clip: Option<QRectF>,
) {
    let left = rect.left().round() as i32;
    let top = rect.top().round() as i32;
    let right = rect.right().round() as i32;
    let bottom = rect.bottom().round() as i32;
    if right <= left || bottom <= top {
        return;
    }
    let radius = radius
        .round()
        .max(0.0)
        .min((right - left).min(bottom - top) as f32 / 2.0);
    let Some(bounds) = intersect(
        QRectF::new(
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ),
        clip,
    ) else {
        return;
    };
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            if inside_rounded_rect(
                x as f32 + 0.5,
                y as f32 + 0.5,
                left,
                top,
                right,
                bottom,
                radius,
            ) {
                image.blend_pixel(x, y, color, None);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_linear_gradient(
    image: &mut QRasterImage,
    rect: QRectF,
    radius: f32,
    start: QColor,
    end: QColor,
    horizontal: bool,
    clip: Option<QRectF>,
) {
    let left = rect.left().round() as i32;
    let top = rect.top().round() as i32;
    let right = rect.right().round() as i32;
    let bottom = rect.bottom().round() as i32;
    if right <= left || bottom <= top {
        return;
    }
    let radius = radius
        .round()
        .max(0.0)
        .min((right - left).min(bottom - top) as f32 / 2.0);
    let Some(bounds) = intersect(
        QRectF::new(
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ),
        clip,
    ) else {
        return;
    };
    let span = if horizontal {
        (right - left).max(1) as f32
    } else {
        (bottom - top).max(1) as f32
    };
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        let column_t = |x: i32| {
            ((x as f32 + 0.5 - left as f32) / span).clamp(0.0, 1.0)
        };
        let row_color = start.lerp(end, ((y as f32 + 0.5 - top as f32) / span).clamp(0.0, 1.0));
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            if inside_rounded_rect(
                x as f32 + 0.5,
                y as f32 + 0.5,
                left,
                top,
                right,
                bottom,
                radius,
            ) {
                let color = if horizontal { start.lerp(end, column_t(x)) } else { row_color };
                image.blend_pixel(x, y, color, None);
            }
        }
    }
}

fn draw_pixmap(
    image: &mut QRasterImage,
    rect: QRectF,
    pixmap: &crate::gui::QPixmap,
    opacity: f32,
    clip: Option<QRectF>,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 || pixmap.width == 0 || pixmap.height == 0 {
        return;
    }
    let Some(bounds) = intersect(rect, clip) else {
        return;
    };
    let opacity = opacity.clamp(0.0, 1.0);
    for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
        let source_y = (((y as f32 + 0.5 - rect.top()) / rect.height) * pixmap.height as f32)
            .floor()
            .clamp(0.0, pixmap.height.saturating_sub(1) as f32) as u32;
        for x in bounds.left().floor() as i32..bounds.right().ceil() as i32 {
            let source_x = (((x as f32 + 0.5 - rect.left()) / rect.width) * pixmap.width as f32)
                .floor()
                .clamp(0.0, pixmap.width.saturating_sub(1) as f32)
                as u32;
            let [red, green, blue, alpha] = pixmap.pixel(source_x, source_y);
            if alpha > 0 {
                let color = QColor::rgba8(red, green, blue, alpha)
                    .with_alpha(alpha as f32 / 255.0 * opacity);
                image.blend_pixel(x, y, color, None);
            }
        }
    }
}

fn fill_rect(image: &mut QRasterImage, rect: QRectF, color: QColor, clip: Option<QRectF>) {
    let mut rect = rect;
    if rect.height <= 1.5 {
        rect.y = rect.y.round();
        rect.height = rect.height.round().max(1.0);
    }
    if rect.width <= 1.5 {
        rect.x = rect.x.round();
        rect.width = rect.width.round().max(1.0);
    }
    let Some(rect) = intersect(rect, clip) else {
        return;
    };
    let left = rect.left().floor() as i32;
    let top = rect.top().floor() as i32;
    let right = rect.right().ceil() as i32;
    let bottom = rect.bottom().ceil() as i32;
    for y in top..bottom {
        for x in left..right {
            image.blend_pixel(x, y, color, None);
        }
    }
}

fn draw_text(
    image: &mut QRasterImage,
    rect: QRectF,
    text: &str,
    font: &QFont,
    color: QColor,
    clip: Option<QRectF>,
) {
    if text.is_empty() {
        return;
    }
    let font = font.clone();
    let text = text.to_string();
    let color = color_to_rgba(color);
    let base = ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32;
    with_font_system(|system| {
        let mut buffer = Buffer::new(
            system,
            Metrics::new(font.pixel_size.max(1.0), font.pixel_size.max(1.0) * 1.3),
        );
        buffer.set_size(Some(1_000_000.0), Some(font.pixel_size.max(1.0) * 1.3));
        buffer.set_text(
            &text,
            &Attrs::new()
                .family(Family::Name(&font.family))
                .weight(cosmic_text::Weight(font.weight as u16))
                .style(if font.italic {
                    cosmic_text::Style::Italic
                } else {
                    cosmic_text::Style::Normal
                }),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(system, false);
        let mut swash = SwashCache::new();
        let runs: Vec<(_, Vec<_>)> = buffer
            .layout_runs()
            .map(|run| (run.line_y, run.glyphs.iter().cloned().collect()))
            .collect();
        for (line_y, glyphs) in runs {
            for glyph in glyphs {
                let physical = glyph.physical((rect.x, rect.y + line_y), 1.0);
                let origin = (physical.x, physical.y);
                let callback_color = cosmic_text::Color((u32::from(color[3]) << 24) | base);
                swash.with_pixels(system, physical.cache_key, callback_color, |x, y, c| {
                    let alpha = ((c.0 >> 24) as u8) as f32 / 255.0;
                    if alpha > 0.0 {
                        let tinted = QColor::rgba(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            alpha * (color[3] as f32 / 255.0),
                        );
                        image.blend_pixel(origin.0 + x, origin.1 + y, tinted, clip);
                    }
                });
            }
        }
    });
}

fn intersect(rect: QRectF, clip: Option<QRectF>) -> Option<QRectF> {
    let Some(clip) = clip else {
        return (!rect.is_empty()).then_some(rect);
    };
    let left = rect.left().max(clip.left());
    let top = rect.top().max(clip.top());
    let right = rect.right().min(clip.right());
    let bottom = rect.bottom().min(clip.bottom());
    (right > left && bottom > top).then_some(QRectF::new(left, top, right - left, bottom - top))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_rects_and_clip() {
        let image = rasterize(
            &[
                QPaintCommand::SetClip {
                    rect: Some(QRectF::new(0.0, 0.0, 4.0, 4.0)),
                },
                QPaintCommand::FillRect {
                    rect: QRectF::new(0.0, 0.0, 8.0, 8.0),
                    color: QColor::rgb(255, 0, 0),
                },
            ],
            8,
            8,
            QColor::rgb(0, 0, 0),
        );
        assert_eq!(image.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(image.pixel(3, 3), [255, 0, 0, 255]);
        assert_eq!(image.pixel(4, 4), [0, 0, 0, 255]);
    }

    #[test]
    fn dialog01_raster_compared_to_qt6() {
        use crate::gui::QPainter;
        use crate::paint::paint_widget_tree;
        use crate::style::QFusionStyle;
        use crate::widgets::load_ui;

        let fixture = include_str!("../../form-xml/tests/fixtures/test_dialog01.ui");
        let (mut tree, root) = load_ui(fixture).expect("loads ui");
        if let Some(line) = tree.find_widget("lineEdit_1") {
            tree.set_focus(line);
        }
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &QFusionStyle::default(), &mut painter);
        let image = rasterize(painter.commands(), 388, 190, QColor::rgb(0xef, 0xef, 0xef));
        let _ = image.save_png("/tmp/rust_dialog01.png");
        let reference = QRasterImage::from_png(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qt6_test_dialog01.png"
        ))
        .expect("reference png");
        assert_eq!(image.width, reference.width);
        assert_eq!(image.height, reference.height);

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
        let mean = total as f64 / count as f64;
        eprintln!(
            "mean channel diff {mean:.2}, pixels >30: {different} / {}",
            image.width * image.height
        );
        assert!(mean < 12.0, "mean diff {mean}");
        assert!(different < (image.width * image.height) as u64 / 8);
    }

    #[test]
    fn dialog02_raster_compared_to_qt6() {
        use crate::gui::QPainter;
        use crate::paint::paint_widget_tree;
        use crate::style::QFusionStyle;
        use crate::widgets::load_ui;

        let fixture = include_str!("../../../../server/priv/qt_forms/test_dialog02.ui");
        let (mut tree, root) = load_ui(fixture).expect("loads ui");
        if let Some(line) = tree.find_widget("lineEdit_1") {
            tree.set_focus(line);
        }
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &QFusionStyle::default(), &mut painter);
        let image = rasterize(painter.commands(), 444, 483, QColor::rgb(0xef, 0xef, 0xef));
        let _ = image.save_png("/tmp/rust_dialog02.png");
        let reference = QRasterImage::from_png(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qt6_test_dialog02.png"
        ))
        .expect("reference png");
        assert_eq!(image.width, reference.width);
        assert_eq!(image.height, reference.height);

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
        let mean = total as f64 / count as f64;
        eprintln!(
            "mean channel diff {mean:.2}, pixels >30: {different} / {}",
            image.width * image.height
        );
        assert!(mean < 12.0, "mean diff {mean}");
        assert!(different < (image.width * image.height) as u64 / 8);
    }

    #[test]
    fn scroll_table_raster_compared_to_qt6() {
        use crate::gui::QPainter;
        use crate::paint::paint_widget_tree;
        use crate::style::QFusionStyle;
        use crate::widgets::load_ui;

        let fixture = include_str!("../tests/fixtures/scroll_table.ui");
        let (mut tree, root) = load_ui(fixture).expect("loads ui");
        tree.focus_first(root);
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &QFusionStyle::default(), &mut painter);
        let image = rasterize(painter.commands(), 420, 240, QColor::rgb(0xef, 0xef, 0xef));
        let _ = image.save_png("/tmp/rust_scroll_table.png");
        let reference = QRasterImage::from_png(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qt6_scroll_table.png"
        ))
        .expect("reference png");
        assert_eq!(image.width, reference.width);
        assert_eq!(image.height, reference.height);

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
        let mean = total as f64 / count as f64;
        eprintln!(
            "scroll table mean channel diff {mean:.2}, pixels >30: {different} / {}",
            image.width * image.height
        );
        assert!(mean < 12.0, "mean diff {mean}");
        assert!(different < (image.width * image.height) as u64 / 8);
    }

    /// Paint a grid fixture and compare it with the Qt6 offscreen raster.
    fn compare_grid_raster_with_qt6(ui: &str, reference: &[u8], name: &str) {
        use crate::gui::QPainter;
        use crate::paint::paint_widget_tree;
        use crate::style::QFusionStyle;
        use crate::widgets::load_ui;

        let reference = QRasterImage::from_png_bytes(reference).expect("reference png");
        let (mut tree, root) = load_ui(ui).expect("loads ui");
        tree.focus_first(root);
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &QFusionStyle::default(), &mut painter);
        let image = rasterize(
            painter.commands(),
            reference.width,
            reference.height,
            QColor::rgb(0xef, 0xef, 0xef),
        );
        let _ = image.save_png(format!("/tmp/rust_{name}.png"));
        assert_eq!(image.width, reference.width);
        assert_eq!(image.height, reference.height);

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
        let mean = total as f64 / count as f64;
        eprintln!(
            "{name} mean channel diff {mean:.2}, pixels >30: {different} / {}",
            image.width * image.height
        );
        assert!(mean < 12.0, "{name} mean diff {mean}");
        assert!(
            different < (image.width * image.height) as u64 / 8,
            "{name} pixels >30: {different}"
        );
    }

    #[test]
    fn grid_2x2_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/grid_2x2.ui"),
            include_bytes!("../tests/fixtures/qt6_grid_2x2.png"),
            "grid_2x2",
        );
    }

    #[test]
    fn grid_spans_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/grid_spans.ui"),
            include_bytes!("../tests/fixtures/qt6_grid_spans.png"),
            "grid_spans",
        );
    }

    #[test]
    fn grid_nested_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/grid_nested.ui"),
            include_bytes!("../tests/fixtures/qt6_grid_nested.png"),
            "grid_nested",
        );
    }

    #[test]
    fn grid_policies_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/grid_policies.ui"),
            include_bytes!("../tests/fixtures/qt6_grid_policies.png"),
            "grid_policies",
        );
    }

    #[test]
    fn form_basic_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_basic.ui"),
            include_bytes!("../tests/fixtures/qt6_form_basic.png"),
            "form_basic",
        );
    }

    #[test]
    fn form_wrap_long_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_wrap_long.ui"),
            include_bytes!("../tests/fixtures/qt6_form_wrap_long.png"),
            "form_wrap_long",
        );
    }

    #[test]
    fn form_wrap_all_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_wrap_all.ui"),
            include_bytes!("../tests/fixtures/qt6_form_wrap_all.png"),
            "form_wrap_all",
        );
    }

    #[test]
    fn form_growth_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_growth.ui"),
            include_bytes!("../tests/fixtures/qt6_form_growth.png"),
            "form_growth",
        );
    }

    #[test]
    fn form_alignment_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_alignment.ui"),
            include_bytes!("../tests/fixtures/qt6_form_alignment.png"),
            "form_alignment",
        );
    }

    #[test]
    fn form_group_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/form_group.ui"),
            include_bytes!("../tests/fixtures/qt6_form_group.png"),
            "form_group",
        );
    }

    #[test]
    fn box_buttons_raster_compared_to_qt6() {
        compare_grid_raster_with_qt6(
            include_str!("../tests/fixtures/box_buttons.ui"),
            include_bytes!("../tests/fixtures/qt6_box_buttons.png"),
            "box_buttons",
        );
    }

    #[test]
    fn rasterizes_text_pixels() {
        let image = rasterize(
            &[QPaintCommand::DrawText {
                rect: QRectF::new(0.0, 0.0, 40.0, 20.0),
                text: "A".into(),
                font: QFont::default(),
                color: QColor::rgb(0, 0, 0),
            }],
            40,
            20,
            QColor::rgb(255, 255, 255),
        );
        assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[0] < 200));
    }
}
