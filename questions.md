# Отложенные вопросы к пользователю

Вопросы, на которые нужно твоё решение. Копятся пачкой, чтобы не
останавливать работу на каждом шаге. Обновлено: 2026-11-25.

Ранее закрытые (реализованы и покрыты тестами, повторно не спрашиваем):
Enter/Leave, FocusIn/FocusOut, `open_opts` → `init`, `open_lua`,
модалки и их cleanup, scrollbar-ы и Tab-фокус.

---

## A. Layout-ы (блокируют 0.12/0.13)

**A1. Stretch-факторы grid-а. — ✅ решено (2026-11-25): (а) не поддерживать.**
В `.ui` Qt Designer их не пишет — они задаются кодом (`setRowStretch`).
Варианты: (а) не поддерживать вовсе; (б) своё свойство в `.ui`
(`rowStretch`/`colStretch` строками); (в) только API `rqt`, без парсинга.
Решение: вариант **(а)** — фиктивное свойство в `.ui` не добавляем;
stretch учитывается как 0 в портированном `qGeomCalc`. Если появится
источник (свойство `.ui` или API `rqt`) — отдельная задача.

**A2. Политики `QFormLayout`.**
Минимально нужны `fieldGrowthPolicy` (по умолчанию `AllNonFixedFieldsGrow`)
и `labelAlignment`. Поддерживать ли ещё `rowWrapPolicy`
(`WrapLongRows`/`WrapAllRows`) и `formAlignment`, или ограничиться
дефолтами?

**A3. Точность геометрии. — ✅ решено (2026-11-25): тот же допуск 1 px.**
Для `test_dialog01/02` держим допуск 1 px. Для grid/form — тот же допуск
или допустимо грубее (например, 2–3 px) на первом этапе?
Grid-фикстуры (`grid_2x2`, `grid_spans`, `grid_nested`, `grid_policies`)
сошлись с Qt6 в пределах 1 px, поэтому допуск оставлен 1 px; для 0.13
(форма) — та же цель.

**A4. Приоритет фронта. — ✅ решено (2026-11-25): сначала grid+form.**
Сначала закрыть grid+form (клиент), или сначала серверные пробелы
(`create_empty_form`, App sugar, `Form:center`)? Влияет на порядок 0.12/0.13.
Grid (0.12) закрыт первым; следующий шаг — 0.13 (`QFormLayout`).

---

## B. Сервер / Lua (из `docs/lua-module-composition.md` §10)

### Принято

**B1. `create_empty_form` — не нужен.** Формы всегда создаются из `.ui`;
метод не добавляем. Динамические сценарии, если появятся, делаются
отдельной задачей.

**B3. `modal_result` — структурный подход.** Решение: host-функция
`open_modal_lua` возвращает Lua-таблицу `{status, value}`, где `status` —
`"ok" | "closed" | "error" | "timeout"`. Совместимость:

- новый канонический `Form:open_modal`/`App:open_modal` возвращает структуру;
- старый `Form:open_modal_lua`/`App:open_modal_lua` — обёртка:
  `ok → value`, `closed → nil`, `error/timeout → "error"/"timeout"`
  (существующие формы и тесты не ломаются);
- тесты: legacy-путь + структура; `finish_modal("error")` от ребёнка не
  должен путаться с системной ошибкой.

**B4. `App` sugar — частично реализован и пересматривается в 0.14.**
`setWindowTitle` → `document.title`
и `setIcon` → favicon работают через команду `setAppProperty`
(свойства `windowTitle`/`windowIcon`). Остальное
(`resize`, `move`, `setGeometry`, `center`, `showMaximized`,
`showMinimized`, `showNormal`, `showFullScreen`, `visible`, `set_menu`)
объявлено в Lua и принимается протоколом, но браузерный клиент это
игнорирует — задел под desktop-бинарник с нативным окном вокруг встроенного
браузера.

**B5. `invoke_qt` — не тащим** (зафиксировано в §2/§10: только узкие
типизированные команды; при необходимости — allowlist отдельной задачей).

### Открыто

**B2. `Form:center` / клиентское центрирование. — ✅ решено (2026-11-25).**
В `ref_qt` `Form:center` = `set("centered", true)` и реально используется
6 раз (`form_common_lib` + тесты). Реализовано свойство формы `centered`:
центрирует `QMdiSubWindow` в MDI-области (top-level — во viewport),
тест `centered_form_property_centers_in_mdi_viewport`. `App:center` — это
центрирование главного окна, в браузере оставляем no-op.

---

### B4. `App` sugar: как сделано

**Что это в старом коде.** `ref_qt` `application.lua` даёт
`setWindowTitle`, `setIcon`, `resize`, `move`, `setGeometry`, `center`,
`set_menu`, `showMaximized/Normal/Minimized/FullScreen`, `visible`. Всё это —
методы `QMainWindow`, реализованные в `tcs_qtgui/invoke_mainwindow.cpp`
(`qt_set_window_title`, `qt_resize`, `qt_move`, `qt_set_geometry`,
`qt_show_*`, `qt_center`, `qt_set_menu`). Формы при этом — MDI-подокна.

**Что в EPlatform-клиенте.** Одно окно = canvas 100%×100% viewport;
заголовок фиксирован `"EPlatform"` при старте, API для title/icon нет;
меню/статусбара нет; формы — `QMdiSubWindow` в `QMdiArea`, у них уже есть
свои `windowTitle` и `geometry` через `setFormProperty`. То есть
`App:resize/move/setGeometry/show*/setIcon/set_menu/center/visible`
не отображаются ни на что осмысленное в браузере.

**Фактическое использование (tcs_back@ref_qt, 21 модуль):**

| Метод App | Где встречается |
|---|---|
| `setWindowTitle`, `setIcon`, `resize`, `center`, `showNormal`, `visible`, `set_menu` | **только** `test_app_01.lua` (синтетический фикстур) |
| `close` | `test_02.lua` (и master/dev — только он) |

Реальные формы (`measurements`, `projects`, `users`, `par_*`, ...)
используют Form-level sugar: `on` (58), `setText` (29), `setWindowTitle` (6),
`setGeometry` (5), `center` (6), `setToolTip` (6), `setToolButton` (6).

**Варианты.**

1. **Не добавлять** (рекомендация). Покрывает 100% фактического
   использования; App-операции — свойство desktop-QMainWindow, которого
   в браузерном клиенте нет.
2. **Минимум:** `App:setWindowTitle` → `document.title` (заголовок
   вкладки) и, опционально, favicon для `setIcon`. 1 команда клиенту;
   ценность — только kiosk/embedded-сценарии.
3. **Полный паритет:** меню-бар, статус-бар, fullscreen API, иконка —
   отдельный крупный этап, конфликтует с web-моделью.

**Решение (2026-11-25):** `setWindowTitle` и `setIcon` делаем; остальные
методы оставляем объявленными как задел под desktop-host, браузер их
игнорирует. Фуллскрин (`showFullScreen`) в браузере пока не включаем —
это отдельная задача про browser chrome.

**Обновление 0.14:** это решение пересматривается. В этапе
`docs/application-shell-plan.md` меню (`set_menu`), statusbar и top-level
окна становятся реальными; no-op остаётся только у геометрии главного
окна в браузере. Источник истины — решения D1–D8 ниже.

**B6. Таблицы.**
Нужны ли `tableLayout`/`groupedHeader` в клиенте, или старые формы можно
упростить до `tableHeaders`/`tableData`?

**B7. Слой данных.**
Планируются ли `list_*`/`data`-модуль и БД в этом проекте? Если да —
один типизированный модуль или набор функций в host API.

---

## C. Процесс

**C1. Review-пак.**
76 форм сгенерированы, `client/review/comments.md` пуст. Начинать цикл
ревью вёрстки сейчас или после grid/form?

**C2. Судьба `tcs_back/db_migrate`.**
Миграция MSSQL→PG→Ash идёт отдельным треком в `tcs_back`. Она входит в
текущий фронт работ или это параллельная задача?

---

## D. Application shell / главное окно (этап 0.14) — ✅ принято

Полный план: `docs/application-shell-plan.md`. Решения от 2026-11-25:

**D1 → B для web.** Строгий standalone: main-оболочки нет, `mdi=true`
коэрсится в top-level. Скрытая оболочка и материализация по MDI —
только будущий desktop-host, не 0.14.

**D2 → принято.** `?main=main.lua` — full (`start_app`);
`?lua=form.lua&ui=...` — standalone (`start_dialog`); `?mode=full&lua=`
— явный full. `?app=<uuid>` остаётся id приложения. Текущие URL
остаются standalone по умолчанию.

**D3 → A.** Фиксированная клиентская оболочка (menu/toolbar/status/MDI),
меню и toolbar из Lua. `QToolBar` — обязательный виджет, входит в 0.14.7;
`QMainWindow.ui` — отдельный 0.14.8 (✅ static menu/toolbar/status).

**D4 → A.** Top-level окна — in-canvas слой внутри одной вкладки;
`placement` сохраняется в протоколе для будущего desktop-host.

**D5 → A.** Main заполняет viewport, его geometry — no-op.
Standalone-форма использует свои `.ui`-координаты с clamp в viewport;
`Form:center`/`centered` — отдельное свойство (B2).

**D6 → A.** `QStatusBar`: transient message от Lua + постоянные
технические поля от клиента (connection, app id, form count, version);
`statusVisible` управляет видимостью.

**D7 → A.** Menu actions выполняет main-сессия через
`App:open_lua`/`open_modal`/`App:close`; client-side запуск не
используем.

**D8 → принято.** Full: закрытие main закрывает приложение и все формы.
Standalone: закрытие последнего top-level окна завершает приложение;
закрытие одной из нескольких форм остальные не закрывает.

Реализовано: 0.14.1–0.14.9 (`QMainWindow` из `.ui`, Qt6 golden shell,
браузерные сценарии full/standalone/top-level/modal).

---

## E. Клиент (0.18)

**E1. Белый экран при запуске без сервера.**
Запуск без `backend=…` (ws к дефолтному адресу → 404): canvas остаётся
белым, оболочка не видна. Проверено на worktree `640a0a7` — воспроизводится
и там, то есть это не регрессия 0.18. С работающим сервером новый
дистрибутив (прогресс-бар + `opt-level=z`) рендерится нормально.
Вопрос: чинить сейчас (idle-экран «нет соединения» поверх canvas) или
отложить? Рекомендация: idle-состояние со статусом, а не пустой белый
canvas — иначе на телефоне выглядит как сломанная загрузка.

**E2. `strip = "symbols"` для wasm (кандидат, 0.18.7).**
Убирает name-секцию: −3.68 MB raw / −509 КБ gzip. Но wasm-bindgen
предупреждает про потерю `target_features`, а при проверке страница
зависла — вероятно, ломается детект фич/инициализация. Путь: выяснить,
как восстановить `target_features` (wasm-bindgen CLI/post-step или
wasm-opt), проверить в браузере, и только потом включать. Решение за
тобой: пробовать или оставить `debuginfo` (сейчас gz 2.64 MB).
