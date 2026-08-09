use gpui::{div, prelude::*, px, rgb, size, App, Bounds, WindowBounds, WindowOptions};
use gpui_mullion::{
    register_key_bindings, Activity, ActivityId, ActivityNode, MullionView, PaneId, PaneNode,
    SplitDirection,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PaneData {
    project: String,
}

fn activity(id: &str, name: &str, color: u32) -> ActivityNode<PaneData> {
    ActivityNode::Activity(Activity {
        id: ActivityId::new(id),
        name: name.to_owned().into(),
        filter: |_| true,
        render: Arc::new(move |pane, data| {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .bg(rgb(color))
                .text_color(gpui::white())
                .text_xl()
                .child(format!("{} · {}", data.project, pane.0))
                .child("Drag a pane onto another to move it")
                .child("Left/right-click a separator to resize")
                .into_any_element()
        }),
    })
}

fn main() {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    gpui_platform::application().run(|cx: &mut App| {
        register_key_bindings(cx);
        let tree = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.62,
            first: Box::new(PaneNode::leaf_with_activity(
                PaneId::new("editor"),
                ActivityId::new("files"),
                PaneData {
                    project: "Rship".into(),
                },
            )),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(PaneNode::leaf_with_activity(
                    PaneId::new("terminal"),
                    ActivityId::new("terminal"),
                    PaneData {
                        project: "Rship".into(),
                    },
                )),
                second: Box::new(PaneNode::leaf_with_activity(
                    PaneId::new("logs"),
                    ActivityId::new("logs"),
                    PaneData {
                        project: "Rship".into(),
                    },
                )),
            }),
        };
        let activities = vec![
            activity("files", "F", 0x243044),
            activity("terminal", "T", 0x26382f),
            activity("logs", "L", 0x3b2929),
        ];
        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("GPUI Mullion".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| MullionView::new(tree, activities)),
        )
        .unwrap();
        cx.activate(true);
    });
}
