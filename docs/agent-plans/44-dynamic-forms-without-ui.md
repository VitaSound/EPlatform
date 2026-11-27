# 44 — Dynamic forms without `.ui` (two variants)

Status: done (A+B adequacy bar; batching still phase 2)
Priority: P0
Depends-on: []
Blocks: []
Estimate: XL (adequate A+B together — not a thin MVP)

## Goal

Lua can build and grow a form **without Qt Designer / on-disk `.ui`**, the same way Qt itself allows either `QUiLoader` **or** `new QWidget` + layouts in code. EPlatform keeps `.ui` as one path, and adds a first-class code path aligned with the old `tcs_qtgui` workflow the product already used after Designer was abandoned.

## Qt correspondence

| Qt | Legacy tcs | EPlatform target |
|----|------------|------------------|
| `QUiLoader::load(.ui)` | `create_form` + `ui_xml` | already works |
| `new QWidget` empty window | `create_empty_form` | missing (decision B1 reversed) |
| `new QVBoxLayout` / `addWidget` / `addLayout` | `invoke_qt` `addLayout` / `addWidget` | typed commands (no open `invoke_qt`) |
| `connect(signal,…)` | `invoke_qt` `connectSignal` | prefer existing `form:on` / `formEvent` |
| setProperty | `set_property` / invoke setProperty | already works |

Reference implementation (read, do not port blindly):

- `/home/sea/tcs_qtgui/tcscore.cpp` — `create_empty_form`, `invoke_qt`
- `/home/sea/tcs_qtgui/mainwindow.cpp` — `onCreateEmptyForm`
- `/home/sea/tcs_qtgui/invoke_mainwindow.cpp` — `addLayout` / `addWidget` / `connectSignal` (~797+)

## Context (read first)

- Prior product decision **B1** (`questions.md`): `create_empty_form` not needed — **superseded by this plan**; document the reversal in `questions.md` when implementing.
- `docs/lua-module-composition.md` § этап 5 already foreshadowed `addLayout`/`addWidget` «только если нужны» and forbade open `invoke_qt`.
- Client already has `QWidgetTree::create_widget` / layout attach in Rust tests; not exposed over the wire.
- Forms today: server reads `.ui` file → `createForm { ui_xml }` → `form-xml` parse → tree.
- Hard rules: no generic `invoke_qt` / reflection bridge; only an allowlisted command set. Wasm geometry invariants unchanged.

### Why two variants

Both are legitimate Qt styles. The product wants **both** over time:

1. **Declarative build-at-open** (no Designer file, one shot tree).
2. **Imperative grow-at-runtime** (empty shell, then add controls — tcs muscle memory).

They share client tree mutation primitives; they differ in **who assembles the first tree** and **how many round-trips**.

---

## Canonical Lua API (product choice — locked)

User-facing sugar is **Form + widget/layout handles**, not raw `invoke_qt`.
A handle is a thin proxy `{ form, object_name }` that reuses existing
`set` / `get` / `on` (and new structure ops). String-based
`form:set("ok", "text", …)` stays valid forever.

```lua
form:create_empty(app, id, { mdi = true })

local root = form:add_layout("QVBoxLayout", "root")
local name = form:add("QLineEdit", "nameEdit", { parent = root })
local ok   = form:add("QPushButton", "ok", { parent = root, text = "OK" })

ok:on("Clicked", function()
  print(name:get("text"))
  name:set("text", "")
  ok:set_enabled(false)
end)

-- same without handles (today's style):
form:set("nameEdit", "text", "hi")
form:on("ok", "Clicked", function() end)

ok:hide()
form:remove(ok)   -- leave layout; destroy optional flag later
```

Also: `form:widget("nameEdit")` / `form:find("nameEdit")` → handle for
widgets that already exist (from `.ui` or prior `add`).

| Handle / Form method | Maps to |
|----------------------|---------|
| `form:create_empty` | `createEmptyForm` / empty `createForm` |
| `form:add(class, name, opts)` | `addWidget` (+ initial props) → returns handle |
| `form:add_layout(class, name, opts)` | `addLayout` → returns handle |
| `handle:set` / `:get` / `:on` | existing property + `formEvent` path |
| `handle:set_enabled` / `:hide` / … | sugar → `setProperty` |
| `form:remove(handle\|name)` | `removeWidget` (A2+; optional in A1) |
| Variant B `Ui.*` DSL | one-shot tree; after create, same handles via `form:widget` |

**Do not** expose open `invoke_qt`. Wire protocol stays typed commands;
handles are Lua-only convenience.

---

## Variant A — Imperative (tcs-shaped)

**Mental model:** `create_empty` → `add_layout` / `add` → handle `:set` / `:on`.

### Protocol (allowlist)

| Command | Payload (sketch) | Client effect |
|---------|------------------|---------------|
| `createForm` | `ui_xml` omitted/empty **or** new `createEmptyForm` | empty root `QDialog`/`QWidget`, same MDI/modal/placement as today |
| `addLayout` | `class`, `object_name`, `parent` (widget or layout name), opts (grid cell?, stretch?) | create layout, set on widget or nest into parent layout |
| `addWidget` | `class`, `object_name`, `parent`, opts (`text`, `visible`, `enabled`, `geometry`, …) | `create_widget`, register name, insert into layout |
| existing | `setProperty` / `setFormProperty` / `getProperty` | unchanged |
| optional later | `removeWidget` / `reparent` | out of first slice |

Host API: `create_empty_form(...)` + thin wrappers that emit the new commands (prefer **separate command names** matching tcs logs for debuggability).

### Pros

- Matches `/home/sea/tcs_qtgui` and incremental UI (show form, then fill).
- Natural for wizards, conditional controls, runtime toolbars.

### Cons

- Many WS round-trips unless batched.
- Harder to golden-diff vs Qt Designer PNG (no single `.ui` unless recorded).

### Adequacy bar (A — not a toy slice)

1. `create_empty` + `add_layout` / `add` / `remove` with **handles**.
2. Layouts: **VBox / HBox / Grid / Form** (cells for grid/form).
3. Widgets: gallery-common allowlist (labels, edits, buttons, checks/radios, combo/spin, plain text, group box, progress/slider, tool button, list/table/tree, tab/splitter/toolbox as containers).
4. Relayout + hit-test + Lua demo **without** any `.ui`.
5. Batching (`applyMutations`) stays a follow-up perf phase, not a gate for “done”.

---

## Variant B — Declarative (Lua tree → `ui_xml` / form-xml)

**Mental model:** build a Lua table (or small DSL), serialize once to the **same** `createForm.ui_xml` (or a parallel JSON spec that `form-xml` already understands), no empty shell required.

```lua
local ui = Ui.dialog {
  name = "DynDialog",
  layout = Ui.vbox {
    Ui.label { name = "title", text = "Hello" },
    Ui.line_edit { name = "nameEdit" },
    Ui.hbox {
      Ui.button { name = "ok", text = "OK" },
      Ui.button { name = "cancel", text = "Cancel" },
    },
  },
}
form:create(app_id, form_id, { ui_spec = ui })  -- or ui_xml = Ui.to_xml(ui)
```

Server (or pure Lua) turns `ui_spec` into Designer-compatible XML **or** a compact IR consumed by an extended `form-xml` loader. Prefer **XML** first: zero new client parse paths; gallery/review tools keep working if you dump the XML.

### Pros

- Reuses entire load/layout/paint stack.
- Easy snapshots / review goldens (write generated XML to fixtures).
- Fewer round-trips; good default when the whole form is known at `init`.

### Cons

- Runtime structural edits still need Variant A (or rebuild+replace form).
- DSL design cost; must stay a subset of what `form-xml` parses.

### Adequacy bar (B — same breadth as A)

1. Lua `Ui` module: dialog/widget + **vbox/hbox/grid/form** + spacers + same control allowlist → Designer-compatible XML.
2. `Form:create(..., { ui_xml = ... })` / `{ ui_spec = Ui.* }` — no on-disk `.ui`.
3. Nested containers (group box, tabs, splitter) expressible in the DSL.
4. Demo `test_dyn_declarative.lua` with **zero** files under `priv/qt_forms/`.
5. Optional XML golden for parse round-trip.

---

## Recommended delivery order

| Phase | What | Why |
|-------|------|-----|
| **0** | Reverse B1 in `questions.md`; Wiki note under Lua Form API | product clarity |
| **1** | **Variant B + Variant A together** at the adequacy bars above (handles, allowlist, both demos) | “no Designer” + tcs runtime growth without a second thin pass |
| **2** | Batching: `applyMutations([…])` or Lua-side buffer flushed once | perf |
| **3** | Stretch: more exotic props / page APIs if gallery gaps appear | polish |

Do **not** ship a “4-widget MVP” and call the plan done. Do **not** reintroduce open `invoke_qt`. `connectSignal` only if `form:on` cannot cover the event; prefer mapping Qt signal names → existing `formEvent` types.

## Scope

### In

- Both variants documented and phased as above
- Allowlisted widget/layout classes ⊆ current [[Supported Widgets]] / rqt support
- Lua platform sugar + host commands + client tree mutation
- Tests: unit (tree mutate), one declarative demo, one imperative demo
- User docs: regenerate Lua API pages; short “Dynamic forms” wiki page (hand-written)

### Out

- Full Qt Designer feature parity in the DSL
- Arbitrary QMetaObject invoke / property reflection kitchen sink
- Shipping Qt Designer or `.ui` authoring tools
- Pinch-zoom, unrelated paint plans (40–43)

## Implementation steps (agent)

1. Confirm phase (B-first vs A-first) with status note in this file if user already chose; default **B then A**.
2. Inventory allowlist from `Supported-Widgets` / `widgets.rs`.
3. Implement chosen first slice; keep commands typed in Phoenix payload.
4. Add `server/priv/lua_modules/test_dyn_*.lua` demos; no `.ui` dependency.
5. Update `questions.md` B1, `docs/user` notes, run `gen_user_docs.py`.
6. Acceptance below; set Status `done` for the slice shipped (or split follow-ups `44b`/`44a` if needed).

## Files to touch (expected)

| Path | Change |
|------|--------|
| `server/lib/e_platform/lua/api.ex` | `create_empty_form`; optional `ui_spec` |
| `server/lib/e_platform/dialog_*.ex` | start dialog without ui file |
| `server/priv/lua_modules/form.lua` | `create_empty`, `add_*` sugar |
| `server/priv/lua_modules/ui.lua` (new) | Variant B DSL → XML |
| `client/crates/app/src/lib.rs` | handle empty create + mutate commands |
| `client/crates/rqt/src/widgets.rs` | public mutate helpers (create/attach layout) |
| `questions.md` / `docs/user/*` | decision + docs |
| `docs/lua-module-composition.md` | stage 5 → implemented |

## Tests / acceptance

- [x] Declarative demo opens with **zero** `.ui` file (`test_dyn_declarative.lua` + mix test asserts inline `ui_xml`)
- [x] Imperative demo: empty form → vbox → line edit + button (`test_dyn_imperative.lua` + mix test for `createEmptyForm`/`add*`)
- [x] `rqt` unit: `dynamic_add_layout_and_widget`; `cargo check -p app`; `mix test test/e_platform/dynamic_forms_test.exs`
- [x] Wasm: open `?lua=test_dyn_….lua` (no `ui=`), live CDP verify (imperative + declarative paint)
- [x] No `invoke_qt` command in protocol
- [x] `docs/user/Dynamic-Forms.md` + Home/Sidebar mention both variants

## Agent handoff prompt (copy-paste)

> Implement **phase 1 (Variant B)** of `docs/agent-plans/44-dynamic-forms-without-ui.md` end-to-end unless the user names Variant A first.
> Lua surface must follow **Canonical Lua API** (Form + handles); keep string `form:set`/`form:on` working.
> Do not expand beyond that phase’s allowlist. Do **not** add open `invoke_qt`.
> Reverse/document B1 in `questions.md`. Update Status notes when the slice passes acceptance.
> Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
> Legacy reference only: `/home/sea/tcs_qtgui` `create_empty_form` + `addWidget`/`addLayout` — mirror semantics with typed EPlatform commands, do not copy QUiLoader factory wholesale.
