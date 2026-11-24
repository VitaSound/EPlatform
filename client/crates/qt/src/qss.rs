use crate::core::QVariant;
use crate::gui::{QColor, QFontWeight};
use crate::widgets::QWidget;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QStyleDeclarations {
    pub background: Option<QColor>,
    pub color: Option<QColor>,
    pub border_width: Option<f32>,
    pub border_color: Option<QColor>,
    pub border_radius: Option<f32>,
    pub padding: Option<(f32, f32, f32, f32)>,
    pub selection_background: Option<QColor>,
    pub selection_color: Option<QColor>,
    pub font_size: Option<f32>,
    pub font_weight: Option<QFontWeight>,
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
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.selection_background.is_some() {
            self.selection_background = other.selection_background;
        }
        if other.selection_color.is_some() {
            self.selection_color = other.selection_color;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.font_weight.is_some() {
            self.font_weight = other.font_weight;
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
}

fn selector_matches(selector: &str, widget: &QWidget) -> bool {
    let mut parts = selector.split(':');
    let base = parts.next().unwrap_or("").trim();
    let states: Vec<&str> = parts.map(|state| state.trim()).collect();

    let (class, object_name) = match base.split_once('#') {
        Some((class, name)) => (class.trim(), Some(name.trim())),
        None => (base, None),
    };
    if !(class.is_empty()
        || class == "*"
        || class == widget.class_name
        || class == "QWidget" && widget.class_name == "QWidget")
    {
        if class != "QWidget" {
            return false;
        }
    }
    if class == "QWidget" && widget.class_name != "QWidget" {
        return false;
    }
    if let Some(name) = object_name {
        if widget.object_name != name {
            return false;
        }
    }
    for state in states {
        let matches = match state {
            "" | "enabled" => widget.enabled,
            "disabled" => !widget.enabled,
            "focus" => widget.has_focus,
            "hover" => widget.under_mouse,
            "checked" => matches!(widget.property("checked"), Some(QVariant::Bool(true))),
            _ => true,
        };
        if !matches {
            return false;
        }
    }
    true
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
                if let Some(color) = parse_color(value) {
                    declarations.background = Some(color);
                }
            }
            "color" => {
                if let Some(color) = parse_color(value) {
                    declarations.color = Some(color);
                }
            }
            "border" => {
                let tokens: Vec<&str> = value.split_whitespace().collect();
                if tokens
                    .iter()
                    .any(|token| token.eq_ignore_ascii_case("none"))
                {
                    declarations.border_width = Some(0.0);
                } else if let Some(width) = tokens.iter().find_map(|token| parse_px(token)) {
                    declarations.border_width = Some(width);
                }
                if let Some(color) = tokens.iter().find_map(|token| parse_color(token)) {
                    declarations.border_color = Some(color);
                } else if declarations.border_width.is_some() {
                    declarations.border_width = Some(declarations.border_width.unwrap_or(1.0));
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
            "padding" => {
                let values: Vec<f32> = value.split_whitespace().filter_map(parse_px).collect();
                declarations.padding = match values.as_slice() {
                    [all] => Some((*all, *all, *all, *all)),
                    [vertical, horizontal] => {
                        Some((*horizontal, *vertical, *horizontal, *vertical))
                    }
                    [top, horizontal, bottom] => Some((*horizontal, *top, *horizontal, *bottom)),
                    [top, right, bottom, left] => Some((*left, *top, *right, *bottom)),
                    _ => declarations.padding,
                };
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
            "font-size" => {
                if let Some(size) = parse_px(value) {
                    declarations.font_size = Some(size);
                }
            }
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
            _ => {}
        }
    }
    declarations
}

fn parse_color(value: &str) -> Option<QColor> {
    let value = value.trim();
    let first = value.split_whitespace().next()?;
    QColor::from_hex(first)
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
}
