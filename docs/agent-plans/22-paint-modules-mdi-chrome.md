# 22 — Split paint modules; MDI chrome into rqt

Status: ready
Priority: P2
Depends-on: []
Blocks: []
Estimate: M

## Goal

`paint.rs` split into `paint/{mod,table,scroll,focus,…}`; MDI subwindow chrome painting lives in `rqt::mdi`, not `app/lib.rs`. `collect_paint` stays thin.

## Qt correspondence

Qt module boundaries (widgets vs gui); chrome still QPainter-based.

## Context (read first)

- `app/src/lib.rs` `paint_subwindow_chrome`
- Large `rqt/src/paint.rs`

## Scope

### In

- File split without behavior change
- Move MDI chrome paint into rqt
- Optional: focus ring without 1×1 dotted rects (or defer to 40)

### Out

- Visual redesign
- GPU work

## Implementation steps

1. Split modules; keep `pub use` API stable.
2. Move chrome; update `collect_paint`.
3. Ensure tests still find paint entry points.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/paint*.rs` | Split |
| `client/crates/rqt/src/mdi.rs` | Chrome paint |
| `client/crates/app/src/lib.rs` | Thinner collect |

## Tests / acceptance

- [ ] `cargo test --workspace` client crates
- [ ] No visual change (same commands order)

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/22-paint-modules-mdi-chrome.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
