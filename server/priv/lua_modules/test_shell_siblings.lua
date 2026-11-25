-- Standalone shell integration: parent form plus two top-level siblings.
-- URL: ?lua=test_shell_siblings.lua&ui=test_dialog01.ui
local form = require("form")()
local application = require("application")()

function init(app_id, form_id, ui_file)
    application:init(app_id)
    form:create(app_id, form_id, ui_file or "test_dialog01.ui")
    form:setFormProperty("windowTitle", "Shell parent")
    form:move(640, 80)
    form:resize(380, 190)

    local moved = application:open_lua("test_form_move.lua", {
        ui_file = "test_dialog01.ui"
    })
    local titled = application:open_lua("test_open_opts.lua", {
        ui_file = "test_dialog01.ui",
        title = "Sibling child"
    })

    form:setProperty("label_1", "text", "siblings " .. tostring(moved) .. " / " .. tostring(titled))
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
