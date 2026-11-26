# Qt parity guide (EPlatform client)

How to bring each graphics/UI layer closer to Qt Widgets + Fusion, without pretending we ship Qt’s paint engine.

Related:

- Handoff plans: [agent-plans/README.md](agent-plans/README.md)
- Canvas DPI: [wasm-canvas-geometry.md](wasm-canvas-geometry.md)
- Widgets waves: [`../widgets-plan.md`](../widgets-plan.md)
- Port history: [`../client/QT_PORT_PLAN.md`](../client/QT_PORT_PLAN.md)

---

## 1. Goal and boundaries

| In scope | Out of scope |
|----------|----------------|
| `.ui` widgets, layouts, Fusion-like style, QSS subset | Qt Quick / QML |
| Logical CSS px layout + wasm canvas DPI modes | Full Qt Modules catalog |
| Display list `QPaintCommand` → GPU/CPU backends | Pixel-perfect 0.0 review median everywhere |
| Product-needed missing widgets (tables first) | Pinch-zoom (deferred) |

---

## 2. Layer map (Qt ↔ EPlatform)

| Qt | EPlatform | Notes |
|----|-----------|--------|
| `.ui` / Widgets | `form-xml` + `QWidgetTree` | Strong |
| `QLayout` | `relayout` | Grid stretch not supported (product decision) |
| `QStyle` Fusion | `QFusionStyle` | Partial; indicators incomplete |
| `QPainter` | `QPaintCommand[]` | Good IR; GPU execution weak |
| Paint engine / RHI | `append_paint_commands` + `Gfx` (wgpu) | Historically solid-rect simulation |
| QSS | `qss` module | No `url()`/`border-image` yet → plan 41 |
| Window / DPR | winit + canvas geometry | Own contract — see geometry doc |

Full widget/primitive tables: see agent plan pack and historical review in chat plan §15.

---

## 3. Waves

| Wave | Focus | Plans |
|------|-------|-------|
| **G0** | GPU primitives, DPI/pointer, remove diag | `00`–`06` |
| **G2-Tables** | grouped headers + table paint | `10`–`11` |
| **G1** | QStyle / paint structure | `20`–`22` |
| **G2-Other** | Tab/Spin/Slider/… | `30`–`35` |
| **G3** | Path/QSS/perf | `40`–`42` |

**Order:** G0 foundation before new shiny widgets (otherwise they inherit strip/square GPU bugs). Tables are product P0 alongside G0.

---

## 4. Canvas / pointer invariants (corrected)

1. Wasm size from **DOM** (`getBoundingClientRect` + `devicePixelRatio`), not `inner_size` as SoT.
2. Pointer (winit-web): `logical = raw / dpr` — **never** `/ ui_scale`.
3. `ui_scale = surface_px / logical_px` with the pair chosen by mode:
   - **Mode 1:** `surface ≈ css×dpr`
   - **Mode 2:** `surface ≈ css` (browser upscales uniformly; **not** letterbox)
   - **Mode 3:** letterbox CSS + `surface ≈ letterbox×dpr`
4. Always `backing == configured surface`.
5. Prefer GPU primitives over logical-pixel strip lowering.

---

## 5. How to add a GPU primitive

1. Add/extend `QPaintCommand` only if Qt semantics need it.
2. Add `RenderPrimitive` variant (plan `05`).
3. Implement WGSL + `Gfx` draw path.
4. Map command → primitive in the shared lowerer (no copy-paste in `lib.rs` and `raster.rs`).
5. Keep `raster.rs` CPU-correct for review PNGs.
6. Add a 32×32 contract test when feasible.
7. Update this guide’s layer status.

---

## 6. How to add a widget

1. Inventory real `.ui` / Lua usage (no fake properties).
2. Parser (`form-xml`) → model on `QWidget`.
3. Layout / size hints.
4. `paint_*` via style + commands.
5. Events + protocol.
6. `tools/qt_reference` golden + gallery.
7. `review.py`; **never** edit `client/review/comments.md` on regen.

---

## 7. How to change QStyle

- Prefer `draw_primitive` / `draw_control` / `draw_complex_control`.
- Do not leave a second divergent path in `paint_control_indicator` for the same control (see plan `20`).
- Scrollbar should move toward complex control (plan `21`).

---

## 8. Tables (product priority)

See [10-tables-grouped-headers.md](agent-plans/10-tables-grouped-headers.md) and [11-tables-paint-parity.md](agent-plans/11-tables-paint-parity.md).

B6 in `questions.md`: `groupedHeader` / `tableLayout` **are required**. Implement parser→model→span geometry→paint→hit→gallery→review. Flat headers must keep working.

---

## 9. Acceptance cheat sheet

| Check | Command / action |
|-------|------------------|
| Unit | `cd client && cargo test --workspace` |
| Wasm compile | `cargo check -p app --target wasm32-unknown-unknown` |
| Review | `python3 tools/formgen/review.py` (from client) |
| DPR | Manual resize @ 1.5; click under cursor |
| Buttons | Corners radius 2; no gradient stripes |

---

## 10. Backlog index

All component briefs live under [agent-plans/](agent-plans/). Assign one file per agent.
