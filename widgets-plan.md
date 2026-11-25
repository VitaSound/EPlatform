# EPlatform — план волны 0: события и layout-ы (rqt)

Обновлено: 2026-11-25 (актуализация по коду и прогонам тестов).
Статус: события волны 0 закрыты; **0.12 (`QGridLayout`) закрыт**
(0.12.1–0.12.5); **0.13 (`QFormLayout`) закрыт** (0.13.1–0.13.3).
Отдельный этап **0.14 (Application shell / главное окно)** реализован
в объёме 0.14.1–0.14.7; 0.14.8 (`.ui` main window) отложен, 0.14.9
(integration/golden) проверен в браузере, golden-снимки ещё нет.
Этап **0.15 PaintBatch / display list** закрыт: painter order сохранён на
всех слоях, маскировка текста под popup удалена.
Следующий шаг — рост в `QBoxLayout` (см. известный дефект 0.12);
0.13 больше не блокирует layout-фронт.
Источники: каталог Qt Widgets (<https://doc.qt.io/qt-6/qtwidgets-module.html>),
код `client/crates/{form-xml,rqt,app}`, `client/QT_PORT_PLAN.md`.

Обозначения: ✅ работает · ⚠️ частично · ❌ нет.
Сложность: **S** <1 дня · **M** 1–3 дня · **L** 3–7 дней · **XL** недели.

---

## Baseline (проверено 2026-11-25, после 0.13.3)

```
cargo test --workspace   → 161 passed
  app 41 · form-xml 12 · rqt 108
mix test (server/)       → 29 passed
```

## Browser testing — обязательное правило для UI

Любые изменения меню/тулбара/форм/MDI/top-level проверять **в настоящем
браузере**, а не только unit-тестами:

- Chrome DevTools MCP, WSLg Chromium: `http://127.0.0.1:9222`;
- full: `http://localhost:8080/?main=test_main.lua`;
- standalone: `http://localhost:8080/?lua=demo_icons.lua&ui=test_dialog01.ui`;
- клики/наведение — через CDP pointer events по canvas;
- сохранять скриншоты до/после и смотреть пересечения текста/фонов
  (именно так ловятся баги painter order).

## Что проверено по коду (и почему план переписан)

| Утверждение из прежней версии | Факт |
|---|---|
| «`GridConfig` парсится из XML» | было ❌ (жёстко `None`); **0.12.1**: `GridConfig` удалён, ячейки парсятся с `<item>` |
| «grid-позиции считаются round-robin» | было ⚠️ (недостижимая ветка); **0.12.1**: мёртвый round-robin удалён |
| «`QLayoutItem::Widget` хранит `position`/`span`» | было ⚠️ (не читались); **0.12.1**: заменены на `LayoutCell` из парсера |
| «`parse_value` понимает `formLayout`/`labelAlignment`/`fieldGrowthPolicy`/`horizontalSpacing`» | ❌ таких строк в `form-xml` нет вообще |
| «QGridLayout деградирует в vbox» | ❌ в hbox; **0.12.2**: вид layout-а теперь явный (`QLayoutKind`), grid раскладывается сеткой |
| «QFormLayout деградирует в vbox» | было ✅; **0.13.2**: двухколоночный движок, wrap и growth-политики |
| «инфраструктура готова» | ❌ готова только модель данных-заглушка; ни парсинга, ни геометрии |

Дополнительно: до 0.12 в фикстурах не было ни одной формы с
`QGridLayout`/`QFormLayout`; сетка закрыта в 0.12.5, форма — в 0.13.3
(6 фикстур + категория `form` в review).

---

## Закрытые задачи волны 0 (0.1–0.11) — ✅

| Задача | Что сделано |
|---|---|
| 0.1 Hover | `under_mouse` выставляется при движении мыши; `:hover` QSS, Fusion hover, скроллбары |
| 0.2 Tooltips | `toolTip` из свойств, таймер ~700 мс |
| 0.3 `.ui` loader `<item>` | `<item>` внутри `<widget>` парсится, текст как `item_text` |
| 0.4 Шрифты из `.ui` | `<property name="font">` → `PropValue::Str` |
| 0.5 Двойной клик | `MouseButtonDblClick` при кликах в пределах 500 мс |
| 0.6 Правая кнопка | `MouseButton::Right` → `dispatch_mouse_press/release` |
| 0.7 toggled/stateChanged | QCheckBox/QRadioButton шлют событие в Lua |
| 0.8 returnPressed | Enter в `QLineEdit` → `returnPressed` |
| 0.9 CurrentIndexChanged | навигация в `QComboBox` → событие |
| 0.10 Enter/Leave | `under_mouse_widget`, `widget_at_point`, переходы `Some(old)↔Some(new)↔None` |
| 0.11 FocusIn/FocusOut | `focused_widget_id`, события на Tab/Shift+Tab |

Эти пункты не требуют доработки; регрессии ловятся существующими тестами
`app`/`rqt`.

---

## 0.12 QGridLayout — ✅ закрыто (0.12.1–0.12.5)

Цель: `.ui` с `QGridLayout` раскладывает виджеты по ячейкам, поддерживает
`rowspan`/`colspan` и per-cell alignment; геометрия сверяется с Qt6
offscreen. Stretch не поддерживается — решение A1 в `questions.md`
(в `.ui` его нет, фиктивное свойство не добавляем).

**Итог (2026-11-25):** движок сетки — порт Qt `QGridLayoutPrivate::
setupLayoutData`/`distribute` и `qGeomCalc` (`qlayoutengine.cpp`) на
целочисленной арифметике. Строки/столбцы считаются по hint/min/max
элементов, span распределяется `distributeMultiBox`, свободное место —
по Qt-правилам (expand-колонки, иначе равные доли, иначе в зазоры),
виджет в ячейке клампится по `maximumSize` и выравнивается по
`<item alignment=>`. Геометрия 4 фикстур совпадает с Qt6 в пределах 1 px,
raster-diff — в общих допусках (mean 0.8–4.7).

### 0.12.1 Парсер: атрибуты `<item>` (S)

Qt Designer кладёт координаты на сам `<item>`:

```xml
<item row="0" column="1" rowspan="2" colspan="2">
  <widget class="QLineEdit" name="lineEdit"/>
</item>
```

- добавить в `LayoutItem::Widget` (или в обёртку `LayoutCell`) поля
  `row`, `column`, `rowspan`, `colspan` (defaults `0/0/1/1`);
- читать атрибуты `<item>` в `parse_layout`;
- `GridConfig` с массивами `row_span`/`col_span` **удалить**: в Qt span —
  свойство элемента, а не строки/столбца; `rows`/`cols` не задаются в `.ui`
  (выводятся из максимума `row+rowspan`/`column+colspan`);
- тесты парсера на формы с явными `row/column/rowspan/colspan`.

**Сделано (2026-11-25):** `LayoutCell { row, column, row_span, column_span }`
(defaults `0/0/1/1`), атрибуты читаются в `parse_layout` для widget / nested
layout / spacer; `GridConfig` и `grid_config` удалены из `form-xml` и `rqt`;
`QLayoutItem` в `rqt` теперь несёт `cell` вместо мёртвых `position`/`span`.
Тесты: `parses_item_cells_from_grid_layouts`,
`box_layout_items_default_to_single_cell`,
`invalid_cell_attributes_fall_back_to_defaults`.

**Важно:** в `.ui` нет и stretch-факторов — Qt задаёт их кодом
(`setRowStretch`). Решение по их поддержке — открытый вопрос №1
(`questions.md`).

### 0.12.2 Явный вид layout-а вместо эвристики по имени (S)

Сейчас вид определяется `spec.class.contains('H')` — хрупко и уже дало
неверную деградацию grid в hbox.

- ввести `QLayoutKind { HBox, VBox, Grid, Form }` (или enum с параметрами);
- `QBoxLayout` → `QLayout { kind, spacing, margins, items }`;
- `load_layout` выбирает вид по классу, а не по подстроке;
- поведение hbox/vbox не меняется (все 106 тестов зелёные).

**Сделано (2026-11-25):** `QLayoutKind { HBox, VBox, Grid, Form }`,
`QBoxLayout` → `QLayout { kind, spacing, margins, items }`;
`QLayoutKind::from_class()` вместо `contains('H')`; `layout_widget`
диспетчеризует в `layout_box`/`layout_grid`/`layout_engine` (Form пока
идёт как vbox). Тесты `app`/`rqt` для box-layout-ов остались зелёными.

### 0.12.3 Геометрический проход для grid (L)

`layout_box` (`widgets.rs:1437`) умеет только одномерную раскладку. Нужен
отдельный `layout_grid`:

1. ширина столбца = максимум hint по элементам без span (span-элементы
   распределяются по объединению столбцов);
2. высота строки — аналогично;
3. свободное место делится по stretch (если stretch нет — поровну, как Qt);
4. каждому элементу выдаётся rect объединённых ячеек;
5. вложенные layout-ы и спейсеры — такие же элементы ячеек;
6. `margins`/`spacing` — из layout-а (Qt default spacing 6, margins 9/11).

Подключение в `layout_widget`: выбор `layout_box`/`layout_grid` по
`QLayoutKind`.

**Сделано (2026-11-25):** `layout_grid` + `grid_tracks` + `q_geom_calc`
(порт Qt, включая fixed-point распределение), `QGridTrack` ~
`QLayoutStruct`, `distributeMultiBox`, `setupSpacings` (зазор только
между разными элементами), `qSmartMinSize`/`qSmartMaxSize`
(`grid_smart_min`/`grid_smart_max`) и классовые size policy по умолчанию
(`widget_axis_policy`). Размеры строк/столбцов и точные позиции —
целочисленные, как в Qt.

### 0.12.4 Span и alignment (M)

- `rowspan`/`colspan` — объединение ячеек с корректным учётом вклада в
  размеры строк/столбцов;
- выравнивание по умолчанию как в Qt: `Expanding`-виджеты заполняют ячейку,
  остальные — по hint с `Qt::AlignLeft|AlignVCenter` (если не задан
  `<property name="alignment">`);
- при необходимости — per-item alignment из `.ui`.

**Сделано (2026-11-25):** span через `distributeMultiBox` (и min, и
hint, и вырожденный случай `maximum_sum < minSize`); `grid_widget_rect`
повторяет `QWidgetItem::setGeometry`: кламп по `maximumSize`, при
alignment — кламп по hint, выравнивание (Qt `visualAlignment`: без
горизонтальных битов — влево, по вертикали без `AlignTop` — центр).
`LayoutCell` получил `alignment: u32`, в `form-xml` парсится
`<item alignment="Qt::AlignRight|…">`. Stretch — **не** поддерживается
(решение A1-a).

### 0.12.5 Стенд и покрытие (M)

- 3–4 фикстуры: простая сетка 2×2, span по строке/столбцу, вложенный
  layout в ячейке, сетка с разными size policy;
- `tools/qt_reference` → Qt6 JSON + PNG, наши тесты:
  `grid_*_geometry_matches_qt6_offscreen` (допуск 1 px) и raster-diff;
- формы добавить в `tools/formgen` (категория `grid`) и в review-пак.

**Сделано (2026-11-25):** фикстуры `client/crates/rqt/tests/fixtures/
grid_{2x2,spans,nested,policies}.ui` + Qt6 golden JSON/PNG; тесты
`grid_*_geometry_matches_qt6_offscreen` (1 px) и `grid_*_raster_
compared_to_qt6`; категория `grid` в `tools/formgen/review.py`
(4 формы, id `grid/76…79`), review-пак перегенерирован
(mean 0.8–4.7, >30 — 1–6 %). Браузер: `?lua=test_grid.lua&ui=
test_grid.ui` — grid, вложенный hbox, span-таблица, spacer, hit-test
Reset/Close и ввод в ячейку проверены в WSLg-Chromium (:9222).

**Definition of done 0.12:** ✅ имя→rect всех виджетов совпадает с Qt6 в
пределах 1 px; span и per-cell alignment работают; hbox/vbox и остальные
тесты не сломаны; формы есть в review. Stretch не входит (A1-a).

---

## 0.13 QFormLayout — ✅ сделано (2026-11-25)

Цель: label-field пары в две колонки с Qt-политиками роста и выравнивания.

### 0.13.1 Парсер и свойства — ✅

- `form_xml::Layout` несёт `field_growth_policy`, `row_wrap_policy`,
  `label_alignment`, `form_alignment`, `horizontal_spacing`,
  `vertical_spacing`; enum-ы `FieldGrowthPolicy`/`RowWrapPolicy`.
- Читаются `<enum>QFormLayout::…</enum>` и `<set>Qt::Align…|…</set>`
  (Fusion-дефолты: `AllNonFixedFieldsGrow`, `DontWrapRows`, labels left,
  form left/top). Поддержан Designer-синтаксис без `AlignmentFlag::`.
- Строки/колонки берутся из `<item row= column= colspan=>` (0.12.1);
  `colspan>=2` → spanning (full-row) item.

### 0.13.2 Двухколоночный движок — ✅

- `layout_form` + `form_data` — порт `QFormLayoutPrivate::updateSizes`,
  `setupVerticalLayoutData`, `setupHorizontalLayoutData` и
  `arrangeWidgets` для не-HFW случая.
- Колонка меток = максимум hint (с учётом wrap); поле получает остаток
  по `fieldGrowthPolicy` (`AllNonFixedFieldsGrow` /
  `FieldsStayAtSizeHint` / `ExpandingFieldsGrow`).
- `rowWrapPolicy` (`DontWrapRows`/`WrapLongRows`/`WrapAllRows`),
  `labelAlignment` (left/center/right), `formAlignment` (left/center/right,
  top/center/bottom), `horizontalSpacing`/`verticalSpacing`.
- Вертикальные треки считаются тем же `qGeomCalc`, что и grid; top/bottom
  stretch повторяет Qt при `formAlignment`.
- `layout_size_hint`/`layout_minimum_size`/`layout_maximum_size` знают про
  Form; вложенный form внутри group box/vbox меряется как Qt-форма.
- `QLineEdit` hint для form-движка приведён к целочисленной
  `QFontMetrics`-формуле Qt (4 px), box-движок не затронут.

### 0.13.3 Стенд и покрытие — ✅

- Фикстуры `form_basic`, `form_wrap_long`, `form_wrap_all`, `form_growth`,
  `form_alignment`, `form_group` + Qt6 JSON/PNG.
- Тесты `form_*_geometry_matches_qt6_offscreen` (допуск 1 px) и
  `form_*_raster_compared_to_qt6`; парсерные тесты
  `parses_form_layout_properties`, `form_layout_defaults_follow_fusion`.
- Категория `form` в `tools/formgen/review.py` (6 форм, id `form/80…85`);
  review перегенерирован: mean diff 1.3–3.4, >30 — 1.6–3.2 %.
- Браузер (chrome-devtools MCP, WSLg Chromium :9222):
  `?lua=test_qform.lua&ui=test_qform.ui` — правая колонка меток, длинная
  метка, checkbox, Reset/Close в spanning-row; Reset очищает text и
  снимает checked. Фикстуры: `server/priv/qt_forms/test_qform.ui`,
  `server/priv/lua_modules/test_qform.lua`.

**Definition of done 0.13:** ✅ две колонки, выравнивание и growth-политика
совпадают с Qt6; формы в review.

---

## 0.14 Application shell / главное окно — ✅ 0.14.1–0.14.7

Не входит в текущую волну 0.12/0.13 и вынесен в отдельный этап:
главное окно как `QMainWindow`-аналог (menu + toolbar + central
`QMdiArea` + statusbar), формы как MDI **или** top-level окна, application/window
modal, меню из Lua и старый запуск без главного окна.

Полный план: `docs/application-shell-plan.md`.
Решения **D1–D8** приняты (`questions.md`); этап готов к старту.

Коротко по этапам:

- **0.14.1** ✅ домен/протокол: `role`, `placement`, `modal_scope`,
  передача `mdi`, main-сессия;
- **0.14.2** ✅ main shell и режимы `full`/`standalone`, viewport под
  menu/statusbar;
- **0.14.3** ✅ `QMenuBar`/`QMenu`/`QAction`, `App:set_menu` и menu actions;
- **0.14.4** ✅ top-level слой (несколько окон, z-order, drag/resize);
- **0.14.5** ✅ application/window modal;
- **0.14.6** ✅ `QStatusBar` и технический подвал;
- **0.14.7** ✅ `QToolBar` — обязательный виджет (actions общие с меню);
- **0.14.8** ⏸ (отложено) загрузка `QMainWindow` из `.ui`;
- **0.14.9** ⚠️ интеграция/браузер проверены; golden-снимки ещё нет.

---

## 0.15 Упорядоченный display list / PaintBatch — ✅ сделано

Реализовано 2026-11-25: `gfx::PaintBatch::{Rects,Text}` вместо двух
глобальных пулов; `append_paint_commands` выдаёт батчи в порядке
`QPaintCommand`; `gfx::render` проигрывает их в одном render-pass
(rect-диапазоны + отдельный glyphon renderer на текстовый батч);
`Shell::popup_rects` и фильтрация текстов удалены.

### Проблема

`QWidgetTree` и `paint_widget` уже иерархичны: родитель рисует себя и
рекурсивно вызывает детей. Но `app::append_paint_commands` разбирает
упорядоченный поток `QPaintCommand` на **два общих массива** —
все `ColoredRect` и все `ColoredText`. `gfx::render` затем рисует
сначала все прямоугольники, потом все тексты.

Из-за этого теряется painter order:

- текст toolbar просвечивает через popup (лечится текущей маскировкой);
- текст одной формы может оказаться поверх другой формы;
- top-level окно не всегда перекрывает текст MDI;
- вложенное меню может смешиваться с родительским.

Эталон: `docs/pas/WINUNIT.PAS` — `tgroup`/`twnd.show` рисует себя, затем
рекурсивно детей; `tmenu.show` для каждого пункта сразу рисует фон и текст.
Это обычный painter's algorithm / display list, а не два глобальных пула.

### Цель

Сохранить порядок рисования до GPU и убрать маскировку текста как
временный костыль. Дерево виджетов переписывать **не нужно** — меняется
только граница `QPainter → gfx`.

### 0.15.1 Модель батчей (S/M)

**Сделано:** `PaintBatch` в `gfx`; соседние однотипные команды
объединяются, но `Rects -> Text -> Rects -> Text` сохраняет порядок.
Clip текста по-прежнему попадает в `ColoredText::clip`.

- ввести `PaintBatch { Rects(Vec<ColoredRect>), Text(Vec<ColoredText>) }`
  (или `PaintList` с командами и диапазонами);
- `append_paint_commands` должен выдавать батчи **в порядке команд**, а не
  два независимых пула;
- сохранить clip и шрифтовые параметры;
- тест: порядок `Rects -> Text -> Rects -> Text` не меняется.

### 0.15.2 Рендерер (M)

**Сделано:** `gfx::render(clear, &[PaintBatch])`; все rect-батчи пишутся
в один vertex buffer с диапазонами, текст — per-batch `TextRenderer`;
всё внутри одного pass, `LoadOp::Clear` только в начале.

- `gfx::render_batches(clear, &[PaintBatch])`:
  - первый непустой батч — `LoadOp::Clear`, остальные — `LoadOp::Load`;
  - либо последовательные render-pass'ы, либо переключение pipeline в
    одном pass;
  - текст каждого батча подготавливается/рисуется после его фона.
- Оценить стоимость: батчи объединять только соседние одинаковые типы;
  меню/окна не должны требовать отдельного pass на каждый виджет.

### 0.15.3 Порядок слоёв в app (M)

**Сделано:** `collect_paint() -> Vec<PaintBatch>`; порядок:
shell base → MDI content+chrome по z-order → top-level по z-order →
popup-меню по уровням. `Shell::popup_rects` и `texts.retain(...)`
удалены, clipping форм/меню сохранён.

- `collect_paint` возвращает батчи в порядке:
  1. shell base: фон/menu/toolbar/status;
  2. MDI-окна в z-order: content + chrome каждого окна;
  3. top-level окна в z-order;
  4. открытые popup-меню по уровням (родительское, затем вложенное).
- Удалить `Shell::popup_rects` и фильтрацию текстов под popup из
  `collect_paint`.
- Сохранить clipping форм/меню.

### 0.15.4 Тесты и browser (M)

**Сделано:** unit-тесты на `Rects -> Text -> Rects -> Text`, popup над
toolbar без маскировки, вложенное подменю, MDI→top-level; браузерные
сценарии `?main=test_main.lua` («Файл», «Вид», вложенное подменю,
toolbar) и `?lua=demo_icons.lua&ui=test_dialog01.ui` проверены через
chrome-devtools MCP на `127.0.0.1:9222`, скриншоты до/после сняты.

- unit: popup над toolbar, выбранный пункт меню с текстом, соседний
  пункт меню; overlapping MDI/top-level; вложенное подменю.
- `cargo test --workspace`, `mix test`, `trunk build --release`.
- **Обязательная браузерная проверка через chrome-devtools MCP**:
  - WSLg Chromium: `http://127.0.0.1:9222`;
  - `http://localhost:8080/?main=test_main.lua` — открыть «Файл», «Вид»,
    вложенное подменю, toolbar;
  - `http://localhost:8080/?lua=demo_icons.lua&ui=test_dialog01.ui` —
    standalone/top-level;
  - клики/наведение через настоящий браузер (CDP pointer events), не
    только unit-тесты; сохранять скриншоты до/после.
- Добавить regression-тест на порядок батчей, чтобы маскировка не
  вернулась.

### 0.15.5 Golden/визуальный diff (S/M)

**Статус:** ⚠️ браузерные PNG до/после сняты и проверены визуально;
автоматический golden-diff меню/toolbar/overlapping/nested ещё не
зафиксирован в репозитории.

- зафиксировать эталонные PNG для menu popup + toolbar, overlapping
  windows и nested menu;
- сравнение с Qt6/rqt raster, если применимо.

**Definition of done 0.15:** порядок рисования иерархичен на всех слоях;
меню/toolbar/формы/top-level не смешиваются без text-masking; браузерные
сценарии и тесты зелёные.

---

## Порядок работ

0.14 идёт отдельным треком; порядок ниже — волна 0.12/0.13.

1. ✅ 0.12.1 парсер `<item row/column/rowspan/colspan>` + тесты.
2. ✅ 0.12.2 `QLayoutKind`, без изменения поведения box-layout-ов.
3. ✅ 0.12.3 `layout_grid` (базовая сетка без span).
4. ✅ 0.12.4 span/alignment (stretch не поддерживаем — A1-a).
5. ✅ 0.12.5 фикстуры + Qt6 golden + review + браузер.
6. ⬅ 0.13.1 свойства `QFormLayout` + распознавание.
7. 0.13.2 движок двух колонок + политики.
8. 0.13.3 фикстуры + golden.
9. 0.14 — отдельный этап `docs/application-shell-plan.md`; 0.14.1–0.14.7
   ✅, 0.14.8 отложен.
10. ✅ 0.15 — `PaintBatch`/упорядоченный display list; text-masking убран,
    проверено через chrome-devtools MCP (:9222).

Каждый шаг заканчивается прогоном `cargo test --workspace`; после 0.12.5 и
0.13.3 — `scripts/review.sh` и проверка в браузере.

## Риски

- **Нет stretch в `.ui`** — не тащить фиктивную поддержку; решено (A1-a):
  stretch-факторы не поддерживаем, пока не появится источник (свойство
  `.ui` или API `rqt`).
- **Ловушка «инфраструктура готова»** — модель данных без парсинга создаёт
  ложное ощущение прогресса; поэтому 0.12.1 идёт первым.
- **Регрессии hit-test/geometry при смене `QBoxLayout` → `QLayout`** —
  делать 0.12.2 отдельным шагом с зелёными тестами.
- **Пиксельная точность grid** — сначала Qt6 golden, потом отрисовка.

## TODO на будущее (известные дефекты)

- [ ] **`QBoxLayout` не растит виджеты с политикой `Minimum`/`Preferred`.**
  В Qt `qGeomCalc` при избытке места делит его между элементами, у которых
  `maximumSize` больше hint (например, ряд `QPushButton`), а `rqt`
  отдаёт избыток в зазоры (`layout_box`, ветка `growers` не включает
  кнопки/combo). Поэтому ряд кнопок без spacer-а растягивается по-разному:
  Qt — кнопки шире, `rqt` — зазоры. Grid уже считает по Qt-алгоритму,
  box — нет. Чинить отдельным шагом (0.12.2 запрещал менять box-поведение),
  с golden-тестом на ряд кнопок.

- [ ] **Кнопки вываливаются за пределы формы при минимальном размере окна.**
  При уменьшении MDI-подокна до `minimumSize` (или до малого размера,
  который разрешает текущий fallback `80×64`) нижние кнопки — например,
  OK/Cancel в `test_dialog01.ui` — рисуются вне content-области, поверх
  рамки/за её пределами. Причина: layout не учитывает `minimumSizeHint`
  layout-а и доступную высоту; `minimumSize` формы сейчас не поднимается
  до минимума содержимого. Нужно:
  - учитывать `minimumSizeHint` корневого layout-а при вычислении
    эффективного минимума подокна (`max(minimumSize, layout min)`);
  - при нехватке места сжимать/переносить/скроллить содержимое, а не
    выпускать детей за content rect;
  - проверять на форме с нижним `QHBoxLayout` кнопок и на длинных
    подписях; добавить regression-тест geometry/hit-test.

## Не входит в волну 0 (из `QT_PORT_PLAN.md`)

- popup `QComboBox` и collapse/expand `QTreeWidget`;
- `QSplitter`;
- QSS-расширения сверх текущего подмножества.

Меню/тулбар/главное окно вынесены в отдельный этап 0.14
(`docs/application-shell-plan.md`), а не в волну 0.

