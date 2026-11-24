-- application.lua
-- модуль методов для работы с приложением

local App = {}
App.__index = App

-- Конструктор
function App.new()
    local self = setmetatable({}, App)

    return self
end

-- Закрываем приложение
function App:close(application_id)
    close_application(application_id)
end

-- Синтаксический сахар: return конструктор
return App.new
