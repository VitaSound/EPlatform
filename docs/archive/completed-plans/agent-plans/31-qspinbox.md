# 31 — QSpinBox / QDoubleSpinBox

Status: done
Priority: P2
Depends-on: [01-canvas-geometry-pointer.md, 02-gpu-linear-gradient-radius.md]
Blocks: []
Estimate: M

## Goal

Spin boxes parse, display value, step via buttons/keys, sync properties to Lua.

## Qt correspondence

`QSpinBox`, `QDoubleSpinBox`.

## Scope

### In

- `.ui` properties: min/max/step/value/suffix/prefix as present in product
- Paint: frame + text + up/down indicators
- Mouse on buttons; optional wheel
- Protocol setValue / valueChanged as existing patterns allow

### Out

- QDateTimeEdit
- Full validator framework

## Implementation steps

1. Usage inventory → minimal property set.
2. Model + paint + hit.
3. Tests + gallery.

## Files to touch

| Path | Change |
|------|--------|
| form-xml, widgets, paint, style, app | As needed |

## Tests / acceptance

- [x] Step up/down changes value
- [x] Golden vs Qt within tolerance
- [x] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/31-qspinbox.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
