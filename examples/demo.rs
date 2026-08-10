#[path = "demo_assets.rs"]
mod demo_assets;

use demo_assets::DemoAssets;
use gpui::{
    div, prelude::*, px, rgb, size, svg, App, Bounds, FontWeight, WindowBounds, WindowOptions,
};
use gpui::{Context, Entity};
use gpui_mullion::{
    register_key_bindings, Activity, ActivityBarHostConfig, ActivityBarSlots, ActivityCatalog,
    ActivityCategory, ActivityChrome, ActivityIcon, ActivityId, ActivityNode, CategoryChrome,
    CategoryId, DropEdge, FocusPresentation, MullionSettings, MullionStyles, MullionView,
    PaneFocusBehavior, PaneId, PaneNode, SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc, Mutex,
};

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

fn icon(name: &'static str) -> ActivityIcon {
    ActivityIcon::new(move |_, _| {
        svg()
            .path(format!("demo-icons/{name}.svg"))
            .size_full()
            .text_color(rgb(0xeeeeee))
            .into_any_element()
    })
}

fn content_shell(title: impl Into<gpui::SharedString>, detail: &'static str) -> gpui::Div {
    div()
        .size_full()
        .overflow_hidden()
        .px(px(16.))
        .pt(px(14.))
        .pb(px(16.))
        .bg(rgb(0x111111))
        .text_color(rgb(0xdddddd))
        .child(
            div()
                .mb(px(12.))
                .text_size(px(12.))
                .line_height(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0xbbbbbb))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(13.))
                .line_height(px(20.8))
                .text_color(rgb(0xbababa))
                .child(detail),
        )
}

fn section_label(label: &'static str) -> gpui::Div {
    div()
        .mt(px(16.))
        .mb(px(6.))
        .text_size(px(10.))
        .line_height(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x777777))
        .child(label)
}

fn files_content(data: &DemoData) -> gpui::AnyElement {
    let files = [
        ("src/components/pane_view.rs", "M", 0xe8ab53),
        ("src/components/mullion_root.rs", "M", 0xe8ab53),
        ("src/commands.rs", "M", 0xe8ab53),
        ("src/context.rs", "M", 0xe8ab53),
        ("src/focus.rs", "M", 0xe8ab53),
        ("src/settings.rs", "M", 0xe8ab53),
        ("src/tree.rs", "", 0x777777),
        ("src/theme.rs", "", 0x777777),
        ("examples/demo/src/main.rs", "M", 0xe8ab53),
        ("examples/demo/index.html", "M", 0xe8ab53),
        ("tests/focus_visual.rs", "A", 0x73c991),
        ("Cargo.toml", "", 0x777777),
        ("README.md", "M", 0xe8ab53),
    ];
    let summaries = [
        ("13", "FILES", 0xdddddd),
        ("76", "TESTS", 0xdddddd),
        ("✓", "WASM", 0x73c991),
    ];
    content_shell(
        format!("{} - FILES", data.label.to_uppercase()),
        "Working tree for the focused-pane presentation pass.",
    )
    .child(section_label("CHANGES"))
    .children(files.into_iter().map(|(path, status, color)| {
        div()
            .h(px(25.))
            .px(px(8.))
            .flex()
            .items_center()
            .text_size(px(13.))
            .child(div().flex_1().min_w_0().overflow_hidden().child(path))
            .child(
                div()
                    .ml(px(10.))
                    .font_family("monospace")
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(10.))
                    .text_color(rgb(color))
                    .child(status),
            )
    }))
    .child(
        div()
            .mt(px(12.))
            .flex()
            .gap(px(7.))
            .children(summaries.into_iter().map(|(value, label, color)| {
                div()
                    .flex_1()
                    .min_w_0()
                    .p(px(9.))
                    .border_1()
                    .border_color(rgb(0x222222))
                    .rounded(px(5.))
                    .bg(rgb(0x131313))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(color))
                            .child(value),
                    )
                    .child(
                        div()
                            .mt(px(3.))
                            .text_size(px(10.))
                            .text_color(rgb(0x707070))
                            .child(label),
                    )
            })),
    )
    .into_any_element()
}

fn editors_content() -> gpui::AnyElement {
    let editors = [
        ("pane_view.rs", "src/components", "428 lines", 0x75beff),
        ("mullion_root.rs", "src/components", "326 lines", 0xc586c0),
        ("main.rs", "examples/demo/src", "641 lines", 0xe8ab53),
        ("README.md", "documentation", "472 lines", 0x73c991),
    ];
    content_shell("OPEN EDITORS", "Four files are open across the focus-state iteration.")
        .child(section_label("WORKING SET"))
        .child(
            div().flex().flex_col().gap(px(8.)).children(editors.into_iter().map(
                |(name, path, meta, color)| {
                    div()
                        .h(px(51.))
                        .p(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .border_1()
                        .border_color(rgb(0x222222))
                        .rounded(px(5.))
                        .bg(rgb(0x131313))
                        .child(div().w(px(3.)).h_full().rounded(px(2.)).bg(rgb(color)))
                        .child(
                            div().flex_1().min_w_0()
                                .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child(name))
                                .child(div().mt(px(3.)).font_family("monospace").text_size(px(10.)).text_color(rgb(0x707070)).child(path)),
                        )
                        .child(div().font_family("monospace").text_size(px(10.)).text_color(rgb(0x777777)).child(meta))
                },
            )),
        )
        .child(section_label("CURRENT SELECTION"))
        .child(
            div()
                .mt(px(4.))
                .p(px(11.))
                .border_1()
                .border_color(rgb(0x202020))
                .rounded(px(5.))
                .bg(rgb(0x0b0b0b))
                .font_family("monospace")
                .text_size(px(11.))
                .line_height(px(18.15))
                .text_color(rgb(0x929292))
                .child("let focused = show_focus_indicator\n    && focused_pane.get() == pane_id;\n\nview! { <PaneView focused=focused /> }"),
        )
        .into_any_element()
}

fn timeline_content() -> gpui::AnyElement {
    let events = [
        (
            "09:45",
            "Release demo built",
            "Optimized WASM bundle generated successfully.",
            0x73c991,
        ),
        (
            "09:42",
            "Focus presentation updated",
            "One-pixel accent and geometry-aware internal edges.",
            0x75beff,
        ),
        (
            "09:38",
            "Test suite passed",
            "76 unit tests and two documentation tests completed.",
            0x73c991,
        ),
        (
            "09:31",
            "Compatibility default restored",
            "Existing consumers retain hover focus and unchanged chrome.",
            0xe8ab53,
        ),
        (
            "09:18",
            "Command catalog registered",
            "Focus, split, move, resize, layout, and zoom actions available.",
            0x75beff,
        ),
        (
            "09:05",
            "Workspace opened",
            "Three-pane main-and-stack layout restored from session state.",
            0x777777,
        ),
    ];
    content_shell(
        "TIMELINE",
        "Recent activity from the Mullion design session.",
    )
    .child(section_label("TODAY"))
    .children(
        events
            .into_iter()
            .enumerate()
            .map(|(index, (time, title, detail, color))| {
                div()
                    .min_h(px(50.))
                    .flex()
                    .child(
                        div()
                            .w(px(48.))
                            .pt(px(2.))
                            .font_family("monospace")
                            .text_size(px(10.))
                            .text_color(rgb(0x666666))
                            .child(time),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(9.))
                            .mr(px(10.))
                            .pt(px(3.))
                            .when(index < 5, |rail| {
                                rail.child(
                                    div()
                                        .absolute()
                                        .top(px(8.))
                                        .bottom(px(-8.))
                                        .left(px(4.))
                                        .w(px(1.))
                                        .bg(rgb(0x252525)),
                                )
                            })
                            .child(
                                div()
                                    .size(px(7.))
                                    .border_2()
                                    .border_color(rgb(color))
                                    .rounded_full()
                                    .bg(rgb(0x111111)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0xd5d5d5))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(11.))
                                    .line_height(px(14.85))
                                    .text_color(rgb(0x737373))
                                    .child(detail),
                            ),
                    )
            }),
    )
    .into_any_element()
}

fn keybindings_content() -> gpui::AnyElement {
    let bindings = [
        ("Alt + Arrow", "Focus in that direction"),
        ("Alt + Shift + Arrow", "Move focused pane"),
        ("Ctrl + Shift + Arrow", "Swap with a neighbor"),
        ("Ctrl + Alt + Arrow", "Resize toward a boundary"),
        ("Ctrl + Alt + Shift + →/↓", "New pane right / down"),
        ("Ctrl + Shift + Backspace", "Close focused pane"),
        ("Ctrl + Shift + Enter", "Toggle focused-pane zoom"),
        ("Ctrl + Alt + =", "Balance splits"),
        ("Ctrl + Alt + 1…5", "Apply a standard layout"),
    ];
    content_shell(
        "MULLION KEYBINDINGS",
        "Direct shortcuts—no leader or pane mode.",
    )
    .child(
        div()
            .mt(px(12.))
            .flex()
            .flex_col()
            .gap(px(7.))
            .children(bindings.into_iter().map(|(keys, action)| {
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(190.))
                            .px(px(6.))
                            .py(px(2.))
                            .border_1()
                            .border_color(rgb(0x333333))
                            .rounded(px(4.))
                            .bg(rgb(0x222222))
                            .font_family("monospace")
                            .text_size(px(12.))
                            .child(keys),
                    )
                    .child(
                        div()
                            .ml(px(12.))
                            .text_size(px(12.))
                            .text_color(rgb(0x888888))
                            .child(action),
                    )
            })),
    )
    .child(
        div()
            .mt(px(14.))
            .text_size(px(13.))
            .text_color(rgb(0x888888))
            .child("Use Ctrl/⌘+K to browse every Mullion command."),
    )
    .into_any_element()
}

fn activity_content(name: &'static str, _pane: &PaneId, data: &DemoData) -> gpui::AnyElement {
    match name {
        "Files" => files_content(data),
        "Open Editors" => editors_content(),
        "Timeline" => timeline_content(),
        "Search" => content_shell(
            format!("{} - SEARCH", data.label.to_uppercase()),
            "Type to search across files...",
        )
        .child(
            div()
                .mt(px(8.))
                .w_full()
                .px(px(8.))
                .py(px(6.))
                .bg(rgb(0x222222))
                .border_1()
                .border_color(rgb(0x333333))
                .rounded(px(3.))
                .text_size(px(13.))
                .text_color(rgb(0x888888))
                .child("Search..."),
        )
        .into_any_element(),
        "Keybindings" => keybindings_content(),
        _ => {
            content_shell(name.to_uppercase(), "This activity is a placeholder.").into_any_element()
        }
    }
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

type DemoClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App)>;

fn settings_activity(control: Arc<DemoControl>) -> ActivityNode<DemoData> {
    ActivityNode::Activity(Activity {
        id: ActivityId::new("9"),
        name: "Settings".into(),
        filter: settings,
        render: Arc::new(move |pane, _| {
            let current = control.focus_behavior();
            let click_control = control.clone();
            let hover_control = control.clone();
            let option = |label: &'static str,
                          description: &'static str,
                          selected: bool,
                          id: String,
                          on_click: DemoClickHandler| {
                div()
                    .id(id)
                    .w_full()
                    .max_w(px(480.))
                    .p(px(10.))
                    .px(px(12.))
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .border_1()
                    .border_color(rgb(0x333333))
                    .rounded(px(5.))
                    .cursor_pointer()
                    .child(
                        div()
                            .mt(px(2.))
                            .size(px(12.))
                            .border_1()
                            .border_color(if selected {
                                rgb(0x00a4ef)
                            } else {
                                rgb(0x888888)
                            })
                            .rounded_full()
                            .when(selected, |dot| {
                                dot.p(px(3.))
                                    .child(div().size_full().rounded_full().bg(rgb(0x00a4ef)))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xeeeeee))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .mt(px(3.))
                                    .text_size(px(12.))
                                    .line_height(px(16.8))
                                    .text_color(rgb(0x888888))
                                    .child(description),
                            ),
                    )
                    .on_click(on_click)
            };
            let click = option(
                "Click",
                "Focus a pane when it is clicked and keep focus there.",
                current == PaneFocusBehavior::Click,
                format!("focus-setting-click:{}", pane.0),
                Box::new(move |_, _, cx| {
                    click_control.focus_behavior.store(0, Ordering::SeqCst);
                    cx.refresh_windows();
                }),
            );
            let hover = option(
                "Hover",
                "Move focus whenever the pointer enters another pane.",
                current == PaneFocusBehavior::Hover,
                format!("focus-setting-hover:{}", pane.0),
                Box::new(move |_, _, cx| {
                    hover_control.focus_behavior.store(1, Ordering::SeqCst);
                    cx.refresh_windows();
                }),
            );
            content_shell(
                "PANE FOCUS BEHAVIOR",
                "Choose whether pointer hover or click changes the focused pane.",
            )
            .child(div().mt(px(14.)).flex().flex_col().gap(px(8.)).child(click).child(hover))
            .child(
                div()
                    .mt(px(14.))
                    .text_size(px(13.))
                    .text_color(rgb(0x888888))
                    .child("This control is rendered and persisted by the demo app from Mullion's headless setting descriptor."),
            )
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
    for (id, asset) in [
        ("1", "description"),
        ("2", "article"),
        ("3", "timeline"),
        ("4", "list"),
        ("5", "search"),
        ("6", "find_replace"),
        ("7", "bookmarks"),
        ("8", "code"),
        ("9", "settings"),
        ("10", "palette"),
        ("11", "keyboard"),
        ("12", "extension"),
    ] {
        let mut chrome = ActivityChrome::new(icon(asset));
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
    for (id, asset) in [
        ("0", "folder"),
        ("1", "edit_note"),
        ("2", "settings"),
        ("3", "tune"),
    ] {
        catalog.insert_category_chrome(CategoryId::new(id), CategoryChrome::new(icon(asset)));
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
        *self.selected_category.lock().unwrap() = None;
        *self.bar_hover.lock().unwrap() = None;
    }
}

fn host_config() -> ActivityBarHostConfig<DemoData> {
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
            .with_app_icon(icon("apps"))
            .with_leading(hairline())
            .with_trailing(hairline())
            .with_pane_accessory(move |_, _, _, _| {
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
    // The reference demo supplies `--ml-unfocused-pane-color: rgb(3 9 14)`;
    // this is a fixture override rather than a change to Mullion's theme.
    styles.pane.unfocused_wash = rgb(0x03090e).into();
    styles.workspace_switcher.gap = px(1.);
    styles.workspace_switcher.font_size = px(11.);
    styles.workspace_switcher.line_height = px(13.);
    styles.workspace_switcher.vertical_padding = px(2.);
    styles.workspace_switcher.horizontal_padding = px(8.);
    styles.workspace_switcher.border_radius = px(2.);
    styles.workspace_switcher.background = gpui::transparent_black();
    styles.workspace_switcher.active_background = rgb(0x222222).into();
    styles.workspace_switcher.active_text = styles.pane.text;
    styles
}

struct DemoRoot {
    view: Entity<MullionView<DemoData>>,
}

impl gpui::Render for DemoRoot {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let (active, workspaces) = {
            let view = self.view.read(cx);
            let set = view
                .workspaces()
                .expect("canonical demo owns a workspace set");
            (set.active.clone(), set.workspaces.clone())
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.view.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(1.))
                    .px(px(4.))
                    .py(px(2.))
                    .border_t_1()
                    .border_color(rgb(0x1a1a1a))
                    .bg(rgb(0x0e0e0e))
                    .children(
                        workspaces
                            .into_iter()
                            .enumerate()
                            .map(|(index, workspace)| {
                                let selected = workspace.id == active;
                                let id = workspace.id;
                                let view = self.view.clone();
                                div()
                                    .id(format!("demo-workspace:{}", id.0))
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(2.))
                                    .cursor_pointer()
                                    .font_family("monospace")
                                    .text_size(px(11.))
                                    .line_height(px(13.))
                                    .text_color(if selected {
                                        rgb(0xeeeeee)
                                    } else {
                                        rgb(0x888888)
                                    })
                                    .when(selected, |tab| tab.bg(rgb(0x222222)))
                                    .child(format!("{}", index + 1))
                                    .on_click(move |_, _, cx| {
                                        view.update(cx, |view, cx| {
                                            view.switch_workspace(&id, cx);
                                        });
                                    })
                            }),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .px(px(6.))
                            .py(px(2.))
                            .font_family("monospace")
                            .text_size(px(11.))
                            .line_height(px(13.))
                            .text_color(rgb(0x888888))
                            .child("Alt+Arrow · focus   Ctrl/⌘+K · all commands"),
                    ),
            )
    }
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
                    .with_workspace_switcher_visible(false)
                    .with_activity_bar_host(host_config())
            });
            gpui_mullion::install_command_palette_for_view(&view, window, cx);
            view.read(cx).focus_handle().clone().focus(window, cx);
            #[cfg(target_family = "wasm")]
            {
                TEST_VIEW.with(|slot| *slot.borrow_mut() = Some(view.clone()));
                TEST_CONTROL.with(|slot| *slot.borrow_mut() = Some(control.clone()));
            }
            cx.new(|_| DemoRoot { view })
        },
    )
    .unwrap();
    cx.activate(true);
    #[cfg(target_family = "wasm")]
    cx.refresh_windows();
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    gpui_platform::application()
        .with_assets(DemoAssets)
        .run(launch);
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
                let palette = view
                    .command_palette()
                    .expect("demo command palette installed")
                    .read(cx);
                let palette_state = palette.state();
                let palette_open = palette_state.is_open();
                let palette_query = palette_state.query().to_owned();
                let palette_results = palette_state
                    .results(&palette.registry().commands())
                    .into_iter()
                    .take(12)
                    .map(|result| result.entry.id)
                    .collect();
                let bar_hover = tree
                    .leaf_ids()
                    .into_iter()
                    .find(|pane| view.activity_bar_is_expanded(pane))
                    .map(|pane| pane.0);
                serde_json::to_string(&BrowserTestState {
                    active_workspace: view.workspaces().unwrap().active.clone(),
                    tree,
                    focused: view.model().focused().cloned(),
                    zoomed: view.model().zoomed().cloned(),
                    active_activities,
                    focus_behavior: control.focus_behavior(),
                    palette_open,
                    palette_query,
                    palette_results,
                    selected_category: control.selected_category.lock().unwrap().clone(),
                    bar_hover,
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
                        view.update(cx, |view, cx| {
                            view.set_activity_bar_hovered(&pane, true, cx);
                        });
                        *control.bar_hover.lock().unwrap() = view
                            .read(cx)
                            .activity_bar_is_expanded(&pane)
                            .then_some(pane.0);
                        cx.refresh_windows();
                        return;
                    }
                    if matches!(
                        action.as_str(),
                        "palette" | "paletteClose" | "palette-close"
                    ) {
                        let palette = view
                            .read(cx)
                            .command_palette()
                            .expect("demo command palette installed")
                            .clone();
                        let window = cx.active_window().expect("demo window is active");
                        window
                            .update(cx, |_, window, cx| {
                                palette.update(cx, |palette, cx| {
                                    if action == "palette" {
                                        palette.open(window, cx);
                                        palette.set_query(
                                            field("query").unwrap_or_else(|| payload_text.clone()),
                                            cx,
                                        );
                                    } else {
                                        palette.close(window, cx);
                                    }
                                });
                            })
                            .expect("demo window is alive");
                        cx.refresh_windows();
                        return;
                    }
                    if action == "reset" {
                        let palette = view
                            .read(cx)
                            .command_palette()
                            .expect("demo command palette installed")
                            .clone();
                        let window = cx.active_window().expect("demo window is active");
                        window
                            .update(cx, |_, window, cx| {
                                palette.update(cx, |palette, cx| palette.close(window, cx));
                            })
                            .expect("demo window is alive");
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
    let application = gpui_platform::application()
        .with_assets(DemoAssets)
        .run_embedded(launch);
    APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
    install_browser_test_bridge();
}
