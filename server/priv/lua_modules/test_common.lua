local form = require("form")()
local common = require("form_common_lib")

function init(app_id, form_id, form_file)
    form:create(app_id, form_id, form_file or "abstract_table_form.ui")
    common.setup_form(form, {
        title = "Common form",
        geometry = {x = 120, y = 80, w = 800, h = 600}
    })

    form:setTableHeaders("tbl", {
        clear = true,
        columnCount = 2,
        horizontalHeaderLabels = {"Name", "Value"},
        columnWidths = {240, 120}
    })

    form:setTableData("tbl", {
        rowCount = 2,
        data = {
            {"one", "1"},
            {"two", "2"}
        }
    })

    form:on("btn_ok", "MouseButtonRelease", function()
        form:close()
    end)

    form:on("btn_cancel", "Clicked", function()
        form:close()
    end)
end

function onEvent(widget_name, event_type)
    form:handleEvent(widget_name, event_type)
end

function close()
    form:close()
end
