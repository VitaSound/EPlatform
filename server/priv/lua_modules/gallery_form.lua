-- Один кейс галереи: форма из gallery/*.ui (список — gallery_index.lua,
-- навигация — main-сессия gallery.lua).
local form = require("form")()

-- Demo items for every gallery QComboBox. Without items the popup refuses
-- to open (`open_combo_popup` no-ops on an empty list).
local DEMO_COMBO_ITEMS = {
    "Alpha", "Apricot", "Banana", "Blueberry", "Cherry",
    "Date", "Elderberry", "Fig", "Grape", "Honeydew",
    "Kiwi", "Lemon", "Mango", "Nectarine", "Orange",
    "Papaya", "Quince", "Raspberry", "Strawberry", "Watermelon",
}

-- All objectNames used by gallery/*.ui QComboBox widgets. Missing names are
-- ignored by setProperty.
local COMBO_NAMES = {
    "plain", "editable",
    "c1", "c2", "c3",
    "comboBox_1", "combo1",
    "idle", "focus",
}

local function seed_combos(ui_file)
    for _, name in ipairs(COMBO_NAMES) do
        form:setProperty(name, "items", DEMO_COMBO_ITEMS)
        form:setProperty(name, "currentIndex", 0)
    end
    -- #155: editable filter demo — force items + free-text mode even if the
    -- .ui was opened without Designer `<item>` rows.
    if ui_file == "gallery/combo/240_editable_search.ui" then
        form:setProperty("plain", "items", DEMO_COMBO_ITEMS)
        form:setProperty("plain", "currentIndex", 0)
        form:setProperty("editable", "items", DEMO_COMBO_ITEMS)
        form:setProperty("editable", "currentIndex", 0)
        form:setProperty("editable", "editable", true)
        -- Empty edit text so the arrow opens the full list; typing filters.
        form:setProperty("editable", "text", "")
    end
end

function init(app_id, frm_id, ui_file)
    form:create(app_id, frm_id, ui_file)
    seed_combos(ui_file)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
