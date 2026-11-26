# 42 — Perf and wasm UX: dirty paint, scissor, letterbox, text scale

Status: ready
Priority: P2
Depends-on: [01-canvas-geometry-pointer.md]
Blocks: []
Estimate: L

## Goal

Fewer full-tree repaints; GPU scissor for clips; centered letterbox; sharper text in render mode 2 (1× CSS).

## Qt correspondence

Qt dirty regions / backing store updates; high-DPI glyph raster.

## Scope

### In

- Dirty/invalidation flags for collect_paint
- GPU scissor from SetClip where feasible
- Letterbox centering in mode 3
- glyphon metrics at surface scale in mode 2 (optional sub-task)

### Out

- Rewriting layout
- Pinch-zoom

## Implementation steps

1. Measure collect_paint cost; add dirty bits.
2. Scissor in gfx render pass.
3. Center letterbox via canvas CSS top/left.
4. Text atlas scale experiment behind clear flag.

## Files to touch

| Path | Change |
|------|--------|
| `app/src/lib.rs`, `gfx.rs`, shell/index styles | As needed |

## Tests / acceptance

- [ ] Idle frames skip full rebuild when possible
- [ ] Letterbox centered
- [ ] Mode 2 text acceptably sharper or documented deferral
- [ ] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/42-perf-dirty-scissor-letterbox.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
