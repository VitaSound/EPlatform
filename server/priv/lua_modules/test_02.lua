local counter = 0
-- local mathlib = require("mathlib")
-- print(mathlib.add(2, 3)) -- вызовет функцию из библиотеки

local application = require("application")()
local form = require("form")()

function init(app_id, frm_id)
    form_file = "test_dialog02.ui"
    -- запускаем форму на клиенте
    form:create(app_id, frm_id, form_file)

    form:setFormProperty("windowTitle", "Тестовая форма")

    form:setProperty("radioButton_2", "checked", true)
    form:setProperty("checkBox_2", "checked", true)

    items = {"Option 1", "Option 2", "Option 3"}
    form:setProperty("comboBox_1", "items", items)
    form:setProperty("comboBox_1", "currentIndex", 2)

    form:setProperty("listWidget", "items", items)
    form:setProperty("listWidget", "currentIndex", 1)  


    form:setProperty("tableWidget", "clear", true)
    form:setProperty("tableWidget", "rowCount", 3)
    form:setProperty("tableWidget", "columnCount", 2)
    form:setProperty("tableWidget", "horizontalHeaderLabels", {"Col 1", "Col 2"})

    form:setProperty("tableWidget", "data", {
        {"Row 1 Col 1", "Row 1 Col 2"},
        {"Row 2 Col 1", "Row 2 Col 2"},
        {"Row 3 Col 1", "Row 3 Col 2"}
    })

    form:setProperty("treeWidget", "headerLabels", {"Column 1", "Column 2"})
    form:setProperty("treeWidget", "data", {
        {text = "Root 1", columns = {"Root Column"}, children = {
            {text = "Child 1.1", columns = {"Data 1.1"}},
            {text = "Child 1.2", columns = {"Data 1.2"}}
        }},
        {text = "Root 2", children = {
            {text = "Child 2.1", columns = {"Data 2.1"}}
        }}
    })
        

    -- Здесь можно добавить любую логику инициализации
    -- Регистрируем обработчики
    form:on("pushButton_1", "Clicked", function()
        a = form:getProperty("comboBox_1", "currentIndex")
        -- b = form:getProperty("comboBox_1", "currentText")

        a = form:getProperty("listWidget", "currentRow")
        -- b = form:getProperty("listWidget", "currentText")

        form:setProperty("lineEdit_1", "text", a)
        -- form:setProperty("lineEdit_2", "text", b)


    end)

    form:on("pushButton_2", "Clicked", function()
        form:close()
        application:close(form.application_id)
    end)

    form:on(frm_id, "Close", function()
        application:close(form.application_id)
    end)
end

-- Глобальная функция, вызываемая из Elixir/GUI
function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

-- Опционально: expose close
function close()
    form:close()
    application:close(form.application_id)
end
