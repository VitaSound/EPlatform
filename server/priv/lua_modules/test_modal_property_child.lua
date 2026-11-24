local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog02.ui")
    form:setFormProperty("windowTitle", "Property child")
    form:setProperty("label_1", "text", "child-ready")

    form:on("pushButton_1", "Clicked", function()
        local value = form:getProperty("lineEdit_1", "text")
        form:setProperty("label_2", "text", "read:" .. tostring(value))
        form:finish_modal("ok")
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:finish_modal(nil)
end
