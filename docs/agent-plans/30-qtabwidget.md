# 30 — QTabWidget / QTabBar

Status: done
Priority: P2
Depends-on: [01-canvas-geometry-pointer.md, 06-buttons-combo-qt-rounding.md]
Blocks: [35-qtoolbox.md]
Estimate: L

## Goal

`QTabWidget` + `QTabBar` parse from `.ui`, lay out, paint, switch tabs on click, expose Lua/protocol as needed; gallery golden + review.

## Qt correspondence

`QTabWidget`, `QTabBar`, tab shape Fusion.

## Context (read first)

- widgets-plan: Tab was scope-creep in 0.18 — this is the dedicated plan
- Rule: parser → model → layout → paint → events → golden → review
- Need G0 visuals first so tabs do not inherit strip/square bugs

## Scope

### In

- `.ui` class support, current index, tab text/icons
- Child page visibility
- Hit-test + keyboard optional if product needs
- Gallery fixtures

### Out

- Document mode / closable tabs / movable tabs unless already in product `.ui`
- QToolBox (35)

## Implementation steps

1. Inventory product `.ui` usage of QTabWidget.
2. form-xml + widget tree state.
3. Layout pages + tab bar metrics.
4. Paint bar + content clip.
5. Events; Lua props.
6. qt_reference + review.

## Files to touch

| Path | Change |
|------|--------|
| `form-xml`, `rqt/widgets`, `paint`, `app` events, gallery | As needed |

## Tests / acceptance

- [x] Unit + gallery golden within tolerance
- [x] `cargo test` relevant crates

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/30-qtabwidget.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
