local form = require("form")()
local application = require("application")()

function init(app_id, form_id, form_file)
    application:init(app_id)
    form:create(app_id, form_id, form_file or "test_dialog01.ui")

    local child_id = application:open_lua("test_open_opts.lua", {
        ui_file = "test_dialog01.ui",
        title = "Child via open_lua"
    })

    form:setProperty("label_1", "text", "child:" .. tostring(child_id))
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
