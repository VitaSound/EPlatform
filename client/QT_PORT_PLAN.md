# План Qt-порта на Rust

Цель: Qt-совместимый рантайм на Rust с максимальным совпадением логики и графики
с Qt Widgets для форм EPlatform. Это clean-room реализация поведения Qt, без
копирования исходников Qt.

## Принципы

- Публичные типы и понятия повторяют Qt: `QObject`, `QWidget`, `QLayout`, `QStyle`,
  `QPainter`, `QFontMetrics`, `QEvent`, `QUiLoader`.
- Внутренняя реализация — Rust: арена объектов и `QObjectId` вместо сырых указателей,
  явные владельцы вместо ручного `delete`.
- Источник истины для графики — эталонные PNG, отрендеренные настоящим Qt6
  (offscreen QPA), а не субъективное сравнение.
- Серверный протокол (Phoenix + Lua + `.ui` XML) не меняем.

## Этапы

1. **Qt core.** `QObject`-дерево, `QVariant`, `QEvent`/`QEventType`, геометрия
   `QPointF`/`QSizeF`/`QRectF`, свойства и поиск объектов. Тесты на parent/child,
   свойства и конвертацию из `form_xml::PropValue`.
2. **Qt GUI.** `QColor` (sRGB/linear), `QPalette` (Fusion), `QFont`,
   `QFontMetrics` на cosmic-text, DPI. Тесты палитры и метрик.
3. **Painting/style.** `QPainter` (команды → wgpu), `QStyle`/`QFusionStyle`:
   `drawPrimitive`, `drawControl`, `drawComplexControl`, `pixelMetric`,
   `sizeFromContents`, `subElementRect`. Стенд golden-тестов против Qt6 offscreen:
   `tools/qt_reference` выгружает геометрию/метрики формы, снимки лежат в
   `crates/rqt/tests/fixtures/`, тест `dialog01_geometry_matches_qt6_offscreen`
   сравнивает нашу раскладку с Qt (допуск 1 px).
4. **Widgets/layout/loader.** `QWidget`, `QBoxLayout`, `QGridLayout`,
   `QFormLayout`, `QLabel`, `QLineEdit`, `QPushButton`, `QCheckBox`,
   `QRadioButton`, `QGroupBox`, `QUiLoader`-аналог на базе `form-xml`.
5. **События и интеграция.** `QEvent`-фильтры, фокус, tab-order, `Clicked` для
   кнопок и `MouseButtonRelease` для остальных, замена текущего `ui-core` в
   приложении без изменения протокола.
6. **MDI.** `QMdiArea`/`QMdiSubWindow`: рамка, активация, перетаскивание,
   изменение размера, maximize, системное меню, cascade/tile.
7. **Модель таблиц.** `QAbstractItemView` + `QAbstractItemModel` + delegate:
   `QTableWidget`, headers, selection, editing, virtual scroll, resize/sort.
8. **Стилизация и остальное.** QSS-подмножество, палитры, иконки,
   `QComboBox`, `QListWidget`, `QTreeWidget`, `QSplitter`, меню/тулбар.

9. **Генератор форм и визуальный ревью-пак.** Запускается параллельно этапу 8,
   чтобы накапливать замечания по вёрстке на большом покрытии, а не на двух фикстурах.
   - `tools/formgen`: детерминированная генерация большого массива `.ui`-форм
     (категории + manifest): все поддерживаемые классы, H/V/Grid/Form layouts,
     spacing/margins, выравнивания, size hints/policies, длинный/короткий текст,
     disabled/checked/focus, вложенные группы, таблицы с разными размерами,
     граничные размеры, пустые/переполненные контейнеры.
   - Имена форм и виджетов стабильные (`gallery/<category>/<case>.ui`,
     `objectName` с индексом), чтобы комментарии не терялись между прогонами.
   - Для каждой формы рендерить два PNG: наш `rqt::raster` и Qt6 offscreen
     через `tools/qt_reference`; считать mean/max diff и сохранять метрики.
   - Статический ревью-пак `review/`:
     - `index.html` — категории, side-by-side «Qt / Rust», diff, размеры;
     - `manifest.json` — форма, категория, покрываемые свойства;
     - `comments.md` — место для твоих комментариев по `form_id`/категории;
     - `screens/` — PNG-снимки.
   - Команда регенерации (например, `./scripts/review.sh`) пересобирает формы,
     снимки, diff и HTML, не затирая `comments.md`.
   - Опционально in-app gallery: `?gallery=1` или отдельный route для просмотра
     форм в живом клиенте с подсветкой `objectName`/rect по клику.
   - Definition of done: >=50 форм, все рендерятся, ревью-пак открывается локально,
     комментарии привязаны к стабильным id, регрессии видны в diff.
   - Статус: pack расширен до 57 форм (включая QSS и button states), `review/index.html`,
     `review/comments.md`, `scripts/review.sh`, сервер на порту 8090.
   - Первый цикл правок по комментариям выполнен: кнопки/default/focus,
     layout slack/gaps, radio/combo, tree headers, таблицы с row/column header,
     QGroupBox title, QSS/disabled.
   - Второй цикл: caret gap, QLabel padding, 1 px stroke/rounded lineedit,
     radio dot, QTree header fallback, полная сетка таблицы и тёмные header-линии.
   - Третий цикл: Fusion-градиент QPushButton (normal/hover/pressed/default/
     disabled), иконки `<iconset>` и QPixmap, `basic/56_button_states`.
   - Четвёртый цикл: radio ellipses, combo gradient/small arrow, Tree/Table header
     gradients, QGroupBox title над frame, Qt-размеры checkbox/radio/combo,
     исправлено negative slack в layout.
   - Fifth cycle: Qt-like table current/selection/focus: `QItemSelectionModel`,
     current != selection, focus-rect только при фокусе таблицы, форма `48`
     без подсветки, `30–41` с focus-rect.
   - Sixth cycle (выполнено): `QScrollBar` для `QTableWidget`/`QListWidget`/
     `QTreeWidget` — extent 14, min slider 26, track/thumb/arrows, wheel + drag +
     track/arrow clicks, keyboard Page/Home/End, viewport clipping, horizontal
     scroll колонок, hit-test/resize с учётом scrollbar extent; модели списка и
     дерева, selection/current, golden-геометрия со scrollbars и PNG-дифф.
     Отложено: popup `QComboBox` и collapse/expand узлов `QTreeWidget`.

## Критерии готовности

- Для фикстур `test_dialog01.ui`/`test_dialog02.ui` совпадение геометрии с Qt
  в пределах 1 px, визуальный diff ниже согласованного порога.
- Одинаковые события: кнопки шлют `Clicked`, остальные — `MouseButtonRelease` и
  `Close`, как `tcs_qtgui/invoke_mainwindow.cpp`.
- Все текущие серверные тесты и клиентские тесты зелёные.
- Ревью-пак `review/` содержит >=50 сгенерированных форм, side-by-side PNG,
  diff-метрики и `comments.md`; повторный прогон сохраняет комментарии.

## Следующий этап (итерация 6): прокрутка, item-view взаимодействие, события

Цель: превратить таблицы/списки/деревья из «рисунка по свойствам» в полноценные
прокручиваемые view с Qt-подобным выбором, клавиатурой, scrollbar-ами и
событиями, пригодными для Lua-логики. Текущее состояние: у `QTableWidget` есть
только вертикальный `scroll` и wheel; `QListWidget`/`QTreeWidget` не имеют
модели, выделения и прокрутки; `QComboBox` не открывает список; `QScrollBar`
отсутствует (`QStyle` знает лишь `ScrollBarExtent = 12`).

### 1. `rqt::scroll` — фундамент прокрутки

- `QScrollBar`: `orientation`, `minimum`, `maximum`, `value`, `page_step`,
  `single_step`, `QScrollBarPolicy` (`AsNeeded`/`AlwaysOn`/`AlwaysOff`),
  `is_needed`, `set_range`, `set_value`, `step_by`, `page_by` с clamp.
- Геометрия sub-controls как в Qt Fusion: `sub-line`/`add-line`, `sub-page`/
  `add-page`, `first`/`last`, `slider`; `slider_rect`, `value_from_pos`,
  `hit_test` (arrow/track/slider), минимальная длина ползунка.
- Отрисовка в `QStyle`/`paint`: track, groove, thumb-градиент, стрелки,
  состояния normal/hover/pressed/disabled, горизонтальная и вертикальная
  ориентации; толщина 12 px из `QPixelMetric::ScrollBarExtent`.
- `QAbstractScrollArea`-хелпер: `viewport_rect`, `vbar_rect`, `hbar_rect`,
  `corner_rect`, `extent`, учёт policy; единый для table/list/tree.
- Golden-стенд Qt6: выгружать rect-ы sub-controls и slider для нескольких
  range/value/page_step, зафиксировать в фикстурах.
- Тесты: арифметика range/value, геометрия ползунка, hit-test, page/step,
  policy и влияние на viewport.

### 2. Модели и прокрутка item-view

- `QTableWidget`: добавить горизонтальный offset колонок и content width;
  вертикальный/горизонтальный scrollbar-ы с `page_step`; viewport уменьшается
  на extent; row-header остаётся закреплённым, column-header скроллится
  горизонтально; клип, grid, selection и resize-handles считаются от viewport.
- `QListWidget`: модель строк (`items`, текущая строка, selection, row height,
  scroll), `currentRow`/`currentText`, keyboard/mouse selection, `ensure_visible`.
- `QTreeWidget`: модель дерева (`headerLabels`, вложенные `data`), flatten в
  видимые строки, current/selection, глубина/отступы, вертикальный scroll,
  раскрытие/сворачивание узлов (если поддержано `.ui`) и `ensure_visible`.
- `QComboBox`: `currentIndex`, стрелки вверх/вниз, `currentIndexChanged`,
  базовая popup-область со списком item-ов и выбором мышью/клавиатурой.
- Фокус: `focus_first` и tab-order включают list/tree; current != selection,
  как уже сделано для таблицы.

### 3. Ввод в qt-слое

- `QWidgetTree::mouse_press/drag/release`: hit-test scrollbar-ов, перетаскивание
  ползунка, клик по track (page step), клик по стрелкам (single step).
- Wheel над item-view: вертикальный и Shift+wheel горизонтальный; шаг как
  `QApplication::wheelScrollLines` (3) × single step; сохранить текущий знак
  winit/web-делт.
- Клавиатура: стрелки, PageUp/PageDown, Home/End (и Ctrl+Home/End) для
  table/list/tree; перемещение current с `ensure_visible`; не ломать
  редактирование ячейки и `QLineEdit`.
- Тесты: drag ползунка, track/arrow клики, wheel, keyboard-навигация,
  hit-test с учётом extent и горизонтальной прокрутки.

### 4. Интеграция в app и протокол

- `app`: маршрутизировать мышь/колесо/клавиши в scrollbar-ы и item-view;
  обновлять `table_focus`; не дублировать scroll-логику в app.
- События в `formEvent` (сверить имена и payload с эталоном `tcs_qtgui`):
  таблица — `CurrentCellChanged`/`CellChanged`; список — `CurrentRowChanged`,
  `ItemClicked`; дерево — `CurrentItemChanged`, `ItemClicked`; combo —
  `CurrentIndexChanged`. Существующие `Clicked`/`MouseButtonRelease`/`Close`
  не менять.
- `getProperty`/`setProperty` для `currentRow`, `currentIndex`, `currentText`,
  `currentRowCell:*`, при необходимости `selectedItems`; серверный `DialogSession`
  зеркалит новые свойства.
- Обновить `test_02.lua` и серверный тест: выбор строки/узла, прокрутка и
  чтение current через `getProperty`; добавить e2e-проверку событий выбора.

### 5. Ревью-пак и golden-тесты

- Расширить `tools/qt_reference`: dump scrollbar/viewport rect-ов и PNG форм с
  переполнением; новые golden-фикстуры для table/list/tree со scrollbar-ами.
- `tools/formgen`: категории `scroll` (вертикальный/горизонтальный/оба/
  AlwaysOn/AlwaysOff/disabled, переполнение по строкам и колонкам) и
  `selection` (current/selected/disabled list/tree/table/combo); довести пак
  до >=70 форм, id стабильные, `comments.md` не затирается.
- PNG-диффы для форм со scrollbar-ами; порог не хуже текущего (mean ≈2.5,
  >30 diff ≈2.6%).
- Ручная проверка в браузере: колесо, drag ползунка, горизонтальная прокрутка
  широкой таблицы, клавиатура, выбор строки/узла и приход событий в Lua.

### 6. Зачистка и проверки

- Удалить неиспользуемые crate-ы `ui-core` и `ui-render` из workspace, если
  после интеграции на них не осталось ссылок.
- `cargo test --workspace`, `cargo check -p app --target wasm32-unknown-unknown`,
  `cargo check -p rqt --target wasm32-unknown-unknown`, `mix test`,
  `mix precommit`.
- Обновить `CHANGELOG.md` и статус этого плана.

### Статус итерации 6 (2026-11-24)

- `rqt::scroll`: `QScrollBar`, sub-controls, policy, `scroll_area_layout`; Fusion
  отрисовка track/thumb/arrows; extent 14 и slider min 26 сверены с Qt 6.2.4.
- `rqt::item_view`: модели `QListWidget`/`QTreeWidget`, current/selection,
  вертикальная и горизонтальная прокрутка, `sync_bars` в Qt-единицах.
- `QTableWidget`: горизонтальный offset/scrollbar, Qt-семантика range/page
  (строки/колонки), row-header закреплён, клип и hit-test по viewport.
- app: мышь/колесо/Shift+wheel, drag ползунка, стрелки/Page/Home/End, события
  `CurrentCellChanged`/`CurrentRowChanged`/`CurrentItemChanged`; scrollbar-клик
  не шлёт item-event.
- Golden: `scroll_table.ui` + Qt6 JSON/PNG; геометрия viewport/vbar/hbar и
  range/page/value совпадает в пределах 1 px; PNG mean diff ≈4.9.
- Ревью-пак: 76 форм (категории `scroll` с policy-кейсами и `selection`), комментарии сохранены.
- Политики `verticalScrollBarPolicy`/`horizontalScrollBarPolicy` (`AsNeeded`,
  `AlwaysOff`, `AlwaysOn`) для table/list/tree: `.ui`, set/getProperty, layout,
  paint, hit-test; Qt-поведение — `AlwaysOff` скрывает bar, но колесо/программный
  scroll остаются доступны.
- Зачистка: удалены неиспользуемые `ui-core` и `ui-render`.
- Проверки: `cargo test --workspace` — 73 passed; `mix test` — 7 passed;
  wasm-check `app`/`qt` проходит.

### Статус Tab-фокуса (2026-11-24)

- `app`: `Tab`/`Shift+Tab` идут по tab-цепочке активной формы, начальный фокус
  ставится на первый focusable при `createForm`/активации; неактивные формы
  теряют фокус.
- item-view: `Tab` двигает `current` (table row-major, list/tree по строкам), на
  краю view фокус уходит на следующий/предыдущий виджет; редактор ячейки
  коммитится перед переходом.
- `QWidgetTree::set_focus` инициализирует `current` у list/tree, как у table.
- Mouse release синхронизирует `focused`/`table_focus` с `QWidgetTree`, поэтому
  выделение и фокус сохраняются при отпускании мыши вне поля/на неfocusable-зоне.
- Layout: прямые дети без layout позиционируются через `explicit_position`;
  `QGroupBox`-layout получил Qt-отступы (9 px + frame/contents inset + title);
  учитываются `minimumSize`/`maximumSize`, спейсеры сжимаются первыми, нулевые
  gaps не добавляются. Размеры `QTableWidget`/`QComboBox`/check/radio подогнаны
  под Qt.
- Golden: `qt6_test_dialog02.json/png`, `dialog02_geometry_matches_qt6_offscreen`,
  `dialog02_raster_compared_to_qt6` (mean diff ≈4.85).
- Проверки: `cargo test --workspace` — все тесты зелёные; клиентский `trunk build`
  обновлён и проверен в headless Chromium (Tab/Shift+Tab меняют фокус).

### Критерии готовности этапа

- Переполненные table/list/tree показывают Qt-подобные scrollbar-ы (12 px,
  track/thumb/arrows), viewport корректно клипается, работает колесо, drag,
  track/arrow клики и клавиатура.
- Горизонтальная прокрутка таблицы работает, row-header закреплён, sort/resize
  hit-test не смещён из-за scrollbar-ов.
- Геометрия scrollbar-ов совпадает с Qt6 offscreen в пределах 1 px; PNG-дифф
  не хуже согласованного порога.
- Выбор в table/list/tree/combo доступен через `getProperty` и приходит в Lua
  событиями; старые события и тесты не сломаны.
- Ревью-пак >=70 форм, стабильные id, `comments.md` сохранён; все проверки
  зелёные.

### Риски и порядок

- Риск №1 — пиксельная точность Fusion-scrollbar (градиенты, стрелки,
  минимальный ползунок): сначала golden-стенд, затем отрисовка.
- Риск №2 — регрессии hit-test в app: все локальные координаты переводить
  через viewport-хелпер, а не через полный rect виджета.
- Риск №3 — новые события выбора могут конфликтовать с текущей семантикой
  `MouseButtonRelease`: имена/payload сверить с `tcs_qtgui` до реализации.
- Порядок: 1 → 2 → 3 → 4 → 5 → 6; каждый блок заканчивается тестами, чтобы
  не накапливать визуальные и событийные регрессии.
