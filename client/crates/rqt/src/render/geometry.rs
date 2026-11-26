//! Rounded-rectangle tests shared by lowering and raster-aligned strokes/fills.

/// True when `(x, y)` lies inside the axis-aligned rect with circular corners.
pub fn inside_rounded_rect(
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

/// Horizontal span of a rounded rect at integer scanline `y` (pixel centers at y+0.5).
pub fn rounded_horizontal_span_at_y(
    y: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
) -> Option<(f32, f32)> {
    if right <= left || bottom <= top {
        return None;
    }
    let radius = radius
        .max(0.0)
        .min((right - left) / 2.0)
        .min((bottom - top) / 2.0);
    if radius <= 0.0 {
        return Some((left, right));
    }
    let py = y + 0.5;
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
    (x1 > x0).then_some((x0, x1))
}

/// Vertical span of a rounded rect at integer column `x` (pixel centers at x+0.5).
pub fn rounded_vertical_span_at_x(
    x: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
) -> Option<(f32, f32)> {
    if right <= left || bottom <= top {
        return None;
    }
    let radius = radius
        .max(0.0)
        .min((right - left) / 2.0)
        .min((bottom - top) / 2.0);
    if radius <= 0.0 {
        return Some((top, bottom));
    }
    let px = x + 0.5;
    let dx = if px < left + radius {
        left + radius - px
    } else if px > right - radius {
        px - (right - radius)
    } else {
        0.0
    };
    let inset = if dx > 0.0 {
        radius - (radius * radius - dx * dx).max(0.0).sqrt()
    } else {
        0.0
    };
    let y0 = top + inset;
    let y1 = bottom - inset;
    (y1 > y0).then_some((y0, y1))
}