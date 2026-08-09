# gpui-mullion

Native split panes and activity surfaces for GPUI, targeting browsers plus Windows, macOS, and Linux. GPUI and `gpui_platform` are pinned to official Zed revision `08827f9208b4848d62f3faf86ffa15155966d63c`.

This repository is the GPUI successor to the Leptos `mullion` library and is intended to become the canonical pane UI for Rship on desktop and the web. One shared `MullionView` implementation runs on every target; only the thin application host differs. The Leptos implementation remains migration/reference material rather than a separately maintained frontend. Persisted pane trees deliberately retain Mullion's serde representation so existing layouts migrate cleanly.

## Current production foundation

- Portable, serde-compatible binary `PaneNode<D>` with stable string pane/activity/category IDs.
- Split, close, move, swap, resize, rotate, balance, five standard layouts, stable split keys, geometric directional navigation, boundary calculations, and extensive inherited parity tests.
- Toolkit-independent `MullionModel`: durable focus, zoom, pane data/activity updates, commands, host-created splits, and typed mutation plus snapshot events.
- Shared GPUI `MullionView`: recursive horizontal/vertical layout, activity rail and content renderers, headers, focus chrome, zoom, native pane drag/drop, clickable resize separators, theming, event emission, actions, and default key bindings.
- Serializable named workspaces and a runnable native demo.

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

At application startup call `register_key_bindings(cx)` and focus `view.read(cx).focus_handle()` after opening the window (as the demo does). Subscribe to `PaneEvent<D>` on the view entity to persist `TreeChanged` snapshots.

## Window architecture

A browser session owns exactly one document/canvas-backed GPUI window. `MullionView` renders every pane and workspace inside that root; it never opens a window while rendering or mutating the model. The same rule is the portable default on desktop. Detached OS windows are an optional, host-owned desktop extension through `DetachedWindowService`. `WindowCapabilities::for_service(...)` reports the capability actually installed by the host; the portable default and `UnavailableDetachedWindows` report unavailable without panicking on wasm. Desktop hosts can use `NativeDetachedWindowService` (see `cargo run --example detached-window`). Thus multi-window policy cannot leak into the shared pane tree, serialization, or view.

## Demo

```sh
cargo run --example demo

# Browser (install wasm32-unknown-unknown and Trunk first)
cd examples/web && trunk serve
```

Demo controls:

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

IDs remain one-field string tuple structs and enum tagging remains serde's external default. Compatibility is guarded by JSON golden tests. UI configuration and render callbacks are intentionally *not* serialized. Use `snapshot()` / `TreeChanged` for persistence and reconstruct native activity renderers at startup.

## Migration / parity roadmap

The new repository stands alone; future work lands here rather than maintaining a browser adapter.

| Area | Status |
|---|---|
| persisted tree, geometry, layouts, commands, events | implemented |
| native recursive view, focus, zoom, theming, activity content | implemented |
| pane center drag/drop and resize interaction | implemented baseline |
| proportional pointer-drag resize with keyboard accessibility | next |
| five-edge drop target overlay and activity-to-create drag | next |
| visual nested category expansion; primary/trailing groups | next |
| activity rails on all four edges; hide/auto-hide/hover intent | next |
| host accessories, native icon/asset abstraction, overlays | next |
| settings/palette integration in the Rship host | host migration phase |
| interaction/snapshot tests on all desktop backends | follow-up |

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
