//! Retained widget tree + Qt `QBoxLayout` semantics (MVP subset).
//!
//! [`build`] converts a parsed [`form_xml::UiForm`] into a [`Node`]
//! tree with bottom-up size hints; [`layout`] assigns a [`Rect`] to
//! every node for a given area using Qt's box distribution rules:
//! hints + spacing + margins, slack goes to expanding items (spacers,
//! `QLineEdit`), shortfall is absorbed by spacers first.

use form_xml::{Layout, LayoutItem, PropValue, UiForm, Widget};

pub mod table;

pub use table::{Table, HEADER_H, ROW_H};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dir {
    H,
    V,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A real widget (root dialog, label, button, ...). May own a
    /// single layout child.
    Widget { class: String, name: String },
    /// `QSpacerItem`: expands along its orientation.
    Spacer { horizontal: bool },
    /// A `QBoxLayout`; children are the layout items in order.
    Box {
        dir: Dir,
        spacing: f32,
        /// `(left, top, right, bottom)`
        margins: (f32, f32, f32, f32),
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: Kind,
    pub rect: Rect,
    /// Bottom-up size hint (Qt `sizeHint()`), computed by [`build`].
    pub hint: Size,
    /// Widget properties (text, enabled, ...). Empty for boxes/spacers.
    pub props: Vec<(String, PropValue)>,
    /// `QTableWidget` payload state (headers/cells/selection); `None`
    /// for every other widget kind.
    pub table: Option<Box<Table>>,
    pub children: Vec<Node>,
}

/// Qt defaults for the MVP: dialog layout margins and box spacing.
pub const ROOT_MARGIN: f32 = 9.0;
pub const DEFAULT_SPACING: f32 = 6.0;

/// Approximate glyph metrics until real font metrics land (glyphon).
pub const CHAR_W: f32 = 7.0;
const LABEL_H: f32 = 16.0;
const LINE_EDIT: Size = Size { w: 100.0, h: 20.0 };
const BUTTON_H: f32 = 24.0;
const BUTTON_PAD: f32 = 24.0;
const DEFAULT_HINT: Size = Size { w: 100.0, h: 20.0 };
/// Tables are content views: a generous default so they claim space
/// in a layout before `tableData` arrives.
const TABLE_HINT: Size = Size { w: 480.0, h: 240.0 };

/// Build the runtime tree from a parsed form, computing size hints.
pub fn build(form: &UiForm) -> Node {
    let mut root = widget_node(&form.root, true);
    measure(&mut root);
    root
}

fn widget_node(widget: &Widget, is_root: bool) -> Node {
    let children: Vec<Node> = widget
        .layout
        .as_ref()
        .map(|layout| vec![layout_node(layout, is_root)])
        .unwrap_or_default();

    // QTableWidget carries its own payload state; the .ui may declare
    // the grid size up front.
    let table = if widget.class == "QTableWidget" {
        let mut table = Table::default();
        if let Some(rows) = widget.property("rowCount").and_then(PropValue::as_i64) {
            table.row_count = rows.max(0) as usize;
        }
        if let Some(cols) = widget.property("columnCount").and_then(PropValue::as_i64) {
            table.set_column_count(cols.max(0) as usize);
        }
        Some(Box::new(table))
    } else {
        None
    };

    Node {
        kind: Kind::Widget {
            class: widget.class.clone(),
            name: widget.name.clone(),
        },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        hint: hint_from_props(&widget.class, &widget.properties),
        props: widget.properties.clone(),
        table,
        children,
    }
}

fn layout_node(layout: &Layout, is_root: bool) -> Node {
    let dir = if layout.class.contains('H') {
        Dir::H
    } else {
        Dir::V
    };
    let spacing = layout.spacing.map(|s| s as f32).unwrap_or(DEFAULT_SPACING);
    let margins = match layout.margins {
        Some((l, t, r, b)) => (l as f32, t as f32, r as f32, b as f32),
        None if is_root => (ROOT_MARGIN, ROOT_MARGIN, ROOT_MARGIN, ROOT_MARGIN),
        None => (0.0, 0.0, 0.0, 0.0),
    };

    let children = layout
        .items
        .iter()
        .map(|item| match item {
            LayoutItem::Widget(w) => widget_node(w, false),
            LayoutItem::Layout(inner) => layout_node(inner, false),
            LayoutItem::Spacer(s) => Node {
                kind: Kind::Spacer {
                    horizontal: s.horizontal,
                },
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                },
                hint: Size {
                    w: s.hint.0 as f32,
                    h: s.hint.1 as f32,
                },
                props: Vec::new(),
                table: None,
                children: Vec::new(),
            },
        })
        .collect();

    Node {
        kind: Kind::Box {
            dir,
            spacing,
            margins,
        },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        hint: Size { w: 0.0, h: 0.0 },
        props: Vec::new(),
        table: None,
        children,
    }
}

/// Qt-approximate size hint for a widget from its stored properties.
fn hint_from_props(class: &str, props: &[(String, PropValue)]) -> Size {
    let text = props
        .iter()
        .find(|(key, _)| key == "text")
        .and_then(|(_, value)| match value {
            PropValue::Str(s) => Some(s.chars().count() as f32),
            PropValue::Int(i) => Some(i.to_string().chars().count() as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    match class {
        "QLabel" => Size {
            w: text * CHAR_W + 16.0,
            h: LABEL_H,
        },
        "QLineEdit" => LINE_EDIT,
        "QPushButton" => Size {
            w: text * CHAR_W + BUTTON_PAD,
            h: BUTTON_H,
        },
        "QTableWidget" => TABLE_HINT,
        _ => DEFAULT_HINT,
    }
}

/// Bottom-up: compute size hints (a widget with a layout takes the
/// layout's hint; a box sums children along its main axis). Leaf hints
/// are recomputed from the current properties so that `setProperty`
/// changes propagate into the next layout pass.
fn measure(node: &mut Node) -> Size {
    match &node.kind {
        Kind::Widget { class, .. } => {
            node.hint = hint_from_props(class, &node.props);
            let mut layout_hint = None;
            for child in &mut node.children {
                layout_hint = Some(measure(child));
            }
            if let Some(hint) = layout_hint {
                // Widget hosting a layout is as big as its layout.
                node.hint = hint;
            }
            node.hint
        }
        Kind::Spacer { .. } => node.hint,
        Kind::Box {
            dir,
            spacing,
            margins,
        } => {
            let (l, t, r, b) = *margins;
            let mut main = 0.0f32;
            let mut perp = 0.0f32;
            let mut count = 0usize;
            for child in &mut node.children {
                let hint = measure(child);
                let (m, p) = match dir {
                    Dir::H => (hint.w, hint.h),
                    Dir::V => (hint.h, hint.w),
                };
                main += m;
                perp = perp.max(p);
                count += 1;
            }
            if count > 1 {
                main += *spacing * (count as f32 - 1.0);
            }
            node.hint = match dir {
                Dir::H => Size {
                    w: main + l + r,
                    h: perp + t + b,
                },
                Dir::V => Size {
                    w: perp + l + r,
                    h: main + t + b,
                },
            };
            node.hint
        }
    }
}

/// Assign `rect` to the whole tree. The root widget fills `area`; its
/// layout distributes the inner region (area minus margins).
pub fn layout(root: &mut Node, area: Rect) {
    root.rect = area;
    for child in &mut root.children {
        place(child, area);
    }
}

/// Give an item its slot and recurse into whatever it contains.
fn place(node: &mut Node, slot: Rect) {
    node.rect = slot;
    match &node.kind {
        Kind::Box { .. } => layout_box(node, slot),
        Kind::Widget { .. } => {
            for child in &mut node.children {
                place(child, slot);
            }
        }
        Kind::Spacer { .. } => {}
    }
}

fn layout_box(node: &mut Node, slot: Rect) {
    let (dir, spacing, margins) = match &node.kind {
        Kind::Box {
            dir,
            spacing,
            margins,
        } => (*dir, *spacing, *margins),
        _ => unreachable!("layout_box on non-box node"),
    };
    node.rect = slot;

    let (l, t, r, b) = margins;
    let inner = Rect {
        x: slot.x + l,
        y: slot.y + t,
        w: (slot.w - l - r).max(0.0),
        h: (slot.h - t - b).max(0.0),
    };

    let count = node.children.len();
    if count == 0 {
        return;
    }

    // 1. Preferred sizes along the main axis.
    let mut sizes: Vec<f32> = node
        .children
        .iter()
        .map(|child| match dir {
            Dir::H => child.hint.w,
            Dir::V => child.hint.h,
        })
        .collect();
    let sum: f32 = sizes.iter().sum();
    let gaps = spacing * (count as f32 - 1.0);
    let available = match dir {
        Dir::H => inner.w,
        Dir::V => inner.h,
    };
    let mut slack = available - sum - gaps;

    // 2. Distribute slack / absorb shortfall (Qt-ish rules).
    if slack < 0.0 {
        // Spacers shrink first (towards zero), then everything else
        // shrinks proportionally.
        for (i, child) in node.children.iter().enumerate() {
            if slack >= 0.0 {
                break;
            }
            if matches!(child.kind, Kind::Spacer { .. }) && spacer_along(child, dir) {
                let take = sizes[i].min(-slack);
                sizes[i] -= take;
                slack += take;
            }
        }
        if slack < 0.0 {
            let deficit = -slack;
            let shrinkable: f32 = sizes.iter().sum();
            if shrinkable > 0.0 {
                for size in &mut sizes {
                    *size = (*size - (*size / shrinkable) * deficit).max(0.0);
                }
            }
        }
    } else if slack > 0.0 {
        let expanders: Vec<usize> = node
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| expands(child, dir))
            .map(|(i, _)| i)
            .collect();
        if !expanders.is_empty() {
            let share = slack / expanders.len() as f32;
            for i in expanders {
                sizes[i] += share;
            }
        }
        // No expander: items pack at the start, slack stays at the end.
    }

    // 3. Place children sequentially; recurse into boxes.
    let mut cursor = match dir {
        Dir::H => inner.x,
        Dir::V => inner.y,
    };
    for (i, child) in node.children.iter_mut().enumerate() {
        let main = sizes[i];
        let perp = perp_size(child, dir, inner);
        let slot = match dir {
            Dir::H => Rect {
                x: cursor,
                y: inner.y,
                w: main,
                h: perp,
            },
            Dir::V => Rect {
                x: inner.x,
                y: cursor,
                w: perp,
                h: main,
            },
        };
        place(child, slot);
        cursor += main + spacing;
    }
}

/// Perpendicular size of one item: fills the box when the item expands
/// in that direction, otherwise keeps its hint (Qt default alignment).
fn perp_size(child: &Node, dir: Dir, inner: Rect) -> f32 {
    let perp_axis = match dir {
        Dir::H => Dir::V,
        Dir::V => Dir::H,
    };
    if expands(child, perp_axis) {
        match perp_axis {
            Dir::H => inner.w,
            Dir::V => inner.h,
        }
    } else {
        match dir {
            Dir::H => child.hint.h,
            Dir::V => child.hint.w,
        }
    }
}

fn spacer_along(child: &Node, dir: Dir) -> bool {
    match &child.kind {
        Kind::Spacer { horizontal } => match dir {
            Dir::H => *horizontal,
            Dir::V => !*horizontal,
        },
        _ => false,
    }
}

/// Does this item claim extra space along `axis`? (MVP size-policy
/// table: spacers expand along their orientation, `QLineEdit`
/// horizontally, boxes inherit from their leaves.)
pub fn expands(child: &Node, axis: Dir) -> bool {
    match &child.kind {
        Kind::Spacer { horizontal } => match axis {
            Dir::H => *horizontal,
            Dir::V => !*horizontal,
        },
        Kind::Widget { class, .. } => match class.as_str() {
            // `QLineEdit` stretches along its main axis only.
            "QLineEdit" => axis == Dir::H,
            // A table is a view: it fills its cell on both axes.
            "QTableWidget" => true,
            _ => false,
        },
        Kind::Box { .. } => child.children.iter().any(|c| expands(c, axis)),
    }
}

/// Depth-first walk over every node (parents before children).
pub fn for_each_node<'a>(node: &'a Node, f: &mut impl FnMut(&'a Node)) {
    f(node);
    for child in &node.children {
        for_each_node(child, f);
    }
}

/// Mutable depth-first walk (parents before children).
pub fn for_each_node_mut(node: &mut Node, f: &mut impl FnMut(&mut Node)) {
    f(node);
    for child in &mut node.children {
        for_each_node_mut(child, f);
    }
}

/// Set a property on the named widget; `true` if the widget exists.
pub fn set_property(root: &mut Node, name: &str, property: &str, value: PropValue) -> bool {
    let mut found = false;
    for_each_node_mut(root, &mut |n| {
        if let Kind::Widget {
            name: node_name, ..
        } = &n.kind
        {
            if node_name == name {
                upsert_prop(&mut n.props, property, value.clone());
                found = true;
            }
        }
    });
    found
}

/// Append one character to the named widget's `text` (MVP `QLineEdit`
/// keyboard input). A missing `text` property starts as an empty
/// string; an integer one (Lua wrote a number) is converted to text.
/// `true` if the widget exists.
pub fn insert_text(root: &mut Node, name: &str, ch: char) -> bool {
    edit_text(root, name, |text| text.push(ch))
}

/// Drop the last character of the named widget's `text`. No-op on an
/// empty string. `true` if the widget exists.
pub fn backspace_text(root: &mut Node, name: &str) -> bool {
    edit_text(root, name, |text| {
        text.pop();
    })
}

fn edit_text(root: &mut Node, name: &str, f: impl Fn(&mut String)) -> bool {
    let mut found = false;
    for_each_node_mut(root, &mut |n| {
        let matches = matches!(
            &n.kind,
            Kind::Widget {
                name: node_name, ..
            } if node_name == name
        );
        if !matches {
            return;
        }
        if !n.props.iter().any(|(key, _)| key == "text") {
            n.props
                .push(("text".to_string(), PropValue::Str(String::new())));
        }
        if let Some(slot) = n.props.iter_mut().find(|(key, _)| key == "text") {
            match &mut slot.1 {
                PropValue::Str(text) => {
                    f(text);
                    found = true;
                }
                // Lua set a number (result of a calculation): start
                // editing its textual form.
                PropValue::Int(i) => {
                    let mut text = i.to_string();
                    f(&mut text);
                    slot.1 = PropValue::Str(text);
                    found = true;
                }
                _ => {}
            }
        }
    });
    found
}

/// Read a property of the named widget.
pub fn get_property<'a>(root: &'a Node, name: &str, property: &str) -> Option<&'a PropValue> {
    let mut value = None;
    for_each_node(root, &mut |n| {
        if let Kind::Widget {
            name: node_name, ..
        } = &n.kind
        {
            if node_name == name {
                value = prop(&n.props, property);
            }
        }
    });
    value
}

/// Set a property on the root widget (windowTitle, geometry, ...).
pub fn set_form_property(root: &mut Node, property: &str, value: PropValue) {
    upsert_prop(&mut root.props, property, value);
}

/// Read a property of the root widget.
pub fn get_form_property<'a>(root: &'a Node, property: &str) -> Option<&'a PropValue> {
    prop(&root.props, property)
}

/// The named widget's node (first match in tree order).
pub fn find_widget<'a>(root: &'a Node, name: &str) -> Option<&'a Node> {
    let mut found = None;
    for_each_node(root, &mut |n| {
        if let Kind::Widget {
            name: node_name, ..
        } = &n.kind
        {
            if node_name == name {
                found = Some(n);
            }
        }
    });
    found
}

/// Mutable lookup by widget name.
pub fn find_widget_mut<'a>(root: &'a mut Node, name: &str) -> Option<&'a mut Node> {
    if let Kind::Widget {
        name: node_name, ..
    } = &root.kind
    {
        if node_name == name {
            return Some(root);
        }
    }
    for child in &mut root.children {
        if let Some(found) = find_widget_mut(child, name) {
            return Some(found);
        }
    }
    None
}

/// Payload state of the named `QTableWidget` (or `None` when the
/// widget does not exist / is not a table).
pub fn table<'a>(root: &'a Node, name: &str) -> Option<&'a Table> {
    find_widget(root, name)?.table.as_deref()
}

/// Mutable payload state of the named `QTableWidget`.
pub fn table_mut<'a>(root: &'a mut Node, name: &str) -> Option<&'a mut Table> {
    find_widget_mut(root, name)?.table.as_deref_mut()
}

/// Table half of `setProperty`: `tableHeaders`, `tableData`,
/// `selectRows`, `readOnly`, `currentRow`, plus the bare table
/// properties Lua scripts set directly (`clear`, `rowCount`,
/// `columnCount`, `horizontalHeaderLabels`, `columnWidths`,
/// `hiddenColumns`, `hideColumn`, `data`).
/// `false` when the widget is not a table or the property is unknown —
/// the caller may fall back to the generic property store.
pub fn set_table_property(root: &mut Node, name: &str, property: &str, value: &PropValue) -> bool {
    let Some(table) = table_mut(root, name) else {
        return false;
    };
    match property {
        "tableHeaders" => {
            table.apply_headers(value);
            true
        }
        "tableData" => {
            table.apply_data(value);
            true
        }
        "clear" => {
            if value.as_bool() == Some(true) {
                table.clear();
            }
            true
        }
        "rowCount" => match value.as_i64() {
            Some(count) => {
                table.set_row_count(count.max(0) as usize);
                true
            }
            None => false,
        },
        "columnCount" => match value.as_i64() {
            Some(count) => {
                table.set_column_count(count.max(0) as usize);
                true
            }
            None => false,
        },
        "horizontalHeaderLabels" => {
            table.set_header_labels(value);
            true
        }
        "columnWidths" => {
            table.set_column_widths(value);
            true
        }
        "hiddenColumns" => {
            table.apply_headers(&PropValue::Map(vec![(
                "hiddenColumns".to_string(),
                value.clone(),
            )]));
            true
        }
        "hideColumn" => {
            table.hide_column(value);
            true
        }
        "data" => {
            // Bare `data` (Lua fixture style) fills the existing grid;
            // unlike `tableData` it does not reset `rowCount`.
            let row_count = table.row_count as i64;
            table.apply_data(&PropValue::Map(vec![
                ("rowCount".to_string(), PropValue::Int(row_count)),
                ("data".to_string(), value.clone()),
            ]));
            true
        }
        "selectRows" | "readOnly" => table.set_flag(property, value),
        "currentRow" => value
            .as_i64()
            .is_some_and(|row| table.set_current_row_1based(row)),
        _ => false,
    }
}

/// Scroll a table so its `currentRow` is visible in a viewport of
/// `view_h` pixels.
pub fn reveal_table_row(root: &mut Node, name: &str, view_h: f32) {
    if let Some(table) = table_mut(root, name) {
        table.reveal_current_row(view_h);
    }
}

/// Table half of `getProperty`: `currentRow` (0-based, Qt property
/// semantics) and `currentRowCell:<col>` (1-based column).
pub fn get_table_property(root: &Node, name: &str, property: &str) -> Option<PropValue> {
    let table = table(root, name)?;
    match property {
        "rowCount" => return Some(PropValue::Int(table.row_count as i64)),
        "columnCount" => return Some(PropValue::Int(table.column_count() as i64)),
        "currentRow" => return Some(table.current_row_property()),
        "currentColumn" => {
            return Some(PropValue::Int(
                table.current_col.map(|c| c as i64).unwrap_or(-1),
            ));
        }
        _ => {}
    }
    let col = property
        .strip_prefix("currentRowCell:")?
        .parse::<i64>()
        .ok()?;
    table.current_row_cell(col)
}

/// Re-run measure + layout for the tree's current area (`root.rect`).
/// Call after any property or geometry change.
pub fn relayout(root: &mut Node) {
    let area = root.rect;
    measure(root);
    layout(root, area);
}

fn prop<'a>(props: &'a [(String, PropValue)], key: &str) -> Option<&'a PropValue> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn upsert_prop(props: &mut Vec<(String, PropValue)>, key: &str, value: PropValue) {
    if let Some(slot) = props.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value;
    } else {
        props.push((key.to_string(), value));
    }
}

/// Number of nodes in the tree (diagnostics).
pub fn count_nodes(node: &Node) -> usize {
    let mut total = 0;
    for_each_node(node, &mut |_| total += 1);
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use form_xml::parse;

    const FIXTURE: &str = include_str!("../../form-xml/tests/fixtures/test_dialog01.ui");

    fn built_fixture() -> Node {
        let form = parse(FIXTURE).expect("fixture parses");
        build(&form)
    }

    fn find<'a>(node: &'a Node, name: &str) -> &'a Node {
        let mut found: Option<&Node> = None;
        for_each_node(node, &mut |n| {
            if let Kind::Widget { name: n_name, .. } = &n.kind {
                if n_name.as_str() == name {
                    found = Some(n);
                }
            }
        });
        found.unwrap_or_else(|| panic!("node {name} not found"))
    }

    fn find_spacer<'a>(node: &'a Node, horizontal: bool) -> &'a Node {
        let mut found: Option<&Node> = None;
        for_each_node(node, &mut |n| {
            if let Kind::Spacer { horizontal: h } = &n.kind {
                if *h == horizontal && found.is_none() {
                    found = Some(n);
                }
            }
        });
        found.expect("spacer not found")
    }

    #[test]
    fn size_hints_from_fixture() {
        let root = built_fixture();
        // V box: widest row (40+94+94+12 = 240) + root margins.
        assert_eq!(root.hint.w, 240.0 + 18.0);
        // rows: label(79) + spacer(40) + lineEdit(100) + 2*spacing(12) = 231
        assert_eq!(find(&root, "label_1").hint.w, 9.0 * CHAR_W + 16.0);
        assert_eq!(find(&root, "lineEdit_1").hint, LINE_EDIT);
        assert_eq!(
            find(&root, "pushButton_1").hint,
            Size {
                w: 10.0 * CHAR_W + BUTTON_PAD,
                h: BUTTON_H
            }
        );
    }

    #[test]
    fn layout_in_dialog_geometry() {
        let mut root = built_fixture();
        layout(
            &mut root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 388.0,
                h: 190.0,
            },
        );

        // Root dialog fills the area.
        assert_eq!(
            root.rect,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 388.0,
                h: 190.0
            }
        );

        // Row 1: label at the top-left margin.
        let label = find(&root, "label_1");
        assert_eq!(label.rect.x, 9.0);
        assert_eq!(label.rect.y, 9.0);

        // Row 1 height = max(label16, spacer20, lineEdit20) = 20.
        // Slack of the row (370 - 221 = 149) is shared by the H spacer
        // and the expanding line edit.
        let line = find(&root, "lineEdit_1");
        assert_eq!(line.rect.h, 20.0);
        assert_eq!(line.rect.y, 9.0);
        assert_eq!(line.rect.right(), 388.0 - 9.0); // stretches to margin

        // Vertical spacer absorbs the dialog's vertical slack.
        let vspacer = find_spacer(&root, false);
        assert_eq!(vspacer.rect.y, 9.0 + 20.0 + 6.0 + 20.0 + 6.0 + 20.0 + 6.0);
        assert_eq!(vspacer.rect.h, 64.0); // 71 hint - 7 shortfall

        // Button row sits at the bottom margin; the H spacer pushes the
        // buttons to the right edge.
        let btn = find(&root, "pushButton_2");
        assert_eq!(btn.rect.bottom(), 190.0 - 9.0);
        assert_eq!(btn.rect.right(), 388.0 - 9.0);
        assert_eq!(btn.rect.h, BUTTON_H);
    }

    #[test]
    fn v_box_stacks_rows_top_down() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="root">
  <layout class="QVBoxLayout" name="v">
   <item><widget class="QLabel" name="a"><property name="text"><string>A</string></property></widget></item>
   <item><widget class="QLabel" name="b"><property name="text"><string>B</string></property></widget></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let mut tree = build(&form);
        layout(
            &mut tree,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
        );
        let a = find(&tree, "a");
        let b = find(&tree, "b");
        assert_eq!(a.rect.y, 9.0);
        assert_eq!(b.rect.y, 9.0 + LABEL_H + DEFAULT_SPACING);
        // Labels don't expand: both keep hint width, top-left aligned.
        assert_eq!(a.rect.w, 1.0 * CHAR_W + 16.0);
    }

    #[test]
    fn negative_slack_shrinks_spacers_first() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="root">
  <layout class="QVBoxLayout" name="v">
   <item><spacer name="s"><property name="orientation"><enum>Qt::Orientation::Vertical</enum></property><property name="sizeHint" stdset="0"><size><width>20</width><height>200</height></size></property></spacer></item>
   <item><widget class="QLabel" name="a"><property name="text"><string>A</string></property></widget></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let mut tree = build(&form);
        layout(
            &mut tree,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 60.0,
            },
        );
        // Content hint = 200 + 16 + 6 = 222 in a 60-18=42px slot:
        // the spacer absorbs everything down to 42-16-6 = 20.
        let spacer = find_spacer(&tree, false);
        assert_eq!(spacer.rect.h, 20.0);
        let label = find(&tree, "a");
        assert_eq!(label.rect.h, LABEL_H);
    }

    #[test]
    fn line_edit_fills_width_in_v_box() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="root">
  <layout class="QVBoxLayout" name="v">
   <item><widget class="QLineEdit" name="e"/></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let mut tree = build(&form);
        layout(
            &mut tree,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 100.0,
            },
        );
        let edit = find(&tree, "e");
        // QLineEdit expands horizontally: full inner width in a V box.
        assert_eq!(edit.rect.x, 9.0);
        assert_eq!(edit.rect.w, 300.0 - 18.0);
        // ...but keeps its hint height (vertical policy is fixed).
        assert_eq!(edit.rect.h, 20.0);
    }

    #[test]
    fn set_property_updates_hint_and_layout() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="root">
  <layout class="QHBoxLayout" name="h">
   <item><widget class="QLabel" name="lbl"><property name="text"><string>A</string></property></widget></item>
   <item><widget class="QLineEdit" name="e"/></item>
  </layout>
 </widget>
</ui>"#;
        let form = parse(xml).expect("parses");
        let mut tree = build(&form);
        let area = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 50.0,
        };
        layout(&mut tree, area);

        assert_eq!(
            get_property(&tree, "lbl", "text"),
            Some(&PropValue::Str("A".into()))
        );

        // Lua sets a much longer caption → label grows on relayout.
        assert!(set_property(
            &mut tree,
            "lbl",
            "text",
            PropValue::Str("Hello, platform!".into())
        ));
        relayout(&mut tree);

        let label = find(&tree, "lbl");
        assert_eq!(
            label.hint.w,
            "Hello, platform!".chars().count() as f32 * CHAR_W + 16.0
        );
        assert_eq!(
            get_property(&tree, "lbl", "text"),
            Some(&PropValue::Str("Hello, platform!".into()))
        );

        // Unknown widget names are reported, not silently ignored.
        assert!(!set_property(
            &mut tree,
            "ghost",
            "text",
            PropValue::Str("x".into())
        ));
        assert_eq!(get_property(&tree, "ghost", "text"), None);
    }

    #[test]
    fn form_geometry_change_moves_whole_tree() {
        let form = parse(FIXTURE).expect("fixture parses");
        let mut tree = build(&form);
        layout(
            &mut tree,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 388.0,
                h: 190.0,
            },
        );
        assert_eq!(find(&tree, "label_1").rect.x, 9.0);

        // Server pushes setFormProperty geometry {100,100,400,400}.
        set_form_property(
            &mut tree,
            "geometry",
            PropValue::Rect {
                x: 100,
                y: 100,
                w: 400,
                h: 400,
            },
        );
        tree.rect = Rect {
            x: 100.0,
            y: 100.0,
            w: 400.0,
            h: 400.0,
        };
        relayout(&mut tree);

        let label = find(&tree, "label_1");
        assert_eq!(label.rect.x, 109.0);
        assert_eq!(label.rect.y, 109.0);
        // The taller dialog gives all the slack to the vertical spacer.
        let vspacer = find_spacer(&tree, false);
        assert_eq!(vspacer.rect.h, 400.0 - 18.0 - 20.0 * 3.0 - 24.0 - 6.0 * 4.0);
        assert_eq!(
            get_form_property(&tree, "geometry"),
            Some(&PropValue::Rect {
                x: 100,
                y: 100,
                w: 400,
                h: 400
            })
        );
    }

    #[test]
    fn keyboard_editing_changes_line_edit_text() {
        let form = parse(FIXTURE).expect("fixture parses");
        let mut tree = build(&form);
        layout(
            &mut tree,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 388.0,
                h: 190.0,
            },
        );

        // The user types into lineEdit_1 (Lua initialised it with "1").
        set_property(&mut tree, "lineEdit_1", "text", PropValue::Str("1".into()));
        assert!(insert_text(&mut tree, "lineEdit_1", '2'));
        assert_eq!(
            get_property(&tree, "lineEdit_1", "text"),
            Some(&PropValue::Str("12".into()))
        );
        assert!(backspace_text(&mut tree, "lineEdit_1"));
        assert_eq!(
            get_property(&tree, "lineEdit_1", "text"),
            Some(&PropValue::Str("1".into()))
        );

        // Backspace on an empty value is a no-op, not a panic.
        assert!(backspace_text(&mut tree, "lineEdit_3"));
        assert!(backspace_text(&mut tree, "lineEdit_3"));
        assert_eq!(
            get_property(&tree, "lineEdit_3", "text"),
            Some(&PropValue::Str("".into()))
        );

        // Editing an integer text (Lua wrote a number) converts it.
        set_property(&mut tree, "lineEdit_3", "text", PropValue::Int(42));
        assert!(insert_text(&mut tree, "lineEdit_3", 'x'));
        assert_eq!(
            get_property(&tree, "lineEdit_3", "text"),
            Some(&PropValue::Str("42x".into()))
        );

        // Unknown widgets are reported instead of silently ignored.
        assert!(!insert_text(&mut tree, "ghost", 'a'));
        assert!(!backspace_text(&mut tree, "ghost"));
    }

    const TABLE_FORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ui version="4.0"><class>X</class>
 <widget class="QWidget" name="root">
  <layout class="QVBoxLayout" name="v">
   <item><widget class="QTableWidget" name="tbl">
    <property name="rowCount"><number>2</number></property>
    <property name="columnCount"><number>2</number></property>
   </widget></item>
  </layout>
 </widget>
</ui>"#;

    fn table_tree() -> Node {
        let form = parse(TABLE_FORM).expect("table form parses");
        build(&form)
    }

    #[test]
    fn table_headers_and_data_apply_to_widget() {
        let mut tree = table_tree();
        // The .ui declares the grid up front.
        {
            let t = table(&tree, "tbl").expect("table state");
            assert_eq!(t.row_count, 2);
            assert_eq!(t.widths.len(), 2);
        }

        let headers = PropValue::Map(vec![
            ("clear".into(), PropValue::Bool(true)),
            ("columnCount".into(), PropValue::Int(3)),
            (
                "horizontalHeaderLabels".into(),
                PropValue::List(vec![
                    PropValue::Str("№".into()),
                    PropValue::Str("Наименование".into()),
                    PropValue::Str("Сумма".into()),
                ]),
            ),
            (
                "columnWidths".into(),
                PropValue::List(vec![PropValue::Int(60), PropValue::Int(220)]),
            ),
            (
                "hiddenColumns".into(),
                PropValue::List(vec![PropValue::Int(3)]),
            ),
        ]);
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "tableHeaders",
            &headers
        ));

        // Sparse payload: map of 1-based rows, list- and map-shaped rows.
        let data = PropValue::Map(vec![
            ("rowCount".into(), PropValue::Int(100_000)),
            (
                "data".into(),
                PropValue::Map(vec![
                    (
                        "1".into(),
                        PropValue::List(vec![PropValue::Str("a".into()), PropValue::Int(10)]),
                    ),
                    (
                        "42".into(),
                        PropValue::Map(vec![("2".into(), PropValue::Str("deep".into()))]),
                    ),
                ]),
            ),
        ]);
        assert!(set_table_property(&mut tree, "tbl", "tableData", &data));

        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.headers, vec!["№", "Наименование", "Сумма"]);
        assert_eq!(t.widths, vec![60.0, 220.0, 90.0]);
        assert!(t.is_hidden(2));
        assert_eq!(t.row_count, 100_000);
        assert_eq!(t.cell(0, 0), Some("a"));
        assert_eq!(t.cell(0, 1), Some("10")); // numbers render as text
        assert_eq!(t.cell(41, 1), Some("deep"));
        assert_eq!(t.cell(7, 7), None); // sparse: nothing was pushed

        // Unknown widget / unknown property fall back to the caller.
        assert!(!set_table_property(&mut tree, "ghost", "tableData", &data));
        assert!(!set_table_property(&mut tree, "tbl", "windowTitle", &data));
    }

    #[test]
    fn table_virtualizes_rows_and_hit_tests() {
        let mut tree = table_tree();
        let data = PropValue::Map(vec![
            ("rowCount".into(), PropValue::Int(100_000)),
            ("data".into(), PropValue::Map(vec![])),
        ]);
        set_table_property(&mut tree, "tbl", "tableData", &data);
        set_table_property(
            &mut tree,
            "tbl",
            "tableHeaders",
            &PropValue::Map(vec![(
                "horizontalHeaderLabels".into(),
                PropValue::List(vec![PropValue::Str("A".into()), PropValue::Str("B".into())]),
            )]),
        );

        let t = table(&tree, "tbl").expect("table state");
        // Only the rows inside the viewport are ever rendered.
        assert_eq!(t.visible_row_range(200.0), 0..9);
        assert_eq!(t.scroll_max(200.0), 100_000.0 * ROW_H - 176.0);

        // Header is not a row; rows map linearly from the header edge.
        assert_eq!(t.row_at(10.0), None);
        assert_eq!(t.row_at(HEADER_H + 1.0), Some(0));
        assert_eq!(t.row_at(HEADER_H + 20.5), Some(1));

        // Column hit-test honours widths and hidden columns.
        assert_eq!(t.col_at(1.0), Some(0));
        assert_eq!(t.col_at(95.0), Some(1)); // 90px first column
        set_table_property(
            &mut tree,
            "tbl",
            "tableHeaders",
            &PropValue::Map(vec![(
                "hiddenColumns".into(),
                PropValue::List(vec![PropValue::Int(1)]),
            )]),
        );
        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.col_at(1.0), Some(1)); // first column is hidden
        assert_eq!(t.col_offset(1), 0.0);

        // Scrolling is clamped to the content.
        let t = table_mut(&mut tree, "tbl").expect("table state");
        t.scroll_by(5000.0, 200.0);
        assert_eq!(t.scroll, 5000.0);
        t.scroll_by(1e9, 200.0);
        assert_eq!(t.scroll, t.scroll_max(200.0));
        t.scroll_by(-1e9, 200.0);
        assert_eq!(t.scroll, 0.0);
    }

    #[test]
    fn table_selection_round_trips_through_get_property() {
        let mut tree = table_tree();
        set_table_property(
            &mut tree,
            "tbl",
            "tableData",
            &PropValue::Map(vec![
                ("rowCount".into(), PropValue::Int(3)),
                (
                    "data".into(),
                    PropValue::Map(vec![(
                        "2".into(),
                        PropValue::List(vec![
                            PropValue::Str("row2col1".into()),
                            PropValue::Str("row2col2".into()),
                        ]),
                    )]),
                ),
            ]),
        );

        // No selection yet: `currentRow` follows Qt (-1), cells are null.
        assert_eq!(
            get_table_property(&tree, "tbl", "currentRow"),
            Some(PropValue::Int(-1))
        );
        assert_eq!(get_table_property(&tree, "tbl", "currentRowCell:1"), None);

        // `currentRow` is 1-based on the wire (like `invoke_qt`).
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(2)
        ));
        assert_eq!(
            get_table_property(&tree, "tbl", "currentRow"),
            Some(PropValue::Int(1))
        );
        // `currentRowCell:<col>` is 1-based too; empty cells read as "".
        assert_eq!(
            get_table_property(&tree, "tbl", "currentRowCell:2"),
            Some(PropValue::Str("row2col2".into()))
        );
        assert_eq!(
            get_table_property(&tree, "tbl", "currentRowCell:1"),
            Some(PropValue::Str("row2col1".into()))
        );
        // Out-of-grid columns and foreign widgets read as null.
        assert_eq!(get_table_property(&tree, "tbl", "currentRowCell:9"), None);
        assert_eq!(get_table_property(&tree, "ghost", "currentRow"), None);

        // Flags and out-of-range rows.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "selectRows",
            &PropValue::Bool(true)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "readOnly",
            &PropValue::Bool(true)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(999)
        ));
        let t = table(&tree, "tbl").expect("table state");
        assert!(t.select_rows);
        assert!(t.read_only);
        assert_eq!(t.current_row, Some(2)); // clamped to the last row
    }

    #[test]
    fn table_accepts_bare_properties_from_lua_fixtures() {
        // Mirrors `priv/lua_modules/test_02.lua`: a QTableWidget is
        // filled with one `setProperty` per Qt property.
        let mut tree = table_tree();

        assert!(set_table_property(
            &mut tree,
            "tbl",
            "clear",
            &PropValue::Bool(true)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "rowCount",
            &PropValue::Int(3)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "columnCount",
            &PropValue::Int(2)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "horizontalHeaderLabels",
            &PropValue::List(vec![
                PropValue::Str("Col 1".into()),
                PropValue::Str("Col 2".into()),
            ]),
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "data",
            &PropValue::List(vec![
                PropValue::List(vec![
                    PropValue::Str("Row 1 Col 1".into()),
                    PropValue::Str("Row 1 Col 2".into()),
                ]),
                PropValue::List(vec![
                    PropValue::Str("Row 2 Col 1".into()),
                    PropValue::Str("Row 2 Col 2".into()),
                ]),
                PropValue::List(vec![
                    PropValue::Str("Row 3 Col 1".into()),
                    PropValue::Str("Row 3 Col 2".into()),
                ]),
            ]),
        ));

        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.row_count, 3);
        assert_eq!(t.column_count(), 2);
        assert_eq!(t.headers, vec!["Col 1", "Col 2"]);
        assert_eq!(t.cell(2, 1), Some("Row 3 Col 2"));
        assert_eq!(
            get_table_property(&tree, "tbl", "rowCount"),
            Some(PropValue::Int(3))
        );
        assert_eq!(
            get_table_property(&tree, "tbl", "columnCount"),
            Some(PropValue::Int(2))
        );

        // `hideColumn {index, hidden}` is 1-based on the wire.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "hideColumn",
            &PropValue::Map(vec![("index".into(), PropValue::Int(1))]),
        ));
        assert!(table(&tree, "tbl").expect("table state").is_hidden(0));
        // Hiding is view-only; the cell data survives an unhide.
        assert_eq!(
            table(&tree, "tbl").expect("table state").cell(2, 0),
            Some("Row 3 Col 1")
        );
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "hideColumn",
            &PropValue::Map(vec![
                ("index".into(), PropValue::Int(1)),
                ("hidden".into(), PropValue::Bool(false)),
            ]),
        ));
        assert!(!table(&tree, "tbl").expect("table state").is_hidden(0));

        // Shrinking rows drops stale cells and an out-of-range selection.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(3)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "rowCount",
            &PropValue::Int(1)
        ));
        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.row_count, 1);
        assert_eq!(t.cell(1, 0), None);
        assert_eq!(t.current_row, None);

        // `tableData` without `rowCount` resets the grid to zero rows,
        // matching `invoke_qt`'s `cfg.value("rowCount", 0)`.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "tableData",
            &PropValue::Map(vec![("data".into(), PropValue::Map(vec![]))]),
        ));
        assert_eq!(table(&tree, "tbl").expect("table state").row_count, 0);
    }

    #[test]
    fn selected_row_scrolls_into_view() {
        let mut tree = table_tree();
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "rowCount",
            &PropValue::Int(100)
        ));
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(100)
        ));

        reveal_table_row(&mut tree, "tbl", 200.0);
        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.current_row, Some(99));
        assert_eq!(t.scroll, t.scroll_max(200.0));

        // Re-selecting the same row keeps the offset.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(100)
        ));
        reveal_table_row(&mut tree, "tbl", 200.0);
        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.scroll, t.scroll_max(200.0));

        // Selecting a row above the viewport scrolls back up to it.
        assert!(set_table_property(
            &mut tree,
            "tbl",
            "currentRow",
            &PropValue::Int(2)
        ));
        reveal_table_row(&mut tree, "tbl", 200.0);
        let t = table(&tree, "tbl").expect("table state");
        assert_eq!(t.current_row, Some(1));
        assert_eq!(t.scroll, ROW_H);
    }
}
