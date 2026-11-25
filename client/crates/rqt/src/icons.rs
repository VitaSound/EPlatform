//! Icon-list (`img24:N`) support.
//!
//! `tcs_qtgui` ships a 24-px icon strip (`IMAGELIST24.bmp` plus a disabled
//! variant). The BMPs are converted to PNG at build time and embedded, so the
//! same slicing works in native tests, the software rasterizer and `wasm32`.
//!
//! Qt indexes the strip from 1 (`1..=count`); index `0` is also accepted and
//! maps to the first cell (`invoke_mainwindow.cpp::resolveIconFromSpec`).

use std::sync::OnceLock;

use crate::gui::QPixmap;
use crate::raster::QRasterImage;

/// Cell size of `IMAGELIST24` in pixels.
pub const ICON_SIZE: u32 = 24;

static SHEET: OnceLock<Option<QRasterImage>> = OnceLock::new();
static SHEET_DISABLED: OnceLock<Option<QRasterImage>> = OnceLock::new();

fn sheet(disabled: bool) -> Option<&'static QRasterImage> {
    let cache = if disabled { &SHEET_DISABLED } else { &SHEET };
    cache
        .get_or_init(|| {
            let bytes: &[u8] = if disabled {
                include_bytes!("../../../assets/icons/IMAGELIST24_DISABLED.png")
            } else {
                include_bytes!("../../../assets/icons/IMAGELIST24.png")
            };
            QRasterImage::from_png_bytes(bytes).ok()
        })
        .as_ref()
}

/// Parse `img24:N` / `img24d:N` into `(disabled, zero_based_index)`.
pub fn spec_index(spec: &str) -> Option<(bool, usize)> {
    let spec = spec.trim();
    let (disabled, raw) = if let Some(raw) = spec.strip_prefix("img24d:") {
        (true, raw)
    } else if let Some(raw) = spec.strip_prefix("img24:") {
        (false, raw)
    } else {
        return None;
    };

    let mut index: i64 = raw.trim().parse().ok()?;
    if index > 0 {
        index -= 1;
    }
    if index < 0 {
        return None;
    }
    Some((disabled, index as usize))
}

fn slice(spec: &str) -> Option<QRasterImage> {
    let (disabled, index) = spec_index(spec)?;
    let sheet = sheet(disabled)?;
    if sheet.height < ICON_SIZE {
        return None;
    }
    let count = (sheet.width / ICON_SIZE) as usize;
    if index >= count {
        return None;
    }

    let mut image = QRasterImage::new(ICON_SIZE, ICON_SIZE);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let [red, green, blue, _] = sheet.pixel(index as u32 * ICON_SIZE + x, y);
            // Qt's `iconFromSprite` makes the white background transparent.
            let alpha = if red == 255 && green == 255 && blue == 255 {
                0
            } else {
                255
            };
            let target = ((y * ICON_SIZE + x) * 4) as usize;
            image.pixels[target..target + 4].copy_from_slice(&[red, green, blue, alpha]);
        }
    }
    Some(image)
}

/// Slice `img24:N` into a pixmap; `None` for other specs or bad indexes.
pub fn pixmap(spec: &str) -> Option<QPixmap> {
    let image = slice(spec)?;
    Some(QPixmap::new(image.width, image.height, image.pixels))
}

/// Slice `img24:N` and encode it as PNG (used for the browser favicon).
pub fn png_bytes(spec: &str) -> Option<Vec<u8>> {
    slice(spec)?.to_png_bytes().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_index_is_one_based_like_qt() {
        assert_eq!(spec_index("img24:1"), Some((false, 0)));
        assert_eq!(spec_index("img24:2"), Some((false, 1)));
        assert_eq!(spec_index("img24:0"), Some((false, 0)));
        assert_eq!(spec_index("img24d:3"), Some((true, 2)));
        assert_eq!(spec_index("img24:"), None);
        assert_eq!(spec_index("img24:abc"), None);
        assert_eq!(spec_index("img24d:-1"), None);
        assert_eq!(spec_index("icons/icon_001.svg"), None);
    }

    #[test]
    fn sprite_slices_24px_cells_and_clears_white_background() {
        let pixmap = pixmap("img24:22").expect("icon 22 exists");
        assert_eq!((pixmap.width, pixmap.height), (ICON_SIZE, ICON_SIZE));

        // The strip has a white background; Qt makes it transparent.
        assert_eq!(pixmap.pixel(0, 0)[3], 0);
        assert!(pixmap.pixels.chunks_exact(4).any(|pixel| pixel[3] == 255));
    }

    #[test]
    fn disabled_sheet_is_available() {
        let normal = pixmap("img24:22").expect("normal icon");
        let disabled = pixmap("img24d:22").expect("disabled icon");
        assert_eq!((disabled.width, disabled.height), (ICON_SIZE, ICON_SIZE));
        assert_ne!(normal.pixels, disabled.pixels);
    }

    #[test]
    fn out_of_range_and_foreign_specs_return_none() {
        assert!(pixmap("img24:99999").is_none());
        assert!(pixmap(":/icons/icon_001.svg").is_none());
        assert!(png_bytes("img24:1").is_some());
    }
}
