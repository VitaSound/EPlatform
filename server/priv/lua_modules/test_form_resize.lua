-- Fixture: resizable flag and content size constraints.
local form = require("form")()

function init(app_id, form_id, ui_file)
    form:create(app_id, form_id, ui_file or "test_dialog01.ui")
    form:setResizable(false)
    form:setMinimumSize(200, 150)
    form:setMaximumSize(400, 300)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
