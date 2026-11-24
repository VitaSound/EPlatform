local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog03.ui")
    form:setFormProperty("windowTitle", "Nested grandchild")
    form:setProperty("cellInfo", "text", "grandchild-ready")

    form:on("close", "Clicked", function()
        form:finish_modal("grand-ok")
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:finish_modal(nil)
end
