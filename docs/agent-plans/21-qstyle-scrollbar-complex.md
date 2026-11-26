# 21 — ScrollBar via QStyle complex control

Status: ready
Priority: P1
Depends-on: [02-gpu-linear-gradient-radius.md, 05-render-primitive-lower.md]
Blocks: [32-qslider.md]
Estimate: L

## Goal

`QComplexControl::ScrollBar` is drawn through `QStyle` (or thin wrapper), not only a standalone `paint_scroll_bar` island — while preserving current Fusion probe fidelity.

## Qt correspondence

`QStyle::drawComplexControl(CC_ScrollBar, …)`.

## Context (read first)

- `paint_scroll_bar` ~200 lines in `paint.rs`
- `QComplexControl` enum already exists in `style.rs`

## Scope

### In

- Move or wrap scrollbar painting behind style API
- Keep visual parity with existing probes
- Use gradient primitive from 02

### Out

- QSlider full widget (32) — only share primitives if easy
- Changing scroll math / hit testing beyond paint ownership

## Implementation steps

1. Design `draw_complex_control` signature.
2. Relocate paint_scroll_bar body.
3. Update call sites in list/table/tree.
4. Regression on tables with bars.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/style.rs` | API |
| `client/crates/rqt/src/paint.rs` | Call style |
| `client/crates/rqt/src/scroll.rs` | If logic moves |

## Tests / acceptance

- [ ] Scroll visuals unchanged vs prior review baselines
- [ ] `cargo test -p rqt`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/21-qstyle-scrollbar-complex.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
