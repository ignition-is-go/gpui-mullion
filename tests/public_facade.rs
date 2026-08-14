use gpui_mullion::{
    gpui::{AppContext, IntoElement},
    prelude::*,
};

#[test]
fn public_facade_exposes_direct_global_theme_api() {
    fn assert_active_theme<T: ActiveMullionTheme>() {}
    assert_active_theme::<gpui_mullion::gpui::App>();
    let theme = MullionTheme::light();
    assert_eq!(theme.root.background, theme.background);
}

#[gpui_mullion::gpui::test]
fn installed_theme_preserves_snapshot_identity(cx: &mut gpui_mullion::gpui::TestAppContext) {
    let snapshot = std::sync::Arc::new(MullionTheme::dark());
    let installed = std::sync::Arc::clone(&snapshot);
    cx.update(|app| set_mullion_theme(app, installed));
    cx.update(|app| assert!(std::sync::Arc::ptr_eq(app.mullion_theme(), &snapshot)));
}

#[test]
fn prelude_supports_the_documented_core_vocabulary() {
    let pane = PaneId::new("pane");
    let activity = ActivityId::new("activity");
    let category = CategoryId::new("category");
    let tree = PaneNode::leaf_with_activity(pane.clone(), activity.clone(), String::new());
    let workspace = Workspace::new("workspace", "Workspace", tree.clone());
    let set = WorkspaceSet::try_new("workspace".into(), vec![workspace]).unwrap();
    let _controls: WorkspaceControls<String> = WorkspaceControls::editable()
        .with_rename_enabled(true)
        .on_changed(|_, _| {});
    let category_node: ActivityNode<String> = ActivityNode::Category(ActivityCategory::new(
        category,
        "Category",
        gpui_mullion::gpui::rgb(0).into(),
        Vec::new(),
    ));

    assert_eq!(tree.leaf_ids(), vec![pane]);
    assert_eq!(set.active().unwrap().id, WorkspaceId::new("workspace"));
    assert!(matches!(category_node, ActivityNode::Category(_)));
    assert_eq!(PaneFocusBehavior::default(), PaneFocusBehavior::Hover);
    let _ = FocusPresentation::default();
    let _ = MullionConfig::default();
    let theme = MullionTheme::light();
    assert_eq!(theme.root.background, theme.background);
}

#[gpui_mullion::gpui::test]
fn fallible_view_constructor_validates_tree_and_flat_catalog(
    cx: &mut gpui_mullion::gpui::TestAppContext,
) {
    let checked = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = checked.clone();
    cx.new(move |cx| {
        let invalid_tree = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: f64::NAN,
            first: Box::new(PaneNode::leaf("a".into(), String::new())),
            second: Box::new(PaneNode::leaf("b".into(), String::new())),
        };
        assert!(matches!(
            MullionView::try_new(invalid_tree, vec![], cx),
            Err(gpui_mullion::MullionViewConstructionError::Tree(_))
        ));

        let duplicate = || {
            Activity::new("duplicate", "Duplicate", |_, _: &String| {
                gpui_mullion::gpui::div().into_any_element()
            })
        };
        assert!(matches!(
            MullionView::try_new(
                PaneNode::leaf("valid".into(), String::new()),
                vec![
                    ActivityNode::Activity(duplicate()),
                    ActivityNode::Activity(duplicate())
                ],
                cx,
            ),
            Err(gpui_mullion::MullionViewConstructionError::Catalog(_))
        ));
        observed.set(true);
        MullionView::new(PaneNode::leaf("valid".into(), String::new()), vec![], cx)
    });
    assert!(checked.get());
}

#[gpui_mullion::gpui::test]
fn workspace_switcher_is_a_public_controlled_element(cx: &mut gpui_mullion::gpui::TestAppContext) {
    let view = cx.new(|cx| {
        MullionView::try_new_with_workspaces(
            WorkspaceSet::try_new(
                WorkspaceId::new("one"),
                vec![Workspace::new(
                    "one",
                    "One",
                    PaneNode::leaf(PaneId::new("pane"), String::new()),
                )],
            )
            .unwrap(),
            vec![],
            cx,
        )
        .unwrap()
        .with_workspace_switcher_visible(false)
    });
    fn accepts_element(_: impl gpui_mullion::gpui::IntoElement) {}
    accepts_element(WorkspaceSwitcher::new(view));
}
