# Client (wasm / rqt) — agent notes

## Docs

- [docs/qt-parity-guide.md](../docs/qt-parity-guide.md) — Qt parity waves + how-to
- [docs/agent-plans/](../docs/agent-plans/) — one MD per component (hand off to agents)
- [docs/wasm-canvas-geometry.md](../docs/wasm-canvas-geometry.md) — DPR / surface / pointer

## Canvas invariants (wasm)

1. Size from **DOM** (`getBoundingClientRect` + `devicePixelRatio`), not `winit::inner_size()` as SoT.
2. Pointer: `logical = raw / dpr` — never `/ ui_scale`. Mode 3 also multiplies by `layout_from_css`.
3. `backing == wgpu surface`. Mode 2/3 fill the window (no CSS letterbox). Mode 3 keeps `scale=1` and shrinks logical (`layout_from_css`); never `ui_scale < 1` (breaks glyphon).
4. Prefer GPU / device-step primitives over logical-pixel strip lowering.
5. **Text draw width:** never pass `QFontMetrics::horizontal_advance(text)` as the `draw_text` rect width. Align with metrics, draw through the content/box right edge (`draw_text_line`). Metric-tight widths clip glyphon (`"OK"`→`"O"`, `"value"`→`"valu"`).
6. Glyphon: on `PrepareError::AtlasFull` trim+retry; `atlas.trim()` after each frame.
7. Never edit `client/review/comments.md` when regenerating review PNGs.
8. **Gallery diff gate:** after Qt vs rust PNG, **read mean / max / %** from the
   overlap diff (and from review `index.html`). If **>2%** pixels differ (or mean
   is large / widget bbox / height / width disagree), open both images, find the
   cause (often `sizePolicy` Fixed vs Preferred, wrong `sizeHint`, inset/border),
   fix, re-render, refresh index metrics, and re-diff. Do not mark paint parity
   done while height is obviously wrong or while index still shows stale %.
   **Live check:** verify in Chrome DevTools on **`10.0.0.5:9223`** (not 9222)
   after hard-reload — confirm on-page % and the real painted widget, not only
   local PNGs. If CDP is down, tell the user to launch Chrome with
   `--remote-debugging-port=9222 --user-data-dir=%TEMP%\chrome-cdp-eplatform`
   (plus `--no-first-run --no-default-browser-check`) and run
   `client/tools/cdp_relay.ps1` (`0.0.0.0:9223` → `127.0.0.1:9222`). Full
   command is in `.cursor/rules/qt-gallery-diff.mdc` § Chrome launch.

## Crates

| Crate | Role |
|-------|------|
| `form-xml` | `.ui` parse |
| `rqt` | widgets, layout, style, paint commands, CPU raster |
| `app` | winit, wgpu, Phoenix, shell, event loop |

## Layout / style metrics (prefer Qt shape)

Do **not** invent one mega `LAYOUT_METRICS` module. Mirror Qt’s split:

| Kind | Where | Pattern |
|------|--------|---------|
| Item-view geometry (row/header/col, indent, branch) | Domain module next to the widget (`item_model`, `item_view`, …) | `pub const QTABLE_ROW_H`, `QVIEW_*` |
| Chrome (scrollbar extent, tab bar height, button margin, frame widths) | `QStyle::pixel_metric(QPixelMetric::…)` in `style.rs` | Callers ask the style; do not duplicate the literal next to layout/paint |
| Layout defaults (root/child margins, box spacing) when `.ui` omits them | Named consts near layout load / `QLayout`, or style metrics if shared | Prefer one named source over `6.0` / `9.0` / `11.0` scattered |
| One-off paint parity tweaks (`cell.width - 1.0`, caret nudge) | Inline OK | Name only if reused in ≥2 places |

**When adding or changing a size:**

1. If the value already exists as a domain `const` or `QPixelMetric` — **reuse it**; do not re-hardcode the number.
2. If it is chrome shared across widgets — extend `QPixelMetric` + `QFusionStyle::pixel_metric`, then call that.
3. If it is view-specific — add/keep a `pub const` in that domain file (table ≠ list unless intentionally shared).
4. `.ui` / Designer overrides win when present; Rust defaults only fill gaps.
5. Comments like “matches Fusion probe (~28)” next to a raw `28.0` are a smell: route through style/const instead.

Cleanup backlog: [docs/agent-plans/43-layout-metrics-dedup.md](../docs/agent-plans/43-layout-metrics-dedup.md) (re-audit required before implementing).

## Checks

```bash
cd client && cargo test --workspace
cargo check -p app --target wasm32-unknown-unknown
```
