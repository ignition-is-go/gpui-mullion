use gpui_mullion::{
    gpui::{AppContext, IntoElement},
    prelude::*,
};

#[test]
fn prelude_supports_the_documented_core_vocabulary() {
    let pane = PaneId::new("pane");
    let activity = ActivityId::new("activity");
    let category = CategoryId::new("category");
    let tree = PaneNode::leaf_with_activity(pane.clone(), activity.clone(), String::new());
    let workspace = Workspace::new("workspace", "Workspace", tree.clone());
    let set = WorkspaceSet::try_new("workspace".into(), vec![workspace]).unwrap();
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
    let look = MullionAppearance::system();
    assert!(matches!(
        look,
        MullionAppearance::Mode(MullionThemeMode::System)
    ));
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
