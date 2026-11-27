use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::scroll::QOrientation;
use crate::widgets::QWidget;

/// Fusion-like `QProgressBar`: Base groove, Highlight chunk gradient, dual-color label.
pub(crate) fn paint_progress_bar(widget: &QWidget, painter: &mut QPainter) {
    let Some(bar) = widget.progress.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    let window = widget.palette.color(QPaletteRole::Window);
    let base = if widget.enabled {
        widget.palette.color(QPaletteRole::Base)
    } else {
        window
    };
    let mut outline = window.darker(140);
    let highlight = widget.palette.color(QPaletteRole::Highlight);

    // Groove: Fusion `adjusted(0,0,-1,-1)` + 0.5 pen translate covers the full
    // widget height; without subpixel AA we keep height=rect.height so the
    // bottom outline lands on the last row (probe h=24 → rows 0..23).
    let groove = QRectF::new(
        rect.x,
        rect.y,
        (rect.width - 1.0).max(1.0),
        rect.height.max(1.0),
    );
    painter.fill_rounded_rect(groove, 2.0, base);
    painter.stroke_rounded_rect(groove, 2.0, 1.0, outline);
    // Skip Fusion's inner topShadow line: on the GPU path a translucent
    // FillRect over the SDF AA fringe reads as a second dark border on top
    // only (bottom stays 1 px). Outline alone matches the live thickness.

    let horizontal = matches!(bar.orientation, QOrientation::Horizontal);
    let indeterminate = bar.is_indeterminate();
    let fraction = bar.fraction().unwrap_or(1.0);
    let complete = !indeterminate && bar.value >= bar.maximum && bar.maximum > bar.minimum;
    let show_chunk = indeterminate || bar.value > bar.minimum;

    if show_chunk {
        let (chunk_w, chunk_h) = if horizontal {
            let w = if indeterminate {
                groove.width
            } else {
                // Fusion: progressSteps * rect.width() / totalSteps, then width-1.
                (fraction * rect.width).floor().max(0.0)
            };
            ((w - 1.0).max(0.0), groove.height)
        } else {
            let h = if indeterminate {
                groove.height
            } else {
                (fraction * rect.height).floor().max(0.0)
            };
            (groove.width, (h - 1.0).max(0.0))
        };
        if chunk_w > 0.5 && chunk_h > 0.5 {
            let chunk = if horizontal {
                QRectF::new(rect.x, rect.y, chunk_w, chunk_h)
            } else {
                // Fill from the bottom upward (Qt default vertical progress).
                QRectF::new(
                    rect.x,
                    rect.y + (groove.height - chunk_h).max(0.0),
                    chunk_w,
                    chunk_h,
                )
            };

            let highlighted_outline = highlight.darker(140);
            if outline.q_gray() > highlighted_outline.q_gray() {
                outline = highlighted_outline;
            }

            let start = highlight.lighter(120);
            let end = highlight;
            // Incomplete chunk: widen fill under clip so the right end stays square
            // (Fusion avoids a round cap mid-bar).
            let fill = if !complete && !indeterminate && horizontal {
                QRectF::new(
                    chunk.x,
                    chunk.y,
                    (chunk.width + 2.0).min(groove.width),
                    chunk.height,
                )
            } else if !complete && !indeterminate && !horizontal {
                QRectF::new(
                    chunk.x,
                    (chunk.y - 2.0).max(rect.y),
                    chunk.width,
                    (chunk.height + 2.0).min(groove.height),
                )
            } else {
                chunk
            };

            painter.save();
            if !complete && !indeterminate {
                painter.set_clip_rect(Some(QRectF::new(
                    chunk.x,
                    chunk.y,
                    chunk.width + 0.5,
                    chunk.height,
                )));
            }
            if horizontal {
                painter.fill_linear_gradient_rect(fill, 2.0, start, end);
            } else {
                painter.fill_horizontal_gradient_rect(fill, 2.0, start, end);
            }
            // Stroke the clipped chunk outline (not the widened fill).
            painter.stroke_rounded_rect(chunk, 2.0, 1.0, outline);
            let inner = QRectF::new(
                chunk.x + 1.0,
                chunk.y + 1.0,
                (chunk.width - 2.0).max(0.0),
                (chunk.height - 2.0).max(0.0),
            );
            if inner.width > 0.0 && inner.height > 0.0 {
                painter.stroke_rounded_rect(inner, 1.0, 1.0, QColor::rgba8(255, 255, 255, 50));
            }
            painter.restore();
        }
    }

    if !bar.text_visible {
        return;
    }
    let text = bar.display_text();
    if text.is_empty() {
        return;
    }
    let metrics = QFontMetrics::new(&widget.font);
    let text_h = metrics.height();
    let text_w = metrics.horizontal_advance(&text);
    let text_x = rect.x + ((rect.width - text_w) / 2.0).max(0.0);
    let text_y = rect.y + ((rect.height - text_h) / 2.0).max(0.0);
    // Draw width through the bar edge so glyphon does not clip the last glyph.
    let text_rect = QRectF::new(text_x, text_y, (rect.right() - text_x).max(1.0), text_h);

    let text_color = widget.palette.color(QPaletteRole::Text);
    let hi_text = widget.palette.color(QPaletteRole::HighlightedText);

    if let Some(frac) = bar.fraction().filter(|f| *f > 0.0 && *f < 1.0) {
        let split = if horizontal {
            rect.x + (frac * rect.width).round()
        } else {
            rect.y + rect.height - (frac * rect.height).round()
        };
        // HighlightedText over the filled side; Text over the empty side.
        painter.save();
        if horizontal {
            painter.set_clip_rect(Some(QRectF::new(
                rect.x,
                rect.y,
                (split - rect.x).max(0.0),
                rect.height,
            )));
        } else {
            painter.set_clip_rect(Some(QRectF::new(
                rect.x,
                split,
                rect.width,
                (rect.bottom() - split).max(0.0),
            )));
        }
        painter.draw_text(text_rect, &text, hi_text);
        painter.restore();

        painter.save();
        if horizontal {
            painter.set_clip_rect(Some(QRectF::new(
                split,
                rect.y,
                (rect.right() - split).max(0.0),
                rect.height,
            )));
        } else {
            painter.set_clip_rect(Some(QRectF::new(
                rect.x,
                rect.y,
                rect.width,
                (split - rect.y).max(0.0),
            )));
        }
        painter.draw_text(text_rect, &text, text_color);
        painter.restore();
    } else if bar.fraction().is_some_and(|f| f >= 1.0) || indeterminate {
        painter.draw_text(text_rect, &text, hi_text);
    } else {
        painter.draw_text(text_rect, &text, text_color);
    }
}
