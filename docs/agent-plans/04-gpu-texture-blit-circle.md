# 04 — Texture blit and circle SDF

Status: ready
Priority: P1
Depends-on: [05-render-primitive-lower.md]
Blocks: [20-qstyle-indicators-radio.md]
Estimate: L

## Goal

`DrawPixmap` is one textured quad (not 1×1 rects per pixel). Radio indicators use circle fill/stroke primitives instead of pixel grids — no banding at DPR 1.5.

## Qt correspondence

`QPainter::drawPixmap` / `drawImage`; `drawEllipse` for radio.

## Context (read first)

- `DrawPixmap` explosion in `append_paint_commands`
- `FillEllipse` / `StrokeEllipse` logical 1×1 loops
- `icons::pixmap` 16/24 icons in shell

## Scope

### In

- GPU textured rect (atlas optional follow-up)
- Circle fill + ring stroke primitives
- Wire DrawPixmap / ellipse commands

### Out

- Full icon theme engine
- Arbitrary path ellipses / arcs (40)

## Implementation steps

1. Add texture upload path for small QPixmaps or shared atlas.
2. Add circle SDF (or tessellation) draw.
3. Remove 1×1 pixmap/ellipse lowering on prod path.
4. Verify radio + menu/toolbar icons.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/app/src/gfx.rs` | Textures + circle |
| `client/crates/app/src/lib.rs` | Lowering |
| `client/crates/rqt/src/icons.rs` | Only if atlas needs format |

## Tests / acceptance

- [ ] Icon toolbar not thousands of rects
- [ ] Radio crisp at DPR 1.5
- [ ] `cargo test -p app -p rqt`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/04-gpu-texture-blit-circle.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
