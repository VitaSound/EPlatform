use crate::core::QVariant;
use crate::gui::{
    QColor, QFont, QFontMetrics, QPainter, QPalette, QPaletteRole, QPointF, QRectF, QSizeF,
};
use crate::item_model::{QSortOrder, QTABLE_HEADER_H, QTABLE_ROW_H};
use crate::item_view::{QVIEW_BRANCH_W, QVIEW_HEADER_H, QVIEW_ROW_H};
use crate::qss::{QBorderSide, QStyleDeclarations, QStyleSheet, QTextAlign};
use crate::scroll::{QOrientation, QScrollBar, QScrollBarSubControl};
use crate::style::{QControlElement, QPrimitiveElement, QStyle, QStyleOption, QStyleState};
use crate::widgets::{QWidget, QWidgetId, QWidgetTree, QCOMBO_POPUP_ROW_H};

/// Paint a form and its open combo popups. Popups are a top-level layer in
/// Qt, so they are replayed after the whole widget tree instead of being
/// clipped by the combo's own geometry.
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

    paint_widget_background(widget, style, &option, painter, &qss);
    if widget.class_name == "QSplitter" {
        paint_splitter_handles(tree, widget, painter);
    }
    paint_table(widget, painter, &item_qss, &header_qss);
    paint_item_views(widget, painter, &item_qss, &header_qss);
    paint_control_indicator(widget, painter);
    paint_line_edit_selection(widget, painter, &qss);
    paint_widget_text(widget, painter, &qss);
    paint_line_edit_caret(widget, painter, &qss);
    paint_focus_indicators(widget, painter, &qss);

    // Keep the widget's clip active while painting children: Qt clips a
    // child to every ancestor, so content that overflows a small form does
    // not paint over the frame or neighbouring windows.
    for child in &widget.children {
        paint_widget(tree, *child, style, painter);
    }
    painter.restore();
}

fn qss_font(widget: &QWidget, qss: &QStyleDeclarations) -> QFont {
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

fn resolved_style_part(tree: &QWidgetTree, widget_id: QWidgetId, part: &str) -> QStyleDeclarations {
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

fn qss_frame_rect(widget: &QWidget, qss: &QStyleDeclarations) -> QRectF {
    let (left, top, right, bottom) = qss.margin.unwrap_or((0.0, 0.0, 0.0, 0.0));
    QRectF::new(
        widget.geometry.x + left,
        widget.geometry.y + top,
        (widget.geometry.width - left - right).max(0.0),
        (widget.geometry.height - top - bottom).max(0.0),
    )
}

fn qss_content_rect(
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

fn resolved_style(tree: &QWidgetTree, widget_id: QWidgetId) -> QStyleDeclarations {
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
        declarations.merge(&sheet.resolve(widget));
    }
    declarations
}

fn paint_widget_background(
    widget: &QWidget,
    style: &dyn QStyle,
    option: &QStyleOption,
    painter: &mut QPainter,
    qss: &QStyleDeclarations,
) {
    let frame = qss_frame_rect(widget, qss);
    let mut frame_option = option.clone();
    frame_option.rect = frame;
    match widget.class_name.as_str() {
        "QDialog" | "QWidget" => {
            painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Panel, &frame_option));
        }
        "QGroupBox" => {
            let metrics = QFontMetrics::new(&qss_font(widget, qss));
            let top_margin = metrics.height().max(14.0) + 3.0;
            frame_option.rect = QRectF::new(
                frame.x,
                frame.y + top_margin,
                frame.width,
                (frame.height - top_margin).max(0.0),
            );
            painter.draw_commands(&style.draw_primitive(QPrimitiveElement::Frame, &frame_option));
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
        "QRadioButton" => {}
        "QComboBox" => {
            painter.draw_commands(&style.draw_control(QControlElement::ComboBox, &frame_option));
        }
        "QTableWidget" | "QListWidget" | "QTreeWidget" => {
            painter.draw_commands(&style.draw_control(QControlElement::TableView, &frame_option));
        }
        "QSplitter" => {
            painter.fill_rect(frame, widget.palette.color(QPaletteRole::Window));
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
        let base = fusion_button_color(palette);
        painter.fill_linear_gradient_rect(rect, 0.0, base.lighter(104), base.darker(102));
        if horizontal {
            painter.fill_rect(
                QRectF::new(rect.x, rect.y, 1.0, rect.height),
                palette.color(QPaletteRole::Window).darker(140),
            );
            painter.fill_rect(
                QRectF::new(rect.right() - 1.0, rect.y, 1.0, rect.height),
                palette.color(QPaletteRole::Window).lighter(115),
            );
        } else {
            painter.fill_rect(
                QRectF::new(rect.x, rect.y, rect.width, 1.0),
                palette.color(QPaletteRole::Window).darker(140),
            );
            painter.fill_rect(
                QRectF::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0),
                palette.color(QPaletteRole::Window).lighter(115),
            );
        }
    }
}

fn property_bool(widget: &QWidget, key: &str) -> bool {
    matches!(widget.property(key), Some(QVariant::Bool(true)))
}

fn property_int(widget: &QWidget, key: &str) -> Option<i64> {
    match widget.property(key) {
        Some(QVariant::Int(value)) => Some(*value),
        Some(QVariant::Double(value)) => Some(*value as i64),
        _ => None,
    }
}

fn variant_text(value: &QVariant) -> Option<String> {
    match value {
        QVariant::String(text) | QVariant::Enum(text) => Some(text.clone()),
        QVariant::Int(value) => Some(value.to_string()),
        QVariant::Double(value) => Some(value.to_string()),
        QVariant::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn property_texts(widget: &QWidget, key: &str) -> Vec<String> {
    match widget.property(key) {
        Some(QVariant::List(items)) => items.iter().filter_map(variant_text).collect(),
        _ => Vec::new(),
    }
}

fn combo_text(widget: &QWidget) -> Option<String> {
    let items = property_texts(widget, "items");
    if !items.is_empty() {
        let index = property_int(widget, "currentIndex").unwrap_or(0).max(0) as usize;
        return items.get(index).cloned().or_else(|| items.first().cloned());
    }
    widget_text(widget)
}

/// Qt's focus indicators are style-dependent: Windows draws a dotted text
/// rectangle for check/radio and a dotted inner rectangle for buttons, while
/// Fusion fills the check/radio text area and keeps only the button frame.
/// EPlatform uses the dotted variant for check/radio/button so the focus is
/// obvious on its flat palette; line edits and combos keep the Fusion blue
/// frame.
fn paint_focus_indicators(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
    if !widget.has_focus || !widget.enabled {
        return;
    }
    let color = widget
        .palette
        .color(QPaletteRole::WindowText)
        .with_alpha(0.72);
    match widget.class_name.as_str() {
        "QPushButton" | "QToolButton" => {
            let frame = qss_frame_rect(widget, qss);
            paint_dotted_rect(
                painter,
                QRectF::new(
                    frame.x + 3.0,
                    frame.y + 3.0,
                    (frame.width - 6.0).max(1.0),
                    (frame.height - 6.0).max(1.0),
                ),
                color,
            );
        }
        "QCheckBox" | "QRadioButton" => {
            let Some(text) = widget_text(widget) else {
                return;
            };
            if text.is_empty() {
                return;
            }
            let content = qss_content_rect(widget, qss, (0.0, 0.0, 0.0, 0.0));
            let font = qss_font(widget, qss);
            let metrics = QFontMetrics::new(&font);
            let line_height = metrics.height();
            let text_width = metrics.horizontal_advance(&text);
            let text_x = content.x + widget.font.pixel_size + 6.0;
            let text_y = content.y + ((content.height - line_height) / 2.0).max(0.0);
            paint_dotted_rect(
                painter,
                QRectF::new(
                    text_x - 2.0,
                    text_y - 1.0,
                    text_width + 4.0,
                    line_height + 2.0,
                ),
                color,
            );
        }
        _ => {}
    }
}

fn paint_dotted_rect(painter: &mut QPainter, rect: QRectF, color: QColor) {
    let left = rect.x.floor();
    let top = rect.y.floor();
    let right = rect.right().floor();
    let bottom = rect.bottom().floor();
    if right <= left || bottom <= top {
        return;
    }
    let mut x = left;
    while x <= right {
        painter.fill_rect(QRectF::new(x, top, 1.0, 1.0), color);
        painter.fill_rect(QRectF::new(x, bottom, 1.0, 1.0), color);
        x += 2.0;
    }
    let mut y = top + 2.0;
    while y < bottom {
        painter.fill_rect(QRectF::new(left, y, 1.0, 1.0), color);
        painter.fill_rect(QRectF::new(right, y, 1.0, 1.0), color);
        y += 2.0;
    }
}

fn paint_item_views(
    widget: &QWidget,
    painter: &mut QPainter,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    match widget.class_name.as_str() {
        "QListWidget" => paint_list_widget(widget, painter, item_qss),
        "QTreeWidget" => paint_tree_widget(widget, painter, item_qss, header_qss),
        _ => {}
    }
}

fn paint_list_widget(widget: &QWidget, painter: &mut QPainter, item_qss: &QStyleDeclarations) {
    let Some(list) = widget.list.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let layout = list.scroll_area(rect);
    let viewport = layout.viewport;
    painter.fill_rect(rect, palette.color(QPaletteRole::Base));
    painter.stroke_rect(rect, 1.0, palette.color(QPaletteRole::Mid));

    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let text_color = item_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(viewport));
    for index in visible_rows(list.scroll, viewport.height, list.items.len()) {
        let y = viewport.y + list.row_y(index);
        let selected = list.selected.contains(&index);
        let row_rect = QRectF::new(viewport.x, y, viewport.width, QVIEW_ROW_H);
        if selected {
            painter.fill_rect(
                row_rect,
                item_qss
                    .selection_background
                    .unwrap_or_else(|| palette.color(QPaletteRole::Highlight)),
            );
        } else if let Some(background) = item_qss.background {
            painter.fill_rect(row_rect, background);
        }
        if widget.has_focus && list.current == Some(index) {
            paint_dotted_rect(
                painter,
                QRectF::new(
                    row_rect.x + 1.0,
                    row_rect.y + 1.0,
                    (row_rect.width - 2.0).max(1.0),
                    (row_rect.height - 2.0).max(1.0),
                ),
                palette.color(QPaletteRole::WindowText).with_alpha(0.72),
            );
        }
        painter.draw_text(
            QRectF::new(
                viewport.x + 4.0 - list.horizontal,
                y + ((QVIEW_ROW_H - line_height) / 2.0).max(0.0),
                (list.content_width + 8.0).max(1.0),
                line_height,
            ),
            &list.items[index],
            if selected { selected_text } else { text_color },
        );
    }
    painter.restore();

    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, bar, &list.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, bar, &list.hbar, palette, widget.enabled);
    }
}

fn fusion_button_color(palette: &crate::gui::QPalette) -> QColor {
    let button = palette.color(QPaletteRole::Button);
    button
        .lighter(100 + ((180_i32 - button.q_gray() as i32) / 6).max(1))
        .with_saturation_factor(75)
}

pub fn paint_scroll_bar(
    painter: &mut QPainter,
    rect: QRectF,
    bar: &QScrollBar,
    palette: &QPalette,
    enabled: bool,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let base = palette.color(QPaletteRole::Window);
    let disabled = palette.color(QPaletteRole::Mid);
    let border = if enabled { base.darker(140) } else { disabled };
    let arrow_color = if enabled {
        palette.color(QPaletteRole::ButtonText).with_alpha(0.70)
    } else {
        disabled
    };
    painter.fill_rect(rect, base);
    let groove = bar.groove_rect(rect);
    if groove.width > 0.0 && groove.height > 0.0 {
        painter.fill_rect(groove, base.darker(104));
    }

    let mut arrow = |control: QScrollBarSubControl, up: bool| {
        let area = bar.sub_control_rect(rect, control);
        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }
        let pressed = bar.pressed == Some(control);
        let hovered = bar.hovered == Some(control);
        let fill = if !enabled {
            base
        } else if pressed {
            base.darker(112)
        } else if hovered {
            base.lighter(105)
        } else {
            base
        };
        painter.fill_linear_gradient_rect(area, 0.0, fill.lighter(104), fill.darker(102));
        paint_scroll_arrow(painter, area, bar.orientation, up, arrow_color);
        painter.fill_rect(
            QRectF::new(area.x, area.y, area.width, 1.0),
            QColor::rgba8(255, 255, 255, 30),
        );
    };
    arrow(QScrollBarSubControl::SubLine, true);
    arrow(QScrollBarSubControl::AddLine, false);

    let slider = bar.slider_rect(rect);
    if slider.width <= 0.0 || slider.height <= 0.0 {
        return;
    }
    let pressed = bar.pressed == Some(QScrollBarSubControl::Slider);
    let hovered = bar.hovered == Some(QScrollBarSubControl::Slider);
    let color = if !enabled {
        base
    } else if pressed {
        base.darker(112)
    } else if hovered {
        base.lighter(106)
    } else {
        base
    };
    painter.fill_linear_gradient_rect(slider, 2.0, color.lighter(104), color.darker(102));
    painter.stroke_rounded_rect(
        QRectF::new(
            slider.x + 0.5,
            slider.y + 0.5,
            (slider.width - 1.0).max(1.0),
            (slider.height - 1.0).max(1.0),
        ),
        2.0,
        1.0,
        border,
    );
}

fn paint_scroll_arrow(
    painter: &mut QPainter,
    area: QRectF,
    orientation: QOrientation,
    up_or_left: bool,
    color: QColor,
) {
    let center_x = area.x + area.width / 2.0;
    let center_y = area.y + area.height / 2.0;
    match orientation {
        QOrientation::Vertical => {
            let half = 3.0_f32.min(area.width / 3.0).max(1.0);
            let height = 3.0_f32.min(area.height / 3.0).max(1.0);
            if up_or_left {
                painter.fill_triangle(
                    QPointF::new(center_x - half, center_y + height / 2.0),
                    QPointF::new(center_x + half, center_y + height / 2.0),
                    QPointF::new(center_x, center_y - height / 2.0),
                    color,
                );
            } else {
                painter.fill_triangle(
                    QPointF::new(center_x - half, center_y - height / 2.0),
                    QPointF::new(center_x + half, center_y - height / 2.0),
                    QPointF::new(center_x, center_y + height / 2.0),
                    color,
                );
            }
        }
        QOrientation::Horizontal => {
            let half = 3.0_f32.min(area.height / 3.0).max(1.0);
            let width = 3.0_f32.min(area.width / 3.0).max(1.0);
            if up_or_left {
                painter.fill_triangle(
                    QPointF::new(center_x + width / 2.0, center_y - half),
                    QPointF::new(center_x + width / 2.0, center_y + half),
                    QPointF::new(center_x - width / 2.0, center_y),
                    color,
                );
            } else {
                painter.fill_triangle(
                    QPointF::new(center_x - width / 2.0, center_y - half),
                    QPointF::new(center_x - width / 2.0, center_y + half),
                    QPointF::new(center_x + width / 2.0, center_y),
                    color,
                );
            }
        }
    }
}

fn paint_header_gradient(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    let base = fusion_button_color(&widget.palette);
    painter.fill_linear_gradient_rect(rect, 0.0, base.lighter(104), base.darker(102));
}

fn paint_header_section(widget: &QWidget, painter: &mut QPainter, rect: QRectF) {
    paint_header_gradient(widget, painter, rect);
    if rect.width > 0.0 && rect.height > 0.0 {
        painter.fill_rect(
            QRectF::new(rect.x, rect.y, rect.width, 1.0),
            QColor::rgba8(255, 255, 255, 30),
        );
        painter.fill_rect(
            QRectF::new(rect.x, rect.bottom() - 1.0, rect.width, 1.0),
            widget.palette.color(QPaletteRole::Window).darker(140),
        );
    }
}

fn paint_tree_widget(
    widget: &QWidget,
    painter: &mut QPainter,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    let Some(tree) = widget.tree.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let layout = crate::scroll::scroll_area_layout(
        rect,
        QSizeF::new(tree.content_width, QVIEW_HEADER_H + tree.content_height()),
        crate::scroll::QScrollBarPolicy::AsNeeded,
        crate::scroll::QScrollBarPolicy::AsNeeded,
        crate::scroll::QSCROLLBAR_EXTENT,
    );
    let viewport = layout.viewport;
    let header = QRectF::new(viewport.x, viewport.y, viewport.width, QVIEW_HEADER_H);
    let body = QRectF::new(
        viewport.x,
        viewport.y + QVIEW_HEADER_H,
        viewport.width,
        (viewport.height - QVIEW_HEADER_H).max(0.0),
    );
    painter.fill_rect(rect, palette.color(QPaletteRole::Base));
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let header_color = header_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::ButtonText));
    painter.save();
    painter.set_clip_rect(Some(viewport));
    paint_header_section(widget, painter, header);
    if let Some(background) = header_qss.background {
        painter.fill_rect(header, background);
    }
    let headers = if tree.headers.is_empty() {
        vec!["1".to_string()]
    } else {
        tree.headers.clone()
    };
    painter.draw_text(
        QRectF::new(
            viewport.x + 4.0 - tree.horizontal,
            viewport.y + ((QVIEW_HEADER_H - line_height) / 2.0).max(0.0),
            (viewport.width + tree.horizontal).max(1.0),
            line_height,
        ),
        headers.join("  "),
        header_color,
    );
    painter.restore();

    let text_color = item_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(body));
    for index in visible_rows(tree.scroll, body.height, tree.rows.len()) {
        let row = &tree.rows[index];
        let y = body.y + tree.row_y(index);
        let selected = tree.selected.contains(&index);
        let row_rect = QRectF::new(body.x, y, body.width, QVIEW_ROW_H);
        if selected {
            painter.fill_rect(
                row_rect,
                item_qss
                    .selection_background
                    .unwrap_or_else(|| palette.color(QPaletteRole::Highlight)),
            );
        } else if let Some(background) = item_qss.background {
            painter.fill_rect(row_rect, background);
        }
        if widget.has_focus && tree.current == Some(index) {
            paint_dotted_rect(
                painter,
                QRectF::new(
                    row_rect.x + 1.0,
                    row_rect.y + 1.0,
                    (row_rect.width - 2.0).max(1.0),
                    (row_rect.height - 2.0).max(1.0),
                ),
                palette.color(QPaletteRole::WindowText).with_alpha(0.72),
            );
        }
        let text_x = body.x + 4.0 + row.indent() + QVIEW_BRANCH_W - tree.horizontal;
        if row.has_children {
            let expanded = tree
                .items
                .get(row.path[0])
                .and_then(|_| {
                    // Walk the path to read the item's expanded flag.
                    let mut items = &tree.items;
                    let mut item = None;
                    for index in &row.path {
                        item = items.get(*index);
                        items = &item?.children;
                    }
                    item
                })
                .is_some_and(|item| item.expanded);
            let center_x = body.x + 2.0 + row.indent() + 4.0 - tree.horizontal;
            let center_y = y + QVIEW_ROW_H / 2.0;
            let half = 3.0;
            let color = text_color.with_alpha(0.75);
            if expanded {
                painter.fill_triangle(
                    QPointF::new(center_x - half, center_y - half / 2.0),
                    QPointF::new(center_x + half, center_y - half / 2.0),
                    QPointF::new(center_x, center_y + half / 2.0 + 1.0),
                    color,
                );
            } else {
                painter.fill_triangle(
                    QPointF::new(center_x - half / 2.0, center_y - half),
                    QPointF::new(center_x - half / 2.0, center_y + half),
                    QPointF::new(center_x + half / 2.0 + 1.0, center_y),
                    color,
                );
            }
        }
        painter.draw_text(
            QRectF::new(
                text_x,
                y + ((QVIEW_ROW_H - line_height) / 2.0).max(0.0),
                (tree.content_width + 8.0).max(1.0),
                line_height,
            ),
            &row.display,
            if selected { selected_text } else { text_color },
        );
    }
    painter.restore();

    painter.stroke_rect(rect, 1.0, palette.color(QPaletteRole::Mid));
    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, bar, &tree.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, bar, &tree.hbar, palette, widget.enabled);
    }
}

fn visible_rows(scroll: f32, height: f32, count: usize) -> std::ops::Range<usize> {
    if height <= 0.0 || count == 0 {
        return 0..0;
    }
    let first = (scroll / QVIEW_ROW_H).floor().max(0.0) as usize;
    let visible = (height / QVIEW_ROW_H).ceil() as usize + 1;
    first.min(count)..(first + visible).min(count)
}

fn paint_control_indicator(widget: &QWidget, painter: &mut QPainter) {
    let checked = property_bool(widget, "checked");
    if widget.class_name == "QRadioButton" {
        let size = widget
            .font
            .pixel_size
            .min(widget.geometry.height - 4.0)
            .max(9.0);
        let x = widget.geometry.x.round();
        let y = (widget.geometry.y + (widget.geometry.height - size) / 2.0).round();
        let outline = if !widget.enabled {
            widget.palette.color(QPaletteRole::Mid)
        } else if widget.has_focus {
            widget.palette.color(QPaletteRole::Highlight).darker(110)
        } else {
            widget.palette.color(QPaletteRole::Window).darker(150)
        };
        painter.fill_ellipse(
            QRectF::new(x, y, size, size),
            widget.palette.color(QPaletteRole::Base),
        );
        painter.stroke_ellipse(QRectF::new(x, y, size, size), 1.0, outline);
        if checked {
            let parity = ((size as i32 + 1) % 2) as f32;
            let outline_radius = (size + parity) / 2.0 - 1.0;
            let radius = outline_radius / 1.8;
            let center_x = x + size / 2.0;
            let center_y = y + size / 2.0;
            painter.fill_ellipse(
                QRectF::new(
                    center_x - radius,
                    center_y - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                widget
                    .palette
                    .color(QPaletteRole::Text)
                    .darker(120)
                    .with_alpha(180.0 / 255.0),
            );
        }
        return;
    }
    if !checked || widget.class_name != "QCheckBox" {
        return;
    }
    let size = widget
        .font
        .pixel_size
        .min(widget.geometry.height - 4.0)
        .max(9.0);
    let x = widget.geometry.x;
    let y = widget.geometry.y + (widget.geometry.height - size) / 2.0;
    let width = (size - 1.0).max(1.0);
    let padding = 1.0 + width * 0.13;
    let pen_width = (width * 0.13).max(1.5).min(width * 0.20);
    let color = if widget.enabled {
        widget
            .palette
            .color(QPaletteRole::Text)
            .darker(120)
            .with_alpha(210.0 / 255.0)
    } else {
        widget.palette.color(QPaletteRole::Mid)
    };
    let first = QPointF::new(x + padding + width * 0.11 - 0.8, y + width * 0.47 + 0.5);
    let second = QPointF::new(x + width * 0.5 - 0.8, y + width - padding + 0.5);
    let third = QPointF::new(x + width - padding - 0.8, y + padding + 0.5);
    painter.draw_line(first, second, pen_width, color);
    painter.draw_line(second, third, pen_width, color);
}

fn paint_table(
    widget: &QWidget,
    painter: &mut QPainter,
    item_qss: &QStyleDeclarations,
    header_qss: &QStyleDeclarations,
) {
    if widget.class_name != "QTableWidget" {
        return;
    }
    let Some(table) = widget.table.as_deref() else {
        return;
    };
    let rect = widget.geometry;
    let palette = &widget.palette;
    let layout = table.scroll_area(rect, &widget.font);
    let viewport = layout.viewport;
    let left = viewport.x;
    let top = viewport.y;
    let right = viewport.right();
    let bottom = viewport.bottom();
    let header_h = QTABLE_HEADER_H;
    let row_header_w = table.row_header_width(&widget.font);
    let body_x = left + row_header_w;
    let body_top = top + header_h;
    let rows_h = (bottom - body_top).max(0.0);
    let content_h =
        (table.model.row_count() as f32 * QTABLE_ROW_H - table.scroll).clamp(0.0, rows_h);
    let content_bottom = body_top + content_h;
    let rows = table.visible_row_range(viewport.height);
    let metrics = QFontMetrics::new(&widget.font);
    let line_height = metrics.height();
    let column_left = |column: usize| body_x + table.model.column_offset(column) - table.horizontal;

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        top,
        (right - left).max(0.0),
        header_h,
    )));
    let row_header_rect = QRectF::new(left, top, row_header_w, header_h);
    paint_header_section(widget, painter, row_header_rect);
    if let Some(background) = header_qss.background {
        painter.fill_rect(row_header_rect, background);
    }
    let mut header_x = body_x;
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        let x = column_left(column);
        let section = QRectF::new(x, top, width, header_h);
        paint_header_section(widget, painter, section);
        if let Some(background) = header_qss.background {
            painter.fill_rect(section, background);
        }
        header_x = x + width;
    }
    if header_x < right {
        painter.fill_rect(
            QRectF::new(header_x, top, (right - header_x).max(0.0), header_h),
            palette.color(QPaletteRole::Window),
        );
    }
    painter.restore();

    if body_top < bottom {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            left,
            body_top,
            (body_x - left).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        painter.fill_rect(
            QRectF::new(left, body_top, row_header_w, (bottom - body_top).max(0.0)),
            palette.color(QPaletteRole::Button),
        );
        for row in rows.clone() {
            let y = viewport.y + table.row_y(row);
            let row_top = y.max(body_top);
            let row_bottom = (y + QTABLE_ROW_H).min(bottom);
            if row_bottom > row_top {
                let section = QRectF::new(left, row_top, row_header_w, row_bottom - row_top);
                paint_header_gradient(widget, painter, section);
                if let Some(background) = header_qss.background {
                    painter.fill_rect(section, background);
                }
            }
        }
        painter.restore();
    }

    let mut focus_border: Option<(QRectF, QColor)> = None;
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        body_x,
        body_top,
        (right - body_x).max(0.0),
        (bottom - body_top).max(0.0),
    )));
    if let Some(background) = item_qss.background {
        for row in rows.clone() {
            let top_y = (viewport.y + table.row_y(row)).max(body_top);
            let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
            if bottom_y <= top_y {
                continue;
            }
            for column in 0..table.model.column_count() {
                if table.model.is_column_hidden(column) {
                    continue;
                }
                painter.fill_rect(
                    QRectF::new(
                        column_left(column),
                        top_y,
                        table.model.column_width(column),
                        bottom_y - top_y,
                    ),
                    background,
                );
            }
        }
    }
    for (row, column) in table.selection.selected_cells() {
        if row < rows.start
            || row >= rows.end
            || column >= table.model.column_count()
            || table.model.is_column_hidden(column)
        {
            continue;
        }
        let top_y = (viewport.y + table.row_y(row)).max(body_top);
        let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
        if bottom_y <= top_y {
            continue;
        }
        painter.fill_rect(
            QRectF::new(
                column_left(column),
                top_y,
                table.model.column_width(column),
                bottom_y - top_y,
            ),
            palette.color(QPaletteRole::Highlight),
        );
    }

    if widget.has_focus {
        if let Some((row, column)) = table.current() {
            if row >= rows.start && row < rows.end && !table.model.is_column_hidden(column) {
                let top_y = (viewport.y + table.row_y(row)).max(body_top);
                let bottom_y = (viewport.y + table.row_y(row) + QTABLE_ROW_H).min(content_bottom);
                if bottom_y > top_y {
                    let cell = QRectF::new(
                        column_left(column),
                        top_y,
                        table.model.column_width(column),
                        bottom_y - top_y,
                    );
                    let mut fill = palette.color(QPaletteRole::Highlight).darker(125);
                    if fill.value() > 160 {
                        fill = fill.with_value(160);
                    }
                    fill = fill.with_alpha(30.0 / 255.0);
                    painter.fill_linear_gradient_rect(cell, 0.0, fill.lighter(160), fill);
                    focus_border = Some((
                        QRectF::new(cell.x, cell.y, (cell.width - 1.0).max(1.0), cell.height),
                        QColor::rgb(0xb1, 0xc9, 0xd7),
                    ));
                }
            }
        }
    }
    painter.restore();

    let grid = palette.color(QPaletteRole::Window).darker(120);
    let header_grid = palette.color(QPaletteRole::Window).darker(140);
    if content_h > 0.0 {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            body_x,
            body_top,
            (right - body_x).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        let mut horizontal: Vec<f32> = rows
            .clone()
            .filter_map(|row| {
                let y = viewport.y + table.row_y(row);
                (y >= body_top - 0.5 && y < content_bottom).then_some(y)
            })
            .collect();
        if let Some(last) = rows.clone().last() {
            horizontal.push(viewport.y + table.row_y(last) + QTABLE_ROW_H);
        }
        for y in &horizontal {
            painter.fill_rect(
                QRectF::new(body_x, *y, (right - body_x).max(0.0), 1.0),
                grid,
            );
        }
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            let x = column_left(column) - 1.0;
            painter.fill_rect(QRectF::new(x, body_top, 1.0, content_h), grid);
        }
        let last_x = if table.model.column_count() == 0 {
            body_x
        } else {
            column_left(table.model.column_count() - 1)
                + table.model.column_width(table.model.column_count() - 1)
        };
        painter.fill_rect(QRectF::new(last_x, body_top, 1.0, content_h), grid);
        painter.fill_rect(
            QRectF::new(body_x - 1.0, body_top, 1.0, content_h),
            header_grid,
        );
        painter.restore();

        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            left,
            body_top,
            (body_x - left).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        for y in &horizontal {
            painter.fill_rect(QRectF::new(left, *y, row_header_w, 1.0), header_grid);
        }
        painter.restore();
    }

    if let Some((rect, color)) = focus_border {
        painter.save();
        painter.set_clip_rect(Some(QRectF::new(
            body_x,
            body_top,
            (right - body_x).max(0.0),
            (bottom - body_top).max(0.0),
        )));
        painter.stroke_rounded_rect(rect, 1.0, 1.0, color);
        painter.restore();
    }

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        top,
        (right - left).max(0.0),
        header_h,
    )));
    painter.fill_rect(
        QRectF::new(left, body_top - 1.0, (right - left).max(0.0), 1.0),
        header_grid,
    );
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let x = column_left(column) - 1.0;
        painter.fill_rect(QRectF::new(x, top, 1.0, header_h), header_grid);
    }
    let last_x = if table.model.column_count() == 0 {
        body_x
    } else {
        column_left(table.model.column_count() - 1)
            + table.model.column_width(table.model.column_count() - 1)
    };
    painter.fill_rect(QRectF::new(last_x, top, 1.0, header_h), header_grid);

    let header_color = header_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::ButtonText));
    for column in 0..table.model.column_count() {
        if table.model.is_column_hidden(column) {
            continue;
        }
        let width = table.model.column_width(column);
        let x = column_left(column);
        let label = table
            .model
            .header(column)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| (column + 1).to_string());
        let label_width = metrics.horizontal_advance(&label);
        painter.draw_text(
            QRectF::new(
                x + ((width - label_width) / 2.0).max(2.0),
                top + ((header_h - line_height) / 2.0).max(0.0),
                (width - 4.0).max(1.0),
                line_height,
            ),
            label,
            header_color,
        );
        if let Some((sort_column, order)) = table.sort_indicator {
            if sort_column == column {
                painter.draw_text(
                    QRectF::new(
                        x + width - 12.0,
                        top + ((header_h - line_height) / 2.0).max(0.0),
                        10.0,
                        line_height,
                    ),
                    if order == QSortOrder::Ascending {
                        "▲"
                    } else {
                        "▼"
                    },
                    header_color,
                );
            }
        }
    }
    painter.restore();

    let text_color = item_qss
        .color
        .unwrap_or_else(|| palette.color(QPaletteRole::Text));
    let selected_text = item_qss
        .selection_color
        .unwrap_or_else(|| palette.color(QPaletteRole::HighlightedText));
    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        left,
        body_top,
        (body_x - left).max(0.0),
        (content_bottom - body_top).max(0.0),
    )));
    for row in rows.clone() {
        let raw_y = viewport.y + table.row_y(row);
        let row_number = (row + 1).to_string();
        let number_width = metrics.horizontal_advance(&row_number);
        painter.draw_text(
            QRectF::new(
                left + ((row_header_w - number_width) / 2.0).max(2.0),
                raw_y + ((QTABLE_ROW_H - line_height) / 2.0).max(0.0),
                (row_header_w - 4.0).max(1.0),
                line_height,
            ),
            row_number,
            text_color,
        );
    }
    painter.restore();

    painter.save();
    painter.set_clip_rect(Some(QRectF::new(
        body_x,
        body_top,
        (right - body_x).max(0.0),
        (content_bottom - body_top).max(0.0),
    )));
    for row in rows {
        let raw_y = viewport.y + table.row_y(row);
        let row_bottom = (raw_y + QTABLE_ROW_H).min(content_bottom);
        for column in 0..table.model.column_count() {
            if table.model.is_column_hidden(column) {
                continue;
            }
            let width = table.model.column_width(column);
            let column_x = column_left(column);
            let editing = table
                .editor
                .as_ref()
                .filter(|editor| editor.row == row && editor.column == column);
            let text = editing
                .map(|editor| editor.text.as_str())
                .or_else(|| table.model.cell(row, column));
            let is_selected = table.is_selected(row, column);
            if let Some(text) = text {
                painter.draw_text(
                    QRectF::new(
                        column_x + 3.0,
                        raw_y + ((QTABLE_ROW_H - line_height) / 2.0).max(0.0),
                        (width - 6.0).max(1.0),
                        line_height,
                    ),
                    text,
                    if is_selected {
                        selected_text
                    } else {
                        text_color
                    },
                );
            }
            if let Some(editor) = editing {
                let caret_y = raw_y + 3.0;
                let caret_h = (QTABLE_ROW_H - 6.0).min(row_bottom - caret_y).max(0.0);
                if caret_h > 0.0 {
                    painter.fill_rect(
                        QRectF::new(
                            (column_x + 3.0 + metrics.horizontal_advance(&editor.text))
                                .min(column_x + width - 2.0),
                            caret_y.max(body_top),
                            1.0,
                            caret_h,
                        ),
                        text_color,
                    );
                }
            }
        }
    }
    painter.restore();

    if let Some(bar) = layout.vertical {
        paint_scroll_bar(painter, bar, &table.vbar, palette, widget.enabled);
    }
    if let Some(bar) = layout.horizontal {
        paint_scroll_bar(painter, bar, &table.hbar, palette, widget.enabled);
    }
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
        painter.draw_text(
            QRectF::new(base_x, y, content.width, line_height),
            text,
            color,
        );
        return;
    }
    let prefix: String = text.chars().take(start).collect();
    let selected: String = text.chars().take(end).skip(start).collect();
    let suffix: String = text.chars().skip(end).collect();
    let prefix_width = metrics.horizontal_advance(&prefix);
    let selected_width = metrics.horizontal_advance(&selected);
    if !prefix.is_empty() {
        painter.draw_text(
            QRectF::new(base_x, y, prefix_width.max(1.0), line_height),
            prefix,
            color,
        );
    }
    if !selected.is_empty() {
        painter.draw_text(
            QRectF::new(
                base_x + prefix_width,
                y,
                selected_width.max(1.0),
                line_height,
            ),
            selected,
            qss.selection_color
                .unwrap_or_else(|| widget.palette.color(QPaletteRole::HighlightedText)),
        );
    }
    if !suffix.is_empty() {
        painter.draw_text(
            QRectF::new(
                base_x + prefix_width + selected_width,
                y,
                (content.width - prefix_width - selected_width).max(1.0),
                line_height,
            ),
            suffix,
            color,
        );
    }
}

fn paint_widget_text(widget: &QWidget, painter: &mut QPainter, qss: &QStyleDeclarations) {
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
                let pad_left = qss.padding.map(|padding| padding.0).unwrap_or(8.0);
                let pad_right = qss.padding.map(|padding| padding.2).unwrap_or(8.0);
                painter.draw_text(
                    QRectF::new(
                        frame.x + pad_left,
                        frame.y + 2.0,
                        (frame.width - pad_left - pad_right).max(1.0),
                        line_height,
                    ),
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
        let area = QRectF::new(
            widget.geometry.right() - 20.5,
            widget.geometry.y - 2.0,
            19.0,
            widget.geometry.height + 4.0,
        );
        let width = 8.0_f32.min(area.width).min(area.height);
        let height = 5.5_f32.min(width);
        let center_x = area.x + area.width / 2.0;
        let center_y = area.y + area.height / 2.0;
        painter.fill_triangle(
            QPointF::new(center_x - width / 2.0, center_y - height / 2.0),
            QPointF::new(center_x + width / 2.0, center_y - height / 2.0),
            QPointF::new(center_x, center_y + height / 2.0),
            text_color(widget, qss).with_alpha(0.63),
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
    let mut rect = QRectF::new(
        content.x,
        content.y + ((content.height - line_height) / 2.0).max(0.0),
        text_width,
        line_height,
    );
    match widget.class_name.as_str() {
        "QPushButton" | "QToolButton" => {
            rect.x += ((content.width - text_width) / 2.0).max(0.0);
        }
        "QLineEdit" => {}
        "QCheckBox" | "QRadioButton" => {
            rect.x += widget.font.pixel_size + 6.0;
        }
        "QLabel" => match qss.text_align {
            Some(QTextAlign::Center) => rect.x += ((content.width - text_width) / 2.0).max(0.0),
            Some(QTextAlign::Right) => rect.x += (content.width - text_width).max(0.0),
            _ => {}
        },
        _ => {
            if qss.padding.is_none() {
                rect.x += 4.0;
            }
        }
    }
    let color = text_color(widget, qss);
    painter.draw_text(rect, text, color);
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
            painter.draw_text(
                QRectF::new(
                    text_x,
                    text_y,
                    (contents.right() - text_x).max(1.0),
                    line_height,
                ),
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
    painter.draw_text(
        QRectF::new(
            contents.x + ((contents.width - text_width) / 2.0).max(0.0) + shift,
            text_y,
            text_width.max(1.0),
            line_height,
        ),
        text,
        color,
    );
}

fn widget_text(widget: &QWidget) -> Option<String> {
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

fn text_color(widget: &QWidget, qss: &QStyleDeclarations) -> QColor {
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
    use crate::style::QFusionStyle;
    use crate::widgets::load_ui;

    const FIXTURE: &str = include_str!("../../form-xml/tests/fixtures/test_dialog01.ui");
    const DIALOG02: &str = include_str!("../../../../server/priv/qt_forms/test_dialog02.ui");
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

        let fills = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::FillRect { .. }))
            .count();
        assert!(fills >= 4);
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
        let text_x = geometry.x + font.pixel_size + 6.0;
        let style = QFusionStyle::default();
        let mut painter = QPainter::new();
        paint_widget_tree(&tree, root, &style, &mut painter);
        let dots = painter
            .commands()
            .iter()
            .filter(|command| matches!(command, QPaintCommand::FillRect { rect, .. }
                if (rect.width - 1.0).abs() < 0.01 && (rect.height - 1.0).abs() < 0.01
                    && rect.x >= text_x - 4.0 && rect.x <= text_x + metrics.horizontal_advance(text) + 4.0))
            .count();
        assert!(dots >= 8, "expected dotted focus rect, got {dots} dots");
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
    }
}
