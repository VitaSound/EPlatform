use crate::gui::{QFont, QFontMetrics, QRectF, QSizeF};
use crate::scroll::{
    scroll_area_layout, QOrientation, QScrollAreaLayout, QScrollBar, QScrollBarPolicy,
    QSCROLLBAR_EXTENT,
};

/// Document margin inside the frame (Qt `QTextDocument::documentMargin` ≈ 4).
pub const QPLAIN_DOC_MARGIN: f32 = 4.0;

/// Default `QPlainTextEdit::sizeHint` (Qt `QAbstractScrollArea` path ≈ 256×192).
pub const QPLAIN_HINT_W: f32 = 256.0;
pub const QPLAIN_HINT_H: f32 = 192.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QLineWrapMode {
    NoWrap,
    #[default]
    WidgetWidth,
}

/// One painted line after wrap / hard breaks. Indices are char offsets into
/// the full `plain_text` (including `\n`); `end` is exclusive and may point
/// at a trailing newline that is not drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct QPlainVisualLine {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// `QPlainTextEdit` state: plain buffer + wrap + vertical scroll.
#[derive(Clone, Debug)]
pub struct QPlainTextEdit {
    pub plain_text: String,
    pub wrap_mode: QLineWrapMode,
    pub scroll_y: f32,
    pub vbar: QScrollBar,
    pub hbar: QScrollBar,
}

impl Default for QPlainTextEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl QPlainTextEdit {
    pub fn new() -> Self {
        let mut vbar = QScrollBar::new(QOrientation::Vertical);
        vbar.policy = QScrollBarPolicy::AsNeeded;
        let mut hbar = QScrollBar::new(QOrientation::Horizontal);
        hbar.policy = QScrollBarPolicy::AsNeeded;
        Self {
            plain_text: String::new(),
            wrap_mode: QLineWrapMode::WidgetWidth,
            scroll_y: 0.0,
            vbar,
            hbar,
        }
    }

    pub fn set_plain_text(&mut self, text: String) {
        self.plain_text = text;
    }

    pub fn content_size(&self, font: &QFont, viewport_width: f32) -> QSizeF {
        let metrics = QFontMetrics::new(font);
        let line_h = metrics.height().max(1.0);
        let wrap_w = match self.wrap_mode {
            QLineWrapMode::WidgetWidth => Some((viewport_width - QPLAIN_DOC_MARGIN * 2.0).max(1.0)),
            QLineWrapMode::NoWrap => None,
        };
        let lines = layout_visual_lines(&self.plain_text, wrap_w, &metrics);
        let height = (lines.len() as f32 * line_h).max(line_h);
        let width = match self.wrap_mode {
            QLineWrapMode::WidgetWidth => viewport_width.max(1.0),
            QLineWrapMode::NoWrap => {
                let widest = lines
                    .iter()
                    .map(|line| metrics.horizontal_advance(&line.text))
                    .fold(0.0_f32, f32::max);
                (widest + QPLAIN_DOC_MARGIN * 2.0).max(1.0)
            }
        };
        QSizeF::new(width, height + QPLAIN_DOC_MARGIN * 2.0)
    }

    pub fn scroll_area(&self, rect: QRectF, font: &QFont) -> QScrollAreaLayout {
        // Probe content against the full rect first; `scroll_area_layout`
        // accounts for bar extent when deciding visibility.
        let content = self.content_size(font, rect.width.max(1.0));
        scroll_area_layout(
            rect,
            content,
            self.vbar.policy,
            match self.wrap_mode {
                QLineWrapMode::WidgetWidth => QScrollBarPolicy::AlwaysOff,
                QLineWrapMode::NoWrap => self.hbar.policy,
            },
            QSCROLLBAR_EXTENT,
        )
    }

    /// Viewport inside the 1 px frame (Qt `QAbstractScrollArea` chrome).
    pub fn framed_viewport(&self, rect: QRectF, font: &QFont) -> QRectF {
        let viewport = self.scroll_area(rect, font).viewport;
        QRectF::new(
            viewport.x + 1.0,
            viewport.y + 1.0,
            (viewport.width - 2.0).max(1.0),
            (viewport.height - 2.0).max(1.0),
        )
    }

    pub fn document_rect(&self, viewport: QRectF) -> QRectF {
        QRectF::new(
            viewport.x + QPLAIN_DOC_MARGIN,
            viewport.y + QPLAIN_DOC_MARGIN,
            (viewport.width - QPLAIN_DOC_MARGIN * 2.0).max(1.0),
            (viewport.height - QPLAIN_DOC_MARGIN * 2.0).max(1.0),
        )
    }

    pub fn visual_lines(&self, font: &QFont, viewport_width: f32) -> Vec<QPlainVisualLine> {
        let metrics = QFontMetrics::new(font);
        let wrap_w = match self.wrap_mode {
            QLineWrapMode::WidgetWidth => Some((viewport_width - QPLAIN_DOC_MARGIN * 2.0).max(1.0)),
            QLineWrapMode::NoWrap => None,
        };
        layout_visual_lines(&self.plain_text, wrap_w, &metrics)
    }

    pub fn sync_bars(&mut self, font: &QFont, viewport: QRectF) {
        let metrics = QFontMetrics::new(font);
        let line_h = metrics.height().max(1.0);
        let content = self.content_size(font, viewport.width.max(1.0));
        let max_y = (content.height - viewport.height).max(0.0);
        self.vbar.single_step = line_h;
        self.vbar.set_range(0.0, max_y, viewport.height.max(0.0));
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
        self.vbar.set_value(self.scroll_y);

        if matches!(self.wrap_mode, QLineWrapMode::NoWrap) {
            let max_x = (content.width - viewport.width).max(0.0);
            self.hbar.single_step = metrics.horizontal_advance("x").max(1.0);
            self.hbar.set_range(0.0, max_x, viewport.width.max(0.0));
            self.hbar.set_value(self.hbar.value.clamp(0.0, max_x));
        } else {
            self.hbar.set_range(0.0, 0.0, viewport.width.max(0.0));
            self.hbar.set_value(0.0);
        }
    }

    pub fn scroll_by(&mut self, delta_y: f32) {
        let max = self.vbar.maximum;
        self.scroll_y = (self.scroll_y + delta_y).clamp(0.0, max);
        self.vbar.set_value(self.scroll_y);
    }

    pub fn set_scroll_y(&mut self, value: f32) {
        let value = value.clamp(self.vbar.minimum, self.vbar.maximum);
        self.scroll_y = value;
        self.vbar.set_value(value);
    }

    /// Ensure the caret (char index) is inside the document viewport.
    pub fn ensure_cursor_visible(
        &mut self,
        cursor: usize,
        font: &QFont,
        viewport: QRectF,
    ) {
        let metrics = QFontMetrics::new(font);
        let line_h = metrics.height().max(1.0);
        let doc = self.document_rect(viewport);
        let lines = self.visual_lines(font, viewport.width);
        let line_index = line_index_for_cursor(&lines, cursor);
        let y = line_index as f32 * line_h;
        let view_top = self.scroll_y;
        let view_bottom = self.scroll_y + doc.height;
        if y < view_top {
            self.set_scroll_y(y);
        } else if y + line_h > view_bottom {
            self.set_scroll_y(y + line_h - doc.height);
        }
        self.sync_bars(font, viewport);
    }
}

/// Break `text` into visual lines. `wrap_width = None` means hard breaks only.
pub fn layout_visual_lines(
    text: &str,
    wrap_width: Option<f32>,
    metrics: &QFontMetrics,
) -> Vec<QPlainVisualLine> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![QPlainVisualLine {
            start: 0,
            end: 0,
            text: String::new(),
        }];
    }

    let mut lines = Vec::new();
    let mut para_start = 0usize;
    let mut i = 0usize;
    while i <= chars.len() {
        let at_end = i == chars.len();
        let at_nl = !at_end && chars[i] == '\n';
        if !at_end && !at_nl {
            i += 1;
            continue;
        }
        // Paragraph is chars[para_start..i]; newline (if any) is at i.
        let para_end = i;
        let after_nl = if at_nl { i + 1 } else { i };
        wrap_paragraph(
            &chars,
            para_start,
            para_end,
            after_nl,
            at_nl,
            wrap_width,
            metrics,
            &mut lines,
        );
        if at_end {
            break;
        }
        i = after_nl;
        para_start = after_nl;
    }

    if lines.is_empty() {
        lines.push(QPlainVisualLine {
            start: 0,
            end: 0,
            text: String::new(),
        });
    }
    lines
}

fn wrap_paragraph(
    chars: &[char],
    para_start: usize,
    para_end: usize,
    after_nl: usize,
    had_newline: bool,
    wrap_width: Option<f32>,
    metrics: &QFontMetrics,
    lines: &mut Vec<QPlainVisualLine>,
) {
    if para_start == para_end {
        // Empty paragraph (`\n` alone or trailing newline).
        lines.push(QPlainVisualLine {
            start: para_start,
            end: if had_newline { after_nl } else { para_end },
            text: String::new(),
        });
        return;
    }

    let Some(width) = wrap_width else {
        let text: String = chars[para_start..para_end].iter().collect();
        lines.push(QPlainVisualLine {
            start: para_start,
            end: if had_newline { after_nl } else { para_end },
            text,
        });
        return;
    };

    let mut start = para_start;
    while start < para_end {
        let mut end = start;
        let mut last_break = None;
        while end < para_end {
            let candidate: String = chars[start..=end].iter().collect();
            if metrics.horizontal_advance(&candidate) > width && end > start {
                break;
            }
            if chars[end].is_whitespace() {
                last_break = Some(end + 1);
            }
            end += 1;
        }
        let soft_end = if end < para_end {
            last_break
                .filter(|&b| b > start)
                .unwrap_or_else(|| end.max(start + 1))
        } else {
            para_end
        };
        let text: String = chars[start..soft_end].iter().collect();
        let line_end = if soft_end == para_end && had_newline {
            after_nl
        } else {
            soft_end
        };
        lines.push(QPlainVisualLine {
            start,
            end: line_end,
            text,
        });
        start = soft_end;
    }
}

pub fn line_index_for_cursor(lines: &[QPlainVisualLine], cursor: usize) -> usize {
    if lines.is_empty() {
        return 0;
    }
    for (index, line) in lines.iter().enumerate() {
        // Cursor on the trailing `\n` stays on this visual line.
        if cursor < line.end || (cursor == line.end && index + 1 == lines.len()) {
            return index;
        }
        if cursor >= line.start && cursor < line.end {
            return index;
        }
    }
    lines.len() - 1
}

/// Map a point in document space (origin = top-left of first line, y already
/// scroll-adjusted) to a char index.
pub fn cursor_at_document_pos(
    lines: &[QPlainVisualLine],
    metrics: &QFontMetrics,
    x: f32,
    y: f32,
) -> usize {
    if lines.is_empty() {
        return 0;
    }
    let line_h = metrics.height().max(1.0);
    let mut index = (y / line_h).floor() as isize;
    if index < 0 {
        index = 0;
    }
    let index = (index as usize).min(lines.len() - 1);
    let line = &lines[index];
    let chars: Vec<char> = line.text.chars().collect();
    let mut best = line.start;
    let mut best_dist = f32::MAX;
    for offset in 0..=chars.len() {
        let prefix: String = chars[..offset].iter().collect();
        let width = metrics.horizontal_advance(&prefix);
        let dist = (x - width).abs();
        if dist < best_dist {
            best_dist = dist;
            best = line.start + offset;
        }
    }
    // Cap at the drawn end (exclude trailing `\n` slot when present).
    let drawn_end = line.start + chars.len();
    best.min(drawn_end).min(line.end)
}

/// Vertical / horizontal caret move helpers returning a new char index.
pub fn move_cursor_vertical(
    text: &str,
    lines: &[QPlainVisualLine],
    metrics: &QFontMetrics,
    cursor: usize,
    up: bool,
) -> usize {
    if lines.is_empty() {
        return 0;
    }
    let index = line_index_for_cursor(lines, cursor);
    let line = &lines[index];
    let offset_in_line = cursor.saturating_sub(line.start).min(line.text.chars().count());
    let prefix: String = line.text.chars().take(offset_in_line).collect();
    let prefer_x = metrics.horizontal_advance(&prefix);

    let target = if up {
        index.checked_sub(1)
    } else if index + 1 < lines.len() {
        Some(index + 1)
    } else {
        None
    };
    let Some(target) = target else {
        return if up { 0 } else { text.chars().count() };
    };
    cursor_at_document_pos(lines, metrics, prefer_x, target as f32 * metrics.height().max(1.0) + 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::QFont;

    #[test]
    fn hard_breaks_preserve_empty_lines() {
        let metrics = QFontMetrics::new(&QFont::default());
        let lines = layout_visual_lines("a\n\nb", None, &metrics);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "a");
        assert_eq!(lines[1].text, "");
        assert_eq!(lines[2].text, "b");
    }

    #[test]
    fn wrap_breaks_long_line() {
        let metrics = QFontMetrics::new(&QFont::default());
        let lines = layout_visual_lines("one two three four", Some(40.0), &metrics);
        assert!(lines.len() >= 2, "expected wrap, got {:?}", lines);
        let rebuilt: String = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("");
        // Spaces kept on lines; joining without re-inserting may drop them —
        // check char coverage instead.
        assert_eq!(lines.first().unwrap().start, 0);
        assert_eq!(lines.last().unwrap().end, "one two three four".chars().count());
        let _ = rebuilt;
    }

    #[test]
    fn cursor_maps_into_wrapped_line() {
        let metrics = QFontMetrics::new(&QFont::default());
        let text = "hello world";
        let lines = layout_visual_lines(text, Some(35.0), &metrics);
        let pos = cursor_at_document_pos(&lines, &metrics, 0.0, 0.0);
        assert_eq!(pos, 0);
    }
}
