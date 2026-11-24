-- form.lua
-- модуль методов для работы с формой

local Form = {}
Form.__index = Form

-- Конструктор
function Form.new()
    local self = setmetatable({}, Form)
    self.application_id = nil
    self.form_id = nil
    self.ui_file = nil
    self.mdi = true
    self.modal = false
    self.open_opts = {}

    -- Храним обработчики событий
    self.event_handlers = {}

    return self
end

-- Создаём форму на GUI
function Form:create(application_id, form_id, ui_file_or_opts)
    local opts = {}
    local ui_file = ui_file_or_opts

    if type(ui_file_or_opts) == "table" then
        opts = ui_file_or_opts
        ui_file = opts.ui_file
    end

    self.application_id = application_id
    self.form_id = form_id
    self.ui_file = ui_file

    if opts.mdi ~= nil then
        self.mdi = opts.mdi and true or false
    end
    if opts.modal ~= nil then
        self.modal = opts.modal and true or false
    end
    if opts.open_opts ~= nil then
        self.open_opts = opts.open_opts
    end

    create_form(self.application_id, self.form_id, self.ui_file)
end

-- Закрываем форму
function Form:close()
    close_form(self.application_id, self.form_id)
end

-- Открываем модальную форму и ждём её результат.
-- Вызов блокирует текущий Lua-процесс, но дочерняя форма живёт в своём
-- GenServer, поэтому вложенные модалки не приводят к дедлоку.
function Form:open_modal_lua(lua_file, opts)
    opts = opts or {}
    return open_modal_lua(self.application_id, self.form_id, lua_file, opts)
end

-- Закрываем модальную форму и возвращаем результат родителю.
function Form:finish_modal(result)
    close_modal(self.application_id, self.form_id, result)
end

-- Установка свойства виджета
function Form:setProperty(object_name, property, value)
    set_property(self.application_id, self.form_id, object_name, property, value)
end

-- Установка свойства формы (например, geometry, windowTitle)
function Form:setFormProperty(property, value)
    set_form_property(self.application_id, self.form_id, property, value)
end

-- Получение свойства виджета
function Form:getProperty(object_name, property)
    return get_property(self.application_id, self.form_id, object_name, property)
end

-- Получение свойства формы
function Form:getFormProperty(property)
    return get_form_property(self.application_id, self.form_id, property)
end

-- Сахар: 2 аргумента — свойство формы, 3 аргумента — свойство виджета.
function Form:set_prop(a, b, c)
    if c ~= nil then
        set_property(self.application_id, self.form_id, a, b, c)
    else
        set_form_property(self.application_id, self.form_id, a, b)
    end
end

function Form:set(a, b, c)
    return self:set_prop(a, b, c)
end

function Form:get_prop(a, b)
    if b ~= nil then
        return get_property(self.application_id, self.form_id, a, b)
    end
    return get_form_property(self.application_id, self.form_id, a)
end

function Form:get(a, b)
    return self:get_prop(a, b)
end

function Form:is_modal()
    return self.modal
end

function Form:is_mdi()
    return self.mdi
end

function Form:setWindowTitle(title)
    self:set_prop("windowTitle", title or "")
end

function Form:setGeometry(geometry)
    self:set_prop("geometry", geometry or {})
end

function Form:setText(object_name, text)
    self:set_prop(object_name, "text", tostring(text or ""))
end

function Form:setVisible(object_name, visible)
    self:set_prop(object_name, "visible", visible and true or false)
end

function Form:setEnabled(object_name, enabled)
    self:set_prop(object_name, "enabled", enabled and true or false)
end

function Form:setChecked(object_name, checked)
    self:set_prop(object_name, "checked", checked and true or false)
end

function Form:setToolTip(object_name, tooltip)
    self:set_prop(object_name, "toolTip", tostring(tooltip or ""))
end

function Form:setIcon(object_name, icon)
    self:set_prop(object_name, "icon", icon or "")
end

function Form:setToolButton(object_name, opts)
    local o = opts or {}
    if o.text ~= nil then
        self:setText(object_name, o.text)
    end
    if o.icon ~= nil then
        self:setIcon(object_name, o.icon)
    end
    if o.icon_size ~= nil then
        self:set_prop(object_name, "iconSize", o.icon_size)
    end
    if o.style ~= nil then
        self:set_prop(object_name, "toolButtonStyle", o.style)
    end
    if o.popup_mode ~= nil then
        self:set_prop(object_name, "popupMode", o.popup_mode)
    end
    if o.arrow_type ~= nil then
        self:set_prop(object_name, "arrowType", o.arrow_type)
    end
    if o.menu_items ~= nil then
        self:set_prop(object_name, "menuItems", o.menu_items)
    end
    if o.tool_tip ~= nil then
        self:setToolTip(object_name, o.tool_tip)
    elseif o.tooltip ~= nil then
        self:setToolTip(object_name, o.tooltip)
    end
end

function Form:setCurrentRow(object_name, row)
    self:set_prop(object_name, "currentRow", row)
end

function Form:setCurrentIndex(object_name, index)
    self:set_prop(object_name, "currentIndex", index)
end

function Form:set_table_column_hidden(object_name, column_index, hidden)
    self:set_prop(object_name, "hideColumn", {
        index = column_index,
        hidden = hidden ~= false
    })
end

function Form:set_table(object_name, headers, rows)
    headers = headers or {}
    rows = rows or {}

    local data = {}
    for row_index = 1, #rows do
        local source_row = rows[row_index] or {}
        local row = {}
        for column_index = 1, #headers do
            row[column_index] = tostring(source_row[column_index] or "")
        end
        data[row_index] = row
    end

    self:set_prop(object_name, "clear", true)
    self:set_prop(object_name, "columnCount", #headers)
    self:set_prop(object_name, "horizontalHeaderLabels", headers)
    self:set_prop(object_name, "rowCount", #rows)
    self:set_prop(object_name, "data", data)
end

function Form:set_table_from_objects(object_name, headers, field_order, objects)
    headers = headers or {}
    field_order = field_order or {}
    objects = objects or {}

    local rows = {}
    for row_index = 1, #objects do
        local source = objects[row_index] or {}
        local row = {}
        for column_index = 1, #field_order do
            row[column_index] = source[field_order[column_index]]
        end
        rows[row_index] = row
    end

    self:set_table(object_name, headers, rows)
end

function Form:setTableHeaders(object_name, payload)
    self:set_prop(object_name, "tableHeaders", payload or {})
end

function Form:setTableData(object_name, payload)
    self:set_prop(object_name, "tableData", payload or {})
end

function Form:setTableDataAndRow(object_name, payload, row)
    self:setTableData(object_name, payload)
    self:setCurrentRow(object_name, row)
end

function Form:set_table_with_headers(object_name, headers, payload)
    self:setTableHeaders(object_name, headers)
    self:setTableData(object_name, payload)
end

-- Регистрация обработчика события
function Form:on(widget_name, event_type, callback)
    local key = widget_name .. "|" .. event_type
    self.event_handlers[key] = callback
end

-- Внутренний метод: вызывается из основного onEvent
function Form:handleEvent(widget_name, event_type)
    local key = widget_name .. "|" .. event_type
    local handler = self.event_handlers[key]

    if not handler and event_type == "Clicked" then
        handler = self.event_handlers[widget_name .. "|MouseButtonRelease"]
    end

    if handler then
        handler()
    end
end

-- Синтаксический сахар: return конструктор
return Form.new
