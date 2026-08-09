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

#[test]
fn static_catalog_metadata_matches_the_exhaustive_reference_golden() {
    use PaneCommand::*;
    use PaneDirection::*;
    let expected = [
        (
            Focus(Left),
            "mullion.focus.left",
            "Focus Pane Left",
            "Mullion · Focus",
            "Focus the nearest pane in this direction",
        ),
        (
            Focus(Right),
            "mullion.focus.right",
            "Focus Pane Right",
            "Mullion · Focus",
            "Focus the nearest pane in this direction",
        ),
        (
            Focus(Up),
            "mullion.focus.up",
            "Focus Pane Up",
            "Mullion · Focus",
            "Focus the nearest pane in this direction",
        ),
        (
            Focus(Down),
            "mullion.focus.down",
            "Focus Pane Down",
            "Mullion · Focus",
            "Focus the nearest pane in this direction",
        ),
        (
            FocusNext,
            "mullion.focus.next",
            "Focus Next Pane",
            "Mullion · Focus",
            "Cycle focus through the pane layout",
        ),
        (
            FocusPrevious,
            "mullion.focus.previous",
            "Focus Previous Pane",
            "Mullion · Focus",
            "Cycle focus through the pane layout",
        ),
        (
            FocusFirst,
            "mullion.focus.first",
            "Focus First Pane",
            "Mullion · Focus",
            "Focus a pane by layout order",
        ),
        (
            FocusLast,
            "mullion.focus.last",
            "Focus Last Pane",
            "Mullion · Focus",
            "Focus a pane by layout order",
        ),
        (
            Split(SplitDirection::Horizontal),
            "mullion.split.horizontal",
            "Split Pane Left/Right",
            "Mullion · Pane",
            "Create and focus a pane beside the focused pane",
        ),
        (
            Split(SplitDirection::Vertical),
            "mullion.split.vertical",
            "Split Pane Top/Bottom",
            "Mullion · Pane",
            "Create and focus a pane beside the focused pane",
        ),
        (
            Close,
            "mullion.close",
            "Close Focused Pane",
            "Mullion · Pane",
            "Close the focused pane and focus an adjacent pane",
        ),
        (
            Move(Left),
            "mullion.move.left",
            "Move Pane Left",
            "Mullion · Arrange",
            "Move the focused pane beside its directional neighbor",
        ),
        (
            Move(Right),
            "mullion.move.right",
            "Move Pane Right",
            "Mullion · Arrange",
            "Move the focused pane beside its directional neighbor",
        ),
        (
            Move(Up),
            "mullion.move.up",
            "Move Pane Up",
            "Mullion · Arrange",
            "Move the focused pane beside its directional neighbor",
        ),
        (
            Move(Down),
            "mullion.move.down",
            "Move Pane Down",
            "Mullion · Arrange",
            "Move the focused pane beside its directional neighbor",
        ),
        (
            Swap(Left),
            "mullion.swap.left",
            "Swap with Pane Left",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            Swap(Right),
            "mullion.swap.right",
            "Swap with Pane Right",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            Swap(Up),
            "mullion.swap.up",
            "Swap with Pane Up",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            Swap(Down),
            "mullion.swap.down",
            "Swap with Pane Down",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            SwapNext,
            "mullion.swap.next",
            "Swap with Next Pane",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            SwapPrevious,
            "mullion.swap.previous",
            "Swap with Previous Pane",
            "Mullion · Arrange",
            "Exchange panes without changing the split topology",
        ),
        (
            Resize(Left),
            "mullion.resize.left",
            "Grow Pane Left",
            "Mullion · Resize",
            "Grow the focused pane toward its nearest boundary",
        ),
        (
            Resize(Right),
            "mullion.resize.right",
            "Grow Pane Right",
            "Mullion · Resize",
            "Grow the focused pane toward its nearest boundary",
        ),
        (
            Resize(Up),
            "mullion.resize.up",
            "Grow Pane Up",
            "Mullion · Resize",
            "Grow the focused pane toward its nearest boundary",
        ),
        (
            Resize(Down),
            "mullion.resize.down",
            "Grow Pane Down",
            "Mullion · Resize",
            "Grow the focused pane toward its nearest boundary",
        ),
        (
            SetParentSplitDirection(SplitDirection::Horizontal),
            "mullion.parent-split.horizontal",
            "Set Parent Split Left/Right",
            "Mullion · Layout",
            "Set the focused pane's parent split axis",
        ),
        (
            SetParentSplitDirection(SplitDirection::Vertical),
            "mullion.parent-split.vertical",
            "Set Parent Split Top/Bottom",
            "Mullion · Layout",
            "Set the focused pane's parent split axis",
        ),
        (
            ToggleParentSplitDirection,
            "mullion.parent-split.toggle",
            "Toggle Parent Split Direction",
            "Mullion · Layout",
            "Flip the focused pane's parent split axis",
        ),
        (
            Balance,
            "mullion.layout.balance",
            "Balance Pane Splits",
            "Mullion · Layout",
            "Reset every split ratio to an equal half",
        ),
        (
            Rotate(PaneRotation::Forward),
            "mullion.rotate.forward",
            "Rotate Panes Forward",
            "Mullion · Arrange",
            "Rotate panes through the existing layout slots",
        ),
        (
            Rotate(PaneRotation::Backward),
            "mullion.rotate.backward",
            "Rotate Panes Backward",
            "Mullion · Arrange",
            "Rotate panes through the existing layout slots",
        ),
        (
            ApplyLayout(PaneLayout::EvenHorizontal),
            "mullion.layout.even-horizontal",
            "Apply Even Horizontal Layout",
            "Mullion · Layout",
            "Rebuild the split topology using a standard layout",
        ),
        (
            ApplyLayout(PaneLayout::EvenVertical),
            "mullion.layout.even-vertical",
            "Apply Even Vertical Layout",
            "Mullion · Layout",
            "Rebuild the split topology using a standard layout",
        ),
        (
            ApplyLayout(PaneLayout::MainHorizontal),
            "mullion.layout.main-horizontal",
            "Apply Main Horizontal Layout",
            "Mullion · Layout",
            "Rebuild the split topology using a standard layout",
        ),
        (
            ApplyLayout(PaneLayout::MainVertical),
            "mullion.layout.main-vertical",
            "Apply Main Vertical Layout",
            "Mullion · Layout",
            "Rebuild the split topology using a standard layout",
        ),
        (
            ApplyLayout(PaneLayout::Tiled),
            "mullion.layout.tiled",
            "Apply Tiled Layout",
            "Mullion · Layout",
            "Rebuild the split topology using a standard layout",
        ),
        (
            ToggleZoom,
            "mullion.zoom.toggle",
            "Toggle Focused Pane Zoom",
            "Mullion · View",
            "Temporarily fill Mullion with the focused pane",
        ),
    ];
    assert_eq!(PaneCommand::catalog().len(), 37);
    for (command, (expected_command, id, name, group, description)) in
        PaneCommand::catalog().into_iter().zip(expected)
    {
        assert_eq!(command, expected_command);
        assert_eq!(command.id(), id);
        assert_eq!(command.name(), name);
        assert_eq!(command.group().label(), group);
        assert_eq!(command.description(), description);
    }
}

#[test]
fn dynamic_focus_index_has_reference_metadata() {
    let command = PaneCommand::FocusIndex(41);
    assert_eq!(command.id(), "mullion.focus.index.41");
    assert_eq!(command.name(), "Focus Pane 42");
    assert_eq!(command.group().label(), "Mullion · Focus");
    assert_eq!(command.description(), "Focus a pane by layout order");
}

#[test]
fn execution_options_preserve_legacy_execution_and_configure_split_and_resize() {
    use gpui_mullion::PaneCommandExecutionOptions;
    use std::sync::Arc;
    let options = PaneCommandExecutionOptions::default().with_resize_step(0.2);
    let mut model = three_panes();
    assert_eq!(
        model.execute_with_options(PaneCommand::Split(SplitDirection::Horizontal), &options),
        Err(PaneCommandError::SplitUnavailable)
    );
    model
        .execute_with_options(PaneCommand::Resize(PaneDirection::Right), &options)
        .unwrap();
    let configured =
        options.with_split_factory(Arc::new(|_, _, _| Some((PaneId::new("new"), Data(9)))));
    model
        .execute_with_options(PaneCommand::Split(SplitDirection::Vertical), &configured)
        .unwrap();
    assert!(model.tree().contains(&PaneId::new("new")));
}
