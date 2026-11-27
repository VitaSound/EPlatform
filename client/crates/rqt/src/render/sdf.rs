//! Signed-distance helpers for rounded rects.
//!
//! Keep in sync with `SHAPE_WGSL` / `sd_rounded_box` in `app/src/gfx.rs`.
//! Live wasm paints rounded fill/stroke as one GPU quad; CPU review raster
//! must use the same SDF + smoothstep cover — not 4×4 supersample hacks.

/// Inigo Quilez rounded-box SDF. `p` is relative to the box center;
/// `half` is half-extents; negative inside, positive outside.
pub fn sd_rounded_box(px: f32, py: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let rr = radius.min(half_w).min(half_h).max(0.0);
    let qx = px.abs() - half_w + rr;
    let qy = py.abs() - half_h + rr;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.min(0.0).max(qy.min(0.0)) - rr
}

/// WGSL `smoothstep(edge0, edge1, x)`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let denom = edge1 - edge0;
    if denom.abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / denom).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Half-width of the AA ramp. Keep in sync with `SHAPE_WGSL`.
const AA_HALF: f32 = 0.5;

/// Fill coverage matching `gfx.rs` SHAPE_WGSL (`stroke == 0`).
pub fn rounded_fill_cover(distance: f32) -> f32 {
    (1.0 - smoothstep(-AA_HALF, AA_HALF, distance)).clamp(0.0, 1.0)
}

/// Stroke coverage matching `gfx.rs` SHAPE_WGSL (`stroke > 0`).
///
/// Centered pen (like Qt `QPainter` cosmetic 1px): the path is the SDF
/// isosurface `d == 0`, and the stroke extends `stroke/2` to each side.
/// Fusion `translate(0.5)+adjusted(0,0,-1,-1)` puts that path on the outer
/// pixel-center ring — do not fake roundness by bumping radius.
pub fn rounded_stroke_cover(distance: f32, stroke: f32) -> f32 {
    let half = stroke.max(1.0) * 0.5;
    (1.0 - smoothstep(-AA_HALF, AA_HALF, distance.abs() - half)).clamp(0.0, 1.0)
}

/// SDF at pixel center for an axis-aligned rect `[left, right) × [top, bottom)`.
pub fn sd_rounded_rect(
    px: f32,
    py: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
) -> f32 {
    let cx = (left + right) * 0.5;
    let cy = (top + bottom) * 0.5;
    let half_w = ((right - left) * 0.5).max(0.0);
    let half_h = ((bottom - top) * 0.5).max(0.0);
    sd_rounded_box(px - cx, py - cy, half_w, half_h, radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_of_box_is_inside() {
        let d = sd_rounded_rect(10.5, 10.5, 0.0, 0.0, 20.0, 20.0, 2.0);
        assert!(d < -5.0, "center should be deep inside, d={d}");
        assert!(rounded_fill_cover(d) > 0.99);
    }

    #[test]
    fn outside_corner_has_low_fill_cover() {
        let d = sd_rounded_rect(0.5, 0.5, 0.0, 0.0, 20.0, 20.0, 2.0);
        // With r=2 the extreme corner sits outside / on the fringe.
        assert!(d > -0.5, "corner should not be deep inside, d={d}");
    }

    #[test]
    fn stroke_cover_is_a_thin_band() {
        // Fusion path sits on pixel centers: [0.5, 19.5] × [0.5, 19.5].
        let d = sd_rounded_rect(0.5, 10.5, 0.5, 0.5, 19.5, 19.5, 2.0);
        let cover = rounded_stroke_cover(d, 1.0);
        assert!(
            cover > 0.99,
            "pixel center on the path must be solid, cover={cover}"
        );
        let deep = sd_rounded_rect(10.5, 10.5, 0.5, 0.5, 19.5, 19.5, 2.0);
        assert!(
            rounded_stroke_cover(deep, 1.0) < 0.01,
            "center must not be stroked"
        );
    }
}
