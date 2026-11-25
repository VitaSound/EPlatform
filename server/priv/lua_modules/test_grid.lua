-- Grid layout demo: ?lua=test_grid.lua&ui=test_grid.ui
local App = require("application")
local Form = require("form")

local app = App()
local form = Form()

function init(application_id, form_id, ui_file)
    app:init(application_id)
    form:create(application_id, form_id, ui_file or "test_grid.ui")
    form:setFormProperty("windowTitle", "QGridLayout demo")
    form:setFormProperty("geometry", { x = 120, y = 80, w = 440, h = 320 })

    form:on("pushButton_1", "Clicked", function()
        form:setProperty("lineEdit_2", "text", "")
    end)
    form:on("pushButton_2", "Clicked", function()
        form:close()
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
