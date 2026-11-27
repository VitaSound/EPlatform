# Agent plans — Qt graphics parity (handoff pack)

Self-contained briefs for parallel agents. Each file is one component of work.
Parent overview: [qt-parity-guide.md](../qt-parity-guide.md).
Canvas DPI invariants: [wasm-canvas-geometry.md](../wasm-canvas-geometry.md).

## How to hand off

1. Pick **one** file below.
2. Paste the **Agent handoff prompt** from that file into a new agent session.
3. Do not expand **Out** of scope.
4. When acceptance passes, set `Status: done` at the top of the file.

Template: [_TEMPLATE.md](_TEMPLATE.md).

## Recommended order

```text
Parallel start:
  A: 00-remove-diag
  B: 05-render-primitive-lower
  C: 01-canvas-geometry-pointer   (after or with 00)

Then:
  02-gpu-linear-gradient-radius → 03-gpu-rounded-rect-stroke → 06-buttons-combo-qt-rounding
  10-tables-grouped-headers → 11-tables-paint-parity
    (10 model/parser can parallel 02; header paint after 02)

Later (handoff-only until requested):
  04 → 20 → 21 → 22
  30…35, 40…43
  44 (dynamic forms; B then A — see plan)
```

## Index

| File | Component | Priority |
|------|-----------|----------|
| [00-remove-diag.md](00-remove-diag.md) | Remove `?diag` HUD / `__ep*` | P0 |
| [01-canvas-geometry-pointer.md](01-canvas-geometry-pointer.md) | DOM geometry, pointer÷dpr, mode tests; doc §2/§8 | P0 |
| [02-gpu-linear-gradient-radius.md](02-gpu-linear-gradient-radius.md) | GPU linear gradient + radius | P0 |
| [03-gpu-rounded-rect-stroke.md](03-gpu-rounded-rect-stroke.md) | GPU/correct rounded stroke+fill | P0 |
| [04-gpu-texture-blit-circle.md](04-gpu-texture-blit-circle.md) | Texture blit + circle SDF | P1 |
| [05-render-primitive-lower.md](05-render-primitive-lower.md) | Shared `RenderPrimitive` lowerer | P0 |
| [06-buttons-combo-qt-rounding.md](06-buttons-combo-qt-rounding.md) | Acceptance: Fusion rounded buttons | P0 |
| [10-tables-grouped-headers.md](10-tables-grouped-headers.md) | `groupedHeader` / `tableLayout` (B6) | P0 |
| [11-tables-paint-parity.md](11-tables-paint-parity.md) | `paint_table` vs Qt gallery | P0 |
| [20-qstyle-indicators-radio.md](20-qstyle-indicators-radio.md) | Style indicators; one radio path | P1 |
| [21-qstyle-scrollbar-complex.md](21-qstyle-scrollbar-complex.md) | `CC_ScrollBar` via QStyle | P1 |
| [22-paint-modules-mdi-chrome.md](22-paint-modules-mdi-chrome.md) | Split paint; MDI chrome → rqt | P2 |
| [30-qtabwidget.md](30-qtabwidget.md) | QTabWidget / QTabBar | P2 |
| [31-qspinbox.md](31-qspinbox.md) | QSpinBox / QDoubleSpinBox | P2 |
| [32-qslider.md](32-qslider.md) | QSlider | P2 |
| [33-qprogressbar.md](33-qprogressbar.md) | QProgressBar | P2 |
| [34-qtextedit.md](34-qtextedit.md) | QTextEdit / QPlainTextEdit | P2 |
| [35-qtoolbox.md](35-qtoolbox.md) | QToolBox | P2 |
| [40-qpainter-path-pen.md](40-qpainter-path-pen.md) | Path / dash / opacity | P2 |
| [41-qss-url-border-image.md](41-qss-url-border-image.md) | QSS `url()` / border-image | P2 |
| [42-perf-dirty-scissor-letterbox.md](42-perf-dirty-scissor-letterbox.md) | Dirty paint, scissor, letterbox UX | P2 |
| [43-layout-metrics-dedup.md](43-layout-metrics-dedup.md) | Layout/style metrics dedup (re-audit first) | P2 |
| [44-dynamic-forms-without-ui.md](44-dynamic-forms-without-ui.md) | Forms without `.ui`: declarative DSL + imperative addWidget (tcs) | P0 |

## Hard rules (all agents)

- Wasm canvas size: DOM only, not `winit::inner_size()` as source of truth.
- Pointer: `logical = raw / dpr` (never `/ ui_scale`).
- `backing == surface`; mode 2 may use `backing ≈ css` (not `css×dpr`) without letterbox.
- Do not touch `client/review/comments.md` when regenerating review.
- Prefer GPU primitives over logical-pixel strip lowering for gradients/rounded/ellipses.
