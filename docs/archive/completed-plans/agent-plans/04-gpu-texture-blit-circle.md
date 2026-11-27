# 04 — Texture blit and circle SDF

Status: done
Priority: P1
Depends-on: [05-render-primitive-lower.md]
Blocks: [20-qstyle-indicators-radio.md]
Estimate: L

## Goal

`DrawPixmap` is one textured quad (not 1×1 rects per pixel). Radio indicators use circle fill/stroke primitives instead of pixel grids — no banding at DPR 1.5.

## Qt correspondence

`QPainter::drawPixmap` / `drawImage`; `drawEllipse` for radio.

## Context (read first)

- `PaintBatch::Textures` + texture pipeline in `app/src/gfx.rs`
- `ShapePrim::ellipse` (kind=3) for fill/stroke
- `icons::pixmap` 16/24 icons in shell

## Scope

### In

- GPU textured rect (hash-cached uploads; atlas optional follow-up)
- Circle/ellipse fill + ring stroke primitives
- Wire DrawPixmap / ellipse commands

### Out

- Full icon theme engine
- Arbitrary path ellipses / arcs (40)

## Implementation notes (landed)

- `DrawPixmap` → `TexturedRect` (RGBA8 upload, bind-group cache by pixmap hash)
- `FillEllipse` / `StrokeEllipse` → SDF ellipse shapes
- `RenderPrimitive::{Pixmap, Ellipse}` extended in `rqt::render`

## Tests / acceptance

- [x] `cargo test -p app -p rqt`
- [ ] Icon toolbar not thousands of rects
- [ ] Radio crisp at DPR 1.5

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/04-gpu-texture-blit-circle.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
