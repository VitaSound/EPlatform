# 42 — Perf and wasm UX: dirty paint, scissor, letterbox, text scale

Status: done
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

- [x] Idle frames skip full rebuild when possible (`paint_dirty` + `paint_cache`; no spin `request_redraw`)
- [x] Letterbox centered (`letterbox_offset` + `resize_canvas_box` top/left; mode 3 still shrink-logical / no letterbox per plan 01)
- [x] Mode 2 text acceptably sharper or documented deferral (`?texthd=1` / `__epTextHd` experiment; default off)
- [x] `cargo test`

## Notes

- Mode 3 does **not** enable CSS letterbox (status bar stuck at `max_dim` — see `wasm-canvas-geometry.md` §7.4). Centering is implemented on the `resize_canvas_box(Some)` path for rollback/future.
- Continuous `about_to_wait` → `request_redraw` loop removed; caret/status use `ControlFlow::WaitUntil`.

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/42-perf-dirty-scissor-letterbox.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
