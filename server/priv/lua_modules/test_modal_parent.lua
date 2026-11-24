local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog01.ui")
    form:setFormProperty("windowTitle", "Modal parent")
    form:setProperty("lineEdit_1", "text", "waiting")

    local result = form:open_modal_lua("test_modal_child.lua", {
        ui_file = "test_dialog02.ui"
    })

    form:setProperty("lineEdit_1", "text", "result:" .. tostring(result))
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
