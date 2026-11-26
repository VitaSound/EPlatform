# EPlatform — план волны 0.18: размещение окон, chrome и галерея форм

Обновлено: 2026-11-26.
Статус: активный план — **волна 0.18** (см. раздел «Волна 0.18» в конце
файла). Волна 0.17 закрыта, её текст ниже сохранён как история.

## Решения пользователя (2026-11-26)

1. «Тулбокс» = **сепараторы `QToolBar`**, не `QToolBox`.
2. Каскад применяется **только к формам без заданной геометрии**. Если
   геометрия задана явно и совпадает с другой формой — это задумано,
   не трогаем.
3. Ленивые wasm-модули — **после** прогресс-бара загрузки и бюджета
   размера, отдельным POC.

---

# Волна 0.17: мобильный ввод и QSS v2 (история)

Обновлено: 2026-11-25. Волна 0.16 закрыта, предыдущий план перенесён в
архив: `docs/archive/completed-plans/widgets-plan-0.0-0.16.md`.
**0.17.1 закрыт (2026-11-25):** скрытое поле ввода, IME, touch-hover,
long tap, double tap, multi-touch и two-finger scroll проверены в
браузере. Pinch отложен до решения по семантике.
**0.17.2 закрыт с ограничениями:** box model, вертикальные
`qlineargradient` и item-view-селекторы работают; `border-image`/`url()`
не поддерживаются (нет asset pipeline).
**0.17.3 закрыт:** `Form:center`, `QSplitter.sizes`, `QTreeWidget`
`expandAll`/`collapseAll`.
**0.17.4:** review-пак пересобран; комментарии заполняет пользователь.

Связанные документы: `EPlan_Status.md`, `questions.md`,
`docs/application-shell-plan.md` (закрытый stage 0.14),
`client/QT_PORT_PLAN.md`, `docs/architecture-ru.md`.

Обозначения: ✅ работает · ⚠️ частично · ❌ нет.
Сложность: **S** <1 дня · **M** 1–3 дня · **L** 3–7 дней · **XL** недели.

---

## Baseline (проверено 2026-11-25, после волны 0.17.4)

```
cargo test --workspace   → 205 passed; 0 failed; 1 ignored
  app 57 (1 ignored) · form-xml 13 · rqt 135
mix test (server/)       → 30 passed
cargo check -p app --target wasm32-unknown-unknown → ok
```

## Правила фронта

1. **Сначала парсер/модель, потом геометрия/отрисовка** — не повторять
   ошибку «модель есть, а данные не читаются».
2. **Qt6 golden — источник истины** для геометрии, QSS и визуального
   diff-а; новые фикстуры снимать `tools/qt_reference`.
3. **Additive changes**: уже закрытые раскладки, hit-test и mouse-сценарии
   не должны менять поведение без отдельного regression-теста.
4. **UI проверять в настоящем браузере** (Chrome DevTools MCP, WSLg
   Chromium `127.0.0.1:9222`), для touch — через CDP
   `Input.dispatchTouchEvent`, а не только unit-тестами.
5. **Не добавлять фиктивные свойства**, которые не имеют источника в Qt,
   `.ui` или Lua-протоколе. Новое свойство — только с тестом и
   использованием в рендере/геометрии.

## Наследие: незакрытые кандидаты из архива

Перенесены из `widgets-plan-0.0-0.16.md`, раздел «Touch-ввод»:

- [x] мягкая клавиатура / IME / ввод текста на iOS/Android;
- [x] сброс `hover` после `TouchPhase::Ended`;
- [x] контекстное меню по долгому тапу (`contextMenuEvent`);
- [x] выделение слова двойным тапом и маркеры выделения в `QLineEdit`;
- [x] два пальца → scroll; pinch-zoom отложен (см. открытые решения);
- [x] несколько одновременных касаний не ломают основной указатель.

Эти пункты входят в 0.17.1 и не считаются сделанными до браузерной
проверки на touch-эмуляции.

---

## 0.17.1 Мобильный ввод — P0

**Цель:** на телефоне пройти полный сценарий: тап в `QLineEdit` →
поднимается мягкая клавиатура → ввод (включая русскую раскладку/IME)
доходит до `text`/`formEvent` → `Enter`/`Tab` работают как на десктопе.

### 0.17.1.1 Мост к скрытому полю ввода (L)

Сейчас `winit` слушает `keydown` на `canvas`; iOS так клавиатуру не
поднимает. Нужен отдельный input-элемент и явная передача текста в
`App`, без попытки «притвориться», что winit сам всё пришлёт.

- [ ] **`client/crates/app/index.html`**: скрытый `<textarea>` (или
      `contenteditable`), `inputmode="text"`, `autocapitalize="off"`,
      `autocomplete="off"`, `spellcheck="false"`, вынесен за viewport, но
      не `display:none` (иначе iOS не откроет клавиатуру).
- [ ] **JS-модуль** `client/crates/app/src/text_input.js` (или inline
      module): `focus`/`blur`, `beforeinput`, `input`,
      `compositionstart/update/end`, `keydown` для
      Backspace/Delete/Enter/Tab/стрелок/Home/End/Ctrl+A; отправка
      событий в Rust через `CustomEvent`/`wasm_bindgen`-экспорт.
- [ ] **Rust-приёмник**: не собирать фальшивый `winit::KeyEvent`, а
      переиспользовать существующий `KeyEdit`/`App::dispatch_key`-путь:
      вынести обработку текста в `App::apply_text_input(action)` или
      эквивалент. Так один код обслуживает десктоп и мобильный ввод.
- [ ] **Синхронизация фокуса**: при фокусе `QLineEdit` — `textarea.focus()`;
      при уходе фокуса/закрытии формы/модалки — `textarea.blur()`.
      Не воровать фокус у меню, таблиц и мобильной клавиатуры без
      `QLineEdit`.
- [ ] **Composition/IME**: коммит `compositionend` в `text`; для
      `compositionupdate` — либо нативная вставка после коммита, либо
      минимальный preedit-слой поверх `QLineEdit` (решить в 0.17.1.3).
- [ ] **Клавиши**: Backspace/Delete/Enter/Tab/стрелки/Home/End и Ctrl+A
      должны идти тем же путём, что сейчас; не допускать двойного ввода
      (`keydown` winit + `input` hidden-поля).
- [ ] **Тесты**: unit на `KeyEdit`-путь; wasm-check; CDP-сценарий
      `Input.dispatchTouchEvent` по `QLineEdit` + `Input.insertText` /
      `keydown`; отдельно проверить, что без тапа в поле клавиатура не
      появляется.

**DoD 0.17.1.1:** ✅ на CDP-эмуляции 390×844 тап в `QLineEdit` поднимает
ввод, текст и `Enter` доходят до формы; на десктопе мышь/клавиатура не
регрессируют. Проверено в браузере: «5», Backspace, Tab, Ctrl+A,
`compositionend` «тест», тап по пустому месту снимает фокус.

### 0.17.1.2 Touch-состояния и контекстное меню (M)

- [ ] После `TouchPhase::Ended/Cancelled` сбрасывать `under_mouse_widget`
      и `hover` у форм (аналог `QEvent::HoverLeave`). Не ломать hover
      для настоящей мыши.
- [ ] Долгий тап (~500 мс) → `formEvent "ContextMenu"` для виджета под
      пальцем; добавить `QWidget::contextMenuEvent`-аналог или явный
      host-путь, не подменять `Clicked`.
- [ ] Двойной тап в `QLineEdit` → выделение слова; drag за маркеры
      выделения — отдельным шагом, если 0.17.1.2 не влезает.
- [ ] Многоточие: несколько `Touch`-id — хранить map указателей; как
      минимум не терять второй палец и не отправлять его в основной
      курсор.

**DoD:** ✅ после тача виджеты не остаются «под курсором»; долгий тап
даёт `ContextMenu` и не превращается в click; двойной тап выделяет
слово; второй палец не ломает основной сценарий.

### 0.17.1.3 Pinch / два пальца (M)

- [ ] Определить продуктовую семантику: pinch → масштаб всей сцены
      (MDI/top-level) или только `QTableWidget`/`QTreeWidget`? Два
      пальца → `MouseWheel` или pinch?
- [ ] Реализовать после решения; не включать browser zoom, а работать
      в canvas-системе координат с учётом DPR.
- [ ] Тест: CDP multi-touch, сохранение координат hit-test после
      масштаба/сдвига.

**Решение (2026-11-25):** два пальца — это scroll (`MouseWheel`-путь),
pinch-zoom в 0.17 не делаем: в клиенте нет модели масштаба сцены, а
самостоятельный zoom сломает hit-test и Qt-геометрию. `Pinch` остаётся
открытым пунктом на отдельное решение.

**DoD:** ✅ two-finger scroll не конфликтует с drag/resize окон; координаты
тапа после жеста остаются точными; при втором пальце tap/click
отменяется.

---

## 0.17.2 QSS v2 — P1 — ✅ закрыто с ограничениями

Сейчас QSS умеет цвета, `background`, `border`, `border-radius`,
`padding` (частично), `font-*`, `text-align`, атрибутные селекторы и
состояния. В 0.17.2 добавлены box model, градиенты и item-view.

### 0.17.2.1 Box model: `margin`/`padding` — ✅

- [x] `margin`, `margin-top/right/bottom/left` — парсятся и применяются:
      margin сдвигает frame (фон/рамку), padding — content-rect.
- [x] Полный `padding`: текст, selection/caret, кнопки, group box;
      добавлены `padding-top/right/bottom/left`.
- [x] `52_checks.ui` с `QGroupBox { margin-top }` больше не игнорирует
      margin; старые box/grid/form-тесты не сломались.
- [x] Тесты: `parses_box_model_sides_and_linear_gradient`,
      `qss_margin_padding_and_gradient_affect_the_painted_rects`.

### 0.17.2.2 Фоны и границы — ⚠️

- [x] `background-image`/`background` c `qlineargradient(...)`: парсятся
      `x1/y1/x2/y2` и все `stop`, painter рисует по крайним стопам
      вертикально через `FillLinearGradientRect`; multi-stop/угол —
      задел на будущее.
- [x] `background: none/transparent` и наследование style sheet по
      цепочке родителей проверены.
- [x] Golden/визуально: review `qss/52_checks` 4.88 → 4.07,
      `qss/55_disabled` 11.44 → 4.41; браузерный скриншот `qss_checks`.
- [ ] `border-image`/`url(...)` — **не поддерживаем в 0.17**: в клиенте
      нет загрузки изображений для QSS. Нужен отдельный asset pipeline.

### 0.17.2.3 Item-view и subcontrol-селекторы — ✅ (базово)

- [x] `QTreeWidget::item`, `QListWidget::item`, `QTableWidget::item`:
      фон/цвет/selection для строк и ячеек.
- [x] `QHeaderView::section`: фон/цвет секций table/tree.
- [x] `QComboBox QAbstractItemView`: фон/цвет popup через `resolve_part`.
- [ ] `QSplitter::handle`, `QScrollBar::handle`, `::chunk` — не меняли:
      Fusion-рендер пока устраивает, отдельный subcontrol API не нужен.
- [x] Descendant-селекторы: поддержан частный случай
      `QComboBox QAbstractItemView`; общий `QDialog QPushButton` не
      добавляли (нет источника в review-формах).

**DoD 0.17.2:** ✅ `margin`/`padding` меняют отрисовку; popup combo и
tree/table item подхватывают QSS; 3 новых unit/paint-теста; review
пересобран. Отдельной категории `qss2` нет — использованы существующие
`qss/50–55`.

---

## 0.17.3 Мелкие закрытия Qt-поведения — P2 — ✅

### 0.17.3.1 `Form:center` / свойство `centered` — ✅

- [x] `centered=true` центрирует `QMdiSubWindow` в `QMdiArea` (и
      top-level — в viewport) с сохранением размера.
- [x] `Form:center` в Lua работает без изменений Lua-кода.
- [x] Тест `centered_form_property_centers_in_mdi_viewport`.

### 0.17.3.2 `QSplitter` API — ✅ (sizes)

- [x] `sizes` через `setProperty` и `rqt`; layout пересчитывает панели,
      resize формы сохраняет пропорции.
- [ ] `childrenCollapsible`/`collapsible` — не добавлено: текущим формам
      не нужно, min-size уже соблюдается.
- [x] Тест `splitter_sizes_property_sets_pane_widths`.

### 0.17.3.3 `QTreeWidget` свойства — ✅

- [x] `expandAll`/`collapseAll` как свойства; `expanded` в `data` уже был.
- [x] Тест `tree_expand_all_and_collapse_all_properties_rebuild_rows`;
      current сохраняется существующим `rebuild_rows`.

### 0.17.3.4 Z-order окон — ✅

- [x] Общий `window_z` для MDI и top-level: активация/клик/создание
      поднимают окно наверх, paint и hit-test идут в одном порядке.
- [x] Исправлен старый `index` после `activate()` в release-body.
- [x] Тест `activating_mdi_window_raises_it_above_floating`; браузер:
      MDI-окно, открытое под top-level, после клика выходит наверх.

### 0.17.3.5 Tab-focus indicators и caret — ✅

- [x] Qt Fusion-подобные focus-индикаторы: пунктир вокруг текста
      check/radio, кольцо radio, пунктир внутри кнопки, пунктир current
      row в list/tree; синие рамки line edit/combo сохранены.
- [x] Caret `QLineEdit` выровнен по Qt: inset 3 px, x = 3 + advance,
      y + 4, height = h - 7; mouse `cursor_at_x` использует тот же inset.
- [x] Тесты `line_edit_caret_sits_at_the_glyph_advance_like_qt`,
      `focused_checkbox_gets_a_dotted_text_focus_rect`; браузер: Tab по
      lineEdit/radio/checkbox/button, пунктир виден.

### 0.17.3.6 App sugar — не расширять без запроса

`setWindowTitle`/`setIcon` уже работают; геометрия главного окна в
браузере остаётся no-op по D5. `set_menu`/toolbar/statusbar закрыты в
0.14.

---

## 0.17.4 Review-пак и продуктовые решения — P2

- [ ] Запустить `scripts/review.sh`, открыть `review/index.html`, заполнить
      `client/review/comments.md` по категориям (C1 в `questions.md`).
- [ ] Исправить топ-расхождения вёрстки пачкой, а не по одному пикселю;
      зафиксировать метрики до/после.
- [ ] **B6/B7** (таблицы, `list_*`/БД) — не начинать без ответа
      пользователя. Это отдельный продуктовый трек, не часть 0.17.

---

## Порядок работ

1. ✅ **0.17.1.1** — hidden input + Rust-приёмник; проверено в браузере.
2. ✅ **0.17.1.2** — hover reset, long tap, double tap, multi-touch.
3. ✅ **0.17.1.3** — two-finger scroll; pinch отложен.
4. ✅ **0.17.2** — QSS box model, вертикальный градиент, item-view
   селекторы; `border-image` отложен.
5. ✅ **0.17.3** — `Form:center`, splitter sizes, tree properties.
6. ✅ **0.17.4** — review-пак пересобран; B6/B7 — только по решению.

Каждый шаг заканчивается `cargo test --workspace`, для UI — браузерной
проверкой; после QSS/раскладок — `scripts/review.sh`.

## Definition of done 0.17

- На телефоне (или CDP touch-эмуляции 390×844) можно тапнуть в
  `QLineEdit`, набрать текст и нажать Enter/Tab; русская раскладка/IME
  работают; после тача hover сброшен.
- Долгий тап даёт контекстное событие; двойной тап выделяет слово;
  второй палец не ломает основной указатель.
- QSS `margin`/`padding`, градиенты и item-view селекторы работают на
  новых фикстурах и не регрессируют старые 126 rqt-тестов.
- `Form:center`, `QSplitter` sizes/API и tree expand/collapse свойства
  доступны из Lua и покрыты тестами.
- `cargo test --workspace` зелёный; wasm-check проходит; review-пак
  пересобран, комментарии не потеряны.

## Риски

- **Двойной ввод**: `keydown` winit и `input` hidden-поля могут дать
  дубли; нужен один владелец текста.
- **IME/composition**: порядок `compositionend`/`input` на iOS и Android
  разный; проверять на реальном устройстве, а не только CDP.
- **Фокус**: скрытое поле может перехватывать фокус у меню/таблиц;
  нужен явный контракт «только `QLineEdit`».
- **QSS margin**: легко получить неверную семантику; делать с golden,
  а не «на глаз».
- **Scope creep**: `QTabWidget`/`QSpinBox`/`QSlider`/`QTextEdit` в
  текущих формах не встречаются; не тащить их в 0.17 без источника.

## Открытые решения

1. **Pinch**: масштаб сцены или только wheel? Два пальца уже сделаны как
   scroll; pinch отложен до появления модели масштаба сцены.
2. **Preedit-слой**: показывать composition-текст поверх `QLineEdit` или
   коммитить только после `compositionend`?
3. **QSS margin**: применять как полноценный CSS box model (влияет на
   layout) или ограничиться content-inset для текста? Рекомендация —
   полноценно, с golden.
4. **QSplitter API**: только `rqt`-API или ещё свойство `sizes` в Lua?
5. **B6/B7**: нужны ли `tableLayout`/`groupedHeader` и слой данных —
   ждёт решения пользователя.
6. **border-image/url()**: нужен asset pipeline для QSS-картинок; в 0.17
   не поддерживаем.

## Файлы, которые будут затронуты

- `client/crates/app/index.html`, `client/crates/app/src/*` — hidden
  input, touch-состояния, приём событий (готово в 0.17.1).
- `client/crates/app/src/lib.rs` — `KeyEdit`/text-input путь, hover reset,
  context menu, `Form:center`.
- `client/crates/rqt/src/qss.rs`, `paint.rs`, `widgets.rs` — QSS v2,
  splitter sizes, tree properties.
- `client/crates/rqt/tests/fixtures/` — QSS v2 и touch-фикстуры/golden.
- `client/review/`, `client/tools/formgen/`, `scripts/review.sh` — review
  и QSS-категория.
- `EPlan_Status.md`, `CHANGELOG.md`, `questions.md` — статусы и решения.

---

# Волна 0.18: размещение окон, chrome и галерея форм

Обновлено: 2026-11-26. Источник истины по-прежнему Qt6 offscreen
(`tools/qt_reference`), допуск геометрии 1 px. Все шаги аддитивные:
поведение закрытых волн не меняем, кроме явно перечисленных багов chrome.

Сложность: **S** <1 дня · **M** 1–3 дня · **L** 3–7 дней.

## 0.18.1 Каскад форм — P0 (M) — ✅ закрыто

Проблема: 4 одинаковых окна встают друг на друга. Причина не в MDI
(там каскад есть, `rqt/src/mdi.rs::add_subwindow`, шаг 28 px), а в том,
что каскад перекрывается явной геометрией, и в top-level слое его нет
вообще (`app/src/lib.rs:~1300`).

Правило (решение пользователя): **каскадим только формы без заданной
геометрии**; при совпадении явных координат ничего не сдвигаем.

- [x] Qt-проба `qt_reference --mdi`: `cascadeSubWindows` = шаг (10, 22)
      (= высота заголовка Qt), origin в углу области; `addSubWindow` в
      Qt — не каскад, а раскладка по 4 углам; wrap у Qt непоследователен
      (переполняет область) — поэтому наш wrap описан ниже явно.
- [x] `rqt::mdi`: слоты абсолютные (viewport не нужен после), inset
      32/28 сохранён, шаг (10, `QMDI_TITLE_H`=24 — как у Qt «шаг =
      высота заголовка»), колонка = viewport/2, wrap по сетке
      rows×cols с повтором слотов.
- [x] Floating: `.ui` (0,0) = «позиция не задана» → каскад; явная
      позиция из `.ui`/Lua всегда побеждает (совпадающие не двигаем).
- [x] Lua: `App:cascadeWindows()`, `App:tileWindows()`, `Form:cascade()`.
- [x] `tile_subwindows`: `ceil(sqrt(n))` колонок, целочисленный сплит
      ячейки, кламп по constraints.
- [x] Тесты: `cascade_slots_step_title_height_and_wrap_into_columns`,
      `cascade_and_tile_rearrange_normal_windows`,
      `forms_without_position_cascade_and_explicit_geometry_stays`,
      `cascade_and_tile_app_commands_rearrange_windows`.

**DoD:** ✅ открытие форм без геометрии сдвигает заголовки (10/24),
явная геометрия не смещается; все тесты зелёные.

## 0.18.2 Сепаратор `QToolBar` — P0 (S) — ✅ закрыто

`app/src/shell.rs::paint_base`: сепаратор заливался как
`fill_rect(6 × (TOOLBAR_H-8), border)` — сплошная серая полоса.

- [x] Qt-проба (пиксели `qt6_mainwindow.png`, action 6×31):
      две 1 px линии по центру rect (тёмная 0xd9 слева, светлая справа),
      инсет 6 px сверху/снизу.
- [x] Примитив в `shell.rs`; тест
      `toolbar_separator_paints_qt_two_line_primitive`, golden
      `shell_popup_toolbar` с сепаратором перегенерирован.
- [x] Иконки/подписи тулбара не тронуты.

**DoD:** ✅ толщина линии как в Qt; `cargo test -p app` зелёный.

## 0.18.3 Главное меню — P0 (M)

Найденные баги в `app/src/shell.rs`:

| # | Симптом | Причина |
|---|---|---|
| 1 | «Фай» вместо «Файл» | ширина пункта `advance+8`, текст в `width-12` → обрезка |
| 2 | нет подсветки/переключения по hover | `hover()` видел только попап; `top_rects` не участвовали |
| 3 | «Ви» непонятно / шапка не разложена | размеры не из Qt-метрик (они уже совпадали — тест геометрии проходил) |
| 4 | сетка в попапе | `stroke_rect` вокруг **каждого** пункта |
| 5 | разделители неверные | в шапке пропускались; в попапе инсет 4 px и цвет 140 вместо Qt 195 |
| 6 | нет иконок в верхнем уровне | не рисовались вообще |

Статус: ✅ закрыто (2026-11-26), всё сверено с пробой `qt_reference
--chrome` (меню normal/active/hover/icons + открытым popup):

- [x] Текст шапки: `rect.x+4`, вертикаль сдвинута на −2 px — попиксельно
      как Qt (`Файл` 10..47 при rect 6..52); активный пункт = рамка
      highlight×0.8 (38,112,158) + заливка (48,140,198) + белый текст.
- [x] Hover: **Qt6 Fusion не подсвечивает обычный hover** (проба
      `menubar_hover` == `menubar_normal`) — при закрытом меню ничего;
      при открытом — tracking переключает активный пункт (как Qt).
- [x] Попап: белый фон (255), одна рамка на колонку (149), выбранный
      пункт = highlight + тёмная рамка, сепаратор 1 px (195) с инсетом 5,
      текст всегда в колонке 28 (Qt резервирует её всегда: 171 без
      иконок / 179 с ними — совпадает с формулой `+51/+59`), иконки 16 px.
- [x] Иконки шапки: наше расширение «значок + надпись» (Qt при иконке в
      шапке вовсе теряет текст — probe menubar_actions: 24 px icon-only);
      без верхних иконок в фикстуре golden-геометрия Qt сохраняется.
- [x] Галочки — в иконочной колонке (заголовки выровнены).
- [x] Элизия не нужна: ширина по метрике Qt вмещает текст; кламп как в Qt.
- [x] Новые golden-тесты против Qt6 PNG:
      `menubar_normal_raster_matches_qt6_offscreen` (mean 2.34),
      `menubar_active_raster_matches_qt6_offscreen` (mean 1.79),
      `popup_without_icons_raster_matches_qt6_offscreen` (mean 1.51);
      поведенческий `menubar_hover_tracks_open_menu_like_qt`.

**DoD:** ✅ «Файл» целиком; меню переключается по hover с подсветкой;
попап без сетки; сепараторы как в Qt.

## 0.18.4 Мобильная загрузка: прогресс-бар — P0 (S)

Сейчас: один wasm **10.5 MB**, страница (`crates/app/dist/index.html`)
пустая до старта — на телефоне «белый экран».

- [x] Оверлей в `index.html` (рисуется первым же кадром HTML): полоса
      прогресса + процент + «X МБ из Y МБ» через стриминговый `fetch` +
      `ReadableStream`; скрипт — classic, выполняется до модуля trunk,
      наш fetch — основной запрос wasm; `init({ module_or_path: bytes })`.
- [x] Ошибка winit «Using exceptions for control flow…» распознаётся
      как старт (не ошибка); скрытие по `init`/появлению `canvas` +
      fade-out; при реальной ошибке — красный бар и текст (нет вечного
      спиннера). Работа проверена в Chrome (Slow 4G: «Загрузка 14% ·
      1.4 МБ из 9.5 МБ», затем форма отрендерилась).
- [x] Бюджет размера: `opt-level="z"`, `codegen-units=1`,
      `strip="debuginfo"`, `panic="abort"` (no-op на wasm32), `lto=true`.
      **До:** 10 562 859 raw / 3 525 100 gz. **После:** 9 990 688 raw
      (−5.4 %) / 3 028 782 gz (−14.1 %). `serve_dist.py` отдаёт
      `X-Uncompressed-Length`, чтобы проценты не врали на gzip.
- [ ] `wasm-opt` **не установлен** в системе → `data-wasm-opt="0"`;
      отдельный рычаг после установки бинарника.

Замечания:

- Тронковский module-скрипт тоже зовёт `init` — это безопасно
  (singleton), но даёт дубль события `TrunkApplicationStarted` (слушателей
  нет) и шум «control flow» в консоли — pre-existing.
- Белый экран при запуске **без сервера** (ws 404) воспроизводится и на
  старом коде (проверено worktree 640a0a7) — не регрессия 0.18,
  занести в `questions.md`.

**DoD:** ✅ на Slow 4G виден прогресс с первого кадра; размеры
зафиксированы выше.

## 0.18.5 Галерея форм для ручного теста — P1 (M) — ✅ закрыто

- [x] `tools/formgen/review.py`: детерминированная матрица из **126**
      кейсов в 12 категориях (`gallery/{hbox,vbox,grid,form,nested,
      policies,states,text,tables,lists,groups,mixed}`) — только
      поддерживаемые классы, без элементов внутри view (паритет с Qt
      рендерами; данные в рантайме приходят из Lua). Id стабильны:
      `gallery/<категория>/<NN>_<case>`. Флаг `--forms-only` (без
      рендеров) для быстрой итерации.
- [x] Формы пишутся и в `client/review/forms/gallery/` (ревью), и в
      `server/priv/qt_forms/gallery/` (рантайм); генерируется
      `server/priv/lua_modules/gallery_index.lua` (number/id/category/ui).
- [x] Драйвер `gallery.lua` (`?main=gallery.lua&mainui=`): меню
      «Галерея» (Следующая/Предыдущая + подменю категорий, иконка на
      верхнем пункте), «Окна» (Каскад/Плитка/Выход), тулбар ◀/▶/Каскад/
      Плитка, статусбар `#NNN id (N/126)`, автооткрытие #001;
      `gallery_form.lua` — сессионный скрипт кейса.
- [x] Заголовок формы несёт `#NNN <id>` (генерируется в windowTitle).
- [x] Побочный фикс (гонка, которую вскрыла галерея): `on_create_form`
      теперь пересчитывает viewport (`refresh_viewports`) с учётом
      текущего chrome — форма, открытая сразу после `setToolBar`, больше
      не заезжает под тулбар (работает и до инициализации GPU).
- [x] Проверено в браузере: меню/подменю/разделитель/подсветка, навигация
      ◀▶, каскад (10/24), `&mainui=` отключает static-меню mainwindow.ui.

**DoD:** ✅ 212 форм (126 галереи), открывается из меню, номер в
заголовке и статусбаре; все тесты зелёные (`cargo test --workspace`
214, `mix test` 30).

## 0.18.6 Скрины и страница сравнения — P1 (M) — ✅ закрыто

Пайплайн: `formgen` → `review/forms` → `review/qt` → `review/rust` →
`review/index.html` + `manifest.json` + `comments.md`.

- [x] Полный прогон `./scripts/review.sh` (42 с): **211 форм**, по два
      PNG (Qt6 offscreen + наш `rqt_render`), mean/max/>30 diff.
- [x] Страница: категории + быстрый переход, **фильтр по id/номеру**,
      колонки `№/id/заголовок`, Qt6, Rust, diff, **заметки** (текст из
      `comments.md` на момент сборки — сам файл не перезаписывается),
      ссылки на `.ui`.
- [x] Нумерация `#NNN` на странице = заголовкам окон в приложении
      (0.18.5).
- [x] Починены причины ложных «размер не совпал» (999):
      `rqt_render` клампит форму до minimum size hint (как Qt и при
      создании формы в приложении); QGroupBox-заголовок участвует в
      min-size (`title`, не `text`; title+30 по пробе Qt — окно 508
      совпало побитово); допуск ±1 px на сравнение с обрезкой (целочисленное
      округление текста).
- [x] Итоги: 0 форм с расхождением размеров; медиана diff по всем —
      **1.9**, по галерее — **1.66**; худшие: `lists/list_disabled`
      (13.16) и `states/disabled_table` (8.07) — визуально проверить и
      занести в `comments.md`.
- [ ] `wordWrap` у QLabel не поддерживается (кейс убран из галереи) —
      кандидат в отдельную задачу.

**DoD:** ✅ страница открывается локально, у каждого кейса два PNG и
diff, заметки сохраняются.

## 0.18.7 Ленивые wasm-паки — P2 (L), только после 0.18.4

- [ ] POC на одном пакете: `core` (старт) + `widgets-advanced`
      (таблицы/дерево/сплиттер) за стабильным C-ABI-диспетчером,
      инстанс по требованию.
- [ ] Мерить выигрыш относительно сжатого монолита; если <30 % по
      времени до интерактивности — не тащить сложность.
- [ ] Данные (Lua/`.ui`) уже on-demand с сервера — это не трогаем.

**DoD:** POC + отчёт «выигрыш/стоимость», решение зафиксировано здесь.

## Порядок работ

0.18.1 → 0.18.2 → 0.18.3 (chrome) → 0.18.4 (мобилка) →
0.18.5 → 0.18.6 (галерея+ревью) → 0.18.7 (по решению).

## Definition of done 0.18

```
cargo test --workspace                      → все зелёные (+ новые golden)
mix test (server/)                          → без регрессий
cargo check -p app --target wasm32-unknown-unknown → ok
./scripts/review.sh                         → ≥100 кейсов, Qt+Rust+diff
```

Плюс: каскад только для форм без геометрии; сепаратор тулбара 1 px;
меню без обрезки/сетки и с hover-переключением; на телефоне виден
прогресс загрузки.

## Риски

- **Каскад**: легко сломать `Form:move`/`resize` (они шлют
  `position`/`size`); явная геометрия должна оставаться приоритетной.
- **Меню**: hover-переключение конфликтует с drag/попапом; нужны тесты
  на «menu tracking» как в Qt, а не эвристики.
- **Иконки в шапке**: Qt такого не делает → включать флагом, иначе
  golden-diff уходит.
- **Размер wasm**: `opt-level=z` может замедлить layout; мерить время, а
  не только байты.
- **Генератор**: матрица легко даёт тысячи кейсов; ограничить разумным
  числом (≈100) и стабильными id.

## Файлы

- `client/crates/rqt/src/mdi.rs`, `app/src/lib.rs` — каскад, слои,
  Lua-сахар.
- `client/crates/app/src/shell.rs` — сепаратор тулбара, шапка меню,
  попап, hover.
- `client/crates/app/dist/index.html` (шаблон в `client/crates/app/`),
  `client/scripts/serve_dist.py`, `client/Cargo.toml` — прогресс,
  бюджет размера.
- `client/tools/formgen/review.py`, `client/scripts/review.sh`,
  `client/review/*` — генератор галереи и скрины.
- `server/priv/lua_modules/gallery.lua`, `server/priv/qt_forms/` —
  драйвер галереи.
- `client/tools/qt_reference/main.cpp` — новые Qt-пробы.
- `EPlan_Status.md`, `CHANGELOG.md` — статус после закрытия шагов.
