# 32 — QSlider

Status: done
Priority: P2
Depends-on: [21-qstyle-scrollbar-complex.md]
Blocks: []
Estimate: M

## Goal

Horizontal/vertical `QSlider` with groove, handle, value binding.

## Qt correspondence

`QSlider`; `CC_Slider` in QStyle.

## Scope

### In

- Orientation, range, value, paint, drag
- Style complex control if 21 landed

### Out

- Tick marks customization beyond Qt defaults needed by product
- QDial

## Implementation steps

1. Inventory `.ui` usage.
2. Model + paint + drag math.
3. Gallery + tests.

## Files to touch

| Path | Change |
|------|--------|
| widgets, style, paint, app | As needed |

## Tests / acceptance

- [x] Drag sets value; keyboard if required
- [x] Visual vs Qt
- [x] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/32-qslider.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
