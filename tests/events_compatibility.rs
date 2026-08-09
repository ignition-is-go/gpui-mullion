use gpui_mullion::{
    ActivityId, DropEdge, MullionModel, PaneEvent, PaneId, PaneNode, PaneValidationError,
    SplitDirection,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Data(u8);

fn leaf(id: &str, data: u8) -> PaneNode<Data> {
    PaneNode::leaf(PaneId::new(id), Data(data))
}

fn two_panes() -> PaneNode<Data> {
    PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.5,
        first: Box::new(leaf("a", 1)),
        second: Box::new(leaf("b", 2)),
    }
}

#[test]
fn split_trace_is_specific_transient_then_snapshot() {
    let mut model = MullionModel::new(two_panes());
    let target = PaneId::new("a");
    let new_id = PaneId::new("c");

    assert!(model.split(&target, SplitDirection::Vertical, new_id.clone(), Data(3),));
    let snapshot = model.snapshot();
    assert_eq!(
        model.take_events(),
        vec![
            PaneEvent::Split {
                target,
                direction: SplitDirection::Vertical,
                new_id: new_id.clone(),
                new_data: Data(3),
            },
            PaneEvent::FocusChanged { pane: Some(new_id) },
            PaneEvent::TreeChanged { tree: snapshot },
        ]
    );
}

#[test]
fn balance_emits_every_exact_ratio_before_snapshot() {
    let tree = PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.2,
        first: Box::new(leaf("a", 1)),
        second: Box::new(PaneNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.8,
            first: Box::new(leaf("b", 2)),
            second: Box::new(leaf("c", 3)),
        }),
    };
    let mut model = MullionModel::new(tree);

    assert!(model.balance());
    let snapshot = model.snapshot();
    assert_eq!(
        model.take_events(),
        vec![
            PaneEvent::Resized {
                split_key: PaneId::new("b"),
                ratio: 0.5,
            },
            PaneEvent::Resized {
                split_key: PaneId::new("c"),
                ratio: 0.5,
            },
            PaneEvent::TreeChanged { tree: snapshot },
        ]
    );
}

#[test]
fn activity_drop_trace_contains_host_values_and_focuses_last() {
    let mut model = MullionModel::new(two_panes());
    let activity = ActivityId::new("logs");
    let destination = PaneId::new("b");
    let new_id = PaneId::new("logs-pane");

    assert!(model.drop_activity(
        &activity,
        &destination,
        DropEdge::Left,
        new_id.clone(),
        Data(9),
    ));
    let snapshot = model.snapshot();
    assert_eq!(model.focused(), Some(&new_id));
    assert!(matches!(
        snapshot.find(&new_id),
        Some(PaneNode::Leaf {
            active_activity: Some(active),
            data: Data(9),
            ..
        }) if active == &activity
    ));
    assert_eq!(
        model.take_events(),
        vec![
            PaneEvent::ActivityDropped {
                activity,
                destination,
                edge: DropEdge::Left,
                new_id: new_id.clone(),
                new_data: Data(9),
            },
            PaneEvent::TreeChanged { tree: snapshot },
            PaneEvent::FocusChanged { pane: Some(new_id) },
        ]
    );
}

#[test]
fn rejected_activity_drops_are_atomic_and_silent() {
    let mut model = MullionModel::new(two_panes());
    let original = model.snapshot();
    let activity = ActivityId::new("logs");

    assert!(!model.drop_activity(
        &activity,
        &PaneId::new("missing"),
        DropEdge::Right,
        PaneId::new("c"),
        Data(3),
    ));
    assert!(!model.drop_activity(
        &activity,
        &PaneId::new("a"),
        DropEdge::Right,
        PaneId::new("b"),
        Data(3),
    ));
    assert_eq!(model.snapshot(), original);
    assert!(model.take_events().is_empty());
}

#[test]
fn upstream_replacement_reconciles_view_state_without_echo() {
    let mut model = MullionModel::new(two_panes());
    assert!(model.focus(&PaneId::new("b")));
    assert!(model.toggle_zoom());
    model.take_events();

    model.set_tree(leaf("c", 3));
    assert_eq!(model.focused(), Some(&PaneId::new("c")));
    assert_eq!(model.zoomed(), None);
    assert_eq!(
        model.take_events(),
        vec![
            PaneEvent::FocusChanged {
                pane: Some(PaneId::new("c")),
            },
            PaneEvent::ZoomChanged { pane: None },
        ]
    );

    // A surviving focus/zoom pair needs no transient reconciliation event.
    assert!(model.toggle_zoom());
    model.take_events();
    model.set_tree(PaneNode::leaf_with_activity(
        PaneId::new("c"),
        ActivityId::new("updated"),
        Data(4),
    ));
    assert!(model.take_events().is_empty());
}

#[test]
fn local_replacement_has_an_explicit_snapshot_event() {
    let mut model = MullionModel::new(two_panes());
    let replacement = leaf("c", 3);
    model.replace_tree(replacement.clone());
    assert_eq!(
        model.take_events(),
        vec![
            PaneEvent::FocusChanged {
                pane: Some(PaneId::new("c")),
            },
            PaneEvent::TreeChanged { tree: replacement },
        ]
    );
}

#[test]
fn invalid_upstream_replacement_is_atomic_and_silent() {
    let mut model = MullionModel::new(two_panes());
    let original = model.snapshot();
    let duplicate = PaneNode::Split {
        direction: SplitDirection::Vertical,
        ratio: 0.5,
        first: Box::new(leaf("duplicate", 1)),
        second: Box::new(leaf("duplicate", 2)),
    };

    assert!(matches!(
        model.try_set_tree(duplicate),
        Err(PaneValidationError::DuplicatePaneId { .. })
    ));
    assert_eq!(model.snapshot(), original);
    assert!(model.take_events().is_empty());
}

#[test]
fn persistence_and_transient_event_classes_are_explicit() {
    let persistent = PaneEvent::<Data>::ActivityDropped {
        activity: ActivityId::new("activity"),
        destination: PaneId::new("a"),
        edge: DropEdge::Bottom,
        new_id: PaneId::new("new"),
        new_data: Data(7),
    };
    let transient = PaneEvent::<Data>::FocusChanged {
        pane: Some(PaneId::new("a")),
    };

    assert!(persistent.is_persistence());
    assert!(!persistent.is_transient());
    assert!(transient.is_transient());
    assert!(!transient.is_persistence());

    // The restored variant uses the existing externally-tagged serde shape.
    assert_eq!(
        serde_json::to_value(persistent).unwrap(),
        serde_json::json!({
            "ActivityDropped": {
                "activity": "activity",
                "destination": "a",
                "edge": "Bottom",
                "new_id": "new",
                "new_data": 7
            }
        })
    );
}
