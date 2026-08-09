//! Minimal desktop host integration for optional detached windows.
#![cfg_attr(target_family = "wasm", allow(dead_code, unused_imports))]

#[cfg(not(target_family = "wasm"))]
use gpui::{div, prelude::*, px, size, App, Bounds, Render, Window, WindowBounds, WindowOptions};
#[cfg(not(target_family = "wasm"))]
use gpui_mullion::{
    DetachError, DetachedWindowService, NativeDetachedWindowService, PaneId, WindowCapabilities,
};

#[cfg(not(target_family = "wasm"))]
struct DetachedPane(PaneId);

#[cfg(not(target_family = "wasm"))]
impl Render for DetachedPane {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(format!("Detached pane: {}", self.0 .0))
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let service = NativeDetachedWindowService::new(|pane, cx| {
            let bounds = Bounds::centered(None, size(px(640.), px(420.)), cx);
            let pane = pane.clone();
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some(format!("Detached {}", pane.0).into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |_, cx| cx.new(|_| DetachedPane(pane)),
            )
            .map(|_| ())
            .map_err(|error| DetachError::Refused(error.to_string()))
        });
        assert!(WindowCapabilities::for_service(&service).detached_windows);
        service.detach(&PaneId::new("editor"), cx).unwrap();
        cx.activate(true);
    });
}

#[cfg(target_family = "wasm")]
fn main() {}
