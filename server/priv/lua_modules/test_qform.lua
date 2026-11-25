-- QFormLayout demo: ?lua=test_qform.lua&ui=test_qform.ui
local App = require("application")
local Form = require("form")

local app = App()
local form = Form()

function init(application_id, form_id, ui_file)
    app:init(application_id)
    form:create(application_id, form_id, ui_file or "test_qform.ui")
    form:setFormProperty("windowTitle", "QFormLayout demo")
    form:setFormProperty("geometry", { x = 140, y = 100, w = 420, h = 240 })

    form:on("resetButton", "Clicked", function()
        form:setProperty("lineEdit_1", "text", "")
        form:setProperty("checkBox_1", "checked", false)
    end)
    form:on("closeButton", "Clicked", function()
        form:close()
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
