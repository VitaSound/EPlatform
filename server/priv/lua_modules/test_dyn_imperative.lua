-- Imperative dynamic form (plan 44 Variant A): empty shell + add_layout/add.
-- Open: ?lua=test_dyn_imperative.lua  (no ui=)
local App = require("application")
local Form = require("form")

local app = App()
local form = Form()

function init(application_id, form_id, _ui_file)
    app:init(application_id)

    form:create_empty(application_id, form_id, {
        object_name = "Dialog",
        window_title = "Dynamic imperative",
        width = 420,
        height = 260,
    })
    form:setFormProperty("geometry", { x = 100, y = 80, w = 420, h = 260 })
    form:setWindowTitle("Dynamic imperative")

    local root = form:add_layout("QVBoxLayout", "root")
    local title = form:add("QLabel", "title", {
        parent = root,
        text = "Built with form:add / add_layout (no .ui file)",
    })
    local name = form:add("QLineEdit", "nameEdit", { parent = root })
    local row = form:add_layout("QHBoxLayout", "row", { parent = root })
    local ok = form:add("QPushButton", "ok", { parent = row, text = "OK" })
    local cancel = form:add("QPushButton", "cancel", { parent = row, text = "Cancel" })
    form:add("QCheckBox", "remember", { parent = root, text = "Remember me" })

    ok:on("Clicked", function()
        local text = name:get("text") or ""
        title:set("text", "Hello, " .. tostring(text))
        ok:set_enabled(false)
    end)

    cancel:on("Clicked", function()
        form:remove(ok)
        form:close()
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
