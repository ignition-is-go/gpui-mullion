# gpui-mullion

Native split panes and activity surfaces for GPUI, targeting browsers plus Windows, macOS, and Linux. GPUI and `gpui_platform` are pinned to official Zed revision `08827f9208b4848d62f3faf86ffa15155966d63c`.

This repository is the GPUI successor to the Leptos `mullion` library and is intended to provide a canonical pane UI on desktop and the web. One shared `MullionView` implementation runs on every target; only the thin application host differs. The Leptos implementation remains migration/reference material rather than a separately maintained frontend. Persisted pane trees deliberately retain Mullion's serde representation so existing layouts migrate cleanly.

## Current implementation baseline

> [!IMPORTANT]
> The model and interaction contracts have broad GPUI-native coverage, but the port is **not yet
> feature-complete or pixel-compatible** with the Leptos reference. Visible split controls,
> activity-to-pane affordances in the shared demo, activity-label flyouts, and the remaining exact
> chrome/layout reproduction are active blockers. Native pane detachment remains an additive,
> explicit non-goal—not reference parity.

- Portable, validated, serde-compatible `PaneNode<D>` with stable string pane/activity/category IDs and reference command/event semantics.
- Split, close, move, swap, proportional resize, rotate, balance, five standard layouts, geometric navigation, five-zone pane docking, and activity-to-new-pane docking.
- Stateful per-`(workspace, pane, activity)` GPUI entities with optional headers, update hooks, stable topology lifecycle, and deterministic disposal.
- Recursive activity catalogs with primary/trailing groups, typed icons/chrome, four-edge pinned/hidden/auto-hide rails, configurable hover delay/duration, scrolling overflow, and host slots.
- Complete command metadata/actions and direct/prefix keymaps, configurable focus/presentation, validated mounted workspaces, root overlays, palette/accessibility adapters, and typed light/dark/system styling.
- Rendered GPUI interactions plus executed Chrome/WASM canvas interactions and a 15-second native Wayland demo smoke in CI.

## Quick start

```rust
use std::sync::Arc;
use gpui::{div, prelude::*};
use gpui_mullion::*;

// D must be Clone + PartialEq + serde + Send + Sync + 'static.
let activity = Activity {
    id: ActivityId::new("files"),
    name: "Files".into(),
    filter: |_| true,
    render: Arc::new(|pane, _data: &MyData| {
        div().child(format!("Files in {}", pane.0)).into_any_element()
    }),
};
let tree = PaneNode::leaf_with_activity(
    PaneId::new("main"), ActivityId::new("files"), my_data,
);
let view = cx.new(|cx| {
    MullionView::new(tree, vec![ActivityNode::Activity(activity)], cx)
});
```

For durable content, keep the legacy `Activity` definition and add a UI-local factory registry:

```rust
let factories = ActivityFactoryRegistry::new().with_factory(
    ActivityId::new("files"),
    |pane, data: &MyData, _window, cx| {
        let body = cx.new(|cx| FilesView::new(pane.clone(), data.clone(), cx));
        ActivityInstance::new(body.clone())
            .with_header(cx.new(|_| FilesHeader::new(pane.clone())))
            .with_update({
                let body = body.clone();
                move |data, _window, cx| body.update(cx, |view, cx| view.set_data(data.clone(), cx))
            })
            .with_dispose(|cx| { /* release host resources; called exactly once */ })
    },
);
let view = cx.new(|cx| {
    MullionView::new(tree, activities, cx).with_activity_factories(factories)
});
```

Factories are `Rc`-backed and deliberately have no `Send + Sync` requirement. They run lazily once per `(workspace, pane, activity)` when selected. Body and optional header are stable `AnyView` entities across activity switching and topology-only changes. A cached instance's update hook runs only when that pane's `D` changes. Filtering out an activity, closing its pane, removing its workspace, or releasing the Mullion root evicts it and calls its App-only disposal hook exactly once. `clear_activity_cache` permits earlier explicit teardown. Registering the same activity ID replaces its factory and returns the previous factory; existing instances retain the factory that created them until eviction. If no factory is registered, the unchanged legacy `Activity::render` path is used.

At application startup call `register_key_bindings(cx)` and focus `view.read(cx).focus_handle()` after opening the window (as the demo does). This registers every sequence in `MullionKeymap::default()` under the `Mullion` context. For a user-defined map, call `try_register_key_bindings(cx, &keymap)` (or `register_keymap`) and report its `KeymapCompileError` rather than panicking.

All 37 static `PaneCommand`s and dynamic `FocusPane { index }` actions route through the view's shared `PaneCommandExecutionOptions`. Configure host-created panes and resize increments with `with_split_factory[_fn]`, `set_split_factory`, `with_resize_step`, or `set_resize_step`; the legacy public `execute(command, factory, cx)` remains available. A missing split factory returns `SplitUnavailable`, while a configured factory returning `None` returns `SplitRefused` at the model API.

Default bindings cooperatively avoid editable descendants. Mullion installs `Mullion` on its root; hosts should install the additional `MullionEditable` key context on text fields, editors, and other controls that must retain editing shortcuts. A custom map can explicitly opt into capturing those shortcuts with `capture_editable_targets(true)`. Splitter-local resize/cancel actions remain separate from `PaneCommand` and become active only in the `MullionSplitter` context after direct splitter selection.

Subscribe to `PaneEvent<D>` for pane/model changes. `MullionView::new_with_workspaces` optionally gives the view ownership of a `WorkspaceSet`; every `TreeChanged` snapshot is persisted into its active workspace, the built-in tab strip switches trees in the same window/canvas, and `WorkspaceChanged` is emitted after a successful switch. Use `workspaces()` to inspect the current set and `switch_workspace(...)` to switch programmatically.

## Window architecture

A browser session owns exactly one document/canvas-backed GPUI window. `MullionView` renders every pane and workspace inside that root; it never opens a window while rendering or mutating the model. The same rule is the portable default on desktop. Detached OS windows are an optional, host-owned desktop extension through `DetachedWindowService`. `WindowCapabilities::for_service(...)` reports the capability actually installed by the host; the portable default and `UnavailableDetachedWindows` report unavailable without panicking on wasm. Desktop hosts can use `NativeDetachedWindowService` (see `cargo run --example detached-window`) to open a host-owned window. This scaffold does **not** detach, render, synchronize, or reattach a Mullion pane and must not be described as pane detachment. Native detached panes are absent from the Leptos reference and are an explicit additive non-goal for parity. Thus multi-window policy cannot leak into the shared pane tree, serialization, or view.

## Demo

```sh
cargo run --example demo
# Force Wayland when both Wayland and X11 session variables are present
# env -u DISPLAY cargo run --example demo

# Browser (install wasm32-unknown-unknown and Trunk first)
cd examples/web && trunk serve
```

Demo controls:

- click the Desktop/Browser tabs to switch internal workspaces;
- hover a pane to focus it with the default policy;
- drag a pane's explicit handle to any of another pane's five docking zones;
- drag a separator proportionally; use its splitter-local keyboard actions for stepping/cancel;
- select or drag activities from the recursive activity rail;
- `Alt+Arrow`, `Alt+PageUp/PageDown` navigate;
- `Ctrl+Shift+Enter` zooms; `Ctrl+Shift+Backspace` closes;
- `Ctrl+Alt+=` balances splits.

## Model compatibility

The public persisted representation matches the reference Mullion model:

```text
PaneNode::Leaf { id, active_activity, data }
PaneNode::Split { direction, ratio, first, second }
```

IDs remain one-field string tuple structs and enum tagging remains serde's external default. Compatibility is guarded by JSON golden tests. UI configuration and render callbacks are intentionally *not* serialized. Use `snapshot()` / `TreeChanged` for persistence, or let an owning `MullionView` keep its `WorkspaceSet` synchronized, and reconstruct native activity renderers at startup. While zoomed, focus navigation coherently moves the zoom viewport to the newly focused pane; zoom and focus therefore never diverge.

## Migration / parity status

The model/interaction core has GPUI-native implementations, but pixel-for-pixel reference parity is still in progress. Behavioral evidence and the reopened visual gaps are recorded in [the re-audit](docs/PARITY_AUDIT.md).

| Area | Status |
|---|---|
| persisted tree, validation, geometry, commands, errors, and event traces | implemented and compatibility-tested |
| stateful activity/header lifecycle and recursive activity catalogs | implemented and rendered-tested |
| proportional resize and five-zone pane/activity docking | interaction core tested; visible affordances still incomplete |
| four-edge activity rails, nested/trailing groups, hide/auto-hide, timed name flyouts, scrolling overflow | implemented; final screenshot pixel parity remains open |
| complete actions/keymaps, focus settings, workspaces, palette/accessibility | model/action core tested; visible split controls open |
| host chrome slots, typed styles/themes, and root overlays | APIs exist; pixel-exact chrome rebuild open |
| browser/WASM canvas interactions | behavioral path validated; screenshot parity open |
| native pane detachment | explicit additive non-goal; host window-opening scaffold only |

DOM mechanisms remain implementation details, but their rendered result is a compatibility target: GPUI bounds/actions/root overlays, ARIA roles, typed styles, and host-rendered icons must reproduce the reference surface pixel-for-pixel. Fixed-viewport reference-versus-GPUI screenshot diffs complement exact geometry, state, event, accessibility, and interaction assertions.

## Development

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo +nightly check --manifest-path examples/web/Cargo.toml --target wasm32-unknown-unknown
cargo +nightly clippy --manifest-path examples/web/Cargo.toml --target wasm32-unknown-unknown -- -D warnings
(cd examples/web && trunk build --release)
```

CI runs native gates on Ubuntu, macOS, and Windows, checks the shared view for `wasm32-unknown-unknown`, and performs a Trunk browser-demo build on Ubuntu. Native MSRV is Rust 1.95; the current GPUI web stack requires nightly for `wasm32-unknown-unknown`.

## License

MIT


## Command palette

Generic command registration, deterministic search, keyboard state, shortcuts, theme, and modal rendering live in [`gpui-command-palette`](https://github.com/ignition-is-go/gpui-command-palette). Mullion retains only `PaletteInvocation`, invocation errors, and adapters for live pane/activity commands. Call `install_command_palette_for_view(&view, window, cx)` after constructing the Mullion entity. It creates and mounts Mullion's adapter palette and installs the external crate's per-window global action route (including initialization), so Ctrl/⌘+K opens the real palette even when focus is elsewhere. The retained `command_palette_for_view(&view, cx)` helper still creates and mounts an uninstalled palette for source compatibility or custom hosts. The shared widget refreshes live entries and routes typed selections back through `MullionView::invoke_palette`; open/query/results/close state is owned solely by its `CommandPalette` entity.
