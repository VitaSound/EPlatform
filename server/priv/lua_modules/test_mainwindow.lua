-- Static QMainWindow.ui demo: ?main=test_mainwindow.lua&ui=mainwindow.ui
-- The client loads menu/toolbar/status/title from mainwindow.ui; this script
-- intentionally does not call set_menu/setToolBar so the static model stays.
local app = require("application")()

function init(application_id, _form_id, _ui_file, _opts)
    app:init(application_id)
    app:setStatusMessage("QMainWindow.ui загружен", 8000)
end

function onMenuAction(action, lua_file, ui_file, mdi, modal, modal_scope)
    if action == "exit" then
        app:close()
    elseif action == "open_lua" and lua_file ~= "" then
        local opts = {
            mdi = mdi,
            modal = modal,
            modal_scope = modal_scope
        }
        if ui_file ~= "" then
            opts.ui_file = ui_file
        end
        app:open_lua(lua_file, opts)
    end
end

function onEvent(_widget_name, _event_type)
end
