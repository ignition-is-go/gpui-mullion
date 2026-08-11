use gpui::{div, prelude::*, SharedString, TestAppContext};
use gpui_mullion::{
    activity_palette_entries, install_command_palette_for_view, Activity, ActivityCatalog,
    ActivityId, ActivityNode, MullionView, PaneId, PaneNode, SplitDirection,
};
use std::sync::Arc;

fn visible(_: &bool) -> bool {
    true
}

#[gpui::test]
fn shared_widget_tracks_dynamic_entries_and_invokes_mullion(cx: &mut TestAppContext) {
    let activity = ActivityNode::Activity(Activity {
        id: ActivityId::new("files"),
        name: SharedString::from("Files"),
        filter: visible,
        render: Arc::new(|_, _| div().into_any_element()),
    });
    let catalog = ActivityCatalog::new(vec![activity.clone()]);
    let projected = activity_palette_entries(&catalog, &PaneId::new("left"), &true);
    assert_eq!(projected[0].id, "mullion.activity.left.files");

    let tree = PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.5,
        first: Box::new(PaneNode::leaf(PaneId::new("left"), true)),
        second: Box::new(PaneNode::leaf(PaneId::new("right"), true)),
    };
    let (view, cx) = cx.add_window_view(move |_, cx| {
        MullionView::new(tree, vec![activity], cx).with_focused_pane_commands(|pane, _| {
            vec![gpui_command_palette::Command::new(
                format!("host.focused.{}", pane.as_ref()),
                format!("Focused {}", pane.as_ref()),
                || {},
            )]
        })
    });
    let palette = cx.update(|window, app| install_command_palette_for_view(&view, window, app));
    cx.run_until_parked();

    cx.update(|window, app| {
        palette.update(app, |palette, cx| {
            palette.open(window, cx);
            palette.set_query("files", cx);
        });
    });
    assert!(palette.read_with(cx, |palette, _| palette.state().is_open()));
    assert_eq!(
        palette.read_with(cx, |palette, _| palette.state().query().to_owned()),
        "files"
    );
    cx.update(|window, app| palette.update(app, |palette, cx| palette.close(window, cx)));
    assert!(!palette.read_with(cx, |palette, _| palette.state().is_open()));
    assert_eq!(
        palette.read_with(cx, |palette, _| palette.state().query().to_owned()),
        ""
    );

    let commands = cx.update(|_, app| palette.read(app).registry().commands());
    assert!(commands
        .iter()
        .any(|entry| entry.id == "mullion.activity.left.files"));
    assert!(commands.iter().any(|entry| entry.id == "host.focused.left"));
    let focus_right = commands
        .iter()
        .find(|entry| entry.id == "mullion.focus.pane")
        .unwrap()
        .resolve_children()
        .unwrap()
        .into_iter()
        .find(|entry| entry.id == "mullion.focus.pane.right")
        .unwrap();
    cx.update(|window, app| focus_right.execute_in(window, app));
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.model().focused().cloned()),
        Some(PaneId::new("right"))
    );

    let commands = cx.update(|_, app| palette.read(app).registry().commands());
    assert!(commands
        .iter()
        .any(|entry| entry.id == "mullion.activity.right.files"));
    assert!(commands
        .iter()
        .any(|entry| entry.id == "host.focused.right"));
    assert!(!commands.iter().any(|entry| entry.id == "host.focused.left"));
    assert!(!commands
        .iter()
        .any(|entry| entry.id == "mullion.activity.left.files"));
    let select_files = commands
        .into_iter()
        .find(|entry| entry.id == "mullion.activity.right.files")
        .unwrap();
    cx.update(|window, app| select_files.execute_in(window, app));
    cx.run_until_parked();

    view.read_with(cx, |view, _| {
        let PaneNode::Leaf {
            active_activity, ..
        } = view.model().tree().find(&PaneId::new("right")).unwrap()
        else {
            panic!("right pane is a leaf")
        };
        assert_eq!(active_activity.as_ref(), Some(&ActivityId::new("files")));
    });
}

#[gpui::test]
fn shared_attachment_injects_without_rendering_palette_inside_mullion(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        MullionView::new(PaneNode::leaf(PaneId::new("only"), true), Vec::new(), cx)
    });
    let palette = cx.new(gpui_command_palette::CommandPalette::<()>::new);
    let _binding =
        cx.update(|_, app| gpui_mullion::attach_command_palette(&view, palette.clone(), app));
    cx.run_until_parked();

    assert!(palette
        .read_with(cx, |palette, _| palette.registry().commands())
        .iter()
        .any(|command| command.id == "mullion.layout.balance"));
    cx.update(|window, app| palette.update(app, |palette, cx| palette.open(window, cx)));
    cx.run_until_parked();
    assert!(cx.debug_bounds("command-palette-dialog").is_none());
}

#[gpui::test]
fn installed_global_toggle_routes_once(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        MullionView::new(PaneNode::leaf(PaneId::new("only"), true), Vec::new(), cx)
    });
    let palette = cx.update(|window, app| {
        let palette = install_command_palette_for_view(&view, window, app);
        window.activate_window();
        palette
    });
    cx.run_until_parked();

    cx.dispatch_action(gpui_command_palette::ToggleCommandPalette);
    cx.run_until_parked();
    assert!(palette.read_with(cx, |palette, _| palette.state().is_open()));

    cx.dispatch_action(gpui_command_palette::ToggleCommandPalette);
    cx.run_until_parked();
    assert!(!palette.read_with(cx, |palette, _| palette.state().is_open()));
}

#[gpui::test]
fn installed_palette_accepts_text_and_arrow_navigation(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        MullionView::new(PaneNode::leaf(PaneId::new("only"), true), Vec::new(), cx)
    });
    let palette = cx.update(|window, app| {
        let palette = install_command_palette_for_view(&view, window, app);
        window.activate_window();
        palette.update(app, |palette, cx| palette.open(window, cx));
        palette
    });
    cx.run_until_parked();

    cx.simulate_keystrokes("f");
    assert_eq!(
        palette.read_with(cx, |palette, _| palette.state().query().to_owned()),
        "f"
    );

    assert_eq!(
        palette.read_with(cx, |palette, _| palette.state().selected_index()),
        0
    );
    cx.simulate_keystrokes("down");
    assert_eq!(
        palette.read_with(cx, |palette, _| palette.state().selected_index()),
        1
    );
}

#[gpui::test]
fn detaching_palette_drops_only_mullion_owned_registrations(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        MullionView::new(PaneNode::leaf(PaneId::new("only"), true), Vec::new(), cx)
    });
    let palette = cx.update(|window, app| install_command_palette_for_view(&view, window, app));
    cx.run_until_parked();

    let registry = palette.read_with(cx, |palette, _| palette.registry().clone());
    let _host_registration = registry.register(gpui_command_palette::Command::new(
        "host.command",
        "Host Command",
        || {},
    ));
    assert!(registry
        .commands()
        .iter()
        .any(|entry| entry.id.starts_with("mullion.")));

    view.update(cx, |view, cx| view.set_command_palette(None, cx));
    cx.run_until_parked();

    let remaining = registry.commands();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "host.command");
}
