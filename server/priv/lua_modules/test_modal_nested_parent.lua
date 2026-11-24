local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog01.ui")

    form:setFormProperty("windowTitle", "Nested parent")
    form:setProperty("label_1", "text", "ready")
    form:setProperty("label_2", "text", "idle")

    form:on("pushButton_1", "Clicked", function()
        form:setProperty("label_1", "text", "opening")
        local result = form:open_modal_lua("test_modal_nested_child.lua", {
            ui_file = "test_dialog02.ui"
        })
        form:setProperty("label_1", "text", "closed:" .. tostring(result))
    end)

    form:on("pushButton_2", "Clicked", function()
        form:setProperty("label_2", "text", "queued")
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
