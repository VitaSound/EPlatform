//! Pure canvas / pointer geometry for wasm (and tests).
//!
//! Keeps DPR, CSS box size, GPU texture limit, and pointer mapping in one
//! place so hit-testing cannot silently diverge from painting.

/// Chosen render mapping for a CSS box under a GPU max texture dimension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderMode {
    /// Logical layout size (units used for widgets / hit-testing).
    pub logical_w: f64,
    pub logical_h: f64,
    /// Surface scale: device pixels per logical pixel (`dpr` or `1.0`).
    /// Never below 1.0 — glyphon corrupts glyphs when `ui_scale < 1`.
    pub scale: f64,
    /// Multiply CSS pointer coords by this to get logical coords.
    /// `1.0` in modes 1/2; `< 1` in mode 3 when logical is shrunk to fit
    /// `max_dim` while the CSS box stays full-window.
    pub layout_from_css: f64,
    /// Deprecated: mode 3 no longer shrinks the CSS box. Always `false`.
    pub letterbox: bool,
}

/// Pick mode 1 / 2 / 3 from [docs/wasm-canvas-geometry.md] §3.4.
pub fn choose_render_mode(css_w: f64, css_h: f64, dpr: f64, max_dim: u32) -> RenderMode {
    let dpr = if dpr > 0.0 { dpr } else { 1.0 };
    let fits = |w: f64, h: f64| {
        w.round() as u32 <= max_dim && h.round() as u32 <= max_dim
    };
    let requested = (css_w * dpr, css_h * dpr);
    if fits(requested.0, requested.1) {
        RenderMode {
            logical_w: css_w,
            logical_h: css_h,
            scale: dpr,
            layout_from_css: 1.0,
            letterbox: false,
        }
    } else if fits(css_w, css_h) {
        RenderMode {
            logical_w: css_w,
            logical_h: css_h,
            scale: 1.0,
            layout_from_css: 1.0,
            letterbox: false,
        }
    } else {
        // Mode 3: CSS box stays 100% of the window (no letterbox). Shrink the
        // *logical* layout so backing fits `max_dim` at scale 1. Browser
        // upscales the smaller texture. Avoids `ui_scale < 1`, which makes
        // glyphon draw white / missing glyphs (menu, labels, title).
        let long_edge = css_w.max(css_h).max(1.0);
        let shrink = (max_dim as f64 / long_edge).clamp(1e-6, 1.0);
        RenderMode {
            logical_w: css_w * shrink,
            logical_h: css_h * shrink,
            scale: 1.0,
            layout_from_css: shrink,
            letterbox: false,
        }
    }
}

/// Surface backing size for a render mode.
pub fn surface_size(mode: RenderMode) -> (u32, u32) {
    let w = (mode.logical_w * mode.scale).round().max(1.0) as u32;
    let h = (mode.logical_h * mode.scale).round().max(1.0) as u32;
    (w, h)
}

/// winit-web delivers CSS×dpr; convert to logical layout pixels.
pub fn pointer_to_logical(
    raw_x: f64,
    raw_y: f64,
    dpr: f64,
    layout_from_css: f64,
) -> (f32, f32) {
    let dpr = if dpr > 0.0 { dpr } else { 1.0 };
    let map = if layout_from_css > 0.0 {
        layout_from_css
    } else {
        1.0
    };
    (
        ((raw_x / dpr) * map) as f32,
        ((raw_y / dpr) * map) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode1_full_dpr_when_under_limit() {
        let m = choose_render_mode(1280.0, 800.0, 1.5, 8192);
        assert!(!m.letterbox);
        assert!((m.scale - 1.5).abs() < 1e-9);
        assert!((m.layout_from_css - 1.0).abs() < 1e-9);
        assert_eq!(surface_size(m), (1920, 1200));
    }

    #[test]
    fn mode2_css_backing_when_dpr_exceeds_limit() {
        let m = choose_render_mode(1707.0, 907.0, 1.5, 2048);
        assert!(!m.letterbox);
        assert!((m.scale - 1.0).abs() < 1e-9);
        assert_eq!(surface_size(m), (1707, 907));
    }

    #[test]
    fn mode3_scale_one_shrinks_logical_not_css_box() {
        let m = choose_render_mode(3000.0, 1200.0, 1.5, 2048);
        assert!(!m.letterbox);
        assert!((m.scale - 1.0).abs() < 1e-9);
        assert!((m.layout_from_css - (2048.0 / 3000.0)).abs() < 1e-9);
        assert!((m.logical_w - 2048.0).abs() < 1e-6);
        let (w, h) = surface_size(m);
        assert_eq!(w, 2048);
        assert!(h <= 2048);
        assert!(m.scale >= 1.0);
    }

    #[test]
    fn mode3_pointer_maps_css_into_shrunk_logical() {
        let m = choose_render_mode(3000.0, 1200.0, 1.5, 2048);
        // CSS click at center of window, winit sends ×dpr.
        let css_x = 1500.0;
        let raw = css_x * 1.5;
        let (x, _) = pointer_to_logical(raw, 0.0, 1.5, m.layout_from_css);
        assert!((x as f64 - m.logical_w / 2.0).abs() < 1.0);
    }

    #[test]
    fn pointer_divides_by_dpr_not_ui_scale() {
        let (x, y) = pointer_to_logical(238.5, 57.0, 1.5, 1.0);
        assert!((x - 159.0).abs() < 0.1);
        assert!((y - 38.0).abs() < 0.1);
    }
}
