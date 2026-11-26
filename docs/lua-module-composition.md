# Lua-модули EPlatform: композиция, defaults, sugar и модальный event loop

> Статус: проектная записка / architecture decision record.
> Дата: 2026-11-25.
> Область: `server/priv/lua_modules`, `server/lib/e_platform/lua`,
> `server/lib/e_platform/dialog_session.ex`, `server/lib/e_platform_web`.
> Документ фиксирует, что берём из `tcs_back@ref_qt`, что улучшаем и что
> сознательно не переносим. Изменения кода идут отдельными коммитами.

## 0. Короткий вывод

1. **Наследования в Lua-модулях нет и не нужно.** Есть один metatable на
   `Form`/`App`, а переиспользование делается композицией:
   `form_common_lib.setup_form(form, opts)`, `icons`, базовые `.ui`-шаблоны.
2. **Берём не «иерархию классов», а три слоя:**
   - host API (Elixir ↔ Lua),
   - платформенные Lua-модули (`form`, `application`, `icons`,
     `form_common_lib`),
   - прикладные формы (`measurements.lua`, `projects.lua`, ...).
3. **Defaults и имена надо нормализовать.** Главные улучшения:
   - один набор `opts` вместо `start_dialog`/`start_file`/`mode`;
   - `center` без инверсии;
   - `null` → `nil`;
   - никаких глобальных `mode_title`, `form_file`, `counter`;
   - `ui_file`, `mdi`, `modal`, `title`, `geometry`, `open_opts` — явные ключи.
4. **Sugar сохраняем, но делаем предсказуемым.** Канонические методы —
   snake_case; Qt-подобные алиасы (`setText`, `setGeometry`, ...) — тонкий
   слой над каноническими. Один смысл — одно имя.
5. **Event loop модалок уже правильный по идее:** отдельный `DialogSession`
   на форму, блокируется только родитель, результат модалки идёт сырым
   `send`/`receive`, а не через `GenServer.call`.
6. **Из старого кода не берём:** ETS-очередь без супервизора, `busy` без
   монитора, `Agent` без остановки, `mfa`, глобальный `invoke_qt` как
   основной путь, `print` в runtime, `list_*` без слоя данных.

## 1. Источники и что именно сравниваем

### 1.1. Старый код

- `../tcs_back@ref_qt:priv/lua_modules/form.lua` — `Form`, sugar, таблицы.
- `../tcs_back@ref_qt:priv/lua_modules/application.lua` — `App`, `exec_qt`.
- `../tcs_back@ref_qt:priv/lua_modules/form_common_lib.lua` — defaults/mixin.
- `../tcs_back@ref_qt:priv/lua_modules/icons.lua` — именованные иконки.
- `../tcs_back@ref_qt:priv/qt_forms/abstract_table_form.ui` и
  `abstract_tree-table_form.ui` — базовые layout-шаблоны.
- `../tcs_back@ref_qt:lib/ex_platform/lua/manager.ex` — ETS-очередь, `busy`.
- `../tcs_back@ref_qt:lib/ex_platform/lua/api.ex` — `open_modal_lua`,
  `close_modal`, `invoke_qt`, `get_prop`, `mfa`, `list_*`.

### 1.2. Текущий код

- `server/priv/lua_modules/form.lua` — минимальный `Form`.
- `server/priv/lua_modules/application.lua` — минимальный `App`.
- `server/lib/e_platform/lua/api.ex` — host API.
- `server/lib/e_platform/dialog_session.ex` — GenServer на форму.
- `server/lib/e_platform/dialog_manager.ex` — запуск/остановка сессий.
- `server/lib/e_platform_web/channels/platform_channel.ex` — транспорт.
- `docs/lua-event-loop-and-modal-forms.md` — сравнение старого event loop.
- `docs/modal-message-matrix.md` — матрица сообщений модалок.

## 2. Что берём, что меняем, что не берём

| Подход из `ref_qt` | Решение | Комментарий |
|---|---|---|
| `Form`/`App` как class-like таблицы | **Берём** | Один `__index`, без цепочек наследования |
| `form_common_lib.setup_form` | **Берём, переименовываем в явный mixin** | `setup_form(form, opts)`; не «базовый класс» |
| `icons.lua` | **Берём** | Чистая таблица, без состояния |
| `abstract_*.ui` | **Берём** | Это база разметки, а не Lua-класс |
| `set_prop`/`set`/`get_prop` | **Берём, но с оговоркой** | Канон — `set_property`/`set_form_property`; алиасы — тонкий слой |
| `setWindowTitle`, `setGeometry`, `setText`, `setVisible`, `setEnabled`, `setChecked`, `setToolTip`, `setIcon`, `resize`, `move`, `setCurrentRow`, `setCurrentIndex`, `setToolButton` | **Берём** | Это удобный Qt-подобный sugar |
| `set_table*` | **Берём частично** | Сначала `setTableHeaders`/`setTableData`; grouped header — после поддержки клиента |
| `open_modal_lua`/`finish_modal`/`close_modal` | **Берём** | Уже есть каркас; усиливаем cleanup и idempotency |
| `open_opts`/`init_opts` | **Берём, но унифицируем** | Сейчас `open_opts` хранится, но не доходит до `init` |
| `create_empty_form`, `open_lua` | **Берём** | Нужны для динамических и немодальных сценариев |
| `get_prop` через `ref` + `receive` | **Берём** | Уже реализовано как `get_property`/`get_form_property` |
| `Clicked` + fallback `MouseButtonRelease` | **Берём** | Старый `handleEvent` умел оба; это удобно для совместимости |
| `invoke_qt` как универсальный мост | **Не берём как основной путь** | Вместо него — узкие типизированные команды; при необходимости — allowlist |
| `mfa` | **Не берём** | Слишком широкий доступ из песочницы |
| `list_*` (Ecto) | **Не берём сейчас** | Нет слоя данных; при появлении — типизированный `repo`/`query`, а не десятки функций |
| `sleep` | **Не берём** | Блокирует сессию; при необходимости — явный таймаут в host API |
| ETS-очередь + `busy` | **Не копируем** | GenServer mailbox уже сериализует события формы |
| `Agent` + `Registry` | **Не копируем** | `DialogSession` + `Registry` + `DynamicSupervisor` дают жизненный цикл |
| `close_form` только broadcast | **Улучшаем** | Останавливаем сессию, проверяем владельца, делаем idempotent |
| `print` в runtime | **Не берём** | Использовать `Logger`/`log` host API с `form_id` |
| Глобальные `mode_title`, `form_file`, `counter` | **Не берём** | Только `local` и поля instance |

## 3. Целевая структура модулей

```
Lua-скрипт формы (measurements.lua, projects.lua, ...)
   │
   ├── require("form")            → Form.new() → instance
   ├── require("form_common_lib") → setup_form(form, opts)
   ├── require("icons")           → Icons (immutable table)
   └── require("table_form")      → опциональный mixin для таблиц
            │
            ▼
     host API EPlatform.Lua.Api
       create_form, set_property, set_form_property,
       get_property, get_form_property, open_modal_lua,
       close_modal, close_form, close_application,
       create_empty_form, open_lua
            │
            ▼
     DialogSession (GenServer на форму)
            │
            ▼
     PlatformChannel → PubSub → клиент
```

Слои:

| Слой | Что содержит | Кто владеет |
|---|---|---|
| L0 host API | Функции, доступные Lua | Elixir, `Lua.Api` |
| L1 platform modules | `form`, `application`, `icons`, `form_common_lib` | `priv/lua_modules` |
| L2 base UI | `abstract_table_form.ui`, `abstract_tree-table_form.ui` | `priv/qt_forms` |
| L3 application forms | Конкретные формы и их обработчики | `priv/lua_modules` |

Правило: **L3 не лезет напрямую в транспорт и не знает про `GenServer`**.
Он вызывает только host API и платформенные модули.

## 4. Наследование: почему композиция, а не иерархия

### 4.1. Что есть в старом коде

```lua
local Form = {}
Form.__index = Form

function Form.new()
  local self = setmetatable({}, Form)
  self.application_id = null
  self.form_id = null
  self.ui_file = null
  self.mdi = true
  self.modal = false
  self.event_handlers = {}
  self.table_schemas = {}
  return self
end

return Form.new
```

Это **не наследование**, а прототипный объект с одним metatable. У `App`
такая же схема. `form_common_lib` — не родитель, а функция, которая
настраивает уже созданный `form`. `abstract_*.ui` — не класс, а базовая
разметка.

### 4.2. Целевое правило

- **Один metatable на модуль.** `Form.__index = Form`, `App.__index = App`.
- **Никаких `__index`-цепочек** `Form -> BaseForm -> BaseObject`.
- Переиспользование — через **mixin-функции** и **базовые `.ui`**.
- Состояние — только в instance (`self`), не в таблице модуля.
- Модуль при `require` не должен иметь побочных эффектов.

Пример:

```lua
-- form.lua
local Form = {}
Form.__index = Form

function Form.new(opts)
  local o = opts or {}
  local self = setmetatable({}, Form)
  self.application_id = nil
  self.form_id = nil
  self.ui_file = nil
  self.mdi = true
  self.modal = false
  self.event_handlers = {}
  self.table_schemas = {}
  return self
end

return Form.new
```

```lua
-- form_common_lib.lua
local M = {}

function M.setup_form(form, opts)
  local o = opts or {}
  form:setWindowTitle(o.title or "Форма")
  form:setGeometry(o.geometry or {x = 120, y = 80, w = 800, h = 600})
  form:setText("btn_ok", o.ok_text or "ОК")
  form:setText("btn_cancel", o.cancel_text or "Отмена")
  return form
end

return M
```

```lua
-- measurements.lua
local Form = require("form")
local Common = require("form_common_lib")

local form = Form()

function init(app_id, form_id, opts)
  form:create(app_id, form_id, opts)
  Common.setup_form(form, {
    title = "Измерения",
    ui_file = "abstract_table_form.ui",
  })
end
```

### 4.3. Если нужна «базовая форма»

Не делать `Form = setmetatable({}, {__index = Base})`. Вместо этого:

```lua
local Common = require("form_common_lib")
local form = Form()
Common.setup_form(form, opts)
```

Если нужна табличная специализация:

```lua
local TableForm = require("table_form")
TableForm.setup(form, { widget = "tbl" })
```

`table_form` — это mixin, который вызывает методы формы. Он не меняет
`__index` и не создаёт вторую иерархию.

## 5. Defaults и имена

### 5.1. Один набор `opts`

Главная проблема старого кода — разные точки входа по-разному передают
аргументы:

- `start_dialog` кладёт `ui_file` третьим, `init_opts` — четвёртым;
- `start_file` кладёт `init_opts` третьим;
- `open_modal_lua` по умолчанию использует `"mode" => "mdi"`, а
  `create_form` ждёт `mdi`;
- `Form:create` принимает `opts`, а `Form.new` — нет.

Целевой контракт:

```lua
function init(app_id, form_id, opts)
  -- opts = {
  --   ui_file = "abstract_table_form.ui",
  --   mdi = true,
  --   modal = false,
  --   title = "Форма",
  --   geometry = {x=120, y=80, w=800, h=600},
  --   open_opts = { ... }  -- данные, переданные родителем
  -- }
end
```

Для совместимости:

```lua
function init(app_id, form_id, third)
  local opts
  if type(third) == "table" then
    opts = third
  else
    opts = { ui_file = third }
  end
  ...
end
```

`create_form` принимает либо строку, либо таблицу:

```lua
form:create(app_id, form_id, "test_dialog01.ui")
form:create(app_id, form_id, { ui_file = "test_dialog01.ui", mdi = true })
```

### 5.2. `center` без инверсии

Старый код:

```lua
local center = o.center ~= true   -- ❌ по умолчанию true, center=true выключает
```

Правильно:

```lua
local center = o.center ~= false  -- ✅ по умолчанию true
```

Но важнее: **`center` должен быть явной командой клиенту**. Сейчас клиент
не обрабатывает `"centered"` как свойство формы. Поэтому:

- либо добавить команду `centerForm`/свойство `centered` в клиент;
- либо вычислять геометрию в Lua и не обещать `center` до реализации.

Документ фиксирует: **не добавлять `Form:center`, пока клиент не умеет
центрировать**. Иначе получится sugar-пустышка.

### 5.3. `mdi`/`modal` и никакого `mode`

- Канонические ключи: `mdi` (boolean), `modal` (boolean).
- `mode` не использовать. Если нужен режим, это `mdi`/`modal`.
- `create_form` должен получать `mdi`/`modal` явно, а не только через
  process dictionary.

### 5.4. `nil` вместо `null`

В Lua нет `null`. Если в окружении есть глобальный `null`, он превращается
в особый объект и ломает проверки `x == nil`. Правило:

```lua
self.application_id = nil
self.form_id = nil
```

### 5.5. Имена

| Что | Правило | Пример |
|---|---|---|
| Поля instance | `snake_case` | `application_id`, `form_id`, `ui_file` |
| Методы Lua | `snake_case` канон | `set_property`, `open_modal_lua` |
| Qt-подобный sugar | `camelCase` | `setWindowTitle`, `setGeometry` |
| Виджеты | по `.ui`/конвенции | `tbl`, `btn_ok`, `btn_cancel`, `tbtn_add` |
| Табличные payload | один смысл — одно имя | `tableHeaders`, `tableData`, `tableLayout` |
| События | канон `Clicked` | fallback `MouseButtonRelease` только для совместимости |

Не смешивать `setProp`, `set_prop`, `setProperty` в одном слое. Канон —
`set_property`; `setProperty` — Qt-подобный алиас; `set`/`set_prop` —
совместимые обёртки.

### 5.6. Никаких глобалов

Плохо:

```lua
mode_title = "Измерения"   -- глобальная
counter = 0                -- глобальная
form_file = "..."          -- глобальная
```

Хорошо:

```lua
local form = require("form")()
local title = "Измерения"
local counter = 0
```

Если нужен shared state между формами — это не Lua-глобал, а host API или
явное хранилище с ключом `application_id`.

## 6. Sugar: что оставить и как не запутаться

### 6.1. Канонические методы

```lua
-- host API
create_form(application_id, form_id, ui_file_or_opts)
create_empty_form(application_id, form_id, opts)
set_property(application_id, form_id, object_name, property, value)
set_form_property(application_id, form_id, property, value)
get_property(application_id, form_id, object_name, property)
get_form_property(application_id, form_id, property)
open_modal_lua(application_id, form_id, lua_file, opts)
close_modal(application_id, form_id, result)
close_form(application_id, form_id)
close_application(application_id)
open_lua(application_id, lua_file, opts)
```

### 6.2. `Form` sugar

| Метод | Что делает | Статус |
|---|---|---|
| `Form:create(app_id, form_id, ui_or_opts)` | создать форму | есть, расширить opts |
| `Form:create_empty(app_id, form_id, opts)` | пустая форма | добавить |
| `Form:close()` | закрыть текущую | есть |
| `Form:open_modal_lua(lua_file, opts)` | открыть модалку | есть |
| `Form:finish_modal(result)` | закрыть модалку с результатом | есть |
| `Form:set_property(object, property, value)` | свойство виджета | есть |
| `Form:set_form_property(property, value)` | свойство формы | есть |
| `Form:set(a, b, c)` | 2/3-аргументный sugar | есть |
| `Form:set_prop(a, b, c)` | совместимый алиас | есть |
| `Form:get_property(object, property)` | чтение виджета | есть |
| `Form:get_form_property(property)` | чтение формы | есть |
| `Form:get_prop(a, b)` | совместимый алиас | есть |
| `Form:setWindowTitle(title)` | заголовок | есть |
| `Form:setGeometry(rect)` | геометрия | есть |
| `Form:setText(object, text)` | текст | есть |
| `Form:setVisible(object, visible)` | видимость | есть |
| `Form:setEnabled(object, enabled)` | доступность | есть |
| `Form:setChecked(object, checked)` | checked | есть |
| `Form:setToolTip(object, text)` | tooltip | есть |
| `Form:setIcon(object, icon)` | иконка | есть |
| `Form:resize(size_or_w, h)` | размер (`setFormProperty "size"`, позиция сохраняется) | есть |
| `Form:move(pos_or_x, y)` | позиция (`setFormProperty "position"`, размер сохраняется) | есть |
| `Form:setResizable(enabled)` | интерактивный ресайз (grip + границы) | есть |
| `Form:setSizeGripEnabled(enabled)` | Qt-совместимый алиас `setResizable` | есть |
| `Form:setMinimumSize(size_or_w, h)` | минимальный content-размер | есть |
| `Form:setMaximumSize(size_or_w, h)` | максимальный content-размер | есть |
| `Form:setCurrentRow(object, row)` | текущая строка | есть |
| `Form:setCurrentIndex(object, index)` | текущий индекс | есть |
| `Form:setToolButton(object, opts)` | настройка QToolButton | есть |
| `Form:on(widget, event, callback)` | обработчик | есть |
| `Form:handleEvent(widget, event)` | диспетчер | есть, добавить fallback |

Координаты `setGeometry`/`move`/`resize` — это **content rect** (клиентская
область MDI-подокна). Заголовок окна рисуется на 24 px выше, поэтому
`move(x, 0)` обрежет заголовок; для видимого заголовка нужно `y >= 24`.
`Form:move` сохраняет размер, `Form:resize` — позицию, `Form:setGeometry`
меняет и то, и другое.

Ресайз можно выключить флагом `Form:setResizable(false)` / `sizeGripEnabled=false`
или Qt-флагом `MSWindowsFixedSizeDialogHint` в `windowFlags` формы. Если
`minimumSize == maximumSize` (по обеим осям), grip тоже скрывается.
`minimumSize`/`maximumSize` задаются в content-координатах и ограничивают
и программный `setGeometry`, и перетаскивание границ. При наведении на
границу/угол клиент меняет курсор на соответствующий `*-resize`.


Важно: `setWindowTitle`/`setGeometry`/`setText` и т.п. не должны быть
«ещё одним протоколом». Они вызывают `set_property`/`set_form_property`.

### 6.3. `App` sugar

`App` — не форма и не отдельный процесс. Это набор операций над
приложением/главным окном:

| Метод | Что делает |
|---|---|
| `App.new()` | создать объект |
| `App:init(application_id)` | запомнить app id |
| `App:close()` | `close_application` |
| `App:open_lua(lua_file, opts)` | открыть немодальную форму |
| `App:open_modal_lua(parent_form_id, lua_file, opts)` | открыть модалку от имени приложения |
| `App:setWindowTitle(title)` | заголовок главного окна |
| `App:setIcon(icon)` | иконка |
| `App:resize(size_or_w, h)` | размер |
| `App:move(pos_or_x, y)` | позиция |
| `App:setGeometry(rect)` | геометрия |
| `App:set_menu(spec)` | меню главного окна (menu actions) |
| `App:setToolBar(spec)` | toolbar главного окна |
| `App:setToolBarVisible(bool)` | видимость toolbar |
| `App:setStatusMessage(text, timeout)` / `showMessage` | сообщение statusbar |
| `App:setStatusVisible(bool)` | видимость statusbar |
| `App:setStatusInfo(text)` | технический текст справа |

`App:exec_qt` и `Form:exec_qt` — **не канон**. Если оставлять, то только
как escape hatch с allowlist методов и без доступа к произвольным Elixir
функциям.

`App` sugar (статус 2026-11-25): `setWindowTitle`/`setIcon` → browser
chrome; `set_menu`/`setToolBar`/statusbar и top-level формы работают в
полноценном application shell (`?main=...`). `resize`/`move`/`setGeometry`/
`center`/`show*` для самого главного окна в браузере остаются no-op
(desktop-host); `visible("toolBar"/"statusbar")` лучше не использовать —
есть типизированные `setToolBarVisible`/`setStatusVisible`.

### 6.4. Таблицы

Клиент сейчас поддерживает:

- `horizontalHeaderLabels`
- `columnWidths`
- `hiddenColumns`
- `hideColumn`
- `tableHeaders`
- `tableData`
- `data`
- `selectRows`
- `readOnly`
- `currentRow`
- `currentRowCell:<column>`

Клиент поддерживает `tableLayout` / `groupedHeader` (см. `QTableWidget::apply_table_layout`
и `docs/agent-plans/10-tables-grouped-headers.md`). Для простых таблиц по-прежнему:

Поэтому для плоских заголовков:

```lua
form:setTableHeaders("tbl", {
  clear = true,
  columnCount = 3,
  horizontalHeaderLabels = {"A", "B", "C"},
  columnWidths = {120, 80, 200},
  hiddenColumns = {3},
})

form:setTableData("tbl", {
  rowCount = 2,
  data = {
    {"a1", "b1", "c1"},
    {"a2", "b2", "c2"},
  },
})
```

`set_table`/`set_table_from_objects`/`set_table_grouped_header` из старого
кода можно оставить как **совместимые обёртки**, но они должны собирать
только те payload, которые реально понимает клиент. Иначе форма «успешно»
отправит данные в никуда.

### 6.5. События

```lua
function Form:handleEvent(widget_name, event_type)
  local key = widget_name .. "|" .. event_type
  local handler = self.event_handlers[key]

  if not handler and event_type == "Clicked" then
    handler = self.event_handlers[widget_name .. "|MouseButtonRelease"]
  end

  if handler then
    handler()
  end
end
```

Это сохраняет старую совместимость: формы, которые подписывались на
`MouseButtonRelease`, продолжают работать, а новые используют `Clicked`.

## 7. Event loop и модальные формы

### 7.1. Схема

```
Клиент
  │  send_event: formEvent / getProperty reply
  ▼
PlatformChannel
  │  dispatch
  ▼
DialogSession (один GenServer на форму)
  │  Lua.onEvent
  ▼
Lua.Api
  │  set_property / open_modal_lua / close_modal
  ▼
PlatformChannel → PubSub → Клиент
```

Модалка:

```
A (L0)  ──open_modal_lua──►  B (L1)
  │                             │
  │ receive {:modal_result,...} │  работает в своём GenServer
  │                             │
  │◄──── send({:modal_result}) ─┘
  ▼
A продолжает обработку очереди
```

### 7.2. Инварианты

1. **Одна форма = один `DialogSession` = одна очередь событий.**
   GenServer mailbox и есть очередь; отдельный ETS-lock не нужен.
2. **Блокируется только родитель.** Пока A ждёт `open_modal_lua`, B и
   sibling-формы живут в своих процессах.
3. **Результат модалки — только сырой `send`/`receive`.**
   `GenServer.call`/`cast` в родителя, который уже стоит в `receive`, даёт
   дедлок.
4. **`getProperty` — сырой `ref` + `receive`.** Ответ приходит как
   `{:property_reply, ref, value}`; таймаут 5 секунд.
5. **Закрытие идемпотентно.** `notify_modal` отправляет результат один раз;
   `close_form`/`close_modal` ставят `:e_platform_closing` и останавливают
   сессию.
6. **Мониторинг родителей и детей.** Падение родителя закрывает модального
   ребёнка; падение ребёнка возвращает родителю `"error"`/`DOWN`.
7. **`closeApplication` останавливает все сессии**, включая вложенные.
8. **События закрывающейся формы отбрасываются**, чтобы не выполнять
   `onEvent` после `close`.
9. **`open_modal_lua` не должен терять родителя при ошибке старта.**
   `start_modal` и `receive` оборачиваются в `try/after`; при ошибке —
   `closeForm` и возврат `"error"`.
10. **`getProperty` внутри `init` модального ребёнка допустим**, потому что
    `DialogSession` уже зарегистрирован в `Registry` до `handle_continue`.

### 7.3. Что может «прилетать» и куда

| Сообщение | Откуда | Куда | Поведение |
|---|---|---|---|
| `formEvent` | клиент | `PlatformChannel` → `DialogSession` | `onEvent`; у закрытой формы отбрасывается |
| `getProperty` reply | клиент | `PlatformChannel` → `DialogSession` | сырой `{:property_reply, ref, value}` |
| `setProperty` | Lua | `PlatformChannel` → клиент | применить свойство виджета |
| `setFormProperty` | Lua | `PlatformChannel` → клиент | применить свойство формы |
| `setAppProperty` | Lua | `PlatformChannel` → клиент | `windowTitle`/`windowIcon` — применить; остальное браузер игнорирует |
| `createForm` | Lua | `PlatformChannel` → клиент | создать форму; `modal`/`mdi` |
| `closeForm` | Lua/сервер | `PlatformChannel` → клиент | закрыть форму |
| `closeApplication` | Lua | `PlatformChannel` → клиент | закрыть всё приложение |
| `dialogError` | сервер | `PlatformChannel` → клиент | ошибка `init`/`onEvent` |
| `{:modal_result, ref, result}` | ребёнок | родитель | сырой `send`, не `call` |
| `{:DOWN, ref, ...}` | монитор | родитель/ребёнок | каскадное закрытие |
| `{:property_reply, ref, value}` | канал | `DialogSession` | разблокировать Lua |
| `:stop_form` | `Lua.Api` | `DialogSession` | остановить сессию |

Подробная матрица по слоям — в `docs/modal-message-matrix.md`.

### 7.4. Текущие пробелы, которые надо закрыть

| Пробел | Где | Улучшение |
|---|---|---|
| `open_opts` хранится, но не передаётся в `init` | `DialogSession` | передавать единый `opts` в `init` |
| `Form:create` не принимает `opts` | `form.lua` | поддержать строку и таблицу |
| `create_empty_form`/`open_lua` отсутствуют | `Lua.Api` | добавить |
| `form_common_lib`/`icons`/`abstract_*` отсутствуют | `priv/lua_modules` | добавить |
| `close_form` не проверяет владельца | `Lua.Api` | проверять `form_id == self.form_id` |
| `open_modal_lua` не делает `try/after` | `Lua.Api` | гарантировать очистку и `closeForm` |
| `modal_result` — строка `"error"`/`"timeout"` | `Lua.Api` | опционально структурировать `{status, value}` |
| `handle_cast` для `getProperty` — мёртвый путь | `DialogSession` | удалить или явно задокументировать |
| `PlatformChannel.terminate` знает только top-level `form_ids` | `platform_channel.ex` | полагаться на монитор родителя или вести все сессии канала |
| `center` не поддержан клиентом | `rqt`/`app` | не добавлять sugar-пустышку |
| `tableLayout`/`groupedHeader` не поддержаны | `rqt` | не отправлять; сначала реализовать |

## 8. Тестовая матрица

### 8.1. Модули

- `require("form")` возвращает конструктор; `Form()` даёт независимые instance.
- `require("icons")` возвращает immutable-таблицу; изменение не течёт между формами.
- `form_common_lib.setup_form` не перетирает явные `opts`.
- `form_common_lib` не зависит от глобального `form`.
- `handleEvent` вызывает `Clicked` и fallback `MouseButtonRelease`.
- `set`/`set_prop`/`set_property` дают одинаковый результат.
- `get`/`get_prop`/`get_property` дают одинаковый результат.
- `setTableHeaders`/`setTableData` отправляют только поддерживаемые payload.
- `create` принимает строку и таблицу `opts`.

### 8.2. Модалки и event loop

- родитель копит события, пока открыта модалка, и разбирает их после;
- sibling принимает события и `getProperty` во время модалки;
- вложенные модалки A → B → C разматываются по слоям;
- `Close` от клиента у модального ребёнка возвращает `nil` родителю;
- падение родителя закрывает ребёнка;
- падение ребёнка возвращает `"error"` и `closeForm`;
- `closeApplication` во время модалки останавливает все сессии;
- `dialogError` при ошибке `init` модального ребёнка;
- `getProperty` из `init` модального ребёнка не дедлочит;
- повторный `close_form`/`finish_modal` не отправляет результат дважды.

### 8.3. Клиент

- `modal_stack` фильтрует hit-test и ввод;
- `createForm` с `modal: true` поднимает форму поверх;
- `closeForm` снимает форму со стека;
- `closeApplication` очищает все формы и стек;
- `getProperty`/`getFormProperty` отвечают на `ref`.

Правило тестов: не использовать `Process.sleep` для ожидания; применять
`assert_receive`, `Process.monitor`, `:sys.get_state` и явные таймауты.

## 9. План миграции

### Этап 1. Совместимость без изменения поведения

1. Добавить `form_common_lib.lua` и `icons.lua` в `server/priv/lua_modules`.
2. Добавить `abstract_table_form.ui` и `abstract_tree-table_form.ui`.
3. Перевести одну демо-форму на `form_common_lib`; остальные не трогать.
4. Добавить тесты на `require`, defaults и `handleEvent` fallback.

### Этап 2. Нормализация `Form`/`App`

1. `Form:create` принимает строку или `opts`.
2. `Form.new(opts)` не хранит глобальный `null`.
3. Добавить sugar `set`/`get`/`setWindowTitle`/`setText`/`setGeometry`/
   `setVisible`/`setEnabled`/`setChecked`/`setToolTip`/`setIcon`.
4. Добавить `App:init`, `App:open_lua`, `App:open_modal_lua`, `App:close`.
5. `Form:setProperty`/`setFormProperty` оставить как алиасы, но не делать
   их единственным путём.

### Этап 3. Host API и `opts`

1. `create_form` принимает `ui_file_or_opts`.
2. `create_empty_form` и `open_lua`.
3. `open_modal_lua` принимает `ui_file`, `mdi`, `modal`, `open_opts`,
   `title`, `geometry`.
4. `DialogSession` передаёт единый `opts` в `init`.
5. `close_form` проверяет владельца и идемпотентность.

### Этап 4. Модалки и cleanup

1. `open_modal_lua` обернуть в `try/after`.
2. При ошибке старта — `closeForm` и `"error"`.
3. При таймауте — остановить ребёнка и вернуть `"timeout"`.
4. `notify_modal` — ровно один раз.
5. Тесты на все сценарии из §8.2.

### Этап 5. Таблицы и динамический UI

1. `setTableHeaders`/`setTableData` на поддерживаемых payload.
2. `setTableLayout`/`groupedHeader` — только после реализации в клиенте.
3. `addLayout`/`addWidget`/`connectSignal` — только если действительно
   нужны; иначе не тащить.
4. `invoke_qt` — только allowlist, не общий мост.

### Этап 6. Данные

`list_*` не добавлять, пока нет слоя данных. Если появится:

```lua
local data = require("data")
local rows = data.list_measurements({limit = 100})
```

Один типизированный модуль лучше десяти `list_*`-функций в host API.

## 10. Решения и открытые вопросы

### Принятые решения

1. Наследование не используем; только композиция и базовые `.ui`.
2. Один metatable на `Form`/`App`.
3. `form_common_lib` — mixin, а не базовый класс.
4. `icons` — чистая таблица.
5. `open_opts`/`init_opts` унифицируются в один `opts`.
6. `mdi`/`modal` — явные boolean; `mode` не используем.
7. `Form:center` — свойство формы `centered` (центрирование MDI-подокна);
   `App:center` (главное окно) не добавляем. Клиентскую поддержку
   `centered` нужно реализовать до включения sugar.
8. `null` не используем; только `nil`.
9. Глобальные переменные в формах запрещены.
10. Модальный результат — сырой `send`, не `GenServer.call`.
11. `getProperty` — сырой `ref` + `receive` + таймаут.
12. `invoke_qt` не основной путь; при необходимости — allowlist.
13. `mfa` не переносим.
14. `list_*` не переносим без слоя данных.
15. `create_empty_form` не нужен: формы всегда создаются из `.ui`
    (решение 2026-11-25).
16. `modal_result` — структура `{status, value}`
    (`ok|closed|error|timeout`); `Form:open_modal`/`App:open_modal` —
    канонические, `*_lua` — совместимые обёртки (решение 2026-11-25).
17. `App` sugar: `setWindowTitle`/`setIcon` → `setAppProperty`
    (`windowTitle`/`windowIcon`; браузер применяет `document.title` и
    favicon). `resize`, `move`, `setGeometry`, `center`, `show*`,
    `visible`, `set_menu` объявлены в Lua и передаются как
    `setAppProperty`, но браузерный клиент их игнорирует — задел под
    desktop-host с нативным окном вокруг встроенного браузера.

**Обновление 0.14 (2026-11-25).** Решение 17 пересматривается:
`set_menu`, statusbar и top-level формы становятся частью реального
application shell. No-op в браузере остаётся только у геометрии
главного окна (`resize`/`move`/`setGeometry`/`center`/`show*` для
самого main window). План и принятые решения D1–D8 — в
`docs/application-shell-plan.md` и `questions.md`.

### Открытые вопросы

1. `Form:center`: рекомендация — реализовать свойство `centered` в клиенте
   (центр MDI-подокна), тогда sugar заработает без изменений Lua.
2. ~~Структура `modal_result`?~~ Закрыт 2026-11-25: структурный подход
   `{status, value}` (решение 16).
3. `open_lua` уже реализован; вопрос снят.
4. ~~`create_empty_form`?~~ Закрыт 2026-11-25: не нужен (решение 15).
5. ~~`invoke_qt`?~~ Закрыт: не тащим (решение 12).
6. Как поступать с `dialogError` у модального ребёнка: возвращать
   `"error"` родителю или отдельный `{:error, reason}`? Согласовать
   со структурным `modal_result` (status `error` + причина в `value`).
7. Нужен ли `tableLayout`/`groupedHeader` в клиенте, или старые формы
   можно упростить до `tableHeaders`/`tableData`?
8. `App` как объект оставляем (`init/close/open_lua/open_modal_lua`);
   sugar не добавляем (решение 17).

## 11. Итоговая схема

```
┌──────────────────────────────────────────────────────────────┐
│ L3: прикладные формы                                         │
│   measurements.lua, projects.lua, users.lua ...              │
│   local form = require("form")()                             │
│   require("form_common_lib").setup_form(form, opts)          │
└──────────────────────────────┬───────────────────────────────┘
                               │ host API
┌──────────────────────────────▼───────────────────────────────┐
│ L1: платформенные Lua-модули                                 │
│   form.lua  application.lua  icons.lua  form_common_lib.lua  │
│   table_form.lua (опционально)                               │
└──────────────────────────────┬───────────────────────────────┘
                               │ Elixir functions
┌──────────────────────────────▼───────────────────────────────┐
│ L0: host API EPlatform.Lua.Api                               │
│   create_form / create_empty_form / open_lua                 │
│   set_property / set_form_property / get_property / ...      │
│   open_modal_lua / close_modal / close_form / ...            │
└──────────────────────────────┬───────────────────────────────┘
                               │ GenServer
┌──────────────────────────────▼───────────────────────────────┐
│ DialogSession (одна форма = один процесс)                    │
│   mailbox = очередь событий                                  │
│   modal: parent/ref/monitor, raw send/receive                │
└──────────────────────────────┬───────────────────────────────┘
                               │ PubSub / Channel
┌──────────────────────────────▼───────────────────────────────┐
│ Клиент: modal_stack, MDI, QWidgetTree, wgpu                  │
└──────────────────────────────────────────────────────────────┘
```

Главная мысль: **не тащить в Lua-модули OTP-механику и не тащить в
Elixir-слой прикладную иерархию форм**. Композиция в Lua, изоляция в
GenServer, сырые сообщения только там, где иначе будет дедлок.

## 12. Статус реализации

Обновлено 2026-11-25 по коду и тестам (`mix test` — 29 passed).

### Реализовано

- `form_common_lib.lua`, `icons.lua`, `test_common.lua`;
- `abstract_table_form.ui` и `abstract_tree_table_form.ui`;
- `form.lua`: `set`/`get`, `setWindowTitle`, `setGeometry`, `move`,
  `resize`, `setResizable`/`setSizeGripEnabled`, `setMinimumSize`,
  `setMaximumSize`, `setText`, `setVisible`, `setEnabled`, `setChecked`,
  `setToolTip`, `setIcon`, `setToolButton`, `setCurrentRow`,
  `setCurrentIndex`, `set_table*`, `setTableHeaders`, `setTableData`;
  `handleEvent` понимает `Clicked` и `MouseButtonRelease`;
- **модалки**: `open_modal_lua`, `close_modal`, `close_form` с сырым
  `send`/`receive`, монитором родителя/ребёнка и таймаутами
  (`lua/api.ex`, `dialog_session.ex`; матрица — `modal-message-matrix.md`);
- **`open_opts` → `init`**: `DialogSession` передаёт опции четвёртым
  аргументом `init(app_id, form_id, ui_file_or_nil, open_opts)`
  (`dialog_session.ex`), покрыто тестом `open_opts reach init/4`;
- **`open_lua`**: немодальный ребёнок через `DialogManager.start_dialog`
  с `owner`-монитором; закрывается вместе с каналом/родителем;
- `App`: `init`, `close`, `open_lua`, `open_modal_lua`;
- **`App` browser chrome**: `setWindowTitle`/`setIcon` → `set_app_property`
  → `setAppProperty`; клиент ставит `document.title`/favicon (включая
  legacy `img24:N` из встроенного спрайта); остальные
  App-операции принимаются протоколом и игнорируются браузером
  (задел под desktop-host);
- e2e-тесты `server/test/e_platform/lua_modules_test.exs` и
  `modal_flow_test.exs` (25 тестов суммарно по серверу).

### Ещё не реализовано

- структурный `modal_result` (`{status, value}`) — решение принято,
  реализация впереди (совместимые обёртки обязательны);
- клиентское свойство `centered` для `Form:center` (сам sugar уже есть в
  `ref_qt`, но не в текущем `form.lua`);
- `tableLayout`/`groupedHeader` в клиенте;
- слой данных (`list_*`/`data`).

Не планируется (решения 2026-11-25): `create_empty_form`, `App` sugar,
`invoke_qt`/`mfa`. Открытые решения — в `questions.md` (раздел B).
