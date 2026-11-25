# Changelog

## 2026-11-25

### Волна 0.17.1: мобильный ввод и touch-сценарии

- **Скрытое поле ввода.** `index.html` содержит off-screen `<textarea>`;
  JS перехватывает `beforeinput`/`input`/`composition*`/`keydown` и
  отправляет в Rust семантические действия (`insert`, `backspace`,
  `left`, `tab`, `selectAll`, …). Rust переиспользует общий
  `apply_key_action`, поэтому десктоп и мобильный ввод идут одним путём.
  `keydown` в capture-фазе блокирует winit, иначе текст вставлялся бы
  дважды. Фокус hidden-поля синхронизирован с `QLineEdit`.
- **IME/раскладка.** `compositionend` коммитит строку в `text`;
  следующий `input` подавляется, чтобы не было дубля. Проверены
  «тест»/русский текст, Backspace, Tab, Ctrl+A и Enter.
- **Enter/ReturnPressed.** Enter на `QLineEdit` теперь шлёт
  `formEvent "ReturnPressed"`; на мобильном это тот же путь, что на
  десктопе.
- **Touch-заголовок.** На touch верхняя полоса окна теперь всегда
  трактуется как `Title`, а не как `ResizeTop`; перетаскивание за
  верхний край двигает окно и не меняет размер. Нижние/боковые края
  по-прежнему ресайзят. Тест `subwindow_layout_and_hit_test`.
- **Клавиатура на iOS.** На touch-устройствах скрытое поле становится
  прозрачным hit-target ровно поверх активного `QLineEdit`
  (`pointer-events:auto`, `opacity:0.01`). Тап по полю — это нативный
  тап по input, поэтому iOS открывает клавиатуру каждый раз, а не
  только после первого фокуса. На десктопе overlay остаётся
  `pointer-events:none`. Rect обновляется Rust-ом при фокусе/движении
  окна; повторный `focus()` каждый кадр убран.
- **Загрузка на телефоне.** Для мобильной проверки собирается release:
  11 MB wasm вместо 36 MB debug (gzip ~3.5 MB), JS-компиляция и старт
  заметно быстрее. Debug-сборка остаётся для разработки.
- **Safari backend URL.** `backend` декодирует `%3A/%2F/%3F/%3D` и сам
  добавляет `?vsn=2.0.0`, если query нет. Ссылка для телефона больше не
  содержит вложенный `?`:
  `...&backend=ws://10.0.0.7:4010/socket/websocket`. Старый вариант с
  `?vsn=2.0.0` и percent-encoding тоже работает.
- **Touch-состояния.** После `TouchPhase::Ended/Cancelled` сбрасываются
  `under_mouse` и `under_mouse_widget`; `CursorLeft` тоже очищает hover.
  `under_mouse` теперь реально выставляется при движении указателя, так
  что `:hover` QSS работает и на десктопе.
- **Долгий тап.** 500 мс → `formEvent "ContextMenu"` для виджета под
  пальцем; release после long tap не превращается в click.
- **Двойной тап.** В `QLineEdit` выделяет слово под курсором
  (`select_word_at`, Unicode alphanumeric + `_`).
- **Multi-touch.** Основной контакт владеет указателем; второй палец
  отменяет tap/click/long-press и переводит жест в two-finger scroll.
  Pinch отложен: в клиенте пока нет модели масштаба сцены.
- **Тесты.** `cargo test --workspace` — 192 passed, 1 ignored
  (app 52+1 ignored, form-xml 13, rqt 127); был добавлен тест
  `select_word_at`, `clear_hover`, маппинг web-действий. Браузер:
  touch-тап, IME, Backspace/Tab/Ctrl+A, double tap, long tap.

### Волна 0.17.2–0.17.4: QSS box model, `Form:center`, splitter/tree API, review

- **QSS box model.** `margin` и `margin-top/right/bottom/left` сдвигают
  frame (фон/рамку), `padding` и per-side `padding-*` — content-rect.
  Padding теперь учитывают текст, selection, caret, кнопки и заголовок
  `QGroupBox`; `QGroupBox { margin-top }` из `52_checks.ui` работает.
- **QSS-градиенты.** `background`/`background-image` понимают
  `qlineargradient(x1:…, y1:…, x2:…, y2:…, stop:…)`; painter рисует по
  крайним стопам вертикально через `FillLinearGradientRect`. `none`/
  `transparent` и наследование style sheet сохранены. `border-image`/
  `url()` не поддерживаются: нужен asset pipeline.
- **Item-view QSS.** `QTreeWidget::item`, `QListWidget::item`,
  `QTableWidget::item`, `QHeaderView::section` и
  `QComboBox QAbstractItemView`: фон/цвет/selection для строк, ячеек,
  заголовков и popup. `QTreeWidget::item` больше не применяется ко всему
  дереву как обычный виджет.
- **`Form:center`.** Свойство `centered` центрирует `QMdiSubWindow` в
  MDI-области (top-level — во viewport), не меняя размер.
- **`QSplitter`/`QTreeWidget` API.** `sizes` через `setProperty`,
  `expandAll`/`collapseAll`; `childrenCollapsible` не добавляли.
- **Review.** `scripts/review.sh` пересобран: `qss/52_checks` 4.88 → 4.07,
  `qss/55_disabled` 11.44 → 4.41; браузерный скриншот QSS-формы проверен.
- **Тесты.** `cargo test --workspace` — 199 passed, 1 ignored
  (app 53+1 ignored, form-xml 13, rqt 133); wasm-check и `cargo fmt
  --check` зелёные.

### Волна 0.16: combo popup, collapse/expand tree, `QSplitter`, QSS

- **Popup `QComboBox`.** Добавлено transient-состояние `QComboBoxPopup`
  (`highlight`, `scroll`) в `QWidget`; `QWidgetTree` умеет
  `open/close_combo_popup`, считать `combo_popup_rect` (под комбобоксом,
  при нехватке места — над ним), hit-test-ить строки и коммитить выбор.
  Popup рисуется после дерева/рамок окон отдельным слоем, поэтому не
  обрезается геометрией комбобокса. Мышь: press открывает список,
  press по строке подсвечивает, release выбирает и шлёт
  `CurrentIndexChanged`; клик вне/по комбобоксу закрывает. Клавиатура:
  Up/Down/Page/Home/End меняют `currentIndex` или highlight, Enter
  открывает/коммитит, Space открывает, Escape закрывает. Строка popup —
  24 px, как `QListView::sizeHintForRow` в Fusion для DejaVu Sans 14.
- **Collapse/expand `QTreeWidget`.** `QTreeItem.expanded` (по умолчанию
  `false`, как в Qt), flatten показывает только детей раскрытых узлов;
  `set_row_expanded`/`toggle_row_expanded`/`expand_all`/`collapse_all`,
  сохранение current по path при перестройке. В paint появились
  треугольные branch-индикаторы; клик по индикатору переключает узел,
  Left/Right в приложении сворачивают/раскрывают и ходят к родителю/
  первому ребёнку.
- **`QSplitter`.** Новый класс в дереве: ориентация из `.ui`,
  `handleWidth` (Fusion 4 px), пропорциональное распределение панелей от
  hint-ов с учётом `minimumSizeHint`, ручка тянется мышью, min панелей
  соблюдается. Геометрия сверена с Qt6 (`splitter_h`/`splitter_v`,
  допуск 1 px), минимум формы считается из минимумов панелей
  (166×92 / 92×166).
- **QSS-расширения.** Цвета: named, `rgb()`/`rgba()`, 4/8-значные
  `#ARGB`/`#AARRGGBB` (Qt-порядок), `transparent`/`none`. Селекторы:
  `[property="value"]`, `:pressed`, `:default`, `:selected`, `:on`,
  `:open` (popup combo), отрицание `:!state`. Свойства:
  `font-family`, `font-style`, `font` shorthand, per-side
  `border-*`/`border-*-width`/`border-*-color`, `border-style`,
  `text-align`; QSS-шрифт применяется к тексту, per-side рамки
  рисуются отдельными полосами.
- **Тесты.** `cargo test --workspace` — 189 passed, 1 ignored
  (app 50+1 ignored, form-xml 13, rqt 126); `cargo check -p app
  --target wasm32-unknown-unknown` проходит. Новые тесты: combo
  popup open/hit/commit/outside/above, paint popup, splitter layout/drag/
  min + Qt6 golden, tree collapse/current и keyboard Left/Right, QSS
  цвета/шрифты/стороны/атрибуты/состояния.
- **Планы.** Закрытый план 0.0–0.16 перенесён в
  `docs/archive/completed-plans/widgets-plan-0.0-0.16.md`; активный
  план волны 0.17 (мобильный ввод, QSS v2, `Form:center`, review) —
  `widgets-plan.md`.

## 2026-11-25

### Минимальный размер формы и клиппинг содержимого

- **Минимум из layout-а.** `QWidgetTree::minimum_size_hint` возвращает
  `max(minimumSize, layout minimum)`; `createForm` и обновление
  `minimumSize`/`maximumSize` используют его для ограничений
  MDI-подокна. Форма больше не сжимается до fallback `80×64`, если
  нижний ряд кнопок требует больше места.
- **Клиппинг как в Qt.** `paint_widget` держит clip родителя активным
  при отрисовке детей, а `QPainter::set_clip_rect` пересекает новый clip
  с текущим. Дети, вылезающие за форму, больше не рисуются поверх
  рамки/соседних окон.
- **Hit-test.** `widget_at_inner` сначала проверяет геометрию родителя:
  переполненный ребёнок не получает клик вне формы.
- **Тесты.** `layout_minimum_raises_form_minimum_size`,
  `overflowing_child_is_clipped_and_not_hit_outside_root` и app-тест
  `shrinking_form_stops_at_root_layout_minimum` (geometry + hit-test);
  `cargo test --workspace` — 173 passed, 1 ignored
  (app 48, form-xml 13, rqt 112).

### `QBoxLayout`: рост виджетов по `qGeomCalc`

- **Рост.** При избытке места `layout_box` теперь, как `qGeomCalc`, даёт
  всем элементам с `maximumSize > hint` равную долю свободной ширины
  (а не `hint + доля`), уступая элементам, упёршимся в hint или
  максимум. Раньше кнопки/combo исключались, и весь избыток уходил в
  зазоры. Остаток после всех максимумов по-прежнему попадает в зазоры.
- **Геометрия элемента.** Box-движок применяет
  `QWidgetItem::setGeometry`: ширина/высота клампятся к максимуму и
  выравниваются (по вертикали — по центру), как в grid.
- **Golden.** `box_buttons.ui` — четыре `QPushButton` в `QHBoxLayout`
  без spacer-а (520×120), одна кнопка с длинной подписью: Qt6 даёт
  hint 80/176/80/80, а ширину 101/176/102/101, y=47 — то есть делит
  свободное место поровну, а не `hint + доля`; `qt6_box_buttons.json`/
  `.png` сняты `tools/qt_reference`.
- **Тесты.** Geometry- и raster-сравнения с Qt6; `cargo test
  --workspace` — 170 passed, 1 ignored (app 47, form-xml 13, rqt 110).

## 2026-11-25

### Paint order 0.15.5: golden PNG для shell и окон

- **Rasterizer батчей.** В app-тестах `rasterize_batches` проигрывает
  реальный `gfx::PaintBatch` через `rqt::raster`: rect-батчи → `FillRect`,
  text-батчи → `DrawText` с сохранённым clip. Это тот же ordered display
  list, что уходит в GPU.
- **Эталоны.** `client/crates/app/tests/fixtures/shell_popup_toolbar.png`
  (popup над toolbar), `shell_nested_menu.png` (parent + nested submenu),
  `shell_overlapping_windows.png` (MDI под top-level окном).
- **Тесты.** `shell_*_raster_matches_golden` собирают `collect_paint()` и
  сравнивают mean/max diff; ignored `regenerate_shell_goldens`
  перегенерирует PNG. Регрессия text-masking/перестановки слоёв теперь
  ловится визуально, а не только по индексам батчей.
- **Проверки.** `cargo test --workspace` — 168 passed, 1 ignored
  (app 47, form-xml 13, rqt 108); `mix test` — 30 passed.

### Application shell 0.14.9: интеграция и Qt6 golden shell

- **Golden.** `tools/qt_reference` выгружает `main_window`: геометрию
  `QMenuBar`/`QToolBar`/`QStatusBar`/central widget, прямоугольники
  actions (`actionGeometry`), размеры меню и popup-строк. Фикстуры
  `client/crates/app/tests/fixtures/qt6_mainwindow.json` + PNG.
- **Геометрия под Qt.** `TOOLBAR_H` 30 → 39, меню-бар начинается с
  x=6 и считает ширину `ceil(hint)+8` с зазором 6, popup separator 2 px,
  ширина меню `hint+51 (+8 при иконке)`, toolbar action rect
  `x=13,y=4,h=31`, ширина `hint+44`, separator 6 px. Тест
  `main_window_shell_geometry_matches_qt6_offscreen` сверяет меню/toolbar/
  status/central с Qt6 (допуск 1 px).
- **Интеграция.** Добавлен `server/priv/lua_modules/test_shell_siblings.lua`
  (parent + два top-level sibling). Серверный тест `open_lua` теперь
  проверяет и `createForm` ребёнка (`placement=mdi`; `mode` наследуется
  от родителя).
- **Браузер (chrome-devtools MCP :9222).** Проверены:
  `?main=test_mainwindow.lua&ui=mainwindow.ui` (static shell + toolbar →
  MDI форма), `?main=test_main.lua` (dynamic menu/toolbar заменяют static;
  File → MDI форма), `?lua=demo_icons.lua&ui=test_dialog01.ui`
  (standalone), `?lua=test_shell_siblings.lua&ui=test_dialog01.ui`
  (3 окна), `?lua=test_modal_flow.lua&ui=test_dialog01.ui` (modal child).
- **Проверки.** `cargo test --workspace` — 165 passed (app 44,
  form-xml 13, rqt 108); `mix test` — 30 passed; `trunk build --release`
  — ok.

### Application shell 0.14.8: `QMainWindow` из `.ui`

- **Парсер.** `form_xml::Widget` получил `actions: Vec<Action>` и
  `add_actions: Vec<String>`; разбираются `<action>` с `<property>` и
  `<addaction name=...>` в порядке документа.
- **Статическая оболочка.** `shell::parse_main_window` вытаскивает из
  `QMainWindow.ui` title, `QMenuBar`/`QMenu` (`<addaction>`), `QToolBar`,
  `QStatusBar` и central widget; проверяет, что central содержит
  `QMdiArea`. `Shell::apply_main_window` применяет меню/toolbar/status.
- **Протокол.** `start_app` принимает `ui_file`; `DialogSession` для main
  читает `priv/qt_forms/<ui_file>` и шлёт команду `loadMainWindow` с
  `ui_xml` (до Lua `init`). Клиент в full mode по умолчанию берёт
  `mainwindow.ui` (`?ui=`/`?mainui=` переопределяют).
- **Замена динамикой.** Static menu/toolbar живут до первого Lua
  `App:set_menu`/`setToolBar`; динамические команды заменяют модель.
- **Фикстуры.** `server/priv/qt_forms/mainwindow.ui` (меню File/View,
  вложенное меню, toolbar с separator, statusbar, central `QMdiArea`) и
  `server/priv/lua_modules/test_mainwindow.lua`.
- **Проверки.** `cargo test --workspace` — 164 passed (app 43,
  form-xml 13, rqt 108); `mix test` — 30 passed; `trunk build --release`
  — ok. Браузер: `?main=test_mainwindow.lua&ui=mainwindow.ui` —
  статические menu/toolbar/status из `.ui`, toolbar-кнопка открывает
  MDI-форму, меню File показывает action+separator.

### `scripts/serve_dist.py`: gzip и keep-alive

- **Проблема.** Страница на телефоне открывалась десятки секунд:
  `app-*_bg.wasm` весит 10.4 МБ и отдавался без сжатия
  (`SimpleHTTPRequestHandler` игнорирует `Accept-Encoding`), а по
  HTTP/1.0 каждое из трёх тел шло в новом TCP-соединении поверх
  WireGuard-релея.
- **Правка.** `send_head` отдаёт `.wasm/.js/.html/.css/.json/.svg/.xml`
  (>1 КБ) с `Content-Encoding: gzip` и `Vary: Accept-Encoding`,
  сжатие кешируется по mtime; `protocol_version = "HTTP/1.1"` — работает
  keep-alive.
- **Эффект.** wasm 10 413 979 → 3 479 075 Б (2.99x), js 149 159 →
  22 934 Б (6.5x). В браузере `encodedBodySize=3 479 075`,
  `decodedBodySize=10 413 979` — приложение стартует как раньше.
- **Осталось в резерве.** 2.7 МБ внутри wasm — секция `name` (символы):
  `strip = "symbols"` в `[profile.release]` даёт 7.3 МБ raw /
  2.95 МБ gzip (−11 %), `wasm-opt -Oz` (сейчас
  `data-wasm-opt="0"`) — ещё больше.

### Ввод с сенсорных экранов в wasm-клиенте (iOS/Android)

- **Симптом.** На iPhone (Chrome = WebKit) страница рисовалась, меню
  открывалось, окно формы создавалось, но кнопки, `QLineEdit`,
  перемещение и ресайз подокон не работали: «работал» только левый
  верхний угол. Причина — три независимых дефекта.
- **1. Не было `WindowEvent::Touch`.** `winit` на web отдаёт касания
  только как `Touch` (`CursorMoved`+`MouseInput` — лишь для
  `pointerType == "mouse"`), а в `App::window_event` ветки `Touch` не
  было. Логика указателя вынесена в `App::pointer_moved(x, y)` /
  `App::pointer_released()` (отдельный `impl App`), `Touch{Started,
  Moved,Ended,Cancelled}` делегируют туда же. В CSS —
  `canvas { touch-action: none; user-select: none; -webkit-touch-callout:
  none; -webkit-tap-highlight-color: transparent }`.
- **2. Не было `<meta name="viewport">`.** Мобильный браузер брал layout
  viewport 980 px: `innerWidth=980`, `canvas=980×2120`,
  `visualViewport.scale=0.398`. Координаты указателя приходят в CSS
  visual viewport, виджеты живут в layout-пикселях → промах 2.5×.
  Добавлен `width=device-width, initial-scale=1, viewport-fit=cover,
  user-scalable=no`.
- **3. DPI-рассинхрон.** Web-бэкенд `winit` отдаёт `inner_size()` в
  CSS-пикселях, а позиции указателя — в device-пикселях
  (`css * dpr`), при `scale_factor()==3`; из-за этого `Gfx::ui_scale`
  становился 3, логическое пространство — `130×281` вместо `390×844`.
  Добавлены `App::physical_size()` (домножает размер окна на DPR на
  wasm) и `App::sync_canvas_backing()` (backing store канваса в
  device-пикселях: `1170×2532` при CSS `390×844`) — `ui_scale` остался
  реальным DPR, а картинка стала HiDPI-резкой.
- **Проверки.** `cargo check --workspace`, `trunk build --release` — ok.
  Доверенный тач через CDP `Input.dispatchTouchEvent` (WSLg Chromium
  :9222, 390×844, `deviceScaleFactor=3`, `maxTouchPoints=5`): тап по
  «Файл» → меню; тап по «Тестовая форма» → `createForm` через WS; тап по
  `QPushButton` → `event Clicked -> pushButton_1` + `formEvent Clicked` на
  сервер; тап по `QLineEdit` → `focus lineEdit_2`; drag заголовка → окно
  переехало; drag правой грани и угла → ресайз.
- **Осталось** (подробно — `widgets-plan.md`, раздел «Touch-ввод на
  смартфоне»): мягкая клавиатура/ввод текста (`keydown` на canvas, нужен
  скрытый `<input>` + bridging в `KeyboardInput`), pinch/зум и
  двухпальцевый скролл, контекстное меню по долгому тапу, выделение
  текста тачем, сброс hover после `TouchPhase::Ended`.

### Form layout 0.13.1–0.13.3: `QFormLayout` раскладывается двумя колонками

- **Парсер.** `form_xml::Layout` получил `field_growth_policy`,
  `row_wrap_policy`, `label_alignment`, `form_alignment`,
  `horizontal_spacing`, `vertical_spacing`; enum-ы
  `FieldGrowthPolicy`/`RowWrapPolicy`. Читаются
  `<enum>QFormLayout::…</enum>` и `<set>Qt::Align…|…</set>`, включая
  Designer-синтаксис без `AlignmentFlag::`. `colspan>=2` трактуется как
  spanning (full-row) item.
- **Движок.** `layout_form` + `form_data` — порт Qt
  `QFormLayoutPrivate::updateSizes`/`setupVerticalLayoutData`/
  `setupHorizontalLayoutData`/`arrangeWidgets` для не-HFW случая:
  колонка меток = максимум hint, поле получает остаток по
  `fieldGrowthPolicy` (`AllNonFixedFieldsGrow`/`FieldsStayAtSizeHint`/
  `ExpandingFieldsGrow`), работают `rowWrapPolicy`
  (`DontWrapRows`/`WrapLongRows`/`WrapAllRows`), `labelAlignment`,
  `formAlignment`, `horizontalSpacing`/`verticalSpacing`.
- **Вертикаль.** Треки формы считаются тем же `qGeomCalc`, что и grid;
  top/bottom stretch повторяет Qt при выравнивании формы.
- **Вложенность.** `layout_size_hint`/`layout_minimum_size`/
  `layout_maximum_size` знают про Form; форма внутри group box меряется
  как Qt-форма. `QLineEdit` hint в form-движке приведён к целочисленной
  `QFontMetrics`-формуле Qt (box-движок не затронут).
- **Тесты.** 6 фикстур `form_{basic,wrap_long,wrap_all,growth,alignment,
  group}` + Qt6 JSON/PNG; `form_*_geometry_matches_qt6_offscreen`
  (допуск 1 px) и `form_*_raster_compared_to_qt6`; парсерные
  `parses_form_layout_properties`,
  `form_layout_defaults_follow_fusion`.
- **Инструменты.** `tools/qt_reference` дополнительно выгружает
  `layout.class` и свойства `QFormLayout`.
- **Review.** Категория `form` в `tools/formgen/review.py` (6 форм,
  id `form/80…85`); пак перегенерирован: mean 1.3–3.4, >30 — 1.6–3.2 %.
- **Проверки.** `cargo test --workspace` — 161 passed (app 41,
  form-xml 12, rqt 108); `mix test` — 29 passed. Браузер
  (chrome-devtools MCP, WSLg Chromium :9222):
  `?lua=test_qform.lua&ui=test_qform.ui` — правая колонка меток,
  длинная метка не сдвигает поля, checkbox и Reset/Close в spanning-row,
  Reset очищает text и снимает checked. Фикстуры:
  `server/priv/qt_forms/test_qform.ui`,
  `server/priv/lua_modules/test_qform.lua`.

### Grid layout 0.12.2–0.12.5: `QGridLayout` раскладывается сеткой

- **Явный вид layout-а.** `QBoxLayoutDirection` + `class.contains('H')`
  заменены на `QLayoutKind { HBox, VBox, Grid, Form }` и
  `QLayout { kind, spacing, margins, items }`; `QLayoutKind::from_class()`
  читает класс из `.ui`. `layout_widget` диспетчеризует в
  `layout_box`/`layout_grid` (Form тогда шёл как vbox; закрыто в 0.13).
  Поведение hbox/vbox не изменилось.
- **Движок сетки — порт Qt.** `layout_grid` + `grid_tracks` повторяют
  `QGridLayoutPrivate::setupLayoutData`/`distribute` и `qGeomCalc`
  (`qlayoutengine.cpp`): целочисленные размеры, fixed-point распределение,
  `distributeMultiBox` для span, `setupSpacings` (зазор только между
  разными элементами), `qSmartMinSize`/`qSmartMaxSize` и классовые
  size policy по умолчанию (`widget_axis_policy`).
- **Парсер.** Добавлен `<property name="sizePolicy">`
  (`hsizetype`/`vsizetype` → `PropValue::Map`) и
  `<item alignment="Qt::Align…">` → `LayoutCell.alignment: u32`
  (константы `form_xml::ALIGN_*`).
- **Span и alignment.** `grid_widget_rect` повторяет
  `QWidgetItem::setGeometry`: кламп по `maximumSize`, при заданном
  alignment — кламп по hint и Qt-выравнивание (без горизонтальных битов —
  влево, по вертикали без `AlignTop` — по центру).
- **Stretch** не поддерживается (решение A1-a: в `.ui` его нет,
  фиктивное свойство не добавляем).
- **Тесты.** `grid_{2x2,spans,nested,policies}_geometry_matches_qt6_
  offscreen` (допуск 1 px) и `grid_*_raster_compared_to_qt6`; фикстуры
  + Qt6 JSON/PNG в `client/crates/rqt/tests/fixtures/`; парсерные тесты
  `parses_size_policy_types`, `parses_item_alignment_flags`; вырожденная
  сетка из одних спейсеров — `grid_with_spacers_in_tight_rect_does_not_panic`
  (защита от underflow в shrink-ветке `qGeomCalc`).
- **Review.** Категория `grid` в `tools/formgen/review.py` (4 формы,
  id `grid/76…79`), пак перегенерирован: mean 0.8–4.7, >30 — 1–6 %.
- **Проверки.** `cargo test --workspace` — 147 passed (app 41,
  form-xml 10, rqt 96); `trunk build --release` — ok. Браузер
  (chrome-devtools MCP, WSLg Chromium :9222):
  `?lua=test_grid.lua&ui=test_grid.ui` — сетка 3×4 с вложенным hbox,
  span-таблицей и spacer-ом; ввод в ячейку, hit-test Reset/Close.
  Фикстуры для этого: `server/priv/qt_forms/test_grid.ui`,
  `server/priv/lua_modules/test_grid.lua`.
- **Известный дефект (вне 0.12).** `QBoxLayout` отдаёт избыток места в
  зазоры, а не растит виджеты с политикой `Minimum`/`Preferred`
  (например, ряд кнопок без spacer-а). Grid уже считает по Qt-алгоритму;
  box — отдельный шаг (TODO в `widgets-plan.md`).

### Paint order 0.15: ordered display list / PaintBatch

- **Модель.** `gfx::PaintBatch::{Rects,Text}` заменил два глобальных
  пула (`Vec<ColoredRect>` + `Vec<ColoredText>`). `append_paint_commands`
  выдаёт батчи в порядке `QPaintCommand`; соседние однотипные команды
  объединяются, но `Rects -> Text -> Rects -> Text` сохраняет порядок.
- **Рендерер.** `gfx::render(clear, &[PaintBatch])` проигрывает батчи в
  одном render-pass: rect-батчи — диапазоны общего vertex buffer, текст —
  отдельный glyphon `TextRenderer` на текстовый батч.
- **Слои.** `App::collect_paint` отдаёт display list в порядке
  shell base → MDI content+chrome по z-order → top-level по z-order →
  popup-меню по уровням (родитель, затем вложенное).
- **Маскировка удалена.** `Shell::popup_rects` и фильтрация
  `texts.retain(...)` под popup удалены: непрозрачный фон popup теперь
  честно перекрывает toolbar/form/MDI текст благодаря painter order.
- **Тесты.** `paint_command_stream_keeps_rect_text_rect_text_order`,
  `popup_background_is_batched_after_base_text_without_masking`,
  `nested_popup_batches_paint_parent_before_child`,
  `mdi_content_is_painted_before_floating_top_level_windows`.
- **Проверки.** `cargo test --workspace` — 136 passed (app 41,
  form-xml 8, rqt 87); `mix test` — 29 passed; `trunk build --release`.
  В браузере (chrome-devtools MCP, WSLg Chromium :9222) проверены
  `?main=test_main.lua` (Файл/Вид/вложенное подменю/toolbar) и
  `?lua=demo_icons.lua&ui=test_dialog01.ui`; скриншоты до/после сняты.

### Application shell 0.14: main window, menu, toolbar, top-level, modal

- **Режимы запуска.** `?main=<script>` — full mode с main-сессией
  (`role=main`) и оболочкой; `?lua=<script>&ui=...` — standalone без
  главного окна; `?mode=full&lua=...` — явный full. `?app=` остаётся
  application id.
- **Протокол.** `createForm` теперь всегда несёт `role`, `mode`,
  `placement` (`mdi`/`top-level`), `modal`, `modal_scope`; `start_app`
  запускает main, `setAppProperty` содержит `source_form_id`/`role`.
- **Main session.** `DialogManager.start_app`, one main per application;
  main владеет top-level формами; `Form:close` на main закрывает
  приложение; menu actions маршрутизируются в main (`menuAction`).
- **Клиент.** `Shell`: menu bar, toolbar, statusbar; `QMdiArea` —
  центральная область; `floating` — top-level слой (z-order, активация,
  drag/resize/close); menu popup/вложенные подменю/иконки; toolbar
  actions; status message + технические поля.
- **Меню не смешивается с toolbar.** Исторически glyphon рисовал все
  text-runs после всех прямоугольников, и текст toolbar просвечивал
  через popup; временно он отбрасывался по popup rect. В 0.15 маскировка
  удалена и заменена упорядоченным display list (см. выше).
- **Standalone.** Формы идут в top-level, `mdi=true` коэрсится в
  top-level; меню/statusbar нет. Full/standalone проверены в браузере.
- **Modal.** `modal_scope` (`application`/`window`); application modal
  блокирует shell, MDI и top-level; клавиатурный guard.
- **Lua.** `App:setToolBar`, `setToolBarVisible`, `setStatusMessage`/
  `showMessage`, `setStatusVisible`, `setStatusInfo`; menu actions
  выполняет main-сессия через `App:open_lua`/`App:close`.
- **0.14.8.** Загрузка `QMainWindow`/`QMenuBar`/`QStatusBar`/`QToolBar`
  из `.ui` реализована (см. отдельный раздел выше); golden shell и
  браузерная интеграция закрыты в 0.14.9.
- **Тесты.** `cargo test --workspace` — 132 passed (app 37, form-xml 8,
  rqt 87); `mix test` — 29 passed; wasm check — ok. Live:
  `?main=test_main.lua` (menu/toolbar/status/MDI/top-level/exit),
  `?lua=demo_icons.lua&ui=test_dialog01.ui` (standalone top-level).

### MDI: ресайзабельность, min/max и курсоры границ

- `QMdiSubWindow` получил `resizable`, `min_content`, `max_content`.
  `can_resize()` учитывает флаг, maximized/minimized и вырожденный диапазон;
  `set_size_constraints`/`set_frame`/`resize_content`/`set_content_geometry`
  клампят размер по `minimumSize`/`maximumSize` формы.
- Интерактивные зоны: 4 px по краям, 8 px в углах, 12 px у нижне-правого
  grip; новые `QMdiSubWindowHit::Resize{Top,Bottom,Left,Right,*}`.
  Перетаскивание любой границы/угла держит противоположную грань на месте
  (`resized_frame`), максимум/минимум соблюдаются.
- Grip рисуется только при `can_resize()`.
- Курсор: при наведении на границу/угол winit `set_cursor` ставит
  `W/E/N/S/Nw/Ne/Sw/Se-resize`, вне границ — `Default`; во время drag
  курсор сохраняется, на `CursorLeft` сбрасывается.
- Флаг: `Form:setResizable(false)` (алиас `setSizeGripEnabled`),
  `sizeGripEnabled=false` из `.ui` или `windowFlags` с
  `MSWindowsFixedSizeDialogHint` (парсится `<set>` в form-xml).
- `Form:setMinimumSize`/`Form:setMaximumSize` (2 аргумента или таблица);
  изменения на лету клампят подокно и перелейавливают форму.
- Тесты: `resize_borders_follow_resizable_flag_and_constraints` (rqt),
  `resized_frame_anchors_opposite_edges_and_clamps`,
  `resize_cursor_follows_the_border_direction`,
  `form_resize_flag_and_constraints_follow_ui_and_commands` (app),
  `parses_window_flag_sets` (form-xml), e2e-фикстура `test_form_resize.lua`.
- Проверки: `cargo test --workspace` — 123 passed (app 28, form-xml 8,
  rqt 87); `mix test` — 25 passed; wasm check — ok.
- Вживую: курсоры `w-resize`/`nw-resize`/`e-resize`; перетаскивание левой
  границы сжимает форму, сохраняя правую; у `setResizable(false)` grip
  скрыт, курсор на границе — обычная стрелка.

### Формы: `Form:move` / `Form:resize`

- `form.lua`: `Form:move(x, y | {x,y})` шлёт `setFormProperty "position"`,
  `Form:resize(w, h | {w,h})` — `setFormProperty "size"`.
- Клиент (`app::on_command`): `position` (Map с `x`/`y`) двигает MDI-подокно,
  сохраняя размер; `size` (Size) меняет размер, сохраняя позицию. Оба
  обновляют `QMdiSubWindow.frame`, content rect и дерево виджетов.
  `Form:setGeometry({x,y,w,h})` по-прежнему двигает и меняет размер сразу.
- Проверено вживую (WSLg-Chromium): `Form:move(40,40)` перемещает форму без
  изменения размера, `Form:resize(380,220)` уменьшает её на месте.
  Координаты — content rect: заголовок MDI рисуется на 24 px выше, поэтому
  для видимого заголовка нужно `y >= 24`.
- Тесты: `form_position_and_size_keep_the_other_dimension` (app),
  `form move/resize sugar sends position and size` + фикстура
  `test_form_move.lua` (server).
- Проверки: `cargo test --workspace` — 117 passed (app 24, form-xml 7,
  rqt 86); `mix test` — 24 passed; wasm check — ok.

### 0.12.1: ячейки `<item>` и первый размер canvas

- `form-xml`: добавлен `LayoutCell { row, column, row_span, column_span }`
  (дефолт `0/0/1/1`), читаются атрибуты
  `<item row= column= rowspan= colspan=>` для widget, nested layout и spacer.
- `GridConfig` (массивы `row_span`/`col_span`/stretch) удалён из `form-xml`:
  модель не соответствовала `.ui`, где span — свойство элемента.
- `rqt`: `QLayoutItem::{Widget,Spacer,Layout}` несут `cell`; удалены мёртвые
  `position`/`span`, `QBoxLayout.grid_config` и round-robin заглушка.
- Тесты: `parses_item_cells_from_grid_layouts`,
  `box_layout_items_default_to_single_cell`,
  `invalid_cell_attributes_fall_back_to_defaults`.
- **Исправлен чёрный экран до ресайза окна.** winit-web присылает `Resized`
  до готовности `Gfx`, а `Window::inner_size()` остаётся `1x1`, поэтому
  surface настраивался на 1×1 и форма не рисовалась. Теперь последний размер
  из `Resized` хранится в `last_size`, применяется через `gfx.resize` сразу
  после создания `Gfx`, затем вызывается `request_redraw()`.
  Проверено в WSLg-Chromium: `canvas = 1280x657`, форма видна сразу, без
  ручного изменения размера окна.
- Проверки: `cargo test --workspace` — 116 passed (app 23, form-xml 7,
  rqt 86); `cargo check -p app --target wasm32-unknown-unknown` — ok;
  `mix test` — 23 passed.
- Документация: `widgets-plan.md` (0.12.1 ✅), `EPlan_Status.md`.

### App sugar: title/icon и задел под desktop-host

- Добавлен host API `set_app_property/3` → команда `setAppProperty`
  (`server/lib/e_platform/lua/api.ex`).
- `application.lua`: `App:setWindowTitle` и `App:setIcon` работают через
  `setAppProperty`; `resize`, `move`, `setGeometry`, `center`,
  `showMaximized/Minimized/Normal/FullScreen`, `visible`, `set_menu`
  объявлены и передаются тем же протоколом, но браузерный клиент их
  игнорирует — задел под desktop-бинарник с нативным окном вокруг
  встроенного браузера.
- Клиент: `windowTitle` → `document.title`, `windowIcon` → favicon
  (`<link rel=icon>`); URL/`data:`/относительные пути применяются напрямую,
  `img24:N`/`img24d:N` — нарезкой встроенного спрайта (см. ниже),
  `:/icons/...` остаются desktop-host.
- `AppChrome` + `icon_url` изолируют разбор свойств от побочных эффектов;
  добавлен юнит-тест `app_property_applies_browser_chrome_and_keeps_desktop_sugar_as_noop`.
- Серверный e2e-тест `app sugar sends browser chrome and reserves
  desktop-only properties` + фикстура `test_app_sugar.lua`.
- Проверки: `cargo test --workspace` — 107 passed (app 22, form-xml 4,
  rqt 81); `mix test` — 23 passed; `cargo check -p app --target
  wasm32-unknown-unknown` — ok (web-sys features `Document`, `Element`,
  `Node`, `HtmlLinkElement`, `HtmlHeadElement`).

### Иконки: нарезка IMAGELIST24 (tcs_qtgui)

- Из `tcs_qtgui/icons/IMAGELIST24.bmp` и `IMAGELIST24_DISABLED.bmp`
  сгенерированы `client/assets/icons/IMAGELIST24.png` и
  `IMAGELIST24_DISABLED.png` (белый фон → alpha 0, как в Qt
  `iconFromSprite`); оба PNG встроены через `include_bytes!`.
- Новый модуль `rqt::icons`: `spec_index` (нумерация Qt с 1, `0` = первый
  элемент), ленивое декодирование спрайта (`OnceLock`), нарезка ячеек
  24×24, `pixmap(spec)` и `png_bytes(spec)` для favicon.
- `QRasterImage`: добавлены `from_png_bytes`/`to_png_bytes`; `save_png`
  использует их, поэтому декод/энкод работает и на `wasm32`.
- `rqt` loader: `<iconset normaloff/disabledoff>` и runtime
  `setProperty("icon", "img24:N")` теперь режут спрайт; для нормальной
  спецификации автоматически подставляется `img24d:N` в `icon_disabled`.
- `app`: legacy-спецификация favicon кодируется в PNG data URL
  (`image/png`) и ставится заменой `<link rel="icon">` — без Blob/object
  URL. Проверено в headless Chromium (CDP): `document.title` меняется,
  `href` начинается с `data:image/png;base64,`, в `window.__epLogs` есть
  `apply app.windowIcon=png:... bytes`.
- Тесты: 4 теста `rqt::icons` + `set_property_icon_slices_img24_sprite` +
  расширенный `app_property_applies_browser_chrome_and_keeps_desktop_sugar_as_noop`.
- Проверки: `cargo test --workspace` — 112 passed (app 22, form-xml 4,
  rqt 86); `mix test` — 23 passed; `cargo check -p app --target
  wasm32-unknown-unknown` — ok.

### Актуализация документации (кодовых изменений нет)

- Зафиксирован фактический baseline: `cargo test --workspace` — 106 passed
  (app 21, form-xml 4, rqt 81); `mix test` в `server/` — 22 passed.
- `EPlan_Status.md` переписан: события 0.1–0.11 закрыты; «инфраструктура»
  QGridLayout/QFormLayout признана нерабочей.
- `widgets-plan.md` переписан: 0.12/0.13 разложены на шаги
  (парсер `<item row/column/rowspan/colspan>` → `QLayoutKind` → геометрия →
  span/alignment → golden), добавлены критерии готовности и риски.
- `questions.md` переписан: вместо отчётов о выполненных задачах — реальные
  открытые вопросы (stretch, политики QFormLayout, `create_empty_form`,
  `Form:center`, `modal_result`, App sugar, `invoke_qt`, таблицы, данные,
  review-пак, `tcs_back/db_migrate`).
- `docs/lua-module-composition.md` §12: отмечены реализованные `open_opts` →
  `init`, `open_lua`, модалки; `create_empty_form`, `Form:center`,
  `tableLayout`/`groupedHeader`, App sugar, `invoke_qt` и слой данных — нет.
- `docs/lua-event-loop-and-modal-forms.md`: добавлена пометка, что §4.3
  («чего нет») устарел — модалки и `open_opts` реализованы.
- Установлено расхождение: `form-xml::parse_layout` ставит `grid_config`
  в `None` (`lib.rs:237`), grid-ветка `widgets.rs:2004` недостижима,
  `position`/`span` в `QLayoutItem::Widget` нигде не читаются,
  `formLayout`/`labelAlignment`/`fieldGrowthPolicy` не парсятся,
  `QGridLayout` деградирует в hbox, `QFormLayout` — в vbox.

## 2026-11-24

### Tab-фокус и переходы по виджетам

- `app`: `Tab`/`Shift+Tab` переключают фокус по tab-цепочке активной формы через
  `QWidgetTree::focus_next`; `QLineEdit` получает `focused`, item-view —
  `table_focus`, остальные — только `has_focus`. При `Tab` в `QLineEdit`
  выделяется текст, фокус неактивных MDI-форм сбрасывается.
- При открытии и активации формы фокус ставится на первый focusable-виджет,
  как в Qt.
- В `QTableWidget`/`QListWidget`/`QTreeWidget` `Tab` сначала двигает `current`
  (row-major у таблицы, вниз/вверх у list/tree), а на краю view переводит фокус
  на следующий/предыдущий виджет; при открытом редакторе ячейки значение
  сначала коммитится.
- `QWidgetTree::set_focus` выставляет `current` у list/tree, если его не было.
- После mouse release `focused`/`table_focus` синхронизируются с фактическим
  фокусом `QWidgetTree`: отпускание мыши вне поля ввода или клик по
  неfocusable-области больше не сбрасывают фокус и выделение текста.
- `QLineEdit` сбрасывает выделение при потере фокуса/disable, как Qt6
  (`hasSelectedText=false`, курсор остаётся на месте); повторный focus-in
  выделение не восстанавливает. Escape и клик вне форм тоже снимают фокус.
- `QAbstractButton`: пока кнопка мыши удерживается, `pressed` следует за
  курсором — увёл мышь с кнопки, она отжимается, вернул — снова нажимается.
  `Clicked` приходит только при отпускании на кнопке; release вне формы
  очищает grab и не оставляет залипшую нажатую кнопку.
- `QRadioButton` теперь auto-exclusive как в Qt: `set_checked(true)` снимает
  другие radio-кнопки с тем же parent, повторный клик по выбранной не снимает
  её, а `autoExclusive=false` разрешает независимое поведение.
- `setProperty("checked", true)` для radio/check теперь идёт через
  `set_checked`, поэтому Lua-инициализация `radioButton_2` снимает
  `radioButton_1` (раньше обе оставались включёнными).
- Исправлено залипание `left_down` после перетаскивания формы/колонки: раньше
  release с активным `dragging`/`column_drag` выходил до сброса состояния.
- Drag формы не двигает содержимое maximized-окна; координаты и смещения
  проверены при `devicePixelRatio > 1` (курсор переводится в logical px).
- `dist` снова собирается в release-режиме (`trunk build --release`,
  `data-wasm-opt="0"` — без скачивания wasm-opt); debug-wasm 35 MB заменён на
  release ~9.9 MB, из-за которого мышь/перерисовка ощущались как зависание.
- Добавлен `client/scripts/serve_dist.py`: отдаёт `index.html` с `no-store`,
  hashed JS/wasm — с `immutable`; в `index.html` добавлены no-cache meta.
  Chrome больше не залипает на старом index/wasm после пересборки.
- Тесты: `create_form_focuses_first_focusable_widget`,
  `mouse_release_keeps_tree_focus`,
  `tab_cycles_focusable_widgets_in_active_form`, переходы внутри table/list/tree
  и на краях, пропуск disabled, изоляция фокуса по активной форме,
  `focusing_list_and_tree_selects_first_current`.

### MDI title bar: кнопки как в Qt и качество отрисовки

- `QMdiSubWindow` получил Qt-геометрию заголовка Fusion: высота 24 px, кнопки
  18×18, top 3 px, правый отступ 6 px, промежуток 2 px.
- Кнопки теперь описываются флагами (`QMdiWindowFlags`): close, minimize,
  maximize; при maximized рисуется restore, при minimized — restore + close.
  System menu/context help зарезервированы флагами, но по умолчанию выключены.
- Minimize сворачивает окно в иконку-заголовок внизу MDI-области; restore
  возвращает прежнюю геометрию и пересчитывает layout формы.
- Заголовок и кнопки перерисованы через `QPainter`: градиент title bar,
  hover-подсветка, иконки `—`, `□`, `❐`, `×` линиями вместо кривых глифов.
- `append_paint_commands` теперь корректно растеризует `FillRoundedRect`
  (раньше скругление терялось и hover рисовался белым квадратом); hover
  задан светло-голубым/серым, maximize/restore получили верхнюю планку.
- Тесты: `title_buttons_minimize_and_restore_like_qt`,
  `minimize_button_collapses_and_restores_form`.

### Qt-layout: QGroupBox, вложенные layout-ы и позиции детей

- Прямые дочерние виджеты без layout (`QGroupBox` с `QRadioButton` по
  `geometry`) снова позиционируются относительно родителя: `explicit_position`
  заполняется в loader-е и применяется после раскладки.
- Layout, установленный на `QGroupBox`, получает Qt-отступы: стилевые 9 px,
  плюс 3 px frame/contents inset и высоту заголовка сверху; layout-ы на виджетах
  (не на root) по умолчанию используют 9 px.
- `minimumSize`/`maximumSize` из `.ui` учитываются при раскладке; hint-ы
  элементов ограничиваются этими значениями.
- Распределение свободного места: расширяемые виджеты и спейсеры делят его
  поровну с учётом hint/min, при дефиците спейсеры сжимаются первыми, нулевые
  спейсеры не добавляют лишний gap.
- Размеры по умолчанию подогнаны под Qt: `QTableWidget` 256×192, пустой
  `QComboBox` 86 px, ширина `QCheckBox`/`QRadioButton` считается по bounding
  rect текста.
- Новый golden: `qt6_test_dialog02.json/png`, тесты
  `dialog02_geometry_matches_qt6_offscreen` и `dialog02_raster_compared_to_qt6`
  (mean diff ≈4.85); `dialog01` по-прежнему совпадает в пределах 1 px.

### Переименование крейта qt → rqt

- Каталог `client/crates/qt` переименован в `client/crates/rqt` (`git mv`),
  пакет `qt` — в `rqt`, workspace member и зависимость `app` обновлены.
- Импорты `qt::...` заменены на `rqt::...`; binary
  `qt_render` переименован в `rqt_render`.
- `tools/formgen/review.py` и `QT_PORT_PLAN.md` обновлены под `rqt`;
  `Cargo.lock` пересобран Cargo. Настоящий Qt6 (`tools/qt_reference`,
  `review/qt`, `qt6_*`) не затронут.

### Прокрутка, item-view и события (итерация 6)

- Добавлен `qt::scroll`: `QScrollBar` (range/value/page/single, policy, arrows,
  track, slider), `QScrollBarSubControl`, `scroll_area_layout` и Fusion-отрисовка
  `paint_scroll_bar`. Extent 14 px и минимальный ползунок 26 px сверены с Qt 6.2.4.
- `QTableWidget` получил горизонтальную прокрутку, `vbar`/`hbar`, закреплённый
  row-header, Qt-семантику range/page в строках и колонках, `reveal_current_column`.
- Добавлены `verticalScrollBarPolicy`/`horizontalScrollBarPolicy`
  (`AsNeeded`/`AlwaysOff`/`AlwaysOn`) для table/list/tree: загрузка из `.ui`,
  `setProperty`/`getProperty`, учёт в layout, отрисовке и hit-test.
- Добавлен `qt::item_view`: модели `QListWidget`/`QTreeWidget` с current/selection,
  вертикальной/горизонтальной прокруткой и Qt-единицами scrollbar-ов.
- `QWidgetTree`: scrollbar hit-test, drag ползунка, клики по track/стрелкам,
  колесо и Shift+wheel, клавиатура (стрелки/PageUp/PageDown/Home/End),
  scrollbar-клик не порождает item-event.
- app шлёт `CurrentCellChanged`/`CurrentRowChanged`/`CurrentItemChanged` и
  читает `currentRow`/`currentIndex`/`currentText` через `getProperty`.
- Golden-стенд: `scroll_table.ui`, Qt6 JSON/PNG; viewport/vbar/hbar и
  range/page/value совпадают в пределах 1 px, PNG mean diff ≈4.9.
- Ревью-пак расширен до 76 форм (категории `scroll` с policy-кейсами и `selection`).
- Удалены неиспользуемые crate-ы `ui-core` и `ui-render`; убрана лишняя
  зависимость `serde` из `form-xml`.
- Проверки: `cargo test --workspace` — 73 passed, `mix test` — 7 passed,
  wasm-check `app`/`qt` проходит.

### Qt-порт: план и фундамент

- Добавлен `client/QT_PORT_PLAN.md` с этапами перехода на Qt-совместимый Rust-рантайм.
- Создан workspace-crate `client/crates/qt`:
  - `core`: `QObject`/`QObjectTree`, `QVariant`, `QEvent`/`QEventType`, `QObjectId`.
  - `gui`: `QPointF`, `QSizeF`, `QRectF`, `QMargins`, `QColor` (sRGB/linear), `QPalette` Fusion, `QFont`, `QFontMetrics`, `QPainter`, `QPaintCommand`.
  - `style`: `QStyle` trait, `QStyleOption`, `QStyleState`, `QFusionStyle`, `QPrimitiveElement`, `QControlElement`, `QPixelMetric`.
  - `widgets`: `QWidgetTree`, `QWidget`, `QSizePolicy`, `QFocusPolicy`, `QBoxLayout`, `QSpacerItem`, `QLayoutItem`, `load_ui()`.
- `QFontMetrics` использует cosmic-text и реальные метрики DejaVu Sans вместо `CHAR_W`.
- `QPainter` записывает Qt-подобные команды: `Save`, `Restore`, `Translate`, `SetClip`, `FillRect`, `StrokeRect`, `FillRoundedRect`, `DrawLine`, `DrawText`.
- `QBoxLayout` подогнан под Qt Fusion: margins 11, spacing 6, заполнение поперечной оси, приоритет спейсеров при распределении свободного места.
- Реализованы Qt-события: hit-test, фокус по клику, tab-order, `Clicked` для кнопок и `MouseButtonRelease` для остальных.

### Golden-стенд Qt6

- Добавлен `client/tools/qt_reference` (Qt6 + QUiLoader + offscreen QPA): выгружает геометрию, size hints, size policies, margins/spacing и метрики шрифта.
- Сгенерирован эталон `client/crates/qt/tests/fixtures/qt6_test_dialog01.json`.
- Тест `dialog01_geometry_matches_qt6_offscreen` сравнивает все именованные виджеты с Qt6 с допуском 1 px.

### Рендер Qt-дерева

- Добавлен модуль `qt::paint`: обход `QWidgetTree` в Qt-порядке (родитель → дети), clip по геометрии виджета, фон и текст через `QFusionStyle`/`QPainter`.
- `QDialog/QWidget/QGroupBox/QLabel/QLineEdit/QPushButton/QToolButton/QCheckBox/QRadioButton/QComboBox/QTableWidget` получили базовую отрисовку через `QStyle`.
- Текст рисуется по реальным `QFontMetrics`: слева для label/line edit, по центру для кнопок, со сдвигом после индикатора для checkbox/radiobutton.
- Добавлены тесты: порядок команд и текстов формы, центрирование текста кнопки.

### QTableWidget: модель и рендер

- `QVariant` получил полноценный `List(Vec<QVariant>)` вместо `StringList`, чтобы сохранять вложенные `tableData`.
- Добавлен `qt::item_model`:
  - `QTableModel`: row/column count, headers, cells, column widths, hidden columns, `apply_headers`, `apply_data`, `hide_column`.
  - `QTableWidget`: current row/column, selection mode, edit trigger, read-only, scroll/reveal.
- `QWidget` теперь владеет `Option<Box<QTableWidget>>`; `QWidgetTree::set_property` маршрутизирует Qt-свойства таблицы (`clear`, `rowCount`, `columnCount`, `horizontalHeaderLabels`, `columnWidths`, `hiddenColumns`, `hideColumn`, `tableHeaders`, `tableData`, `data`, `selectRows`, `readOnly`, `editTriggers`, `currentRow`).
- Клик по таблице выбирает ячейку/строку; `selectRows` закрепляет колонку 0.
- `qt::paint` рисует заголовок, сетку, выделение, заголовки колонок и ячейки, скролл-диапазон через `QTableWidget`.
- Тесты: применение headers/data, выделение/скролл, клик по таблице, отрисовка заголовков и ячеек.

### Интеграция qt-слоя в app

- `app` теперь хранит формы как `QtForm { QWidgetTree, root }`; старый `ui_core::Node` остался только как переходный слой для MDI-хрома.
- `createForm` использует `qt::widgets::load_ui`, Qt-раскладку и реальные метрики шрифта.
- `setProperty`/`setFormProperty`/`getProperty`/`getFormProperty` работают через `QWidgetTree`; таблицы обновляются через `QTableWidget`.
- Рендер формы переведён на `qt::paint` + `QFusionStyle` + `QPainter` с конвертацией `QPaintCommand` в батчи wgpu (clip, заливки, рамки, текст). Старые самодельные рендер-функции удалены.
- События мыши идут через Qt hit-test/`mouse_press`/`mouse_release`; фокус, tab-order и выбор ячейки таблицы используют Qt-слой.
- Кнопки теперь шлют `Clicked`, остальные — `MouseButtonRelease`; фикстуры `test_01.lua`/`test_02.lua` и серверные тесты обновлены под Qt-семантику.
- Проверено в браузере: форма рисуется в Fusion-палитре, `A умножить B` даёт `2` через `Clicked` → Lua → `setProperty`.
- `QTableWidget` в приложении читает/пишет ячейки, текущую строку/колонку и scroll через новый model.

### QMdiArea/QMdiSubWindow и финальная зачистка app

- Добавлен `qt::mdi`:
  - `QMdiSubWindow<T>` с frame/content rect, title bar, close/maximize, resize handle и hit-test.
  - `QMdiArea<T>` с cascade, активацией/подъёмом, удалением, перемещением, resize, maximize/restore, viewport.
- `app` переведён на `QMdiArea<String>`; самодельные `Card`/`CardHit`/`cards`/`form_order`/`cascade` удалены.
- Перетаскивание заголовка, resize за правый нижний угол, maximize/restore и активация работают через Qt MDI.
- MDI-хром рисуется в Fusion-палитре: активный заголовок `Highlight`, неактивный `Button`, кнопки `×`/`□`.
- `setFormProperty("geometry")` синхронизирует frame/content и пересчитывает layout; content всегда ниже title bar.
- `app` больше не зависит от `ui_core` и `ui-render`; `qt_form_node`/`qt_widget_node` удалены.
- Проверено в браузере: заголовок сверху, контент ниже, активная синяя полоса, кнопки закрытия/максимизации.

### PNG-диффы и software raster

- Добавлен `qt::raster`: программный рендер `QPaintCommand` в RGBA-изображение с clip, заливками, рамками, линиями и текстом через cosmic-text/swash.
- `QRasterImage`: `new/fill/pixel/blend`, сохранение/чтение PNG.
- Qt6-стенд `tools/qt_reference` теперь умеет сохранять PNG формы (`argv[2]`), добавлен эталон `qt6_test_dialog01.png`.
- Тест `dialog01_raster_compared_to_qt6` рендерит форму нашим растеризатором и сравнивает с Qt6 PNG: средняя разница каналов ≈2.5/255, отличающихся (>30) пикселей ≈2.6%.
- `QFusionStyle` для `QLineEdit` рисует `Highlight`-рамку в фокусе, как Qt.

### QAbstractItemView/delegate, редактирование и сортировка

- Добавлены `QAbstractItemModel` и `QAbstractItemDelegate` + `QStyledItemDelegate`; `QTableModel` реализует model-интерфейс.
- В `QTableWidget` добавлен `QTableWidgetEditor`: `begin_edit`, `begin_edit_current`, `editor_insert`, `editor_backspace`, `commit_editor`, `cancel_editor`, `is_editing`.
- `app` больше не хранит буфер ячейки: редактирование идёт через `QTableWidget`/delegate, `CellEdit` хранит только form/table.
- `paint` рисует текст редактора и каретку в редактируемой ячейке.
- Добавлены `QSortOrder`, `sort_by_column`, `toggle_sort`, индикатор `▲/▼` в заголовке.
- Добавлен resize колонок: `column_resize_handle_at`, `resize_column`; в `app` перетаскивание границы заголовка меняет ширину.
- Клик по заголовку сортирует таблицу, клик по границе — начинает resize.
- Тесты: delegate-редактирование, сортировка asc/desc, toggle, resize-хэндлы скрытых колонок.
- В браузере проверена `test_dialog02`: таблица с `Col 1/Col 2`, тремя строками данных, checkbox-ами и Fusion-контролами.

### QLineEdit: каретка и позиция курсора

- Добавлен `QWidget.cursor_position`; `QLineEdit` хранит позицию курсора.
- Клик внутри поля ставит курсор по ближайшей границе символа (`cursor_at_x`).
- Ввод/Backspace работают в позиции курсора, а не только в конце строки; `setText` ставит курсор в конец.
- `QPainter`/`paint` рисует каретку у focused `QLineEdit`.
- В `app` добавлены `ArrowLeft/ArrowRight/Home/End` для перемещения курсора в line edit.
- Canvas-растр `dialog01` теперь визуально содержит каретку, как Qt6-эталон.

### QLineEdit: мигание, выделение, толщина каретки

- Мигание каретки приведено к Qt-модели `QApplication::cursorFlashTime`: цикл 1000 ms (500 видима / 500 скрыта), сброс при клике/вводе/смене фокуса.
- Для wasm время берётся из `performance.now()`; убран `std::time::Instant`, который приводил к `unreachable` в браузере.
- Добавлено выделение текста:
  - Shift+клик и drag мышью;
  - Shift+←/→/Home/End;
  - Ctrl+A;
  - выделение рисуется `Highlight`, текст — `HighlightedText`.
- Ввод и Backspace заменяют/удаляют выделение; Delete удаляет символ/выделение.
- Rect-батчи в `gfx` теперь выравниваются по физическим пикселям, поэтому каретка постоянной толщины независимо от позиции.
- Проверено в браузере: клик ставит курсор, ввод вставляет в позицию, Shift+клик выделяет символ.
- Мигание проверено двумя скриншотами: различается только область каретки, 2 физических пикселя при DPR 1.5 (стабильная толщина).
- Для сверки с Qt использованы реальные контракты `QLineEdit::cursorPositionAt`, `selectionStart/End`, `selectAll`, `QApplication::cursorFlashTime` и роли палитры `Highlight/HighlightedText`.

### Защита от потерянного press

- `QWidgetTree::mouse_release` больше не требует флага `pressed`, если release пришёл над кнопкой; если press начался на другом виджете, клик по-прежнему отменяется.
- Добавлен тест `button_release_without_seen_press_still_clicks`.
- Это защита от web-потерь press, но реальная причина «через раз» оказалась в изоляции клиентов (см. ниже).

### Изоляция клиентов: причина «через раз»

- Найдена реальная причина: несколько браузерных клиентов подключались к одному `app:demo`; сервер рассылал `getProperty` всем, и первый ответивший клиент (например, старый headless-клиент) возвращал свои значения. Отсюда `22 * 33 = 726` только когда отвечал актуальный таб.
- Сверено с эталоном: `tcs_qtgui/tcscore.cpp:5` (`generateUniqueAppId`, задуман UUID клиента) и `startDialog(applicationId, ...)`; у нас `app` был общим `demo`.
- В wasm-клиенте `app` по умолчанию теперь UUID экземпляра на каждую страницу через `crypto.randomUUID()` (как `QUuid::createUuid()`), fallback — time/random; `?app=...` по-прежнему задаёт общий id явно.
- Удалён тестовый headless-клиент, висевший на `app:demo`.
- Проверено в браузере: `A=22`, `B=33`, два нажатия подряд — оба раза `A*B=726`; логи показывают `reply lineEdit_1=22`, `reply lineEdit_2=33`, `apply lineEdit_3=726`.
- Серверный тест усилен: `22 * 33 = 726`.

### Контролы: checkbox/radio/combo/list/tree

- `QCheckBox`/`QRadioButton`: `checked`, toggle по клику, auto-exclusive для radio в одном parent.
- Отрисовка checked: `✓`/`●`, состояния через `QStyleState::SELECTED`.
- `QComboBox`: отрисовка текущего item по `items/currentIndex` + стрелка.
- `QListWidget`: строки, current row, выделение.
- `QTreeWidget`: `headerLabels` + рекурсивные `data/text/columns/children`.
- `form-xml` теперь парсит direct child `<widget>` без layout (нужно для radio/QGroupBox с явной геометрией).
- Размеры `QListWidget/QTreeWidget` исправлены на Qt-подобные 256×192.

### Генератор форм и review-пак

- Добавлен `tools/formgen/review.py`: генерирует **50** `.ui` форм по категориям
  `basic`, `spacing`, `controls`, `tables`, `groups`.
- Добавлен `crates/qt/src/bin/qt_render.rs`: `.ui` → наш PNG через `qt::raster`.
- `scripts/review.sh` собирает формы, Qt6 offscreen PNG, Rust PNG, diff-метрики и `review/index.html`.
- Созданы `review/manifest.json`, `review/comments.md` (стабильные id), `review/forms/`, `review/qt/`, `review/rust/`.
- Review-сервер запущен на `http://172.25.21.226:8090/` (Windows: `http://<WSL_IP>:8090/`).
- Исправлен UUID клиента: `randomUUID()` недоступен на HTTP (не secure context), теперь UUID v4 из `crypto.getRandomValues` с fallback.

### QSS, QGroupBox и disabled-стили

- Добавлен `qt::qss`: парсер QSS-подмножества.
  - Селекторы: класс, `#objectName`, `:enabled/:disabled/:focus/:hover/:checked`, `*`.
  - Свойства: `background(-color)`, `color`, `border`, `border-width/color/radius`,
    `padding`, `selection-background-color`, `selection-color`, `font-size/weight`.
- QSS наследуется от родителей: root `styleSheet` применяется к дочерним виджетам по их селекторам.
- QSS применяется в `paint`: фон, скругление, рамка, цвет текста, selection, padding.
- `QGroupBox`: title теперь рисуется, layout получает верхний отступ под заголовок.
- `enabled/visible` из properties теперь синхронизируются с полями `QWidget` (disabled QSS работает).
- RadioButton рисуется круглым `○/●`, checkbox — `☐ + ✓`.
- В генератор добавлена категория `qss` (6 форм), всего стало **56** форм.
- `review/comments.md` пополняется новыми id без затирания старых комментариев.

### Правки по первому ревью вёрстки

- Кнопки: минимальная ширина 80 по умолчанию; первый `QPushButton` в форме помечается default и рисуется с `Highlight`-рамкой.
- Первый focusable-виджет получает фокус при рендере галереи (`focus_first`), у `QLineEdit` видны фокус и каретка.
- `QBoxLayout`: если нет expanders, Preferred-виджеты (label/check/radio/groupbox) делят slack; если расти нечему — slack распределяется в промежутки (before/between/after). Это выровняло `05` и `10–19`.
- RadioButton: круглый `○`, выбранный — чёрная точка `●`; ComboBox рисует стрелку даже без items.
- `QTreeWidget`: заголовки рисуются, даже если нет data; заголовки колонок в таблице — labels или номера.
- `QTableWidget`: добавлен vertical row header (номера строк), column header с номерами/labels, grid только по реальным строкам/колонкам, пустая область справа/снизу не расчерчивается, первая ячейка выбирается по умолчанию, есть нижняя линия.
- `QGroupBox`: title больше не перекрывает содержимое, layout получает верхний отступ.
- Обновлена галерея: 56 форм, новые PNG Qt/Rust и diff.

### Правки по второму ревью вёрстки

- `05`: каретка `QLineEdit` больше не сливается с предыдущим символом — добавлен 1 px зазор после текста.
- `12`: убран лишний 4 px default-padding у `QLabel`; текст снова начинается от геометрии Qt-метки, слово `Left` не обрезается.
- `17`/`22`: `StrokeRect`/`StrokeRoundedRect` растеризуются по целым пикселям — верх/низ/бока рамки одинаковой толщины 1 px; у `QLineEdit` и `QComboBox` скругление radius 2.
- `20`: чёрная точка radio уменьшена (~6 px при внешнем круге ~13 px), внешний круг рисуется серой окантовкой.
- `26`: `QTreeWidget` всегда рисует header; при отсутствии `headerLabels` используется Qt-дефолт `1`.
- `30–41`: таблица рисует все четыре границы каждой ячейки; границы header/row-header темнее (`Dark`), body-grid — `Mid`; header больше не сливается с телом.
- `30+`: таблица приведена к Qt-дефолтам: header 21 px, строка 30 px, колонка 100 px, frame 1 px; пустая область не расчерчивается.
- `43`: checkbox/radio border рисуется ровно 1 px со всех сторон.
- Галерея пересобрана: 56 форм, все PNG Qt/Rust обновлены.

### QPushButton: Fusion-градиент, состояния и иконки

- Изучен Qt 6.2.4 `QFusionStyle`: `PE_PanelButtonCommand`, `CE_PushButtonLabel`, `QFusionStylePrivate::buttonColor/outline/highlightedOutline`.
- `QColor`: `lighter/darker` приведены к Qt-семантике (по каналам с округлением), добавлены `with_value`, `with_saturation_factor`, `blend`, `lerp`, `q_gray`.
- Кнопка теперь рисуется как Qt Fusion:
  - normal: вертикальный градиент `buttonColor.darker(104)` → `lighter(124)..lighter(102)`;
  - hover (если выставлен `State_MouseOver`): более светлый `buttonColor`;
  - pressed/checked: сплошной `buttonColor.darker(110)`;
  - default: подмешивается `highlightedOutline`, синяя рамка;
  - disabled: рамка `outline.lighter(115)`, тот же градиент, серый текст;
  - рамка radius 2, внутренняя белая линия `rgba(255,255,255,30)`.
- Добавлены `FillLinearGradientRect`, `FillRoundedRect` (реальное скругление), `DrawPixmap`; поддержаны в software-raster и wgpu-батче.
- Иконки: парсинг `<iconset>`/`normaloff`/`disabledoff`, загрузка PNG в `QPixmap`.
  - размер по умолчанию 16×16, содержимое кнопки inset 1 px;
  - иконка и текст центрируются группой, между ними 2 px после центрирования (Qt `iconSpacing/2`);
  - disabled использует disabled-pixmap/QIcon::Disabled;
  - pressed/checked сдвигает иконку и текст на 1 px.
- `focus_first` теперь ставит фокус только в `QLineEdit` (как нужно для каретки), а не в первую кнопку — убирает ложную синюю рамку в галерее.
- Добавлена форма `basic/56_button_states`: normal/default/checked/disabled/icon-only; галерея теперь **57** форм.

### Правки по следующему ревью

- `24`: radio теперь рисуется настоящими эллипсами (`FillEllipse`), а не глифами: внешний круг, белая внутренняя заливка и чёрная точка имеют общий центр; точка увеличена до Qt-размера.
- `21`: `QComboBox` больше не плоский — использует Fusion-градиент кнопки, рамку radius 2 и маленькую треугольную стрелку вниз (`alpha 160`), а не крупный глиф. Стрелка сдвинута на 3 px левее; после сравнения с Qt треугольник увеличен на 1 px по ширине и высоте: 8×5.5, bbox `394..401` / `136..141` (Qt 7×5, `394..400` / `137..141`). Треугольник растеризуется с 4×4 supersampling, поэтому нижний пиксель не выглядит «длинным».
- `26`/`27`: `QTreeWidget` header теперь реально внутри рамки: верхняя граница не затирается header-градиентом, `1` находится в ячейке заголовка; header/row-header получили градиент Qt `CE_HeaderSection`.
- Нижние границы header-ячеек списков/таблиц больше не размазываются на 2 px: 1-px линии в raster теперь привязываются к целому пикселю (`round`), поэтому граница совпадает с Qt (одна строка).
- `33`: ряд header-ячеек больше не рисует свою нижнюю линию — границы рядов дают единую сетку; последняя линия теперь на `content_bottom - 1` (как Qt), а не на 1 px ниже. У `4` одна линия снизу.
- Ширина row-header таблицы теперь адаптивная, как `QHeaderView::sizeHint`/`sectionSizeFromContents(CT_HeaderSection)`: берётся максимальный номер строки, bold-шрифт Qt, `ceil(advance + 4)` с минимумом 14 px. `36` (1..5) → 14 px, `37` (1..10) → 24 px, `100` строк → 34 px. Ширина используется и в paint, и в hit-test/column resize.
- Вертикальная граница row-header/первый столбец теперь одна: все column-grid линии сдвинуты на 1 px влево (`body_x - 1`), совпадают с разделителем row-header (Qt `x=25` для `36`), без второй линии на `x=26`.
- `38`: текст строк/номеров строк клипается областью данных таблицы (`body_top..content_bottom`), поэтому частично видимая строка `11` больше не рисуется поверх нижней рамки; цифры обрезаются до границы, как в Qt.
- `38`: убрано специальное условие «не рисовать нижнюю grid-линию». Теперь рисуется реальная граница последнего ряда (`row_y(last) + rowHeight`) и **клип вьюпорта сам отрезает** всё, что выходит за пределы таблицы; синтетической линии на `content_bottom - 1` больше нет. Для полностью видимых строк граница остаётся.
- Выделенная ячейка таблицы без фокуса теперь рисуется как Qt `PE_FrameFocusRect`: градиент `highlightedOutline` с alpha 30 (`lighter(160)` → base), а не сплошной яркий `Highlight`; текст в таком состоянии — `Text`. При фокусе таблицы остаётся обычный активный `Highlight` + `HighlightedText`. Пиксели совпадают с Qt (232/246/254 → 230/238/244).
- Добавлена внутренняя focus-рамка выделенной ячейки (как Qt `PE_FrameFocusRect`, цвет `#b1c9d7`) — одна линия поверх grid, а не двойная серая; внешняя grid-линия остаётся серой `#ababab`, внутренняя — голубой.
- Таблицы переведены на Qt-семантику current/selection/focus:
  - добавлен `QItemSelectionModel` (current + selected set, clear/select/prune);
  - `current` и selection разделены: `ensure_current` не выделяет ячейку;
  - клик ставит current + selection, режим SelectRows выбирает всю строку;
  - focus-rect рисуется только когда таблица имеет фокус, selection — отдельно;
  - `focus_first` фокусирует первый `QLineEdit`, иначе первый `QTableWidget`;
  - `focusInEvent`-аналог ставит current на первую ячейку без selection.
- `48`: при фокусе на `QLineEdit` таблица неактивна и current-ячейка больше не подсвечивается, как в Qt; `30–41` остаются с focus-rect.
- Тесты: `current_and_selection_are_separate_like_qt`, `focus_first_uses_line_edit_or_table_and_current_not_selected`; всего `cargo test --workspace` — 75 passed.
- `15`: `QLineEdit` и `QPushButton` теперь одинаковой ширины: pushbutton/combo рисуются на полную ширину rect (компенсация отсутствия AA в нашем stroke), правые границы совпадают с Qt (`x=408`).
- Тесты: добавлен `row_header_width_adapts_to_row_numbers`; всего `cargo test --workspace` — 73 passed.
- `30`: заголовки столбцов и строк таблицы рисуются градиентными секциями; границы header остаются тёмными.
- `42`/`43`: `QGroupBox` frame начинается ниже title (`max(fontHeight, indicatorHeight)+3`), поэтому `Group 0`/`Group 1`/`Nested` находятся НАД блоком, а не внутри.
- `42`: checkbox больше не с чёрной рамкой — контур тот же серый, что у radio (`window.darker(150)`); checkbox и radio выровнены по левому краю, оба индикатора `fontPixelSize+1` (15 px), поэтому вертикальная линия проходит через один и тот же x.
- Radio border переведён с двух заливок на `StrokeEllipse` (1 px, AA): кольцо стало тонким и чётким, как Qt, без «размытой» толщины.
- Checkmark checkbox теперь рисуется Qt-путём из двух сегментов (`pad=1+0.13w`, `penWidth≈1.8`, `alpha 210`) вместо глифа; радио-точка центрирована, `alpha 180`, радиус подобран по пиксельному bbox Qt (`outlineRadius/1.8`, тёмное ядро 6×6).
- Пиксельные размеры индикаторов сверены с Qt: checkbox 14×14, radio 14×14; Rust был 15×15. Оба возвращены к `fontPixelSize` (14 px), radio-координаты округлены до целых пикселей.
- `DrawLine` растеризуется с AA для произвольных (в т.ч. диагональных) отрезков в software-raster и wgpu-батче.
- Выровнены Qt-размеры `QCheckBox/QRadioButton` (23 px) и `QComboBox` (25 px); исправлено отрицательное распределение slack в layout (list/tree теперь делят высоту как Qt).
- Review-галерея пересобрана: 57 форм.

### Проверки

- `cargo test --workspace`: 75 passed.
- `mix test` в `server/`: 7 passed (известный редкий флейк 100 ms channel timeout, повторный прогон зелёный).
- `cargo check -p app --target wasm32-unknown-unknown`: passes.
- `cargo check -p qt --target wasm32-unknown-unknown`: passes.
- Golden-геометрия `test_dialog01` совпадает с Qt6 offscreen с допуском 1 px.
- PNG-дифф `test_dialog01`: mean diff ≈2.5, >30 diff ≈2.6% пикселей.
- Ручная проверка в браузере: Fusion-форма, `Clicked` на кнопке, `A=22`, `B=33`, повторные нажатия дают `A*B=726`; MDI-рамка, таблица `test_dialog02`.
