# Agent plans — Qt graphics parity (handoff pack)

Self-contained briefs for parallel agents. Each file is one component of work.
Parent overview: [qt-parity-guide.md](../qt-parity-guide.md).
Canvas DPI invariants: [wasm-canvas-geometry.md](../wasm-canvas-geometry.md).

## How to hand off

1. Pick **one** file below (or add a new plan from the template).
2. Paste the **Agent handoff prompt** from that file into a new agent session.
3. Do not expand **Out** of scope.
4. When acceptance passes, set `Status: done` at the top of the file, then move it to the archive.

Template: [_TEMPLATE.md](_TEMPLATE.md).

## Active plans

*(none — pack `00`–`44` archived 2026-11-27.)*

## Archive

Closed handoffs `00`–`44` live under
[archive/completed-plans/agent-plans/](../archive/completed-plans/agent-plans/).

## Hard rules (all agents)

- Wasm canvas size: DOM only, not `winit::inner_size()` as source of truth.
- Pointer: `logical = raw / dpr` (never `/ ui_scale`).
- `backing == surface`; mode 2 may use `backing ≈ css` (not `css×dpr`) without letterbox.
- Do not touch `client/review/comments.md` when regenerating review.
- Prefer GPU primitives over logical-pixel strip lowering for gradients/rounded/ellipses.
