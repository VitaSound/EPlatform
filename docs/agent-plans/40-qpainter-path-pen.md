# 40 — QPainter path, dash pen, opacity

Status: done
Priority: P2
Depends-on: [05-render-primitive-lower.md]
Blocks: []
Estimate: XL

## Goal

Minimum path/arc support, dashed pens, and a coherent opacity stack for controls that need them (focus rings, complex frames).

## Qt correspondence

`QPainterPath`, `QPen` dash/cap/join, `QPainter::setOpacity`.

## Scope

### In

- Path subset actually required by upcoming widgets
- Dash stroke primitive (replace `paint_dotted_rect` 1×1)
- Opacity on commands that already have partial support

### Out

- Full SVG path engine
- Arbitrary composition modes

## Implementation steps

1. List call sites that fake dashes/paths today.
2. Extend `QPaintCommand` carefully.
3. GPU or CPU raster support via RenderPrimitive.
4. Tests 32×32 contract images.

## Files to touch

| Path | Change |
|------|--------|
| gui.rs, render/, gfx.rs, raster.rs, paint focus | As needed |

## Call sites (step 1)

- `paint/focus.rs` `paint_dotted_rect` — button/check/radio focus (was 1×1 FillRect)
- `paint/scroll.rs` list/tree current-row focus via `paint_dotted_rect`
- Opacity already on `QPainter` state + draw_* alpha multiply; Save/Restore stacks it

## Done

- `QPainterPath` move/line/arc/close + `stroke_path`
- `QPaintCommand::StrokeDashedRect` → `RenderPrimitive::DashedStroke`
- GPU `ShapePrim` kind 5 (one SDF quad); CPU `expand_dashed_stroke_rect`
- Focus/list/tree use dashed stroke (no 1×1 flood)
- 32×32 raster contract + opacity save/restore test

## Tests / acceptance

- [x] Focus ring without 1×1 explosion
- [x] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/40-qpainter-path-pen.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
