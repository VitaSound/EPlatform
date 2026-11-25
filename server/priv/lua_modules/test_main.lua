local app = require("application")()

function init(application_id, _form_id, _ui_file, _opts)
    app:init(application_id)
    app:setWindowTitle("EPlatform main window")
    app:setStatusVisible(true)
    app:setStatusMessage("Готово", 5000)
    app:set_menu({
        {
            title = "Файл",
            children = {
                {
                    title = "Тестовая форма",
                    action = "open_lua",
                    lua_file = "test_form_move.lua",
                    mdi = true,
                    icon = "img24:45"
                },
                { separator = true },
                { title = "Выход", action = "exit", icon = "img24:12" }
            }
        },
        {
            title = "Вид",
            children = {
                {
                    title = "Вложенное",
                    children = {
                        {
                            title = "Отдельное окно",
                            action = "open_lua",
                            lua_file = "test_form_move.lua",
                            mdi = false
                        }
                    }
                }
            }
        }
    })
    app:setToolBar({
        {
            title = "Тест",
            action = "open_lua",
            lua_file = "test_form_move.lua",
            mdi = true,
            icon = "img24:45"
        },
        { separator = true },
        {
            title = "Выход",
            action = "exit",
            icon = "img24:12"
        }
    })
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
