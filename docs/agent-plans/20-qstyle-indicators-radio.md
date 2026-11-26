# 20 — QStyle indicators and single radio path

Status: ready
Priority: P1
Depends-on: [04-gpu-texture-blit-circle.md]
Blocks: []
Estimate: M

## Goal

`PE_Indicator*` / arrows implemented in `draw_primitive`. `QRadioButton` uses one path (style + indicator), not square in style and circle in `paint_control_indicator`.

## Qt correspondence

`QStyle::drawPrimitive` indicators; `CE_RadioButton` / `CE_CheckBox`.

## Context (read first)

- `style.rs`: Indicator arms currently `_ => Vec::new()`
- `paint_control_indicator` draws radio ellipses; CheckBox uses style square + lines
- Contradiction documented in qt-parity-guide §15.3

## Scope

### In

- Implement missing primitives used by widgets
- Unify radio drawing
- Checkbox check mark via style or shared helper

### Out

- Scrollbar complex control (21)
- New widgets

## Implementation steps

1. Inventory which PE_* are referenced.
2. Implement draw_primitive arms.
3. Route radio through style; delete duplicate path.
4. Tests for command streams.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/style.rs` | Primitives |
| `client/crates/rqt/src/paint.rs` | Call sites |

## Tests / acceptance

- [ ] Radio/check gallery states match Qt within tolerance
- [ ] No dual square+circle for radio
- [ ] `cargo test -p rqt`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/20-qstyle-indicators-radio.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
