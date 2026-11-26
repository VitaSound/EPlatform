# 41 — QSS url() / border-image + assets

Status: ready
Priority: P2
Depends-on: [04-gpu-texture-blit-circle.md]
Blocks: []
Estimate: L

## Goal

QSS `url()` and `border-image` work with an asset pipeline (widgets-plan noted these as unsupported).

## Qt correspondence

Qt Style Sheets images.

## Scope

### In

- Asset loading path for wasm + native tests
- Parse url/border-image in qss module
- Paint via textured primitives from 04

### Out

- Full CSS image slicing edge cases unused by product
- Remote http URLs unless required

## Implementation steps

1. Define asset root / packing.
2. Parser + resolve.
3. Paint border-image.
4. Gallery fixture with local asset.

## Files to touch

| Path | Change |
|------|--------|
| `rqt/src/qss.rs`, paint, gfx, build/assets | As needed |

## Tests / acceptance

- [ ] Fixture with border-image matches expectation
- [ ] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/41-qss-url-border-image.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
