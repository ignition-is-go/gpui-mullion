use gpui::{div, prelude::*, SharedString, TestAppContext};
use gpui_mullion::{
    activity_palette_entries, command_palette_for_view, Activity, ActivityCatalog, ActivityId,
    ActivityNode, MullionView, PaletteInvocation, PaneCommand, PaneId, PaneNode, SplitDirection,
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
    let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, vec![activity], cx));
    let palette = cx.update(|_, app| command_palette_for_view(&view, app));
    cx.run_until_parked();

    let ids = cx.update(|_, app| {
        palette
            .read(app)
            .registry()
            .commands()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
    });
    assert!(ids.contains(&"mullion.focus.index.1".to_string()));
    assert!(ids.contains(&"mullion.activity.left.files".to_string()));

    view.update(cx, |view, cx| {
        view.invoke_palette(
            PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(1)),
            cx,
        )
        .unwrap();
        assert_eq!(view.model().focused(), Some(&PaneId::new("right")));
        view.invoke_palette(
            PaletteInvocation::SelectActivity {
                pane: PaneId::new("right"),
                activity: ActivityId::new("files"),
            },
            cx,
        )
        .unwrap();
        let PaneNode::Leaf {
            active_activity, ..
        } = view.model().tree().find(&PaneId::new("right")).unwrap()
        else {
            panic!("right pane is a leaf")
        };
        assert_eq!(active_activity.as_ref(), Some(&ActivityId::new("files")));
    });
}
