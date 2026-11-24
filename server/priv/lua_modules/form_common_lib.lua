local Icons = require("icons")

local M = {}

M.DEFAULTS = {
    ui_file = "abstract_table_form.ui",
    title = "Форма",
    geometry = {x = 120, y = 80, w = 800, h = 600},
    ok_text = "ОК",
    cancel_text = "Отмена",
    show_buttons = true
}

function M.setup_form(form, opts)
    local o = opts or {}

    form:setWindowTitle(o.title or M.DEFAULTS.title)
    form:setGeometry(o.geometry or M.DEFAULTS.geometry)

    form:setText("btn_ok", o.ok_text or M.DEFAULTS.ok_text)
    Icons.set_button_icon(form, "btn_ok", "ok")
    form:setText("btn_cancel", o.cancel_text or M.DEFAULTS.cancel_text)
    Icons.set_button_icon(form, "btn_cancel", "cancel")

    local show_buttons = o.show_buttons
    if show_buttons == nil then
        show_buttons = M.DEFAULTS.show_buttons
    end
    form:setVisible("btn_ok", show_buttons)
    form:setVisible("btn_cancel", show_buttons)

    form:setToolButton("tbtn_add", {
        text = "",
        icon = Icons.get("add"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_add", "Добавить")

    form:setToolButton("tbtn_edit", {
        text = "",
        icon = Icons.get("edit"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_edit", "Редактировать")

    form:setToolButton("tbtn_delete", {
        text = "",
        icon = Icons.get("delete"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_delete", "Удалить")

    form:setToolButton("tbtn_refresh", {
        text = "",
        icon = Icons.get("refresh"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_refresh", "Обновить")

    form:setToolButton("tbtn_view", {
        text = "",
        icon = Icons.get("view"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_view", "Просмотр")

    form:setToolButton("tbtn_macro", {
        text = "",
        icon = Icons.get("macro"),
        style = "icon_only",
        icon_size = {w = 16, h = 16}
    })
    form:setToolTip("tbtn_macro", "Макрос")

    form:set_prop("tbl", "selectRows", true)
    form:set_prop("tbl", "readOnly", true)

    return form
end

return M
