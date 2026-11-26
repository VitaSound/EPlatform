# 33 — QProgressBar

Status: ready
Priority: P2
Depends-on: [02-gpu-linear-gradient-radius.md]
Blocks: []
Estimate: S

## Goal

`QProgressBar` shows determinate (and optional indeterminate) progress matching Fusion-ish look.

## Qt correspondence

`QProgressBar`.

## Scope

### In

- min/max/value, text visible, orientation if needed
- Paint chunk + groove
- Property updates from Lua

### Out

- Busy indicator animations beyond simple pulse unless required

## Implementation steps

1. Parse + model + paint.
2. Gallery fixture.
3. Tests.

## Files to touch

| Path | Change |
|------|--------|
| form-xml, widgets, paint | As needed |

## Tests / acceptance

- [ ] Value 0/50/100 layouts correct
- [ ] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/33-qprogressbar.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
