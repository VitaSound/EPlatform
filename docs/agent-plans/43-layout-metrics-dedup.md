# 43 — Layout / style metrics: dedup + Qt-shaped sources

Status: ready
Priority: P2
Depends-on: []
Blocks: []
Estimate: M

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

_(Fill during re-audit; replace this stub.)_

- …

## Files to touch

| Path | Change |
|------|--------|
| `client/crates/rqt/src/style.rs` | Extend `QPixelMetric` / `pixel_metric` if audit needs it |
| `client/crates/rqt/src/widgets.rs` | Layout defaults, size hints, drop duplicate helpers |
| `client/crates/rqt/src/scroll.rs` and other domain modules | Dedup vs style / shared consts |
| paint / app callers | Only where they hardcode the same chrome/view metrics |

Exact list comes from **Current findings**, not this table alone.

## Tests / acceptance

- [ ] Re-audit section filled with current paths before merge of the cleanup
- [ ] No intentional behavior/visual change vs prior Fusion defaults (same numbers, better sources)
- [ ] Call sites for consolidated metrics use const / `pixel_metric`, not raw duplicates
- [ ] `cargo test` (workspace or scoped `rqt` + affected crates)
- [ ] Quick gallery smoke if layout defaults moved

## Agent handoff prompt (copy-paste)

> Implement the plan in `docs/agent-plans/43-layout-metrics-dedup.md` end-to-end.
> **First step is mandatory:** re-scan the current tree and rewrite **Current findings** before changing code. Do not treat **Seed findings** as complete.
> Follow the metrics convention in `client/AGENTS.md`. Do not create a mega constants module. Do not expand scope beyond **In**. Update **Status** to `done` when acceptance passes.
> Do not edit unrelated agent-plans. Follow `docs/qt-parity-guide.md` and `docs/wasm-canvas-geometry.md` invariants.
