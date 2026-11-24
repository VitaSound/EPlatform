local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog02.ui")
    form:setFormProperty("windowTitle", "Nested child")
    form:setProperty("label_1", "text", "child-ready")

    form:on("pushButton_1", "Clicked", function()
        form:setProperty("label_1", "text", "grandchild-opening")
        local result = form:open_modal_lua("test_modal_grandchild.lua", {
            ui_file = "test_dialog03.ui"
        })
        form:setProperty("label_1", "text", "grand:" .. tostring(result))
        form:finish_modal("child:" .. tostring(result))
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
