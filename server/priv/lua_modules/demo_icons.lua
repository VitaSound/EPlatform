-- Demo: sprite slicing (`img24:N` from IMAGELIST24) and browser chrome.
-- Open with: ?lua=demo_icons.lua&ui=test_dialog01.ui
local App = require("application")
local Form = require("form")

local app = App()
local form = Form()

function init(application_id, form_id, ui_file)
    form:create(application_id, form_id, ui_file or "test_dialog01.ui")
    form:setFormProperty("windowTitle", "Иконки IMAGELIST24")
    form:setFormProperty("geometry", { x = 120, y = 80, w = 460, h = 260 })

    -- Browser chrome: tab title + favicon (sliced icon 22).
    app:init(application_id)
    app:setWindowTitle("EPlatform — нарезка IMAGELIST24")
    app:setIcon("img24:22")

    form:setProperty("label_1", "text", "icon 22:")
    form:setProperty("label_2", "text", "icon 3:")
    form:setProperty("label_3", "text", "favicon:")
    form:setProperty("lineEdit_1", "text", "img24:22 (OK)")
    form:setProperty("lineEdit_2", "text", "img24:3 (Cancel)")
    form:setProperty("lineEdit_3", "text", "тот же img24:22")
    form:setProperty("lineEdit_3", "enabled", false)

    form:setProperty("pushButton_1", "text", "OK")
    form:setProperty("pushButton_1", "icon", "img24:22")
    form:setProperty("pushButton_2", "text", "Cancel")
    form:setProperty("pushButton_2", "icon", "img24:3")

    -- Move the MDI subwindow by changing its content coordinates.
    form:on("pushButton_1", "Clicked", function()
        form:move(40, 40)
    end)
    form:on("pushButton_2", "Clicked", function()
        form:resize(380, 220)
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
