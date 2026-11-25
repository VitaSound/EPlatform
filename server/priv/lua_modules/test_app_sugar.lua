-- Fixture for application-level (browser chrome) sugar.
local App = require("application")

local app = App()

function init(application_id, _form_id, _ui_file, _opts)
    app:init(application_id)

    app:setWindowTitle("Sugar title")
    app:setIcon("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>")

    -- Reserved for a desktop host; the browser client ignores these.
    app:resize(640, 480)
    app:move(10, 20)
    app:setGeometry({ x = 1, y = 2, w = 640, h = 480 })
    app:center()
    app:showMaximized()
    app:showMinimized()
    app:visible(true)
end
