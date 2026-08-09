# gpui-mullion

Native split panes and activity surfaces for GPUI, targeting browsers plus Windows, macOS, and Linux. GPUI and `gpui_platform` are pinned to official Zed revision `08827f9208b4848d62f3faf86ffa15155966d63c`.

This repository is the GPUI successor to the Leptos `mullion` library and is intended to become the canonical pane UI for Rship on desktop and the web. One shared `MullionView` implementation runs on every target; only the thin application host differs. The Leptos implementation remains migration/reference material rather than a separately maintained frontend. Persisted pane trees deliberately retain Mullion's serde representation so existing layouts migrate cleanly.

## Current implementation baseline

> [!IMPORTANT]
> The audited Leptos behavior now has GPUI-native implementations and focused tests. Exact-head
> CI for `cb1d1aa` is still pending; see the [parity audit](docs/PARITY_AUDIT.md) for evidence and
> release gates. Native pane detachment is an additive, explicit non-goal—not reference parity.

- Portable, validated, serde-compatible `PaneNode<D>` with stable string pane/activity/category IDs and reference command/event semantics.
- Split, close, move, swap, proportional resize, rotate, balance, five standard layouts, geometric navigation, five-zone pane docking, and activity-to-new-pane docking.
- Stateful per-`(workspace, pane, activity)` GPUI entities with optional headers, update hooks, stable topology lifecycle, and deterministic disposal.
- Recursive activity catalogs with primary/trailing groups, typed icons/chrome, four-edge pinned/hidden/auto-hide rails, configurable hover intent, and host slots.
- Complete command metadata/actions and direct/prefix keymaps, configurable focus/presentation, validated mounted workspaces, root overlays, palette/accessibility adapters, and typed light/dark/system styling.
- Rendered GPUI interaction coverage plus an executed Chrome/WASM canvas smoke in CI; the exact-head run remains pending.

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

- click the Rship/Browser tabs to switch internal workspaces;
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

The Leptos reference's audited behavior has a GPUI-native implementation. The exact APIs and tests for every original parity item are recorded in [the re-audit](docs/PARITY_AUDIT.md).

| Area | Status |
|---|---|
| persisted tree, validation, geometry, commands, errors, and event traces | implemented and compatibility-tested |
| stateful activity/header lifecycle and recursive activity catalogs | implemented and rendered-tested |
| proportional resize and five-zone pane/activity docking | implemented and rendered-tested |
| four-edge activity rails, nested/trailing groups, hide/auto-hide/hover intent | implemented and rendered-tested |
| complete actions/keymaps, focus settings, workspaces, palette/accessibility | implemented and rendered-tested |
| host chrome slots, typed styles/themes, and root overlays | implemented and rendered-tested |
| browser/WASM canvas interactions | implemented in CI; exact-head `cb1d1aa` run pending |
| native pane detachment | explicit additive non-goal; host window-opening scaffold only |

DOM/CSS concepts (`web_sys`, portals, HTML drag transfer, CSS classes, and CSS/URL/SVG icon mechanisms) are intentionally not API-compatibility goals. GPUI bounds/actions/root overlays, ARIA roles, typed style tokens, and host-rendered `ActivityIcon`s are their native equivalents. Cross-platform pixel snapshots are also not a stable parity oracle; tests assert exact geometry, state, events, accessibility metadata, and rendered interactions instead.

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
