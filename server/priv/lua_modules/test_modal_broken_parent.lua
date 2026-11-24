local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file)
    form:setFormProperty("windowTitle", "Broken modal parent")
    form:setProperty("label_1", "text", "ready")

    form:on("pushButton_1", "Clicked", function()
        form:setProperty("label_1", "text", "opening")
        local result = form:open_modal_lua("test_modal_broken_child.lua", {})
        form:setProperty("label_1", "text", "closed:" .. tostring(result))
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
