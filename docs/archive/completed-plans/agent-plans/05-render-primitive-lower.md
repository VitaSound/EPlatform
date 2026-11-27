# 05 — RenderPrimitive shared lowerer

Status: done
Priority: P0
Depends-on: []
Blocks: [02-gpu-linear-gradient-radius.md, 04-gpu-texture-blit-circle.md]
Estimate: L

## Goal

One IR (`RenderPrimitive`) between `QPaintCommand` and both GPU (`Gfx`) and CPU (`raster`) consumers. Eliminates divergent lowering (B4-class drift).

## Qt correspondence

Qt paint engine command stream → backend; here a thin portable IR.

## Context (read first)

- Dual interpreters: `append_paint_commands` vs `raster.rs`
- Plan §1.2–1.6 / qt-parity-guide “how to add a GPU primitive”

## Scope

### In

- `rqt` module e.g. `render/{primitive,lower}.rs` (or `app` if coupling forces it — prefer `rqt`)
- `QPaintCommand[]` → `Vec<RenderPrimitive>` with `ui_scale` where needed
- GPU submit from primitives; CPU raster can keep executing commands OR primitives
- MAX_RECTS logging/assert on GPU path when solid rects overflow

### Out

- Rewriting all widgets’ paint
- QSS engine changes

## Implementation steps

1. Define `RenderPrimitive` enum (SolidRect, LinearGradient, Rounded…, Text passthrough).
2. Implement `lower_commands`.
3. Point `collect_paint` / gfx render at primitives.
4. Keep raster working for review (either lower→cpu or existing command executor).
5. Add debug_assert / log when rect count would truncate.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/render/*.rs` | New |
| `client/crates/rqt/src/lib.rs` | mod |
| `client/crates/app/src/lib.rs` | Use lowerer |
| `client/crates/app/src/gfx.rs` | Consume primitives |

## Tests / acceptance

- [ ] Existing paint order tests still pass
- [ ] Gradient/rounded paths share one definition
- [ ] Truncation is visible in logs (debug), not silent forever

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/05-render-primitive-lower.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> This unblocks 02/03/04 — keep the IR extensible.
