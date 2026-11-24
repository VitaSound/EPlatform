local application = require("application")()
local form = require("form")()

function init(app_id, frm_id, form_file)
    form:create(app_id, frm_id, form_file or "test_dialog03.ui")
    form:setFormProperty("windowTitle", "Modal sibling")
    form:setProperty("cellInfo", "text", "sibling-ready")

    form:on("toggleScroll", "Clicked", function()
        local value = form:getProperty("cellInfo", "text")
        local title = form:getFormProperty("windowTitle")
        form:setProperty("cellInfo", "text", "sibling:" .. tostring(value) .. "/" .. tostring(title))
    end)

    form:on("alwaysOn", "Clicked", function()
        application:close(form.application_id)
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
