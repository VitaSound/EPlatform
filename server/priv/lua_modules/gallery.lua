-- EPlatform: галерея форм для ручного теста виджетов (0.18.5).
--
-- Запуск: ?main=gallery.lua (полный режим, центральная MDI-область).
-- Формы генерирует client/tools/formgen/review.py в
-- server/priv/qt_forms/gallery/, порядок и номера — gallery_index.lua.
-- Каждая форма открывается своим сессионным скриптом gallery_form.lua;
-- заголовок формы несёт "#NNN id", статусбар показывает номер и позицию.
--
-- Навигация: меню «Галерея» (Следующая/Предыдущая + категории), кнопки
-- тулбара «◀/▶», а также «Каскад»/«Плитка» (App:cascadeWindows/tileWindows).

local app = require("application")()
local cases = require("gallery_index")

local current = 0

local function display_id(id)
    -- "gallery/hbox/105_label_line_btn" -> "hbox/label_line_btn"
    local short = id:gsub("^gallery/", ""):gsub("^%d+_", "")
    return short
end

local function open_case(index)
    if #cases == 0 then
        app:setStatusMessage("Галерея пуста: прогони tools/formgen/review.py", 8000)
        return
    end
    while index < 1 do
        index = index + #cases
    end
    while index > #cases do
        index = index - #cases
    end
    current = index
    local case = cases[current]
    app:open_lua("gallery_form.lua", { ui_file = case.ui, mdi = true })
    app:setStatusMessage(
        string.format("#%03d %s (%d/%d)", case.number, display_id(case.id), current, #cases),
        8000
    )
end

local function index_of_ui(ui_file)
    for index, case in ipairs(cases) do
        if case.ui == ui_file then
            return index
        end
    end
    return nil
end

local function build_menu()
    local groups = {}
    local order = {}
    for _, case in ipairs(cases) do
        if not groups[case.category] then
            groups[case.category] = {}
            table.insert(order, case.category)
        end
        table.insert(groups[case.category], case)
    end

    local children = {
        { title = "Следующая форма", action = "gallery_next" },
        { title = "Предыдущая форма", action = "gallery_prev" },
        { separator = true },
    }
    for _, category in ipairs(order) do
        local submenu = { title = display_id(category), children = {} }
        for _, case in ipairs(groups[category]) do
            table.insert(submenu.children, {
                title = string.format("#%03d %s", case.number, display_id(case.id)),
                action = "gallery_open",
                ui_file = case.ui,
                mdi = true,
            })
        end
        table.insert(children, submenu)
    end

    return {
        {
            title = "Галерея",
            icon = "img24:45",
            children = children,
        },
        {
            title = "Окна",
            children = {
                { title = "Каскад", action = "cascade" },
                { title = "Плитка", action = "tile" },
                { separator = true },
                { title = "Выход", action = "exit", icon = "img24:12" },
            },
        },
    }
end

function init(application_id, _form_id, _ui_file, _opts)
    app:init(application_id)
    app:setWindowTitle("Галерея форм")
    app:set_menu(build_menu())
    app:setToolBar({
        { title = "◀ Назад", action = "gallery_prev" },
        { title = "Вперёд ▶", action = "gallery_next" },
        { separator = true },
        { title = "Каскад", action = "cascade" },
        { title = "Плитка", action = "tile" },
    })
    app:setStatusMessage(string.format("Галерея: %d форм", #cases), 8000)
    open_case(1)
end

function onMenuAction(action, lua_file, ui_file, _mdi, _modal, _modal_scope)
    if action == "exit" then
        app:close()
    elseif action == "gallery_next" then
        open_case(current + 1)
    elseif action == "gallery_prev" then
        open_case(current - 1)
    elseif action == "gallery_open" and ui_file ~= "" then
        local index = index_of_ui(ui_file)
        if index then
            open_case(index)
        end
    elseif action == "cascade" then
        app:cascadeWindows()
        app:setStatusMessage("Окна раскаскадированы", 4000)
    elseif action == "tile" then
        app:tileWindows()
        app:setStatusMessage("Окна разложены плиткой", 4000)
    elseif action == "open_lua" and lua_file ~= "" then
        local opts = {}
        if ui_file ~= "" then
            opts.ui_file = ui_file
        end
        app:open_lua(lua_file, opts)
    end
end

function onEvent(_widget_name, _event_type)
end
