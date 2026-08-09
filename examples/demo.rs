use gpui::{div, prelude::*, px, rgb, size, App, Bounds, WindowBounds, WindowOptions};
use gpui_mullion::{
    register_key_bindings, Activity, ActivityId, ActivityNode, MullionView, PaneId, PaneNode,
    SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
};
use serde::{Deserialize, Serialize};

#[cfg(target_family = "wasm")]
use gpui::Entity;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PaneData {
    project: String,
}

#[cfg(target_family = "wasm")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserTestState {
    active_workspace: WorkspaceId,
    tree: PaneNode<PaneData>,
    focused: Option<PaneId>,
    zoomed: Option<PaneId>,
    active_activities: Vec<BrowserActiveActivity>,
}

#[cfg(target_family = "wasm")]
#[derive(Serialize)]
struct BrowserActiveActivity {
    pane: PaneId,
    activity: Option<ActivityId>,
}

fn activity(id: &str, name: &str, color: u32) -> ActivityNode<PaneData> {
    let activity_id = ActivityId::new(id);
    let content_label = activity_id.0.clone();
    let name: gpui::SharedString = name.to_owned().into();
    ActivityNode::Activity(Activity {
        id: activity_id,
        name: name.clone(),
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
                .child(format!("{content_label} · {} · {}", data.project, pane.0))
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
            #[cfg(target_family = "wasm")]
            TEST_VIEW.with(|slot| *slot.borrow_mut() = Some(view.clone()));
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
    static TEST_VIEW: std::cell::RefCell<Option<Entity<MullionView<PaneData>>>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(target_family = "wasm")]
fn browser_test_state() -> String {
    APPLICATION.with(|application| {
        let application = application.borrow();
        let application = application
            .as_ref()
            .expect("the embedded GPUI application is retained");
        TEST_VIEW.with(|view| {
            let view = view.borrow();
            let view = view.as_ref().expect("the demo Mullion entity is retained");
            application.update(|cx| {
                let view = view.read(cx);
                let tree = view.model().snapshot();
                let active_activities = tree
                    .leaf_ids()
                    .into_iter()
                    .map(|pane| {
                        let activity = match tree.find(&pane) {
                            Some(PaneNode::Leaf {
                                active_activity, ..
                            }) => active_activity.clone(),
                            _ => None,
                        };
                        BrowserActiveActivity { pane, activity }
                    })
                    .collect();
                let state = BrowserTestState {
                    active_workspace: view
                        .workspaces()
                        .expect("the demo owns workspaces")
                        .active
                        .clone(),
                    tree,
                    focused: view.model().focused().cloned(),
                    zoomed: view.model().zoomed().cloned(),
                    active_activities,
                };
                serde_json::to_string(&state).expect("browser test state is serializable")
            })
        })
    })
}

#[cfg(target_family = "wasm")]
fn install_browser_test_bridge() {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};

    // NOTE(ts): CI drives the rendered canvas and can only snapshot the retained entity.
    let snapshot = Closure::<dyn Fn() -> String>::new(browser_test_state);
    js_sys::Reflect::set(
        &js_sys::global(),
        &JsValue::from_str("__mullionTestState"),
        snapshot.as_ref().unchecked_ref(),
    )
    .expect("globalThis accepts the browser test bridge");
    snapshot.forget();
}

#[cfg(target_family = "wasm")]
fn main() {
    gpui_platform::web_init();
    let application = gpui_platform::application().run_embedded(launch);
    APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
    install_browser_test_bridge();
}
