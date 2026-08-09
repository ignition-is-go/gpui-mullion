# Mullion GPUI parity audit

**Audit date:** 2026-08-09  
**Reference:** `../mullion` at `09a8b8cbe88521f5c975e42bc0d3104af5afa448` (`feat/activity-bar-hover-delay`; its only unreleased addition over `origin/main` is configurable hover intent)  
**Port:** `gpui-mullion` at `e6825b405154c5f369b9ccaf5a8b66c77e2707d1`

## Verdict

`gpui-mullion` is **not feature complete** and is not yet a behavioral replacement for the reference Mullion. It is a strong portable tree/model foundation with a basic GPUI renderer.

The persisted pane-tree representation and low-level layout algorithms have near-exact parity. The largest gaps are in the product surface around that model: stateful activity integration, activity-bar behavior, proportional resizing, five-zone docking, activity-to-pane creation, the full command/keymap surface, configurable focus behavior, host chrome slots, theming, overlays, and tested rendered interaction.

The reference implementation is roughly 9,000 lines of library UI and interaction code in addition to its tree model. The GPUI view is currently one 505-line renderer. This is not a criticism of GPUI; it describes the amount of behavior still to port.

## Scope and parity rules

Parity means preserving the reference behavior and durable data contracts, not translating Leptos or DOM mechanisms literally.

- DOM-only APIs such as `HtmlElement` and `DomRect` need GPUI-native equivalents only where a host needs the behavior.
- CSS class names and HTML event plumbing are implementation details.
- Icons, host slots, focus policy, docking semantics, commands, event semantics, content lifecycle, and persistence behavior are product contracts.
- Native detached windows are an additive GPUI capability, not reference parity. They must not be counted as complete pane detachment until they render and synchronize a real pane.
- Exact visual pixel matching is not required, but every theme/behavior control used by consumers needs a GPUI equivalent or an explicitly documented migration.

## Status summary

| Area | Status | Summary |
|---|---:|---|
| Stable string IDs and `PaneData` bounds | Complete | Same string newtypes, serde shapes, and generic data contract. |
| `PaneNode` JSON and tree operations | Complete | The implementation and 30 tree tests are effectively shared with the reference. |
| Layout/geometry algorithms | Complete | Split, close, move, swap, rotate, balance, standard layouts, navigation geometry, and ratio bounds exist. |
| Headless model commands | Mostly complete | All command variants exist, but some result, focus, ID, and event semantics differ. |
| Activity body rendering | Partial | Real `AnyElement` content can render, but the API lacks the reference's reactive per-pane data and header/lifecycle contract. |
| Activity definitions and categories | Partial | Filtering and recursive registration exist; categories are flattened and their chrome is discarded. |
| Activity bar | Missing beyond baseline | Fixed left 42px text rail; no icons, nested category UI, edge policy, trailing group, hover intent, hide/auto-hide, or host slots. |
| Split resizing | Missing | Separators change by fixed `0.04` left/right clicks; no proportional pointer drag. |
| Pane docking | Partial | Whole pane is a drag source and every drop uses `Center`; no spatial zones or feedback. |
| Activity-to-new-pane drag | Missing | No pane factory, payload, `ActivityDropped`, or copy drop flow. |
| Focus/zoom | Partial | Core state/navigation work; focus acquisition and presentation are hard-coded and differ from the reference. |
| Full command/action/keymap | Partial | Model enum is present; built-in GPUI actions expose only nine operations. |
| Command palette/catalog | Missing | No names, descriptions, groups, dynamic focus submenu, or adapter. |
| Workspaces | Partial | Switching and outgoing-tree persistence work; mounted add/remove/rename and independent view state do not. |
| Theming and chrome extensibility | Partial | Eight colors only; geometry, typography, icons, headers, accessories, and overlay styling are fixed or absent. |
| Overlay escape hatch | Missing | Activity content is clipped with no Mullion-level modal/toast/drag overlay. |
| Detached pane windows | Scaffold only | Capability callback opens a host window, but does not detach, render, synchronize, or reattach a real pane. |
| Rendered interaction tests | Missing | All 42 tests target tree/model/serde/capabilities; `src/view.rs` has no tests. |

## What already has strong parity

### Persisted representation and layout algorithms

The reference and GPUI crates use the same:

- `PaneId`, `ActivityId`, and `CategoryId` string newtypes;
- `PaneData` bounds;
- `SplitDirection`, `PaneDirection`, `PaneLayout`, and `PaneRotation`;
- `PaneNode::Leaf { id, active_activity, data }`;
- `PaneNode::Split { direction, ratio, first, second }`;
- tree split/close/move/insert/swap/rotate/balance/layout operations;
- geometry-based neighbor and resize-boundary calculations;
- stable split keys and ratio clamping.

Evidence: reference `src/tree.rs:3-1686`; GPUI `src/tree.rs:3-1684`. The source differences are essentially visibility/documentation plus GPUI serde support for `DropEdge`. The GPUI golden fixture in `tests/model_compatibility.rs:8-35` confirms the nested JSON shape.

### Useful GPUI additions

The port adds several sound foundations:

- `MullionModel` isolates toolkit-independent mutations (`src/model.rs:7-396`).
- `MullionView::update_model` forwards queued events and schedules repaint (`src/view.rs:130-155`).
- Focus and zoom changes are explicit events.
- `WorkspaceSet` persists the outgoing tree during view-owned switching (`src/view.rs:101-124`).
- Detached-window availability is an honest host service boundary (`src/platform.rs:6-95`).

These additions should be retained while closing the gaps below.

## P0: compatibility and production-content blockers

### MUL-P0-001 — Define and test the stateful activity-content contract

**Reference behavior:** Each activity has an icon, filtered body renderer, and optional custom header renderer. Body/header receive `(PaneId, Signal<D>)`, so a pane's content reacts to that pane's data and remains mounted across unrelated topology changes (`../mullion/src/activity.rs:88-160`; `../mullion/src/components/pane_view.rs:55-183`; `../mullion/src/components/pane_content.rs:7-83`).

**Current GPUI behavior:** `ActivityRenderer<D>` is `Arc<dyn Fn(&PaneId, &D) -> AnyElement + Send + Sync>` and `Activity` has no icon/header (`src/activity.rs:5-14`). It is called during view rendering with a borrowed snapshot (`src/view.rs:248-307`). Hosts can return real GPUI elements and can pre-arrange captured state, but there is no explicit per-pane entity factory, `Window`/`App` context, update subscription, stable lifecycle contract, or custom header hook.

**Risk:** Simple stateless elements work, as the demo proves. It is not yet demonstrated or documented how Rship should mount one durable stateful GPUI entity per `(pane, activity)`, update it when only that pane's data changes, dispose it, and preserve it through move/resize/layout operations.

**Acceptance:**

- Provide a GPUI-native activity factory/lifecycle API for durable per-pane content.
- Support optional custom header content.
- Define data-update and disposal semantics.
- Preserve content entity identity across resize, move, balance, rotate, and workspace persistence where the pane survives.
- Add tests with a stateful entity, not only a stateless `div()`.

### MUL-P0-002 — Freeze command compatibility

`PaneCommand` variants and serde match, but stable behavior does not fully match:

- GPUI layout IDs use debug lowercase (`evenhorizontal`, `mainhorizontal`) rather than the reference kebab slugs (`even-horizontal`, `main-horizontal`) (`src/command.rs:29-50`; reference `src/commands.rs:91-117,236-254`).
- GPUI collapses most failures to `InvalidOperation`; `NoNeighbor` is declared but not returned (`src/command.rs:55-69`; `src/model.rs:305-394`).
- Reference errors distinguish split unavailable/refused, bad index, no neighbor, and not applicable (`../mullion/src/commands.rs:258-453`).
- Closing the middle of `[a,b,c]` focuses `c` in the reference but `a` in GPUI (`../mullion/src/context.rs:536-555`; `src/model.rs:154-176`).

**Acceptance:** Golden-test every command ID and serde shape; run command-trace parity tests for tree, focus, errors, and events; either match the reference or document and version every intentional break.

### MUL-P0-003 — Freeze event-stream compatibility

GPUI omits `ActivityDropped`, adds `DataChanged`, `FocusChanged`, and `ZoomChanged`, and changes some ordering/content (`src/events.rs:6-49`; reference `src/events.rs:3-50`). Examples:

- GPUI split emits `Split`, `FocusChanged`, `TreeChanged`; the reference persistence stream emits `Split`, `TreeChanged`.
- Reference balance emits one `Resized` per split then `TreeChanged`; GPUI emits only `TreeChanged` (`../mullion/src/context.rs:677-694`; `src/model.rs:238-244`).
- Reference upstream `set_tree` deliberately does not echo `TreeChanged`; GPUI `replace_tree` does, which can create a host feedback loop (`../mullion/src/context.rs:884-897`; `src/model.rs:40-79`).

**Acceptance:** Specify which events are persistence events versus transient view events; restore `ActivityDropped`; cover exact event traces; provide a non-echoing upstream replacement path.

### MUL-P0-004 — Validate persisted layout invariants at trust boundaries

`PaneNode` can deserialize duplicate pane IDs, non-finite ratios, and out-of-range ratios. `MullionModel::new` accepts the tree without validation (`src/tree.rs:118-134`; `src/model.rs:15-24`). Duplicate IDs make focus, split keys, docking, and lookups ambiguous.

**Acceptance:** Add `validate`/`try_new` (and an explicit normalization policy if desired) covering unique pane IDs, finite/clamped ratios, and a nonempty valid tree. Add hostile JSON fixtures and Unicode/special-character IDs.

## P1: core user-visible parity

### MUL-P1-001 — Implement proportional split-handle dragging

Reference handles track pointer delta against the parent dimension, update continuously, clamp to `0.1..=0.9`, lock the cursor, and commit the final exact ratio (`../mullion/src/components/split_handle.rs:80-192`).

GPUI separators currently map left click to `ratio + 0.04` and right click to `ratio - 0.04` (`src/view.rs:201-225`).

**Acceptance:** Pointer down/move/up with capture, continuous visual feedback, exact final `Resized`, correct nested geometry, clamp behavior, cancellation, and keyboard accessibility. Keep click stepping only if intentionally offered as an additional action.

### MUL-P1-002 — Implement five-zone pane docking with feedback

Reference docking resolves left/right/top/bottom 25% zones plus center, paints a target overlay, distinguishes move/copy, and rejects self-drop (`../mullion/src/components/drop_overlay.rs:35-164`).

GPUI makes the whole pane a drag source and every target drop calls `move_pane(..., DropEdge::Center)` (`src/view.rs:337-363`). `Center` becomes a horizontal insertion after the destination (`src/tree.rs:82-101,345-388`). `MullionTheme::drop_target` is unused.

**Acceptance:** Use an explicit move handle; do not hijack interactive activity content. Calculate all five zones from real bounds, render feedback, preserve source content, handle cancellation/self-drop, and test every edge on nested trees.

### MUL-P1-003 — Implement activity-to-new-pane docking

Reference drag payloads distinguish existing panes from new activities. A host pane factory mints the ID/data, all five edges work, the new pane selects the activity and gains focus, and `ActivityDropped` precedes `TreeChanged` (`../mullion/src/drag.rs:1-56`; `../mullion/src/context.rs:742-796`).

There is no GPUI equivalent.

**Acceptance:** Add the host factory, activity drag affordance, copy cursor/feedback, refusal behavior, event, focus behavior, and single-pane coverage.

### MUL-P1-004 — Port the activity-bar information architecture

Reference capabilities include:

- arbitrarily nested categories with icons and inherited colors;
- expanded/collapsed category UI and active ancestor opening;
- CSS/SVG/URL icon semantics through a GPUI-native icon abstraction;
- primary and trailing activity groups;
- left/right/top/bottom placement;
- hover labels and configurable open-only hover intent;
- per-pane pinned, hidden, or auto-hidden policy;
- app icon/move handle, split/close controls, accessories, and leading/trailing host slots.

Evidence: `../mullion/src/activity.rs:6-184`, `../mullion/src/components/activity_bar.rs:35-1532`, and provider props at `../mullion/src/components/mullion_root.rs:54-126`.

GPUI recursively flattens categories, discarding category ID/name/color, then renders the full activity name in a fixed `30x30` item in a fixed left `42px` rail (`src/activity.rs:16-42`; `src/view.rs:174-179,248-297,364-377`).

**Acceptance:** Port the behavior using GPUI-native elements; do not preserve DOM/CSS implementation details. Add nested/filter/active-path/edge/hide/auto-hide/intent tests.

### MUL-P1-005 — Expose the entire command set through GPUI actions

The model contains focus, split, close, move, swap, resize, orientation, balance, rotation, layouts, and zoom (`src/command.rs:5-26`). The view registers only focus/cycle, close, zoom, and balance actions/bindings (`src/view.rs:11-24,439-504`). Its internal action path always refuses splits because it supplies no factory (`src/view.rs:171-173`).

**Acceptance:** Provide actions for the full static catalog, host-configurable split factory and resize step, stable metadata, customizable direct/prefix keymaps, exact modifier semantics, editable-content policy, and rebinding/serde tests. A command-palette adapter may remain optional, but the catalog cannot be duplicated by every host.

### MUL-P1-006 — Port focus policy and focus presentation

Reference focus supports configurable `Hover` or `Click`, host-controlled settings, optional internal-edge focus frame, optional unfocused-pane wash, and correct interaction while zoomed (`../mullion/src/focus.rs:1-29`; `../mullion/src/settings.rs:130-261`; `../mullion/src/components/pane_view.rs:185-319`).

GPUI always focuses on left mouse down and always paints a pane border (`src/view.rs:337-353`).

**Acceptance:** Add a serializable/configurable focus policy, programmatic pane focus, opt-in presentation, inactive treatment, and tests for pointer, keyboard, zoom, close, and workspace transitions.

### MUL-P1-007 — Make workspace management complete and define view-state scope

Reference `WorkspaceManager` supports list/switch/add/remove/update/rename (`../mullion/src/workspace.rs:19-105`). GPUI `WorkspaceSet` only provides active/switch/persist, and a mounted `MullionView` exposes it immutably (`src/workspace.rs:14-35`; `src/view.rs:96-124`).

GPUI improves parity by saving the outgoing tree automatically, but focus and zoom live outside each serialized workspace and can leak when IDs overlap.

**Acceptance:** Mounted add/remove/rename/reorder APIs, deterministic invalid-switch behavior, per-workspace focus/zoom policy, complete serde fixtures, and event ordering tests.

### MUL-P1-008 — Add rendered GPUI interaction tests

The GPUI suite has 42 passing tests: 30 copied tree tests, seven model tests, two platform tests, one workspace test, and two compatibility tests. `src/view.rs` has no tests. Current native/WASM CI mainly proves compilation; the browser job builds the bundle but does not execute Mullion interactions.

**Acceptance:** GPUI test-context coverage for activity content, activity selection, focus, close-last behavior, commands, workspaces, drag/drop, and resizing; image/snapshot coverage where stable; real browser runtime smoke; at least one Linux runtime smoke. Each UI feature above must land with behavior tests.

## P2: integration, styling, and polish parity

### MUL-P2-001 — Port host chrome slots and header extensibility

Reference provider hooks include custom activity header content, app icon, pane accessory, bottom leading/trailing content, per-pane border color, header visibility, and activity-bar policy (`../mullion/src/components/mullion_root.rs:54-126`). GPUI exposes only global header visibility and a fixed name/close header (`src/view.rs:78-92,308-336`).

### MUL-P2-002 — Expand theming beyond eight colors

GPUI hard-codes rail width, handle thickness, header height, tab dimensions, borders, radii, typography, and spacing throughout `src/view.rs`. The reference exposes component styles for the root, pane, activity bar, split handle, drop overlay, header, and workspace switcher.

Provide GPUI-native theme structs/tokens for geometry and state styling, including light/system theme support and the currently unused `drop_target` color.

### MUL-P2-003 — Provide an overlay escape hatch

Activity content is placed under `overflow_hidden` (`src/view.rs:378-387`). Reference `MullionOverlay` provides modal/toast/drag tiers, optional backdrop, centering, outside-click, and click-through in a body-level portal (`../mullion/src/components/overlay.rs:58-195`).

Provide a GPUI-native window-level overlay service/slot so host dialogs and palettes are not clipped by a pane.

### MUL-P2-004 — Add command-palette and accessibility adapters

Reference offers an optional command-palette registration adapter and live focus submenu (`../mullion/src/command_palette.rs:14-82`). It also labels/click-enables activity controls and supports Enter/Space selection.

Expose command metadata independent of any palette implementation, then add the adapter needed by Rship. Give icon-only controls accessible labels, keyboard activation, visible focus, and sensible pointer targets using GPUI semantics.

### MUL-P2-005 — Make detached windows a real pane feature or label them as scaffold

`NativeDetachedWindowService` receives a `PaneId` and host `App`, but it is not connected to the view/model (`src/platform.rs:44-95`). The example opens a separate text-only `DetachedPane`, not the pane's activity/data (`examples/detached_window.rs:12-47`).

Do not describe this as pane detachment until it defines ownership transfer, activity entity lifecycle, synchronized data/events, close/re-attach behavior, and workspace persistence. Until then, call it a host window-opening example.

## Required regression matrix

Before claiming parity, tests should cover at least:

1. Golden JSON for every serializable enum/variant, nested vertical/horizontal trees, all workspace shapes, command IDs, events, Unicode IDs, and legacy fixtures.
2. Shared command traces against the reference: output tree, focus, zoom, exact error, and exact event sequence.
3. Three-pane close successor, balance event stream, invalid index/neighbor, refused split, duplicate ID, and non-finite ratio cases.
4. Activity filtering/fallback, nested order, duplicate IDs, active ancestor path, custom header, per-pane update, stateful entity identity, and disposal.
5. Pointer resize start/move/end/cancel, min/max clamp, nested handle geometry, and exact persisted ratio.
6. Pane docking at all five zones, self-drop, cancellation, activity copy drop, factory refusal, and feedback state.
7. Full action/keymap catalog, customization, exact modifiers, editable-content handling, and serialization.
8. Focus hover/click policy, keyboard focus, zoom navigation, close repair, and workspace switching.
9. Workspace add/remove/rename/switch/persist/invalid-active behavior and view-state policy.
10. Native Windows/macOS/Linux builds, Linux Wayland runtime, WASM build, and executed browser-runtime interaction smoke.

## Recommended implementation order

1. Freeze command/event/focus compatibility and add validation/golden traces.
2. Design the stateful GPUI activity/header lifecycle; prove it with real host entities.
3. Implement proportional split dragging.
4. Implement explicit-handle five-zone pane docking.
5. Add activity-to-new-pane docking and `ActivityDropped`.
6. Port activity hierarchy/icons/trailing groups and bar edge/hide/auto-hide behavior.
7. Expose the full action/keymap/catalog surface.
8. Port focus settings/presentation and workspace management.
9. Add host slots, theme tokens, overlay service, accessibility, and palette adapter.
10. Upgrade the detached-window scaffold only after single-window pane parity is complete.

## Parity completion definition

The GPUI port can be called feature complete only when:

- every reference user-visible feature is **implemented**, **explicitly replaced by a documented GPUI-native equivalent**, or **accepted as a named non-goal**;
- persisted layouts and configured commands migrate through golden fixtures;
- command, focus, event, activity, resize, docking, and workspace semantics have automated parity tests;
- real stateful Rship activity content survives layout changes;
- Windows, macOS, Linux/Wayland, and browser/WASM run the same tested view;
- the README no longer labels known missing behavior as merely “next” while describing the product surface as implemented.
