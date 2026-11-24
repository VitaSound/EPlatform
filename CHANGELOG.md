# Changelog

## 2026-11-24

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
