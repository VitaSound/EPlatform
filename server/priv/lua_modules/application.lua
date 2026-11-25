-- application.lua
-- модуль методов для работы с приложением

local App = {}
App.__index = App

-- Конструктор
function App.new()
    local self = setmetatable({}, App)
    self.application_id = nil
    return self
end

function App:init(application_id)
    self.application_id = application_id
end

function App:close(application_id)
    close_application(application_id or self.application_id)
end

function App:open_modal_lua(parent_form_id, lua_file, opts)
    return open_modal_lua(self.application_id, parent_form_id, lua_file, opts or {})
end

function App:open_lua(lua_file, opts)
    return open_lua(self.application_id, lua_file, opts or {})
end

-- Application-level properties go through a single host call.
local function set(self, property, value)
    return set_app_property(self.application_id, property, value)
end

local function size_of(size_or_w, h)
    if type(size_or_w) == "table" then
        return size_or_w
    end
    return { w = size_or_w, h = h }
end

local function point_of(pos_or_x, y)
    if type(pos_or_x) == "table" then
        return pos_or_x
    end
    return { x = pos_or_x, y = y }
end

local function rect_of(rect_or_x, y, w, h)
    if type(rect_or_x) == "table" then
        return rect_or_x
    end
    return { x = rect_or_x, y = y, w = w, h = h }
end

-- Browser chrome: these are applied by the browser client.
function App:setWindowTitle(title)
    return set(self, "windowTitle", title)
end

-- `icon` is a URL / data URI (browser). Legacy Qt specs (`img24:N`,
-- `:/icons/...`) are stored and left for a desktop host to resolve.
function App:setIcon(icon)
    return set(self, "windowIcon", icon)
end

-- Native-window geometry/state below is kept for the future desktop binary.
-- Menu/toolbar/status methods above are applied by the web application shell;
-- `setAppProperty` is the stable protocol hook for both.
function App:resize(size_or_w, h)
    return set(self, "size", size_of(size_or_w, h))
end

function App:move(pos_or_x, y)
    return set(self, "position", point_of(pos_or_x, y))
end

function App:setGeometry(rect_or_x, y, w, h)
    return set(self, "geometry", rect_of(rect_or_x, y, w, h))
end

function App:center()
    return set(self, "centered", true)
end

function App:showMaximized()
    return set(self, "windowState", "maximized")
end

function App:showMinimized()
    return set(self, "windowState", "minimized")
end

function App:showNormal()
    return set(self, "windowState", "normal")
end

function App:showFullScreen()
    return set(self, "windowState", "fullscreen")
end

function App:visible(flag)
    return set(self, "visible", flag ~= false)
end

function App:set_menu(menu_spec)
    return set(self, "menu", menu_spec or {})
end

function App:setToolBar(toolbar_spec)
    return set(self, "toolBar", toolbar_spec or {})
end

function App:setToolBarVisible(visible)
    return set(self, "toolBarVisible", visible ~= false)
end

function App:setStatusMessage(text, timeout)
    if text == nil then
        return set(self, "statusMessage", nil)
    end
    return set(self, "statusMessage", { text = text, timeout = timeout })
end

function App:showMessage(text, timeout)
    return self:setStatusMessage(text, timeout)
end

function App:setStatusVisible(visible)
    return set(self, "statusVisible", visible ~= false)
end

function App:setStatusInfo(text)
    return set(self, "statusInfo", text)
end

-- Синтаксический сахар: return конструктор
return App.new
