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

    assert len(cases) >= 70, len(cases)
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
        if left.size != right.size:
            metrics[case["id"]] = {
                "mean": 999.0,
                "max": 255,
                "over30": 1.0,
                "sizes": [list(left.size), list(right.size)],
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
            "sizes": [list(left.size), list(right.size)],
        }
    return metrics


def write_manifest(cases):
    manifest = []
    for case in cases:
        manifest.append(
            {
                "id": case["id"],
                "category": case["category"],
                "title": case["title"],
                "description": case["description"],
                "path": case["path"],
            }
        )
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")


def write_index(cases, metrics):
    by_category = {}
    for case in cases:
        by_category.setdefault(case["category"], []).append(case)
    sections = []
    for category, items in by_category.items():
        rows = []
        for case in items:
            slug = case["id"].replace("/", "__")
            metric = metrics[case["id"]]
            rows.append(
                "<tr>"
                f"<td><code>{esc(case['id'])}</code><br><small>{esc(case['description'])}</small></td>"
                f"<td><img src=\"qt/{slug}.png\"></td>"
                f"<td><img src=\"rust/{slug}.png\"></td>"
                f"<td>{metric['mean']}<br><small>max {metric['max']}, &gt;30 {metric['over30']*100:.1f}%</small></td>"
                f"<td><a href=\"forms/{esc(case['path'].split('forms/',1)[-1])}\">.ui</a></td>"
                "</tr>"
            )
        anchor = esc(category)
        sections.append(
            f"<h2 id=\"{anchor}\">{esc(category)} ({len(items)})</h2>"
            "<table><thead><tr><th>Form</th><th>Qt6</th><th>Rust</th><th>diff</th><th></th></tr></thead>"
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
.note{{background:#fff8d8;border:1px solid #e4d38a;padding:12px;border-radius:6px}}
nav{{background:white;border:1px solid #d8dde6;padding:10px 12px;border-radius:6px;margin:16px 0}}
</style></head><body>
<h1>EPlatform Qt review</h1>
<p class="note">Комментарии: <a href="comments.md">review/comments.md</a>. ID формы стабилен между прогонами.
Категории <code>scroll</code> и <code>selection</code> — в конце страницы (быстрый переход в навигации ниже).</p>
<p>Форм: {len(cases)}. Diff: mean по каналам 0–255.</p>
<nav>{nav}</nav>
{''.join(sections)}
</body></html>
"""
    (REVIEW / "index.html").write_text(document, encoding="utf-8")


def main():
    cases = build_cases()
    for index, case in enumerate(cases):
        case["name"] = f"{index:02d}_{case['name']}"
        case["id"] = f"{case['category']}/{case['name']}"
        case["path"] = f"review/forms/{case['category']}/{case['name']}.ui"
    if FORMS.exists():
        shutil.rmtree(FORMS)
    FORMS.mkdir(parents=True, exist_ok=True)
    for case in cases:
        path = ROOT / case["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(case["xml"], encoding="utf-8")
    ensure_comments(cases)
    write_manifest(cases)
    build_tools()
    render(cases)
    metrics = diff_images(cases)
    write_index(cases, metrics)
    print(f"generated {len(cases)} forms in {REVIEW}")


if __name__ == "__main__":
    main()
