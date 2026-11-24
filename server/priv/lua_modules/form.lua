-- form.lua
-- модуль методов для работы с формой

local Form = {}
Form.__index = Form

-- Конструктор
function Form.new()
    local self = setmetatable({}, Form)
    self.application_id = null
    self.form_id = null
    self.ui_file = null

    -- Храним обработчики событий
    self.event_handlers = {}

    return self
end

-- Создаём форму на GUI
function Form:create(application_id, form_id, ui_file)
    self.application_id = application_id
    self.form_id = form_id
    self.ui_file = ui_file

    print("Form:create application_id", self.application_id)
    print("Form:create form_id", self.form_id)
    print("Form:create ui_file", self.ui_file)
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
function Form:getProperty(object_name, property, value)
    value = get_property(self.application_id, self.form_id, object_name, property)
    return value
end

-- Получение свойства формы
function Form:getFormProperty(property)
    return get_form_property(self.application_id, self.form_id, property)
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
    if handler then
        handler()
    end
end

-- Синтаксический сахар: return конструктор
return Form.new
