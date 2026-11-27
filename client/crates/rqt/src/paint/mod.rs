use crate::core::QVariant;
use crate::gui::{QColor, QFont, QFontMetrics, QPainter, QPaletteRole, QRectF};
use crate::qss::{
    QBackgroundRepeat, QBackgroundVAlign, QBorderImage, QBorderImageMode, QBorderSide,
    QStyleDeclarations, QStyleSheet, QTextAlign,
};
use crate::style::{
    QControlElement, QPixelMetric, QPrimitiveElement, QStyle, QStyleOption, QStyleState,
};
use crate::widgets::{
    resolve_widget_style, wrap_text_lines, QWidget, QWidgetId, QWidgetTree, QCOMBO_POPUP_ROW_H,
};

mod focus;
mod plain_text;
mod progress;
mod scroll;
mod slider;
mod spin;
mod table;
mod tabs;
mod toolbox;

pub use scroll::paint_scroll_bar;

use focus::paint_focus_indicators;
use plain_text::paint_plain_text_edit;
use progress::paint_progress_bar;
use scroll::paint_item_views;
use slider::paint_slider;
use spin::paint_spin_box;
use table::paint_table;
use tabs::paint_tab_widget;
use toolbox::paint_toolbox;


pub fn paint_widget_tree(
    tree: &QWidgetTree,
    root: QWidgetId,
    style: &dyn QStyle,
    painter: &mut QPainter,
) {
    paint_widget_tree_content(tree, root, style, painter);
    paint_combo_popups(tree, style, painter);
}

/// Paint only the widget tree, without top-level combo popups. The app paints
/// those after MDI/top-level chrome so they stay above the window frame.
pub fn paint_widget_tree_content(
    tree: &QWidgetTree,
    root: QWidgetId,
    style: &dyn QStyle,
    painter: &mut QPainter,
) {
    paint_widget(tree, root, style, painter);
}

pub fn paint_combo_popups(tree: &QWidgetTree, _style: &dyn QStyle, painter: &mut QPainter) {
    for (id, widget) in tree.iter_widgets() {
        if widget.combo_popup.is_none() {
            continue;
        }
        let Some(rect) = tree.combo_popup_rect(id) else {
            continue;
        };
        paint_combo_popup(tree, id, widget, rect, painter);
    }
}

fn paint_widget(
    tree: &QWidgetTree,
    widget_id: QWidgetId,
    style: &dyn QStyle,
    painter: &mut QPainter,
) {
    let Some(widget) = tree.widget(widget_id) else {
        return;
    };
    if !widget.visible {
        return;
    }
    painter.save();
    painter.set_clip_rect(Some(widget.geometry));
    painter.set_font(widget.font.clone());
    painter.set_palette(widget.palette.clone());

    let mut state = if widget.enabled {
        QStyleState::ENABLED
    } else {
        QStyleState::DISABLED
    };
    if widget.has_focus {
        state = state | QStyleState::FOCUSED | QStyleState::ACTIVE;
    }
    if widget.pressed {
        state = state | QStyleState::SUNKEN;
    }
    if widget.under_mouse {
        state = state | QStyleState::HOVER;
    }
    if property_bool(widget, "checked") {
        state = state | QStyleState::SELECTED;
    }
    if widget.default_button {
        state = state | QStyleState::DEFAULT;
    }
    let mut option =
        QStyleOption::new(widget.geometry, widget.palette.clone(), widget.font.clone());
    option.state = state;
    let qss = resolved_style(tree, widget_id);
    let item_qss = resolved_style_part(tree, widget_id, "item");
    let header_qss = resolved_style_part(tree, widget_id, "section");

    let tab_page = widget.parent.is_some_and(|parent| {
        tree.widget(parent)
            .is_some_and(|node| node.class_name == "QTabWidget")
    });
    paint_widget_background(widget, style, &option, painter, &qss, tab_page);
    if widget.class_name == "QSplitter" {
        paint_splitter_handles(tree, widget, painter);
    }
    if widget.class_name == "QTabWidget" {
        paint_tab_widget(widget, style, painter);
    }
    if widget.class_name == "QToolBox" {
        paint_toolbox(widget, style, painter);
    }
    if matches!(
        widget.class_name.as_str(),
        "QSpinBox" | "QDoubleSpinBox"
    ) {
        paint_spin_box(widget, style, &option, painter);
    }
    if widget.class_name == "QProgressBar" {
        paint_progress_bar(widget, painter);
    }
    if widget.class_name == "QSlider" {
        paint_slider(widget, style, &option, painter);
    }
    if widget.class_name == "QPlainTextEdit" {
        paint_plain_text_edit(widget, style, &option, painter, &qss);
    }
    paint_table(widget, painter, style, &qss, &item_qss, &header_qss);
    paint_item_views(widget, painter, style, &item_qss, &header_qss);
    paint_line_edit_selection(widget, painter, &qss);
    paint_widget_text(tree, widget_id, widget, painter, style, &option, &qss);
    paint_line_edit_caret(widget, painter, &qss);
    paint_focus_indicators(widget, painter, style, &qss);

    // Keep the widget's clip active while painting children: Qt clips a
    // child to every ancestor, so content that overflows a small form does
    // not paint over the frame or neighbouring windows.
    for child in &widget.children {
        paint_widget(tree, *child, style, painter);
    }
    painter.restore();
}

pub(crate) fn qss_font(widget: &QWidget, qss: &QStyleDeclarations) -> QFont {
    let mut font = widget.font.clone();
    if let Some(family) = &qss.font_family {
        font.family = family.clone();
    }
    if let Some(size) = qss.font_size {
        font.pixel_size = size;
    }
    if let Some(weight) = qss.font_weight {
        font.weight = weight;
    }
    if let Some(italic) = qss.font_italic {
        font.italic = italic;
    }
    font
}

pub(crate) fn resolved_style_part(tree: &QWidgetTree, widget_id: QWidgetId, part: &str) -> QStyleDeclarations {
    let mut sheets: Vec<&QStyleSheet> = Vec::new();
    let mut current = Some(widget_id);
    while let Some(id) = current {
        let Some(widget) = tree.widget(id) else {
            break;
        };
        if let Some(sheet) = &widget.style_sheet {
            sheets.push(sheet);
        }
        current = widget.parent;
    }
    sheets.reverse();
    let Some(widget) = tree.widget(widget_id) else {
        return QStyleDeclarations::default();
    };
    let mut declarations = QStyleDeclarations::default();
    for sheet in sheets {
        declarations.merge(&sheet.resolve_part(widget, part));
    }
    declarations
}

pub(crate) fn qss_frame_rect(widget: &QWidget, qss: &QStyleDeclarations) -> QRectF {
    let (left, top, right, bottom) = qss.margin.unwrap_or((0.0, 0.0, 0.0, 0.0));
    QRectF::new(
        widget.geometry.x + left,
        widget.geometry.y + top,
        (widget.geometry.width - left - right).max(0.0),
        (widget.geometry.height - top - bottom).max(0.0),
    )
}

pub(crate) fn qss_content_rect(
    widget: &QWidget,
    qss: &QStyleDeclarations,
    fallback: (f32, f32, f32, f32),
) -> QRectF {
    let frame = qss_frame_rect(widget, qss);
    let (left, top, right, bottom) = qss.padding.unwrap_or(fallback);
    QRectF::new(
        frame.x + left,
        frame.y + top,
        (frame.width - left - right).max(1.0),
        (frame.height - top - bottom).max(1.0),
    )
}

pub(crate) fn resolved_style(tree: &QWidgetTree, widget_id: QWidgetId) -> QStyleDeclarations {
    resolve_widget_style(tree, widget_id)
}

fn ancestor_qss_background(tree: &QWidgetTree, widget_id: QWidgetId) -> Option<QColor> {
    // Title-in-margin sits over the parent face, not the group box fill.
    let mut current = tree.widget(widget_id).and_then(|widget| widget.parent);
    while let Some(id) = current {
        let qss = resolved_style(tree, id);
        if let Some(bg) = qss.background {
            return Some(bg);
        }
        current = tree.widget(id).and_then(|widget| widget.parent);
    }
    None
}

fn paint_widget_background(
    widget: &QWidget,
    style: &dyn QStyle,
    option: &QStyleOption,
    painter: &mut QPainter,
    qss: &QStyleDeclarations,
    tab_page: bool,
) {
    let frame = qss_frame_rect(widget, qss);
    let mut frame_option = option.clone();
    frame_option.rect = frame;
    match widget.class_name.as_str() {
        "QDialog" => {
            painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Panel, &frame_option));
        }
        "QWidget" => {
            // Tab pages sit on Fusion's tab frame (Base-ish), not Window gray.
            if tab_page {
                let fill = widget.palette.color(QPaletteRole::Base).darker(101);
                painter.fill_rect(frame, fill);
            } else {
                painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Panel, &frame_option));
            }
        }
        "QGroupBox" => {
            // QSS `margin-top` + `border`: frame is the QSS box; title sits on
            // the top edge (drawn later). Skip Fusion inset + frame stroke —
            // the QSS border pass owns the outline.
            if qss.group_box_title_on_frame() {
                // no-op: border/background below
            } else {
                let metrics = QFontMetrics::new(&qss_font(widget, qss));
                let top_margin = metrics.height().max(14.0) + 3.0;
                frame_option.rect = QRectF::new(
                    frame.x,
                    frame.y + top_margin,
                    frame.width,
                    (frame.height - top_margin).max(0.0),
                );
                painter.draw_commands(
                    &style.draw_primitive(QPrimitiveElement::Frame, &frame_option),
                );
            }
        }
        "QLabel" => {}
        "QLineEdit" => {
            painter.draw_commands(&style.draw_control(QControlElement::LineEdit, &frame_option));
        }
        "QPushButton" | "QToolButton" => {
            painter.draw_commands(&style.draw_control(QControlElement::PushButton, &frame_option));
        }
        "QCheckBox" => {
            painter.draw_commands(&style.draw_control(QControlElement::CheckBox, &frame_option));
        }
        "QRadioButton" => {
            painter.draw_commands(&style.draw_control(QControlElement::RadioButton, &frame_option));
        }
        "QComboBox" => {
            painter.draw_commands(&style.draw_control(QControlElement::ComboBox, &frame_option));
        }
        "QTableWidget" | "QListWidget" | "QTreeWidget" => {
            // QSS `border` replaces Fusion's Mid frame; keep the Base fill only.
            if qss.replaces_native_frame() {
                let fill = if option.state.contains(QStyleState::ENABLED) {
                    widget.palette.color(QPaletteRole::Base)
                } else {
                    widget.palette.color(QPaletteRole::Window)
                };
                painter.fill_rect(frame, fill);
            } else {
                painter.draw_commands(
                    &style.draw_control(QControlElement::TableView, &frame_option),
                );
            }
        }
        "QSplitter" => {
            painter.fill_rect(frame, widget.palette.color(QPaletteRole::Window));
        }
        "QTabWidget" => {
            // Chrome is drawn in `paint_tab_widget` after the background pass.
        }
        "QToolBox" => {
            // Headers drawn in `paint_toolbox`; body is Window/Button fill.
            painter.fill_rect(frame, widget.palette.color(QPaletteRole::Button));
        }
        "QSpinBox" | "QDoubleSpinBox" => {
            // Frame/text/buttons drawn in `paint_spin_box`.
        }
        "QProgressBar" => {
            // Groove/chunk/label drawn in `paint_progress_bar`.
        }
        "QPlainTextEdit" => {
            // Frame/text/caret/scroll drawn in `paint_plain_text_edit`.
        }
        _ => {}
    }
    if let Some(gradient) = &qss.background_gradient {
        let start = gradient
            .stops
            .first()
            .map(|(_, color)| *color)
            .unwrap_or_else(|| QColor::rgba8(0, 0, 0, 0));
        let end = gradient
            .stops
            .last()
            .map(|(_, color)| *color)
            .unwrap_or(start);
        painter.fill_linear_gradient_rect(frame, qss.border_radius.unwrap_or(0.0), start, end);
    } else if let Some(background) = qss.background {
        let radius = qss.border_radius.unwrap_or(0.0);
        if radius > 0.0 {
            painter.fill_rounded_rect(frame, radius, background);
        } else {
            painter.fill_rect(frame, background);
        }
    }
    if let Some(path) = qss.background_image.as_deref() {
        if let Some(pixmap) = crate::assets::pixmap(path) {
            paint_background_image(painter, frame, &pixmap, qss);
        }
    }
    if let Some(border_image) = &qss.border_image {
        // Qt: border-image needs a positive border width; destination edge
        // sizes come from border-width (not from the source slice).
        if paint_border_image(painter, frame, border_image, qss) {
            return;
        }
    }
    if let Some(width) = qss.border_width {
        if width > 0.0 {
            let color = qss
                .border_color
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::WindowText));
            if let Some(radius) = qss.border_radius {
                painter.stroke_rounded_rect(frame, radius, width, color);
            } else {
                painter.stroke_rect(frame, width, color);
            }
        }
    }
    paint_qss_side_borders(widget, painter, qss);
}

/// Qt `background-image`: intrinsic size, `background-repeat` + `background-position`.
fn paint_background_image(
    painter: &mut QPainter,
    frame: QRectF,
    pixmap: &crate::gui::QPixmap,
    qss: &QStyleDeclarations,
) {
    let tile_w = (pixmap.width as f32).max(1.0);
    let tile_h = (pixmap.height as f32).max(1.0);
    let repeat = qss.background_repeat.unwrap_or(QBackgroundRepeat::Repeat);
    let position = qss.background_position.unwrap_or_default();
    let origin_x = align_background_x(frame, tile_w, position.horizontal);
    let origin_y = align_background_y(frame, tile_h, position.vertical);

    let (repeat_x, repeat_y) = match repeat {
        QBackgroundRepeat::Repeat => (true, true),
        QBackgroundRepeat::RepeatX => (true, false),
        QBackgroundRepeat::RepeatY => (false, true),
        QBackgroundRepeat::NoRepeat => (false, false),
    };

    let start_x = if repeat_x {
        let mut x = origin_x;
        while x > frame.x {
            x -= tile_w;
        }
        x
    } else {
        origin_x
    };
    let start_y = if repeat_y {
        let mut y = origin_y;
        while y > frame.y {
            y -= tile_h;
        }
        y
    } else {
        origin_y
    };
    let end_x = if repeat_x {
        frame.x + frame.width
    } else {
        (origin_x + tile_w).min(frame.x + frame.width)
    };
    let end_y = if repeat_y {
        frame.y + frame.height
    } else {
        (origin_y + tile_h).min(frame.y + frame.height)
    };

    let mut y = start_y;
    while y < end_y {
        if y + tile_h <= frame.y {
            y += tile_h;
            continue;
        }
        let mut x = start_x;
        while x < end_x {
            if x + tile_w <= frame.x {
                x += tile_w;
                continue;
            }
            let Some(dest) = QRectF::new(x, y, tile_w, tile_h).intersected(&frame) else {
                x += tile_w;
                continue;
            };
            let src_x = (dest.x - x).round().max(0.0) as u32;
            let src_y = (dest.y - y).round().max(0.0) as u32;
            let patch = pixmap.copy_rect(
                src_x,
                src_y,
                dest.width.round().max(1.0) as u32,
                dest.height.round().max(1.0) as u32,
            );
            painter.draw_pixmap(dest, patch, 1.0);
            if !repeat_x {
                break;
            }
            x += tile_w;
        }
        if !repeat_y {
            break;
        }
        y += tile_h;
    }
}

fn align_background_x(frame: QRectF, tile_w: f32, align: QTextAlign) -> f32 {
    match align {
        QTextAlign::Left => frame.x,
        QTextAlign::Center => frame.x + ((frame.width - tile_w) * 0.5),
        QTextAlign::Right => frame.x + frame.width - tile_w,
    }
}

fn align_background_y(frame: QRectF, tile_h: f32, align: QBackgroundVAlign) -> f32 {
    match align {
        QBackgroundVAlign::Top => frame.y,
        QBackgroundVAlign::Center => frame.y + ((frame.height - tile_h) * 0.5),
        QBackgroundVAlign::Bottom => frame.y + frame.height - tile_h,
    }
}

fn border_edge_widths(qss: &QStyleDeclarations) -> (f32, f32, f32, f32) {
    let base = qss.border_width.unwrap_or(0.0);
    let width = |side: QBorderSide| {
        qss.side_border(side)
            .and_then(|decl| decl.width)
            .unwrap_or(base)
            .max(0.0)
    };
    (
        width(QBorderSide::Top),
        width(QBorderSide::Right),
        width(QBorderSide::Bottom),
        width(QBorderSide::Left),
    )
}

/// Nine-slice `border-image` (Qt: corners as-is; edges stretch or repeat).
fn paint_border_image(
    painter: &mut QPainter,
    frame: QRectF,
    border_image: &QBorderImage,
    qss: &QStyleDeclarations,
) -> bool {
    let Some(pixmap) = crate::assets::pixmap(&border_image.source) else {
        return false;
    };
    if frame.width <= 0.0 || frame.height <= 0.0 || pixmap.width == 0 || pixmap.height == 0 {
        return false;
    }

    let (dest_top, dest_right, dest_bottom, dest_left) = border_edge_widths(qss);
    if dest_top + dest_right + dest_bottom + dest_left <= 0.0 {
        return false;
    }

    let (slice_top, slice_right, slice_bottom, slice_left) = border_image.slice;
    let src_w = pixmap.width as f32;
    let src_h = pixmap.height as f32;
    let left = slice_left.clamp(0.0, src_w);
    let right = slice_right.clamp(0.0, src_w);
    let top = slice_top.clamp(0.0, src_h);
    let bottom = slice_bottom.clamp(0.0, src_h);
    let mid_w = (src_w - left - right).max(0.0);
    let mid_h = (src_h - top - bottom).max(0.0);

    let mut dest_left = dest_left.min(frame.width * 0.5);
    let mut dest_right = dest_right.min(frame.width * 0.5);
    let mut dest_top = dest_top.min(frame.height * 0.5);
    let mut dest_bottom = dest_bottom.min(frame.height * 0.5);
    if dest_left + dest_right > frame.width {
        let scale = frame.width / (dest_left + dest_right).max(1.0);
        dest_left *= scale;
        dest_right *= scale;
    }
    if dest_top + dest_bottom > frame.height {
        let scale = frame.height / (dest_top + dest_bottom).max(1.0);
        dest_top *= scale;
        dest_bottom *= scale;
    }
    let dest_mid_w = (frame.width - dest_left - dest_right).max(0.0);
    let dest_mid_h = (frame.height - dest_top - dest_bottom).max(0.0);

    let src_x = [0.0, left, left + mid_w];
    let src_y = [0.0, top, top + mid_h];
    let src_widths = [left, mid_w, right];
    let src_heights = [top, mid_h, bottom];
    let dst_x = [frame.x, frame.x + dest_left, frame.x + dest_left + dest_mid_w];
    let dst_y = [frame.y, frame.y + dest_top, frame.y + dest_top + dest_mid_h];
    let dst_widths = [dest_left, dest_mid_w, dest_right];
    let dst_heights = [dest_top, dest_mid_h, dest_bottom];

    for row in 0..3 {
        for col in 0..3 {
            let sw = src_widths[col];
            let sh = src_heights[row];
            let dw = dst_widths[col];
            let dh = dst_heights[row];
            if sw <= 0.0 || sh <= 0.0 || dw <= 0.0 || dh <= 0.0 {
                continue;
            }
            let patch = pixmap.copy_rect(
                src_x[col].round() as u32,
                src_y[row].round() as u32,
                sw.round().max(1.0) as u32,
                sh.round().max(1.0) as u32,
            );
            // Skip fully transparent patches (typical empty border-image center).
            if patch.pixels.chunks_exact(4).all(|px| px[3] == 0) {
                continue;
            }
            let dest = QRectF::new(dst_x[col], dst_y[row], dw, dh);
            let tile_h = row == 1 && border_image.vertical == QBorderImageMode::Repeat;
            let tile_w = col == 1 && border_image.horizontal == QBorderImageMode::Repeat;
            if tile_w || tile_h {
                paint_tiled_patch(painter, dest, &patch, tile_w, tile_h);
            } else {
                painter.draw_pixmap(dest, patch, 1.0);
            }
        }
    }
    true
}

fn paint_tiled_patch(
    painter: &mut QPainter,
    dest: QRectF,
    patch: &crate::gui::QPixmap,
    tile_x: bool,
    tile_y: bool,
) {
    let tile_w = (patch.width as f32).max(1.0);
    let tile_h = (patch.height as f32).max(1.0);
    let end_x = if tile_x {
        dest.x + dest.width
    } else {
        dest.x + tile_w.min(dest.width)
    };
    let end_y = if tile_y {
        dest.y + dest.height
    } else {
        dest.y + tile_h.min(dest.height)
    };
    let mut y = dest.y;
    while y < end_y {
        let mut x = dest.x;
        while x < end_x {
            let w = if tile_x {
                tile_w.min(dest.x + dest.width - x)
            } else {
                dest.width
            };
            let h = if tile_y {
                tile_h.min(dest.y + dest.height - y)
            } else {
                dest.height
            };
            let cell = if w < tile_w || h < tile_h {
                patch.copy_rect(0, 0, w.round().max(1.0) as u32, h.round().max(1.0) as u32)
            } else {
                patch.clone()
            };
            painter.draw_pixmap(QRectF::new(x, y, w, h), cell, 1.0);
            if !tile_x {
                break;
            }
            x += tile_w;
        }
        if !tile_y {
            break;
        }
        y += tile_h;
    }
}

fn paint_qss_side_borders(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if !qss.has_side_borders() {
        return;
    }
    let rect = qss_frame_rect(widget, qss);
    let fallback = qss
        .border_color
        .unwrap_or_else(|| widget.palette.color(QPaletteRole::WindowText));
    let base = qss.border_width.unwrap_or(0.0);
    for side in [
        QBorderSide::Top,
        QBorderSide::Right,
        QBorderSide::Bottom,
        QBorderSide::Left,
    ] {
        let declared = qss.side_border(side);
        let width = declared.and_then(|side| side.width).unwrap_or(base);
        if width <= 0.0 {
            continue;
        }
        let color = declared.and_then(|side| side.color).unwrap_or(fallback);
        let line = match side {
            QBorderSide::Top => QRectF::new(rect.x, rect.y, rect.width, width),
            QBorderSide::Bottom => QRectF::new(rect.x, rect.bottom() - width, rect.width, width),
            QBorderSide::Left => QRectF::new(rect.x, rect.y, width, rect.height),
            QBorderSide::Right => QRectF::new(rect.right() - width, rect.y, width, rect.height),
        };
        painter.fill_rect(line, color);
    }
}

fn paint_combo_popup(
    tree: &QWidgetTree,
    widget_id: QWidgetId,
    widget: &QWidget,
    rect: QRectF,
    painter: &mut QPainter,
) {
    let Some(popup) = widget.combo_popup.as_ref() else {
        return;
    };
    let items = property_texts(widget, "items");
    if items.is_empty() {
        return;
    }
    let qss = resolved_style(tree, widget_id);
    let view_qss = resolved_style_part(tree, widget_id, "view");
    let item_qss = resolved_style_part(tree, widget_id, "item");
    let font = qss_font(widget, &qss);
    let metrics = QFontMetrics::new(&font);
    let line_height = metrics.height();
    let palette = &widget.palette;
    let highlight = palette.color(QPaletteRole::Highlight);
    let highlighted_text = item_qss
        .selection_color
        .or(view_qss.selection_color)
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    let text_color = item_qss
        .color
        .or(view_qss.color)
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let visible = ((rect.height - 2.0) / QCOMBO_POPUP_ROW_H).floor().max(1.0) as usize;

    painter.fill_rect(
        rect,
        view_qss
            .background
            .unwrap_or_else(|| palette.color(QPaletteRole::Base)),
    );
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        rect.x + 1.0,
        rect.y + 1.0,
        (rect.width - 2.0).max(0.0),
        (rect.height - 2.0).max(0.0),
    )));
    for row in popup.scroll..(popup.scroll + visible).min(items.len()) {
        let y = rect.y + 1.0 + (row - popup.scroll) as f32 * QCOMBO_POPUP_ROW_H;
        let selected = row == popup.highlight;
        let row_rect = QRectF::new(
            rect.x + 1.0,
            y,
            (rect.width - 2.0).max(0.0),
            QCOMBO_POPUP_ROW_H,
        );
        if selected {
            painter.fill_rect(
                row_rect,
                item_qss
                    .selection_background
                    .or(view_qss.selection_background)
                    .unwrap_or(highlight),
            );
        } else if let Some(background) = item_qss.background {
            painter.fill_rect(row_rect, background);
        }
        painter.set_font(font.clone());
        painter.draw_text(
            QRectF::new(
                rect.x + 5.0,
                y + ((QCOMBO_POPUP_ROW_H - line_height) / 2.0).max(0.0),
                (rect.width - 10.0).max(1.0),
                line_height,
            ),
            &items[row],
            if selected {
                highlighted_text
            } else {
                text_color
            },
        );
    }
    painter.restore();
    painter.stroke_rect(rect, 1.0, palette.color(QPaletteRole::Mid));
}

fn paint_splitter_handles(tree: &QWidgetTree, widget: &QWidget, painter: &mut QPainter) {
    let Some(splitter) = widget.splitter.as_deref() else {
        return;
    };
    let horizontal = splitter.is_horizontal();
    let palette = &widget.palette;
    for pair in widget.children.windows(2) {
        let (Some(a), Some(b)) = (tree.widget(pair[0]), tree.widget(pair[1])) else {
            continue;
        };
        let rect = if horizontal {
            let start = a.geometry.right();
            let width = (b.geometry.x - start).max(0.0);
            QRectF::new(start, widget.geometry.y, width, widget.geometry.height)
        } else {
            let start = a.geometry.bottom();
            let height = (b.geometry.y - start).max(0.0);
            QRectF::new(widget.geometry.x, start, widget.geometry.width, height)
        };
        if rect.is_empty() {
            continue;
        }
        // Qt's handle is a flat window-coloured gap between the pane
        // frames — no gradient and no extra edge lines (the panes' own
        // 1 px borders already flank it; mixed/206 probe: 278..281 = 239).
        painter.fill_rect(rect, palette.color(QPaletteRole::Window));
    }
}

pub(crate) fn property_bool(widget: &QWidget, key: &str) -> bool {
    matches!(widget.property(key), Some(QVariant::Bool(true)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelVAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LabelAlign {
    horizontal: QTextAlign,
    vertical: LabelVAlign,
}

fn widget_alignment_flags(widget: &QWidget) -> u32 {
    match widget.property("alignment") {
        Some(QVariant::String(text)) | Some(QVariant::Enum(text)) => {
            form_xml::parse_alignment(text)
        }
        Some(QVariant::List(items)) => items.iter().fold(0u32, |mask, item| {
            mask | match item {
                QVariant::String(text) | QVariant::Enum(text) => form_xml::parse_alignment(text),
                _ => 0,
            }
        }),
        _ => 0,
    }
}

/// QLabel text align from QSS `text-align` (horizontal) + `.ui` `alignment` flags.
///
/// Qt default when unset: left + vertical center.
fn label_align(widget: &QWidget, qss: &QStyleDeclarations) -> LabelAlign {
    let flags = widget_alignment_flags(widget);
    let horizontal = if let Some(align) = qss.text_align {
        align
    } else if flags & form_xml::ALIGN_HCENTER != 0 {
        QTextAlign::Center
    } else if flags & form_xml::ALIGN_RIGHT != 0 {
        QTextAlign::Right
    } else {
        QTextAlign::Left
    };
    let vertical = if flags & form_xml::ALIGN_TOP != 0 {
        LabelVAlign::Top
    } else if flags & form_xml::ALIGN_BOTTOM != 0 {
        LabelVAlign::Bottom
    } else {
        // AlignVCenter, AlignCenter, or unset → Qt default VCenter.
        LabelVAlign::Center
    };
    LabelAlign {
        horizontal,
        vertical,
    }
}

fn aligned_text_x(content: QRectF, text_width: f32, horizontal: QTextAlign) -> f32 {
    match horizontal {
        QTextAlign::Center => {
            content.x + ((content.width - text_width) / 2.0).max(0.0)
        }
        QTextAlign::Right => content.x + (content.width - text_width).max(0.0),
        QTextAlign::Left => content.x,
    }
}

fn aligned_text_y(
    content: QRectF,
    block_height: f32,
    vertical: LabelVAlign,
    glyphon_nudge: bool,
) -> f32 {
    let mut y = match vertical {
        LabelVAlign::Top => content.y,
        LabelVAlign::Center => {
            content.y + ((content.height - block_height) / 2.0).max(0.0)
        }
        LabelVAlign::Bottom => (content.bottom() - block_height).max(content.y),
    };
    // Glyphon places ink ~1px lower than Qt inside a tall VCenter label
    // (gallery/spin #129: "Количество:" starts at y=49 in Qt, y=50 here).
    if glyphon_nudge && vertical == LabelVAlign::Center && content.height > block_height + 2.0 {
        y = (y - 1.0).max(content.y);
    }
    y
}

pub(crate) fn property_int(widget: &QWidget, key: &str) -> Option<i64> {
    match widget.property(key) {
        Some(QVariant::Int(value)) => Some(*value),
        Some(QVariant::Double(value)) => Some(*value as i64),
        _ => None,
    }
}

pub(crate) fn variant_text(value: &QVariant) -> Option<String> {
    match value {
        QVariant::String(text) | QVariant::Enum(text) => Some(text.clone()),
        QVariant::Int(value) => Some(value.to_string()),
        QVariant::Double(value) => Some(value.to_string()),
        QVariant::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(crate) fn property_texts(widget: &QWidget, key: &str) -> Vec<String> {
    match widget.property(key) {
        Some(QVariant::List(items)) => items.iter().filter_map(variant_text).collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn combo_text(widget: &QWidget) -> Option<String> {
    let items = property_texts(widget, "items");
    if !items.is_empty() {
        let index = property_int(widget, "currentIndex").unwrap_or(0).max(0) as usize;
        return items.get(index).cloned().or_else(|| items.first().cloned());
    }
    widget_text(widget)
}

fn paint_line_edit_selection(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if widget.class_name != "QLineEdit" {
        return;
    }
    let Some(anchor) = widget.selection_anchor else {
        return;
    };
    let text = widget_text(widget).unwrap_or_default();
    let length = text.chars().count();
    let start = anchor.min(widget.cursor_position).min(length);
    let end = anchor.max(widget.cursor_position).min(length);
    if start >= end {
        return;
    }
    let font = qss_font(widget, qss);
    let metrics = QFontMetrics::new(&font);
    let prefix: String = text.chars().take(start).collect();
    let selected: String = text.chars().take(end).skip(start).collect();
    let content = qss_content_rect(widget, qss, (3.0, 4.0, 3.0, 3.0));
    let x = content.x + metrics.horizontal_advance(&prefix);
    let width = metrics.horizontal_advance(&selected);
    let y = content.y;
    let height = content.height;
    if width > 0.0 && height > 0.0 {
        painter.fill_rect(
            QRectF::new(x, y, width, height),
            qss.selection_background
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::Highlight)),
        );
    }
}

fn paint_line_edit_caret(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if widget.class_name != "QLineEdit"
        || !widget.has_focus
        || !widget.enabled
        || !widget.cursor_visible
    {
        return;
    }
    let text = widget_text(widget).unwrap_or_default();
    let prefix: String = text.chars().take(widget.cursor_position).collect();
    let font = qss_font(widget, qss);
    let metrics = QFontMetrics::new(&font);
    let content = qss_content_rect(widget, qss, (3.0, 4.0, 3.0, 3.0));
    // Qt puts the 1 px caret exactly at the glyph advance; no extra gap.
    let caret_x = (content.x + metrics.horizontal_advance(&prefix)).min(content.right());
    let caret_y = content.y;
    let caret_h = content.height.max(0.0);
    if caret_h > 0.0 {
        painter.fill_rect(
            QRectF::new(caret_x, caret_y, 1.0, caret_h),
            text_color(widget, qss),
        );
    }
}

fn paint_line_edit_text(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    let Some(text) = widget_text(widget) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let font = qss_font(widget, qss);
    let metrics = QFontMetrics::new(&font);
    let line_height = metrics.height();
    let content = qss_content_rect(widget, qss, (3.0, 4.0, 3.0, 3.0));
    let y = content.y + ((content.height - line_height) / 2.0).max(0.0);
    let base_x = content.x;
    let color = text_color(widget, qss);
    painter.set_font(font);
    let (start, end) = match widget.selection_anchor {
        Some(anchor) => {
            let length = text.chars().count();
            let start = anchor.min(widget.cursor_position).min(length);
            let end = anchor.max(widget.cursor_position).min(length);
            (start, end)
        }
        None => (0, 0),
    };
    if start >= end {
        draw_text_line(painter, base_x, y, content.right(), line_height, text, color);
        return;
    }
    let prefix: String = text.chars().take(start).collect();
    let selected: String = text.chars().take(end).skip(start).collect();
    let suffix: String = text.chars().skip(end).collect();
    let prefix_width = metrics.horizontal_advance(&prefix);
    let selected_width = metrics.horizontal_advance(&selected);
    // Each run starts at its metric origin but may extend to `content.right()`
    // — never use advance as the draw width (glyphon clips the last glyph).
    if !prefix.is_empty() {
        draw_text_line(
            painter,
            base_x,
            y,
            content.right(),
            line_height,
            prefix,
            color,
        );
    }
    if !selected.is_empty() {
        draw_text_line(
            painter,
            base_x + prefix_width,
            y,
            content.right(),
            line_height,
            selected,
            qss.selection_color
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::HighlightedText)),
        );
    }
    if !suffix.is_empty() {
        draw_text_line(
            painter,
            base_x + prefix_width + selected_width,
            y,
            content.right(),
            line_height,
            suffix,
            color,
        );
    }
}

fn paint_widget_text(
    tree: &QWidgetTree,
    widget_id: QWidgetId,
    widget: &QWidget,
    painter: &mut QPainter,
    style: &dyn QStyle,
    option: &QStyleOption,
    qss: &QStyleDeclarations,
) {
    if widget.class_name == "QProgressBar" {
        // Label is painted with dual-color clipping in `paint_progress_bar`.
        return;
    }
    if widget.class_name == "QPlainTextEdit" {
        // Text/selection/caret painted in `paint_plain_text_edit`.
        return;
    }
    if widget.class_name == "QLineEdit" {
        paint_line_edit_text(widget, painter, qss);
        return;
    }
    if widget.class_name == "QGroupBox" {
        if let Some(title) = widget_text(widget) {
            if !title.is_empty() {
                let font = qss_font(widget, qss);
                let metrics = QFontMetrics::new(&font);
                let line_height = metrics.height();
                painter.set_font(font);
                let frame = qss_frame_rect(widget, qss);
                let pad_left = qss.padding.map(|padding| padding.0).unwrap_or(0.0);
                let pad_right = qss.padding.map(|padding| padding.2).unwrap_or(0.0);
                let title_on_frame = qss.group_box_title_on_frame();
                let border_w = qss.border_width.unwrap_or(1.0);
                // Qt `::title` with subcontrol-origin margin: vertically center
                // on the top border; wipe a text-width strip so the stroke does
                // not cut through the caption.
                let title_y = if title_on_frame {
                    frame.y - line_height * 0.5 + border_w * 0.5
                } else {
                    frame.y + 2.0
                };
                let text_w = metrics.horizontal_advance(&title);
                let title_x = frame.x + pad_left + if title_on_frame { 2.0 } else { 0.0 };
                let max_w = (frame.width - pad_left - pad_right).max(1.0);
                if title_on_frame {
                    let wipe_pad = 3.0;
                    let wipe = QRectF::new(
                        (title_x - wipe_pad).max(frame.x),
                        title_y,
                        (text_w + wipe_pad * 2.0).min(max_w),
                        line_height,
                    );
                    // Caption sits in the margin over the parent fill (e.g.
                    // `QDialog { background-color }`), not the group box face.
                    let bg = ancestor_qss_background(tree, widget_id)
                        .unwrap_or_else(|| widget.palette.color(QPaletteRole::Window));
                    painter.fill_rect(wipe, bg);
                }
                painter.draw_text(
                    QRectF::new(title_x, title_y, max_w, line_height),
                    title,
                    text_color(widget, qss),
                );
            }
        }
        return;
    }
    if matches!(widget.class_name.as_str(), "QPushButton" | "QToolButton") {
        paint_button_label(widget, painter, qss);
        return;
    }
    if widget.class_name == "QComboBox" {
        let font = qss_font(widget, qss);
        let metrics = QFontMetrics::new(&font);
        let line_height = metrics.height();
        painter.set_font(font);
        if let Some(text) = combo_text(widget) {
            let content = qss_content_rect(widget, qss, (3.0, 0.0, 26.0, 0.0));
            painter.draw_text(
                QRectF::new(
                    content.x,
                    content.y + ((content.height - line_height) / 2.0).max(0.0),
                    content.width.max(1.0),
                    line_height,
                ),
                text,
                text_color(widget, qss),
            );
        }
        let mut arrow_option = option.clone();
        arrow_option.rect = QRectF::new(
            widget.geometry.right() - 20.5,
            widget.geometry.y - 2.0,
            19.0,
            widget.geometry.height + 4.0,
        );
        painter.draw_commands(
            &style.draw_primitive(QPrimitiveElement::IndicatorArrowDown, &arrow_option),
        );
        return;
    }
    if matches!(widget.class_name.as_str(), "QListWidget" | "QTreeWidget") {
        return;
    }
    let Some(text) = widget_text(widget) else {
        return;
    };
    if text.is_empty() {
        return;
    }
    if widget.class_name == "QTableWidget" {
        return;
    }
    let font = qss_font(widget, qss);
    let metrics = QFontMetrics::new(&font);
    let text_width = metrics.horizontal_advance(&text);
    let line_height = metrics.height();
    painter.set_font(font);
    let content = qss_content_rect(widget, qss, (0.0, 0.0, 0.0, 0.0));
    let align = label_align(widget, qss);
    if widget.class_name == "QLabel" && property_bool(widget, "wordWrap") {
        // Word-wrapped label: greedy lines at content width; block aligned
        // per Qt::Align* (default VCenter), clipped to the rect.
        let lines = wrap_text_lines(&text, content.width, &metrics);
        let block_height = lines.len() as f32 * line_height;
        let top = aligned_text_y(content, block_height, align.vertical, true);
        let max_lines = ((content.height / line_height).ceil() as usize).max(1);
        let color = text_color(widget, qss);
        for (index, line) in lines.iter().take(max_lines).enumerate() {
            let x = aligned_text_x(
                content,
                metrics.horizontal_advance(line),
                align.horizontal,
            );
            draw_text_line(
                painter,
                x,
                top + index as f32 * line_height,
                content.right(),
                line_height,
                line.clone(),
                color,
            );
        }
        return;
    }
    // Align using metrics, but always draw through `content.right()`.
    // Metric-tight widths clip the last glyph under glyphon (OK→O, value→valu).
    let mut text_x = content.x;
    let mut y = content.y + ((content.height - line_height) / 2.0).max(0.0);
    match widget.class_name.as_str() {
        "QPushButton" | "QToolButton" => {
            text_x += ((content.width - text_width) / 2.0).max(0.0);
        }
        "QLineEdit" => {}
        "QCheckBox" | "QRadioButton" => {
            text_x += widget.font.pixel_size + style.pixel_metric(QPixelMetric::ButtonMargin);
        }
        "QLabel" => {
            text_x = aligned_text_x(content, text_width, align.horizontal);
            y = aligned_text_y(content, line_height, align.vertical, true);
        }
        _ => {
            if qss.padding.is_none() {
                text_x += 4.0;
            }
        }
    }
    draw_text_line(
        painter,
        text_x,
        y,
        content.right(),
        line_height,
        text,
        text_color(widget, qss),
    );
}

/// Single-line text from `x` through `right` (exclusive).
///
/// **Invariant:** never pass `horizontal_advance(text)` as the draw width —
/// cosmic-text / glyphon advances can exceed `QFontMetrics` by a few px and
/// the last character gets clipped (`"OK"` → `"O"`).
pub(crate) fn draw_text_line(
    painter: &mut QPainter,
    x: f32,
    y: f32,
    right: f32,
    line_height: f32,
    text: impl Into<String>,
    color: QColor,
) {
    painter.draw_text(
        QRectF::new(x, y, (right - x).max(1.0), line_height),
        text,
        color,
    );
}

fn paint_button_label(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    let text = widget_text(widget).unwrap_or_default();
    let font = qss_font(widget, qss);
    let metrics = QFontMetrics::new(&font);
    let line_height = metrics.height();
    painter.set_font(font);
    let down = widget.pressed || property_bool(widget, "checked");
    let shift = if down { 1.0 } else { 0.0 };
    let color = text_color(widget, qss);
    let frame = qss_frame_rect(widget, qss);
    let (pad_left, pad_top, pad_right, pad_bottom) = qss.padding.unwrap_or((0.0, 0.0, 0.0, 0.0));
    let contents = QRectF::new(
        frame.x + pad_left + 1.0,
        frame.y + pad_top + 1.0,
        (frame.width - pad_left - pad_right - 2.0).max(1.0),
        (frame.height - pad_top - pad_bottom - 2.0).max(1.0),
    );
    let icon = if widget.enabled {
        widget.icon.as_ref()
    } else {
        widget.icon_disabled.as_ref().or(widget.icon.as_ref())
    };
    if let Some(pixmap) = icon {
        let icon_width = if widget.icon_size.width > 0.0 {
            widget.icon_size.width
        } else {
            pixmap.width as f32
        };
        let icon_height = if widget.icon_size.height > 0.0 {
            widget.icon_size.height
        } else {
            pixmap.height as f32
        };
        let text_width = if text.is_empty() {
            0.0
        } else {
            metrics.horizontal_advance(&text)
        };
        let label_width = icon_width
            + if text.is_empty() {
                0.0
            } else {
                text_width + 4.0
            };
        let start = contents.x + ((contents.width - label_width) / 2.0).max(0.0) + shift;
        let icon_y = contents.y + ((contents.height - icon_height) / 2.0).max(0.0) + shift;
        let opacity = if widget.enabled || widget.icon_disabled.is_some() {
            1.0
        } else {
            0.45
        };
        painter.draw_pixmap(
            QRectF::new(start, icon_y, icon_width, icon_height),
            pixmap.clone(),
            opacity,
        );
        if !text.is_empty() {
            let text_x = start + icon_width + 2.0;
            let text_y = contents.y + ((contents.height - line_height) / 2.0).max(0.0) + shift;
            draw_text_line(
                painter,
                text_x,
                text_y,
                contents.right(),
                line_height,
                text,
                color,
            );
        }
        return;
    }
    if text.is_empty() {
        return;
    }
    let text_width = metrics.horizontal_advance(&text);
    let text_y = contents.y + ((contents.height - line_height) / 2.0).max(0.0) + shift;
    let text_x = contents.x + ((contents.width - text_width) / 2.0).max(0.0) + shift;
    draw_text_line(
        painter,
        text_x,
        text_y,
        contents.right(),
        line_height,
        text,
        color,
    );
}

pub(crate) fn widget_text(widget: &QWidget) -> Option<String> {
    if widget.class_name == "QGroupBox" {
        return match widget.property("title") {
            Some(QVariant::String(text)) | Some(QVariant::Enum(text)) => Some(text.clone()),
            _ => None,
        };
    }
    match widget.property("text") {
        Some(crate::core::QVariant::String(text)) | Some(crate::core::QVariant::Enum(text)) => {
            Some(text.clone())
        }
        Some(crate::core::QVariant::Int(value)) => Some(value.to_string()),
        Some(crate::core::QVariant::Double(value)) => Some(value.to_string()),
        _ => None,
    }
}

pub(crate) fn text_color(widget: &QWidget, qss: &QStyleDeclarations) -> QColor {
    if let Some(color) = qss.color {
        return color;
    }
    let role = match widget.class_name.as_str() {
        "QLineEdit" => QPaletteRole::Text,
        "QPushButton" | "QToolButton" | "QComboBox" => QPaletteRole::ButtonText,
        _ => QPaletteRole::WindowText,
    };
    let color = widget.palette.color(role);
    if widget.enabled {
        color
    } else {
        widget.palette.color(QPaletteRole::Mid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::QPaintCommand;
    use crate::item_model::QTABLE_HEADER_H;
    use crate::style::QFusionStyle;
    use crate::widgets::load_ui;

    const FIXTURE: &str = include_str!("../../../form-xml/tests/fixtures/test_dialog01.ui");
    const DIALOG02: &str = include_str!("../../../../../server/priv/qt_forms/test_dialog02.ui");
    const TABLE_UI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>320</width><height>200</height></rect></property>
  <property name="windowTitle"><string>Table</string></property>
  <layout class="QVBoxLayout" name="verticalLayout">
   <item><widget class="QTableWidget" name="tableWidget"/></item>
  </layout>
 </widget>
</ui>"#;

    #[test]
    fn paints_dialog_widgets_in_qt_order() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            [
                "TextLabel",
                "TextLabel",
                "TextLabel",
                "PushButton",
                "PushButton"
            ]
        );

        // Dialog panel + controls may use FillRect, FillRoundedRect, or
        // FillLinearGradientRect (Fusion line-edit / button chrome).
        let fills = painter
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    QPaintCommand::FillRect { .. }
                        | QPaintCommand::FillRoundedRect { .. }
                        | QPaintCommand::FillLinearGradientRect { .. }
                )
            })
            .count();
        assert!(fills >= 4, "expected painted fills, got {fills}");
        assert!(painter
            .commands()
            .iter()
            .any(|command| matches!(command, QPaintCommand::SetClip { .. })));
    }

    #[test]
    fn qss_margin_padding_and_gradient_affect_the_painted_rects() {
        let ui = r#"<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>220</width><height>100</height></rect></property>
  <widget class="QPushButton" name="button">
   <property name="geometry"><rect><x>0</x><y>0</y><width>100</width><height>40</height></rect></property>
   <property name="text"><string>OK</string></property>
   <property name="styleSheet"><string>QPushButton { margin: 5px; background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #ffffff, stop:1 #204020); border: 1px solid #000000; }</string></property>
  </widget>
  <widget class="QLineEdit" name="edit">
   <property name="geometry"><rect><x>0</x><y>50</y><width>100</width><height>30</height></rect></property>
   <property name="text"><string>abc</string></property>
   <property name="styleSheet"><string>QLineEdit { margin: 5px; padding-left: 8px; }</string></property>
  </widget>
 </widget></ui>"#;
        let (tree, root) = load_ui(ui).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        let gradient = painter.commands().iter().find_map(|command| match command {
            QPaintCommand::FillLinearGradientRect { rect, .. } => Some(*rect),
            _ => None,
        });
        assert_eq!(gradient, Some(QRectF::new(5.0, 5.0, 90.0, 30.0)));

        let edit_text = painter.commands().iter().find_map(|command| match command {
            QPaintCommand::DrawText { rect, text, .. } if text == "abc" => Some(*rect),
            _ => None,
        });
        let edit_text = edit_text.expect("line edit text");
        assert_eq!(edit_text.x, 13.0);
    }

    #[test]
    fn qlabel_alignment_covers_all_horizontal_and_vertical_flags() {
        use crate::gui::QPaintCommand;
        use crate::style::QFusionStyle;
        use crate::widgets::load_ui;
        use std::collections::BTreeMap;

        fn text_origin(alignment: &str) -> (f32, f32) {
            let ui = format!(
                r#"<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>200</width><height>100</height></rect></property>
  <widget class="QLabel" name="label">
   <property name="geometry"><rect><x>0</x><y>0</y><width>200</width><height>100</height></rect></property>
   <property name="text"><string>Hi</string></property>
   <property name="alignment"><set>{alignment}</set></property>
  </widget>
 </widget></ui>"#
            );
            let (tree, root) = load_ui(&ui).expect("loads ui");
            let style = QFusionStyle::default();
            let mut painter = QPainter::new();
            paint_widget_tree(&tree, root, &style, &mut painter);
            let rect = painter
                .commands()
                .iter()
                .find_map(|command| match command {
                    QPaintCommand::DrawText { rect, text, .. } if text == "Hi" => Some(*rect),
                    _ => None,
                })
                .expect("label text");
            (rect.x, rect.y)
        }

        // 3×3 matrix + AlignCenter shorthand.
        let cases = [
            ("Qt::AlignLeft|Qt::AlignTop", "left_top"),
            ("Qt::AlignHCenter|Qt::AlignTop", "hcenter_top"),
            ("Qt::AlignRight|Qt::AlignTop", "right_top"),
            ("Qt::AlignLeft|Qt::AlignVCenter", "left_vcenter"),
            ("Qt::AlignHCenter|Qt::AlignVCenter", "hcenter_vcenter"),
            ("Qt::AlignRight|Qt::AlignVCenter", "right_vcenter"),
            ("Qt::AlignLeft|Qt::AlignBottom", "left_bottom"),
            ("Qt::AlignHCenter|Qt::AlignBottom", "hcenter_bottom"),
            ("Qt::AlignRight|Qt::AlignBottom", "right_bottom"),
            ("Qt::AlignCenter", "align_center"),
        ];
        let mut got = BTreeMap::new();
        for (flags, key) in cases {
            got.insert(key, text_origin(flags));
        }

        let left_top = got["left_top"];
        let hcenter_top = got["hcenter_top"];
        let right_top = got["right_top"];
        let left_vcenter = got["left_vcenter"];
        let hcenter_vcenter = got["hcenter_vcenter"];
        let right_vcenter = got["right_vcenter"];
        let left_bottom = got["left_bottom"];
        let hcenter_bottom = got["hcenter_bottom"];
        let right_bottom = got["right_bottom"];
        let align_center = got["align_center"];

        // Horizontal ordering on each row.
        for row in [
            [left_top, hcenter_top, right_top],
            [left_vcenter, hcenter_vcenter, right_vcenter],
            [left_bottom, hcenter_bottom, right_bottom],
        ] {
            assert!(
                row[0].0 < row[1].0 && row[1].0 < row[2].0,
                "expected left < hcenter < right, got {row:?}"
            );
        }
        // Vertical ordering on each column.
        for col in [
            [left_top, left_vcenter, left_bottom],
            [hcenter_top, hcenter_vcenter, hcenter_bottom],
            [right_top, right_vcenter, right_bottom],
        ] {
            assert!(
                col[0].1 < col[1].1 && col[1].1 < col[2].1,
                "expected top < vcenter < bottom, got {col:?}"
            );
        }
        // Qt::AlignCenter == HCenter|VCenter.
        assert_eq!(align_center, hcenter_vcenter);
        // Absolute anchors: top/left near origin, bottom near widget bottom.
        assert!(left_top.0 < 2.0, "left x {}", left_top.0);
        assert!(left_top.1 < 2.0, "top y {}", left_top.1);
        assert!(left_bottom.1 > 70.0, "bottom y {}", left_bottom.1);
        assert!(right_top.0 > 100.0, "right x {}", right_top.0);
    }

    #[test]
    fn qss_border_image_emits_nine_textured_patches() {
        let ui = r#"<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>200</width><height>160</height></rect></property>
  <widget class="QLabel" name="frame">
   <property name="geometry"><rect><x>10</x><y>10</y><width>180</width><height>140</height></rect></property>
   <property name="text"><string></string></property>
   <property name="styleSheet"><string>QLabel { border-width: 52px; border-style: solid; border-color: transparent; border-image: url(qss/border-frame.png) 113 113 113 113 stretch stretch; }</string></property>
  </widget>
 </widget></ui>"#;
        let (tree, root) = load_ui(ui).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        let patches = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::DrawPixmap { .. }))
            .count();
        // Center patch is fully transparent and skipped → 8 edge/corner draws.
        assert_eq!(patches, 8, "expected 8 border-image patches, got {patches}");
    }

    #[test]
    fn qss_item_selector_paints_tree_rows() {
        let ui = r#"<ui version="4.0"><class>Dialog</class>
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>240</width><height>120</height></rect></property>
  <widget class="QTreeWidget" name="tree">
   <property name="geometry"><rect><x>0</x><y>0</y><width>240</width><height>120</height></rect></property>
   <property name="styleSheet"><string>QTreeWidget::item { background: #ff0000; color: #00ff00; }</string></property>
   <column><property name="text"><string>Name</string></property></column>
  </widget>
 </widget></ui>"#;
        let (mut tree, root) = load_ui(ui).expect("loads ui");
        let list = tree.find_widget("tree").expect("tree");
        tree.set_property(
            list,
            "data",
            crate::core::QVariant::List(vec![crate::core::QVariant::Map(vec![(
                "text".into(),
                crate::core::QVariant::String("Root".into()),
            )])]),
        );
        tree.layout(root, tree.widget(root).unwrap().geometry);
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        assert!(painter.commands().iter().any(|command| matches!(
            command,
            QPaintCommand::FillRect { color, .. } if *color == QColor::rgb(255, 0, 0)
        )));
        assert!(painter.commands().iter().any(|command| matches!(
            command,
            QPaintCommand::DrawText { text, color, .. }
                if text == "Root" && *color == QColor::rgb(0, 255, 0)
        )));
    }

    #[test]
    fn line_edit_caret_sits_at_the_glyph_advance_like_qt() {
        let (mut tree, root) = load_ui(FIXTURE).expect("loads ui");
        let line = tree.find_widget("lineEdit_1").expect("line edit");
        tree.set_property(line, "text", QVariant::String("abcd".into()));
        tree.layout(root, tree.widget(root).unwrap().geometry);
        tree.set_cursor_position(line, 2);
        tree.widget_mut(line).unwrap().cursor_visible = true;
        tree.set_focus(line);

        let geometry = tree.widget(line).unwrap().geometry;
        let font = tree.widget(line).unwrap().font.clone();
        let metrics = QFontMetrics::new(&font);
        let expected_x = geometry.x + 3.0 + metrics.horizontal_advance("ab");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);

        let caret = painter.commands().iter().find_map(|command| match command {
            QPaintCommand::FillRect { rect, .. }
                if (rect.width - 1.0).abs() < 0.01
                    && rect.height > 8.0
                    && (rect.x - expected_x).abs() < 0.01 =>
            {
                Some(*rect)
            }
            _ => None,
        });
        let caret = caret.expect("caret at Qt advance position");
        assert!((caret.y - (geometry.y + 4.0)).abs() < 0.01);
        assert!((caret.height - (geometry.height - 7.0)).abs() < 0.01);
    }

    #[test]
    fn focused_checkbox_gets_a_dotted_text_focus_rect() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let check = tree.find_widget("checkBox_1").expect("checkbox");
        tree.layout(root, tree.widget(root).unwrap().geometry);
        tree.set_focus(check);
        let geometry = tree.widget(check).unwrap().geometry;
        let font = tree.widget(check).unwrap().font.clone();
        let metrics = QFontMetrics::new(&font);
        let text = "CheckBox";
        let style = QFusionStyle::default();
        let text_x =
            geometry.x + font.pixel_size + style.pixel_metric(QPixelMetric::ButtonMargin);
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let rings = painter
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    QPaintCommand::StrokeDashedRect { rect, dash, gap, .. }
                        if (*dash - 1.0).abs() < 0.01
                            && (*gap - 1.0).abs() < 0.01
                            && rect.x >= text_x - 4.0
                            && rect.x <= text_x + metrics.horizontal_advance(text) + 4.0
                )
            })
            .count();
        assert_eq!(rings, 1, "expected one StrokeDashedRect focus ring");
        let ones = painter
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    QPaintCommand::FillRect { rect, .. }
                        if (rect.width - 1.0).abs() < 0.01 && (rect.height - 1.0).abs() < 0.01
                            && rect.x >= text_x - 4.0
                            && rect.x <= text_x + metrics.horizontal_advance(text) + 4.0
                )
            })
            .count();
        assert_eq!(ones, 0, "focus ring must not emit 1×1 FillRect flood");
    }

    #[test]
    fn paints_table_headers_and_cells() {
        let (mut tree, root) = load_ui(TABLE_UI).expect("loads ui");
        let table = tree.find_widget("tableWidget").expect("table");
        tree.set_property(table, "rowCount", crate::core::QVariant::Int(2));
        tree.set_property(table, "columnCount", crate::core::QVariant::Int(2));
        tree.set_property(
            table,
            "horizontalHeaderLabels",
            crate::core::QVariant::List(vec![
                crate::core::QVariant::String("Col 1".into()),
                crate::core::QVariant::String("Col 2".into()),
            ]),
        );
        tree.set_property(
            table,
            "data",
            crate::core::QVariant::List(vec![
                crate::core::QVariant::List(vec![
                    crate::core::QVariant::String("A".into()),
                    crate::core::QVariant::String("B".into()),
                ]),
                crate::core::QVariant::List(vec![
                    crate::core::QVariant::String("C".into()),
                    crate::core::QVariant::String("D".into()),
                ]),
            ]),
        );
        tree.layout(root, tree.widget(root).unwrap().geometry);

        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Col 1"));
        assert!(texts.contains(&"Col 2"));
        assert!(texts.contains(&"A"));
        assert!(texts.contains(&"D"));
    }

    #[test]
    fn paints_open_combo_popup_items_after_the_form() {
        let mut tree = crate::widgets::QWidgetTree::new();
        let root = tree.create_widget("QDialog", "Dialog");
        tree.set_geometry(root, QRectF::new(0.0, 0.0, 320.0, 220.0));
        let combo = tree.create_widget("QComboBox", "combo");
        tree.add_child(root, combo);
        tree.set_geometry(combo, QRectF::new(10.0, 10.0, 140.0, 25.0));
        tree.set_property(
            combo,
            "items",
            QVariant::List(vec![
                QVariant::String("Alpha".into()),
                QVariant::String("Beta".into()),
                QVariant::String("Gamma".into()),
            ]),
        );
        tree.open_combo_popup(combo);
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for item in ["Alpha", "Beta", "Gamma"] {
            assert!(
                texts.contains(&item),
                "popup item {item} missing: {texts:?}"
            );
        }
        let rect = tree.combo_popup_rect(combo).expect("popup rect");
        // The popup is painted after the tree, so the last row's text is not
        // clipped by the 25 px combo rectangle.
        let last = painter
            .commands()
            .iter()
            .rev()
            .find_map(|command| match command {
                QPaintCommand::DrawText { rect, text, .. } if text == "Gamma" => Some(*rect),
                _ => None,
            })
            .expect("Gamma text");
        assert!(last.y >= rect.y);
    }

    #[test]
    fn paints_checkbox_combo_list_and_tree_values() {
        let (mut tree, root) = load_ui(DIALOG02).expect("loads ui");
        let checkbox = tree.find_widget("checkBox_2").unwrap();
        tree.set_property(checkbox, "checked", QVariant::Bool(true));
        let combo = tree.find_widget("comboBox_1").unwrap();
        tree.set_property(
            combo,
            "items",
            QVariant::List(vec![
                QVariant::String("Option 1".into()),
                QVariant::String("Option 2".into()),
                QVariant::String("Option 3".into()),
            ]),
        );
        tree.set_property(combo, "currentIndex", QVariant::Int(2));
        let list = tree.find_widget("listWidget").unwrap();
        tree.set_property(
            list,
            "items",
            QVariant::List(vec![
                QVariant::String("Item 1".into()),
                QVariant::String("Item 2".into()),
            ]),
        );
        tree.set_property(list, "currentIndex", QVariant::Int(1));
        let tree_widget = tree.find_widget("treeWidget").unwrap();
        tree.set_property(
            tree_widget,
            "headerLabels",
            QVariant::List(vec![QVariant::String("Column 1".into())]),
        );
        tree.set_property(
            tree_widget,
            "data",
            QVariant::List(vec![QVariant::Map(vec![
                ("text".into(), QVariant::String("Root 1".into())),
                (
                    "columns".into(),
                    QVariant::List(vec![QVariant::String("Root Column".into())]),
                ),
            ])]),
        );
        tree.layout(root, tree.widget(root).unwrap().geometry);

        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let texts: Vec<&str> = painter
            .commands()
            .iter()
            .filter_map(|command| match command {
                QPaintCommand::DrawText { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Option 3"));
        assert!(texts.contains(&"Item 2"));
        assert!(texts.contains(&"Root 1  Root Column"));
        let check_lines = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::DrawLine { .. }))
            .count();
        assert!(check_lines >= 2);
    }

    #[test]
    fn clicking_table_selects_a_cell() {
        let (mut tree, root) = load_ui(TABLE_UI).expect("loads ui");
        let table = tree.find_widget("tableWidget").expect("table");
        tree.set_property(table, "rowCount", crate::core::QVariant::Int(2));
        tree.set_property(table, "columnCount", crate::core::QVariant::Int(2));
        tree.layout(root, tree.widget(root).unwrap().geometry);
        let geometry = tree.widget(table).unwrap().geometry;
        let point =
            crate::gui::QPointF::new(geometry.x + 14.0 + 5.0, geometry.y + QTABLE_HEADER_H + 5.0);
        tree.mouse_press(root, point);
        tree.mouse_release(root, point);
        let table = tree.table(table).expect("table");
        assert_eq!(table.current_row(), Some(0));
        assert_eq!(table.current_column(), Some(0));
        assert!(table.is_selected(0, 0));
    }

    #[test]
    fn button_text_is_centered() {
        let (tree, root) = load_ui(FIXTURE).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let button = tree.find_widget("pushButton_1").unwrap();
        let geometry = tree.widget(button).unwrap().geometry;
        let metrics = QFontMetrics::new(&tree.widget(button).unwrap().font);
        let expected =
            geometry.x + (geometry.width - metrics.horizontal_advance("PushButton")) / 2.0;
        let command = painter.commands().iter().find_map(|command| match command {
            QPaintCommand::DrawText { rect, text, .. } if text == "PushButton" => Some(*rect),
            _ => None,
        });
        let rect = command.expect("button text");
        assert!((rect.x - expected).abs() < 0.5);
        let advance = metrics.horizontal_advance("PushButton");
        // Draw width must exceed metric advance — tight advance clips glyphon.
        assert!(
            rect.width + 0.01 >= advance,
            "button text rect width {} < advance {}",
            rect.width,
            advance
        );
        assert!(
            rect.right() <= geometry.right() + 0.5,
            "text run should stay inside the button"
        );
    }

    #[test]
    fn short_button_label_draw_width_exceeds_advance() {
        // Reproduces "OK"→"O": metric-tight DrawText width under-clips glyphon.
        let ui = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0">
 <widget class="QDialog" name="Dialog">
  <property name="geometry"><rect><x>0</x><y>0</y><width>200</width><height>80</height></rect></property>
  <widget class="QPushButton" name="ok">
   <property name="geometry"><rect><x>20</x><y>20</y><width>80</width><height>28</height></rect></property>
   <property name="text"><string>OK</string></property>
  </widget>
 </widget>
</ui>"#;
        let (tree, root) = load_ui(ui).expect("loads ui");
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let button = tree.find_widget("ok").unwrap();
        let metrics = QFontMetrics::new(&tree.widget(button).unwrap().font);
        let advance = metrics.horizontal_advance("OK");
        let rect = painter
            .commands()
            .iter()
            .find_map(|command| match command {
                QPaintCommand::DrawText { rect, text, .. } if text == "OK" => Some(*rect),
                _ => None,
            })
            .expect("OK label");
        assert!(
            rect.width > advance + 1.0,
            "OK draw width {} must leave slack past advance {}",
            rect.width,
            advance
        );
    }
}
