# GPUI architecture

Mullion is a GPUI library with compatibility requirements inherited from the earlier Leptos implementation. The Leptos code defines observable behavior and persisted data, not the implementation architecture.

## Design rules

1. **GPUI owns UI lifetime.** Stateful content is an `Entity<T>`/`AnyView`; registrations and subscriptions are retained as RAII handles and dropped with their owner.
2. **The model stays portable.** Pane-tree algorithms do not depend on a window. `MullionView` translates model events into GPUI notifications and rendered elements.
3. **Render projects stable state.** Rendering should not perform external registration, invoke host lifecycle callbacks, or create independent timer loops. Reconciliation that can call host code belongs in explicit mutation/frame phases.
4. **Frames come from GPUI.** Pointer work is coalesced with `Window::on_next_frame`; coordinated motion uses `Window::request_animation_frame`. Timers are reserved for elapsed-time policies such as hover intent, not synthetic refresh clocks.
5. **Cache only at meaningful ownership boundaries.** Activity bodies are durable GPUI entities. Lightweight value chrome can be rebuilt until measurement shows that a stable sub-entity reduces work.
6. **Foreground data stays foreground-local.** `PaneData` requires only `Clone + PartialEq + 'static`; serialization and thread-safety bounds belong on the operations that need them.
7. **Browser and native share the view.** WASM bridges are demo/test adapters and cannot bypass production interaction semantics.
8. **Performance changes require invariants.** Coalescing, validation shortcuts, and caches must have deterministic tests for mutation count, ownership, and invalidation.

## Current ownership boundaries

- `MullionModel<D>` owns validated pane topology, focus, zoom, and portable events.
- `MullionView<D>` coordinates the model with one GPUI root window.
- `ActivityCache<D>` retains stable host-created activity entities by workspace/pane/activity identity.
- Command-palette entries are owned by retained `Registration` handles. Detaching or releasing Mullion removes only Mullion-owned entries.
- Split resize input is handled once at the root and applies at most one latest-value mutation per GPUI frame.
- Activity/focus motion uses one state scan per requested GPUI animation frame rather than an executor timer per motion.
- Host activity factories and update/dispose callbacks run in deferred reconciliation after render.
- Splitters retain stable `FocusHandle`s and own their key context/actions locally.
- The model tree is immutable-`Rc` shared during render, so unrelated frames never clone consumer pane data.
- Transient motion, hover, split-bound, and focus-handle maps are bounded by the reconciled live topology/catalog.

## Known hardening work

The following focused work remains before a future 1.0 stability promise:

- Add a dedicated entity-backed modal layer if hosts need keyboard-focus containment and restoration;
  `a11y_modal` currently describes assistive-technology semantics rather than owning host content focus.
- Decide whether pane chrome merits additional entity/cache boundaries only after profiling real
  applications; entity-per-pane is not an assumed optimization.
- Prefer an observable entity-backed overlay source if controlled overlay counts become material;
  the compatibility source remains a documented pure pull callback.
- Remove hidden migration aliases in an explicitly breaking pre-1.0 release after known consumers
  have adopted the canonical typed APIs.

Deterministic scaling tests now cover 1/8/29/128 panes with varying activity and workspace counts.
The curated root facade and stability rules are documented in `API_EVOLUTION.md`.
These remaining items are tracked by `lv-dcc1`.
