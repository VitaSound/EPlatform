# 11 — Tables: paint_table parity with Qt gallery

Status: done
Priority: P0
Depends-on: [10-tables-grouped-headers.md, 02-gpu-linear-gradient-radius.md]
Blocks: []
Estimate: M

## Goal

`paint_table` (headers, grid, selection, disabled, scrollbars, frame-over-bars) matches Qt probes used in gallery/tables without regressing median diffs.

## Qt correspondence

`QTableWidget` + `QHeaderView` + `QScrollBar` Fusion look.

## Context (read first)

- `paint_table`, `paint_header_section`, `paint_scroll_bar` in `paint.rs`
- Known probes: frame over scrollbar (tables/179), header gradient, disabled gray fill
- Key review ids historically: tables/35, 181, etc.

## Scope

### In

- Align remaining visual gaps called out in review comments (except comments.md edits)
- Ensure header gradients use GPU path from 02 (no banding)
- Preserve virtualization behavior

### Out

- Grouped header feature work (10)
- New table edit modes

## Implementation steps

1. Run / inspect review diffs for `gallery/tables/*`.
2. Fix paint order (frame above bars), disabled fills, header edges.
3. Re-check after gradient GPU lands.
4. Regenerate review if needed; do not touch `comments.md`.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/paint.rs` | Table/header/scroll |
| review PNGs | Only via regen tools |

## Tests / acceptance

- [ ] `cargo test -p rqt`
- [ ] tables/* median not worse; spot-check 35, 181, 179
- [ ] Headers no stripe banding @ DPR 1.5 on wasm

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/11-tables-paint-parity.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
