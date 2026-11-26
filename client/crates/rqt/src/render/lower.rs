use crate::gui::{QColor, QPaintCommand, QRectF};

use super::geometry::{rounded_horizontal_span_at_y, rounded_vertical_span_at_x};
use super::primitive::RenderPrimitive;

/// One axis-aligned fragment produced by expanding a primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct LoweredRect {
    pub rect: QRectF,
    pub color: QColor,
}

/// Map paint commands to coarse primitives (clip stack not applied here).
pub fn lower_commands(commands: &[QPaintCommand], _scale: f32) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    for command in commands {
        match command {
            QPaintCommand::FillRect { rect, color } => out.push(RenderPrimitive::SolidRect {
                rect: *rect,
                radius: 0.0,
                color: *color,
            }),
            QPaintCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => out.push(RenderPrimitive::SolidRect {
                rect: *rect,
                radius: *radius,
                color: *color,
            }),
            QPaintCommand::FillLinearGradientRect {
                rect,
                radius,
                start,
                end,
                horizontal,
            } => out.push(RenderPrimitive::LinearGradient {
                rect: *rect,
                radius: *radius,
                start: *start,
                end: *end,
                horizontal: *horizontal,
            }),
            QPaintCommand::StrokeRoundedRect {
                rect,
                radius,
                width,
                color,
            } => out.push(RenderPrimitive::RoundedStroke {
                rect: *rect,
                radius: *radius,
                width: *width,
                color: *color,
            }),
            QPaintCommand::FillEllipse { rect, color } => out.push(RenderPrimitive::Ellipse {
                rect: *rect,
                stroke: 0.0,
                color: *color,
            }),
            QPaintCommand::StrokeEllipse {
                rect,
                width,
                color,
            } => out.push(RenderPrimitive::Ellipse {
                rect: *rect,
                stroke: *width,
                color: *color,
            }),
            QPaintCommand::FillTriangle { a, b, c, color } => {
                out.push(RenderPrimitive::Triangle {
                    a: *a,
                    b: *b,
                    c: *c,
                    color: *color,
                })
            }
            QPaintCommand::DrawPixmap {
                rect,
                pixmap,
                opacity,
            } => out.push(RenderPrimitive::Pixmap {
                rect: *rect,
                pixmap: pixmap.clone(),
                opacity: *opacity,
            }),
            _ => {}
        }
    }
    out
}

/// Device-stepped linear gradient bands, clipped to `radius` when positive.
pub fn expand_linear_gradient(
    rect: QRectF,
    radius: f32,
    start: QColor,
    end: QColor,
    horizontal: bool,
    scale: f32,
) -> Vec<LoweredRect> {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let left = rect.left();
    let top = rect.top();
    let right = rect.right();
    let bottom = rect.bottom();
    let radius = radius
        .max(0.0)
        .min(rect.width / 2.0)
        .min(rect.height / 2.0);

    let (from, to) = if horizontal {
        ((left * scale).round() as i32, (right * scale).round() as i32)
    } else {
        ((top * scale).round() as i32, (bottom * scale).round() as i32)
    };
    let span = (to - from).max(1) as f32;
    let mut rects = Vec::new();

    for device in from..to {
        let t = ((device as f32 + 0.5 - from as f32) / span).clamp(0.0, 1.0);
        let color = start.lerp(end, t);
        let offset = device as f32 / scale;
        let thickness = 1.0 / scale;

        if horizontal {
            let band_left = offset;
            let band_right = offset + thickness;
            if radius <= 0.0 {
                rects.push(LoweredRect {
                    rect: QRectF::new(band_left, top, band_right - band_left, rect.height),
                    color,
                });
            } else {
                // One vertical span per device column — O(width), not O(width×height).
                let Some((y0, y1)) =
                    rounded_vertical_span_at_x(offset, left, top, right, bottom, radius)
                else {
                    continue;
                };
                rects.push(LoweredRect {
                    rect: QRectF::new(band_left, y0, band_right - band_left, y1 - y0),
                    color,
                });
            }
        } else if radius <= 0.0 {
            rects.push(LoweredRect {
                rect: QRectF::new(left, offset, rect.width, thickness),
                color,
            });
        } else {
            let Some((x0, x1)) =
                rounded_horizontal_span_at_y(offset, left, top, right, bottom, radius)
            else {
                continue;
            };
            rects.push(LoweredRect {
                rect: QRectF::new(x0, offset, x1 - x0, thickness),
                color,
            });
        }
    }
    rects
}

/// Rounded stroke outline matching `raster::fill_rounded_border`.
///
/// Emits O(height) horizontal spans (outer \ inner), never per-pixel 1×1 rects.
/// Per-pixel expansion blows past `MAX_RECTS` once a few rounded buttons are
/// on screen (e.g. after maximizing), leaving the clear color in the rest of
/// the frame — typically a black title bar / broken chrome.
pub fn expand_stroke_rounded_border(
    rect: QRectF,
    radius: f32,
    width: f32,
    color: QColor,
) -> Vec<LoweredRect> {
    let left = rect.left().round() as i32;
    let top = rect.top().round() as i32;
    let right = rect.right().round() as i32;
    let bottom = rect.bottom().round() as i32;
    if right <= left || bottom <= top {
        return Vec::new();
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

    if radius <= 0.0 && stroke > 0 {
        return expand_stroke_rect_border(rect, width, color);
    }

    let left_f = left as f32;
    let top_f = top as f32;
    let right_f = right as f32;
    let bottom_f = bottom as f32;
    let inner_left_f = inner_left as f32;
    let inner_top_f = inner_top as f32;
    let inner_right_f = inner_right as f32;
    let inner_bottom_f = inner_bottom as f32;
    let has_inner = inner_right > inner_left && inner_bottom > inner_top;

    let mut rects = Vec::with_capacity(((bottom - top) as usize).saturating_mul(2));
    for y in top..bottom {
        let yf = y as f32;
        let Some((ox0, ox1)) =
            rounded_horizontal_span_at_y(yf, left_f, top_f, right_f, bottom_f, radius)
        else {
            continue;
        };

        let in_inner_rows = has_inner && y >= inner_top && y < inner_bottom;
        if !in_inner_rows {
            rects.push(LoweredRect {
                rect: QRectF::new(ox0, yf, ox1 - ox0, 1.0),
                color,
            });
            continue;
        }

        let Some((ix0, ix1)) = rounded_horizontal_span_at_y(
            yf,
            inner_left_f,
            inner_top_f,
            inner_right_f,
            inner_bottom_f,
            inner_radius,
        ) else {
            rects.push(LoweredRect {
                rect: QRectF::new(ox0, yf, ox1 - ox0, 1.0),
                color,
            });
            continue;
        };

        if ix0 > ox0 {
            rects.push(LoweredRect {
                rect: QRectF::new(ox0, yf, ix0 - ox0, 1.0),
                color,
            });
        }
        if ox1 > ix1 {
            rects.push(LoweredRect {
                rect: QRectF::new(ix1, yf, ox1 - ix1, 1.0),
                color,
            });
        }
    }
    rects
}

fn expand_stroke_rect_border(rect: QRectF, width: f32, color: QColor) -> Vec<LoweredRect> {
    let left = rect.left().round();
    let top = rect.top().round();
    let right = rect.right().round();
    let bottom = rect.bottom().round();
    let width = width.round().max(1.0);
    let push = |r: QRectF| LoweredRect { rect: r, color };
    vec![
        push(QRectF::new(left, top, right - left, width)),
        push(QRectF::new(left, bottom - width, right - left, width)),
        push(QRectF::new(left, top, width, bottom - top)),
        push(QRectF::new(right - width, top, width, bottom - top)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::QColor;

    #[test]
    fn rounded_stroke_is_linear_in_height_not_area() {
        let rect = QRectF::new(10.0, 20.0, 120.0, 40.0);
        let fragments =
            expand_stroke_rounded_border(rect, 6.0, 1.0, QColor::rgba8(0, 0, 0, 255));
        // Worst case ~2 spans per scanline; never near width×height.
        assert!(
            fragments.len() <= (rect.height as usize + 2) * 2,
            "got {} fragments for {}×{} stroke",
            fragments.len(),
            rect.width,
            rect.height
        );
        assert!(fragments.iter().all(|f| f.rect.height <= 1.0 + f32::EPSILON));
        assert!(fragments.iter().all(|f| f.rect.width >= 1.0 - f32::EPSILON));
    }

    #[test]
    fn horizontal_rounded_gradient_is_linear_in_width() {
        let rect = QRectF::new(0.0, 0.0, 80.0, 30.0);
        let start = QColor::rgba8(0, 0, 0, 255);
        let end = QColor::rgba8(255, 255, 255, 255);
        let bands = expand_linear_gradient(rect, 4.0, start, end, true, 1.0);
        assert_eq!(bands.len(), 80);
    }
}
