use gpui::{div, prelude::*, px, rgb, size, App, Bounds, WindowBounds, WindowOptions};
use gpui_mullion::{
    register_key_bindings, Activity, ActivityBarHostConfig, ActivityBarSlots, ActivityCatalog,
    ActivityCategory, ActivityChrome, ActivityIcon, ActivityId, ActivityNode, CategoryChrome,
    CategoryId, DropEdge, FocusPresentation, MullionSettings, MullionStyles, MullionView,
    PaneFocusBehavior, PaneId, PaneNode, SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex,
};

#[cfg(target_family = "wasm")]
use gpui::Entity;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct DemoData {
    label: String,
    show_files: bool,
    show_search: bool,
    show_settings: bool,
}

impl DemoData {
    fn pane(label: &str) -> Self {
        Self {
            label: label.into(),
            show_files: true,
            show_search: true,
            show_settings: true,
        }
    }
}

fn default_workspace() -> PaneNode<DemoData> {
    PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.4,
        first: Box::new(PaneNode::leaf_with_activity(
            PaneId::new("1"),
            ActivityId::new("1"),
            DemoData::pane("Left"),
        )),
        second: Box::new(PaneNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf_with_activity(
                PaneId::new("2"),
                ActivityId::new("2"),
                DemoData::pane("Right Top"),
            )),
            second: Box::new(PaneNode::leaf_with_activity(
                PaneId::new("3"),
                ActivityId::new("3"),
                DemoData::pane("Right Bottom"),
            )),
        }),
    }
}

fn triple_workspace() -> PaneNode<DemoData> {
    PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.33,
        first: Box::new(PaneNode::leaf_with_activity(
            PaneId::new("10"),
            ActivityId::new("1"),
            DemoData::pane("Files"),
        )),
        second: Box::new(PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf_with_activity(
                PaneId::new("11"),
                ActivityId::new("2"),
                DemoData::pane("Search"),
            )),
            second: Box::new(PaneNode::leaf_with_activity(
                PaneId::new("12"),
                ActivityId::new("3"),
                DemoData::pane("Settings"),
            )),
        }),
    }
}

fn stacked_workspace() -> PaneNode<DemoData> {
    let mut bottom = DemoData::pane("Bottom");
    bottom.show_files = false;
    PaneNode::Split {
        direction: SplitDirection::Vertical,
        ratio: 0.5,
        first: Box::new(PaneNode::leaf_with_activity(
            PaneId::new("20"),
            ActivityId::new("1"),
            DemoData::pane("Top"),
        )),
        second: Box::new(PaneNode::leaf_with_activity(
            PaneId::new("21"),
            ActivityId::new("3"),
            bottom,
        )),
    }
}

fn workspace_set() -> WorkspaceSet<DemoData> {
    WorkspaceSet {
        active: WorkspaceId("default".into()),
        workspaces: vec![
            Workspace {
                id: WorkspaceId("default".into()),
                name: "Default".into(),
                tree: default_workspace(),
            },
            Workspace {
                id: WorkspaceId("triple".into()),
                name: "Triple".into(),
                tree: triple_workspace(),
            },
            Workspace {
                id: WorkspaceId("stacked".into()),
                name: "Stacked".into(),
                tree: stacked_workspace(),
            },
        ],
    }
}

fn icon(glyph: &'static str) -> ActivityIcon {
    ActivityIcon::new(move |_, _| {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.))
            .child(glyph)
            .into_any_element()
    })
}

fn activity_content(name: &'static str, pane: &PaneId, data: &DemoData) -> gpui::AnyElement {
    let title = if matches!(name, "Files" | "Search") {
        format!("{} - {name}", data.label)
    } else {
        name.into()
    };
    let detail = match name {
        "Files" => "Working tree for the focused-pane presentation pass.",
        "Open Editors" => "Four files are open across the focus-state iteration.",
        "Timeline" => "Recent activity from the Mullion design session.",
        "Search" => "Type to search across files...",
        "Keybindings" => "Alt+Arrow focus · Ctrl+Alt+Arrow resize · Ctrl+Shift+Enter zoom",
        "Settings" => "Choose whether pointer hover or click changes the focused pane.",
        _ => "This activity is a placeholder.",
    };
    let rows: &[&str] = match name {
        "Files" => &[
            "M  src/components/pane_view.rs",
            "M  src/components/mullion_root.rs",
            "M  examples/demo/src/main.rs",
            "A  tests/focus_visual.rs",
            "   Cargo.toml",
        ],
        "Open Editors" => &[
            "pane_view.rs       src/components · 428 lines",
            "mullion_root.rs    src/components · 326 lines",
            "main.rs            examples/demo/src · 641 lines",
            "README.md          documentation · 472 lines",
        ],
        "Timeline" => &[
            "09:45  Release demo built",
            "09:42  Focus presentation updated",
            "09:38  Test suite passed · 76 tests",
            "09:18  Command catalog registered",
        ],
        "Keybindings" => &[
            "Alt + Arrow             Focus in that direction",
            "Alt + Shift + Arrow     Move focused pane",
            "Ctrl + Shift + Arrow    Swap with a neighbor",
            "Ctrl + Alt + =          Balance splits",
            "Ctrl/⌘ + K              Browse every Mullion command",
        ],
        "Settings" => &[
            "◉  Click   Focus a pane when it is clicked and keep focus there.",
            "○  Hover   Move focus whenever the pointer enters another pane.",
            "",
            "Controlled by the host through Mullion's typed setting descriptor.",
        ],
        _ => &[],
    };
    div()
        .size_full()
        .overflow_hidden()
        .p_4()
        .bg(rgb(0x111111))
        .text_color(rgb(0xeeeeee))
        .child(
            div()
                .text_xl()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.))
                .text_color(rgb(0x888888))
                .child(detail),
        )
        .child(
            div()
                .mt_4()
                .text_size(px(11.))
                .font_family("monospace")
                .children(rows.iter().map(|row| {
                    div()
                        .py_1()
                        .border_b_1()
                        .border_color(rgb(0x1a1a1a))
                        .child(*row)
                })),
        )
        .child(
            div()
                .absolute()
                .right_3()
                .bottom_3()
                .text_size(px(10.))
                .text_color(rgb(0x555555))
                .child(format!("pane {}", pane.0)),
        )
        .into_any_element()
}

fn activity(id: &str, name: &'static str, filter: fn(&DemoData) -> bool) -> ActivityNode<DemoData> {
    ActivityNode::Activity(Activity {
        id: ActivityId::new(id),
        name: name.into(),
        filter,
        render: Arc::new(move |pane, data| activity_content(name, pane, data)),
    })
}

fn always(_: &DemoData) -> bool {
    true
}
fn files(data: &DemoData) -> bool {
    data.show_files
}
fn search(data: &DemoData) -> bool {
    data.show_search
}
fn settings(data: &DemoData) -> bool {
    data.show_settings
}

fn category(
    id: &str,
    name: &str,
    color: u32,
    children: Vec<ActivityNode<DemoData>>,
) -> ActivityNode<DemoData> {
    ActivityNode::Category(ActivityCategory {
        id: CategoryId::new(id),
        name: name.to_owned().into(),
        color: rgb(color).into(),
        children,
    })
}

fn settings_activity(control: Arc<DemoControl>) -> ActivityNode<DemoData> {
    ActivityNode::Activity(Activity {
        id: ActivityId::new("9"),
        name: "Settings".into(),
        filter: settings,
        render: Arc::new(move |pane, _| {
            let current = control.focus_behavior();
            let click_control = control.clone();
            let hover_control = control.clone();
            div()
                .size_full()
                .p_4()
                .bg(rgb(0x111111))
                .text_color(rgb(0xeeeeee))
                .child(div().text_xl().child("Pane focus behavior"))
                .child(div().mt_2().text_size(px(12.)).text_color(rgb(0x888888)).child(
                    "Choose whether pointer hover or click changes the focused pane.",
                ))
                .child(
                    div()
                        .mt_4()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .id(format!("focus-setting-click:{}", pane.0))
                                .p_3()
                                .border_1()
                                .rounded_md()
                                .border_color(if current == PaneFocusBehavior::Click { rgb(0x75beff) } else { rgb(0x333333) })
                                .cursor_pointer()
                                .child("Click")
                                .child(div().mt_1().text_size(px(11.)).text_color(rgb(0x888888)).child("Focus a pane when it is clicked and keep focus there."))
                                .on_click(move |_, _, cx| { click_control.focus_behavior.store(0, Ordering::SeqCst); cx.refresh_windows(); }),
                        )
                        .child(
                            div()
                                .id(format!("focus-setting-hover:{}", pane.0))
                                .p_3()
                                .border_1()
                                .rounded_md()
                                .border_color(if current == PaneFocusBehavior::Hover { rgb(0x75beff) } else { rgb(0x333333) })
                                .cursor_pointer()
                                .child("Hover")
                                .child(div().mt_1().text_size(px(11.)).text_color(rgb(0x888888)).child("Move focus whenever the pointer enters another pane."))
                                .on_click(move |_, _, cx| { hover_control.focus_behavior.store(1, Ordering::SeqCst); cx.refresh_windows(); }),
                        ),
                )
                .child(div().mt_4().text_size(px(11.)).text_color(rgb(0x888888)).child("Controlled and persisted by the demo host from Mullion's typed setting descriptor."))
                .into_any_element()
        }),
    })
}

fn catalog(control: Arc<DemoControl>) -> ActivityCatalog<DemoData> {
    let primary = vec![
        category(
            "0",
            "Explorer",
            0x75beff,
            vec![
                activity("1", "Files", files),
                activity("2", "Open Editors", always),
                activity("3", "Timeline", always),
                activity("4", "Outline", always),
            ],
        ),
        category(
            "1",
            "Edit",
            0xe8ab53,
            vec![
                activity("5", "Search", search),
                activity("6", "Replace", always),
                activity("7", "Bookmarks", always),
                activity("8", "Snippets", always),
            ],
        ),
        category(
            "2",
            "Preferences",
            0xc586c0,
            vec![
                activity("10", "Themes", always),
                category(
                    "3",
                    "Advanced",
                    0xe8ab53,
                    vec![
                        activity("11", "Keybindings", always),
                        activity("12", "Extensions", always),
                    ],
                ),
            ],
        ),
    ];
    let mut catalog = ActivityCatalog::new(primary).with_trailing(vec![settings_activity(control)]);
    for (id, glyph) in [
        ("1", "F"),
        ("2", "E"),
        ("3", "T"),
        ("4", "O"),
        ("5", "⌕"),
        ("6", "R"),
        ("7", "B"),
        ("8", "<>"),
        ("9", "⚙"),
        ("10", "◐"),
        ("11", "⌨"),
        ("12", "+"),
    ] {
        let mut chrome = ActivityChrome::new(icon(glyph));
        if id == "1" {
            chrome = chrome.with_header(|_, data: &DemoData, _, _| {
                div()
                    .opacity(0.6)
                    .child(data.label.clone())
                    .into_any_element()
            });
        }
        catalog.insert_activity_chrome(ActivityId::new(id), chrome);
    }
    for (id, glyph) in [("0", "▣"), ("1", "✎"), ("2", "⚙"), ("3", "≡")] {
        catalog.insert_category_chrome(CategoryId::new(id), CategoryChrome::new(icon(glyph)));
    }
    catalog
        .validate()
        .expect("canonical demo activity catalog is valid");
    catalog
}

#[derive(Default)]
struct DemoControl {
    split_counter: AtomicU64,
    drop_counter: AtomicU64,
    focus_behavior: AtomicU8,
    palette_open: AtomicBool,
    palette_query: Mutex<String>,
    selected_category: Mutex<Option<String>>,
    bar_hover: Mutex<Option<String>>,
}

impl DemoControl {
    fn focus_behavior(&self) -> PaneFocusBehavior {
        if self.focus_behavior.load(Ordering::SeqCst) == 0 {
            PaneFocusBehavior::Click
        } else {
            PaneFocusBehavior::Hover
        }
    }

    fn reset(&self) {
        self.split_counter.store(0, Ordering::SeqCst);
        self.drop_counter.store(0, Ordering::SeqCst);
        self.focus_behavior.store(0, Ordering::SeqCst);
        self.palette_open.store(false, Ordering::SeqCst);
        *self.palette_query.lock().unwrap() = String::new();
        *self.selected_category.lock().unwrap() = None;
        *self.bar_hover.lock().unwrap() = None;
    }
}

fn host_config(control: Arc<DemoControl>) -> ActivityBarHostConfig<DemoData> {
    let palette = control.clone();
    let hairline = || {
        move |_: &PaneId, _: &DemoData, _: &mut gpui::Window, _: &mut App| {
            div()
                .h(px(1.))
                .mx_1()
                .my_1()
                .bg(rgb(0x1a1a1a))
                .into_any_element()
        }
    };
    ActivityBarHostConfig::new().with_slots(
        ActivityBarSlots::new()
            .with_app_icon(icon("◆"))
            .with_leading(hairline())
            .with_trailing(hairline())
            .with_pane_accessory(move |_, _, _, _| {
                let palette = palette.clone();
                div()
                    .id("demo-command-palette")
                    .debug_selector(|| "demo-command-palette".into())
                    .px_1()
                    .text_size(px(9.))
                    .cursor_pointer()
                    .child("⌘K")
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(
                            Box::new(gpui_command_palette::ToggleCommandPalette),
                            cx,
                        );
                        let next = !palette.palette_open.load(Ordering::SeqCst);
                        palette.palette_open.store(next, Ordering::SeqCst);
                    })
                    .into_any_element()
            }),
    )
}

fn demo_styles() -> MullionStyles {
    let mut styles = MullionStyles::default();
    styles.activity_bar.inactive_icon_opacity = 1.0;
    styles.activity_bar.active_icon_opacity = 1.0;
    styles.activity_bar.expanded_padding = px(10.);
    styles.split_handle.thickness = px(2.);
    styles
}

fn launch(cx: &mut App) {
    register_key_bindings(cx);
    gpui_command_palette::init(cx);
    let control = Arc::new(DemoControl::default());
    control.reset();
    let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("GPUI Mullion · shared reference demo".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        move |window, cx| {
            let split_control = control.clone();
            let drop_control = control.clone();
            let settings_control = control.clone();
            let view = cx.new(|cx| {
                let split_factory = move |source: &PaneId,
                                          direction: SplitDirection,
                                          data: &DemoData| {
                    let sequence = split_control.split_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    let axis = match direction {
                        SplitDirection::Horizontal => "horizontal",
                        SplitDirection::Vertical => "vertical",
                    };
                    Some((
                        PaneId::new(format!("split-{sequence}")),
                        DemoData {
                            label: format!("{} ({axis} split from {})", data.label, source.0),
                            ..data.clone()
                        },
                    ))
                };
                let new_pane = move |activity: &ActivityId, destination: &PaneId, _: DropEdge| {
                    let sequence = drop_control.drop_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    Some((
                        PaneId::new(format!("drop-{sequence}")),
                        DemoData {
                            label: format!("Activity {} beside {}", activity.0, destination.0),
                            show_files: true,
                            show_search: true,
                            show_settings: true,
                        },
                    ))
                };
                let settings_reader = settings_control.clone();
                let settings_writer = settings_control.clone();
                let settings = MullionSettings::controlled(
                    move || settings_reader.focus_behavior(),
                    move |next| {
                        settings_writer
                            .focus_behavior
                            .store(u8::from(next == PaneFocusBehavior::Hover), Ordering::SeqCst)
                    },
                );
                MullionView::new_with_workspaces(workspace_set(), Vec::new(), cx)
                    .expect("demo workspace set is valid")
                    .with_activity_catalog(catalog(control.clone()))
                    .expect("demo catalog is valid")
                    .with_split_factory_fn(split_factory)
                    .with_new_pane_factory(new_pane)
                    .with_settings(settings)
                    .with_focus_presentation(
                        FocusPresentation::new()
                            .with_focus_indicator(true)
                            .with_unfocused_pane_opacity(0.75),
                    )
                    .with_styles(demo_styles())
                    .with_activity_bar_host(host_config(control.clone()))
            });
            gpui_mullion::command_palette_for_view(&view, cx);
            view.read(cx).focus_handle().clone().focus(window, cx);
            #[cfg(target_family = "wasm")]
            {
                TEST_VIEW.with(|slot| *slot.borrow_mut() = Some(view.clone()));
                TEST_CONTROL.with(|slot| *slot.borrow_mut() = Some(control.clone()));
            }
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
    static APPLICATION: std::cell::RefCell<Option<gpui::ApplicationHandle>> = const { std::cell::RefCell::new(None) };
    static TEST_VIEW: std::cell::RefCell<Option<Entity<MullionView<DemoData>>>> = const { std::cell::RefCell::new(None) };
    static TEST_CONTROL: std::cell::RefCell<Option<Arc<DemoControl>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(target_family = "wasm")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserTestState {
    active_workspace: WorkspaceId,
    tree: PaneNode<DemoData>,
    focused: Option<PaneId>,
    zoomed: Option<PaneId>,
    active_activities: Vec<BrowserActiveActivity>,
    focus_behavior: PaneFocusBehavior,
    palette_open: bool,
    palette_query: String,
    palette_results: Vec<String>,
    selected_category: Option<String>,
    bar_hover: Option<String>,
    split_sequence: u64,
    drop_sequence: u64,
    catalog: serde_json::Value,
}

#[cfg(target_family = "wasm")]
#[derive(Serialize)]
struct BrowserActiveActivity {
    pane: PaneId,
    activity: Option<ActivityId>,
    label: String,
}

#[cfg(target_family = "wasm")]
fn browser_test_state() -> String {
    APPLICATION.with(|application| {
        let application = application.borrow();
        let application = application.as_ref().expect("embedded application retained");
        TEST_VIEW.with(|view| TEST_CONTROL.with(|control| {
            let view = view.borrow();
            let view = view.as_ref().expect("demo view retained");
            let control = control.borrow();
            let control = control.as_ref().expect("demo control retained");
            application.update(|cx| {
                let view = view.read(cx);
                let tree = view.model().snapshot();
                let active_activities = tree.leaf_ids().into_iter().map(|pane| {
                    let (activity, label) = match tree.find(&pane) {
                        Some(PaneNode::Leaf { active_activity, data, .. }) => (active_activity.clone(), data.label.clone()),
                        _ => unreachable!(),
                    };
                    BrowserActiveActivity { pane, activity, label }
                }).collect();
                let query = control.palette_query.lock().unwrap().clone();
                let palette_results = view.search_palette(&query).into_iter().take(12).map(|result| result.entry.id).collect();
                serde_json::to_string(&BrowserTestState {
                    active_workspace: view.workspaces().unwrap().active.clone(),
                    tree,
                    focused: view.model().focused().cloned(),
                    zoomed: view.model().zoomed().cloned(),
                    active_activities,
                    focus_behavior: control.focus_behavior(),
                    palette_open: control.palette_open.load(Ordering::SeqCst),
                    palette_query: query,
                    palette_results,
                    selected_category: control.selected_category.lock().unwrap().clone(),
                    bar_hover: control.bar_hover.lock().unwrap().clone(),
                    split_sequence: control.split_counter.load(Ordering::SeqCst),
                    drop_sequence: control.drop_counter.load(Ordering::SeqCst),
                    catalog: serde_json::json!({
                        "primary": [
                            {"id":"0","name":"Explorer","color":"#75beff","children":["1","2","3","4"]},
                            {"id":"1","name":"Edit","color":"#e8ab53","children":["5","6","7","8"]},
                            {"id":"2","name":"Preferences","color":"#c586c0","children":["10", {"id":"3","name":"Advanced","color":"#e8ab53","children":["11","12"]}]}
                        ],
                        "trailing": [{"id":"9","name":"Settings"}],
                        "activities": {"1":"Files","2":"Open Editors","3":"Timeline","4":"Outline","5":"Search","6":"Replace","7":"Bookmarks","8":"Snippets","9":"Settings","10":"Themes","11":"Keybindings","12":"Extensions"}
                    }),
                }).unwrap()
            })
        }))
    })
}

#[cfg(target_family = "wasm")]
fn js_text(value: &wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::JSON::stringify(value)
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_default()
}

#[cfg(target_family = "wasm")]
fn hover_activity_bar(view: &Entity<MullionView<DemoData>>, pane: &PaneId, cx: &mut App) -> bool {
    let tree = view.read(cx).model().snapshot();
    let Some(rect) = gpui_mullion::leaf_rect(&tree, pane, |key| {
        gpui_mullion::find_ratio(&tree, key).unwrap_or(0.5)
    }) else {
        return false;
    };
    let Some(window) = cx.active_window() else {
        return false;
    };
    window
        .update(cx, |_, window, cx| {
            let viewport = window.viewport_size();
            let position = gpui::point(
                viewport.width * rect.left as f32 + px(14.),
                viewport.height * (rect.top + rect.height / 2.) as f32,
            );
            let _ = window.dispatch_event(
                gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                    position,
                    modifiers: gpui::Modifiers::none(),
                    pressed_button: None,
                }),
                cx,
            );
        })
        .expect("demo window remains open while its test bridge is installed");
    true
}

#[cfg(target_family = "wasm")]
fn browser_test_action(action: wasm_bindgen::JsValue, payload: wasm_bindgen::JsValue) -> String {
    let action = js_text(&action);
    let payload_text = js_text(&payload);
    let payload_json: serde_json::Value = serde_json::from_str(&payload_text)
        .unwrap_or_else(|_| serde_json::Value::String(payload_text.clone()));
    APPLICATION.with(|application| {
        TEST_VIEW.with(|view| {
            TEST_CONTROL.with(|control| {
                let application = application.borrow();
                let application = application.as_ref().expect("embedded application retained");
                let view = view.borrow();
                let view = view.as_ref().expect("demo view retained").clone();
                let control = control.borrow();
                let control = control.as_ref().expect("demo control retained").clone();
                application.update(|cx| {
                    let field = |name: &str| {
                        payload_json
                            .get(name)
                            .and_then(|v| v.as_str())
                            .map(str::to_owned)
                    };
                    if matches!(action.as_str(), "barHover" | "bar-hover") {
                        let pane =
                            PaneId::new(field("pane").unwrap_or_else(|| payload_text.clone()));
                        *control.bar_hover.lock().unwrap() =
                            hover_activity_bar(&view, &pane, cx).then_some(pane.0);
                        cx.refresh_windows();
                        return;
                    }
                    view.update(cx, |view, cx| {
                        let field = &field;
                        match action.as_str() {
                            "reset" => {
                                control.reset();
                                for (id, tree) in [
                                    ("default", default_workspace()),
                                    ("triple", triple_workspace()),
                                    ("stacked", stacked_workspace()),
                                ] {
                                    view.update_workspace_tree(&WorkspaceId(id.into()), tree, cx)
                                        .unwrap();
                                }
                                view.switch_workspace(&WorkspaceId("default".into()), cx);
                                view.update_model(cx, |model| {
                                    model.focus(&PaneId::new("1"));
                                });
                            }
                            "workspace" => {
                                let id = field("id").unwrap_or_else(|| payload_text.clone());
                                view.switch_workspace(&WorkspaceId(id), cx);
                            }
                            "focus" => {
                                let pane = field("pane").unwrap_or_else(|| payload_text.clone());
                                view.update_model(cx, |model| {
                                    model.focus(&PaneId::new(pane));
                                });
                            }
                            "activity" => {
                                let pane = field("pane").unwrap_or_else(|| {
                                    view.model()
                                        .focused()
                                        .map(|p| p.0.clone())
                                        .unwrap_or_default()
                                });
                                let activity = field("activity")
                                    .or_else(|| payload_json.as_str().map(str::to_owned))
                                    .unwrap_or_default();
                                view.update_model(cx, |model| {
                                    model.set_activity(
                                        &PaneId::new(pane),
                                        Some(ActivityId::new(activity)),
                                    );
                                });
                            }
                            "category" => {
                                let category =
                                    field("category").unwrap_or_else(|| payload_text.clone());
                                *control.selected_category.lock().unwrap() = Some(category.clone());
                                let first_activity = match category.as_str() {
                                    "0" => "1",
                                    "1" => "5",
                                    "2" => "10",
                                    "3" => "11",
                                    _ => "1",
                                };
                                if let Some(pane) = view.model().focused().cloned() {
                                    view.update_model(cx, |model| {
                                        model.set_activity(
                                            &pane,
                                            Some(ActivityId::new(first_activity)),
                                        );
                                    });
                                } else {
                                    cx.notify();
                                }
                            }
                            "focusBehavior" | "focus-behavior" => {
                                let value = field("value").unwrap_or_else(|| payload_text.clone());
                                view.set_focus_behavior(
                                    if value.eq_ignore_ascii_case("hover") {
                                        PaneFocusBehavior::Hover
                                    } else {
                                        PaneFocusBehavior::Click
                                    },
                                    cx,
                                );
                            }
                            "palette" => {
                                control.palette_open.store(true, Ordering::SeqCst);
                                *control.palette_query.lock().unwrap() =
                                    field("query").unwrap_or_else(|| payload_text.clone());
                                cx.notify();
                            }
                            "paletteClose" | "palette-close" => {
                                control.palette_open.store(false, Ordering::SeqCst);
                                cx.notify();
                            }
                            "split" => {
                                let pane = field("pane").unwrap_or_else(|| {
                                    view.model()
                                        .focused()
                                        .map(|p| p.0.clone())
                                        .unwrap_or_default()
                                });
                                let direction = if field("direction")
                                    .unwrap_or_default()
                                    .eq_ignore_ascii_case("vertical")
                                {
                                    SplitDirection::Vertical
                                } else {
                                    SplitDirection::Horizontal
                                };
                                let source = PaneId::new(pane);
                                if let Some(PaneNode::Leaf { data, .. }) =
                                    view.model().tree().find(&source)
                                {
                                    let data = data.clone();
                                    let sequence =
                                        control.split_counter.fetch_add(1, Ordering::SeqCst) + 1;
                                    view.update_model(cx, |model| {
                                        model.split(
                                            &source,
                                            direction,
                                            PaneId::new(format!("split-{sequence}")),
                                            DemoData {
                                                label: format!("{} (browser split)", data.label),
                                                ..data
                                            },
                                        );
                                    });
                                }
                            }
                            "drop" => {
                                let activity = ActivityId::new(
                                    field("activity").unwrap_or_else(|| "1".into()),
                                );
                                let destination =
                                    PaneId::new(field("destination").unwrap_or_else(|| "1".into()));
                                let edge = match field("edge")
                                    .unwrap_or_else(|| "right".into())
                                    .to_ascii_lowercase()
                                    .as_str()
                                {
                                    "left" => DropEdge::Left,
                                    "top" => DropEdge::Top,
                                    "bottom" => DropEdge::Bottom,
                                    "center" => DropEdge::Center,
                                    _ => DropEdge::Right,
                                };
                                let sequence =
                                    control.drop_counter.fetch_add(1, Ordering::SeqCst) + 1;
                                view.update_model(cx, |model| {
                                    model.drop_activity(
                                        &activity,
                                        &destination,
                                        edge,
                                        PaneId::new(format!("drop-{sequence}")),
                                        DemoData::pane(&format!("Activity {}", activity.0)),
                                    );
                                });
                            }
                            _ => {}
                        }
                    });
                    cx.refresh_windows();
                });
            })
        })
    });
    browser_test_state()
}

#[cfg(target_family = "wasm")]
fn install_browser_test_bridge() {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};
    let snapshot = Closure::<dyn Fn() -> String>::new(browser_test_state);
    js_sys::Reflect::set(
        &js_sys::global(),
        &JsValue::from_str("__mullionTestState"),
        snapshot.as_ref().unchecked_ref(),
    )
    .unwrap();
    snapshot.forget();
    let action = Closure::<dyn Fn(JsValue, JsValue) -> String>::new(browser_test_action);
    js_sys::Reflect::set(
        &js_sys::global(),
        &JsValue::from_str("__mullionTestAction"),
        action.as_ref().unchecked_ref(),
    )
    .unwrap();
    action.forget();
}

#[cfg(target_family = "wasm")]
fn main() {
    gpui_platform::web_init();
    let application = gpui_platform::application().run_embedded(launch);
    APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
    install_browser_test_bridge();
}
