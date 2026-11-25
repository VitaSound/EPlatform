# EPlatform — статус проекта

Актуализировано: 2026-11-25 по коду и прогонам тестов.
Активный план: `widgets-plan.md` (волна 0.17).
Архив закрытых планов: `docs/archive/completed-plans/`.
Открытые решения: `questions.md`.

## Baseline

```
cargo test --workspace   → 199 passed; 0 failed; 1 ignored
  app 53 (1 ignored) · form-xml 13 · rqt 133
mix test (server/)       → 30 passed; 0 failed
```

## События (0.1–0.11) — ✅ закрыто

| Задача | Статус | Детали |
|---|---|---|
| 0.1 Hover | ✅ | `under_mouse`, `:hover` QSS, Fusion hover, скроллбары |
| 0.2 Tooltips | ✅ | `toolTip` из свойств, таймер ~700 мс |
| 0.3 `.ui` loader `<item>` | ✅ | `<item>` внутри `<widget>` парсится |
| 0.4 Шрифты из `.ui` | ✅ | `<property name="font">` → `PropValue::Str` |
| 0.5 Двойной клик | ✅ | `MouseButtonDblClick` (окно 500 мс) |
| 0.6 Правая кнопка | ✅ | те же события, что у левой |
| 0.7 toggled/stateChanged | ✅ | QCheckBox/QRadioButton → `formEvent` |
| 0.8 returnPressed | ✅ | Enter в `QLineEdit` |
| 0.9 CurrentIndexChanged | ✅ | навигация в `QComboBox` |
| 0.10 Enter/Leave | ✅ | `under_mouse_widget` + `widget_at_point`, Qt-переходы |
| 0.11 FocusIn/FocusOut | ✅ | `focused_widget_id`, Tab/Shift+Tab |

## Layout-ы 0.12/0.13 — ✅ закрыто

### 0.12.2–0.12.5 QGridLayout — ✅ сделано (2026-11-25)

- **0.12.2.** `QLayoutKind { HBox, VBox, Grid, Form }` и
  `QLayout { kind, spacing, margins, items }` вместо `QBoxLayout` +
  эвристики `class.contains('H')`; `layout_engine` диспетчеризует
  box/grid (Form пока идёт как vbox). Box-layout-ы не изменились.
- **0.12.3.** `layout_grid` + `grid_tracks` — порт Qt
  `QGridLayoutPrivate::setupLayoutData`/`distribute` и `qGeomCalc`
  (фиксированная точка, целочисленные размеры). Учитываются
  `qSmartMinSize`/`qSmartMaxSize`, классовые size policy по умолчанию и
  `setupSpacings` (зазор только между разными элементами).
- **0.12.4.** span через `distributeMultiBox` (min/hint/вырожденный
  `maximum_sum < minSize`); `grid_widget_rect` повторяет
  `QWidgetItem::setGeometry` (кламп по `maximumSize`, при alignment —
  кламп по hint, Qt-выравнивание). `LayoutCell` получил `alignment: u32`,
  парсится `<item alignment="Qt::Align…">`. Stretch не поддерживается
  (A1-a).
- **0.12.5.** Фикстуры `grid_2x2`, `grid_spans`, `grid_nested`,
  `grid_policies` + Qt6 JSON/PNG; тесты геометрии (1 px) и raster-diff;
  категория `grid` в `formgen`/review (4 формы, mean diff 0.8–4.7).
  Браузер (WSLg-Chromium :9222): `?lua=test_grid.lua&ui=test_grid.ui` —
  сетка, вложенный hbox, span-таблица, spacer, ввод в ячейку,
  hit-test Reset/Close.
- **`QBoxLayout`** — ✅ закрыто отдельным шагом: избыток делится между
  growers по `qGeomCalc`, golden `box_buttons.ui`. Детали — в архиве
  `docs/archive/completed-plans/widgets-plan-0.0-0.16.md`.

### 0.13.1–0.13.3 QFormLayout — ✅ сделано (2026-11-25)

- **Парсер.** `form_xml::Layout` получил `field_growth_policy`,
  `row_wrap_policy`, `label_alignment`, `form_alignment`,
  `horizontal_spacing`, `vertical_spacing`; enum-ы
  `FieldGrowthPolicy`/`RowWrapPolicy`, разбор `<set>Qt::Align…</set>`
  (и Designer-варианта без `AlignmentFlag::`).
- **Движок.** `layout_form` + `form_data` — порт
  `QFormLayoutPrivate::updateSizes`/`setupVerticalLayoutData`/
  `setupHorizontalLayoutData`/`arrangeWidgets`: колонка меток по hint,
  остаток поля по `fieldGrowthPolicy`, `rowWrapPolicy`
  (`DontWrapRows`/`WrapLongRows`/`WrapAllRows`), label/form alignment,
  h/v spacing, top/bottom stretch через общий `qGeomCalc`.
- **Вложенность.** `layout_size_hint`/`layout_minimum_size`/
  `layout_maximum_size` учитывают Form; форма внутри group box меряется
  как Qt-форма. `QLineEdit` hint в form-движке приведён к целочисленной
  `QFontMetrics`-формуле Qt.
- **Тесты.** 6 фикстур (`form_basic`, `form_wrap_long`, `form_wrap_all`,
  `form_growth`, `form_alignment`, `form_group`) + Qt6 JSON/PNG;
  геометрия 1 px и raster-diff; парсерные тесты свойств.
- **Review.** Категория `form` (6 форм, id `form/80…85`),
  mean diff 1.3–3.4, >30 — 1.6–3.2 %.
- **Браузер.** `?lua=test_qform.lua&ui=test_qform.ui` — выравнивание
  меток, длинная метка, checkbox, Reset/Close в spanning-row; Reset
  очищает text и снимает checked.

### 0.12.1 Парсер ячеек — ✅ сделано

- `form_xml::LayoutCell { row, column, row_span, column_span }` с дефолтом
  `0/0/1/1`; читается с `<item row= column= rowspan= colspan=>`.
- Ячейка хранится у widget / nested layout / spacer:
  `LayoutItem::{Widget, Layout, Spacer}` теперь несут `LayoutCell`.
- `GridConfig` (массивы `row_span`/`col_span`/stretch) удалён — он не
  соответствовал `.ui` и был мёртвой моделью.
- `rqt`: `QLayoutItem` несёт `cell`; мёртвые `position`/`span` и
  `QBoxLayout.grid_config` удалены; round-robin заглушка убрана.
- Тесты: `parses_item_cells_from_grid_layouts`,
  `box_layout_items_default_to_single_cell`,
  `invalid_cell_attributes_fall_back_to_defaults`.

### Что было до 0.12.1 (историческая проверка)

| Заявлено ранее | Было фактически |
|---|---|
| `GridConfig` парсится из XML | `grid_config = None` жёстко |
| grid-позиции считаются round-robin | ветка недостижима |
| `position`/`span` в `QLayoutItem::Widget` | поля никем не читались |
| `formLayout`/`labelAlignment`/`fieldGrowthPolicy` в `parse_value` | отсутствуют |
| QGridLayout деградирует в vbox | в hbox |
| QFormLayout деградирует в vbox | было да; закрыто в 0.13.2 |

### Волна 0.16: combo popup, tree expand, splitter, QSS — ✅

Ранее вынесенные за волну 0 пункты закрыты:

| Пункт | Что сделано |
|---|---|
| popup `QComboBox` | transient popup state, геометрия под/над combo, hit-test, выбор мышью и клавиатурой, `CurrentIndexChanged`; отдельный слой paint после рамок окон |
| collapse/expand `QTreeWidget` | `expanded` у item (default `false`, как Qt), flatten видимых строк, branch-треугольники, click по индикатору, Left/Right в приложении |
| `QSplitter` | horizontal/vertical, `handleWidth` 4, пропорции от hint, min панелей, drag ручки, Qt6 golden `splitter_h`/`splitter_v` (1 px) |
| QSS сверх подмножества | named/`rgb()`/`rgba()`/`#ARGB`/`#AARRGGBB` цвета, `[prop="value"]`, `:pressed`/`:default`/`:open`/`:!state`, per-side `border-*`, `border-style`, `font`, `font-family`, `font-style`, `text-align` |

Тесты: combo popup, tree collapse/current/keyboard, splitter Qt6
geometry + drag/min, QSS parser/selector. Итог: `cargo test --workspace`
— 189 passed, 1 ignored; wasm-check проходит.

### Волна 0.17.1–0.17.4: мобильный ввод, QSS v2, API, review — ✅

| Пункт | Что сделано |
|---|---|
| мобильный ввод | off-screen `<textarea>`, IME/composition, Backspace/Tab/Ctrl+A/Enter, hidden-фокус синхронизирован с `QLineEdit`; `keydown` в capture-фазе блокирует winit (нет двойной вставки) |
| touch-сценарии | hover reset после touch, long tap → `ContextMenu`, double tap → слово в `QLineEdit`, multi-touch и two-finger scroll, pinch отложен |
| QSS box model | `margin`/`padding` + per-side, frame/content rect, группа `QGroupBox { margin-top }` |
| QSS градиенты | `qlineargradient(...)` (вертикально по крайним стопам), `background-image`; `border-image`/`url()` отложены |
| QSS item-view | `QTreeWidget::item`/`QListWidget::item`/`QTableWidget::item`, `QHeaderView::section`, `QComboBox QAbstractItemView` |
| API | `Form:center` (`centered`), `QSplitter.sizes`, `QTreeWidget.expandAll/collapseAll` |
| review | `scripts/review.sh`: `qss/52_checks` 4.88 → 4.07, `qss/55_disabled` 11.44 → 4.41; браузерный QSS-скриншот |

Тесты: `cargo test --workspace` — 199 passed, 1 ignored; wasm-check и
`cargo fmt --check` зелёные. Браузер: touch-тап, IME «тест», Backspace,
Tab, Ctrl+A, double tap, long tap, QSS-форма.

## Осталось

1. **Review-комментарии:** `client/review/comments.md` заполняет
   пользователь; после этого — пачка правок вёрстки.
2. **B6/B7:** `tableLayout`/`groupedHeader` и слой данных — ждут решения
   пользователя.
3. **`border-image`/QSS `url(...)`:** нужен asset pipeline для картинок.
4. **Pinch-zoom:** нужна модель масштаба сцены; два пальца уже scroll.
5. **Новые виджеты** (`QTabWidget`, `QSpinBox`, `QSlider`,
   `QProgressBar`, `QTextEdit`) — только когда появятся в формах.

### Попутно исправлено

- **Чёрный экран до ресайза**: winit-web присылает `Resized` до готовности
  `Gfx`, а `Window::inner_size()` остаётся `1x1`; surfase настраивался на
  1x1. Теперь последний размер из `Resized` сохраняется в `last_size` и
  применяется (`gfx.resize`) сразу после создания `Gfx`, затем
  `request_redraw()`. Проверено: `canvas` = `1280x657` без ресайза окна.

### 0.15.5 Golden/визуальный diff — ✅

- `rasterize_batches` в app-тестах проигрывает `gfx::PaintBatch` через
  `rqt::raster` (rects + clipped text).
- Эталоны: `client/crates/app/tests/fixtures/shell_popup_toolbar.png`,
  `shell_nested_menu.png`, `shell_overlapping_windows.png`; тесты
  `shell_*_raster_matches_golden` сравнивают реальный `collect_paint()`.
- Ignored `regenerate_shell_goldens` — команда перегенерации эталонов.
- Покрывает возврат text-masking, порядок popup→nested и
  MDI→top-level наложение.

## Application shell 0.14 — ✅ 0.14.1–0.14.9

Главное окно как `QMainWindow`-аналог: menu + toolbar + central
`QMdiArea` + statusbar; формы — MDI или top-level; application/window modal;
запуск full (`?main=...`) и standalone (`?lua=...`, без главного окна).

- Полный план: `docs/application-shell-plan.md` (§12 — статус реализации).
- 0.14.1–0.14.9 реализованы; `QMainWindow.ui` грузит static
  menu/toolbar/status/title, Lua `set_menu`/`setToolBar` их заменяют.
- 0.14.9: Qt6 golden меню/toolbar/status/central (1 px) и браузерные
  сценарии full/standalone/top-level siblings/modal проверены.
- `mdi` и `modal` разведены; `createForm` несёт `role`/`mode`/
  `placement`/`modal_scope`; main-сессия владеет top-level формами.
- Этап не входит в 0.12/0.13 и не меняет их порядок.

## Paint order 0.15 — ✅ закрыто

Упорядоченный display list и golden shell: `gfx::PaintBatch::{Rects,Text}`
вместо двух
глобальных пулов; `append_paint_commands` сохраняет порядок команд, а
`gfx::render` проигрывает батчи в одном pass (rect-диапазоны и отдельный
glyphon renderer на текстовый батч). Порядок слоёв: shell → MDI →
top-level → popup/вложенные меню. `Shell::popup_rects` и маскировка
текста под popup удалены.

Тесты: `paint_command_stream_keeps_rect_text_rect_text_order`,
`popup_background_is_batched_after_base_text_without_masking`,
`nested_popup_batches_paint_parent_before_child`,
`mdi_content_is_painted_before_floating_top_level_windows`.
Браузерная проверка через chrome-devtools MCP (`:9222`):
`?main=test_main.lua` (Файл/Вид/вложенное подменю/toolbar) и
`?lua=demo_icons.lua&ui=test_dialog01.ui`; скриншоты до/после сняты.
Автоматический golden-дифф меню/toolbar пока не зафиксирован.

---

## Ключевые принципы

1. **Additive changes**: box-layout-ы (hbox/vbox) не должны менять
   поведение; смена `QBoxLayout` → `QLayout` — отдельный шаг с зелёными
   тестами.
2. **Сначала парсер, потом геометрия**: модель без парсинга уже один раз
   создала ложный статус «готово».
3. **Qt6 golden — источник истины** для геометрии и PNG-диффа.
4. **Не добавлять фиктивные свойства**: stretch в `.ui` не пишется, поэтому
   поддержка только после решения (вопрос A1).

## Файлы, которые будут затронуты

- `client/crates/form-xml/src/lib.rs` — `<item>`-атрибуты, `LayoutCell`,
  чистка `GridConfig`.
- `client/crates/rqt/src/widgets.rs` — `QLayoutKind`, `layout_grid`
  (готово), `layout_form` (0.13).
- `client/crates/rqt/tests/fixtures/` — grid/form golden.
- `client/tools/qt_reference`, `client/tools/formgen`, `client/review/` —
  категории `grid`/`form`.
