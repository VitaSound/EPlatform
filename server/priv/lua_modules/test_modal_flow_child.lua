local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog02.ui")
    form:setFormProperty("windowTitle", "Modal flow child")
    form:setProperty("label_1", "text", "child-ready")

    form:on("pushButton_1", "Clicked", function()
        form:finish_modal("ok")
    end)

    form:on("pushButton_2", "Clicked", function()
        form:finish_modal("cancel")
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:finish_modal(nil)
end
