local counter = 0
-- local mathlib = require("mathlib")
-- print(mathlib.add(2, 3)) -- вызовет функцию из библиотеки

local form = require("form")()

function init(app_id, frm_id, form_file)
    -- запускаем форму на клиенте
    form:create(app_id, frm_id, form_file)

    form:setFormProperty("windowTitle", "Тестовая форма")
    -- form:setFormProperty("pos", {x = 10, y = 10})
    -- form:setFormProperty("size", {w = 200, h = 400})
    form:setFormProperty("geometry", {x = 100, y = 100, w = 400, h = 400})

    form:setProperty("label_1", "text", "A=")
    form:setProperty("label_2", "text", "B=")
    form:setProperty("label_3", "text", "A*B=")

    form:setProperty("lineEdit_1", "text", "1")
    form:setProperty("lineEdit_2", "text", "2")
    form:setProperty("lineEdit_3", "text", "")
    form:setProperty("lineEdit_3", "enabled", false)

    form:setProperty("pushButton_1", "text", "A умножить B")
    form:setProperty("pushButton_2", "text", "Закрыть") 

    print("application_id", form.application_id)
    print("ui_form_file", form.ui_form_file)
    print("form_id", form.form_id)
    print("Инициализация приложения выполнена!")
    -- Здесь можно добавить любую логику инициализации
    -- Регистрируем обработчики
    form:on("pushButton_1", "Clicked", function()
        a = tonumber(form:getProperty("lineEdit_1", "text")) or 0
        b = tonumber(form:getProperty("lineEdit_2", "text")) or 0

        form:setProperty("lineEdit_3", "text", a * b)
    end)

    form:on("pushButton_2", "Clicked", function()
        form:close()
    end)
end

-- Глобальная функция, вызываемая из Elixir/GUI
function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

-- Опционально: expose close
function close()
    form:close()
end
