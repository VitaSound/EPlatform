# <id> — <короткое имя>

Status: ready
Priority: P0 | P1 | P2
Depends-on: []
Blocks: []
Estimate: S | M | L | XL

## Goal

1–3 предложения: что станет true после merge.

## Qt correspondence

Какой Qt API / виджет / поведение. См. [qt-parity-guide.md](../qt-parity-guide.md).

## Context (read first)

- Related docs and code paths
- Known bugs (B1–B4) if relevant
- Hard prohibitions (do not violate)

## Scope

### In

- …

### Out

- …

## Implementation steps

1. …
2. …

## Files to touch

| Path | Change |
|------|--------|
| … | … |

## Tests / acceptance

- [ ] `cargo test --workspace` (or scoped crates)
- [ ] Gallery / review ids if visual
- [ ] Manual case if wasm/DPI

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/<this-file>.md` end-to-end.
> Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Do not edit unrelated agent-plans. Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
