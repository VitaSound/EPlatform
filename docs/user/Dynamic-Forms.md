# Dynamic Forms (without `.ui`)

EPlatform supports the same two Qt styles for building a form:

1. **Load Designer XML** — `Form:create(app, id, "file.ui")` (existing path).
2. **Build in Lua** — no Qt Designer, no file under `priv/qt_forms/`.

## Variant B — Declarative (`Ui.*` → XML)

```lua
local Ui = require("ui")
local form = require("form")()

local spec = Ui.dialog {
  name = "Hello",
  windowTitle = "Hello",
  width = 400, height = 240,
  layout = Ui.vbox {
    name = "root",
    Ui.label { name = "title", text = "Name" },
    Ui.line_edit { name = "nameEdit" },
    Ui.hbox {
      name = "row",
      Ui.button { name = "ok", text = "OK" },
      Ui.button { name = "cancel", text = "Cancel" },
    },
  },
}

form:create(app_id, form_id, { ui_spec = spec })
-- or: { ui_xml = Ui.to_xml(spec) }
```

After create, use the usual `form:set` / `form:on`, or handles via `form:widget("ok")`.

Demo: `test_dyn_declarative.lua` (open with `?lua=test_dyn_declarative.lua`, no `ui=`).

## Variant A — Imperative (empty + add)

```lua
form:create_empty(app_id, form_id, { window_title = "Dyn", width = 420, height = 260 })

local root = form:add_layout("QVBoxLayout", "root")
local name = form:add("QLineEdit", "nameEdit", { parent = root })
local ok   = form:add("QPushButton", "ok", { parent = root, text = "OK" })

ok:on("Clicked", function()
  print(name:get("text"))
end)

form:remove(ok)  -- optional
```

Wire commands are typed (`createEmptyForm`, `addLayout`, `addWidget`, `removeWidget`) — there is **no** open `invoke_qt`.

Demo: `test_dyn_imperative.lua`.

## Handles

| API | Meaning |
|-----|---------|
| `form:widget(name)` / `form:find(name)` | handle for an existing object |
| `form:add(class, name, opts)` | `addWidget` → handle |
| `form:add_layout(class, name, opts)` | `addLayout` → handle |
| `handle:set` / `:get` / `:on` | same as `form:set(name, …)` |
| `handle:set_enabled` / `:hide` / `:show` | sugar |
| `form:remove(handle\|name)` | detach from layout |

String-based `form:set("ok", "text", …)` remains fully supported.

## Allowlist

Use classes from [[Supported-Widgets]] (layouts + gallery-common controls). Exotic Designer-only nodes are out of scope; prefer the declarative path when you need nested tabs/splitters in one shot.
