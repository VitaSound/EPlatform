# 10 — Tables: groupedHeader / tableLayout (B6)

Status: done
Priority: P0
Depends-on: [01-canvas-geometry-pointer.md]
Blocks: [11-tables-paint-parity.md]
Estimate: L

## Goal

`groupedHeader` / `tableLayout` (questions.md B6 — accepted as needed) works end-to-end: protocol/model → geometry spans → paint → hit/resize → gallery golden → review.

## Qt correspondence

`QHeaderView` multi-level / span sections; product `tableLayout`/`groupedHeader` props (not invent fake Qt properties without a source — follow existing Lua/protocol design in questions B6).

## Context (read first)

- `questions.md` B6
- `rqt/src/item_model.rs` `QTableWidget`
- `paint_table` / `paint_header_*` in `paint.rs`
- Rule: parser → model → layout → paint → events → golden → review

## Scope

### In

- Data model for grouped/span headers
- Parsing from `.ui` and/or Lua/protocol as already decided in B6
- Paint spanning sections
- Hit-testing and column resize interaction with groups
- Gallery fixtures + qt_reference PNGs as needed
- Do not break flat single-row headers

### Out

- QTabWidget / other widgets
- Full `QHeaderView` class extraction (can stay embedded)
- Changing selection model semantics unrelated to headers

## Implementation steps

1. Spec fields in guide + this file: span tree, heights, labels.
2. Extend `QTableWidget` / header model.
3. Geometry: section rects with horizontal spans.
4. Paint: reuse header gradient after 02 if available.
5. Hit: section at point; resize edges.
6. Add gallery form(s); regenerate review carefully (never edit `comments.md`).

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/item_model.rs` | Model |
| `client/crates/rqt/src/paint.rs` | Header paint |
| `client/crates/form-xml` / protocol / server Lua | As required by B6 source |
| `client/crates/app` | Events if resize/hit live there |
| Gallery / review assets | New fixtures |

## Tests / acceptance

- [ ] Unit tests for span geometry
- [ ] Flat tables unchanged
- [ ] Grouped fixture matches Qt golden within project tolerance
- [ ] `cargo test --workspace` relevant crates
- [ ] review tables/* not regressed

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/10-tables-grouped-headers.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Never modify `client/review/comments.md`. Follow widgets-plan additive rules.
