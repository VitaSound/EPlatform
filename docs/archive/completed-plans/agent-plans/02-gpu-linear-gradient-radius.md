# 02 — GPU linear gradient rect with radius

Status: done
Priority: P0
Depends-on: [05-render-primitive-lower.md]
Blocks: [03-gpu-rounded-rect-stroke.md, 06-buttons-combo-qt-rounding.md, 11-tables-paint-parity.md]
Estimate: L

## Goal

`FillLinearGradientRect` honors `radius` and `horizontal` without logical-pixel banding.
One-quad WGSL `LinearGradient` in `PaintBatch::Shapes` / `Gfx` (no per-row strips).

## Qt correspondence

`QLinearGradient` + `QPainter::fillRect` / Fusion button gradients; rounded clip via radius on the brush fill.

## Context (read first)

- B4 in wasm-canvas-geometry.md
- `QPaintCommand::FillLinearGradientRect` in `rqt/src/gui.rs`
- `gfx::ShapePrim` + SDF shape pipeline in `app/src/gfx.rs`
- Style PushButton/Combo already set `radius: 2.0`

## Scope

### In

- Extend paint batch / render primitive with linear gradient fields
- WGSL: interpolate `t` along axis; `mix(start,end,t)`; optional rounded-corner mask
- Map `FillLinearGradientRect` → one primitive (no per-row strips)
- Keep `raster.rs` CPU path for review (pixel-correct); do not require strips there
- MAX_RECTS / batch limits: gradients no longer consume N rects

### Out

- Full SDF stroke for rounded outline (03)
- Texture blit (04)
- Changing Fusion color formulas

## Implementation notes (landed)

- `PaintBatch::Shapes` + SDF shader (`kind` 1/2 = H/V gradient, `radius` clips corners)
- `append_paint_commands` emits one `ShapePrim::gradient` per FLG command
- CPU `expand_linear_gradient` kept for raster / review / unit tests

## Tests / acceptance

- [x] `cargo test -p app -p rqt`
- [ ] Visual: PushButton corners not square on fill; no stripe pairs in gradient
- [ ] review median not worse beyond noise (headers still match raster)

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/02-gpu-linear-gradient-radius.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Coordinate with `05-render-primitive-lower.md` if both in flight. Follow qt-parity-guide + wasm-canvas-geometry.
