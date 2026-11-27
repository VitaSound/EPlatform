//! Local image assets for QSS `url(...)` / `border-image` and widget icons.
//!
//! Embedded bytes ship with wasm and native tests. On native hosts, unresolved
//! keys also try `client/assets/<key>` relative to this crate.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::gui::QPixmap;
use crate::raster::QRasterImage;

static EMBEDDED: &[(&str, &[u8])] = &[
    (
        "qss/border-frame.png",
        include_bytes!("../../../assets/qss/border-frame.png"),
    ),
    (
        "qss/brick.png",
        include_bytes!("../../../assets/qss/brick.png"),
    ),
];

static CACHE: OnceLock<Mutex<HashMap<String, Option<QPixmap>>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Option<QPixmap>>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Normalize a QSS/`:/` path to the asset registry key.
pub fn normalize_key(path: &str) -> String {
    let path = path.trim().trim_matches(|c| c == '"' || c == '\'');
    let path = path.strip_prefix(":/").unwrap_or(path);
    let path = path.strip_prefix("assets/").unwrap_or(path);
    let path = path.strip_prefix("./").unwrap_or(path);
    path.trim_start_matches('/').to_string()
}

fn decode_png(bytes: &[u8]) -> Option<QPixmap> {
    let image = QRasterImage::from_png_bytes(bytes).ok()?;
    Some(QPixmap::new(image.width, image.height, image.pixels))
}

fn assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn load_uncached(key: &str) -> Option<QPixmap> {
    for (name, bytes) in EMBEDDED {
        if *name == key {
            return decode_png(bytes);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = assets_root().join(key);
        if path.is_file() {
            let image = QRasterImage::from_png(path).ok()?;
            return Some(QPixmap::new(image.width, image.height, image.pixels));
        }
    }
    let _ = key;
    None
}

/// Resolve `url(...)` / pixmap path to an RGBA pixmap.
pub fn pixmap(path: &str) -> Option<QPixmap> {
    let key = normalize_key(path);
    if key.is_empty() {
        return None;
    }
    let mut guard = cache().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(cached) = guard.get(&key) {
        return cached.clone();
    }
    let loaded = load_uncached(&key);
    guard.insert(key, loaded.clone());
    loaded
}

/// Register or replace an in-memory asset (tests / dynamic packs).
pub fn insert(path: &str, pixmap: QPixmap) {
    let key = normalize_key(path);
    if key.is_empty() {
        return;
    }
    let mut guard = cache().lock().unwrap_or_else(|error| error.into_inner());
    guard.insert(key, Some(pixmap));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_resource_and_assets_prefixes() {
        assert_eq!(normalize_key(":/qss/frame.png"), "qss/frame.png");
        assert_eq!(normalize_key("assets/qss/frame.png"), "qss/frame.png");
        assert_eq!(normalize_key("\"qss/frame.png\""), "qss/frame.png");
    }

    #[test]
    fn embedded_border_frame_loads() {
        let pixmap = pixmap("qss/border-frame.png").expect("embedded");
        assert_eq!((pixmap.width, pixmap.height), (340, 340));
        // Rounded ornate corners leave (2,2) empty; sample inset of TL slice.
        let corner = pixmap.pixel(40, 4);
        let center = pixmap.pixel(170, 170);
        // Ornate frame: opaque ring, transparent content hole.
        assert_eq!(center[3], 0, "center must be transparent");
        assert!(corner[3] > 0, "corner ring opaque {corner:?}");
    }

    #[test]
    fn embedded_brick_tile_is_photo() {
        let pixmap = pixmap("qss/brick.png").expect("embedded");
        assert!(pixmap.width >= 128 && pixmap.height >= 128);
        let a = pixmap.pixel(8, 8);
        let b = pixmap.pixel(pixmap.width / 2, pixmap.height / 2);
        assert_ne!(a, b, "brick photo should vary");
    }
}
