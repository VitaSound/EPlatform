# 06 — Acceptance: buttons/combo rounded like Qt Fusion

Status: done
Priority: P0
Depends-on: [02-gpu-linear-gradient-radius.md, 03-gpu-rounded-rect-stroke.md]
Blocks: []
Estimate: S

## Goal

QPushButton / QToolButton / QComboBox frames show radius ≈ 2 like Qt Fusion on wasm at DPR 1.0 and 1.5 — fill and stroke. No square stroke over rounded fill.

## Qt correspondence

Fusion `CE_PushButton` / `CE_ComboBox` (already emit radius 2 in `QFusionStyle`).

## Context (read first)

- `rqt/src/style.rs` PushButton/ComboBox commands
- Plans 02 + 03 must be done (or implement minimal fix here only if blocked — prefer not)

## Scope

### In

- Visual verification + any glue fixes if 02/03 left gaps
- Optional small gallery fixture focus on buttons
- Document acceptance in qt-parity-guide checklist

### Out

- New button styles / QSS themes
- Checkbox/radio (20)

## Implementation steps

1. Confirm style still emits radius 2.
2. Confirm GPU/lowerer honors fill+stroke radius.
3. Compare to Qt golden or review button forms.
4. Fix any remaining square outline.

## Files to touch

| Path | Change |
|------|--------|
| Mostly verification; touch style/gfx/lib only if bugs found | … |

## Tests / acceptance

- [ ] Enabled/hover/pressed/disabled button corners rounded
- [ ] Combo field rounded
- [ ] DPR 1.5 no gradient banding on button face
- [ ] `cargo test -p rqt` style tests green

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/06-buttons-combo-qt-rounding.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
