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
   `crates/qt/tests/fixtures/`, тест `dialog01_geometry_matches_qt6_offscreen`
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
   - Для каждой формы рендерить два PNG: наш `qt::raster` и Qt6 offscreen
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
   - TODO следующий цикл: `QScrollBar` для `QTableWidget`/`QListWidget`/`QTreeWidget`:
     вертикальный и горизонтальный, Qt Fusion track/thumb/arrows, wheel + drag,
     clipping viewport, horizontal scroll колонок, hit-test/resize с учётом
     scrollbar extent, синхронизация в app/WebSocket-клиенте.

## Критерии готовности

- Для фикстур `test_dialog01.ui`/`test_dialog02.ui` совпадение геометрии с Qt
  в пределах 1 px, визуальный diff ниже согласованного порога.
- Одинаковые события: кнопки шлют `Clicked`, остальные — `MouseButtonRelease` и
  `Close`, как `tcs_qtgui/invoke_mainwindow.cpp`.
- Все текущие серверные тесты и клиентские тесты зелёные.
- Ревью-пак `review/` содержит >=50 сгенерированных форм, side-by-side PNG,
  diff-метрики и `comments.md`; повторный прогон сохраняет комментарии.
