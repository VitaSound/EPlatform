# Архитектура EPlatform

Проект состоит из трёх частей:

| Часть | Что это | Стек |
|---|---|---|
| `server/` | Сессионный сервер: Phoenix-каналы + рантайм Lua-диалогов | Elixir / OTP, luerl |
| `client/` | Десктопный клиент — чистовая реализация Qt-семантики | Rust, winit + wgpu |
| `docs/` | ADR и спецификации (event-loop, Lua-модули, матрица модалок) | Markdown |

```
┌──────────────────────────┐          Phoenix Channels v2           ┌──────────────────────────┐
│  client/ (Rust workspace)│  ◄────── WS ws://…:4000/socket ──────► │  server/ (Phoenix/Elixir) │
│  form-xml · rqt · app    │        JSON ["1",ref,topic,…]          │  EPlatform · EPlatformWeb │
└──────────────────────────┘                                        └──────────────────────────┘
```

---

## 1. Сервер (`server/`)

### OTP-дерево

`server/lib/e_platform/application.ex:10-18`:

```
EPlatform.Supervisor
├── EPlatformWeb.Telemetry
├── DNSCluster
├── Phoenix.PubSub        (EPlatform.PubSub)
├── Registry              (EPlatform.DialogRegistry,  unique-ключи)
├── DynamicSupervisor     (EPlatform.DialogSupervisor)  ← одна сессия = одна форма
└── EPlatformWeb.Endpoint (socket "/socket", WS без check_origin)
```

Ключевая идея: **одна открытая форма = один GenServer** (`DialogSession`).
Крах одной формы никогда не задевает остальные.

### Модули ядра

| Модуль | Роль |
|---|---|
| `dialog_manager.ex` | Публичное API над DynamicSupervisor: `start_dialog/4`, `start_modal/4`, `whereis/2`, `dispatch/3`, `reply_property/4`, `stop/2`, `stop_application/1` |
| `dialog_session.ex` | GenServer на форму, зарегистрирован через `{:via, Registry, {DialogRegistry, {app_id, form_id}}}`. Владеет Lua-состоянием, запускает `init` в `handle_continue(:boot)`, форвардит клиентские события в Lua `onEvent`, мониторит модального/верхнеуровневого родителя |
| `lua/logic.ex` | Компиляция Lua: `Lua.new(exclude: [[:package], [:require]]) |> set_lua_paths(priv/lua_modules) |> load_api(EPlatform.Lua.Api)` — `require` доступен, скрипты подключают соседние модули |
| `lua/api.ex` | Хост-API для Lua (`deflua`): `create_form`, `set_property`, `get_property` (блокирующий `receive` на 5 с), `open_modal_lua` (блокирующий `receive {:modal_result,…}` на 300 с), `open_lua`, `close_modal`, `close_application`. Выполняется **внутри** сессии, поэтому process dictionary используется как контекст |
| `id.ex` | Генерация id из `:crypto.strong_rand_bytes` (без Ecto) |

### Web-слой (`lib/e_platform_web/`)

- `channels/user_socket.ex` — `channel "app:*", PlatformChannel`.
- `channels/platform_channel.ex` — **один канал на приложение**:
  - `join("app:" <> id)` — подписка на PubSub-топик `app_events:<id>`, `form_ids` в assign;
  - `handle_in("start_dialog")` — старт сессии, reply `{form_id}`;
  - `handle_in("send_event")` — маршрутизация: `getProperty`-ответ → `reply_property`, остальное (обычно `formEvent`) → `dispatch` (cast в сессию);
  - `handle_info(%Broadcast{event: "command"})` — трансляция `push`-сообщений клиенту;
  - `terminate/2` — останавливает все сессии, открытые этим каналом (вкладку закрыли).

### Сценарий «клик по кнопке» (сквозной)

```
Клиент                    PlatformChannel              DialogSession (GenServer)        Lua
  │ send_event formEvent        │                              │                        │
  │ ──────────────────────────► │ cast {:dispatch,…}           │                        │
  │                             │ ───────────────────────────► │ onEvent(widget,event) ►│
  │                             │                              │ ◄── getProperty ───────│
  │ ◄──── command getProperty ──│ ◄── broadcast("app_events:") │  (блок в receive)      │
  │ send_event getProperty      │                              │                        │
  │ ──────────────────────────► │ reply_property (raw send)    │ {:property_reply,…}    │
  │                             │ ───────────────────────────► │ ◄────────────────────  │ продолжает
  │ ◄──── command setProperty ──│ ◄── broadcast                │ set_property ►         │
  │ repaint через wgpu          │                              │                        │
```

### Lua-модули (`priv/lua_modules/`)

| Модуль | Роль |
|---|---|
| `form.lua` | Объект Form: `create/close`, `setProperty/getProperty`, сахар (`setText`, `setChecked`…), реестр обработчиков `form:on(...)` + `handleEvent` |
| `application.lua` | Объект App: `close`, `open_lua` (не-модальный ребёнок), `open_modal_lua` (блокирующая модалка) |
| `form_common_lib.lua` | `setup_form(form, opts)` — дефолты для абстрактных форм |
| `icons.lua`, `mathlib.lua` | Карта иконок, демонстрация `require` |
| `test_*.lua` | Фикстуры тестов (модалки, вложенность, open_opts) |
| `priv/qt_forms/*.ui` | Qt Designer XML — уходит клиенту как есть в `createForm.ui_xml` |

Слои Lua (см. `docs/lua-module-composition.md`): **хост-API (Elixir) → платформенные модули → скрипты приложений**, без наследования.

### Жизненный цикл сессии и очистка

```
клиент закрыл вкладку
        │
PlatformChannel.terminate
        │ DialogManager.stop(form_id)   ── DynamicSupervisor.terminate_child
        ▼
DialogSession родителя погибает
        │ Process.monitor(owner) срабатывает
        ▼
open_lua-дети получают {:DOWN, owner,…} → {:stop, :normal}
        │ (каскад, внуки — аналогично)
        ▼
все формы приложения остановлены
```

Важный инвариант: остановку детей нельзя делать вызовом
`DynamicSupervisor.terminate_child` из `terminate/2` самого родителя —
супервизор в этот момент ждёт завершения родителя и не обработает вызов
(взаимоблокировка). Поэтому используется **монитор владельца**.

---

## 2. Клиент (`client/`)

Cargo-workspace без настоящего Qt — это чистовая реализация Qt-семантики
(см. `client/QT_PORT_PLAN.md`):

```
client/
├── crates/form-xml   — .ui XML → дерево спецификаций виджетов (roxmltree, PropValue)
├── crates/rqt        — «Qt-рантайм»: QObject-арена, layout/size policy,
│                       QSS-подмножество, Fusion-стиль, MDI, QTableWidget,
│                       CPU-растеризатор (golden-PNG тесты против Qt6)
└── crates/app        — оболочка: окно winit, рендер wgpu/glyphon,
                        Phoenix-клиент, карта форм, модальный стек, MDI
```

- Транспорт — **минимальный Phoenix Channels v2 клиент** (`app/src/socket.rs`),
  JSON-массив `[join_ref, ref, topic, event, payload]`, heartbit 25 с;
  по умолчанию `ws://127.0.0.1:4000/socket/websocket`, топик `app:<id>`.
- После join-ack клиент сразу шлёт `start_dialog {lua_file, ui_file}`.
- Не Qt-биндинги: `rqt` пере реализует семантику Qt, `app` растеризует paint-команды через wgpu; точность проверяется golden-PNG против Qt6 offscreen.

### Application shell (этап 0.14)

Клиент поддерживает два режима:

- **full** — `?main=<script>`: main-сессия (`role=main`) и фиксированная
  оболочка `QMenuBar` + `QToolBar` + central `QMdiArea` + `QStatusBar`;
  формы с `placement=mdi` встраиваются в центральную область, формы с
  `placement=top-level` живут отдельным слоем;
- **standalone** — `?lua=<script>`: главного окна нет, форма открывается
  top-level; `mdi=true` в этом режиме коэрсится в top-level.

`App:set_menu`, `App:setToolBar`, `statusMessage` и menu actions
обрабатываются main-сессией; `mdi` и `modal` — независимые флаги.
Подробный план/статус: `docs/application-shell-plan.md`.

### Обработка команд сервера (`app/src/lib.rs`, `on_command`)

| Команда | Действие клиента |
|---|---|
| `createForm` | `load_ui(ui_xml)` → дерево виджетов, карточка `QMdiSubWindow`, в `modal_stack` если `modal` |
| `setProperty` / `setFormProperty` | `tree.set_property(...)` + relayout |
| `setAppProperty` | `windowTitle` → `document.title`, `windowIcon` → favicon; остальные свойства игнорируются (desktop-host) |
| `getProperty` / `getFormProperty` | читает локально и отвечает `send_event {command:"getProperty", ref, value}` |
| `closeForm` / `closeApplication` | убрать форму / все формы |
| `dialogError` | лог |

События наружу: `send_event {command:"formEvent", widget_name, event}`
(`Clicked`, `MouseButtonRelease`, `Close`, `CellChanged`, `CurrentRowChanged`…).
Ввод в QLineEdit — локальный; Lua забирает текст блокирующим `getProperty`.

---

## 3. Протокол (кратко)

Топик: **`app:<application_id>`**.

Клиент → сервер:
- `start_app {lua_file, open_opts?}` → reply `{form_id, role:"main"}`;
- `start_dialog {lua_file, ui_file, mode?, placement?, modal?, modal_scope?, open_opts?}` → reply `{form_id}`;
- `send_event` c `formEvent`/`menuAction` или ответом на `getProperty`;
- `phxheartbeat` на топике `"phoenix"`.

Сервер → клиент (всё на событии `command`, payload содержит `application_id` и `form_id`):
`createForm {ui_xml, role?, mode?, placement?, modal_scope?}`,
`setProperty`, `setFormProperty`, `setAppProperty` (menu/toolBar/status),
`getProperty` (запрос), `closeForm`, `closeApplication`, `dialogError`.

Модальные результаты ходят **только сырым `send`/`receive`**
(`{:modal_result, ref, result}`), никогда `GenServer.call` — иначе вложенные
модалки дедлокнутся (см. `lua-event-loop-and-modal-forms.md`,
`modal-message-matrix.md`).
