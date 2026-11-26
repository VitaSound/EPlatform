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

## Crates

| Crate | Role |
|-------|------|
| `form-xml` | `.ui` parse |
| `rqt` | widgets, layout, style, paint commands, CPU raster |
| `app` | winit, wgpu, Phoenix, shell, event loop |

## Checks

```bash
cd client && cargo test --workspace
cargo check -p app --target wasm32-unknown-unknown
```
