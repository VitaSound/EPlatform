# 01 — Canvas geometry, pointer scale, invariant docs

Status: done
Priority: P0
Depends-on: [00-remove-diag.md]
Blocks: [06-buttons-combo-qt-rounding.md, 10-tables-grouped-headers.md]
Estimate: M

## Goal

Wasm canvas geometry and pointer mapping are isolated, unit-tested, and match the three render modes. Mouse hit-testing stays correct at DPR ≥ 1.5 across resize. Document §2/§8 match reality (mode 2 allows `backing ≈ css`).

## Qt correspondence

Qt logical DPI / `devicePixelRatio` mapping to widget coordinates. EPlatform uses CSS px as logical layout units.

## Context (read first)

- [wasm-canvas-geometry.md](../wasm-canvas-geometry.md) B1–B3, §3.4 modes, §14 critique in parent plan
- `App::sync_canvas_geometry`, `pointer_scale`, `dom_canvas_size` in `client/crates/app/src/lib.rs`
- `Gfx::apply_geometry`, `ui_scale_for` in `gfx.rs`

### Hard prohibitions

- Do not use `winit::inner_size()` as source of truth on wasm
- Do not divide pointer by `ui_scale` — only `dpr`
- Do not derive logical viewport as `clamped_surface / dpr`
- Mode 2: `backing = css` without letterbox is intentional

## Scope

### In

- Extract `canvas_geometry` module (pure functions: `choose_render_mode`, `pointer_to_logical`)
- Optional newtypes or clear conversion fns (`CssPx` / `DevicePx` / `LogicalPx` — at least documented APIs)
- Table-driven unit tests for DPR × size × max_dim matrix (doc §6)
- Fix `wasm-canvas-geometry.md` §2 table, §5 (if not done in 00), §8 rewritten
- Create/update `client/AGENTS.md` with invariants + link to this pack

### Out

- GPU shaders (02–04)
- Letterbox centering UX (42)
- Changing widget layout math

## Implementation steps

1. Add `client/crates/app/src/canvas_geometry.rs` with pure mode selection + pointer convert.
2. Wire `sync_canvas_geometry` / `pointer_moved` through it.
3. Add unit tests covering modes 1/2/3 and pointer÷dpr vs ui_scale trap.
4. Rewrite doc invariants; add `client/AGENTS.md`.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/app/src/canvas_geometry.rs` | New |
| `client/crates/app/src/lib.rs` | Use module |
| `client/crates/app/src/gfx.rs` | Only if apply_geometry API needs clarity |
| `docs/wasm-canvas-geometry.md` | §2, §5, §8 |
| `client/AGENTS.md` | New short guide |

## Tests / acceptance

- [ ] Unit tests for mode matrix (incl. maxdim=2048 @ 1707×907 DPR 1.5 → mode 2)
- [ ] `pointer_to_logical(raw, dpr=1.5)` ≠ dividing by ui_scale when ui_scale=1
- [ ] Manual: resize at DPR 1.5, click button under cursor (if browser available)
- [ ] Docs §8 no longer forbids mode-2 backing without letterbox

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/01-canvas-geometry-pointer.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Do not edit unrelated agent-plans. Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
