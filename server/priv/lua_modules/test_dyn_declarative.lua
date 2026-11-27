-- Declarative dynamic form (plan 44 Variant B): no .ui file on disk.
-- Open: ?lua=test_dyn_declarative.lua  (no ui=)
local App = require("application")
local Form = require("form")
local Ui = require("ui")

local app = App()
local form = Form()

function init(application_id, form_id, _ui_file)
    app:init(application_id)

    local spec = Ui.dialog {
        name = "DynDeclarative",
        windowTitle = "Dynamic declarative",
        width = 440,
        height = 300,
        layout = Ui.vbox {
            name = "root",
            Ui.label { name = "title", text = "Built from Lua Ui.* (no .ui file)" },
            Ui.line_edit { name = "nameEdit", text = "" },
            Ui.hbox {
                name = "row",
                Ui.button { name = "ok", text = "OK" },
                Ui.button { name = "clear", text = "Clear" },
                Ui.button { name = "closeBtn", text = "Close" },
            },
            Ui.group {
                name = "opts",
                title = "Options",
                layout = Ui.hbox {
                    name = "optsRow",
                    Ui.check_box { name = "remember", text = "Remember" },
                    Ui.combo { name = "mode", currentText = "A" },
                },
            },
        },
    }

    form:create(application_id, form_id, { ui_spec = spec })
    form:setFormProperty("geometry", { x = 80, y = 60, w = 440, h = 300 })

    local name = form:widget("nameEdit")
    local ok = form:widget("ok")
    ok:on("Clicked", function()
        local text = name:get("text") or ""
        form:set("title", "text", "Hello, " .. tostring(text))
    end)

    form:on("clear", "Clicked", function()
        name:set("text", "")
        form:set("title", "text", "Built from Lua Ui.* (no .ui file)")
    end)

    form:on("closeBtn", "Clicked", function()
        form:close()
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
