# 35 — QToolBox

Status: done
Priority: P2
Depends-on: [30-qtabwidget.md]
Blocks: []
Estimate: M

## Goal

`QToolBox` accordion pages work. Do **not** confuse with `QToolBar` separators (widgets-plan decision).

## Qt correspondence

`QToolBox`.

## Scope

### In

- Pages with titles; expand one/more per Qt default used by product
- Paint headers + content
- Click to switch

### Out

- QToolBar changes
- QTabWidget (30)

## Implementation steps

1. Confirm product needs ToolBox vs tabs.
2. Parser + state + paint + events.
3. Gallery + tests.

## Files to touch

| Path | Change |
|------|--------|
| form-xml, widgets, paint, app | As needed |

## Tests / acceptance

- [x] Page switch works
- [x] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/35-qtoolbox.md` end-to-end.
> Do not treat toolbar separators as ToolBox. Update **Status** to `done` when acceptance passes.
