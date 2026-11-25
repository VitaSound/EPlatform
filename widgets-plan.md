# EPlatform — план волны 0.17: мобильный ввод и QSS v2

Обновлено: 2026-11-25.
Статус: активный план. Волна 0.16 закрыта, предыдущий план перенесён в
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
