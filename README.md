# gpui-mullion

Native split panes and activity surfaces for GPUI, targeting browsers plus Windows, macOS, and Linux. GPUI and `gpui_platform` are pinned to official Zed revision `08827f9208b4848d62f3faf86ffa15155966d63c`.

This repository is the GPUI successor to the Leptos `mullion` library and is intended to become the canonical pane UI for Rship on desktop and the web. One shared `MullionView` implementation runs on every target; only the thin application host differs. The Leptos implementation remains migration/reference material rather than a separately maintained frontend. Persisted pane trees deliberately retain Mullion's serde representation so existing layouts migrate cleanly.

## Current implementation baseline

> [!IMPORTANT]
> This is not yet a feature-complete replacement for the reference Mullion. See the
> [parity audit](docs/PARITY_AUDIT.md) for the exact gaps and acceptance criteria.

- Portable, serde-compatible binary `PaneNode<D>` with stable string pane/activity/category IDs.
- Split, close, move, swap, resize, rotate, balance, five standard layouts, stable split keys, geometric directional navigation, boundary calculations, and extensive inherited model tests.
- Toolkit-independent `MullionModel`: durable focus, zoom, pane data/activity updates, commands, host-created splits, and typed mutation plus snapshot events.
- Shared GPUI `MullionView`: recursive layout, basic activity content, headers, focus chrome, zoom, center-only pane docking, click-step resize, a partial action set, and default bindings.
- Serializable named workspaces switched inside the shared view, plus native/browser baseline demos.

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

At application startup call `register_key_bindings(cx)` and focus `view.read(cx).focus_handle()` after opening the window (as the demo does). Subscribe to `PaneEvent<D>` for pane/model changes. `MullionView::new_with_workspaces` optionally gives the view ownership of a `WorkspaceSet`; every `TreeChanged` snapshot is persisted into its active workspace, the built-in tab strip switches trees in the same window/canvas, and `WorkspaceChanged` is emitted after a successful switch. Use `workspaces()` to inspect the current set and `switch_workspace(...)` to switch programmatically.

## Window architecture

A browser session owns exactly one document/canvas-backed GPUI window. `MullionView` renders every pane and workspace inside that root; it never opens a window while rendering or mutating the model. The same rule is the portable default on desktop. Detached OS windows are an optional, host-owned desktop extension through `DetachedWindowService`. `WindowCapabilities::for_service(...)` reports the capability actually installed by the host; the portable default and `UnavailableDetachedWindows` report unavailable without panicking on wasm. Desktop hosts can use `NativeDetachedWindowService` (see `cargo run --example detached-window`). Thus multi-window policy cannot leak into the shared pane tree, serialization, or view.

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
- click a pane to focus it;
- drag a pane onto another pane to relocate it;
- left/right-click a separator to adjust its ratio;
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

## Migration / parity roadmap

The new repository stands alone; future work lands here rather than maintaining a browser adapter.

| Area | Status |
|---|---|
| persisted tree, geometry, and layout algorithms | parity baseline complete |
| command IDs/errors/focus policy and event traces | partial; compatibility fixes required |
| native recursive view, zoom, and basic activity content | implemented baseline |
| center-only pane docking and click-step resize | implemented baseline |
| stateful per-pane activity/header lifecycle | implemented with lazy stable entities and deterministic disposal |
| proportional pointer-drag resize with keyboard accessibility | missing |
| five-edge drop target overlay and activity-to-create drag | missing |
| visual nested category expansion; primary/trailing groups | missing |
| activity rails on all four edges; hide/auto-hide/hover intent | missing |
| host accessories, native icon/asset abstraction, overlays | missing |
| full actions/keymaps, settings, and palette integration | partial/missing |
| real pane detachment into desktop windows | scaffold only |
| rendered interaction and browser runtime tests | missing |

DOM/CSS concepts (`web_sys`, portals, HTML drag transfer, CSS class/URL icons) are intentionally not API compatibility goals. They will receive GPUI-native replacements.

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
