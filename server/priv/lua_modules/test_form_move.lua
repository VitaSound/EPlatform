-- Fixture: Form:move / Form:resize sugar (position and size properties).
local form = require("form")()

function init(app_id, form_id, ui_file)
    form:create(app_id, form_id, ui_file or "test_dialog01.ui")
    form:move(40, 40)
    form:resize(320, 200)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
