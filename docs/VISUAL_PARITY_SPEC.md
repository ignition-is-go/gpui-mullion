# Mullion visual parity specification

**Status:** normative rebuild specification; parity is **not complete**.
**Reference revision:** Leptos Mullion `09a8b8cbe88521f5c975e42bc0d3104af5afa448` (the linked local sources at that revision are authoritative).
**GPUI revision audited:** `68f2fde6f71490f5c4bc306a9c5d464951431419`.
**Related audit:** [PARITY_AUDIT.md](PARITY_AUDIT.md).

> This document specifies a pixel rebuild, not an API-equivalence exercise. Passing model, event, accessibility, or rendered interaction tests does not establish visual parity. No item below may be marked complete without reference and GPUI goldens at the prescribed states.

## 1. Acceptance contract

The GPUI native and GPUI/WASM surfaces shall reproduce the Leptos reference **pixel for pixel** at the capture conditions in §9. Acceptance means:

1. identical painted bounds, clipping, stacking, colors (including alpha compositing), typography, icon artwork, and state;
2. identical geometry at rest and at every prescribed transition sample;
3. an exact RGBA diff (`0` differing pixels, maximum channel delta `0`) for deterministic DPR-1 browser goldens after both sides use the same checked-in font files and raster backend;
4. exact logical geometry/token assertions plus a zero-diff capture on the pinned native CI renderer. If a platform font or GPU makes zero-diff impossible, the exception must name the platform, field, measured cause, and narrow per-pixel mask. A blanket percentage threshold is forbidden;
5. no antialiasing, font, browser-default, or “GPUI-native” waiver unless documented as the preceding narrow exception.

A screenshot that merely looks close is a failure. This specification makes **no completion claim**. Closing the umbrella requires all matrix cells, mismatches, and linked Levi tasks to be resolved with committed artifacts.

## 2. Compared source surface and authority

Every Leptos visual component and the two requested data/theme sources was compared against current GPUI:

| Reference source | Normative responsibility | Current GPUI counterpart |
|---|---|---|
| [`components/mullion_root.rs`](../../mullion/src/components/mullion_root.rs) | provider CSS, 100% root | [`view.rs`](../src/view.rs), [`styles.rs`](../src/styles.rs) |
| [`components/pane_view.rs`](../../mullion/src/components/pane_view.rs) | flat leaf geometry, pane axis/border, focus/wash, hidden controls | `view.rs` |
| [`components/pane_content.rs`](../../mullion/src/components/pane_content.rs) | header/body fill, clipping and stacking isolation | `view.rs` |
| [`components/pane_header.rs`](../../mullion/src/components/pane_header.rs) | header band | `view.rs`, `styles.rs` |
| [`components/activity_bar.rs`](../../mullion/src/components/activity_bar.rs) | every rail row/category/control/edge/transition | [`activity_bar.rs`](../src/activity_bar.rs), `view.rs`, `styles.rs` |
| [`components/split_handle.rs`](../../mullion/src/components/split_handle.rs) | separator paint and hit target | `view.rs`, `styles.rs` |
| [`components/drop_overlay.rs`](../../mullion/src/components/drop_overlay.rs) | five-zone drag target and indicator | [`drag.rs`](../src/drag.rs), `view.rs`, `styles.rs` |
| [`components/workspace_switcher.rs`](../../mullion/src/components/workspace_switcher.rs) | workspace buttons | [`workspace.rs`](../src/workspace.rs), `view.rs`, `styles.rs` |
| [`components/overlay.rs`](../../mullion/src/components/overlay.rs) | viewport escape layer, backdrop and tiers | [`overlay.rs`](../src/overlay.rs), `view.rs` |
| [`components/mod.rs`](../../mullion/src/components/mod.rs) | exported component set | [`lib.rs`](../src/lib.rs) |
| [`theme.rs`](../../mullion/src/theme.rs) | canonical default palette | [`theme.rs`](../src/theme.rs) |
| [`activity.rs`](../../mullion/src/activity.rs) | ordered recursive nodes, icon variants, inherited category color | [`activity.rs`](../src/activity.rs), [`activity_catalog.rs`](../src/activity_catalog.rs) |

Behavior/model compatibility is outside this visual oracle except where it selects a painted state.

## 3. Canonical colors and box model

### 3.1 Reference default palette

These are literal Leptos defaults, and override the currently divergent GPUI defaults:

| Token | Exact reference value | Current GPUI dark | Disposition |
|---|---:|---:|---|
| root `--ml-bg` | `#0e0e0e` | `#0e0e0e` | match |
| surface `--ml-surface` | `#111111` | `#151515` | mismatch |
| border `--ml-border` | `#1a1a1a` | `#303030` | mismatch |
| accent `--ml-accent` | `#222222` | `#242424` | mismatch |
| text `--ml-text` | `#eeeeee` | `#eeeeee` | match |
| muted `--ml-text-muted` | `#888888` | `#909090` | mismatch |
| highlight `--ml-highlight` | `#333333` | GPUI `focused=#62a0ea` | mismatch/semantic conflation |
| drop indicator | `rgba(255,255,255,0.06)` | `#355070` | mismatch |
| open category card | `rgba(255,255,255,0.045)` | `rgba(255,255,255,0.043)` (`0x0b`) | rounding mismatch |
| category leading edge | `rgba(255,255,255,0.08)` | `rgba(255,255,255,0.078)` (`0x14`) | rounding mismatch |
| overlay scrim fallback | `rgba(0,0,0,0.5)` | policy-dependent | must match fixture |
| focus indicator | `var(--ml-focus-color,color-mix(in srgb,var(--ml-primary,#00a4ef) 65%,var(--ml-border,#1a1a1a)))` | pane border color | mismatch |
| focused grabber | `var(--ml-focused-grabber-color,var(--ml-focus-color,color-mix(in srgb,var(--ml-primary,#00a4ef) 65%,var(--ml-border,#1a1a1a))))` | normal theme text | mismatch |
| unfocused wash | `var(--ml-unfocused-pane-color,var(--ml-bg,#0e0e0e))` | whole-pane opacity | mismatch |

Light/system palettes are GPUI extensions, not the reference default. They require their own GPUI stability goldens but cannot substitute for the dark reference comparison.

### 3.2 Box-model rule and ambiguity protocol

The reference component stylesheet does **not** globally set `box-sizing`. Only the horizontal activity panel and a pane with a host bottom border explicitly use `border-box`; ordinary rail panels, buttons, headers, and workspace buttons otherwise inherit the host/default (`content-box`). Therefore a declared `28px` header plus a `1px` bottom border can occupy 29 physical pixels, and a `28px` vertical panel plus an inner border can occupy 29. GPUI currently uses its own layout/border semantics and also paints a 1px border around every pane, which the reference does not.

Golden pages shall pin `html, body { margin:0 }`, the exact font, and `box-sizing:content-box` on the Mullion fixture unless a reference rule explicitly changes it. They shall not use a global `* { box-sizing:border-box }`. For each ambiguous edge, record both declared style and `getBoundingClientRect()`/computed content, padding, border, and outer sizes in golden metadata. The browser's computed reference bounds, not an inferred CSS arithmetic model, decide parity. Fractional split boundaries use browser layout output at DPR 1; captures must not silently round each child independently.

## 4. Layering and clipping contract

Bottom to top within a leaf:

1. pane surface and activity body; `PaneContent` is `position:absolute; inset:0`, clipped, and `isolation:isolate`, so arbitrary activity z-indices cannot escape;
2. unfocused wash: absolute inset `0`, z `4`, pointer transparent;
3. split handles: z `5` in the flat reference pane tree (the visible line is 4px inside an 8px centered target);
4. focus frame: z `6`, pointer transparent;
5. auto-hide edge target: z `9`;
6. activity panel: z `10`;
7. hidden-pane capsule: z `16`;
8. drop event plane/indicator: z `20`;
9. body-level overlay root: fixed inset `0`, `z-index:var(--ml-overlay-z,10000)`, pointer transparent. Its fixed wrappers are Modal `10`, Toast `20`, Drag `30` within that root.

A pane and every leaf slot clip overflow. Auto-hidden rails therefore cannot bleed into adjacent panes. GPUI must create equivalent stacking/clipping boundaries rather than relying on child order alone. Overlay content is the only supported escape from pane clipping.

## 5. Activity bar: exact state machine and paint

### 5.1 Base anatomy and rows

The scope reserves `28px` on the configured edge when pinned. Its absolute panel is surface `#111111`, radius `0`, inner-edge `1px solid #1a1a1a`, z `10`, scrollbar hidden, and splits into primary and trailing groups with `space-between` (horizontal trailing group uses `margin-left:auto`). The vertical panel is `28px` wide and full height. Each row is exactly `28px` high, width `100%`, padding/border/background none, 11px text, nowrap, pointer cursor, text color `#eeeeee`, inactive opacity `0.5`. Icon slot is fixed 28px, icon artwork box 14×14, centered, clipped; SVG stroke is current text. Labels are always in layout but `opacity:0`, `min-width:0`, overflow/ellipsis.

Render order is invariant:

- primary: optional app icon/pane grabber, then filtered primary node tree in registration order;
- trailing: `bottom_leading`, filtered trailing tree, `bottom_trailing`, pane accessory, Split H, Split V, Close;
- the app-icon grabber has empty label, grab cursor, and on focused panes the exact `--ml-focused-grabber-color` fallback chain from §3.1, opacity 1, color/opacity transition `125ms ease-out`;
- controls use the reference 13×13 SVG paths, not Unicode approximations. Labels are exactly `Split H`, `Split V`, `Close`. There is no separate visible Move text row in the pinned rail.

A top-level activity is active with opacity `1` and foreground `var(--ab-float-active-color,#eeeeee)`. A categorized activity inherits the nearest category color. Reference active rows do **not** acquire an accent background; GPUI's current active/hover `theme.accent` fill is a mismatch.

`ActivityIcon` variants are exact: `Class` produces an `<i class=...>`, `Svg` injects the supplied markup inside the 14px clipped box, and `Url` produces an image constrained to 100% width/height with object-fit containment. Golden fixtures use checked-in SVGs so all implementations raster the same paths.

### 5.2 Categories and “flyout” expansion

The reference “flyout” is the expanded absolute panel plus inline recursive category rows; it is not a detached popup. Clicking (or Enter/Space on activities) toggles by stable category ID. All ancestors of the selected activity open automatically. Categories whose filtered subtree is empty do not render.

Closed category:

- plain 28px row; semibold (`600`); category label color `#888888`;
- opacity 1 when it contains the active descendant, otherwise 0.5;
- if it contains the active descendant, show a 4×4 circular dot at x=2, vertically centered (right x=2 on a right rail; bottom-center on top rail; top-center on bottom rail), in category color;
- card fill and 1px leading card edge remain geometrically present but transparent.

Open category:

- same row geometry (no shift); opacity 1;
- wrapper fill `rgba(255,255,255,0.045)` and `1px` edge `rgba(255,255,255,0.08)`; vertical edge is on top, horizontal edge is on left;
- top-level left wrapper bleeds right by `-8px`, right wrapper bleeds left by `-8px`; nested wrappers do not add bleed;
- children are inline recursively, with a 2px category-colored stripe on the pane-edge side (top/bottom for horizontal). Translucent nested card fills composite, so depth changes actual pixels;
- chevron is one `▸`, fixed 14px centered slot, 9px size, line-height 1, opacity 0.5. Closed is unrotated; open vertical rotates 90°; open horizontal rotates 180°. It appears/hides with labels.

No ad-hoc 5px depth margin, permanent category left border, square category background, or missing label/chevron is acceptable. Those describe the current GPUI output, not the reference.

### 5.3 Hover, drag, edge, and autohide transitions

All dimensions below are exact CSS transition endpoints:

| State | Vertical | Horizontal |
|---|---|---|
| pinned compact | panel 28px; labels 0 | toolbar height 28px; every item 28px |
| pinned hover-expand | panel width 150px plus 8px padding on content-edge; labels 1 | only hovered item width 150px; its label 1 |
| `hover_expand=false` | remains compact (`collapsed`) | remains compact |
| dragging | panel width 150px, all labels 1, transitions disabled | panel stays full width; all rows 150px, labels 1, transitions disabled |
| autohide closed | scope reserves 0; panel translate −100% left / +100% right | scope height 0; translate −100% top / +100% bottom |
| autohide target | invisible 12px full-length edge strip, z 9 | invisible 12px full-width edge strip, z 9 |
| autohide open | translated to 0 over content | translated to 0; hovered item expands without a second intent delay |

The default transitions are 150ms ease for width, edge padding, transform, and label opacity; GPUI exposes this as `transition_duration_ms`. Intent delay (`expand_delay_ms`) applies only on entry; leave cancels pending intent immediately and animates closed. Capture at start, configured midpoint, endpoint, and settled. For the canonical delayed fixture (175ms delay, 150ms duration), capture 0, 174, 175, 250, and 325ms. Native drag holds the bar open even after pointer hover is lost; drag end restores the applicable hover/autohide state. When activity content exceeds the available rail length, the rail scrolls on its primary axis without resizing pane content.

Left/right/top/bottom mirror border, padding, dot, stripe, transform, and source/trailing order exactly. A bottom bar follows content in flex order and uses a top border; a right bar uses left border/padding and its category markers on the right.

## 6. Root, pane, header, split, drop, focus, workspace, overlay

### 6.1 Root, pane tree, zoom, header, content

The root and pane-tree container are 100%×100%, with root `#0e0e0e`. The reference tree is flat absolute leaf rectangles expressed as percentages; there is no layout thickness deducted for split handles. Split handles overlay the boundary. A zoomed leaf is absolute inset 0, z 1; other mounted leaves remain inset 0 but `visibility:hidden; pointer-events:none`, preserving activity state.

A leaf is a relative, overflow-hidden flex row (column for top/bottom bars), surface `#111111`, text `#eeeeee`. It has **no default all-side pane border**. A host `pane_border_color` adds only a 2px bottom border with border-box sizing.

Content beside the bar is `flex:1 1 0`, min width/height 0, relative, clipped. Its isolated absolute column fills it. Header exists only when headers are enabled and an activity resolves: declared height/min-height 28px, flex-shrink 0, 0 8px padding, 8px gap, surface background, text 11px, bottom border 1px, nowrap/clipped. Title is 600 weight and ellipsized; custom header content follows, flex centered, gap 8, min-width 0 and clipped. Body fills the remainder and clips. Empty activity sets show no header and an empty body (not a centered `No activity` legend).

### 6.2 Split handles

At every internal boundary paint a centered 4px `#1a1a1a` line inside an 8px pointer target. Horizontal tree split means vertical line/col-resize; vertical tree split means horizontal line/row-resize. On hover **and throughout drag**, line color is `#333333`; release returns it. Target overlays rather than consumes layout. Ratio clamps `0.1..0.9`. The current GPUI nested flex separator consumes 4px and uses blue hover; both are mismatches.

### 6.3 Drop geometry

While a valid pane drag is active, every non-source pane gets an absolute inset-0 z-20 event plane; activity-copy drag also includes its source. Normalize pointer against the full target rectangle:

- `nx < .25` Left; `nx > .75` Right; else `ny < .25` Top; `ny > .75` Bottom; else Center (x precedence is intentional);
- Left/Right indicator covers exactly the corresponding 50% width and full height;
- Top/Bottom covers 50% height and full width;
- Center covers 100%;
- fill is `rgba(255,255,255,0.06)` and indicator transition is `all 0.1s ease`.

The currently rendered GPUI five 25%/50% accessibility zones may remain hit metadata, but they must not paint. Its visible indicator must use the above 50%/100% result, not blue, and must stack below overlay root but above pane chrome.

### 6.4 Focus and hidden-bar controls

With focus indicator disabled (default), no frame paints. Enabled focus uses only **internal** pane edges, 1px exact `--ml-focus-color` fallback chain from §3.1, and excludes the pinned bar strip from its inset. It fades opacity in/out over `100ms ease-out`. The outside root perimeter never receives a frame. Focus mechanics still work when the indicator is hidden.

Unfocused treatment is an inset wash above content at z 4, color root background and opacity `1 - clamp(pane_opacity,0,1)`; focused opacity is 0. It transitions `125ms ease-out`. It is not whole-pane opacity: rail chrome, hidden controls, drop feedback, and focus chrome retain their own alpha. GPUI's current `.opacity(unfocused_opacity)` on `pane-visual` is non-equivalent.

When the bar is hidden, only the focused pane shows the absolute management capsule: top/right 6px, z 16, flex gap 2px, padding 2px, radius 6px, surface background, 1px border, opacity 0.95. Unfocused: opacity 0, no hit testing; transition opacity `.12s`. Children in order: 22×22 `⠿` move (13px), exact 13×13 Split H SVG, Split V SVG, Close SVG; transparent backgrounds, 4px child radius, base opacity 0.75. The capsule must not be shifted by a GPUI-only pane border.

### 6.5 Workspace switcher

The switcher itself is a flex row with 4px gap and no implicit padding/background. Buttons have no border; content-box padding 4px vertical/12px horizontal, 3px radius, 12px font, pointer cursor. Inactive is accent `#222222` with muted `#888888`; active is highlight `#333333` with text `#eeeeee`. Reference defines no inactive-button hover fill. Tabs render in workspace order and switching changes only the selected styling/tree.

### 6.6 Overlay geometry and dismissal

The singleton root is appended after the Mullion root to `document.body`, fixed to the **viewport**, not merely the component bounds. An overlay wrapper is fixed inset 0 at tier 10/20/30 and defaults to pointer events auto; click-through changes wrapper to none and requires content to opt in. Optional backdrop is absolute inset 0, z 0; content layer is relative z 1, 100%×100%. `center=true` adds grid/place-items-center; otherwise caller content determines placement. Outside dismissal fires only when the event target is the content layer itself, never for descendant clicks. Multiple overlays preserve mount order within a tier and tier order globally.

GPUI's richer alignment/size/a11y policy is additive. The parity fixture must exercise the exact reference subset above. Rendering the overlay inside a clipped pane or below activity/drop chrome fails.

## 7. Stable selectors and required states

Reference DOM selectors/attributes to expose in the reference capture metadata:

- `.mullion-root`, `.mullion-pane`, `[data-mullion-focused="true|false"]`;
- `.mullion-ab[data-axis="vertical|horizontal"][data-side="leading|trailing"]` plus `.collapsed`, `.auto_hide`, `.dragging`;
- `.mullion-ab-panel`, `-group`, `-category`, `-children`, `-btn`, `-icon-slot`, `-icon`, `-label`, `-dot`, `-cat-border`;
- `.mullion-header`, `-title`, `-content`; `.msh.horizontal|vertical.dragging`, `.msh-bar`; `.mullion-drop`;
- `.mullion-ws`, `.mullion-ws-btn.active`; `[data-mullion-pane-grabber]`, `[data-mullion-unfocused-wash]`, `[data-mullion-focus-frame]`; `#mullion-overlay-root`.

GPUI debug selectors used for paired state control/crop metadata:

- `pane:<id>`, `pane-visual:<id>`, `pane-content:<id>`;
- `activity-bar:<id>`, `activity:<pane>:<activity>`, `activity-category:<pane>:<category>`;
- `pane-control:<move|split-h|split-v|close>:<id>`, `pane-controls:<id>`, `pane-drag-handle:<id>`;
- `split-container:<key>`, `split-handle:<key>`, `split-hit-target:<key>`;
- `dock-target:<id>:<edge>`, `dock-indicator:<id>:<edge>`, `focus-edge:<id>:<edge>`;
- `workspace:<id>`, `mullion-overlay:<id>`, `mullion-overlay-content:<id>`, `mullion-overlay-backdrop:<id>`.

Every selector must have a state driver that does not depend on scanning approximate coordinates. Golden metadata records selector bounds, computed colors, opacity, overflow, and state flags beside each PNG.

## 8. Canonical fixture geometry

The canonical content viewport is **1000×700 CSS pixels, DPR 1**, dark theme, default browser zoom, animations enabled only for transition frames. Use the same checked-in font at explicit normal/600 weights and disable caret/blink/time-dependent content. The fixture contains:

- workspace strip first, measured from its reference content-box result;
- below it, a 40/60 horizontal root split; the right side is a 50/50 vertical split;
- stable pane IDs `main`, `top-right`, `bottom-right`; ratios and all text fixed;
- primary tree: top-level activity, an outer category containing an activity and nested category/activity, and a filtered-empty category;
- trailing top-level activity, app icon, leading/trailing/accessory slots, enabled split/new-pane factories;
- headers with plain and custom content, a host 2px bottom pane border fixture, an empty-activity pane fixture, workspaces `One` and `Two`;
- modal, toast, and drag overlays with deterministic colored rectangles.

Additional browser viewports, all DPR 1: **1280×720** (CI/browser landscape), **800×600** (compact), and **390×844** (narrow portrait/overflow). The 1000×700 pair is the acceptance anchor; the others detect axis overflow, clipping, and responsive rounding. Native capture uses an exact 1000×700 **client area** (exclude OS decoration) and records scale factor; test 1× and 2×, comparing logical geometry at both and device pixels against same-scale reference.

## 9. Screenshot golden matrix

Capture both `reference/` and `gpui/{browser,native}/` using `{viewport}/{theme}/{edge}/{state}.png` plus JSON metadata. Minimum matrix:

| Dimension | Required values |
|---|---|
| viewport | 1000×700 all rows; 1280×720, 800×600, 390×844 for rest/expanded/autohide/drop |
| theme | reference dark; GPUI dark/light/system stability (only dark is cross-implementation oracle) |
| edge | Left, Right, Top, Bottom |
| bar mode | pinned compact, hover mid/settled, collapsed behavior, auto-hide closed/mid/open/leave, dragging |
| activity tree | top-level inactive/active; category closed/inactive; closed/active-dot; open; nested open; selected child; filtered empty |
| controls | app grabber unfocused/focused; each split/close hover; disabled close; hidden capsule absent/shown/hover |
| pane | single, three-pane canonical, zoomed, header plain/custom/hidden/empty, host bottom border |
| focus | indicator off/on for each canonical pane, wash 1.0/0.75/0.0, hover- and click-focus transitions |
| split | horizontal/vertical rest, hover, drag start/mid/clamped/end |
| drop | pane and activity payload × Left/Right/Top/Bottom/Center, self refusal, leave, drop |
| workspace | inactive/active/hover, switched tree |
| overlay | none, modal no backdrop, modal 50% scrim centered, click-through, modal+toast+drag tier stack, outside-dismiss before/after |

Transition PNG times are those in §5.3 plus focus 0/50/100ms, wash/grabber 0/62/125ms, hidden capsule 0/60/120ms, and drop indicator 0/50/100ms. Each intermediate capture uses a deterministic clock; `sleep`-and-hope captures are invalid.

Diff output must retain: raw reference, raw candidate, absolute RGBA diff, blink/heat map, differing-pixel count, max/mean channel delta, bounding rectangle of differences, selector geometry JSON, commit SHAs, browser/GPUI versions, OS, font hashes, DPR and color space. CI fails on a missing cell, stale SHA, unexpected mask, or nonzero unmasked pixel at the canonical deterministic target.

## 10. Current mismatch checklist and ownership

None of these boxes is checked by this document.

### [`lv-7514`](https://github.com/ignition-is-go/gpui-mullion/issues?q=lv-7514) — activity bar

- [ ] Replace GPUI dark activity colors with §3 reference colors.
- [ ] Implement real 150px vertical panel expansion and 8px edge padding; GPUI currently changes rail extent only for auto-hide.
- [ ] Render labels, ellipsis, horizontal per-row expansion, exact delays, immediate leave, and deterministic 150ms transitions.
- [ ] Rebuild category wrapper/card/edge/dot/stripe/chevron nesting; remove GPUI 5px depth indent/permanent left border.
- [ ] Remove active/hover accent background; apply nearest-category/floating foreground and exact opacity.
- [ ] Use exact reference SVG controls and icon variant sizing rather than `◫`, `⊟`, `×`/text fallbacks.
- [ ] Match primary/trailing grouping, slot positions, focused grabber, drag-held-open state, all four mirrors, and 12px autohide target (current GPUI closed extent is 3px).

### [`lv-6feb`](https://github.com/ignition-is-go/gpui-mullion/issues?q=lv-6feb) — chrome/layout

- [ ] Remove GPUI default all-side pane border and make split handles overlay rather than consume layout.
- [ ] Match header content-box bounds, clipping, empty-pane rendering, and isolated activity stacking.
- [ ] Match split line `#1a1a1a`, hover `#333333`, 4/8px geometry and drag state.
- [ ] Match drop color, exact 25% classification and 50%/100% indicator geometry/transition.
- [ ] Replace whole-pane opacity with z-4 wash; render focus in fallback blue, internal edges only, pinned-bar inset, exact transitions.
- [ ] Match hidden capsule z/order/opacity/artwork and workspace colors/no extra hover rule.
- [ ] Prove viewport-root overlay geometry, fixed layering and reference outside-click semantics.

### [`lv-23eb`](https://github.com/ignition-is-go/gpui-mullion/issues?q=lv-23eb) — shared fixture/demo

- [ ] Expose the deterministic canonical catalog, slots, categories, headers, factories, workspaces, focus policies, all bar modes/edges and overlay states on native and WASM.
- [ ] Add selector-addressable state controls; do not use coordinate scans as a golden driver.
- [ ] Ensure changing activity content and pane/activity drag results are visibly distinguishable and stable.

### [`lv-8546`](https://github.com/ignition-is-go/gpui-mullion/issues?q=lv-8546) — screenshot regression

- [ ] Check in reference/candidate images and metadata for every §9 cell.
- [ ] Pin viewport, DPR, fonts, renderer, clock and color space; enforce zero unmasked canonical diff.
- [ ] Produce reviewable diff artifacts and stale/missing-matrix CI failures on browser and native.

### [`lv-fab0`](https://github.com/ignition-is-go/gpui-mullion/issues?q=lv-fab0) — drag performance without visual compromise

- [ ] Instrument pane docking, activity docking, and split resize frame times at 1000×700 with the full canonical tree.
- [ ] Bound pointer-move work without dropping indicator/ratio transition frames or changing pixels.
- [ ] Avoid full catalog/activity/tree recomputation where unchanged; prove state and screenshots remain identical.
- [ ] Establish a recorded frame budget and CI regression gate; performance work is not permission to reduce golden coverage.

## 11. Completion gate

A parity completion statement is prohibited until all five linked tasks are resolved at commits containing their artifacts, every matrix cell passes, local Markdown links validate, and a reviewer has compared the canonical reference/candidate/diff triplets. This document deliberately records the current non-parity baseline and **does not claim completion**.
