# 03 — Rounded rect fill/stroke (fix StrokeRoundedRect)

Status: done
Priority: P0
Depends-on: [02-gpu-linear-gradient-radius.md]
Blocks: [06-buttons-combo-qt-rounding.md]
Estimate: M

## Goal

`StrokeRoundedRect` and `FillRoundedRect` respect `radius` on the GPU path via one SDF quad each (no scanline / O(height) strip flood).

## Qt correspondence

`QPainter::drawRoundedRect` stroke and fill.

## Context (read first)

- `append_paint_commands` → `ShapePrim::solid` / `ShapePrim::stroke_rounded`
- Shared SDF with plan 02 (`sd_rounded_box`, stroke = outer−inner cover)
- `raster.rs` / `expand_stroke_rounded_border` remain for CPU review

## Scope

### In

- Correct stroke with radius (GPU SDF ring)
- Fill rounded without logical-only scanline banding at fractional DPR
- LineEdit / PushButton / Combo outer+inner strokes

### Out

- Ellipse/radio (04)
- QSS border-radius edge cases beyond command radius field

## Implementation notes (landed)

- `FillRoundedRect` with `radius > 0` → `PaintBatch::Shapes`
- `StrokeRoundedRect` with `radius > 0` → SDF stroke; `radius == 0` keeps 4-edge solid rects
- Scanline `push_rounded_rect` removed from the GPU path

## Tests / acceptance

- [x] `cargo test -p app -p rqt`
- [ ] Unit/visual: StrokeRoundedRect with radius=2 ≠ axis-aligned box
- [ ] Buttons/combo match Qt corner roundness with 02 fill

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/03-gpu-rounded-rect-stroke.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
