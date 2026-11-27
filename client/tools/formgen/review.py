#!/usr/bin/env python3
import html
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REVIEW = ROOT / "review"
FORMS = REVIEW / "forms"
QT_PNG = REVIEW / "qt"
RUST_PNG = REVIEW / "rust"
COMMENTS = REVIEW / "comments.md"
MANIFEST = REVIEW / "manifest.json"
QT_TOOL = ROOT / "tools" / "qt_reference"
QT_BIN = QT_TOOL / "qt_reference"
RQT_RENDER = ROOT / "target" / "debug" / "rqt_render"
ICONS = REVIEW / "icons"
SERVER_QT_FORMS = ROOT.parent / "server" / "priv" / "qt_forms"
SERVER_LUA_MODULES = ROOT.parent / "server" / "priv" / "lua_modules"


def esc(text):
    return html.escape(str(text), quote=True)


def prop_text(name, text):
    return f'<property name="{name}"><string>{esc(text)}</string></property>'


def prop_bool(name, value):
    return f'<property name="{name}"><bool>{"true" if value else "false"}</bool></property>'


def prop_int(name, value):
    return f'<property name="{name}"><number>{value}</number></property>'


def prop_enum(name, value):
    return f'<property name="{name}"><enum>{value}</enum></property>'


def widget(cls, name, props="", layout=""):
    return f'<widget class="{cls}" name="{name}">{props}{layout}</widget>'


def attr_text(name, text):
    return f'<attribute name="{name}"><string>{esc(text)}</string></attribute>'


def tab_page(name, title, items, spacing=6, margins=9):
    """QTabWidget page: child QWidget with title attribute + layout."""
    return widget(
        "QWidget",
        name,
        attr_text("title", title),
        layout("QVBoxLayout", name + "Layout", items, spacing, margins),
    )


def tab_widget(name, pages, current=0):
    body = "".join(pages)
    return widget("QTabWidget", name, prop_int("currentIndex", current), body)


def toolbox_page(name, label, items, spacing=6, margins=9):
    """QToolBox page: child QWidget with label attribute + layout."""
    return widget(
        "QWidget",
        name,
        attr_text("label", label),
        layout("QVBoxLayout", name + "Layout", items, spacing, margins),
    )


def toolbox_widget(name, pages, current=0):
    body = "".join(pages)
    return widget("QToolBox", name, prop_int("currentIndex", current), body)



def plain_text_edit(name, plain="", wrap="QPlainTextEdit::WidgetWidth"):
    props = ""
    if plain:
        props += prop_text("plainText", plain)
    if wrap:
        props += prop_enum("lineWrapMode", wrap)
    return widget("QPlainTextEdit", name, props)

def spin_box(name, value=0, minimum=0, maximum=99, suffix=""):
    props = (
        prop_int("minimum", minimum)
        + prop_int("maximum", maximum)
        + prop_int("value", value)
        + prop_int("singleStep", 1)
    )
    if suffix:
        props += prop_text("suffix", suffix)
    return widget("QSpinBox", name, props)


def slider(name, value=0, minimum=0, maximum=100, orientation="Qt::Horizontal"):
    return widget(
        "QSlider",
        name,
        prop_enum("orientation", orientation)
        + prop_int("minimum", minimum)
        + prop_int("maximum", maximum)
        + prop_int("value", value)
        + prop_int("singleStep", 1)
        + prop_int("pageStep", 10),
    )


def progress_bar(name, value=0, minimum=0, maximum=100, text_visible=True, orientation="Qt::Horizontal"):
    props = (
        prop_enum("orientation", orientation)
        + prop_int("minimum", minimum)
        + prop_int("maximum", maximum)
        + prop_int("value", value)
        + prop_bool("textVisible", text_visible)
    )
    return widget("QProgressBar", name, props)


def prop_double(name, value):
    return f'<property name="{name}"><double>{value}</double></property>'


def double_spin_box(name, value="0.00", minimum="0", maximum="99.99", decimals=2, prefix=""):
    # Qt QUiLoader only applies float spin props from `<double>` tags;
    # `<number>12.5</number>` is ignored and the box stays at 0.00.
    props = (
        prop_int("decimals", decimals)
        + prop_double("minimum", minimum)
        + prop_double("maximum", maximum)
        + prop_double("value", value)
        + prop_double("singleStep", "0.5")
    )
    if prefix:
        props += prop_text("prefix", prefix)
    return widget("QDoubleSpinBox", name, props)


def label(name, text):
    return widget("QLabel", name, prop_text("text", text))


def line(name, text="", enabled=True):
    props = ""
    if text:
        props += prop_text("text", text)
    if not enabled:
        props += prop_bool("enabled", False)
    return widget("QLineEdit", name, props)


def button(name, text, enabled=True):
    return widget(
        "QPushButton",
        name,
        prop_text("text", text) + ("" if enabled else prop_bool("enabled", False)),
    )


def button_state(name, text, icon=False, checked=False, enabled=True, default=False):
    props = prop_text("text", text)
    if icon:
        props += (
            '<property name="icon"><iconset>'
            '<normaloff>review/icons/button_state.png</normaloff>'
            '<disabledoff>review/icons/button_state.png</disabledoff>'
            "</iconset></property>"
        )
    if checked:
        props += prop_bool("checkable", True) + prop_bool("checked", True)
    if not enabled:
        props += prop_bool("enabled", False)
    if default:
        props += prop_bool("default", True)
    return widget("QPushButton", name, props)


def ensure_icon():
    from PIL import Image, ImageDraw

    ICONS.mkdir(parents=True, exist_ok=True)
    path = ICONS / "button_state.png"
    if path.exists():
        return
    image = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.ellipse((1, 1, 14, 14), fill=(220, 40, 40, 255), outline=(120, 0, 0, 255))
    draw.rectangle((7, 4, 8, 11), fill=(255, 255, 255, 255))
    image.save(path)


def checkbox(name, text, checked=False):
    return widget("QCheckBox", name, prop_text("text", text) + prop_bool("checked", checked))


def radio(name, text, checked=False):
    return widget("QRadioButton", name, prop_text("text", text) + prop_bool("checked", checked))


def combo(name):
    return widget("QComboBox", name)


def list_widget(name):
    return widget("QListWidget", name)


def tree_widget(name):
    return widget("QTreeWidget", name)


def table(name, rows, cols):
    return widget("QTableWidget", name, prop_int("rowCount", rows) + prop_int("columnCount", cols))


def table_with_policy(name, rows, cols, vertical=None, horizontal=None):
    props = prop_int("rowCount", rows) + prop_int("columnCount", cols)
    if vertical:
        props += prop_enum("verticalScrollBarPolicy", vertical)
    if horizontal:
        props += prop_enum("horizontalScrollBarPolicy", horizontal)
    return widget("QTableWidget", name, props)


def layout(cls, name, items, spacing=6, margins=11):
    if isinstance(items, str):
        items = [items]
    body = "".join(f"<item>{item}</item>" for item in items)
    props = "".join(
        f'<property name="{key}"><number>{value}</number></property>'
        for key, value in (
            ("leftMargin", margins),
            ("topMargin", margins),
            ("rightMargin", margins),
            ("bottomMargin", margins),
            ("spacing", spacing),
        )
    )
    return f'<layout class="{cls}" name="{name}">{props}{body}</layout>'


def group(title, name, items, spacing=6, margins=9):
    return widget(
        "QGroupBox",
        name,
        prop_text("title", title),
        layout("QVBoxLayout", name + "Layout", items, spacing, margins),
    )


def form(title, width, height, body, spacing=6, margins=11, style=""):
    root_layout = layout("QVBoxLayout", "verticalLayout", body, spacing, margins)
    root = widget(
        "QDialog",
        "Dialog",
        (
            '<property name="geometry"><rect>'
            f'<x>0</x><y>0</y><width>{width}</width><height>{height}</height>'
            "</rect></property>"
            + prop_text("windowTitle", title)
            + (prop_text("styleSheet", style) if style else "")
        ),
        root_layout,
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<ui version="4.0"><class>Dialog</class>{root}</ui>\n'
    )


def row(label_text, line_text="", enabled=True):
    return layout(
        "QHBoxLayout",
        "rowLayout",
        [label("label", label_text), line("lineEdit", line_text, enabled)],
        spacing=6,
        margins=0,
    )


def spacer(horizontal, width=40, height=20):
    if horizontal:
        return (
            '<spacer name="horizontalSpacer"><property name="orientation">'
            "<enum>Qt::Orientation::Horizontal</enum></property>"
            f'<property name="sizeHint" stdset="0"><size><width>{width}</width><height>{height}</height></size></property>'
            "</spacer>"
        )
    return (
        '<spacer name="verticalSpacer"><property name="orientation">'
        "<enum>Qt::Orientation::Vertical</enum></property>"
        f'<property name="sizeHint" stdset="0"><size><width>{width}</width><height>{height}</height></size></property>'
        "</spacer>"
    )


def prop_size_policy(horizontal, vertical):
    return (
        '<property name="sizePolicy">'
        f'<sizepolicy hsizetype="{horizontal}" vsizetype="{vertical}">'
        "<horstretch>0</horstretch><verstretch>0</verstretch>"
        "</sizepolicy></property>"
    )


def grid_item(row, column, content, rowspan=1, colspan=1, alignment=None):
    attrs = f'row="{row}" column="{column}"'
    if rowspan != 1:
        attrs += f' rowspan="{rowspan}"'
    if colspan != 1:
        attrs += f' colspan="{colspan}"'
    if alignment:
        attrs += f' alignment="{alignment}"'
    return f"<item {attrs}>{content}</item>"


def grid_layout(name, items, spacing=6, margins=11):
    body = "".join(items)
    props = "".join(
        f'<property name="{key}"><number>{value}</number></property>'
        for key, value in (
            ("leftMargin", margins),
            ("topMargin", margins),
            ("rightMargin", margins),
            ("bottomMargin", margins),
            ("spacing", spacing),
        )
    )
    return f'<layout class="QGridLayout" name="{name}">{props}{body}</layout>'


def form_grid(title, width, height, grid):
    root = widget(
        "QDialog",
        "Dialog",
        (
            '<property name="geometry"><rect>'
            f'<x>0</x><y>0</y><width>{width}</width><height>{height}</height>'
            "</rect></property>"
            + prop_text("windowTitle", title)
        ),
        grid,
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<ui version="4.0"><class>Dialog</class>{root}</ui>\n'
    )


def add_grid(cases, name, title, width, height, items, description=""):
    cases.append(
        {
            "id": f"grid/{name}",
            "category": "grid",
            "name": name,
            "title": title,
            "description": description,
            "xml": form_grid(
                title, width, height, grid_layout("gridLayout", items)
            ),
        }
    )


def form_layout(name, items, props=""):
    body = "".join(items)
    return f'<layout class="QFormLayout" name="{name}">{props}{body}</layout>'


def form_prop_enum(name, value):
    return f'<property name="{name}"><enum>QFormLayout::{value}</enum></property>'


def form_prop_set(name, flags):
    return f'<property name="{name}"><set>{flags}</set></property>'


def add_form(cases, name, title, width, height, items, props="", description=""):
    cases.append(
        {
            "id": f"form/{name}",
            "category": "form",
            "name": name,
            "title": title,
            "description": description,
            "xml": form_grid(
                title, width, height, form_layout("formLayout", items, props)
            ),
        }
    )


def add(
    cases,
    category,
    name,
    title,
    body,
    width=420,
    height=300,
    description="",
    spacing=6,
    margins=11,
    style="",
):
    cases.append(
        {
            "id": f"{category}/{name}",
            "category": category,
            "name": name,
            "title": title,
            "description": description,
            "xml": form(
                title, width, height, body, spacing=spacing, margins=margins, style=style
            ),
        }
    )


def build_cases():
    cases = []

    texts = [
        "Short",
        "Medium label text",
        "A very long label that should still be readable in layout",
        "A=",
        "Итого:",
    ]
    for index, text in enumerate(texts):
        add(
            cases,
            "basic",
            f"label_button_{index}",
            f"Label {index}",
            [
                label("label_1", text),
                line("lineEdit_1", "12345" if index % 2 else ""),
                layout(
                    "QHBoxLayout",
                    "buttons",
                    [spacer(True), button("ok", "OK"), button("cancel", "Cancel")],
                    6,
                    0,
                ),
                spacer(False),
            ],
            description="label + line edit + button row",
        )

    for index, text in enumerate(texts):
        add(
            cases,
            "basic",
            f"form_rows_{index}",
            f"Rows {index}",
            [
                row("A=", str(10 + index)),
                row("B=", str(20 + index)),
                row("A*B=", "", enabled=False),
                layout(
                    "QHBoxLayout",
                    "buttons",
                    [spacer(True), button("calc", "Calculate"), button("close", "Close")],
                    6,
                    0,
                ),
            ],
            height=320,
            description="vertical form rows with disabled result",
        )

    for index in range(5):
        margins = 4 + index * 4
        add(
            cases,
            "spacing",
            f"margins_{margins}",
            f"Margins {margins}",
            [
                label("title", f"Margins {margins}"),
                layout(
                    "QHBoxLayout",
                    "row",
                    [label("left", "Left"), line("edit", "value"), button("go", "Go")],
                    6,
                    0,
                ),
            ],
            width=420,
            height=220,
            description="root margins vary",
        )
        cases[-1]["xml"] = form(
            f"Margins {margins}",
            420,
            220,
            [
                label("title", f"Margins {margins}"),
                layout(
                    "QHBoxLayout",
                    "row",
                    [label("left", "Left"), line("edit", "value"), button("go", "Go")],
                    6,
                    0,
                ),
            ],
            spacing=6,
            margins=margins,
        )

    for index in range(5):
        add(
            cases,
            "spacing",
            f"spacing_{index}",
            f"Spacing {index}",
            [
                label("l1", "First"),
                label("l2", "Second"),
                label("l3", "Third"),
                line("edit", f"Value {index}"),
                button("ok", "OK"),
            ],
            spacing=index * 6,
            description="box spacing varies",
        )

    for index in range(6):
        add(
            cases,
            "controls",
            f"checks_{index}",
            f"Checks {index}",
            [
                checkbox("check_1", "Enabled", index % 2 == 0),
                checkbox("check_2", "Disabled", False),
                radio("radio_1", "First option", index % 3 == 0),
                radio("radio_2", "Second option", index % 3 == 1),
                combo("combo"),
                spacer(False),
            ],
            height=320,
            description="checkbox/radio checked states and combo",
        )

    for index in range(4):
        add(
            cases,
            "controls",
            f"lists_{index}",
            f"Lists {index}",
            [combo("combo"), list_widget("list"), tree_widget("tree")],
            width=480,
            height=420,
            description="combo/list/tree placeholders",
        )

    table_sizes = [(1, 1), (2, 2), (3, 2), (4, 3), (3, 4), (2, 5), (5, 2), (10, 4), (20, 3), (2, 3), (6, 2), (8, 5)]
    for rows, cols in table_sizes:
        add(
            cases,
            "tables",
            f"table_{rows}x{cols}",
            f"Table {rows}x{cols}",
            [table("table", rows, cols)],
            width=500,
            height=360,
            description=f"QTableWidget {rows} rows x {cols} columns",
        )

    for index in range(4):
        add(
            cases,
            "groups",
            f"group_{index}",
            f"Group {index}",
            [
                group(
                    f"Group {index}",
                    "group",
                    [
                        checkbox("check_1", "Option A", index % 2 == 0),
                        checkbox("check_2", "Option B", False),
                        radio("radio_1", "Choice A", index % 2 == 1),
                        radio("radio_2", "Choice B", False),
                    ],
                ),
                group(
                    "Nested",
                    "group_2",
                    [label("inside", "Nested content"), line("inside_edit", "value")],
                ),
                spacer(False),
            ],
            width=440,
            height=380,
            description="QGroupBox and nested controls",
        )

    for index in range(4):
        add(
            cases,
            "tables",
            f"table_with_form_{index}",
            f"Table form {index}",
            [
                row("Filter:", f"value {index}"),
                table("table", 3 + index, 2),
                layout(
                    "QHBoxLayout",
                    "buttons",
                    [spacer(True), button("apply", "Apply"), button("close", "Close")],
                    6,
                    0,
                ),
            ],
            width=520,
            height=420,
            description="table plus form controls",
        )

    qss_buttons = """
QDialog { background-color: #f7f3e8; }
QLabel { color: #7a1f1f; }
QPushButton { background-color: #2f6f4f; color: #ffffff; border: 2px solid #1f4f37; border-radius: 5px; padding: 4px 10px; }
QPushButton:disabled { background-color: #d7d7d7; color: #666666; border-color: #b0b0b0; }
QLineEdit { background-color: #fffbe6; border: 1px solid #c7a34d; padding: 2px 5px; }
"""
    add(
        cases,
        "qss",
        "buttons",
        "QSS buttons",
        [
            label("title", "Styled buttons"),
            line("edit", "Styled input"),
            layout(
                "QHBoxLayout",
                "buttons",
                [spacer(True), button("ok", "Primary"), button("cancel", "Disabled")],
                6,
                0,
            ),
        ],
        width=460,
        height=260,
        description="background/color/border/radius/padding",
        style=qss_buttons,
    )
    add(
        cases,
        "qss",
        "form",
        "QSS form",
        [
            row("Name:", "Ivan"),
            row("Code:", "42"),
            row("Result:", "OK"),
            layout(
                "QHBoxLayout",
                "buttons",
                [spacer(True), button("save", "Save")],
                6,
                0,
            ),
        ],
        width=460,
        height=320,
        description="QSS applied to a simple form",
        style=qss_buttons,
    )
    qss_checks = """
QDialog { background-color: #eef3f0; }
QCheckBox { color: #24463a; padding: 2px; }
QCheckBox:checked { color: #1565c0; }
QRadioButton { color: #5a3a1e; }
QGroupBox { border: 2px solid #8aa39a; border-radius: 4px; margin-top: 8px; }
"""
    add(
        cases,
        "qss",
        "checks",
        "QSS checks",
        [
            group(
                "Options",
                "group",
                [
                    checkbox("check_1", "Checked option", True),
                    checkbox("check_2", "Unchecked option", False),
                    radio("radio_1", "First", True),
                    radio("radio_2", "Second", False),
                ],
            ),
            spacer(False),
        ],
        width=440,
        height=340,
        description="QSS on checkbox/radio/groupbox",
        style=qss_checks,
    )
    qss_table = """
QDialog { background-color: #f5f5f5; }
QTableWidget { background-color: #ffffff; border: 2px solid #3f51b5; }
"""
    add(
        cases,
        "qss",
        "table",
        "QSS table",
        [table("table", 4, 3)],
        width=500,
        height=340,
        description="QSS border/background on table",
        style=qss_table,
    )
    add(
        cases,
        "qss",
        "nested",
        "QSS nested",
        [
            group(
                "Styled group",
                "group",
                [
                    label("inside", "Inside group"),
                    line("inside_edit", "value"),
                ],
            ),
            layout(
                "QHBoxLayout",
                "buttons",
                [spacer(True), button("apply", "Apply"), button("close", "Close")],
                6,
                0,
            ),
        ],
        width=460,
        height=360,
        description="QSS with nested group",
        style=qss_buttons,
    )
    add(
        cases,
        "qss",
        "disabled",
        "QSS disabled",
        [
            label("title", "Disabled controls"),
            line("disabled_edit", "locked", enabled=False),
            button("disabled_button", "Disabled", enabled=False),
            checkbox("disabled_check", "Disabled check", False),
        ],
        width=440,
        height=260,
        description="disabled style hook",
        style=qss_buttons,
    )

    ensure_icon()
    add(
        cases,
        "basic",
        "button_states",
        "Button states",
        [
            button_state("normal", "Normal", icon=True),
            button_state("default", "Default", default=True),
            button_state("checked", "Checked", icon=True, checked=True),
            button_state("disabled", "Disabled", icon=True, enabled=False),
            button_state("icon_only", "", icon=True),
        ],
        width=420,
        height=330,
        description="Fusion gradients, checked, disabled and icon buttons",
    )

    scroll_cases = [
        ("vertical_50x3", 50, 3, 420, 240, "vertical scrollbar, 50 rows"),
        ("vertical_100x2", 100, 2, 420, 300, "long vertical scrollbar"),
        ("horizontal_3x10", 3, 10, 420, 240, "horizontal scrollbar, 10 columns"),
        ("both_30x8", 30, 8, 500, 320, "vertical and horizontal scrollbars"),
        ("no_scroll_3x2", 3, 2, 500, 300, "content fits, no scrollbars"),
        ("narrow_5x12", 5, 12, 300, 220, "narrow viewport, horizontal overflow"),
        ("tall_60x1", 60, 1, 300, 200, "tall table, one column"),
        ("wide_2x20", 2, 20, 300, 200, "wide table, two rows"),
        ("both_8x6_small", 8, 6, 320, 200, "both scrollbars in a small table"),
        ("form_overflow", 40, 5, 460, 300, "table with overflow plus controls"),
    ]
    for name, rows, cols, width, height, description in scroll_cases:
        body = [table("table", rows, cols)]
        if name == "form_overflow":
            body = [
                row("Filter:", "value"),
                table("table", rows, cols),
                layout("QHBoxLayout", "buttons", [button("ok", "OK")], 6, 0),
            ]
        add(
            cases,
            "scroll",
            name,
            f"Scroll {name}",
            body,
            width=width,
            height=height,
            description=description,
        )

    selection_cases = [
        ("rows_20", 20, 3, "table with many rows and focus"),
        ("columns_12", 4, 12, "table with many columns"),
        ("both_25x9", 25, 9, "large table with both scrollbars"),
        ("empty_0x3", 0, 3, "empty table baseline"),
    ]
    for name, rows, cols, description in selection_cases:
        add(
            cases,
            "selection",
            name,
            f"Selection {name}",
            [table("table", rows, cols)],
            width=460,
            height=280,
            description=description,
        )

    policy_cases = [
        ("policy_off_both", 50, 8, "Qt::ScrollBarAlwaysOff", "Qt::ScrollBarAlwaysOff"),
        ("policy_off_vertical", 50, 3, "Qt::ScrollBarAlwaysOff", "Qt::ScrollBarAlwaysOn"),
        ("policy_off_horizontal", 30, 10, "Qt::ScrollBarAlwaysOn", "Qt::ScrollBarAlwaysOff"),
        ("policy_on_small", 2, 2, "Qt::ScrollBarAlwaysOn", "Qt::ScrollBarAlwaysOn"),
        ("policy_as_needed", 30, 8, "Qt::ScrollBarAsNeeded", "Qt::ScrollBarAsNeeded"),
    ]
    for name, rows, cols, vertical, horizontal in policy_cases:
        add(
            cases,
            "scroll",
            name,
            f"Scroll {name}",
            [table_with_policy("table", rows, cols, vertical, horizontal)],
            width=460,
            height=260,
            description=f"scrollbar policy {vertical} / {horizontal}",
        )

    add_grid(
        cases,
        "2x2",
        "Grid 2x2",
        320,
        220,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, label("label_2", "Code:")),
            grid_item(1, 1, combo("comboBox_1")),
        ],
        description="simple 2x2 grid, label/field pairs",
    )
    add_grid(
        cases,
        "spans",
        "Grid spans",
        400,
        300,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(0, 1, line("lineEdit_1"), colspan=2),
            grid_item(
                1,
                0,
                button("ok", "OK"),
                rowspan=2,
                alignment="Qt::AlignHCenter|Qt::AlignVCenter",
            ),
            grid_item(1, 1, table("table", 10, 3), colspan=2),
            grid_item(2, 1, label("label_2", "Status:")),
            grid_item(2, 2, label("label_3", "Ready")),
        ],
        description="rowspan/colspan with item alignment",
    )
    add_grid(
        cases,
        "nested",
        "Grid nested",
        420,
        300,
        [
            grid_item(0, 0, label("label_1", "Filter:")),
            grid_item(0, 1, line("lineEdit_1"), colspan=2),
            grid_item(
                1,
                0,
                layout(
                    "QHBoxLayout",
                    "noteLayout",
                    [label("label_2", "Note:"), line("lineEdit_2")],
                    6,
                    0,
                ),
                colspan=3,
            ),
            grid_item(2, 0, spacer(True), colspan=2),
            grid_item(2, 2, button("close", "Close")),
        ],
        description="nested box layout and spacer inside grid cells",
    )
    add_grid(
        cases,
        "policies",
        "Grid policies",
        480,
        340,
        [
            grid_item(
                0,
                0,
                widget(
                    "QPushButton",
                    "fixed",
                    prop_size_policy("Fixed", "Fixed") + prop_text("text", "Fixed"),
                ),
            ),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, table("table", 8, 3), colspan=2),
            grid_item(
                2,
                0,
                widget(
                    "QLabel",
                    "growing",
                    prop_size_policy("Expanding", "Preferred")
                    + prop_text("text", "Status:"),
                ),
            ),
            grid_item(2, 1, label("label_2", "ready")),
        ],
        description="explicit sizePolicy Expanding/Fixed in a grid",
    )

    add_form(
        cases,
        "basic",
        "Form basic",
        360,
        190,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, label("label_2", "Code:")),
            grid_item(1, 1, combo("comboBox_1")),
            grid_item(2, 0, label("label_3", "Enabled")),
            grid_item(2, 1, checkbox("checkBox_1", "Check")),
        ],
        description="line edit, combo and checkbox in a two-column form",
    )
    add_form(
        cases,
        "long_labels",
        "Form long labels",
        300,
        220,
        [
            grid_item(0, 0, label("label_1", "A very long label that should wrap:")),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, label("label_2", "Short:")),
            grid_item(1, 1, line("lineEdit_2")),
            grid_item(2, 0, label("label_3", "Code:")),
            grid_item(2, 1, combo("comboBox_1")),
        ],
        props=form_prop_enum("rowWrapPolicy", "WrapLongRows"),
        description="WrapLongRows with a label wider than the label column",
    )
    add_form(
        cases,
        "wrap_all",
        "Form wrap all",
        320,
        250,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, label("label_2", "Description:")),
            grid_item(1, 1, line("lineEdit_2")),
            grid_item(2, 0, label("label_3", "Enabled")),
            grid_item(2, 1, checkbox("checkBox_1", "Check")),
        ],
        props=form_prop_enum("rowWrapPolicy", "WrapAllRows"),
        description="WrapAllRows puts every field below its label",
    )
    add_form(
        cases,
        "growth",
        "Form growth policy",
        460,
        200,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(0, 1, line("lineEdit_1")),
            grid_item(1, 0, label("label_2", "Code:")),
            grid_item(1, 1, combo("comboBox_1")),
            grid_item(
                2,
                0,
                widget(
                    "QLabel",
                    "label_3",
                    prop_text("text", "Status:")
                    + prop_size_policy("Expanding", "Preferred"),
                ),
            ),
            grid_item(2, 1, label("statusLabel", "Ready")),
        ],
        props=form_prop_enum("fieldGrowthPolicy", "FieldsStayAtSizeHint"),
        description="FieldsStayAtSizeHint keeps fields at their size hint",
    )
    add_form(
        cases,
        "alignment",
        "Form alignment",
        420,
        210,
        [
            grid_item(0, 0, label("label_1", "Name:")),
            grid_item(
                0,
                1,
                widget(
                    "QLineEdit",
                    "lineEdit_1",
                    prop_size_policy("Fixed", "Fixed")
                    + '<property name="minimumSize"><size><width>160</width><height>0</height></size></property>',
                ),
            ),
            grid_item(1, 0, label("label_2", "Code:")),
            grid_item(1, 1, combo("comboBox_1")),
            grid_item(2, 0, label("label_3", "Enabled")),
            grid_item(2, 1, checkbox("checkBox_1", "Check")),
        ],
        props=(
            form_prop_enum("fieldGrowthPolicy", "FieldsStayAtSizeHint")
            + form_prop_set("labelAlignment", "Qt::AlignRight|Qt::AlignVCenter")
            + form_prop_set("formAlignment", "Qt::AlignHCenter|Qt::AlignTop")
            + '<property name="horizontalSpacing"><number>12</number></property>'
            + '<property name="verticalSpacing"><number>9</number></property>'
        ),
        description="right-aligned labels, centred form and custom spacing",
    )
    add(
        cases,
        "form",
        "group",
        "Form in group box",
        [
            widget(
                "QGroupBox",
                "groupBox",
                prop_text("title", "Details"),
                form_layout(
                    "formLayout",
                    [
                        grid_item(0, 0, label("label_1", "Name:")),
                        grid_item(0, 1, line("lineEdit_1")),
                        grid_item(1, 0, label("label_2", "Code:")),
                        grid_item(1, 1, combo("comboBox_1")),
                        grid_item(2, 0, label("label_3", "Enabled")),
                        grid_item(2, 1, checkbox("checkBox_1", "Check")),
                    ],
                ),
            ),
            spacer(False),
        ],
        width=380,
        height=270,
        description="nested group box with a form layout inside",
    )

    assert len(cases) >= 70, len(cases)
    return cases


def build_gallery_cases():
    """Deterministic matrix of forms for manual widget testing (0.18.5).

    Ids are stable: `gallery/<category>/<case>`. The window title carries
    the sequential `#NNN` number assigned in `main()` (via `make_xml`),
    matching `gallery_index.lua` that drives `gallery.lua`.
    Only supported widget classes are used; views stay item-less (parity
    with Qt renders, items would come from Lua at runtime).
    """
    cases = []

    def add_case(category, name, description, make_xml):
        cases.append(
            {
                "id": f"{category}/{name}",
                "category": category,
                "name": name,
                "title": "",
                "description": description,
                "make_xml": make_xml,
            }
        )

    def vg(category, name, description, body, width=460, height=320, spacing=6, margins=11, style=""):
        def make_xml(title, body=body, width=width, height=height,
                     spacing=spacing, margins=margins, style=style):
            return form(
                title, width, height, body, spacing=spacing, margins=margins, style=style
            )
        add_case(category, name, description, make_xml)

    def gr(category, name, description, items, width=460, height=320, spacing=6, margins=11):
        def make_xml(title, items=items, width=width, height=height,
                     spacing=spacing, margins=margins):
            return form_grid(
                title, width, height,
                grid_layout("gridLayout", items, spacing, margins),
            )
        add_case(category, name, description, make_xml)

    def fl(category, name, description, items, props="", width=460, height=320):
        def make_xml(title, items=items, props=props, width=width, height=height):
            return form_grid(
                title, width, height,
                form_layout("formLayout", items, props),
            )
        add_case(category, name, description, make_xml)

    # ---- gallery/hbox: horizontal mixes ----
    category = "gallery/hbox"
    vg(category, "label_line_btn", "label + line edit + button in one row",
       [layout("QHBoxLayout", "row",
               [label("label_1", "Name:"), line("lineEdit_1", "value"),
                button("ok", "OK")], 6, 0)])
    vg(category, "spacer_between", "buttons pushed apart by a horizontal spacer",
       [layout("QHBoxLayout", "row",
               [button("left", "Left"), spacer(True), button("right", "Right")], 6, 0)])
    vg(category, "disabled_mix", "disabled label/line/button in a row",
       [layout("QHBoxLayout", "row",
               [widget("QLabel", "label_1",
                       prop_text("text", "Locked") + prop_bool("enabled", False)),
                widget("QLineEdit", "lineEdit_1",
                       prop_text("text", "read-only") + prop_bool("enabled", False)),
                widget("QPushButton", "go",
                       prop_text("text", "Go") + prop_bool("enabled", False))], 6, 0)])
    vg(category, "long_text", "long label next to a fixed field",
       [layout("QHBoxLayout", "row",
               [label("label_1", "A very long label that must not be clipped in the row"),
                line("lineEdit_1")], 6, 0)])
    vg(category, "cyrillic", "cyrillic label and button",
       [layout("QHBoxLayout", "row",
               [label("label_1", "Итого:"), line("lineEdit_1", "1 234,56"),
                button("calc", "Рассчитать")], 6, 0)])
    vg(category, "checks_radio", "checkbox, radio and another checkbox",
       [layout("QHBoxLayout", "row",
               [checkbox("check_1", "Включить", True),
                radio("radio_1", "Выбор", True),
                checkbox("check_2", "Ещё", False)], 6, 0)])
    vg(category, "combo_line", "combo box beside a line edit",
       [layout("QHBoxLayout", "row",
               [combo("comboBox_1"), line("lineEdit_1", "text")], 6, 0)])
    vg(category, "button_states_row", "normal/default/disabled buttons in a row",
       [layout("QHBoxLayout", "row",
               [button_state("btn_normal", "Обычная"),
                button_state("btn_default", "Default", default=True),
                button_state("btn_disabled", "Отключена", enabled=False)], 6, 0)],
       width=520)
    vg(category, "fixed_sizes", "fixed-size widgets inside a horizontal row",
       [layout("QHBoxLayout", "row",
               [widget("QPushButton", "fixed_btn",
                       prop_size_policy("Fixed", "Fixed") + prop_text("text", "Fixed")),
                widget("QLineEdit", "fixed_edit",
                       prop_size_policy("Fixed", "Fixed")
                       + '<property name="minimumSize"><size><width>140</width><height>0</height></size></property>'),
                widget("QLabel", "grow_label",
                       prop_size_policy("Expanding", "Preferred") + prop_text("text", "Expanding"))],
               6, 0)])
    vg(category, "empty_and_widgets", "empty label keeps the row layout honest",
       [layout("QHBoxLayout", "row",
               [label("label_empty", ""), line("lineEdit_1"),
                button("ok", "OK"), label("label_tail", "хвост")], 6, 0)])

    # ---- gallery/vbox: vertical stacks ----
    category = "gallery/vbox"
    vg(category, "labels_only", "four labels stacked",
       [label("l1", "Первая строка"), label("l2", "Вторая строка"),
        label("l3", "Третья строка"), label("l4", "Четвёртая")])
    vg(category, "fields_stack", "label plus three inputs",
       [label("l1", "Поля:"), line("e1", "один"), line("e2", "два"),
        line("e3", "три")], height=280)
    vg(category, "buttons_stack", "three buttons right aligned with spacers",
       [button("b1", "Применить"), spacer(False), button("b2", "Сохранить"),
        button("b3", "Отмена")], height=280)
    vg(category, "mixed_disabled", "stack with disabled and enabled widgets",
       [label("l1", "Активная подпись"),
        widget("QLineEdit", "e1", prop_text("text", "живое поле")),
        widget("QLineEdit", "e2",
               prop_text("text", "мёртвое поле") + prop_bool("enabled", False)),
        widget("QPushButton", "b1",
               prop_text("text", "Мёртвая кнопка") + prop_bool("enabled", False))])
    vg(category, "long_lines", "long texts wrapping behaviour",
       [label("l1", "Очень длинная подпись, которая не помещается в одну строку"),
        line("e1", "длинное значение в поле ввода"),
        button("b1", "Тоже очень длинная надпись на кнопке")], height=300)
    vg(category, "stretch", "content pushed to the top by a vertical spacer",
       [label("l1", "Вверху"), button("b1", "Кнопка"), spacer(False)])
    vg(category, "group_in_vbox", "group box with controls inside a vbox",
       [group("Параметры", "group",
              [checkbox("c1", "Флажок", True), radio("r1", "Переключатель", True),
               line("e1", "значение")]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "OK")], 6, 0)],
       height=340)
    vg(category, "dense_rows", "six compact rows of label+value",
       [layout("QHBoxLayout", f"row{index}",
               [label(f"k{index}", f"Ключ {index}:"), line(f"v{index}", str(index * 11))],
               6, 2)
        for index in range(6)], height=340)
    vg(category, "empty_label_row", "empty label between fields",
       [label("l_empty", ""), line("e1", "до"), label("l_mid", "между"),
        line("e2", "после")])
    vg(category, "cyrillic_long", "long cyrillic caption and button",
       [label("l1", "Непрерывная русская подпись для проверки метрик шрифта"),
        button("b1", "Выполнить действие")], height=260)

    # ---- gallery/grid: QGridLayout combinations ----
    category = "gallery/grid"
    gr(category, "label_field_2x2", "2x2 label/field grid",
       [grid_item(0, 0, label("l1", "Имя:")),
        grid_item(0, 1, line("e1", "Иван")),
        grid_item(1, 0, label("l2", "Код:")),
        grid_item(1, 1, combo("c1"))], width=360, height=220)
    gr(category, "span_colspan", "colspan over two columns",
       [grid_item(0, 0, label("l1", "Фильтр:")),
        grid_item(0, 1, line("e1"), colspan=2),
        grid_item(1, 0, button("b1", "Найти")),
        grid_item(1, 1, label("l2", "Готово"), colspan=2)], width=420, height=240)
    gr(category, "span_rowspan_align", "rowspan with centred alignment",
       [grid_item(0, 0, button("b1", "Вверх"), rowspan=2,
                  alignment="Qt::AlignHCenter|Qt::AlignVCenter"),
        grid_item(0, 1, table("table", 6, 2)),
        grid_item(1, 1, checkbox("c1", "Галочка")),
        grid_item(2, 0, label("l1", "Статус:")),
        grid_item(2, 1, label("l2", "OK"))], width=440, height=300)
    gr(category, "dense_4x4", "16-cell mixed grid",
       [grid_item(row, column,
                  label(f"l{row}{column}", f"{row},{column}")
                  if (row + column) % 2 == 0
                  else line(f"e{row}{column}"))
        for row in range(4) for column in range(4)], width=440, height=300)
    gr(category, "sparse_empty", "grid with intentionally empty cells",
       [grid_item(0, 0, label("l1", "0,0")),
        grid_item(0, 3, label("l2", "0,3")),
        grid_item(3, 0, label("l3", "3,0")),
        grid_item(3, 3, button("b1", "3,3"))], width=400, height=280)
    gr(category, "nested_hbox_cell", "hbox layout inside a grid cell",
       [grid_item(0, 0, label("l1", "Строка:")),
        grid_item(0, 1, layout("QHBoxLayout", "cellRow",
                               [line("e1", "значение"), button("b1", "…")], 6, 0),
                  colspan=2),
        grid_item(1, 0, label("l2", "Ёщё:")),
        grid_item(1, 1, line("e2"), colspan=2)], width=420, height=240)
    gr(category, "policy_cells", "Fixed vs Expanding cells",
       [grid_item(0, 0, widget("QPushButton", "fixed",
                               prop_size_policy("Fixed", "Fixed") + prop_text("text", "Fixed"))),
        grid_item(0, 1, line("e1")),
        grid_item(1, 0, widget("QLabel", "grow",
                               prop_size_policy("Expanding", "Preferred")
                               + prop_text("text", "Status:"))),
        grid_item(1, 1, label("l1", "ready"))], width=440, height=240)
    gr(category, "table_span", "table spanning two columns",
       [grid_item(0, 0, label("l1", "Таблица:")),
        grid_item(1, 0, table("table", 8, 3), colspan=2),
        grid_item(2, 0, button("b1", "Добавить")),
        grid_item(2, 1, button("b2", "Удалить"))], width=500, height=340)
    gr(category, "combo_check_grid", "combo and checkbox rows",
       [grid_item(0, 0, label("l1", "Режим:")),
        grid_item(0, 1, combo("c1")),
        grid_item(1, 0, label("l2", "Флаги:")),
        grid_item(1, 1, checkbox("c2", "Использовать", True)),
        grid_item(2, 0, label("l3", "Повтор:")),
        grid_item(2, 1, radio("r1", "Да", True))], width=420, height=260)
    gr(category, "long_text_cell", "long label cell versus short fields",
       [grid_item(0, 0, label("l1", "Подпись, которая существенно длиннее соседних полей")),
        grid_item(0, 1, line("e1", "x")),
        grid_item(1, 0, label("l2", "Короткая:")),
        grid_item(1, 1, line("e2", "y"))], width=480, height=220)
    gr(category, "spacers_grid", "spacers inside grid cells",
       [grid_item(0, 0, button("b1", "Слева")),
        grid_item(0, 1, spacer(True), colspan=2),
        grid_item(1, 0, spacer(False), rowspan=2,
                  alignment="Qt::AlignVCenter"),
        grid_item(1, 1, label("l1", "Справа")),
        grid_item(2, 2, button("b2", "Угол"))], width=440, height=260)
    gr(category, "mixed_widgets", "every supported class in one grid",
       [grid_item(0, 0, label("l1", "Метка")),
        grid_item(0, 1, line("e1", "поле")),
        grid_item(1, 0, checkbox("c1", "Флажок", True)),
        grid_item(1, 1, radio("r1", "Радио", True)),
        grid_item(2, 0, combo("c2")),
        grid_item(2, 1, button("b1", "Кнопка")),
        grid_item(3, 0, table("table", 3, 2), colspan=2)], width=460, height=360)
    gr(category, "wide_table_cell", "wide table in a spanned cell",
       [grid_item(0, 0, label("l1", "Таблица"), colspan=3),
        grid_item(1, 0, table("table", 2, 12), colspan=3)], width=520, height=240)
    gr(category, "vertical_stack_cell", "vbox inside a grid cell",
       [grid_item(0, 0, label("l1", "Стек:")),
        grid_item(0, 1, layout("QVBoxLayout", "cellStack",
                               [line("e1"), line("e2"), button("b1", "Ок")], 4, 0)),
        grid_item(1, 0, label("l2", "Итог:")),
        grid_item(1, 1, label("l3", "2 значения"))], width=420, height=280)

    # ---- gallery/form: QFormLayout policies ----
    category = "gallery/form"
    fl(category, "basic", "plain label/field pairs",
       [grid_item(0, 0, label("l1", "Имя:")), grid_item(0, 1, line("e1", "Иван")),
        grid_item(1, 0, label("l2", "Код:")), grid_item(1, 1, combo("c1"))],
       width=360, height=200)
    fl(category, "wrap_long", "WrapLongRows for a long label",
       [grid_item(0, 0, label("l1", "Очень длинная подпись, требующая переноса:")),
        grid_item(0, 1, line("e1")),
        grid_item(1, 0, label("l2", "Короткая:")),
        grid_item(1, 1, line("e2"))],
       props=form_prop_enum("rowWrapPolicy", "WrapLongRows"), width=320, height=240)
    fl(category, "wrap_all", "WrapAllRows puts fields on their own line",
       [grid_item(0, 0, label("l1", "Имя:")), grid_item(0, 1, line("e1")),
        grid_item(1, 0, label("l2", "Описание:")), grid_item(1, 1, line("e2")),
        grid_item(2, 0, label("l3", "Флаг:")), grid_item(2, 1, checkbox("c1", "Да"))],
       props=form_prop_enum("rowWrapPolicy", "WrapAllRows"), width=340, height=300)
    fl(category, "stay_size_hint", "FieldsStayAtSizeHint keeps fields narrow",
       [grid_item(0, 0, label("l1", "Имя:")), grid_item(0, 1, line("e1")),
        grid_item(1, 0, label("l2", "Статус:")),
        grid_item(1, 1, label("l3", "готов"))],
       props=form_prop_enum("fieldGrowthPolicy", "FieldsStayAtSizeHint"), width=380, height=200)
    fl(category, "align_right", "right-aligned labels and centred form",
       [grid_item(0, 0, label("l1", "Имя:")), grid_item(0, 1, line("e1")),
        grid_item(1, 0, label("l2", "Код:")), grid_item(1, 1, combo("c1"))],
       props=(form_prop_enum("fieldGrowthPolicy", "FieldsStayAtSizeHint")
              + form_prop_set("labelAlignment", "Qt::AlignRight|Qt::AlignVCenter")
              + form_prop_set("formAlignment", "Qt::AlignHCenter|Qt::AlignTop")
              + '<property name="horizontalSpacing"><number>12</number></property>'
              + '<property name="verticalSpacing"><number>9</number></property>'),
       width=420, height=220)
    fl(category, "in_group", "form layout inside a group box",
       [grid_item(0, 0, label("l1", "Имя:")), grid_item(0, 1, line("e1")),
        grid_item(1, 0, label("l2", "Пароль:")), grid_item(1, 1, line("e2"))],
       width=360, height=200)
    fl(category, "combo_check_rows", "combo and check rows",
       [grid_item(0, 0, label("l1", "Режим:")), grid_item(0, 1, combo("c1")),
        grid_item(1, 0, label("l2", "Опции:")),
        grid_item(1, 1, checkbox("c2", "Запомнить", True))], width=380, height=200)
    fl(category, "long_labels", "long labels vs fixed field column",
       [grid_item(0, 0, label("l1", "Непрерывно длинная подпись слева")),
        grid_item(0, 1, line("e1", "a")),
        grid_item(1, 0, label("l2", "B:")),
        grid_item(1, 1, line("e2", "b"))], width=420, height=220)
    fl(category, "disabled_field", "disabled field among enabled rows",
       [grid_item(0, 0, label("l1", "Живое:")), grid_item(0, 1, line("e1", "ok")),
        grid_item(1, 0, label("l2", "Мёртвое:")),
        grid_item(1, 1, widget("QLineEdit", "e2",
                               prop_text("text", "нет") + prop_bool("enabled", False)))],
       width=380, height=200)
    fl(category, "fixed_field", "fixed-width field with a minimum",
       [grid_item(0, 0, label("l1", "Код:")),
        grid_item(0, 1, widget("QLineEdit", "e1",
                               prop_size_policy("Fixed", "Fixed")
                               + '<property name="minimumSize"><size><width>160</width><height>0</height></size></property>')),
        grid_item(1, 0, label("l2", "Значение:")), grid_item(1, 1, line("e2"))],
       width=400, height=200)

    # ---- gallery/nested: deep layout nesting ----
    category = "gallery/nested"
    vg(category, "vbox_grid_hbox", "vbox > grid > hbox three levels",
       [label("l1", "Верх"),
        grid_layout("innerGrid",
                    [grid_item(0, 0, label("l2", "Ключ:")),
                     grid_item(0, 1, line("e1", "значение")),
                     grid_item(1, 0, label("l3", "Ёщё:")),
                     grid_item(1, 1, combo("c1"))]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "OK"),
                                           button("cancel", "Отмена")], 6, 0)],
       height=340)
    vg(category, "group_vbox_group_hbox", "group > vbox > group > hbox",
       [group("Внешняя", "g1",
              [label("l1", "внутри внешней"),
               group("Вложенная", "g2",
                     [line("e1", "глубоко"),
                      layout("QHBoxLayout", "row",
                             [button("b1", "A"), button("b2", "B")], 6, 0)])]),
        spacer(False)], height=380)
    vg(category, "grid_group_form", "grid root wrapping a group with a form layout",
       [group("Свойства", "g1",
              [form_layout("formLayout",
                           [grid_item(0, 0, label("l1", "Имя:")),
                            grid_item(0, 1, line("e1")),
                            grid_item(1, 0, label("l2", "Тип:")),
                            grid_item(1, 1, combo("c1"))])]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "OK")], 6, 0)],
       height=340)
    vg(category, "hbox_vbox_grid", "hbox of vbox and grid columns",
       [layout("QHBoxLayout", "split",
               [layout("QVBoxLayout", "left",
                       [label("l1", "Лево"), line("e1"), button("b1", "←")], 4, 0),
                layout("QVBoxLayout", "right",
                       [label("l2", "Право"),
                        grid_layout("g",
                                    [grid_item(0, 0, checkbox("c1", "1")),
                                     grid_item(0, 1, checkbox("c2", "2")),
                                     grid_item(1, 0, checkbox("c3", "3")),
                                     grid_item(1, 1, checkbox("c4", "4"))]),
                        button("b2", "→")], 4, 0)], 8, 0)],
       width=520, height=300)
    vg(category, "three_levels_table", "vbox > hbox > vbox around a table",
       [layout("QHBoxLayout", "toolbarRow",
               [label("l1", "Фильтр:"), line("e1"), button("b1", "Применить")], 6, 0),
        table("table", 10, 3),
        layout("QHBoxLayout", "statusRow", [label("l2", "Строк: 10"),
                                             spacer(True), button("b2", "Закрыть")], 6, 0)],
       width=520, height=400)
    vg(category, "group_in_group_in_group", "three nested group boxes",
       [group("Уровень 1", "g1",
              [group("Уровень 2", "g2",
                     [group("Уровень 3", "g3",
                            [label("l1", "Глубоко внутри"), line("e1", "value")])])]),
        spacer(False)], height=420)
    vg(category, "vbox_hbox_vbox_btns", "alternating box directions",
       [layout("QHBoxLayout", "top", [label("l1", "Заголовок"), spacer(True),
                                       button("settings", "⚙")], 6, 0),
        layout("QVBoxLayout", "body",
               [line("e1"), layout("QHBoxLayout", "mid", [button("b1", "Ок"),
                                                           button("b2", "Ещё")], 6, 0),
                label("l2", "подвал")], 4, 0)], height=300)
    vg(category, "grid_hbox_spacers", "grid cells filled with boxes and spacers",
       [grid_layout("g",
                    [grid_item(0, 0, layout("QHBoxLayout", "r1",
                                           [button("b1", "1"), spacer(True)], 6, 0),
                               colspan=2),
                     grid_item(1, 0, label("l1", "Ячейка")),
                     grid_item(1, 1, line("e1")),
                     grid_item(2, 0, spacer(False)),
                     grid_item(2, 1, button("b2", "Готово"))]),
        spacer(False)], height=320)
    vg(category, "form_in_vbox_in_group", "form layout nested through vbox and group",
       [group("Данные", "g1",
              [layout("QVBoxLayout", "stack",
                      [form_layout("formLayout",
                                   [grid_item(0, 0, label("l1", "Поле 1:")),
                                    grid_item(0, 1, line("e1")),
                                    grid_item(1, 0, label("l2", "Поле 2:")),
                                    grid_item(1, 1, combo("c1"))]),
                       checkbox("c2", "Сохранить", True)], 6, 0)]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "OK")], 6, 0)],
       height=360)
    vg(category, "deep_mixed", "four levels with line, check and table",
       [group("Блок", "g1",
              [layout("QHBoxLayout", "row",
                      [label("l1", "Имя:"), line("e1", "текст"),
                       checkbox("c1", "ок", True)], 6, 0),
               table("table", 6, 2),
               layout("QHBoxLayout", "buttons", [spacer(True), button("b1", "Применить"),
                                                  button("b2", "Сброс")], 6, 0)])],
       width=520, height=380)

    # ---- gallery/policies: size policies and constraints ----
    category = "gallery/policies"
    vg(category, "fixed_btn_expanding_line", "fixed button + expanding line",
       [layout("QHBoxLayout", "row",
               [widget("QPushButton", "b1",
                       prop_size_policy("Fixed", "Fixed") + prop_text("text", "Fixed")),
                widget("QLineEdit", "e1",
                       prop_size_policy("Expanding", "Fixed") + prop_text("text", "Expanding"))],
               6, 0)])
    vg(category, "expanding_label", "expanding label in a row",
       [layout("QHBoxLayout", "row",
               [widget("QLabel", "l1",
                       prop_size_policy("Expanding", "Preferred")
                       + prop_text("text", "Растягиваемая подпись")),
                button("b1", "OK")], 6, 0)])
    vg(category, "minimum_size_field", "minimumSize larger than the hint",
       [layout("QHBoxLayout", "row",
               [label("l1", "Минимум:"),
                widget("QLineEdit", "e1",
                       '<property name="minimumSize"><size><width>220</width><height>0</height></size></property>'
                       + prop_text("text", "220px"))], 6, 0)], width=520)
    vg(category, "maximum_size_label", "maximumSize caps a long label",
       [layout("QHBoxLayout", "row",
               [widget("QLabel", "l1",
                       '<property name="maximumSize"><size><width>120</width><height>0</height></size></property>'
                       + prop_text("text", "Очень длинная подпись с ограничением ширины")),
                button("b1", "OK")], 6, 0)], width=480)
    gr(category, "fixed_vs_expanding_grid", "policy contrast inside a grid",
       [grid_item(0, 0, widget("QPushButton", "b1",
                               prop_size_policy("Fixed", "Fixed") + prop_text("text", "Fixed"))),
        grid_item(0, 1, widget("QLineEdit", "e1",
                               prop_size_policy("Expanding", "Expanding"))),
        grid_item(1, 0, widget("QLabel", "l1",
                               prop_size_policy("Minimum", "Preferred") + prop_text("text", "Minimum"))),
        grid_item(1, 1, widget("QLineEdit", "e2",
                               prop_size_policy("Preferred", "Preferred")))],
       width=440, height=220)
    vg(category, "preferred_everything", "all widgets preferred (baseline)",
       [layout("QHBoxLayout", "row",
               [label("l1", "Preferred"), line("e1"), button("b1", "Btn")], 6, 0)])
    gr(category, "expanding_btn_grid", "expanding button stretches in the grid",
       [grid_item(0, 0, widget("QPushButton", "b1",
                               prop_size_policy("Expanding", "Preferred")
                               + prop_text("text", "Растянуть")), colspan=2),
        grid_item(1, 0, label("l1", "Ряд:")),
        grid_item(1, 1, line("e1"))], width=420, height=220)
    gr(category, "fixed_width_label_grid", "fixed-width label column",
       [grid_item(0, 0, widget("QLabel", "l1",
                               prop_size_policy("Fixed", "Fixed")
                               + prop_text("text", "Фикс:"))),
        grid_item(0, 1, line("e1")),
        grid_item(1, 0, widget("QLabel", "l2",
                               prop_size_policy("Fixed", "Fixed")
                               + prop_text("text", "Ещё:"))),
        grid_item(1, 1, line("e2"))], width=400, height=200)
    vg(category, "line_fixed_btn_expanding", "opposite pairing: fixed line, expanding button",
       [layout("QHBoxLayout", "row",
               [widget("QLineEdit", "e1",
                       prop_size_policy("Fixed", "Fixed") + prop_text("text", "fixed")),
                widget("QPushButton", "b1",
                       prop_size_policy("Expanding", "Fixed") + prop_text("text", "Expanding"))],
               6, 0)])
    gr(category, "table_expanding_grid", "expanding table cell",
       [grid_item(0, 0, table("table", 6, 3)),
        grid_item(1, 0, button("b1", "Вниз"))], width=500, height=320)

    # ---- gallery/states: enabled/checked/focus states ----
    category = "gallery/states"
    vg(category, "disabled_label", "disabled label",
       [widget("QLabel", "l1", prop_text("text", "Отключённая метка") + prop_bool("enabled", False)),
        label("l2", "Активная метка")])
    vg(category, "disabled_line", "disabled line edit with text",
       [line("e1", "активное"),
        widget("QLineEdit", "e2", prop_text("text", "отключённое") + prop_bool("enabled", False))])
    vg(category, "disabled_button", "disabled button among enabled",
       [button("b1", "Активная"),
        widget("QPushButton", "b2", prop_text("text", "Отключённая") + prop_bool("enabled", False)),
        button("b3", "Тоже активная")])
    vg(category, "disabled_check_radio", "disabled checkbox and radio",
       [checkbox("c1", "Активный флажок", True),
        widget("QCheckBox", "c2",
               prop_text("text", "Отключённый флажок") + prop_bool("enabled", False)),
        radio("r1", "Активный переключатель", True),
        widget("QRadioButton", "r2",
               prop_text("text", "Отключённый переключатель") + prop_bool("enabled", False))],
       height=280)
    vg(category, "disabled_combo", "disabled combo box",
       [combo("c1"),
        widget("QComboBox", "c2", prop_bool("enabled", False))], height=240)
    vg(category, "disabled_table", "disabled table",
       [table("table", 4, 3),
        widget("QTableWidget", "table2",
               prop_int("rowCount", 2) + prop_int("columnCount", 2) + prop_bool("enabled", False))],
       height=340)
    vg(category, "disabled_group", "disabled group box cascades to children",
       [group("Отключённая группа", "g1",
              [checkbox("c1", "Флажок", True), line("e1", "поле"), button("b1", "Кнопка")]),
        button("b2", "Снаружи активна")], height=320)
    vg(category, "checks_pair", "checked and unchecked pair",
       [checkbox("c1", "Включено", True), checkbox("c2", "Выключено", False)])
    vg(category, "radio_trio", "exclusive radio trio",
       [radio("r1", "Первый", True), radio("r2", "Второй", False),
        radio("r3", "Третий", False)], height=240)
    vg(category, "default_button", "default button gets the initial focus ring",
       [line("e1", "поле ввода"),
        layout("QHBoxLayout", "buttons",
               [spacer(True), button_state("ok", "OK", default=True),
                button("cancel", "Отмена")], 6, 0)])
    vg(category, "toggle_checked", "checkable buttons with checked state",
       [layout("QHBoxLayout", "row",
               [button_state("t1", "Вкл", checked=True),
                button_state("t2", "Выкл", checked=False)], 6, 0)])
    vg(category, "all_disabled", "whole form disabled except the title",
       [widget("QLabel", "l1", prop_text("text", "Тоже отключено") + prop_bool("enabled", False)),
        widget("QLineEdit", "e1", prop_text("text", "нет доступа") + prop_bool("enabled", False)),
        widget("QPushButton", "b1", prop_text("text", "Нельзя") + prop_bool("enabled", False))])

    # ---- gallery/text: text metrics edge cases ----
    category = "gallery/text"
    vg(category, "single_char", "single character labels",
       [layout("QHBoxLayout", "row",
               [label("l1", "x"), label("l2", "Я"), button("b1", "!"),
                label("l3", "—")], 6, 0)])
    vg(category, "empty_label", "empty strings everywhere",
       [label("l1", ""), line("e1", ""), button("b1", ""), label("l2", "после пустых")])
    vg(category, "very_long_wrapping", "wordWrap label with a long sentence",
       [widget("QLabel", "l1",
               prop_text("text", "Это очень длинная подпись, которая переносится по словам, чтобы проверить вертикальные метрики компоновки")
               + prop_bool("wordWrap", True)),
        button("b1", "OK")], width=360, height=240)
    vg(category, "numbers", "numeric content in widgets",
       [label("l1", "3.14159"), line("e1", "0123456789"),
        button("b1", "1000000")])
    vg(category, "punctuation", "punctuation-heavy strings",
       [label("l1", "<> [] {} | \\ / ? ! @ # $ % ^ & * ( )"),
        line("e1", "key=value; flag:true;"), button("b1", "a + b = c")])
    vg(category, "cyrillic_upper", "cyrillic uppercase and mixed case",
       [label("l1", "ЗАГОЛОВОК"), line("e1", "Смешанный Регистр Текста"),
        button("b1", "ВЫПОЛНИТЬ")])
    vg(category, "mixed_ru_en", "mixed russian and english text",
       [label("l1", "Имя файла: report_Q3 (final).xlsx"),
        line("e1", "user@example.com"), button("b1", "Отправить send")])
    vg(category, "spaces", "leading/trailing/double spaces",
       [label("l1", "   отступы   "), line("e1", "двойной  пробел"),
        button("b1", "  кнопка  ")])
    vg(category, "button_long_text", "long button captions",
       [button("b1", "Очень длинная надпись на кнопке, длиннее соседних"),
        button("b2", "Короткая")])
    vg(category, "group_long_title", "long group box title",
       [group("Название группы, существенно длиннее обычного заголовка", "g1",
              [label("l1", "внутри"), line("e1")]),
        spacer(False)], height=300)

    # ---- gallery/tables ----
    category = "gallery/tables"
    for rows, cols, width, height in [
        (1, 1, 320, 200), (2, 2, 360, 220), (4, 3, 460, 280),
        (8, 5, 540, 340), (20, 3, 480, 340), (3, 12, 520, 240),
    ]:
        vg(category, f"size_{rows}x{cols}", f"QTableWidget {rows}x{cols}",
           [table("table", rows, cols)], width=width, height=height)
    vg(category, "narrow_tall", "narrow table with vertical overflow",
       [table("table", 60, 1)], width=300, height=260)
    vg(category, "wide_short", "short table with horizontal overflow",
       [table("table", 2, 20)], width=340, height=220)
    vg(category, "in_grid", "table inside a grid cell",
       [grid_layout("g",
                    [grid_item(0, 0, label("l1", "Данные:")),
                     grid_item(0, 1, button("b1", "Обновить")),
                     grid_item(1, 0, table("table", 8, 3), colspan=2)]),
        spacer(False)], width=520, height=380)
    vg(category, "in_form_filter", "table behind a filter row",
       [row("Фильтр:", "value"), table("table", 12, 4),
        layout("QHBoxLayout", "buttons", [spacer(True), button("b1", "Применить")], 6, 0)],
       width=520, height=420)
    vg(category, "policy_off", "scrollbars forced off (content clips)",
       [table_with_policy("table", 50, 8,
                          "Qt::ScrollBarAlwaysOff", "Qt::ScrollBarAlwaysOff")],
       width=460, height=260)
    vg(category, "policy_always", "scrollbars always on even when unused",
       [table_with_policy("table", 2, 2,
                          "Qt::ScrollBarAlwaysOn", "Qt::ScrollBarAlwaysOn")],
       width=460, height=260)

    # ---- gallery/lists: lists, trees, combos ----
    category = "gallery/lists"
    vg(category, "list_small", "list widget in a small form",
       [list_widget("list")], width=360, height=240)
    vg(category, "list_tall", "tall list",
       [list_widget("list")], width=360, height=420)
    vg(category, "tree_small", "tree widget in a small form",
       [tree_widget("tree")], width=380, height=240)
    vg(category, "tree_overflow", "tree with vertical overflow",
       [tree_widget("tree")], width=380, height=420)
    vg(category, "list_disabled", "disabled list among active widgets",
       [line("e1", "живой"),
        widget("QListWidget", "list", prop_bool("enabled", False))], height=320)
    vg(category, "tree_disabled", "disabled tree",
       [tree_widget("tree"), button("b1", "Добавить")], height=320,
       spacing=4)
    vg(category, "list_in_grid", "list inside a grid cell",
       [grid_layout("g",
                    [grid_item(0, 0, label("l1", "Элементы:")),
                     grid_item(0, 1, button("b1", "+")),
                     grid_item(1, 0, list_widget("list"), rowspan=2),
                     grid_item(1, 1, line("e1")),
                     grid_item(2, 1, checkbox("c1", "Только чтение"))]),
        spacer(False)], width=520, height=380)
    vg(category, "tree_hbox", "tree beside an hbox detail column",
       [layout("QHBoxLayout", "split",
               [tree_widget("tree"),
                layout("QVBoxLayout", "detail",
                       [label("l1", "Свойства"), line("e1"), line("e2"),
                        button("b1", "Изменить")], 4, 0)], 8, 0)],
       width=560, height=380)
    vg(category, "list_wide", "wide list inside a grid",
       [grid_layout("g",
                    [grid_item(0, 0, combo("c1"), colspan=2),
                     grid_item(1, 0, list_widget("list"), colspan=2)]),
        spacer(False)], width=560, height=340)
    vg(category, "combo_in_grid", "combo boxes in a grid column",
       [grid_layout("g",
                    [grid_item(0, 0, label("l1", "Источник:")),
                     grid_item(0, 1, combo("c1")),
                     grid_item(1, 0, label("l2", "Формат:")),
                     grid_item(1, 1, combo("c2")),
                     grid_item(2, 0, label("l3", "Период:")),
                     grid_item(2, 1, combo("c3"))]),
        spacer(False)], width=420, height=280)

    # ---- gallery/groups: group box layouts ----
    category = "gallery/groups"
    vg(category, "single_group", "one group with mixed controls",
       [group("Основное", "g1",
              [label("l1", "Подпись"), line("e1", "значение"),
               checkbox("c1", "Флажок", True), button("b1", "Применить")]),
        spacer(False)], height=340)
    vg(category, "two_groups", "two sibling groups",
       [group("Левая", "g1", [radio("r1", "A", True), radio("r2", "B", False)]),
        group("Правая", "g2", [checkbox("c1", "X", False), checkbox("c2", "Y", True)]),
        spacer(False)], height=360)
    vg(category, "nested_groups", "group inside a group",
       [group("Внешняя", "g1",
              [group("Внутренняя", "g2", [line("e1", "глубоко"), button("b1", "Ок")]),
               label("l1", "рядом")]),
        spacer(False)], height=380)
    vg(category, "group_hbox", "hbox layout inside a group box",
       [group("Панель", "g1",
              [layout("QHBoxLayout", "row",
                      [button("b1", "◀"), button("b2", "▶"), spacer(True),
                       button("b3", "⏹")], 6, 0)]),
        spacer(False)], height=260)
    vg(category, "group_grid", "grid layout inside a group box",
       [group("Сетка", "g1",
              [grid_layout("g",
                           [grid_item(0, 0, label("l1", "A:")),
                            grid_item(0, 1, line("e1")),
                            grid_item(1, 0, label("l2", "B:")),
                            grid_item(1, 1, line("e2"))])]),
        spacer(False)], height=320)
    vg(category, "group_disabled", "disabled sibling group",
       [group("Заблокировано", "g1",
              [line("e1", "нет"), button("b1", "нет")]),
        group("Активно", "g2", [line("e2", "да"), button("b2", "да")]),
        spacer(False)], height=380)
    vg(category, "three_groups", "three compact groups stacked",
       [group("1", "g1", [checkbox("c1", "один", True)]),
        group("2", "g2", [checkbox("c2", "два", False)]),
        group("3", "g3", [checkbox("c3", "три", False)]),
        spacer(False)], height=340)
    vg(category, "group_with_table", "table inside a group box",
       [group("Таблица данных", "g1", [table("table", 6, 3)]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "OK")], 6, 0)],
       width=520, height=400)

    # ---- gallery/mixed: realistic combinations ----
    category = "gallery/mixed"
    vg(category, "login_form", "login dialog shape",
       [label("l1", "Вход в систему"),
        layout("QVBoxLayout", "fields",
               [label("l2", "Логин:"), line("e1", "user"),
                label("l3", "Пароль:"), line("e2", "1234")], 4, 0),
        layout("QHBoxLayout", "buttons",
               [spacer(True), button("ok", "Войти"), button("cancel", "Отмена")], 6, 0)],
       width=380, height=300)
    vg(category, "filter_bar", "filter controls above a table",
       [layout("QHBoxLayout", "filters",
               [label("l1", "Поиск:"), line("e1"), combo("c1"),
                checkbox("c2", "Только активные", True)], 6, 0),
        table("table", 12, 4),
        layout("QHBoxLayout", "buttons",
               [label("l2", "12 строк"), spacer(True),
                button("b1", "Экспорт")], 6, 0)], width=560, height=420)
    vg(category, "properties_grid", "grid of checkboxes like a properties panel",
       [grid_layout("g",
                    [grid_item(row, column,
                               checkbox(f"c{row}{column}", f"Опция {row}.{column}",
                                        (row + column) % 3 == 0))
                     for row in range(3) for column in range(2)]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "Применить")], 6, 0)],
       width=440, height=320)
    vg(category, "button_panel", "two aligned button rows",
       [layout("QHBoxLayout", "row1",
               [button("b1", "Новый"), button("b2", "Открыть"),
                button("b3", "Сохранить"), spacer(True)], 6, 0),
        layout("QHBoxLayout", "row2",
               [button("b4", "Вырезать"), button("b5", "Копировать"),
                button("b6", "Вставить"), spacer(True)], 6, 0),
        spacer(False)], width=520, height=260)
    vg(category, "splitter_h", "horizontal splitter: list and detail",
       [widget("QSplitter", "splitter",
               prop_enum("orientation", "Qt::Orientation::Horizontal"),
               widget("QListWidget", "leftList")
               + widget("QTreeWidget", "rightTree",
                        "<column>" + prop_text("text", "Имя") + "</column>"))],
       width=560, height=380)
    vg(category, "splitter_v", "vertical splitter: list above a table",
       [widget("QSplitter", "splitter",
               prop_enum("orientation", "Qt::Orientation::Vertical"),
               list_widget("topList") + table("bottomTable", 6, 3))],
       width=560, height=380)
    vg(category, "form_with_table", "form rows, table and buttons together",
       [row("Название:", "Отчёт"),
        table("table", 8, 3),
        layout("QHBoxLayout", "buttons",
               [spacer(True), button("save", "Сохранить"),
                button("close", "Закрыть")], 6, 0)], width=540, height=420)
    vg(category, "checks_panel", "panels of toggles with actions",
       [group("Фильтры", "g1",
              [checkbox("c1", "Показывать скрытые", False),
               checkbox("c2", "Группировать", True),
               radio("r1", "По дате", True), radio("r2", "По имени", False)]),
        layout("QHBoxLayout", "buttons", [spacer(True), button("ok", "Обновить")], 6, 0)],
       width=460, height=380)
    vg(category, "nav_plus_detail", "list navigation with a detail column",
       [layout("QHBoxLayout", "split",
               [list_widget("nav"),
                layout("QVBoxLayout", "detail",
                       [label("l1", "Карточка"),
                        label("l2", "Имя:"), line("e1", "Значение"),
                        label("l3", "Тип:"), combo("c1"),
                        spacer(False),
                        layout("QHBoxLayout", "row",
                               [button("b1", "Изменить"),
                                button("b2", "Удалить")], 6, 0)], 4, 0)], 8, 0)],
       width=600, height=400)
    vg(category, "table_and_tree", "table and tree side by side",
       [layout("QHBoxLayout", "split",
               [table("table", 8, 3), tree_widget("tree")], 8, 0),
        layout("QHBoxLayout", "buttons",
               [label("l1", "Строк: 8"), spacer(True),
                button("b1", "Выполнить")], 6, 0)], width=620, height=380)

    category = "gallery/tabs"
    vg(category, "basic_two", "QTabWidget with two pages and cyrillic titles",
       [tab_widget("tabs", [
           tab_page("pageA", "Первая", [label("l1", "Содержимое первой вкладки")]),
           tab_page("pageB", "Вторая", [label("l2", "Содержимое второй вкладки")]),
       ])], width=460, height=280)
    vg(category, "fields_three", "three tabs with form controls",
       [tab_widget("tabs", [
           tab_page("pageGeneral", "Общие", [
               label("l1", "Имя:"), line("e1", "Значение"),
               checkbox("c1", "Активно", True),
           ]),
           tab_page("pageExtra", "Дополнительно", [
               label("l2", "Тип:"), combo("combo1"),
               radio("r1", "Вариант A", True), radio("r2", "Вариант B", False),
           ]),
           tab_page("pageNotes", "Заметки", [
               label("l3", "Комментарий"),
               line("e2", ""),
               button("b1", "Сохранить"),
           ]),
       ], current=1)], width=500, height=320)

    category = "gallery/toolbox"
    vg(category, "basic_two", "QToolBox with two pages and cyrillic labels",
       [toolbox_widget("toolbox", [
           toolbox_page("pageA", "Первая", [label("l1", "Содержимое первой страницы")]),
           toolbox_page("pageB", "Вторая", [label("l2", "Содержимое второй страницы")]),
       ])], width=460, height=280)
    vg(category, "fields_three", "three toolbox pages; middle page selected",
       [toolbox_widget("toolbox", [
           toolbox_page("pageGeneral", "Общие", [
               label("l1", "Содержимое общих настроек"),
               spacer(False),
           ]),
           toolbox_page("pageExtra", "Дополнительно", [
               label("l2", "Содержимое дополнительных настроек"),
               spacer(False),
           ]),
           toolbox_page("pageNotes", "Заметки", [
               label("l3", "Содержимое заметок"),
               spacer(False),
           ]),
       ], current=1)], width=500, height=320)

    category = "gallery/spin"
    vg(category, "int_suffix", "QSpinBox with suffix",
       [label("l1", "Количество:"), spin_box("spin", 3, 0, 20, " шт")],
       width=360, height=140)
    vg(category, "double_prefix", "QDoubleSpinBox with prefix",
       [label("l1", "Сумма:"), double_spin_box("dspin", "12.5", "0", "100", 2, "₽ ")],
       width=360, height=140)

    category = "gallery/slider"
    vg(category, "horizontal", "horizontal QSlider",
       [label("l1", "Volume:"), slider("slider", 40, 0, 100, "Qt::Horizontal")],
       width=360, height=140)
    vg(category, "vertical", "vertical QSlider",
       [label("l1", "Level:"), slider("slider", 70, 0, 100, "Qt::Vertical")],
       width=200, height=280)

    category = "gallery/progress"
    vg(category, "value_0", "QProgressBar value 0",
       [label("l1", "Прогресс:"), progress_bar("bar", 0, 0, 100)],
       width=360, height=140)
    vg(category, "value_50", "QProgressBar value 50",
       [label("l1", "Прогресс:"), progress_bar("bar", 50, 0, 100)],
       width=360, height=140)
    vg(category, "value_100", "QProgressBar value 100",
       [label("l1", "Прогресс:"), progress_bar("bar", 100, 0, 100)],
       width=360, height=140)


    category = "gallery/textedit"
    vg(category, "multiline_static", "QPlainTextEdit with wrapped static text",
       [label("l1", "Заметки:"),
        plain_text_edit("edit", "Первая строка\nВторая строка с длинным текстом для переноса.\nТретья.")],
       width=420, height=240)
    vg(category, "empty_focus_sized", "empty QPlainTextEdit at default size hint",
       [label("l1", "Комментарий:"), plain_text_edit("edit", "")],
       width=420, height=240)

    category = "gallery/qss"
    # Demonstrates Qt Style Sheets image contract (stylesheet-reference):
    # - border-image: 9-slice; needs border-width; edges stretch|repeat
    # - background-image: intrinsic size; background-repeat / background-position
    # Paths: client/qss → assets/qss (Qt); wasm uses embedded rqt::assets.
    qss_border_image = """
QDialog { background-color: #eceff4; }
QLabel#title { color: #2e3440; }
QLabel#framed {
  background-color: #eceff4;
  border-width: 52px;
  border-style: solid;
  border-color: transparent;
  /* user-style ornate 9-slice; transparent center (no fill image) */
  border-image: url(qss/border-frame.png) 113 113 113 113 stretch stretch;
  padding: 10px;
  color: #2e3440;
}
QLabel#tiled {
  color: #ffffff;
  padding: 8px;
  background: #d8dee9 url(qss/brick.png) repeat;
}
QLabel#once {
  color: #2e3440;
  padding: 8px;
  background: #d8dee9 url(qss/brick.png) no-repeat center;
}
"""
    fixed = (
        '<property name="sizePolicy"><sizepolicy hsizetype="Preferred" vsizetype="Fixed">'
        "<horstretch>0</horstretch><verstretch>0</verstretch></sizepolicy></property>"
    )
    vg(
        category,
        "border_image",
        "Qt QSS: border-image 9-slice + background-image repeat/no-repeat",
        [
            label("title", "Qt QSS images"),
            widget(
                "QLabel",
                "framed",
                prop_text("text", "border-image (ornate frame, empty center)")
                + fixed
                + '<property name="minimumSize"><size><width>0</width><height>140</height></size></property>'
                + '<property name="maximumSize"><size><width>16777215</width><height>140</height></size></property>'
                + '<property name="alignment"><set>Qt::AlignCenter</set></property>',
            ),
            widget(
                "QLabel",
                "tiled",
                prop_text("text", "background-image repeat (brick)")
                + fixed
                + '<property name="minimumSize"><size><width>0</width><height>64</height></size></property>'
                + '<property name="maximumSize"><size><width>16777215</width><height>64</height></size></property>'
                + '<property name="alignment"><set>Qt::AlignCenter</set></property>',
            ),
            widget(
                "QLabel",
                "once",
                prop_text("text", "background-image no-repeat + center")
                + fixed
                + '<property name="minimumSize"><size><width>0</width><height>64</height></size></property>'
                + '<property name="maximumSize"><size><width>16777215</width><height>64</height></size></property>'
                + '<property name="alignment"><set>Qt::AlignCenter</set></property>',
            ),
            spacer(False),
        ],
        width=450,
        height=360,
        style=qss_border_image,
    )

    assert len(cases) >= 100, f"gallery too small: {len(cases)}"
    return cases


def ensure_comments(cases):
    if COMMENTS.exists():
        text = COMMENTS.read_text(encoding="utf-8")
        additions = []
        for case in cases:
            marker = f"<!-- id: {case['id']} -->"
            if marker not in text:
                additions += [
                    f"## {case['id']}",
                    "",
                    marker,
                    "",
                    "Комментарий:",
                    "",
                ]
        if additions:
            text = text.rstrip() + "\n\n" + "\n".join(additions) + "\n"
            COMMENTS.write_text(text, encoding="utf-8")
        return
    lines = ["# Review comments", "", "Пиши замечания под нужной формой (`id` стабилен между прогонами).", ""]
    for case in cases:
        lines += [f"## {case['id']}", "", f"<!-- id: {case['id']} -->", "", "Комментарий:", ""]
    COMMENTS.write_text("\n".join(lines), encoding="utf-8")


def run(cmd, cwd=None, env=None):
    subprocess.run(cmd, cwd=cwd, env=env, check=True)


def build_tools():
    run(["cargo", "build", "-p", "rqt", "--bin", "rqt_render"], cwd=ROOT)
    if not QT_BIN.exists():
        run(["qmake6", "qt_reference.pro"], cwd=QT_TOOL)
        run(["make", "-j4"], cwd=QT_TOOL)


def render(cases):
    for directory in (QT_PNG, RUST_PNG):
        if directory.exists():
            for file in directory.glob("*.png"):
                file.unlink()
        directory.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["QT_QPA_PLATFORM"] = "offscreen"
    for case in cases:
        path = ROOT / case["path"]
        slug = case["id"].replace("/", "__")
        run([str(RQT_RENDER), str(path), str(RUST_PNG / f"{slug}.png")], cwd=ROOT)
        run(
            [str(QT_BIN), str(path), str(QT_PNG / f"{slug}.png")],
            cwd=ROOT,
            env=env,
        )


def diff_images(cases):
    try:
        from PIL import Image, ImageChops
    except Exception as error:
        print(f"Pillow is required for diff: {error}", file=sys.stderr)
        raise
    metrics = {}
    for case in cases:
        slug = case["id"].replace("/", "__")
        left_path = QT_PNG / f"{slug}.png"
        right_path = RUST_PNG / f"{slug}.png"
        left = Image.open(left_path).convert("RGB")
        right = Image.open(right_path).convert("RGB")
        original_sizes = [list(left.size), list(right.size)]
        if left.size != right.size:
            # Qt and we can differ by a single pixel when a layout minimum
            # rounds text advances differently; crop the overlap and keep
            # the real sizes instead of an unusable 999 diff.
            lw, lh = left.size
            rw, rh = right.size
            if abs(lw - rw) <= 1 and abs(lh - rh) <= 1:
                left = left.crop((0, 0, min(lw, rw), min(lh, rh)))
                right = right.crop((0, 0, min(lw, rw), min(lh, rh)))
            else:
                metrics[case["id"]] = {
                    "mean": 999.0,
                    "max": 255,
                    "over30": 1.0,
                    "sizes": original_sizes,
                }
                continue
        diff = ImageChops.difference(left, right)
        total = 0
        count = 0
        over30 = 0
        max_diff = 0
        for pixel in diff.getdata():
            local = max(pixel)
            max_diff = max(max_diff, local)
            if local > 30:
                over30 += 1
            total += sum(pixel)
            count += 3
        metrics[case["id"]] = {
            "mean": round(total / max(count, 1), 2),
            "max": max_diff,
            "over30": round(over30 / max(left.size[0] * left.size[1], 1), 4),
            "sizes": original_sizes,
        }
    return metrics


def write_manifest(cases):
    manifest = []
    for case in cases:
        entry = {
            "id": case["id"],
            "category": case["category"],
            "title": case["title"],
            "description": case["description"],
            "path": case["path"],
        }
        if "number" in case:
            entry["number"] = case["number"]
        manifest.append(entry)
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")


def read_comments():
    """Parse review/comments.md into {id: text} for the notes column."""
    notes = {}
    if not COMMENTS.exists():
        return notes
    current = None
    collecting = False
    buffer = []
    for line in COMMENTS.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            if current and collecting:
                notes[current] = "\n".join(buffer).strip()
            current = line[3:].strip()
            collecting = False
            buffer = []
        elif line.startswith("<!-- id:"):
            collecting = True
        elif collecting and line.startswith("Комментарий:"):
            continue
        elif collecting:
            buffer.append(line)
    if current and collecting:
        notes[current] = "\n".join(buffer).strip()
    return {key: value for key, value in notes.items() if value}


def write_index(cases, metrics):
    by_category = {}
    for case in cases:
        by_category.setdefault(case["category"], []).append(case)
    notes = read_comments()
    sections = []
    for category, items in by_category.items():
        rows = []
        for case in items:
            slug = case["id"].replace("/", "__")
            metric = metrics[case["id"]]
            number = case.get("number")
            title_cell = f"<code>{esc(case['id'])}</code>"
            if number:
                title_cell = (
                    f"<b>#{number:03d}</b> " + title_cell
                    + f"<br><small>{esc(case['title'])}</small>"
                )
            rows.append(
                "<tr>"
                f'<td data-search="{esc(case["id"] + " " + str(number or ""))}">'
                f"{title_cell}<br><small>{esc(case['description'])}</small></td>"
                f"<td><img src=\"qt/{slug}.png\"></td>"
                f"<td><img src=\"rust/{slug}.png\"></td>"
                f"<td>{metric['mean']}<br><small>max {metric['max']}, &gt;30 {metric['over30']*100:.1f}%</small></td>"
                f"<td class=\"note\">{esc(notes.get(case['id'], ''))}</td>"
                f"<td><a href=\"forms/{esc(case['path'].split('forms/',1)[-1])}\">.ui</a></td>"
                "</tr>"
            )
        anchor = esc(category)
        sections.append(
            f"<h2 id=\"{anchor}\">{esc(category)} ({len(items)})</h2>"
            "<table><thead><tr><th>Form</th><th>Qt6</th><th>Rust</th>"
            "<th>diff</th><th>заметки</th><th></th></tr></thead>"
            f"<tbody>{''.join(rows)}</tbody></table>"
        )
    nav = " · ".join(
        f'<a href="#{esc(category)}">{esc(category)} ({len(items)})</a>'
        for category, items in by_category.items()
    )
    document = f"""<!doctype html>
<html lang="ru"><head><meta charset="utf-8"><title>EPlatform Qt review</title>
<meta http-equiv="cache-control" content="no-cache">
<style>
body{{font-family:system-ui,sans-serif;margin:24px;background:#f4f5f7;color:#1d2129}}
h1{{margin-top:0}} h2{{margin-top:36px}}
table{{border-collapse:collapse;width:100%;background:white;margin-bottom:24px}}
th,td{{border:1px solid #d8dde6;padding:8px;vertical-align:top}}
td img{{max-width:360px;display:block;border:1px solid #d8dde6}}
small{{color:#5b6573}} code{{font-size:12px}}
.note{{white-space:pre-wrap;max-width:220px;background:#fffdf3}}
td.note:empty::after{{content:"—";color:#9aa2ad}}
.header{{background:white;border:1px solid #d8dde6;padding:10px 12px;border-radius:6px;margin:16px 0}}
.note-box{{background:#fff8d8;border:1px solid #e4d38a;padding:12px;border-radius:6px}}
nav{{background:white;border:1px solid #d8dde6;padding:10px 12px;border-radius:6px;margin:16px 0}}
#q{{width:420px;padding:6px 10px;border:1px solid #b9c1cd;border-radius:4px;font-size:14px}}
tr.hidden{{display:none}}
</style></head><body>
<h1>EPlatform Qt review</h1>
<div class="note-box">Комментарии — в <a href="comments.md">review/comments.md</a>
(id формы стабилен между прогонами; сохранённые заметки показаны в колонке «заметки»).
Галерея (<code>gallery/*</code>) — те же формы, что открываются в приложении:
<code>?main=gallery.lua&amp;mainui=</code>, номера <b>#NNN</b> совпадают с заголовками окон.</div>
<p>Форм: {len(cases)}. Diff: mean по каналам 0–255.
<label>Фильтр: <input id="q" type="search" placeholder="id, номер, текст…"></label></p>
<nav>{nav}</nav>
{''.join(sections)}
<script>
document.getElementById('q').addEventListener('input', function () {{
  var needle = this.value.trim().toLowerCase();
  document.querySelectorAll('tbody tr').forEach(function (row) {{
    var hay = (row.getAttribute('data-search') || row.textContent).toLowerCase();
    row.classList.toggle('hidden', needle !== '' && hay.indexOf(needle) === -1);
  }});
}});
</script>
</body></html>
"""
    (REVIEW / "index.html").write_text(document, encoding="utf-8")


def sync_gallery_to_server(cases):
    """Copy generated gallery forms to the server's runtime directory."""
    gallery_cases = [case for case in cases if case["id"].startswith("gallery/")]
    target_root = SERVER_QT_FORMS / "gallery"
    if target_root.exists():
        shutil.rmtree(target_root)
    for case in gallery_cases:
        relative = case["path"].split("forms/", 1)[-1]
        target = SERVER_QT_FORMS / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / case["path"], target)
    return gallery_cases


def write_gallery_index(gallery_cases):
    """Stable ordered index consumed by server/priv/lua_modules/gallery.lua."""
    lines = [
        "-- GENERATED by client/tools/formgen/review.py — do not edit.",
        "-- Порядок и номера форм галереи; драйвер main-сессии: gallery.lua.",
        "return {",
    ]
    for case in gallery_cases:
        ui = case["path"].split("forms/", 1)[-1]
        lines.append(
            "    { number = %d, id = \"%s\", category = \"%s\", ui = \"%s\" },"
            % (case["number"], case["id"], case["category"], ui)
        )
    lines += ["}", ""]
    (SERVER_LUA_MODULES / "gallery_index.lua").write_text(
        "\n".join(lines), encoding="utf-8"
    )


def main():
    forms_only = "--forms-only" in sys.argv
    cases = build_cases()
    for index, case in enumerate(cases):
        case["name"] = f"{index:02d}_{case['name']}"
        case["id"] = f"{case['category']}/{case['name']}"
        case["path"] = f"review/forms/{case['category']}/{case['name']}.ui"
    gallery = build_gallery_cases()
    base = len(cases)
    for offset, case in enumerate(gallery):
        number = offset + 1
        case["name"] = f"{base + offset:02d}_{case['name']}"
        case["id"] = f"{case['category']}/{case['name']}"
        case["path"] = f"review/forms/{case['category']}/{case['name']}.ui"
        case["number"] = number
        # The window title carries the sequential number the driver shows
        # in the status bar: "#007 grid/span_colspan".
        short_id = case["id"][len("gallery/") :]
        case["title"] = f"#{number:03d} {short_id}"
        case["xml"] = case.pop("make_xml")(case["title"])
    cases += gallery
    if FORMS.exists():
        shutil.rmtree(FORMS)
    FORMS.mkdir(parents=True, exist_ok=True)
    for case in cases:
        path = ROOT / case["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(case["xml"], encoding="utf-8")
    ensure_comments(cases)
    write_manifest(cases)
    gallery_cases = sync_gallery_to_server(cases)
    write_gallery_index(gallery_cases)
    if forms_only:
        print(
            f"generated {len(cases)} forms "
            f"({len(gallery_cases)} gallery, forms only — no renders)"
        )
        return
    build_tools()
    render(cases)
    metrics = diff_images(cases)
    write_index(cases, metrics)
    print(f"generated {len(cases)} forms in {REVIEW}")


if __name__ == "__main__":
    main()
