# Этап 0.14 — Application shell: главное окно, меню, MDI/top-level, statusbar

Статус: ✅ 0.14.1–0.14.9 реализованы (2026-11-25); Qt6 golden
menu/toolbar/status/central и браузерные сценарии full/standalone/
top-level/modal проверены.

Дата: 2026-11-25.
Связанные документы: `widgets-plan.md`, `EPlan_Status.md`,
`questions.md`, `docs/lua-module-composition.md` §6.3/§10,
`docs/architecture-ru.md`.

---

## 1. Цель

Сделать EPlatform-клиент не «одной MDI-областью на весь canvas», а
аналогом Qt-приложения с `QMainWindow`:

- главное окно с меню, центральной `QMdiArea` и техническим statusbar;
- формы — дочерние окна приложения;
- формы могут быть MDI или самостоятельными top-level окнами;
- модальность — отдельная ось, не связанная с MDI;
- меню строится из Lua (как `qt_set_menu` в `tcs_qtgui`);
- сохраняется старый сценарий запуска **без главного окна**, когда
  открывается только конкретная форма.

Главное окно при этом концептуально является «такой же формой»: у него
есть `QWidgetTree`, свойства, title/icon, при необходимости — `.ui`.
Но это **корневое окно приложения**, а не `QMdiSubWindow` и не обычная
модалка.

---

## 2. Три независимые оси

| Ось | Значения | Смысл |
|---|---|---|
| `role` | `main` / `form` | жизненный цикл, владелец application-scope |
| `placement` | `mdi` / `top-level` | где рисуется окно |
| `modality` | `none` / `window` / `application` | что блокирует ввод |

`mdi` и `modal` — разные флаги. Это уже было так в старой реализации:
`TCSCore::sendStartDialog(..., modal, mdi)` передавал их отдельно.

Дополнительно режим запуска:

| `mode` | Что запускается |
|---|---|
| `full` | main-сессия + главное окно (menu/MDI/statusbar); формы открываются из меню/API |
| `standalone` | главное окно не показывается; стартует конкретная форма как top-level |

### 2.1. Принятые решения (2026-11-25)

| # | Решение |
|---|---|
| D1 | **Web:** строгий standalone без main-оболочки; `mdi=true` коэрсится в top-level. Скрытая оболочка/материализация — только будущий desktop-host, не 0.14. |
| D2 | `?main=<main.lua>` → full (`start_app`); `?lua=<form>&ui=` → standalone (`start_dialog`); `?mode=full&lua=` — явный full. `?app=<uuid>` остаётся id приложения. Текущие URL остаются standalone. |
| D3 | **A:** фиксированная клиентская оболочка (menu/toolbar/status/MDI), меню и toolbar из Lua. `QMainWindow.ui` — отдельный 0.14.8; `QToolBar` обязателен и входит в 0.14.7. |
| D4 | **A:** top-level окна — in-canvas слой внутри одной вкладки. |
| D5 | **A:** main заполняет viewport, его geometry — no-op; standalone-форма использует `.ui`-геометрию (clamp в viewport). |
| D6 | **A:** `QStatusBar` с transient message и постоянными техническими полями; сообщения — Lua, технические поля — клиент. |
| D7 | **A:** menu actions выполняет main-сессия через `App:open_lua`/`open_modal`/`App:close`. |
| D8 | Full: закрытие main закрывает приложение; standalone: последнее top-level окно завершает приложение; одна из нескольких форм — не завершает. |

---

## 3. Старое поведение (`tcs_qtgui`)

Это важно сохранить, а не переизобрести.

`tcs_qtgui/main.cpp`:

- `--start=<lua>` — стартовый Lua-скрипт;
- `--show-mainwindow` / `--hide-mainwindow`; по умолчанию
  `showMainWindow = false`;
- `--embed-mdi` — отдельный флаг (в этой версии `MainWindow` его
  сохраняет, но `onCreateForm` фактически не использует; решение об MDI
  приходит с сервера в `mdi`);
- при `--hide-mainwindow --embed-mdi` флаг MDI принудительно выключается.

`TCSCore` при старте делает:

```cpp
sendStartDialog(application_id, lua_file, "", false, false);
```

То есть первый скрипт открывается как **top-level** (`mdi=false`,
`modal=false`). Поэтому при скрытом главном окне видно только окно формы.

`MainWindow::onCreateForm`:

- `mdi=true` → `QMdiArea::addSubWindow`; если главное окно было скрыто,
  вызывается `show()` — MDI-форма материализует главное окно;
- `mdi=false` → `Qt::Window`, отдельное top-level окно;
  `setWindowModality(modal ? Qt::ApplicationModal : Qt::NonModal)`.

`--embed-mdi` в текущем коде ни на что не влияет; не переносим его как
отдельную сущность.

Итого старый standalone — это не «другой сервер», а **скрытая главная
оболочка + первый скрипт с `mdi=false`**. Формы остаются окнами
приложения; `close_application` завершает всё.

---

## 4. Целевая модель

### 4.1. Full mode

```text
Application (application_id)
├─ Main session / role=main
│  ├─ MainWindow shell
│  │  ├─ QMenuBar        ← App:set_menu(spec)
│  │  ├─ QToolBar        ← App:setToolBar(spec)
│  │  ├─ central QMdiArea ← формы placement=mdi
│  │  └─ QStatusBar      ← техническая информация
│  └─ top-level forms    ← placement=top-level, owner=main
└─ modal forms           ← modal_scope=window|application
```

### 4.2. Standalone mode (web)

```text
Application (application_id)
└─ одна/несколько form-сессий
   └─ top-level окна, без меню/statusbar/MDI и без main-оболочки
```

`mdi=true`, пришедший в standalone, коэрсится в `top-level` с записью в
лог. Скрытая main-оболочка и её материализация при MDI — поведение
старого desktop-клиента; в веб-варианте оно не реализуется. Если позже
появится desktop-host, он сможет включить его отдельным режимом.

### 4.3. Кто чем владеет

- **main-сессия** владеет top-level формами приложения;
- **модальная форма** владеет своим модальным ребёнком
  (`modal_parent`/`owner`), чтобы работал каскад и `modal_result`;
- **MDI** — это только размещение; MDI-форма не обязана быть модальной;
- закрытие main в full mode = закрытие приложения; закрытие последнего
  top-level окна в standalone = завершение приложения.

Важно: «дочерняя» форма — это логическое владение/размещение, а не
обязательно `QObject::parent`. Top-level окно может иметь родителя, но
оставаться отдельным окном (как в Qt).

---

## 5. Текущее состояние EPlatform (факты)

| Что | Сейчас |
|---|---|
| главное окно | нет; canvas целиком занят MDI-областью |
| `mdi` | клиент всегда делает `QMdiSubWindow`; флаг не читается |
| `mdi` с сервера | `create_form` добавляет `mdi` только для модальных форм |
| App title/icon | работают; `menu` и остальное игнорируются |
| `App:set_menu` | spec уходит в `setAppProperty`, клиент его не рисует |
| меню/statusbar | моделей `QMenuBar`/`QStatusBar` нет |
| `QMainWindow` в `.ui` | `form-xml` читает только корневой `<widget>` и его `<widget>/<layout>`; `QMenuBar`/`QStatusBar`/`<action>` рядом с корнем игнорируются |
| MDI viewport | весь canvas; `add_subwindow` ставит окна в абсолютные `(32,28)` |
| модальность | `modal_stack` + фильтр только по MDI; меню/statusbar/top-level не блокируются |
| top-level слой | отсутствует |
| владелец `open_lua` | вызывающая форма, а не main |

Ключевые места:

- `server/lib/e_platform/lua/api.ex` — `create_form`, `open_lua`,
  `set_app_property`;
- `server/lib/e_platform/dialog_session.ex` — сессия формы, `mdi/modal`;
- `client/crates/app/src/lib.rs` — `forms`, `mdi`, `modal_stack`,
  `AppChrome`, `on_create_form`;
- `client/crates/rqt/src/mdi.rs` — `QMdiArea`/`QMdiSubWindow`;
- `client/crates/form-xml/src/lib.rs` — парсер `.ui`;
- `client/crates/rqt/src/widgets.rs` — `load_ui`.

---

## 6. Требования

### R1. Full mode

- main-сессия одна на `application_id`;
- главное окно показывает menu, toolbar, central MDI area и statusbar;
- main использует фиксированную клиентскую оболочку; menu/toolbar/status;
  `QMainWindow.ui` реализован в 0.14.8 (static menu/toolbar/status
  вытесняются динамическими `set_menu`/`setToolBar`);
- `App:setWindowTitle`, `setIcon`, `set_menu`, `setToolBar` принадлежат
  main;
- `App:open_lua` из main создаёт top-level форму с owner=main;
- закрытие main завершает приложение.

### R2. Standalone mode

- главное окно не рисуется;
- первый скрипт открывается как top-level;
- меню/statusbar/main-оболочка отсутствуют;
- `mdi=true` коэрсится в `top-level` (решение D1, веб-вариант);
- несколько top-level форм допускаются;
- закрытие последнего окна завершает приложение.

### R3. Меню

Поддержать spec старого `qt_set_menu`:

```lua
{
  title = "Справочники",
  children = {
    { title = "Единицы измерения", action = "open_lua",
      lua_file = "measurements.lua", mdi = true, icon = "img24:27" },
    { separator = true },
    { title = "Выход", action = "exit" },
  }
}
```

- вложенность, `separator`, `enabled`, icon, `title == "-"`;
- `action = "exit"` и `action = "open_lua"`;
- `lua_file`, `mdi`, `modal`; `ui_file` опционально (старый spec его не
  содержит — скрипт сам решает, какой `.ui` открыть);
- actions выполняются **main-сессией**, а не клиентом напрямую
  (решение D7);
- popup, hover, disabled, z-order; shortcuts — позже.

### R4. Top-level окна

- `placement=top-level` не попадает в `QMdiArea`;
- собственный title bar, активация, z-order, drag, resize, close;
- общая рамка/отрисовка с MDI, но отдельный слой;
- в браузере — in-canvas слой (решение D4), не `window.open`;
- несколько окон одновременно.

### R5. Модальность

- `window-modal` блокирует владельца и его ancestors;
- `application-modal` блокирует menu, statusbar, MDI и все top-level;
- фокус возвращается после закрытия;
- серверная блокировка родителя (`open_modal_lua`) не меняется;
- sibling-сессии на сервере продолжают жить; клиент просто не даёт
  пользователю вводить в заблокированные окна.

### R6. Statusbar

- модель `QStatusBar`: transient message + постоянные технические поля;
- `App:showMessage(text, timeout)` / `statusMessage` / `statusVisible`;
- технические поля (соединение, application id, число форм, версия)
  может заполнять клиент; Lua — сообщения;
- statusbar не перекрывает MDI; central viewport уменьшается.

### R7. Совместимость

- существующие формы/скрипты не должны знать о mode/role;
- `?lua=<form>.lua&ui=<...>` остаётся standalone-сценарием (решение D2);
- старый menu spec можно передать без изменений;
- `modal_result`, `open_lua`, `open_modal_lua` сохраняют поведение.

### R8. Toolbar

`QToolBar` — обязательный виджет, а не опция. Он входит в 0.14.7.

- модель `QToolBar` + `QAction` (общая с меню);
- верхняя область по умолчанию; высота/отступы как в Fusion;
- actions: icon, text, tooltip, `enabled`, `separator`, `checkable`;
- `App:setToolBar(spec)` / `setAppProperty("toolBar", spec)` и
  `toolbarVisible`;
- action dispatch — тот же путь, что у меню (main-сессия);
- статический toolbar из `QMainWindow.ui` — ✅ 0.14.8;
- overflow/extension, drag между областями и кастомизация — позже,
  но модель должна допускать `Top/Bottom/Left/Right`.

---

## 7. Этапы

### 0.14.1 Домен и протокол (S/M)

- ввести `LaunchMode { Full, Standalone }`,
  `WindowRole { Main, Form }`,
  `Placement { Mdi, TopLevel }`,
  `ModalScope { None, Window, Application }`;
- `createForm` несёт `role`, `placement`, `modal`, `modal_scope` для
  **всех** форм; починить потерю `mdi` у немадальных;
- `setAppProperty` несёт `source_form_id`/role; менять меню/title может
  только main;
- команда `start_app` для main и `start_dialog` для standalone/форм;
  уникальность main на `application_id`;
- закрытие main → `close_application`;
- тесты: `mdi=false` доходит до клиента; main один; owner/cascade.

**DoD:** серверные тесты на mode/role/placement; клиентский парсер
payload; без изменений UI.

### 0.14.2 Main shell и режимы запуска (M)

- `MainWindow` в клиенте: menu placeholder, central rect, status
  placeholder, `QMdiArea`;
- MDI viewport = central rect (учесть menu/status), позиции подокон
  относительно origin;
- full mode: главное окно + одна форма в MDI;
- standalone: без shell, форма top-level;
- закрытие main/last window;
- тесты: viewport offset, hit-test, full/standalone.

**DoD:** full и standalone запускаются; формы не заезжают под меню;
старые тесты зелёные.

### 0.14.3 Меню из Lua (M)

- `QMenuBar`/`QMenu`/`QAction` в `rqt`: модель, rects, popup, painting;
- `App:set_menu`/`setAppProperty("menu")` → модель;
- action id/path, `enabled`, `separator`, submenu, icon;
- `menuAction` → main-сессия → `App:open_lua`/`open_modal`/`exit`;
- тесты на старый spec `test_app_01.lua`;
- `action=open_lua`: `ui_file` опционален; скрипт открывает свой `.ui`.

**DoD:** меню рисуется, клики открывают MDI/top-level формы, exit
закрывает приложение.

### 0.14.4 Top-level слой (M)

- `WindowFrame`/top-level stack: z-order, activation, drag, resize,
  close, title;
- `placement=top-level`; несколько окон;
- MDI и top-level могут сосуществовать в full mode;
- standalone использует только top-level;
- тесты: две top-level формы, перекрытие, активация, закрытие.

**DoD:** `mdi=false` реально даёт отдельное окно; MDI-формы остаются в
central area.

### 0.14.5 Application/window modal (M)

- `ModalScope` на клиенте;
- блокировка menu/statusbar/MDI/top-level;
- window-modal — только owner+ancestors;
- focus restore, keyboard/menu shortcuts;
- тесты: app-modal блокирует sibling и меню; window-modal — только
  родителя; закрытие возвращает фокус.

**DoD:** модальность визуально и по hit-test соответствует Qt.

### 0.14.6 Statusbar (S/M)

- `QStatusBar`-модель, высота, painting;
- transient message с timeout, постоянные поля;
- `App:showMessage`, `statusMessage`, `statusVisible`;
- technical info: connection/app id/form count/version (решение D6);
- тесты: сообщение исчезает по таймауту; viewport не перекрыт.

**DoD:** подвал отображается и не ломает MDI/geometry.

### 0.14.7 `QToolBar` — обязательный виджет (M)

- `QToolBar` + `QAction` в `rqt`; actions переиспользуют модель меню;
- верхняя область, высота/отступы Fusion, icon/text/tooltip/
  `enabled`/`separator`/`checkable`;
- `App:setToolBar(spec)`/`setAppProperty("toolBar", spec)` и
  `toolbarVisible`;
- action dispatch — тем же `menuAction`-путём через main-сессию;
- main shell: menu + toolbar + central MDI + statusbar; viewport
  уменьшается на обе панели;
- overflow, drag между областями и кастомизация — за пределами 0.14,
  но `Top/Bottom/Left/Right` в модели предусмотреть.

**DoD:** toolbar рисуется и кликается; actions открывают формы/закрывают
приложение; скрытие через `toolbarVisible` работает; golden-тест
панели.

### 0.14.8 `QMainWindow` в `.ui` — ✅ сделано (2026-11-25)

- `form-xml`: `QMainWindow`/`QWidget` actions — `Widget.actions` и
  `Widget.add_actions` (`<action>`, `<addaction>`), свойства actions
  через общий `parse_property`;
- `shell::parse_main_window` собирает title, `QMenuBar`/`QMenu`,
  `QToolBar`, `QStatusBar`, central widget и проверяет наличие
  `QMdiArea`; `Shell::apply_main_window` применяет модель;
- `start_app` принимает `ui_file`; main-сессия шлёт `loadMainWindow`
  с `ui_xml`, клиент по умолчанию грузит `mainwindow.ui` в full mode;
- dynamic `App:set_menu`/`setToolBar` заменяет static;
- toolbar из `.ui` — статический, пока не переопределён Lua;
- фикстуры `server/priv/qt_forms/mainwindow.ui`,
  `server/priv/lua_modules/test_mainwindow.lua`.

**DoD:** ✅ `mainwindow.ui` загружается без ручной сборки shell; в
браузере работают static menu/toolbar/status, action открывает MDI-форму.

### 0.14.9 Интеграция и golden — ✅ сделано (2026-11-25)

- фикстуры: `test_main.lua`/`test_mainwindow.lua` + menu,
  `test_dialog01.ui` standalone, `test_shell_siblings.lua` (parent + два
  top-level sibling), `test_modal_flow.lua` (modal child);
- браузерные сценарии full/standalone/top-level/modal проверены через
  chrome-devtools MCP (:9222);
- Qt6 golden: `client/crates/app/tests/fixtures/qt6_mainwindow.json` +
  PNG; `tools/qt_reference` выгружает `main_window` (menu/toolbar/status/
  central, `actionGeometry`, размеры меню);
- тест `main_window_shell_geometry_matches_qt6_offscreen` — допуск 1 px;
  геометрия shell подогнана под Qt (toolbar 39 px, меню-бар x=6,
  popup separator 2 px, action rects);
- обновлены doc/status; `cargo test --workspace` — 165 passed,
  `mix test` — 30 passed, `trunk build --release` — ok.

**DoD:** ✅ обе модели запуска работают в браузере; регрессий нет.

---

## 8. Черновик протокола

Команды и режимы (D2 принят):

```json
{
  "command": "start_app",
  "lua_file": "main.lua",
  "ui_file": "mainwindow.ui",
  "mode": "full"
}
```

```json
{
  "command": "start_dialog",
  "lua_file": "measurements.lua",
  "ui_file": "",
  "mode": "standalone",
  "role": "form",
  "placement": "top-level",
  "modal": false,
  "modal_scope": "none"
}
```

```json
{
  "command": "createForm",
  "form_id": "...",
  "role": "form",
  "placement": "mdi",
  "modal": false,
  "modal_scope": "none",
  "ui_xml": "..."
}
```

```json
{
  "command": "setAppProperty",
  "source_form_id": "main",
  "property": "menu",
  "value": [ ... ]
}
```

```json
{
  "command": "setAppProperty",
  "source_form_id": "main",
  "property": "toolBar",
  "value": [ { "title": "Обновить", "icon": "img24:41", "action": "open_lua",
               "lua_file": "measurements.lua", "mdi": true } ]
}
```

```json
{
  "command": "menuAction",
  "form_id": "main",
  "action_id": "menu/0/children/1",
  "action": "open_lua",
  "lua_file": "measurements.lua",
  "mdi": true,
  "modal": false
}
```

`open_lua` опции: `mdi`, `modal`, `modal_scope`, `ui_file`, `open_opts`.
`create_form` должен получать `placement` из состояния сессии, а не
только из process dictionary модалки.

---

## 9. Принятые решения D1–D8

### D1. Standalone и скрытая оболочка

Старый Qt: `MainWindow` создаётся всегда, по умолчанию скрыт;
`--start=...` открывает первый скрипт с `mdi=false` (top-level). Если
форма просит `mdi=true`, `onCreateForm` вызывает `show()` — главное окно
появляется. `--embed-mdi` при `--hide-mainwindow` отключается.

Что повторяем?

- **A. 1:1.** В standalone скрытая main-сессия/оболочка существует;
  `mdi=true` материализует её.
- **B. Строгий standalone.** Оболочки нет; `mdi=true` принудительно
  превращается в top-level.
- **C. Гибрид.** Оболочка есть только как невидимый координатор
  владения, но никогда не показывается; `mdi=true` — ошибка.

**Решение (web): B — строгий standalone.** Main-оболочки нет; `mdi`
коэрсится в top-level. Поведение A (скрытая оболочка, материализация)
оставлено для будущего desktop-host и в 0.14 не реализуется.

### D2. Контракт запуска

Принято:

- `?main=main.lua` — full mode (main script, меню, shell);
- `?lua=form.lua&ui=...` — standalone (top-level, без меню), как сейчас;
- `?mode=full&lua=main.lua` — явный full для отладки;
- `?app=<uuid>` остаётся id приложения (нельзя занять под имя скрипта:
  в текущем клиенте `app` уже используется для application id).

На сервере: `start_app` для main, `start_dialog` для standalone/форм.

**Решение:** схема принята. `start_app` — full, `start_dialog` —
standalone/формы; текущие `?lua`-URL остаются standalone по умолчанию.

### D3. Источник главного окна и toolbar

- **A. Фиксированная клиентская оболочка** (menu/toolbar/status/MDI),
  main.lua задаёт title/icon/menu/toolbar/status. Быстро, не требует
  парсера `QMainWindow`. **QToolBar обязателен** и реализуется в 0.14.7.
- **B. Загружать `mainwindow.ui`** через `form-xml`/`rqt`, парсить
  `QMenuBar`/`QStatusBar`/`<action>`; dynamic `set_menu` заменяет
  статическое меню.

**Решение: A.** Фиксированная оболочка в 0.14; меню и toolbar из Lua.
`QToolBar` — обязательный виджет, входит в 0.14.7; `QMainWindow.ui` —
отдельный 0.14.8 (✅ static menu/toolbar/status из `.ui`, сама оболочка
остаётся клиентской).

### D4. Top-level окна в браузере

- **A. In-canvas слой** внутри одной вкладки (рекомендуется): один
  websocket, один canvas, z-order/модалки как в desktop-окне.
- **B. Реальные окна браузера** (`window.open`) на каждую форму: popup
  blockers, отдельные canvas/socket, сложнее.
- **C. Позже нативный desktop-host**; сейчас in-canvas.

**Решение: A**, с сохранением `placement` в протоколе, чтобы
desktop-host мог отобразить top-level в настоящие окна.

### D5. Геометрия в браузере

- **A. Main window заполняет viewport** (не двигается/не ресайзится;
  `App:resize/move/geometry` остаются no-op). Standalone-форма
  использует свою `.ui`-геометрию/top-level.
- **B. Main window — плавающее окно** внутри canvas, `App:resize/move`
  реально работают.
- **C. Всё заполняет viewport**, standalone-форма тоже.

**Решение: A.** Standalone-форма ставится в свои `.ui`-координаты с
clamp в viewport; `Form:center`/`centered` — отдельное свойство (B2).

### D6. Statusbar / технический подвал

- **A. `QStatusBar`:** transient message (`showMessage` с timeout) +
  постоянные поля (connection, app id, form count, version). Часть
  полей заполняет клиент, сообщения — Lua.
- **B. Одна строка текста** без постоянных полей.
- **C. Отдельная кастомная панель** (не `QStatusBar`).

**Решение: A.** Transient message — Lua; постоянные technical fields
(connection, app id, form count, version) — клиент; `statusVisible`
управляет видимостью.

### D7. Кто выполняет menu actions

- **A. Main-сессия** получает `menuAction` и вызывает
  `App:open_lua`/`open_modal`/`App:close` (рекомендуется): правильный
  owner, cleanup, единая точка.
- **B. Клиент** по spec сам вызывает `start_dialog` (как старый
  `sendStartDialog`): проще, но обходит main и владение.

**Решение: A.** Main-сессия выполняет actions; client-side запуск не
используем.

### D8. Закрытие и последняя форма

**Решение: принято.** Full — закрытие main закрывает приложение и все
формы; standalone — закрытие последнего top-level окна завершает
приложение; закрытие одной из нескольких top-level форм не закрывает
остальные.

---

## 10. Инварианты и риски

1. **Не смешивать `mdi` и `modal`.** MDI — размещение, modal — ввод.
2. **Не делать все окна `QMdiSubWindow`.** Top-level не клипается MDI
   и имеет свой z-order.
3. **Не давать любой форме менять application-scope.** Меню/title/
   statusbar — только main; иначе «последний победил».
4. **Не тащить `invoke_qt`.** Только типизированные команды.
5. **Не ломать старые формы.** `Form`, `open_lua`, `open_modal_lua`,
   `modal_result` сохраняют API; mode/role — снаружи.
6. **Не считать `mainwindow.ui` готовым.** Текущий парсер игнорирует
   `QMenuBar`/`QStatusBar`/`<action>`; это отдельная работа.
7. **Браузерная геометрия main** — viewport, не native window.
8. **MDI viewport offset.** Иначе подокна заедут под меню/statusbar.
9. **Owner top-level форм = main**, а не случайная форма; иначе
   закрытие инициатора убьёт соседние окна.
10. **Меню без `ui_file`.** Старый spec его не содержит; скрипт сам
    открывает `.ui`. Не возвращать `create_empty_form`.

---

## 11. Definition of done (этап 0.14)

- Full mode: main window с menu, toolbar, central MDI area, statusbar;
  формы открываются из меню/toolbar; title/icon/menu/toolBar из Lua.
- Standalone: без main-оболочки; форма top-level; `mdi=true` коэрсится
  в top-level; несколько top-level окон; закрытие последнего завершает
  приложение.
- `mdi=false` реально top-level, `mdi=true` — MDI; `modal` работает
  независимо.
- Application-modal блокирует menu/statusbar/MDI/top-level.
- Statusbar с сообщением и техническими полями.
- Старые URL/скрипты/меню-spec работают.
- `cargo test --workspace`, `mix test`, wasm build, браузерная проверка
  full/standalone — зелёные.

---

## 12. Статус реализации (2026-11-25)

- **0.14.1** ✅ домен/протокол: `role`, `mode`, `placement`,
  `modal_scope`, `start_app`, main-сессия, передача `mdi` и `modal_scope`
  в `createForm`; menu actions маршрутизируются в main.
- **0.14.2** ✅ main shell: fixed `QMenuBar`/`QToolBar`/`QStatusBar`,
  central `QMdiArea`, viewport offset; standalone без shell.
- **0.14.3** ✅ меню из Lua: `QMenuBar`/`QMenu`/`QAction`-модель,
  вложенные подменю, separators, icons, `menuAction` через main.
- **0.14.4** ✅ top-level слой: `floating`-окна, z-order, активация,
  drag/resize/close, несколько окон; standalone использует его.
- **0.14.5** ✅ application/window modal: `modal_scope`, блокировка
  MDI/top-level/shell для application modal; keyboard/focus guard.
- **0.14.6** ✅ statusbar: transient message, technical fields,
  `statusVisible`/`statusMessage`/`statusInfo`.
- **0.14.7** ✅ `QToolBar` + actions, общий `QAction`-путь с меню,
  `setToolBar`/`toolBarVisible`; кнопки/иконки/separators.
- **0.15** ✅ popup/toolbar/MDI/top-level больше не используют
  маскировку текста: painter order сохранён упорядоченным
  `PaintBatch`-display list (см. `widgets-plan.md` §0.15).
- **0.14.8** ✅ `QMainWindow` из `.ui`: `form-xml` actions/addaction,
  `shell::parse_main_window`, start_app `ui_file` → `loadMainWindow`;
  static menu/toolbar/status заменяются Lua-свойствами.
- **0.14.9** ✅ Qt6 golden shell (`qt6_mainwindow.json`/PNG, 1 px);
  браузерные сценарии full/standalone/top-level siblings/modal,
  static и dynamic menu/toolbar проверены.

Проверки на момент статуса: `cargo test --workspace`, `mix test`,
`trunk build --release`; live-сценарии `?main=test_main.lua` и
`?lua=demo_icons.lua&ui=test_dialog01.ui`.
