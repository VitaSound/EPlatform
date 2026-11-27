-- ui.lua — declarative Lua DSL → Qt Designer–compatible .ui XML (plan 44 Variant B).
--
-- Build a tree with Ui.dialog / Ui.vbox / Ui.button / … then Ui.to_xml(spec)
-- and pass it to Form:create(..., { ui_xml = ... }) or { ui_spec = ... }.

local Ui = {}

local function is_array(t)
    if type(t) ~= "table" then
        return false
    end
    local n = 0
    for k, _ in pairs(t) do
        if type(k) ~= "number" then
            return false
        end
        n = n + 1
    end
    return n > 0
end

local function copy_opts(opts)
    local out = {}
    if type(opts) ~= "table" then
        return out
    end
    for k, v in pairs(opts) do
        out[k] = v
    end
    return out
end

local function node(kind, class, opts)
    opts = copy_opts(opts)
    opts.__ui = kind
    opts.class = opts.class or class
    return opts
end

local function collect_children(opts)
    local children = {}
    if type(opts) ~= "table" then
        return children
    end
    for i = 1, #opts do
        children[#children + 1] = opts[i]
        opts[i] = nil
    end
    if type(opts.children) == "table" then
        for i = 1, #opts.children do
            children[#children + 1] = opts.children[i]
        end
        opts.children = nil
    end
    if type(opts.layout) == "table" and opts.layout.__ui == "layout" then
        -- keep as layout field
    elseif type(opts.layout) == "table" then
        children[#children + 1] = opts.layout
        opts.layout = nil
    end
    return children
end

function Ui.dialog(opts)
    opts = copy_opts(opts)
    local children = collect_children(opts)
    local n = node("widget", "QDialog", opts)
    n.name = n.name or "Dialog"
    if n.layout == nil then
        for i = 1, #children do
            if children[i].__ui == "layout" then
                n.layout = children[i]
            else
                n.children = n.children or {}
                n.children[#n.children + 1] = children[i]
            end
        end
    end
    return n
end

function Ui.widget(opts)
    opts = copy_opts(opts)
    local class = opts.class or "QWidget"
    local children = collect_children(opts)
    local n = node("widget", class, opts)
    for i = 1, #children do
        if children[i].__ui == "layout" and n.layout == nil then
            n.layout = children[i]
        else
            n.children = n.children or {}
            n.children[#n.children + 1] = children[i]
        end
    end
    return n
end

local function layout(class, opts)
    opts = copy_opts(opts)
    local items = collect_children(opts)
    local n = node("layout", class, opts)
    n.items = items
    return n
end

function Ui.vbox(opts)
    return layout("QVBoxLayout", opts)
end

function Ui.hbox(opts)
    return layout("QHBoxLayout", opts)
end

function Ui.grid(opts)
    return layout("QGridLayout", opts)
end

function Ui.form(opts)
    return layout("QFormLayout", opts)
end

function Ui.spacer(opts)
    opts = copy_opts(opts)
    return node("spacer", "QSpacerItem", opts)
end

local function control(class, opts)
    opts = copy_opts(opts)
    local children = collect_children(opts)
    local n = node("widget", class, opts)
    if #children > 0 then
        for i = 1, #children do
            if children[i].__ui == "layout" and n.layout == nil then
                n.layout = children[i]
            else
                n.children = n.children or {}
                n.children[#n.children + 1] = children[i]
            end
        end
    end
    return n
end

function Ui.label(opts)
    return control("QLabel", opts)
end

function Ui.line_edit(opts)
    return control("QLineEdit", opts)
end

function Ui.button(opts)
    return control("QPushButton", opts)
end

function Ui.check_box(opts)
    return control("QCheckBox", opts)
end

function Ui.radio(opts)
    return control("QRadioButton", opts)
end

function Ui.combo(opts)
    return control("QComboBox", opts)
end

function Ui.spin(opts)
    return control("QSpinBox", opts)
end

function Ui.double_spin(opts)
    return control("QDoubleSpinBox", opts)
end

function Ui.plain_text(opts)
    return control("QPlainTextEdit", opts)
end

function Ui.group(opts)
    return control("QGroupBox", opts)
end

function Ui.progress(opts)
    return control("QProgressBar", opts)
end

function Ui.slider(opts)
    return control("QSlider", opts)
end

function Ui.tool_button(opts)
    return control("QToolButton", opts)
end

function Ui.list(opts)
    return control("QListWidget", opts)
end

function Ui.table(opts)
    return control("QTableWidget", opts)
end

function Ui.tree(opts)
    return control("QTreeWidget", opts)
end

function Ui.tabs(opts)
    return control("QTabWidget", opts)
end

function Ui.splitter(opts)
    return control("QSplitter", opts)
end

function Ui.toolbox(opts)
    return control("QToolBox", opts)
end

-- Alias table for class-style construction: Ui.class("QPushButton", opts)
function Ui.class(class_name, opts)
    return control(class_name, opts)
end

-- ---------- XML serialization ----------

local function xml_escape(s)
    s = tostring(s or "")
    s = s:gsub("&", "&amp;")
    s = s:gsub("<", "&lt;")
    s = s:gsub(">", "&gt;")
    s = s:gsub('"', "&quot;")
    return s
end

local function write_prop(name, value)
    if value == nil then
        return ""
    end
    local t = type(value)
    if t == "boolean" then
        return string.format('<property name="%s"><bool>%s</bool></property>', name, value and "true" or "false")
    end
    if t == "number" then
        if value == math.floor(value) then
            return string.format('<property name="%s"><number>%d</number></property>', name, value)
        end
        return string.format('<property name="%s"><number>%s</number></property>', name, tostring(value))
    end
    if t == "table" then
        if value.x ~= nil and value.y ~= nil and (value.w ~= nil or value.width ~= nil) then
            local w = value.w or value.width or 0
            local h = value.h or value.height or 0
            return string.format(
                '<property name="%s"><rect><x>%d</x><y>%d</y><width>%d</width><height>%d</height></rect></property>',
                name,
                value.x or 0,
                value.y or 0,
                w,
                h
            )
        end
        if value.w ~= nil or value.width ~= nil then
            return string.format(
                '<property name="%s"><size><width>%d</width><height>%d</height></size></property>',
                name,
                value.w or value.width or 0,
                value.h or value.height or 0
            )
        end
        if is_array(value) then
            -- stringlist-ish: emit as a single joined string for simple cases
            local parts = {}
            for i = 1, #value do
                parts[#parts + 1] = tostring(value[i])
            end
            return string.format(
                '<property name="%s"><string>%s</string></property>',
                name,
                xml_escape(table.concat(parts, "\n"))
            )
        end
    end
    return string.format('<property name="%s"><string>%s</string></property>', name, xml_escape(value))
end

local RESERVED = {
    __ui = true,
    class = true,
    name = true,
    layout = true,
    children = true,
    items = true,
    parent = true,
    row = true,
    column = true,
    col = true,
    row_span = true,
    rowspan = true,
    column_span = true,
    colspan = true,
    alignment = true,
    title = true, -- tab page attribute
    label = true, -- toolbox page attribute
    orientation = true, -- spacer
    sizeHint = true,
    width = true,
    height = true,
}

local function write_widget_props(n)
    local chunks = {}
    if n.geometry then
        chunks[#chunks + 1] = write_prop("geometry", n.geometry)
    elseif n.width or n.height then
        chunks[#chunks + 1] = write_prop("geometry", {
            x = n.x or 0,
            y = n.y or 0,
            w = n.width or 400,
            h = n.height or 300,
        })
    end
    if n.windowTitle or n.window_title then
        chunks[#chunks + 1] = write_prop("windowTitle", n.windowTitle or n.window_title)
    end
    if n.text ~= nil then
        chunks[#chunks + 1] = write_prop("text", n.text)
    end
    if n.plainText ~= nil or n.plain_text ~= nil then
        chunks[#chunks + 1] = write_prop("plainText", n.plainText or n.plain_text)
    end
    if n.title ~= nil and n.class == "QGroupBox" then
        chunks[#chunks + 1] = write_prop("title", n.title)
    end
    if n.checked ~= nil then
        chunks[#chunks + 1] = write_prop("checked", n.checked)
    end
    if n.enabled ~= nil then
        chunks[#chunks + 1] = write_prop("enabled", n.enabled)
    end
    if n.visible ~= nil then
        chunks[#chunks + 1] = write_prop("visible", n.visible)
    end
    if n.toolTip ~= nil or n.tooltip ~= nil then
        chunks[#chunks + 1] = write_prop("toolTip", n.toolTip or n.tooltip)
    end
    if n.currentIndex ~= nil or n.current_index ~= nil then
        chunks[#chunks + 1] = write_prop("currentIndex", n.currentIndex or n.current_index)
    end
    if n.value ~= nil then
        chunks[#chunks + 1] = write_prop("value", n.value)
    end
    if n.minimum ~= nil then
        chunks[#chunks + 1] = write_prop("minimum", n.minimum)
    end
    if n.maximum ~= nil then
        chunks[#chunks + 1] = write_prop("maximum", n.maximum)
    end
    if n.orientation ~= nil and n.class == "QSlider" then
        local orient = n.orientation
        if orient == "horizontal" or orient == "Horizontal" then
            orient = "Qt::Orientation::Horizontal"
        elseif orient == "vertical" or orient == "Vertical" then
            orient = "Qt::Orientation::Vertical"
        end
        chunks[#chunks + 1] = string.format(
            '<property name="orientation"><enum>%s</enum></property>',
            xml_escape(orient)
        )
    end
    for k, v in pairs(n) do
        if not RESERVED[k] and type(k) == "string" then
            if not (
                k == "geometry"
                or k == "windowTitle"
                or k == "window_title"
                or k == "text"
                or k == "plainText"
                or k == "plain_text"
                or k == "title"
                or k == "checked"
                or k == "enabled"
                or k == "visible"
                or k == "toolTip"
                or k == "tooltip"
                or k == "currentIndex"
                or k == "current_index"
                or k == "value"
                or k == "minimum"
                or k == "maximum"
                or k == "x"
                or k == "y"
                or k == "width"
                or k == "height"
                or (k == "orientation" and n.class == "QSlider")
            ) then
                chunks[#chunks + 1] = write_prop(k, v)
            end
        end
    end
    return table.concat(chunks)
end

local function item_attrs(n)
    local parts = {}
    local row = n.row
    local col = n.column or n.col
    local rs = n.row_span or n.rowspan
    local cs = n.column_span or n.colspan
    if row ~= nil then
        parts[#parts + 1] = string.format(' row="%d"', row)
    end
    if col ~= nil then
        parts[#parts + 1] = string.format(' column="%d"', col)
    end
    if rs ~= nil then
        parts[#parts + 1] = string.format(' rowspan="%d"', rs)
    end
    if cs ~= nil then
        parts[#parts + 1] = string.format(' colspan="%d"', cs)
    end
    if n.alignment ~= nil and n.alignment ~= 0 then
        parts[#parts + 1] = string.format(' alignment="%d"', n.alignment)
    end
    return table.concat(parts)
end

local write_node

local function write_layout(n)
    local name = n.name or ""
    local body = {}
    if n.spacing ~= nil then
        body[#body + 1] = write_prop("spacing", n.spacing)
    end
    if n.leftMargin or n.topMargin or n.rightMargin or n.bottomMargin then
        if n.leftMargin then
            body[#body + 1] = write_prop("leftMargin", n.leftMargin)
        end
        if n.topMargin then
            body[#body + 1] = write_prop("topMargin", n.topMargin)
        end
        if n.rightMargin then
            body[#body + 1] = write_prop("rightMargin", n.rightMargin)
        end
        if n.bottomMargin then
            body[#body + 1] = write_prop("bottomMargin", n.bottomMargin)
        end
    end
    local items = n.items or {}
    for i = 1, #items do
        local child = items[i]
        local attrs = item_attrs(child)
        body[#body + 1] = "<item" .. attrs .. ">"
        body[#body + 1] = write_node(child)
        body[#body + 1] = "</item>"
    end
    return string.format(
        '<layout class="%s" name="%s">%s</layout>',
        xml_escape(n.class or "QVBoxLayout"),
        xml_escape(name),
        table.concat(body)
    )
end

local function write_spacer(n)
    local name = n.name or "spacer"
    local horizontal = n.orientation == "horizontal"
        or n.orientation == "Horizontal"
        or n.horizontal == true
    local orient = horizontal and "Qt::Orientation::Horizontal" or "Qt::Orientation::Vertical"
    local w = n.width or (n.sizeHint and (n.sizeHint.w or n.sizeHint.width)) or 40
    local h = n.height or (n.sizeHint and (n.sizeHint.h or n.sizeHint.height)) or 20
    return string.format(
        '<spacer name="%s"><property name="orientation"><enum>%s</enum></property>'
            .. '<property name="sizeHint" stdset="0"><size><width>%d</width><height>%d</height></size></property></spacer>',
        xml_escape(name),
        orient,
        w,
        h
    )
end

local function write_widget(n)
    local name = n.name or n.class or "widget"
    local attrs = string.format('class="%s" name="%s"', xml_escape(n.class or "QWidget"), xml_escape(name))
    -- Tab / toolbox page attributes.
    if n.title ~= nil and n.class ~= "QGroupBox" then
        attrs = attrs .. string.format(' title="%s"', xml_escape(n.title))
    end
    if n.label ~= nil then
        attrs = attrs .. string.format(' label="%s"', xml_escape(n.label))
    end
    local body = { write_widget_props(n) }
    if n.layout then
        body[#body + 1] = write_layout(n.layout)
    end
    local children = n.children or {}
    for i = 1, #children do
        body[#body + 1] = write_node(children[i])
    end
    return string.format("<widget %s>%s</widget>", attrs, table.concat(body))
end

write_node = function(n)
    if type(n) ~= "table" then
        return ""
    end
    if n.__ui == "layout" then
        return write_layout(n)
    end
    if n.__ui == "spacer" then
        return write_spacer(n)
    end
    return write_widget(n)
end

function Ui.to_xml(spec)
    if type(spec) == "string" then
        return spec
    end
    local root = spec
    if root.__ui == "layout" then
        root = Ui.dialog({ name = "Dialog", layout = root })
    end
    local class_name = root.name or root.class or "Dialog"
    local geometry = root.geometry
    if not geometry and not root.width and not root.height then
        root.width = root.width or 420
        root.height = root.height or 280
    end
    return string.format(
        '<?xml version="1.0" encoding="UTF-8"?>\n<ui version="4.0">\n <class>%s</class>\n%s\n</ui>\n',
        xml_escape(class_name),
        write_widget(root)
    )
end

return Ui
