# 43 — Layout / style metrics: dedup + Qt-shaped sources

Status: done
Priority: P2
Depends-on: []
Blocks: []
Estimate: M

### Status notes

Consolidated (same Fusion numbers, named sources):
- Layout defaults → `QLAYOUT_DEFAULT_SPACING` / `QLAYOUT_ROOT_MARGIN` / `QLAYOUT_NESTED_MARGIN`; `QTOOLBOX_SPACING` aliases spacing.
- `tab_bar_height()` → `QFusionStyle::pixel_metric(TabBarHeight)`.
- `QPixelMetric::ScrollBar*` / `SliderThickness|Length` → domain consts `QSCROLLBAR_*` / `QSLIDER_THICKNESS`.
- Check/radio text inset → `ButtonMargin`; shared sizeHint v-pad → `QSIZEHINT_FONT_V_PAD`.
- `cargo test -p rqt --lib`: 175 passed.

## Goal

Repeated layout/chrome sizes have a single named source (domain `const` or `QPixelMetric`), matching the convention in `client/AGENTS.md` § Layout / style metrics. Call sites stop re-hardcoding the same Fusion probe literals.

## Qt correspondence

`QStyle::pixelMetric` / `sizeFromContents` for chrome; item-view section sizes and layout margins/spacing as separate concerns (not one global metrics dump). See [qt-parity-guide.md](../qt-parity-guide.md).

## Context (read first)

- Convention (SoT): [client/AGENTS.md](../../client/AGENTS.md) — **Layout / style metrics**
- Partial API already: `rqt/src/style.rs` → `QPixelMetric`, `QFusionStyle::pixel_metric`
- Domain consts already: `QTABLE_*` (`item_model.rs`), `QVIEW_*` (`item_view.rs`), `QMDI_*`, `QSCROLLBAR_*`, some widget consts in `widgets.rs`

### Mandatory re-audit before coding

This plan is intentionally **open**. By the time it is picked up, many new literals will exist from waves 20–42. **Do not** implement only the seed list below.

Before any edits:

1. Re-scan `client/crates/rqt` (and app hit-test paths that import metrics) for duplicated size literals: margins, spacing, row/header heights, scrollbar/tab/menu chrome, size-hint paddings reused in ≥2 places.
2. Rewrite the **Seed findings → Current findings** section in this file with paths/line refs from *today’s* tree.
3. Only then dedup what the fresh scan shows; keep scope to repeated/shared values, not every one-off paint nudge.

## Scope

### In

- Re-audit + document current duplicates in this file
- Extract shared layout defaults (spacing / root vs nested margins) to named consts or style metrics
- Route chrome callers through `QPixelMetric` where the value already lives in (or belongs in) `QFusionStyle`
- Remove duplicate literals that shadow existing domain consts / pixel metrics
- Short note in this plan’s Status notes when done (what was consolidated)

### Out

- One mega `LAYOUT_METRICS` / god-constants crate
- Renaming every local `-1.0` / caret nudge used once
- Changing visual probe targets (numbers stay Fusion-matched; only the *source of truth* moves)
- QSS theme metrics overhaul (see 41)

## Implementation steps

1. **Re-audit** (required): ripgrep / read layout load, size hints, paint modules; fill **Current findings**.
2. Layout defaults: single named source for default spacing and margins used when `.ui` omits them.
3. Chrome: extend `QPixelMetric` if needed; replace local helpers that return the same number (e.g. tab bar height) with `style.pixel_metric(...)`.
4. Align duplicated `QSCROLLBAR_*` (or similar) with style metrics — one definition, others call through or re-export clearly.
5. Run tests; spot-check gallery layouts that rely on default margins/spacing.

## Seed findings (historical — verify again)

Known at plan creation; treat as hints only:

| Area | Symptom |
|------|---------|
| `widgets.rs` layout load | Default spacing `6.0`, root margins `11.0`, nested `9.0` as literals |
| `widgets.rs` `tab_bar_height()` | Hardcoded `28.0` while `QPixelMetric::TabBarHeight` already exists |
| `scroll.rs` vs `style.rs` | `QSCROLLBAR_EXTENT` / `SLIDER_MIN` mirror `ScrollBarExtent` / `SliderMin` |
| size hints | Repeated paddings (`+8`, `+9`, min widths) not always named |

## Current findings

Re-audit of `client/crates/rqt` (2026-03-27). Seed items still present; additional repeats below. Scope = shared/repeated only.

| Area | Paths | Symptom |
|------|-------|---------|
| Layout defaults | `widgets.rs` `QLayout::new` (~145), `form_*_spacing` (~173/181), `load_widget` nested margins (~6149), groupbox top pad (~6154), `load_layout` root/spacing (~6242/6247) | Literals `6.0` / `9.0` / `11.0` with no named source; `QTOOLBOX_SPACING` also `6.0` separately |
| Tab bar chrome | `widgets.rs` `tab_bar_height()` (~2229) vs `style.rs` `QPixelMetric::TabBarHeight` (~764) vs `paint/tabs.rs` (already `pixel_metric`) | Helper hardcodes `28.0` while style + paint already agree |
| Scrollbar chrome | `scroll.rs` `QSCROLLBAR_EXTENT`/`SLIDER_MIN` (~3–4) vs `style.rs` `ScrollBarExtent`/`ScrollBarSliderMin` (~760–761) | Same literals twice; callers correctly use domain consts |
| Slider chrome | `slider.rs` `QSLIDER_THICKNESS` (~6) vs `style.rs` `SliderThickness`/`SliderLength` (~762) | Same `15.0` twice (domain already SoT for layout) |
| Checkbox/radio text inset | `paint/mod.rs` (~1152), `paint/focus.rs` (~49); test mirror in `paint/mod.rs` (~1620) | `font.pixel_size + 6.0` — `6.0` matches `ButtonMargin` |
| Size-hint v-pad | `widgets.rs` `size_hint_from_props` LineEdit (~672), ProgressBar (~734), fallback (~757) | Shared `font_height + 8.0` unnamed |

**Left alone (out of scope / not duplicates):** one-off paint nudges; QTABLE_* vs QVIEW_* parallel domain sizes; pushbutton `+9` / combo `+8.7` / spin `25` probe-only hints; MDI `INSET_Y = 28` (cascade inset, not tab bar); unused `MenuItemHeight` definition.

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/style.rs` | Extend `QPixelMetric` / `pixel_metric` if audit needs it |
| `client/crates/rqt/src/widgets.rs` | Layout defaults, size hints, drop duplicate helpers |
| `client/crates/rqt/src/scroll.rs` and other domain modules | Dedup vs style / shared consts |
| paint / app callers | Only where they hardcode the same chrome/view metrics |

Exact list comes from **Current findings**, not this table alone.

## Tests / acceptance

- [x] Re-audit section filled with current paths before merge of the cleanup
- [x] No intentional behavior/visual change vs prior Fusion defaults (same numbers, better sources)
- [x] Call sites for consolidated metrics use const / `pixel_metric`, not raw duplicates
- [x] `cargo test` (workspace or scoped `rqt` + affected crates)
- [x] Quick gallery smoke if layout defaults moved (numbers unchanged; no visual delta expected)

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/43-layout-metrics-dedup.md` end-to-end.
> **First step is mandatory:** re-scan the current tree and rewrite **Current findings** before changing code. Do not treat **Seed findings** as complete.
> Follow the metrics convention in `client/AGENTS.md`. Do not create a mega constants module. Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Do not edit unrelated agent-plans. Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
