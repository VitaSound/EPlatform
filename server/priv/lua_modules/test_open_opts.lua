local form = require("form")()

function init(app_id, form_id, form_file, opts)
    form:create(app_id, form_id, form_file or "test_dialog01.ui")

    local title = "no opts"
    if opts and opts.title then
        title = opts.title
    end

    form:setFormProperty("windowTitle", title)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
