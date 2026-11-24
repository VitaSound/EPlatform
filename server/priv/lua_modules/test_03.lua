local application = require("application")()
local form = require("form")()

local hidden = false

function init(app_id, frm_id)
    local form_file = "test_dialog03.ui"
    form:create(app_id, frm_id, form_file)

    form:setFormProperty("windowTitle", "Scroll + policy demo")

    local headers = {}
    for c = 1, 10 do
        headers[c] = "Column " .. c
    end
    form:setProperty("table", "horizontalHeaderLabels", headers)

    local data = {}
    for r = 1, 60 do
        local row = {}
        for c = 1, 10 do
            row[c] = "R" .. r .. " C" .. c
        end
        data[r] = row
    end
    form:setProperty("table", "data", data)
    form:setProperty("table", "currentRow", 1)

    form:on("toggleScroll", "Clicked", function()
        hidden = not hidden
        local policy = hidden and "AlwaysOff" or "AsNeeded"
        form:setProperty("table", "verticalScrollBarPolicy", policy)
        form:setProperty("table", "horizontalScrollBarPolicy", policy)
        form:setProperty("cellInfo", "text", "policy: " .. policy)
    end)

    form:on("alwaysOn", "Clicked", function()
        form:setProperty("table", "verticalScrollBarPolicy", "AlwaysOn")
        form:setProperty("table", "horizontalScrollBarPolicy", "AlwaysOn")
        form:setProperty("cellInfo", "text", "policy: AlwaysOn")
    end)

    form:on("table", "CurrentCellChanged", function()
        local row = form:getProperty("table", "currentRow")
        local col = form:getProperty("table", "currentColumn")
        local cell = form:getProperty("table", "currentRowCell:" .. tostring(col + 1))
        form:setProperty("cellInfo", "text", "r" .. row .. " c" .. col .. " = " .. tostring(cell))
    end)

    form:on("close", "Clicked", function()
        form:close()
    end)

    form:on(frm_id, "Close", function()
        application:close(form.application_id)
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end