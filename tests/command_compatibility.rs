use serde::{Deserialize, Serialize};

use gpui_mullion::{
    MullionModel, PaneCommand, PaneCommandError, PaneDirection, PaneId, PaneLayout, PaneNode,
    PaneRotation, SplitDirection,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Data(u8);

fn leaf(id: &str, value: u8) -> PaneNode<Data> {
    PaneNode::leaf(PaneId::new(id), Data(value))
}

fn three_panes() -> MullionModel<Data> {
    MullionModel::new(PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.5,
        first: Box::new(leaf("a", 1)),
        second: Box::new(PaneNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(leaf("b", 2)),
            second: Box::new(leaf("c", 3)),
        }),
    })
}

#[test]
fn pane_command_ids_match_the_reference_golden() {
    use PaneCommand::*;
    use PaneDirection::*;

    let cases = [
        (Focus(Left), "mullion.focus.left"),
        (Focus(Right), "mullion.focus.right"),
        (Focus(Up), "mullion.focus.up"),
        (Focus(Down), "mullion.focus.down"),
        (FocusNext, "mullion.focus.next"),
        (FocusPrevious, "mullion.focus.previous"),
        (FocusFirst, "mullion.focus.first"),
        (FocusLast, "mullion.focus.last"),
        (FocusIndex(0), "mullion.focus.index.0"),
        (FocusIndex(42), "mullion.focus.index.42"),
        (
            Split(SplitDirection::Horizontal),
            "mullion.split.horizontal",
        ),
        (Split(SplitDirection::Vertical), "mullion.split.vertical"),
        (Close, "mullion.close"),
        (Move(Left), "mullion.move.left"),
        (Move(Right), "mullion.move.right"),
        (Move(Up), "mullion.move.up"),
        (Move(Down), "mullion.move.down"),
        (Swap(Left), "mullion.swap.left"),
        (Swap(Right), "mullion.swap.right"),
        (Swap(Up), "mullion.swap.up"),
        (Swap(Down), "mullion.swap.down"),
        (SwapNext, "mullion.swap.next"),
        (SwapPrevious, "mullion.swap.previous"),
        (Resize(Left), "mullion.resize.left"),
        (Resize(Right), "mullion.resize.right"),
        (Resize(Up), "mullion.resize.up"),
        (Resize(Down), "mullion.resize.down"),
        (
            SetParentSplitDirection(SplitDirection::Horizontal),
            "mullion.parent-split.horizontal",
        ),
        (
            SetParentSplitDirection(SplitDirection::Vertical),
            "mullion.parent-split.vertical",
        ),
        (ToggleParentSplitDirection, "mullion.parent-split.toggle"),
        (Balance, "mullion.layout.balance"),
        (Rotate(PaneRotation::Forward), "mullion.rotate.forward"),
        (Rotate(PaneRotation::Backward), "mullion.rotate.backward"),
        (
            ApplyLayout(PaneLayout::EvenHorizontal),
            "mullion.layout.even-horizontal",
        ),
        (
            ApplyLayout(PaneLayout::EvenVertical),
            "mullion.layout.even-vertical",
        ),
        (
            ApplyLayout(PaneLayout::MainHorizontal),
            "mullion.layout.main-horizontal",
        ),
        (
            ApplyLayout(PaneLayout::MainVertical),
            "mullion.layout.main-vertical",
        ),
        (ApplyLayout(PaneLayout::Tiled), "mullion.layout.tiled"),
        (ToggleZoom, "mullion.zoom.toggle"),
    ];

    for (command, expected) in cases {
        assert_eq!(command.id(), expected, "id for {command:?}");
    }
}

#[test]
fn command_error_messages_match_the_reference_golden() {
    let cases = [
        (
            PaneCommandError::NoFocusedPane,
            "the layout has no focused pane",
        ),
        (
            PaneCommandError::NoNeighbor,
            "there is no pane in that direction",
        ),
        (
            PaneCommandError::SplitUnavailable,
            "no split-pane factory is configured",
        ),
        (
            PaneCommandError::SplitRefused,
            "the split-pane factory refused the split",
        ),
        (
            PaneCommandError::CannotCloseLastPane,
            "the last pane cannot be closed",
        ),
        (
            PaneCommandError::InvalidPaneIndex,
            "there is no pane at that index",
        ),
        (
            PaneCommandError::NotApplicable,
            "the command does not apply to this layout",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn closing_focused_middle_pane_prefers_traversal_successor() {
    let mut model = three_panes();
    assert!(model.focus(&PaneId::new("b")));

    assert_eq!(model.close(&PaneId::new("b")), Some(Data(2)));
    assert_eq!(model.focused(), Some(&PaneId::new("c")));
}

#[test]
fn closing_focused_last_pane_falls_back_to_predecessor() {
    let mut model = three_panes();
    assert!(model.focus(&PaneId::new("c")));

    assert_eq!(model.close(&PaneId::new("c")), Some(Data(3)));
    assert_eq!(model.focused(), Some(&PaneId::new("b")));
}

#[test]
fn execute_reports_specific_reference_errors() {
    let mut model = three_panes();
    let no_split = |_: &PaneId, _: SplitDirection, _: &Data| None;

    assert_eq!(
        model.execute(PaneCommand::FocusIndex(99), no_split),
        Err(PaneCommandError::InvalidPaneIndex)
    );
    assert_eq!(
        model.execute(PaneCommand::Focus(PaneDirection::Left), no_split),
        Err(PaneCommandError::NoNeighbor)
    );
    assert_eq!(
        model.execute(PaneCommand::Move(PaneDirection::Left), no_split),
        Err(PaneCommandError::NoNeighbor)
    );
    assert_eq!(
        model.execute(PaneCommand::Split(SplitDirection::Horizontal), no_split),
        Err(PaneCommandError::SplitRefused)
    );

    let mut single = MullionModel::new(leaf("only", 1));
    assert_eq!(
        single.execute(PaneCommand::Close, no_split),
        Err(PaneCommandError::CannotCloseLastPane)
    );
    assert_eq!(
        single.execute(PaneCommand::SwapNext, no_split),
        Err(PaneCommandError::NoNeighbor)
    );
    assert_eq!(
        single.execute(PaneCommand::ToggleParentSplitDirection, no_split),
        Err(PaneCommandError::NotApplicable)
    );
}
