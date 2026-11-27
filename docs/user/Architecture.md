# Architecture

Многослойная система: **Lua описывает поведение**, **сервер держит сессии**, **клиент рисует и шлёт события**.

```
┌─────────────────────────────┐         Phoenix Channels          ┌─────────────────────────────┐
│  browser client (wasm)      │ ◄──── WS /socket/websocket ─────► │  server (Elixir / Phoenix)  │
│  form-xml → rqt → app/gfx   │         topic app:<id>            │  DialogSession + Lua (luerl)│
└─────────────────────────────┘                                   └─────────────────────────────┘
```

## Слои (сверху вниз)

| # | Слой | Где | Ответственность |
|---|------|-----|-----------------|
| 1 | Скрипт приложения | `priv/lua_modules/*.lua` | бизнес-логика, обработчики `form:on` |
| 2 | Платформенный Lua | `form.lua`, `application.lua` | сахар над host API |
| 3 | Host API | `EPlatform.Lua.Api` | `create_form`, `set_property`, модалки… |
| 4 | Сессия формы | `DialogSession` GenServer | один процесс ≈ одна открытая форма |
| 5 | Канал | `PlatformChannel` | join `app:*`, `start_dialog`, `send_event`, push `command` |
| 6 | Клиентский протокол | `app/src/socket.rs` | Phoenix v2 JSON frames |
| 7 | Shell / формы | `app` | MDI, menu/toolbar, карта форм, event loop |
| 8 | Виджет-рантайм | `rqt` | дерево, layout, style/QSS, hit-test, paint commands |
| 9 | Парсер `.ui` | `form-xml` | XML → спецификация виджетов |
| 10 | Пиксели | `gfx` (wgpu) + `raster` (CPU golden) | GPU в браузере; CPU для тестов vs Qt |

Подробный инженерный разбор: [`docs/architecture-ru.md`](https://github.com/VitaSound/EPlatform/blob/main/docs/architecture-ru.md).

## OTP на сервере (идея)

```
EPlatform.Supervisor
├── PubSub / Endpoint
├── DialogRegistry
└── DialogSupervisor  →  DialogSession × N
```

Крах одной формы не роняет остальные. Дети (`open_lua` / модалки) следят за родителем через monitor и закрываются каскадом.

## Сквозной путь «клик по кнопке»

```
Клиент                  Channel                 DialogSession              Lua
  │ formEvent Clicked      │                        │                       │
  │───────────────────────►│ cast dispatch          │                       │
  │                        │───────────────────────►│ onEvent ─────────────►│
  │                        │                        │◄── getProperty ───────│
  │◄──── command getProp ──│◄── broadcast ──────────│                       │
  │ send_event reply       │ reply_property         │ продолжает            │
  │───────────────────────►│───────────────────────►│                       │
  │◄──── setProperty ──────│◄── broadcast ──────────│ set_property ◄────────│
  │ relayout + repaint     │                        │                       │
```

Модальные результаты идут **сырым** `send`/`receive`, не через `GenServer.call` — иначе вложенные модалки дедлокнутся.

## Режимы клиента

| Режим | Как открыть | UI |
|-------|-------------|-----|
| **full** | `?main=…` | MainWindow-аналог: menu + toolbar + MDI + status |
| **standalone** | `?lua=…` | без главного окна; форма top-level |

Формы: `placement=mdi` в центральной области или `top-level`; `modal` независим от MDI.

## Что не является слоем

- Нет настоящего Qt / moc / QObject C++.
- Нет отдельного «дизайнерского рантайма» на сервере — `.ui` уходит клиенту как XML в `createForm`.
- Отрисовка не на сервере: сервер не знает пикселей.

См. также [[Rendering Pipeline]] и [[Supported Widgets]].
