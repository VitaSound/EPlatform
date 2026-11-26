# 03 — Rounded rect fill/stroke (fix StrokeRoundedRect)

Status: done
Priority: P0
Depends-on: [02-gpu-linear-gradient-radius.md]
Blocks: [06-buttons-combo-qt-rounding.md]
Estimate: M

## Goal

`StrokeRoundedRect` and `FillRoundedRect` respect `radius` on the GPU path. Today `StrokeRoundedRect` calls `push_stroke_rect` and **ignores radius** — buttons look square-framed vs Qt.

## Qt correspondence

`QPainter::drawRoundedRect` stroke and fill.

## Context (read first)

- `append_paint_commands` match arm `StrokeRoundedRect` → `push_stroke_rect` (~3988)
- `push_rounded_rect` scanline path for fill (logical rows — banding risk)
- `raster.rs` already has proper rounded border fill

## Scope

### In

- Correct stroke with radius (GPU SDF/ring or shared lowerer that matches raster)
- Fill rounded without logical-only scanline banding at fractional DPR (GPU preferred)
- LineEdit / PushButton / Combo outer+inner strokes

### Out

- Ellipse/radio (04)
- QSS border-radius edge cases beyond command radius field

## Implementation steps

1. Replace SRR → `push_stroke_rect` with radius-aware path.
2. Prefer GPU rounded after 02’s pipeline patterns; else device-step scanline as interim.
3. Align visual with `raster::fill_rounded_border` semantics for review consistency where possible.
4. Acceptance screenshots vs Qt fusion button frame.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/app/src/lib.rs` | Lowering |
| `client/crates/app/src/gfx.rs` | If GPU rounded |
| `client/crates/rqt/src/raster.rs` | Only if syncing semantics |

## Tests / acceptance

- [ ] Unit/visual: StrokeRoundedRect with radius=2 ≠ axis-aligned box
- [ ] Buttons/combo match Qt corner roundness with 02 fill
- [ ] `cargo test -p app -p rqt`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/03-gpu-rounded-rect-stroke.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
