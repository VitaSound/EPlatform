# Quick Start

## Состав

| Часть | Путь | Роль |
|-------|------|------|
| Сервер | `server/` | Phoenix + Lua-сессии форм |
| Клиент | `client/` | wasm UI (form-xml · rqt · app) |
| Формы | `server/priv/qt_forms/` | `.ui` |
| Скрипты | `server/priv/lua_modules/` | Lua |

## Запуск (типичный)

1. Поднять Phoenix (`server/`, обычно порт **4000** / **4010** для WS — смотрите локальный конфиг).
2. Собрать/раздать wasm-клиент (`client/`, trunk / `serve_dist`).
3. Открыть URL клиента с параметрами:

| Режим | URL | Смысл |
|-------|-----|--------|
| Full shell | `?main=gallery.lua` | главное окно (menu/toolbar/MDI) + скрипт |
| Одна форма | `?lua=test_01.lua&ui=test_dialog01.ui` | standalone без MainWindow |

Точный host/port зависят от вашей машины (WSL/Windows). Backend сокета часто задаётся так: `backend=ws://<host>:4010/socket/websocket`.

## Минимальный сценарий формы

1. Нарисовать диалог в Qt Designer → сохранить `.ui` в `priv/qt_forms/`.
2. Написать Lua: создать `Form`, подписаться на события, читать/писать свойства.
3. Открыть клиент с `lua` + `ui` (или из `main`-скрипта через `App:open_lua` / модалку).

Пример идеи (упрощённо):

```lua
local Form = require("form")
-- после create / init платформы:
form:on("okButton", "Clicked", function()
  local text = form:get("nameEdit", "text")
  form:finish_modal({ status = "ok", value = text })
end)
```

Подробности API — [[Lua Form API]], оболочка приложения — [[Lua App API]].

## Галерея и review

- Живые примеры: `gallery.lua` / формы в `priv/qt_forms/gallery/`.
- Визуальное сравнение с Qt: `client/review/` (инженерный пак, не user manual).

## Ограничения, о которых сразу знать

- Нет полного Qt / QML / rich `QTextEdit` (HTML).
- QSS: локальные `url()` / `border-image` (stretch); remote URL и полный CSS slice/repeat — нет.
- Часть `App:resize` / `showMaximized` и т.п. в браузере — no-op (задел под desktop-host).
