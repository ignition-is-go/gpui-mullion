# Mullion GPUI parity audit

**Re-audit date:** 2026-08-09
**Reference:** `../mullion` at `09a8b8cbe88521f5c975e42bc0d3104af5afa448` (`feat/activity-bar-hover-delay`)
**Port inspected and validated:** `gpui-mullion` through `b25111177628e43b6f2f91c93ef0e8036ee043f0`
**Validation:** [CI run 31340159314](https://github.com/ignition-is-go/gpui-mullion/actions/runs/31340159314) passed Windows, macOS, Linux, native Wayland runtime, WASM/Trunk, and rendered Chrome interactions.

## Verdict and scope

All user-visible behavior and durable contracts identified in the original audit now have a GPUI-native implementation and focused tests. The only deliberately unimplemented item is **native pane detachment**: it is additive desktop functionality which does not exist in the Leptos reference, so it is an explicit parity non-goal. `NativeDetachedWindowService` remains honestly scoped as a host window-opening scaffold and is not counted as pane parity.

The implementation re-audit found no substantive reference behavior hidden behind a closed Levi task. Pixel-perfect DOM/CSS reproduction is out of scope; typed GPUI styles and deterministic rendered/state assertions replace DOM classes and browser pixel snapshots.

Parity here means behavior and durable contracts, not a literal translation of Leptos signals, DOM events, portals, `HtmlElement`, `DomRect`, CSS classes, or URL/SVG rendering mechanisms. GPUI entities, bounds, actions, overlays, ARIA semantics, and host-rendered `ActivityIcon`s are the scoped native equivalents.

## Status summary (re-audited original rows)

| Original area | Final disposition | Exact implementation/test evidence |
|---|---|---|
| Stable string IDs and `PaneData` bounds | **Implemented** | `PaneId`, `ActivityId`, `CategoryId`, `PaneData`; `tests/model_compatibility.rs::reference_json_shape_round_trips`. |
| `PaneNode` JSON and tree operations | **Implemented** | `PaneNode`, all split/close/move/insert/swap/rotate/balance/layout APIs; 35 `src/tree.rs` tests plus the JSON golden. |
| Layout/geometry algorithms | **Implemented** | `leaf_rect`, `directional_neighbor`, `resize_boundary`, `split_parent_rect`, and `DropEdge` docking geometry; geometry, nested split, and five-zone tests in `src/tree.rs`, `src/drag.rs`, and `src/view.rs`. |
| Headless model commands | **Implemented** | `MullionModel::{try_new,execute_with_options,...}`, exact error/focus semantics; `tests/command_compatibility.rs` and model tests. |
| Activity body rendering | **Implemented** | `ActivityFactoryRegistry`, `ActivityInstance`, stable `(workspace,pane,activity)` cache, update/header/dispose hooks; activity cache tests and `rendered_stateful_activity_is_lazy_stable_updated_and_filtered`. |
| Activity definitions and categories | **Implemented** | `ActivityCatalog`, recursive `ActivityProjection`, typed activity/category chrome and validation; `src/activity_catalog.rs` tests and `rendered_catalog_composes_recursive_chrome_slots_activation_and_trailing_cache`. |
| Activity bar | **Implemented** | `ActivityBarConfig` with four edges, pinned/hidden/auto-hide, configurable open-only hover intent, nested categories, primary/trailing groups, icons and host slots; `src/activity_bar.rs` and rendered rail tests. |
| Split resizing | **Implemented** | proportional pointer tracking against parent bounds, clamp/cancel, splitter actions and cursor context in `MullionView`; `horizontal_split_drag_is_proportional_clamped_exact_and_released` and `nested_vertical_drag_uses_its_parent_bounds_and_cancels`. |
| Pane docking | **Implemented** | `DockDrag`, `DockHover`, `DropEdge`, explicit pane handle, five-zone feedback/self-drop/cancel; portable drag tests and two rendered pane-drag tests. |
| Activity-to-new-pane drag | **Implemented** | `DockConfig::with_new_pane_factory`, `MullionView::{with_new_pane_factory,set_new_pane_factory}`, `PaneEvent::ActivityDropped`; model/event tests and `typed_nested_and_trailing_activity_drags_create_panes_in_all_five_zones`. |
| Focus/zoom | **Implemented** | `PaneFocusBehavior`, controlled/local `MullionSettings`, `FocusPresentation`, coherent focus/zoom repair; settings/focus/model tests and four rendered policy/transition tests. |
| Full command/action/keymap | **Implemented** | 37 static `PaneCommand`s, dynamic `FocusPane`, exhaustive GPUI action conversion, `PaneCommandExecutionOptions`, direct and prefix keymaps, editable-target policy; command/action/keymap and rendered dispatch tests. |
| Command palette/catalog | **Implemented** | `PaletteEntry`, `PaletteInvocation`, full metadata, focus submenu, activity projection/search and `MullionView::{palette_entries,search_palette,invoke_palette}`; palette unit tests and `live_palette_projects_searches_and_executes_typed_invocations`. |
| Workspaces | **Implemented** | validated `WorkspaceSet::{add,remove,rename,reorder,update_tree,try_switch}`, mounted view operations, typed snapshots, defined focus/zoom scope; workspace and six rendered workspace tests. |
| Theming and chrome extensibility | **Implemented** | `MullionStyles`, light/dark/system `MullionThemeMode`, activity/category icons and headers, app/leading/trailing/accessory slots, pane border callback; style/theme/catalog rendered tests. |
| Overlay escape hatch | **Implemented** | window-root `OverlayStack`, controlled source, modal/toast/drag tiers, placement/size/backdrop/dismiss/click-through/accessibility policy; seven unit and five rendered overlay tests. |
| Detached pane windows | **Explicit parity non-goal** | Absent from the Leptos reference. `DetachedWindowService`/`NativeDetachedWindowService` and `examples/detached_window.rs` only demonstrate a host-owned window; no detach/sync/reattach claim is made. |
| Rendered interaction tests | **Implemented and runtime-validated** | 32 `#[gpui::test]` cases in `src/view.rs`; `.github/scripts/check_browser.mjs` drives real canvas hover/click, actions, zoom, workspace switching, and activity selection. CI run 31340159314 passed at `b251111`. |

## Original P0 acceptance items

### MUL-P0-001 — Stateful activity-content contract — **Implemented**

- **API:** `ActivityFactoryRegistry<D>`, `ActivityInstance<D>::{with_header,with_update,with_dispose}`, `ActivityCacheKey`, and `MullionView::{with_activity_factories,register_activity_factory,clear_activity_cache}`.
- **Semantics:** one lazy stable entity per `(workspace, pane, activity)`; topology-only changes retain identity; pane-data changes call the update hook; filtering, pane/workspace removal, explicit clearing, and root release dispose exactly once.
- **Tests:** the four `src/activity.rs` cache/registry tests and view tests `rendered_stateful_activity_is_lazy_stable_updated_and_filtered`, `root_release_disposes_cached_instance_exactly_once`, `explicit_clear_then_root_release_does_not_double_dispose`, and `removing_workspace_disposes_only_its_activity_cache_namespace`.

### MUL-P0-002 — Command compatibility — **Implemented**

- **API:** `PaneCommand::{catalog,id,name,group,description}`, `PaneCommandError`, `MullionModel::execute_with_options`, `PaneCommandExecutionOptions`.
- **Semantics:** kebab-case reference IDs, distinct `SplitUnavailable`, `SplitRefused`, `InvalidIndex`, `NoNeighbor`, and `NotApplicable` failures, and reference close-successor focus behavior.
- **Tests:** all eight `tests/command_compatibility.rs` goldens/traces, including every command ID/error/catalog entry and middle/last close successors.

### MUL-P0-003 — Event-stream compatibility — **Implemented**

- **API:** `PaneEvent::ActivityDropped`, `PaneEvent::{is_persistence,is_transient}`, non-echoing `MullionModel::try_set_tree`, and explicitly local `try_replace_tree`.
- **Semantics:** exact specific-event/ratio/snapshot ordering; upstream replacement reconciles transient state without echo; GPUI-only focus/zoom/data events are explicitly classified.
- **Tests:** all eight `tests/events_compatibility.rs` traces, including balance ratios, activity drop/refusal, upstream/local replacement, and event classification.

### MUL-P0-004 — Persisted layout validation — **Implemented**

- **API:** `PaneNode::validate`, `PaneValidationError`, `MullionModel::try_new`, `try_set_tree`, `try_replace_tree`, and validated workspace/catalog constructors.
- **Tests:** five hostile validation tests in `src/tree.rs`, model atomicity tests, workspace validation tests, and Unicode/special-character IDs in compatibility goldens. Duplicate IDs, non-finite ratios, and out-of-range ratios are rejected with paths/split keys.

## Original P1 acceptance items

### MUL-P1-001 — Proportional split-handle dragging — **Implemented**

`MullionView` uses pointer down/move/up state and the actual parent split bounds, updates continuously, clamps to `0.1..=0.9`, emits the exact stored `Resized` ratio, releases/cancels safely, and exposes splitter-local keyboard resize/cancel actions under `MULLION_SPLITTER_KEY_CONTEXT`. Evidence: the two rendered split-drag tests, `model::resize_event_reports_the_stored_clamped_ratio`, and `tree::set_split_ratio_rejects_non_finite`.

### MUL-P1-002 — Five-zone pane docking with feedback — **Implemented**

`DropEdge::{from_point,indicator_in}`, `DockDrag::pane`, and view-owned hover state implement left/right/top/bottom/center zones, an explicit pane drag handle, real-bounds feedback, cancellation, self-drop refusal, and source-preserving moves. Evidence: eight `src/drag.rs` table/property tests and rendered tests `typed_pane_drag_drives_all_five_zones_with_exact_events_and_indicators` and `dock_drag_self_right_click_nested_cancel_and_release_are_no_ops_until_valid_drop`.

### MUL-P1-003 — Activity-to-new-pane docking — **Implemented**

`DockDrag::new_activity`, `NewPaneFactory`, `DockConfig`, and view factory setters distinguish copy from pane move, mint host ID/data, support all five zones/single panes, focus the new pane, and order `ActivityDropped` before `TreeChanged`. Evidence: drag/model tests, `events_compatibility::{activity_drop_trace_contains_host_values_and_focuses_last,rejected_activity_drops_are_atomic_and_silent}`, and the rendered all-zones activity-drag test.

### MUL-P1-004 — Activity-bar information architecture — **Implemented**

`ActivityCatalog` preserves arbitrary nesting, order, active paths, inherited color and typed GPUI icon/header chrome; primary/trailing trees remain distinct. `ActivityBarConfig` provides four edges, `Pinned`/`Hidden`/`AutoHide`, and `ActivityBarHoverIntent::expand_delay_ms`, matching reference HEAD's configurable open-only hover intent. `ActivityBarSlots`/`PaneHeaderConfig` provide app icon, move handle composition, controls, accessories, and leading/trailing host content. Evidence: catalog/bar unit tests and rendered catalog/hidden-auto-hide tests.

### MUL-P1-005 — Entire command set through GPUI actions — **Implemented**

All 37 static commands and dynamic `FocusPane` round-trip through `action_for_command`/`command_for_action`; `register_key_bindings`, `try_register_key_bindings`, `MullionKeymap::{mullion,tmux}`, split factory and resize-step options cover host behavior and rebinding. `MullionEditable` cooperative context prevents editing shortcut capture unless explicitly enabled. Evidence: five command-action tests, ten keymap tests, and three rendered dispatch/configuration tests.

### MUL-P1-006 — Focus policy and presentation — **Implemented**

Serializable `PaneFocusBehavior::{Hover,Click}`, controlled/local `MullionSettings`, programmatic model focus, and opt-in `FocusPresentation` implement pointer/keyboard policy, internal focus framing, inactive wash, zoom, close, tree-replacement, and workspace reconciliation. Evidence: focus/settings tests and rendered focus tests named in the summary.

### MUL-P1-007 — Workspace management and view-state scope — **Implemented**

`WorkspaceSet` and mounted `MullionView` expose validated add/remove/rename/reorder/update/switch operations. Invalid changes are atomic; outgoing trees persist; typed `WorkspaceEvent::SnapshotChanged` and `WorkspaceChanged` ordering is tested. Focus and zoom are transient Mullion view state: they survive a switch only when their IDs exist in the incoming tree, otherwise reconcile without contaminating serialized workspaces. Evidence: seven workspace unit tests and six rendered workspace tests.

### MUL-P1-008 — Rendered interaction tests — **Implemented and validated**

- **Native GPUI test context:** 32 `#[gpui::test]` cases in `src/view.rs` exercise content lifecycle, selection/catalog, focus, close/tree repair, complete commands/keymaps, workspace mutation/switching, five-zone pane/activity docking, resize, overlays, styles, accessibility, and palette behavior.
- **Browser runtime:** `.github/scripts/check_browser.mjs` connects to Chrome DevTools under Xvfb, requires a sustained live GPUI canvas/test bridge, then drives pointer focus, full-keymap focus/zoom, workspace tabs, and activity selection and asserts the portable tree/state.
- **CI matrix:** native check/test/clippy on Ubuntu/macOS/Windows, executed native Wayland demo liveness, wasm check/clippy, Trunk build, and executed Chrome interactions.
- **Scoped testing choice:** cross-platform GPUI pixel screenshots are not a stable parity oracle. Exact geometry/state/event assertions and rendered interaction tests are used instead; screenshots are retained as browser failure diagnostics.
- **Validation:** run 31340159314 passed at `b251111`, including a nested X11-backed Weston compositor with a real Wayland client/surface/seat path and a sustained rendered Chrome interaction sequence.

## Original P2 acceptance items

### MUL-P2-001 — Host chrome slots and headers — **Implemented**

`ActivityChrome::with_header`, `ActivityBarSlots::{with_app_icon,with_leading,with_trailing,with_pane_accessory}`, `PaneHeaderConfig::{with_visible,with_accessory}`, and `ActivityBarHostConfig::with_pane_border_color` provide every audited provider hook using GPUI renderers. The rendered catalog/chrome test proves composition.

### MUL-P2-002 — Expanded theming — **Implemented**

`MullionStyles` owns typed root, pane, activity-bar, split-handle, drop-overlay, header, and workspace-switcher geometry/state tokens. `MullionTheme::{light,dark}` and serializable `MullionThemeMode::{Light,Dark,System}` cover light/system selection; `drop_target` feeds `DropOverlayStyle`. Evidence: style/theme tests and `custom_style_geometry_and_theme_mode_are_composed_independently`.

### MUL-P2-003 — Overlay escape hatch — **Implemented**

`OverlayStack`, `MullionOverlay`, `OverlayPolicy`, `ControlledOverlaySource`, and `OverlayHostConfig` render at the Mullion root above clipped pane content. Modal/toast/drag ordering, backdrop, alignment/size, outside dismissal, click-through, validation, controlled updates, and accessibility labels are covered by seven unit and five rendered overlay tests.

### MUL-P2-004 — Palette and accessibility adapters — **Implemented**

The typed palette catalog/search/invocation APIs are independent of host UI and include a live focus submenu plus activity entries. `MullionAccessibilityNode` supplies stable roles, labels, selected/expanded/disabled/modal state; the rendered view applies GPUI roles/ARIA labels and keyboard actions to icon-only controls. Evidence: four palette tests, two accessibility tests, `live_palette_projects_searches_and_executes_typed_invocations`, and the catalog/overlay rendered tests.

### MUL-P2-005 — Detached windows — **Explicit scoped non-goal (not parity)**

The Leptos reference has no native OS-window detachment. Therefore ownership transfer, synchronized activity/data/events, close/reattach, and workspace persistence across native windows are not required for parity and are intentionally not implemented here. `DetachedWindowService`, `WindowCapabilities`, and `examples/detached_window.rs` are only an additive host-window capability scaffold. Do not market them as pane detachment. The existing P2-005 Levi task should be resolved by explicitly accepting this scope, not by claiming implementation.

## Required regression matrix disposition

1. **Implemented:** serde/ID/tree/workspace/command/event goldens and hostile validation; see compatibility tests plus tree/workspace suites.
2. **Implemented:** exact command tree/focus/error/event traces; see `tests/command_compatibility.rs` and `tests/events_compatibility.rs`.
3. **Implemented:** close successors, balance ratios, invalid/refused operations, duplicate IDs, and non-finite ratios.
4. **Implemented:** filtering/fallback, recursive ordering/identity/active paths, headers, pane updates, stable entity identity, and disposal.
5. **Implemented:** pointer start/move/end/cancel, clamp, nested parent geometry, exact stored ratio, and keyboard splitter actions.
6. **Implemented:** five pane zones, self-drop/cancel/feedback, five activity-copy zones, and factory refusal.
7. **Implemented:** complete action/keymap catalog, direct/prefix customization, exact modifiers, editable policy, and serde.
8. **Implemented:** hover/click, keyboard, zoom, close/tree repair, and workspace-switch focus policy.
9. **Implemented:** workspace add/remove/rename/reorder/update/switch/persist/invalid-active behavior and transient view-state policy.
10. **Implemented and validated:** native Windows/macOS/Linux gates, a 15-second native Wayland demo under nested Weston, wasm/Trunk, and executed Chrome interactions.

## Close recommendations

- **MUL-P1-008 / `lv-5d80`:** validation is complete; close at the validated implementation/documentation ancestry.
- **MUL-P2-005 / `lv-3d13`:** closed as the explicit additive non-goal above, without claiming a real detached-pane implementation.
- **Umbrella / `lv-24d4`:** all parity requirements are implemented, validated, or explicitly scoped; close after the final documentation head passes CI.
- **New follow-ups:** none filed. The re-audit found no substantive missing reference behavior.
