local App = {}
App.__index = App

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

return App.new
