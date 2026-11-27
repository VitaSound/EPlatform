use crate::gui::{QColor, QFontMetrics, QPainter, QPaletteRole, QPointF, QRectF};
use crate::scroll::QOrientation;
use crate::widgets::QWidget;

/// Fusion-like `QProgressBar`: Base groove, Highlight chunk gradient, dual-color label.
///
/// Groove frame matches `QLineEdit` / Fusion `translate(0.5)+adjusted(0,0,-1,-1)`:
/// sharp Base fill inset 1px, AA rounded outline radius 2 (outer corner stays Window).
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

    // Same path as LineEdit: sharp Base inset + centered SDF stroke on
    // `translate(0.5,0.5)` + `adjusted(0,0,-1,-1)`, radius 2.
    let fill_rect = QRectF::new(
        rect.x + 1.0,
        rect.y + 1.0,
        (rect.width - 2.0).max(1.0),
        (rect.height - 2.0).max(1.0),
    );
    let frame = QRectF::new(
        rect.x + 0.5,
        rect.y + 0.5,
        (rect.width - 1.0).max(1.0),
        (rect.height - 1.0).max(1.0),
    );
    painter.fill_rect(fill_rect, base);
    painter.stroke_rounded_rect(frame, 2.0, 1.0, outline);
    // Fusion/LineEdit inner top shadow (skip outer alpha-16 line above the bar).
    painter.draw_line(
        QPointF::new(rect.x + 2.0, rect.y + 1.0),
        QPointF::new(rect.right() - 2.0, rect.y + 1.0),
        1.0,
        QColor::rgba8(0, 0, 0, 18),
    );

    let horizontal = matches!(bar.orientation, QOrientation::Horizontal);
    let indeterminate = bar.is_indeterminate();
    let fraction = bar.fraction().unwrap_or(1.0);
    let complete = !indeterminate && bar.value >= bar.maximum && bar.maximum > bar.minimum;
    let show_chunk = indeterminate || bar.value > bar.minimum;

    if show_chunk {
        // Fusion CE_ProgressBarContents after translate(0.5): size is
        // (progressWidth-1) × (height-1) / (width-1) × (progressHeight-1).
        let (chunk_w, chunk_h) = if horizontal {
            let w = if indeterminate {
                rect.width
            } else {
                (fraction * rect.width).floor().max(0.0)
            };
            ((w - 1.0).max(0.0), (rect.height - 1.0).max(1.0))
        } else {
            let h = if indeterminate {
                rect.height
            } else {
                (fraction * rect.height).floor().max(0.0)
            };
            ((rect.width - 1.0).max(1.0), (h - 1.0).max(0.0))
        };
        if chunk_w > 0.5 && chunk_h > 0.5 {
            let chunk = if horizontal {
                QRectF::new(frame.x, frame.y, chunk_w, chunk_h)
            } else {
                // Fill from the bottom upward (Qt default vertical progress).
                QRectF::new(
                    frame.x,
                    frame.y + (frame.height - chunk_h).max(0.0),
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
                    (chunk.width + 2.0).min(frame.width),
                    chunk.height,
                )
            } else if !complete && !indeterminate && !horizontal {
                QRectF::new(
                    chunk.x,
                    (chunk.y - 2.0).max(frame.y),
                    chunk.width,
                    (chunk.height + 2.0).min(frame.height),
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
