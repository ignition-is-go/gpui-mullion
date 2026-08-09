use gpui::{div, prelude::*, px, rgb, size, App, Bounds, WindowBounds, WindowOptions};
use gpui_mullion::{
    register_key_bindings, Activity, ActivityId, ActivityNode, MullionView, PaneId, PaneNode,
    SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
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

fn launch(cx: &mut App) {
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
    let workspaces = WorkspaceSet {
        active: WorkspaceId("rship".into()),
        workspaces: vec![
            Workspace {
                id: WorkspaceId("rship".into()),
                name: "Rship".into(),
                tree,
            },
            Workspace {
                id: WorkspaceId("browser".into()),
                name: "Browser".into(),
                tree: PaneNode::Split {
                    direction: SplitDirection::Vertical,
                    ratio: 0.55,
                    first: Box::new(PaneNode::leaf_with_activity(
                        PaneId::new("browser-main"),
                        ActivityId::new("files"),
                        PaneData {
                            project: "Browser".into(),
                        },
                    )),
                    second: Box::new(PaneNode::leaf_with_activity(
                        PaneId::new("browser-console"),
                        ActivityId::new("terminal"),
                        PaneData {
                            project: "Browser".into(),
                        },
                    )),
                },
            },
        ],
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
        move |window, cx| {
            let view = cx.new(|cx| {
                MullionView::new_with_workspaces(workspaces, activities, cx)
                    .expect("demo workspace set has a valid active workspace")
            });
            view.read(cx).focus_handle().clone().focus(window, cx);
            view
        },
    )
    .unwrap();
    cx.activate(true);
    #[cfg(target_family = "wasm")]
    cx.refresh_windows();
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    gpui_platform::application().run(launch);
}

#[cfg(target_family = "wasm")]
thread_local! {
    static APPLICATION: std::cell::RefCell<Option<gpui::ApplicationHandle>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(target_family = "wasm")]
fn main() {
    gpui_platform::web_init();
    let application = gpui_platform::application().run_embedded(launch);
    APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
}
