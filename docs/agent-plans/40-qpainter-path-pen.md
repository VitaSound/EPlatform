# 40 — QPainter path, dash pen, opacity

Status: ready
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

## Tests / acceptance

- [ ] Focus ring without 1×1 explosion
- [ ] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/40-qpainter-path-pen.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
