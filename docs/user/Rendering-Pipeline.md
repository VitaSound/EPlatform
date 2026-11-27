# Rendering Pipeline

Как картинка попадает на экран: от дерева виджетов до GPU (и до CPU-golden в тестах).

## Обзор

```
.ui XML
  → form-xml parse
  → QWidgetTree (rqt)
  → relayout (box / grid / form, size policy)
  → style + QSS → метрики / цвета
  → paint_* → Vec<QPaintCommand>     ← «логический QPainter»
  → lower → Vec<RenderPrimitive>     ← GPU-friendly
  → app/gfx (wgpu + glyphon)         ← кадр в браузере

параллельно для тестов:
  QPaintCommand[] → rqt/raster → PNG  ↔  Qt6 offscreen PNG
```

## Слои paint

| Слой | Qt-аналог | EPlatform |
|------|-----------|-----------|
| Виджеты / layout | Widgets + QLayout | `QWidgetTree`, `relayout` |
| Стиль | QStyle Fusion | `QFusionStyle`, `pixel_metric` |
| Стилиheet | QSS | модуль `qss` (подмножество) |
| Рисование | QPainter | `QPaintCommand` в `gui.rs` |
| Движок | paint engine / RHI | `lower` + `Gfx` / `raster` |

Порядок слоёв кадра (упрощённо): **shell → MDI → top-level окна → popup/меню**.

## `QPaintCommand` (логический display list)

Команды близки к примитивам painter’а (не полный QPainterPath API):

| Команда | Назначение |
|---------|------------|
| `Save` / `Restore` | стек состояния |
| `Translate` | сдвиг |
| `SetClip` | клип |
| `FillRect` / `StrokeRect` | прямоугольник |
| `StrokeDashedRect` | пунктир (focus ring и др.) |
| `FillRoundedRect` / `StrokeRoundedRect` | скругление Fusion |
| `FillLinearGradientRect` | линейный градиент |
| `DrawLine` | линия |
| `FillEllipse` / `StrokeEllipse` | эллипс |
| `FillTriangle` | треугольник (стрелки и т.п.) |
| `DrawPixmap` | растровая иконка/картинка |
| `DrawText` | текст (далее glyphon / cosmic-text) |

Исходник: `client/crates/rqt/src/gui.rs`.

Путь (`MoveTo` / `LineTo` / `ArcTo` / `Close`) есть в модели path; полный произвольный path+dash+opacity — зона роста (см. agent-plan 40 в репо).

## Lower → `RenderPrimitive`

`rqt/src/render/lower.rs` переводит display list в примитивы, удобные GPU:

- `SolidRect`
- `LinearGradient`
- `RoundedStroke`
- `DashedStroke`
- `Ellipse`
- `Triangle`
- `Pixmap`

Текст обычно идёт отдельным путём (glyphon), а не как один из этих шейдерных примитивов.

## Два бэкенда

| Бэкенд | Где | Зачем |
|--------|-----|--------|
| **wgpu** (`app/gfx`) | браузер / native shell | продакшен-кадр |
| **CPU raster** (`rqt/raster`) | unit/golden тесты | сравнение с Qt6 PNG |

Правило продукта: **Qt6 golden — источник истины** для геометрии и визуального diff (review-pack).

## Canvas / DPI (wasm)

Размер поверхности берётся из **DOM** (`getBoundingClientRect` + `devicePixelRatio`), не из `winit::inner_size()` как SoT. Указатель: `logical = raw / dpr`.

Инженерная спека: [`docs/wasm-canvas-geometry.md`](https://github.com/VitaSound/EPlatform/blob/main/docs/wasm-canvas-geometry.md).

## QSS (кратко)

Поддерживается подмножество: цвета, состояния (`:hover`, `:pressed`, …), box model margin/padding, item-view селекторы, вертикальный `qlineargradient`, локальные `url(...)` / `border-image` (stretch 9-slice через `rqt::assets`).

Нет: remote `http(s)` URL, полный CSS repeat/round/space для border-image.

Актуальный enum-справочник: [[Paint Commands]] (генерируется из кода).

## Что автор формы должен знать

- Вы **не** вызываете paint API из Lua. Меняете свойства/модель → клиент сам перерисует.
- Кастомный «нарисовать что угодно» как в полном QPainter **не экспонирован**.
- Пиксель-перфект к Qt — цель для поддерживаемых виджетов в галерее, не гарантия для любого `.ui` из интернета.
