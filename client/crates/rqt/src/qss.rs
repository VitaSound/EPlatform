use crate::core::QVariant;
use crate::gui::{QColor, QFontWeight};
use crate::widgets::QWidget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QTextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QBorderSide {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QBorderSideDecl {
    pub width: Option<f32>,
    pub color: Option<QColor>,
}

/// `qlineargradient(...)` from QSS. Coordinates are kept for future
/// non-vertical rendering; the current painter samples the stops along the
/// widget's vertical axis.
#[derive(Clone, Debug, PartialEq)]
pub struct QLinearGradient {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub stops: Vec<(f32, QColor)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QStyleDeclarations {
    pub background: Option<QColor>,
    pub color: Option<QColor>,
    pub border_width: Option<f32>,
    pub border_color: Option<QColor>,
    pub border_radius: Option<f32>,
    pub border_top: Option<QBorderSideDecl>,
    pub border_right: Option<QBorderSideDecl>,
    pub border_bottom: Option<QBorderSideDecl>,
    pub border_left: Option<QBorderSideDecl>,
    /// `(left, top, right, bottom)` in px.
    pub padding: Option<(f32, f32, f32, f32)>,
    /// `(left, top, right, bottom)` in px; outside the border.
    pub margin: Option<(f32, f32, f32, f32)>,
    pub background_gradient: Option<QLinearGradient>,
    pub selection_background: Option<QColor>,
    pub selection_color: Option<QColor>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<QFontWeight>,
    pub font_italic: Option<bool>,
    pub text_align: Option<QTextAlign>,
}

impl QStyleDeclarations {
    pub fn merge(&mut self, other: &Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.color.is_some() {
            self.color = other.color;
        }
        if other.border_width.is_some() {
            self.border_width = other.border_width;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }
        if other.border_radius.is_some() {
            self.border_radius = other.border_radius;
        }
        if other.border_top.is_some() {
            self.border_top = other.border_top;
        }
        if other.border_right.is_some() {
            self.border_right = other.border_right;
        }
        if other.border_bottom.is_some() {
            self.border_bottom = other.border_bottom;
        }
        if other.border_left.is_some() {
            self.border_left = other.border_left;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.margin.is_some() {
            self.margin = other.margin;
        }
        if other.background_gradient.is_some() {
            self.background_gradient = other.background_gradient.clone();
        }
        if other.selection_background.is_some() {
            self.selection_background = other.selection_background;
        }
        if other.selection_color.is_some() {
            self.selection_color = other.selection_color;
        }
        if other.font_family.is_some() {
            self.font_family = other.font_family.clone();
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.font_weight.is_some() {
            self.font_weight = other.font_weight;
        }
        if other.font_italic.is_some() {
            self.font_italic = other.font_italic;
        }
        if other.text_align.is_some() {
            self.text_align = other.text_align;
        }
    }

    pub fn has_side_borders(&self) -> bool {
        self.border_top.is_some()
            || self.border_right.is_some()
            || self.border_bottom.is_some()
            || self.border_left.is_some()
    }

    pub fn side_border(&self, side: QBorderSide) -> Option<&QBorderSideDecl> {
        match side {
            QBorderSide::Top => self.border_top.as_ref(),
            QBorderSide::Right => self.border_right.as_ref(),
            QBorderSide::Bottom => self.border_bottom.as_ref(),
            QBorderSide::Left => self.border_left.as_ref(),
        }
    }

    fn side_border_mut(&mut self, side: QBorderSide) -> &mut Option<QBorderSideDecl> {
        match side {
            QBorderSide::Top => &mut self.border_top,
            QBorderSide::Right => &mut self.border_right,
            QBorderSide::Bottom => &mut self.border_bottom,
            QBorderSide::Left => &mut self.border_left,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QStyleSheet {
    rules: Vec<QStyleRule>,
}

#[derive(Clone, Debug, PartialEq)]
struct QStyleRule {
    selectors: Vec<String>,
    declarations: QStyleDeclarations,
}

impl QStyleSheet {
    pub fn parse(text: &str) -> Self {
        let text = strip_comments(text);
        let mut rules = Vec::new();
        let mut rest = text.as_str();
        while let Some(open) = rest.find('{') {
            let selector_text = rest[..open].trim();
            let Some(close) = rest[open + 1..].find('}') else {
                break;
            };
            let body = &rest[open + 1..open + 1 + close];
            if !selector_text.is_empty() {
                let selectors = selector_text
                    .split(',')
                    .map(|selector| selector.trim().to_string())
                    .filter(|selector| !selector.is_empty())
                    .collect();
                rules.push(QStyleRule {
                    selectors,
                    declarations: parse_declarations(body),
                });
            }
            rest = &rest[open + 1 + close + 1..];
        }
        Self { rules }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn resolve(&self, widget: &QWidget) -> QStyleDeclarations {
        let mut declarations = QStyleDeclarations::default();
        for rule in &self.rules {
            if rule
                .selectors
                .iter()
                .any(|selector| selector_matches(selector, widget))
            {
                declarations.merge(&rule.declarations);
            }
        }
        declarations
    }

    /// Resolve a sub-control (`item`, `section`, `view`, `handle`, …).
    pub fn resolve_part(&self, widget: &QWidget, part: &str) -> QStyleDeclarations {
        let mut declarations = QStyleDeclarations::default();
        for rule in &self.rules {
            if rule
                .selectors
                .iter()
                .any(|selector| selector_matches_part(selector, widget, part))
            {
                declarations.merge(&rule.declarations);
            }
        }
        declarations
    }
}

/// Split a selector into base, optional `::part`, `[...]` attributes and
/// `:state` pseudo classes. Sub-controls (`QTreeWidget::item`) are not
/// pseudo states, so they must be recognized before single-colon parsing.
fn split_selector(selector: &str) -> (String, Option<String>, Vec<String>, Vec<String>) {
    let chars: Vec<char> = selector.trim().chars().collect();
    let mut base = String::new();
    let mut part = None;
    let mut attributes = Vec::new();
    let mut states = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '[' {
            i += 1;
            let mut attribute = String::new();
            while i < chars.len() && chars[i] != ']' {
                attribute.push(chars[i]);
                i += 1;
            }
            attributes.push(attribute.trim().to_string());
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        if ch == ':' {
            if i + 1 < chars.len() && chars[i + 1] == ':' {
                i += 2;
                let mut name = String::new();
                while i < chars.len()
                    && chars[i] != ':'
                    && chars[i] != '['
                    && !chars[i].is_whitespace()
                {
                    name.push(chars[i]);
                    i += 1;
                }
                part = Some(name.trim().to_ascii_lowercase());
            } else {
                i += 1;
                let mut state = String::new();
                while i < chars.len() && chars[i] != ':' && chars[i] != '[' {
                    state.push(chars[i]);
                    i += 1;
                }
                states.push(state.trim().to_string());
            }
            continue;
        }
        base.push(ch);
        i += 1;
    }
    (base.trim().to_string(), part, attributes, states)
}

fn base_matches(base: &str, widget: &QWidget) -> bool {
    let base = base.strip_prefix('.').unwrap_or(base);
    let (class, object_name) = match base.split_once('#') {
        Some((class, name)) => (class.trim(), Some(name.trim())),
        None => (base.trim(), None),
    };
    if !(class.is_empty()
        || class == "*"
        || class == widget.class_name
        || (class == "QWidget" && widget.class_name == "QWidget"))
    {
        return false;
    }
    if let Some(name) = object_name {
        if widget.object_name != name {
            return false;
        }
    }
    true
}

fn filters_match(widget: &QWidget, attributes: &[String], states: &[String]) -> bool {
    for attribute in attributes {
        if !attribute_matches(widget, attribute) {
            return false;
        }
    }
    for state in states {
        let (negated, state) = match state.strip_prefix('!') {
            Some(state) => (true, state.trim()),
            None => (false, state.as_str()),
        };
        let matches = match state {
            "" | "enabled" => widget.enabled,
            "disabled" => !widget.enabled,
            "focus" => widget.has_focus,
            "hover" => widget.under_mouse,
            "pressed" => widget.pressed,
            "default" => widget.default_button,
            "checked" | "selected" | "on" => {
                matches!(widget.property("checked"), Some(QVariant::Bool(true)))
            }
            "open" => widget.combo_popup.is_some(),
            _ => true,
        };
        if matches == negated {
            return false;
        }
    }
    true
}

fn selector_matches(selector: &str, widget: &QWidget) -> bool {
    let (base, part, attributes, states) = split_selector(selector);
    if part.is_some() {
        return false;
    }
    base_matches(&base, widget) && filters_match(widget, &attributes, &states)
}

fn selector_matches_part(selector: &str, widget: &QWidget, part: &str) -> bool {
    let (base, selector_part, attributes, states) = split_selector(selector);
    let tokens: Vec<&str> = base.split_whitespace().collect();
    let Some(target) = tokens.last().copied() else {
        return false;
    };
    let explicit_part = match selector_part.as_deref() {
        Some(selector_part) => selector_part == part,
        // `QComboBox QAbstractItemView` targets the popup view without `::`.
        None => part == "view" && target == "QAbstractItemView",
    };
    if !explicit_part {
        return false;
    }

    let base_ok = if tokens.len() > 1 {
        let ancestor = tokens[..tokens.len() - 1].join(" ");
        part == "view"
            && target == "QAbstractItemView"
            && ancestor == "QComboBox"
            && widget.class_name == "QComboBox"
    } else if part == "section" && target == "QHeaderView" {
        matches!(widget.class_name.as_str(), "QTableWidget" | "QTreeWidget")
    } else if part == "view" && target == "QAbstractItemView" {
        matches!(widget.class_name.as_str(), "QComboBox")
    } else {
        base_matches(target, widget)
    };
    base_ok && filters_match(widget, &attributes, &states)
}

fn attribute_matches(widget: &QWidget, attribute: &str) -> bool {
    let (name, expected) = match attribute.split_once('=') {
        Some((name, expected)) => (name.trim(), Some(expected.trim().trim_matches('"'))),
        None => (attribute.trim(), None),
    };
    let Some(property) = widget.property(name) else {
        return false;
    };
    let Some(expected) = expected else {
        return true;
    };
    match property {
        QVariant::String(text) | QVariant::Enum(text) => text == expected,
        QVariant::Bool(value) => value.to_string() == expected,
        QVariant::Int(value) => value.to_string() == expected,
        QVariant::Double(value) => value.to_string() == expected,
        _ => false,
    }
}

fn parse_declarations(body: &str) -> QStyleDeclarations {
    let mut declarations = QStyleDeclarations::default();
    for declaration in body.split(';') {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "background" | "background-color" => {
                if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("transparent") {
                    declarations.background = Some(QColor::rgba8(0, 0, 0, 0));
                } else if let Some(gradient) = parse_linear_gradient(value) {
                    declarations.background_gradient = Some(gradient);
                } else if let Some(color) = parse_color(value) {
                    declarations.background = Some(color);
                }
            }
            "background-image" | "border-image" => {
                if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("transparent") {
                    declarations.background_gradient = None;
                } else if let Some(gradient) = parse_linear_gradient(value) {
                    declarations.background_gradient = Some(gradient);
                }
            }
            "color" => {
                if let Some(color) = parse_color(value) {
                    declarations.color = Some(color);
                }
            }
            "border" => {
                let side = parse_border_side(value);
                if let Some(width) = side.width {
                    declarations.border_width = Some(width);
                }
                if let Some(color) = side.color {
                    declarations.border_color = Some(color);
                }
            }
            "border-style" => {
                let (width, _) = border_style_width(value);
                if let Some(width) = width {
                    declarations.border_width = Some(width);
                }
            }
            "border-width" => {
                if let Some(width) = parse_px(value) {
                    declarations.border_width = Some(width);
                }
            }
            "border-color" => {
                if let Some(color) = parse_color(value) {
                    declarations.border_color = Some(color);
                }
            }
            "border-radius" => {
                if let Some(radius) = parse_px(value) {
                    declarations.border_radius = Some(radius);
                }
            }
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                let side = match name.as_str() {
                    "border-top" => QBorderSide::Top,
                    "border-right" => QBorderSide::Right,
                    "border-bottom" => QBorderSide::Bottom,
                    _ => QBorderSide::Left,
                };
                merge_side_decl(declarations.side_border_mut(side), parse_border_side(value));
            }
            name if name.starts_with("border-") && name.ends_with("-width") => {
                if let Some(side) = border_side_name(&name[..name.len() - 6]) {
                    let width = parse_px(value);
                    merge_side_decl(
                        declarations.side_border_mut(side),
                        QBorderSideDecl { width, color: None },
                    );
                }
            }
            name if name.starts_with("border-") && name.ends_with("-color") => {
                if let Some(side) = border_side_name(&name[..name.len() - 6]) {
                    let color = parse_color(value);
                    merge_side_decl(
                        declarations.side_border_mut(side),
                        QBorderSideDecl { width: None, color },
                    );
                }
            }
            "padding" => {
                declarations.padding = parse_box_sides(value).or(declarations.padding);
            }
            "margin" => {
                declarations.margin = parse_box_sides(value).or(declarations.margin);
            }
            name if name.starts_with("padding-") => {
                if let Some(index) = box_side_index(&name["padding-".len()..]) {
                    declarations.padding =
                        set_box_side(declarations.padding, index, parse_px(value));
                }
            }
            name if name.starts_with("margin-") => {
                if let Some(index) = box_side_index(&name["margin-".len()..]) {
                    declarations.margin = set_box_side(declarations.margin, index, parse_px(value));
                }
            }
            "selection-background-color" => {
                if let Some(color) = parse_color(value) {
                    declarations.selection_background = Some(color);
                }
            }
            "selection-color" => {
                if let Some(color) = parse_color(value) {
                    declarations.selection_color = Some(color);
                }
            }
            "font" => parse_font_shorthand(value, &mut declarations),
            "font-family" => {
                let family = value
                    .split(',')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                if !family.is_empty() {
                    declarations.font_family = Some(family.to_string());
                }
            }
            "font-size" => {
                if let Some(size) = parse_font_size(value) {
                    declarations.font_size = Some(size);
                }
            }
            "font-style" => match value.to_ascii_lowercase().as_str() {
                "italic" | "oblique" => declarations.font_italic = Some(true),
                "normal" => declarations.font_italic = Some(false),
                _ => {}
            },
            "font-weight" => {
                let weight = match value.to_ascii_lowercase().as_str() {
                    "normal" => Some(QFontWeight::Normal),
                    "bold" => Some(QFontWeight::Bold),
                    "medium" => Some(QFontWeight::Medium),
                    value => value.parse::<u16>().ok().and_then(|value| match value {
                        100 => Some(QFontWeight::Thin),
                        200 => Some(QFontWeight::ExtraLight),
                        300 => Some(QFontWeight::Light),
                        400 => Some(QFontWeight::Normal),
                        500 => Some(QFontWeight::Medium),
                        600 => Some(QFontWeight::DemiBold),
                        700 => Some(QFontWeight::Bold),
                        800 => Some(QFontWeight::ExtraBold),
                        900 => Some(QFontWeight::Black),
                        _ => None,
                    }),
                };
                declarations.font_weight = weight;
            }
            "text-align" => {
                declarations.text_align = match value.to_ascii_lowercase().as_str() {
                    "left" => Some(QTextAlign::Left),
                    "center" | "middle" => Some(QTextAlign::Center),
                    "right" => Some(QTextAlign::Right),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    declarations
}

/// QSS 1/2/3/4-value box shorthand → `(left, top, right, bottom)`.
fn parse_box_sides(value: &str) -> Option<(f32, f32, f32, f32)> {
    let values: Vec<f32> = value.split_whitespace().filter_map(parse_px).collect();
    match values.as_slice() {
        [all] => Some((*all, *all, *all, *all)),
        [vertical, horizontal] => Some((*horizontal, *vertical, *horizontal, *vertical)),
        [top, horizontal, bottom] => Some((*horizontal, *top, *horizontal, *bottom)),
        [top, right, bottom, left] => Some((*left, *top, *right, *bottom)),
        _ => None,
    }
}

fn box_side_index(side: &str) -> Option<usize> {
    match side {
        "left" => Some(0),
        "top" => Some(1),
        "right" => Some(2),
        "bottom" => Some(3),
        _ => None,
    }
}

fn set_box_side(
    current: Option<(f32, f32, f32, f32)>,
    index: usize,
    value: Option<f32>,
) -> Option<(f32, f32, f32, f32)> {
    let Some(value) = value else {
        return current;
    };
    let mut sides = current.unwrap_or((0.0, 0.0, 0.0, 0.0));
    match index {
        0 => sides.0 = value,
        1 => sides.1 = value,
        2 => sides.2 = value,
        _ => sides.3 = value,
    }
    Some(sides)
}

fn parse_linear_gradient(value: &str) -> Option<QLinearGradient> {
    let lower = value.to_ascii_lowercase();
    let start = lower.find("qlineargradient(")?;
    let open = start + "qlineargradient".len();
    let close = value.rfind(')')?;
    if close <= open {
        return None;
    }
    let body = &value[open + 1..close];
    let mut gradient = QLinearGradient {
        x1: 0.0,
        y1: 0.0,
        x2: 0.0,
        y2: 1.0,
        stops: Vec::new(),
    };
    for token in split_top_level_commas(body) {
        let Some((name, value)) = token.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "x1" => gradient.x1 = parse_coordinate(value),
            "y1" => gradient.y1 = parse_coordinate(value),
            "x2" => gradient.x2 = parse_coordinate(value),
            "y2" => gradient.y2 = parse_coordinate(value),
            "stop" => {
                let mut parts = value.splitn(2, char::is_whitespace);
                let offset = parts
                    .next()
                    .and_then(|offset| offset.trim_end_matches('%').parse::<f32>().ok())
                    .map(|offset| {
                        if value.contains('%') {
                            offset / 100.0
                        } else {
                            offset
                        }
                    })
                    .unwrap_or(0.0);
                if let Some(color) = parts.next().and_then(parse_color) {
                    gradient.stops.push((offset.clamp(0.0, 1.0), color));
                }
            }
            _ => {}
        }
    }
    gradient
        .stops
        .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    (!gradient.stops.is_empty()).then_some(gradient)
}

fn parse_coordinate(value: &str) -> f32 {
    let value = value.trim();
    let number = value.trim_end_matches('%').parse::<f32>().unwrap_or(0.0);
    if value.ends_with('%') {
        number / 100.0
    } else {
        number
    }
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn border_side_name(value: &str) -> Option<QBorderSide> {
    match value {
        "border-top" => Some(QBorderSide::Top),
        "border-right" => Some(QBorderSide::Right),
        "border-bottom" => Some(QBorderSide::Bottom),
        "border-left" => Some(QBorderSide::Left),
        _ => None,
    }
}

fn merge_side_decl(target: &mut Option<QBorderSideDecl>, source: QBorderSideDecl) {
    let entry = target.get_or_insert_with(QBorderSideDecl::default);
    if source.width.is_some() {
        entry.width = source.width;
    }
    if source.color.is_some() {
        entry.color = source.color;
    }
}

fn border_style_width(value: &str) -> (Option<f32>, bool) {
    let lower = value.to_ascii_lowercase();
    if lower.contains("none") || lower.contains("hidden") {
        (Some(0.0), true)
    } else if lower.contains("solid")
        || lower.contains("dashed")
        || lower.contains("dotted")
        || lower.contains("double")
    {
        (
            Some(value.split_whitespace().find_map(parse_px).unwrap_or(1.0)),
            false,
        )
    } else {
        (value.split_whitespace().find_map(parse_px), false)
    }
}

fn parse_border_side(value: &str) -> QBorderSideDecl {
    let mut side = QBorderSideDecl::default();
    let lower = value.to_ascii_lowercase();
    if lower.contains("none") || lower.contains("hidden") {
        side.width = Some(0.0);
    } else if let Some(width) = value.split_whitespace().find_map(parse_px) {
        side.width = Some(width);
    } else if lower.contains("solid")
        || lower.contains("dashed")
        || lower.contains("dotted")
        || lower.contains("double")
    {
        side.width = Some(1.0);
    }
    side.color = parse_color(value);
    side
}

fn parse_font_shorthand(value: &str, declarations: &mut QStyleDeclarations) {
    let tokens = split_font_tokens(value);
    let mut family = Vec::new();
    for token in tokens {
        match token.to_ascii_lowercase().as_str() {
            "italic" | "oblique" => declarations.font_italic = Some(true),
            "normal" => declarations.font_italic = Some(false),
            "bold" => declarations.font_weight = Some(QFontWeight::Bold),
            "thin" => declarations.font_weight = Some(QFontWeight::Thin),
            "medium" => declarations.font_weight = Some(QFontWeight::Medium),
            "black" => declarations.font_weight = Some(QFontWeight::Black),
            _ => {
                if let Some(size) = parse_font_size(&token) {
                    declarations.font_size = Some(size);
                } else {
                    family.push(token.trim_matches('"').trim_matches('\'').to_string());
                }
            }
        }
    }
    if !family.is_empty() {
        declarations.font_family = Some(family.join(" "));
    }
}

fn split_font_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in value.chars() {
        if let Some(q) = quote {
            if ch == q {
                tokens.push(current.clone());
                current.clear();
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_font_size(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(number) = value.strip_suffix("px") {
        number.trim().parse::<f32>().ok()
    } else if let Some(number) = value.strip_suffix("pt") {
        number
            .trim()
            .parse::<f32>()
            .ok()
            .map(|points| points * 4.0 / 3.0)
    } else {
        value.parse::<f32>().ok()
    }
}

fn parse_color(value: &str) -> Option<QColor> {
    let value = value.trim();
    if let Some(color) = parse_color_function(value) {
        return Some(color);
    }
    let lower = value.to_ascii_lowercase();
    for prefix in ["rgba(", "rgb("] {
        if let Some(start) = lower.find(prefix) {
            if let Some(end) = value[start..].find(')') {
                if let Some(color) = parse_color_function(&value[start..start + end + 1]) {
                    return Some(color);
                }
            }
        }
    }
    // In a shorthand such as `border: 1px solid red` or
    // `background-color: transparent none`, pick the color token.
    for token in value.split_whitespace() {
        if let Some(color) = named_color(token) {
            return Some(color);
        }
        if let Some(color) = QColor::from_hex(token) {
            return Some(color);
        }
    }
    None
}

fn parse_color_function(value: &str) -> Option<QColor> {
    let lower = value.to_ascii_lowercase();
    let (inner, alpha) = if let Some(inner) = lower.strip_prefix("rgba(") {
        (inner.strip_suffix(')')?, true)
    } else if let Some(inner) = lower.strip_prefix("rgb(") {
        (inner.strip_suffix(')')?, false)
    } else {
        return None;
    };
    let mut parts = inner.split(',').map(str::trim);
    let red = parse_channel(parts.next()?)?;
    let green = parse_channel(parts.next()?)?;
    let blue = parse_channel(parts.next()?)?;
    let alpha = if alpha {
        parts.next().and_then(parse_alpha).unwrap_or(255)
    } else {
        255
    };
    Some(QColor::rgba8(red, green, blue, alpha))
}

fn parse_channel(value: &str) -> Option<u8> {
    if let Some(percent) = value.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|percent| (percent / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8)
    } else {
        let value = value.trim().parse::<f32>().ok()?;
        Some(value.round().clamp(0.0, 255.0) as u8)
    }
}

fn parse_alpha(value: &str) -> Option<u8> {
    let value = value.trim();
    let number = value.parse::<f32>().ok()?;
    Some(if number <= 1.0 {
        (number * 255.0).round().clamp(0.0, 255.0) as u8
    } else {
        number.round().clamp(0.0, 255.0) as u8
    })
}

fn named_color(value: &str) -> Option<QColor> {
    let rgb = match value.to_ascii_lowercase().as_str() {
        "transparent" => return Some(QColor::rgba8(0, 0, 0, 0)),
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "gray" | "grey" => (128, 128, 128),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "silver" => (192, 192, 192),
        "maroon" => (128, 0, 0),
        "olive" => (128, 128, 0),
        "navy" => (0, 0, 128),
        "purple" => (128, 0, 128),
        "teal" => (0, 128, 128),
        "lime" => (0, 255, 0),
        "orange" => (255, 165, 0),
        "darkred" => (139, 0, 0),
        "darkgreen" => (0, 100, 0),
        "darkblue" => (0, 0, 139),
        _ => return None,
    };
    Some(QColor::rgb(rgb.0, rgb.1, rgb.2))
}

fn parse_px(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(number) = value.strip_suffix("px") {
        number.trim().parse::<f32>().ok()
    } else {
        value.parse::<f32>().ok()
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start + 2..].find("*/") {
            rest = &rest[start + 2 + end + 2..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::QWidgetTree;

    fn widget() -> (QWidgetTree, usize) {
        let mut tree = QWidgetTree::new();
        let id = tree.create_widget("QPushButton", "ok");
        (tree, id)
    }

    #[test]
    fn parses_colors_borders_and_padding() {
        let sheet = QStyleSheet::parse(
            "QPushButton { background-color: #2f6f4f; color: #fff; border: 2px solid #123456; border-radius: 4px; padding: 3px 7px; }",
        );
        let (tree, id) = widget();
        let declarations = sheet.resolve(tree.widget(id).unwrap());
        assert_eq!(declarations.background, Some(QColor::rgb(0x2f, 0x6f, 0x4f)));
        assert_eq!(declarations.color, Some(QColor::rgb(0xff, 0xff, 0xff)));
        assert_eq!(declarations.border_width, Some(2.0));
        assert_eq!(
            declarations.border_color,
            Some(QColor::rgb(0x12, 0x34, 0x56))
        );
        assert_eq!(declarations.border_radius, Some(4.0));
        assert_eq!(declarations.padding, Some((7.0, 3.0, 7.0, 3.0)));
    }

    #[test]
    fn parses_box_model_sides_and_linear_gradient() {
        let sheet = QStyleSheet::parse(
            "QPushButton { margin: 8px 4px 2px 6px; padding-top: 3px; padding-left: 9px; \
             background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #ffffff, stop:1 #204020); }",
        );
        let (tree, id) = widget();
        let declarations = sheet.resolve(tree.widget(id).unwrap());
        assert_eq!(declarations.margin, Some((6.0, 8.0, 4.0, 2.0)));
        assert_eq!(declarations.padding, Some((9.0, 3.0, 0.0, 0.0)));
        let gradient = declarations.background_gradient.expect("gradient");
        assert_eq!(gradient.x1, 0.0);
        assert_eq!(gradient.y2, 1.0);
        assert_eq!(gradient.stops.len(), 2);
        assert_eq!(gradient.stops[0], (0.0, QColor::rgb(255, 255, 255)));
        assert_eq!(gradient.stops[1], (1.0, QColor::rgb(0x20, 0x40, 0x20)));
    }

    #[test]
    fn subcontrol_selectors_resolve_per_part_not_for_the_widget() {
        let sheet = QStyleSheet::parse(
            "QTreeWidget::item { background: #ff0000; color: #111111; } \
             QHeaderView::section { background: #00ff00; } \
             QComboBox QAbstractItemView { background: #0000ff; }",
        );
        let (mut tree, _) = widget();
        let list = tree.create_widget("QTreeWidget", "tree");
        let combo = tree.create_widget("QComboBox", "combo");

        assert_eq!(sheet.resolve(tree.widget(list).unwrap()).background, None);
        let item = sheet.resolve_part(tree.widget(list).unwrap(), "item");
        assert_eq!(item.background, Some(QColor::rgb(255, 0, 0)));
        assert_eq!(item.color, Some(QColor::rgb(0x11, 0x11, 0x11)));
        let section = sheet.resolve_part(tree.widget(list).unwrap(), "section");
        assert_eq!(section.background, Some(QColor::rgb(0, 255, 0)));
        let view = sheet.resolve_part(tree.widget(combo).unwrap(), "view");
        assert_eq!(view.background, Some(QColor::rgb(0, 0, 255)));
    }

    #[test]
    fn selectors_match_name_and_state() {
        let mut sheet = QStyleSheet::parse(
            "QPushButton { color: #111111; } QPushButton#ok:disabled { color: #999999; }",
        );
        let (mut tree, id) = widget();
        assert_eq!(
            sheet.resolve(tree.widget(id).unwrap()).color,
            Some(QColor::rgb(0x11, 0x11, 0x11))
        );
        tree.set_enabled(id, false);
        assert_eq!(
            sheet.resolve(tree.widget(id).unwrap()).color,
            Some(QColor::rgb(0x99, 0x99, 0x99))
        );
        sheet = QStyleSheet::parse("#other { color: #ff0000; }");
        assert_eq!(sheet.resolve(tree.widget(id).unwrap()).color, None);
    }

    #[test]
    fn parses_extended_colors_fonts_and_side_borders() {
        let sheet = QStyleSheet::parse(
            "QPushButton { border-left: 3px solid rgb(10, 20, 30); border-right-color: rgba(255, 0, 0, 0.5); \
             font: italic bold 18px \"DejaVu Sans\"; text-align: right; }",
        );
        let (tree, id) = widget();
        let declarations = sheet.resolve(tree.widget(id).unwrap());
        assert_eq!(
            declarations.border_left,
            Some(QBorderSideDecl {
                width: Some(3.0),
                color: Some(QColor::rgb(10, 20, 30)),
            })
        );
        assert_eq!(
            declarations.border_right.and_then(|side| side.color),
            Some(QColor::rgba8(255, 0, 0, 128))
        );
        assert_eq!(declarations.font_family.as_deref(), Some("DejaVu Sans"));
        assert_eq!(declarations.font_size, Some(18.0));
        assert_eq!(declarations.font_weight, Some(QFontWeight::Bold));
        assert_eq!(declarations.font_italic, Some(true));
        assert_eq!(declarations.text_align, Some(QTextAlign::Right));
    }

    #[test]
    fn matches_attribute_pressed_default_and_open_selectors() {
        let sheet = QStyleSheet::parse(
            "QPushButton[role=\"danger\"] { color: #ff0000; } \
             QPushButton:pressed { color: #00ff00; } \
             QPushButton:default { color: #0000ff; } \
             QComboBox:open { color: #00ffff; }",
        );
        let (mut tree, id) = widget();
        tree.set_property(id, "role", QVariant::String("danger".into()));
        assert_eq!(
            sheet.resolve(tree.widget(id).unwrap()).color,
            Some(QColor::rgb(0xff, 0x00, 0x00))
        );
        tree.set_property(id, "role", QVariant::String("normal".into()));
        assert_eq!(sheet.resolve(tree.widget(id).unwrap()).color, None);
        tree.widget_mut(id).unwrap().pressed = true;
        assert_eq!(
            sheet.resolve(tree.widget(id).unwrap()).color,
            Some(QColor::rgb(0x00, 0xff, 0x00))
        );
        tree.widget_mut(id).unwrap().pressed = false;
        tree.widget_mut(id).unwrap().default_button = true;
        assert_eq!(
            sheet.resolve(tree.widget(id).unwrap()).color,
            Some(QColor::rgb(0x00, 0x00, 0xff))
        );

        let combo = tree.create_widget("QComboBox", "combo");
        tree.set_property(
            combo,
            "items",
            QVariant::List(vec![QVariant::String("one".into())]),
        );
        assert!(tree.open_combo_popup(combo));
        assert_eq!(
            sheet.resolve(tree.widget(combo).unwrap()).color,
            Some(QColor::rgb(0x00, 0xff, 0xff))
        );
    }

    #[test]
    fn parses_four_and_eight_digit_colors_as_argb() {
        let sheet = QStyleSheet::parse(
            "QPushButton { color: #80ff0000; background-color: #f0f; border-color: transparent; }",
        );
        let (tree, id) = widget();
        let declarations = sheet.resolve(tree.widget(id).unwrap());
        assert_eq!(declarations.color, Some(QColor::rgba8(255, 0, 0, 128)));
        assert_eq!(declarations.background, Some(QColor::rgb(255, 0, 255)));
        assert_eq!(declarations.border_color, Some(QColor::rgba8(0, 0, 0, 0)));
    }
}
