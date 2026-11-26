# 00 — Remove diagnostic HUD and `__ep*` hooks

Status: done
Priority: P0
Depends-on: []
Blocks: [01-canvas-geometry-pointer.md]
Estimate: S

## Goal

Production wasm client has no `?diag=1` HUD, no colored markers, and no `window.__epPtr` / `__epView` / `__ptrLog` / `__epBuild` publishing. Texture-limit override (`maxdim`) is test-only or removed from the prod URL path.

## Qt correspondence

N/A (host tooling). Keeps the browser shell closer to a normal Qt-hosted window (no debug overlay).

## Context (read first)

- [wasm-canvas-geometry.md](../wasm-canvas-geometry.md) §5 (diag inventory — delete and rewrite section)
- `client/crates/app/index.html` — `?diag=1`, markers, `__ptrLog`, `maxdim`
- `client/crates/app/src/lib.rs` — `publish_pointer_debug`
- `client/crates/app/src/gfx.rs` — `max_dim_override` / `__epMaxDim`

## Scope

### In

- Remove HUD DOM, cyan/magenta/red markers, `__ptrLog` recording
- Remove `publish_pointer_debug` and all `__epPtr` / `__epView` sets from Rust
- Remove or gate `__epBuild` (prefer remove from prod path)
- Move `maxdim` / `__epMaxDim` behind `#[cfg(test)]` or cargo feature `ep-test-hooks`; strip from `index.html` query parsing for release UX
- Update wasm-canvas-geometry §5 to “diagnostics removed; use unit tests”

### Out

- Changing geometry algorithms (see 01)
- Adding a replacement HUD

## Implementation steps

1. Strip diag JS block from `index.html` (HUD, markers, pointer log, maxdim query).
2. Delete `publish_pointer_debug` and its call from `pointer_moved`.
3. Remove Reflect sets for `__epView` / `__epPtr` / `__epBuild` if present.
4. Gate or delete `max_dim_override` in `gfx.rs`.
5. Rewrite docs §5; link to geometry unit tests in 01.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/app/index.html` | Remove diag/maxdim UI hooks |
| `client/crates/app/src/lib.rs` | Remove debug publish |
| `client/crates/app/src/gfx.rs` | Gate/remove maxdim override |
| `docs/wasm-canvas-geometry.md` | §5 rewrite |

## Tests / acceptance

- [ ] Grep client for `__epPtr`, `__epView`, `__ptrLog`, `ep-diag`, `diag=1` → no prod hits
- [ ] `cargo test -p app` still passes
- [ ] Page load without query shows no HUD overlays

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/00-remove-diag.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Do not edit unrelated agent-plans. Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
