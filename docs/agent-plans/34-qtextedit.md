# 34 — QTextEdit / QPlainTextEdit

Status: ready
Priority: P2
Depends-on: [01-canvas-geometry-pointer.md]
Blocks: []
Estimate: XL

## Goal

Multi-line text edit with selection, caret, scroll, basic bindings — enough for product forms.

## Qt correspondence

`QPlainTextEdit` first (simpler); `QTextEdit` rich text only if product needs HTML.

## Scope

### In

- Plain text model, wrap modes used in product
- Paint text + selection + caret
- Scroll; IME hooks consistent with existing line edit mobile path
- Lua get/set plain text

### Out

- Full HTML/QTextDocument
- Syntax highlighting
- Tables inside documents

## Implementation steps

1. Decide Plain vs Rich from `.ui` inventory.
2. Buffer + layout lines + paint.
3. Input events; selection.
4. Gallery + tests; budget XL carefully.

## Files to touch

| Path | Change |
|------|--------|
| widgets, paint, app input, gfx text | Large |

## Tests / acceptance

- [ ] Type, select, scroll work on wasm
- [ ] Golden for static content
- [ ] `cargo test`

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/34-qtextedit.md` end-to-end.
> Do not expand scope beyond **In**. Prefer QPlainTextEdit-first. Update **Status** to `done` when acceptance passes.
