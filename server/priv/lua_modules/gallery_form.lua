-- Один кейс галереи: форма из gallery/*.ui (список — gallery_index.lua,
-- навигация — main-сессия gallery.lua).
local form = require("form")()

function init(app_id, frm_id, ui_file)
    form:create(app_id, frm_id, ui_file)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
