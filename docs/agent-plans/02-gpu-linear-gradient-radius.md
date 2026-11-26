# 02 — GPU linear gradient rect with radius

Status: done
Priority: P0
Depends-on: [05-render-primitive-lower.md]
Blocks: [03-gpu-rounded-rect-stroke.md, 06-buttons-combo-qt-rounding.md, 11-tables-paint-parity.md]
Estimate: L

## Goal

`FillLinearGradientRect` honors `radius` and `horizontal` without logical-pixel banding (device-step bands + rounded clip via `rqt::render::expand_linear_gradient`).  
**Follow-up (not done):** true one-quad WGSL `LinearGradient` in `PaintBatch` / `Gfx` for fewer rects.

## Qt correspondence

`QLinearGradient` + `QPainter::fillRect` / Fusion button gradients; rounded clip via radius on the brush fill.

## Context (read first)

- B4 in wasm-canvas-geometry.md
- `QPaintCommand::FillLinearGradientRect` in `rqt/src/gui.rs`
- Strip loop in `app/src/lib.rs` `append_paint_commands` (~3833)
- `gfx.rs` currently solid-color rect pipeline only
- Style PushButton/Combo already set `radius: 2.0` but lowerer drops it

## Scope

### In

- Extend paint batch / render primitive with linear gradient fields
- WGSL: interpolate `t` along axis; `mix(start,end,t)`; optional rounded-corner mask
- Map `FillLinearGradientRect` → one primitive (no per-row strips)
- Keep `raster.rs` CPU path for review (pixel-correct); do not require strips there
- MAX_RECTS / batch limits: document impact (gradients no longer consume N rects)

### Out

- Full SDF stroke for rounded outline (03)
- Texture blit (04)
- Changing Fusion color formulas

## Implementation steps

1. Prefer aligning with 05’s `RenderPrimitive` if present; else extend `PaintBatch`.
2. Add gradient pipeline or extended vertex format in `gfx.rs`.
3. Stop strip loop for FLG in `append_paint_commands`.
4. Verify consumers: style buttons, shell toolbar, MDI chrome, scroll bar, table headers.
5. Smoke: no banding on title/toolbar at DPR 1.5; radius clips corners on fill.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/app/src/gfx.rs` | Pipeline + render |
| `client/crates/app/src/lib.rs` | Lowering |
| `client/crates/rqt/src/gui.rs` | Only if command fields need tweak |
| Possibly `rqt` render module from 05 | Shared IR |

## Tests / acceptance

- [ ] `cargo test -p app -p rqt`
- [ ] Visual: PushButton corners not square on fill; no stripe pairs in gradient
- [ ] review median not worse beyond noise (headers still match raster)

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/02-gpu-linear-gradient-radius.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Coordinate with `05-render-primitive-lower.md` if both in flight. Follow qt-parity-guide + wasm-canvas-geometry.
