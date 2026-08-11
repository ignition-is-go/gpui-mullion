//! GPUI actions and key-binding compilation for the portable command catalog.
//!
//! Action type names are an integration detail. Persisted references and command
//! palette entries should continue to use [`PaneCommand::id`] instead.

use std::{error::Error, fmt, rc::Rc};

use gpui::{Action, DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate};

use crate::{
    BalancePanes, ClosePane, FocusDown, FocusLeft, FocusRight, FocusUp, MullionKeymap, PaneCommand,
    PaneDirection, PaneLayout, PaneRotation, SplitDirection,
};

/// The key context installed by `MullionView` around pane content.
pub const MULLION_KEY_CONTEXT: &str = "Mullion";

gpui::actions!(
    mullion,
    [
        /// GPUI action corresponding to [`PaneCommand::FocusFirst`].
        FocusFirst,
        /// GPUI action corresponding to [`PaneCommand::FocusLast`].
        FocusLast,
        /// GPUI action for a left/right split.
        SplitPaneHorizontal,
        /// GPUI action for a top/bottom split.
        SplitPaneVertical,
        /// GPUI action that moves the focused pane left.
        MovePaneLeft,
        /// GPUI action that moves the focused pane right.
        MovePaneRight,
        /// GPUI action that moves the focused pane up.
        MovePaneUp,
        /// GPUI action that moves the focused pane down.
        MovePaneDown,
        /// GPUI action that swaps the focused pane with its left neighbor.
        SwapPaneLeft,
        /// GPUI action that swaps the focused pane with its right neighbor.
        SwapPaneRight,
        /// GPUI action that swaps the focused pane with its upper neighbor.
        SwapPaneUp,
        /// GPUI action that swaps the focused pane with its lower neighbor.
        SwapPaneDown,
        /// GPUI action that swaps with the next pane in layout order.
        SwapPaneNext,
        /// GPUI action that swaps with the previous pane in layout order.
        SwapPanePrevious,
        /// GPUI action that grows the focused pane toward the left.
        ResizePaneLeft,
        /// GPUI action that grows the focused pane toward the right.
        ResizePaneRight,
        /// GPUI action that grows the focused pane upward.
        ResizePaneUp,
        /// GPUI action that grows the focused pane downward.
        ResizePaneDown,
        /// GPUI action that arranges the focused pane's parent left/right.
        SetParentSplitHorizontal,
        /// GPUI action that arranges the focused pane's parent top/bottom.
        SetParentSplitVertical,
        /// GPUI action that toggles the focused pane's parent split axis.
        ToggleParentSplitDirection,
        /// GPUI action that rotates panes forward through layout slots.
        RotatePanesForward,
        /// GPUI action that rotates panes backward through layout slots.
        RotatePanesBackward,
        /// GPUI action that applies equal-width columns.
        ApplyEvenHorizontalLayout,
        /// GPUI action that applies equal-height rows.
        ApplyEvenVerticalLayout,
        /// GPUI action that places a large pane above secondary panes.
        ApplyMainHorizontalLayout,
        /// GPUI action that places a large pane left of secondary panes.
        ApplyMainVerticalLayout,
        /// GPUI action that applies a balanced, alternating grid.
        ApplyTiledLayout
    ]
);

/// Parameterized GPUI action for the dynamic, zero-based
/// [`PaneCommand::FocusIndex`] command.
///
/// It is intentionally declared with GPUI's `no_json` option: persisted
/// bindings should identify the portable command with [`PaneCommand::id`]
/// rather than relying on GPUI action JSON.
#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = mullion, no_json)]
pub struct FocusPane {
    /// Zero-based target in depth-first pane layout order.
    pub index: usize,
}

/// Converts a portable pane command into the GPUI action dispatched by views.
///
/// The nine action types historically exported by `view` are reused rather
/// than redeclared here. Static commands map to registered unit actions;
/// [`PaneCommand::FocusIndex`] maps to a [`FocusPane`] carrying its index.
pub fn action_for_command(command: PaneCommand) -> Box<dyn Action> {
    use PaneCommand::*;
    use PaneDirection::*;

    match command {
        Focus(Left) => Box::new(FocusLeft),
        Focus(Right) => Box::new(FocusRight),
        Focus(Up) => Box::new(FocusUp),
        Focus(Down) => Box::new(FocusDown),
        FocusNext => Box::new(crate::FocusNext),
        FocusPrevious => Box::new(crate::FocusPrevious),
        FocusFirst => Box::new(crate::FocusFirst),
        FocusLast => Box::new(crate::FocusLast),
        FocusIndex(index) => Box::new(FocusPane { index }),
        Split(SplitDirection::Horizontal) => Box::new(SplitPaneHorizontal),
        Split(SplitDirection::Vertical) => Box::new(SplitPaneVertical),
        Close => Box::new(ClosePane),
        Move(Left) => Box::new(MovePaneLeft),
        Move(Right) => Box::new(MovePaneRight),
        Move(Up) => Box::new(MovePaneUp),
        Move(Down) => Box::new(MovePaneDown),
        Swap(Left) => Box::new(SwapPaneLeft),
        Swap(Right) => Box::new(SwapPaneRight),
        Swap(Up) => Box::new(SwapPaneUp),
        Swap(Down) => Box::new(SwapPaneDown),
        SwapNext => Box::new(SwapPaneNext),
        SwapPrevious => Box::new(SwapPanePrevious),
        Resize(Left) => Box::new(ResizePaneLeft),
        Resize(Right) => Box::new(ResizePaneRight),
        Resize(Up) => Box::new(ResizePaneUp),
        Resize(Down) => Box::new(ResizePaneDown),
        SetParentSplitDirection(SplitDirection::Horizontal) => Box::new(SetParentSplitHorizontal),
        SetParentSplitDirection(SplitDirection::Vertical) => Box::new(SetParentSplitVertical),
        ToggleParentSplitDirection => Box::new(crate::ToggleParentSplitDirection),
        Balance => Box::new(BalancePanes),
        Rotate(PaneRotation::Forward) => Box::new(RotatePanesForward),
        Rotate(PaneRotation::Backward) => Box::new(RotatePanesBackward),
        ApplyLayout(PaneLayout::EvenHorizontal) => Box::new(ApplyEvenHorizontalLayout),
        ApplyLayout(PaneLayout::EvenVertical) => Box::new(ApplyEvenVerticalLayout),
        ApplyLayout(PaneLayout::MainHorizontal) => Box::new(ApplyMainHorizontalLayout),
        ApplyLayout(PaneLayout::MainVertical) => Box::new(ApplyMainVerticalLayout),
        ApplyLayout(PaneLayout::Tiled) => Box::new(ApplyTiledLayout),
        ToggleZoom => Box::new(crate::ToggleZoom),
    }
}

/// Recovers the portable command represented by a Mullion GPUI action.
///
/// Returns `None` for actions owned by hosts, GPUI, or other components.
pub fn command_for_action(action: &dyn Action) -> Option<PaneCommand> {
    use PaneCommand::*;
    use PaneDirection::*;

    macro_rules! unit {
        ($type:ty, $command:expr) => {
            if action.as_any().is::<$type>() {
                return Some($command);
            }
        };
    }

    unit!(FocusLeft, Focus(Left));
    unit!(FocusRight, Focus(Right));
    unit!(FocusUp, Focus(Up));
    unit!(FocusDown, Focus(Down));
    unit!(crate::FocusNext, FocusNext);
    unit!(crate::FocusPrevious, FocusPrevious);
    unit!(crate::FocusFirst, FocusFirst);
    unit!(crate::FocusLast, FocusLast);
    if let Some(action) = action.as_any().downcast_ref::<FocusPane>() {
        return Some(FocusIndex(action.index));
    }
    unit!(SplitPaneHorizontal, Split(SplitDirection::Horizontal));
    unit!(SplitPaneVertical, Split(SplitDirection::Vertical));
    unit!(ClosePane, Close);
    unit!(MovePaneLeft, Move(Left));
    unit!(MovePaneRight, Move(Right));
    unit!(MovePaneUp, Move(Up));
    unit!(MovePaneDown, Move(Down));
    unit!(SwapPaneLeft, Swap(Left));
    unit!(SwapPaneRight, Swap(Right));
    unit!(SwapPaneUp, Swap(Up));
    unit!(SwapPaneDown, Swap(Down));
    unit!(SwapPaneNext, SwapNext);
    unit!(SwapPanePrevious, SwapPrevious);
    unit!(ResizePaneLeft, Resize(Left));
    unit!(ResizePaneRight, Resize(Right));
    unit!(ResizePaneUp, Resize(Up));
    unit!(ResizePaneDown, Resize(Down));
    unit!(
        SetParentSplitHorizontal,
        SetParentSplitDirection(SplitDirection::Horizontal)
    );
    unit!(
        SetParentSplitVertical,
        SetParentSplitDirection(SplitDirection::Vertical)
    );
    unit!(
        crate::ToggleParentSplitDirection,
        ToggleParentSplitDirection
    );
    unit!(BalancePanes, Balance);
    unit!(RotatePanesForward, Rotate(PaneRotation::Forward));
    unit!(RotatePanesBackward, Rotate(PaneRotation::Backward));
    unit!(
        ApplyEvenHorizontalLayout,
        ApplyLayout(PaneLayout::EvenHorizontal)
    );
    unit!(
        ApplyEvenVerticalLayout,
        ApplyLayout(PaneLayout::EvenVertical)
    );
    unit!(
        ApplyMainHorizontalLayout,
        ApplyLayout(PaneLayout::MainHorizontal)
    );
    unit!(
        ApplyMainVerticalLayout,
        ApplyLayout(PaneLayout::MainVertical)
    );
    unit!(ApplyTiledLayout, ApplyLayout(PaneLayout::Tiled));
    unit!(crate::ToggleZoom, ToggleZoom);
    None
}

/// Returns the stable persisted reference for a command.
///
/// This deliberately delegates to [`PaneCommand::id`] rather than exposing
/// GPUI action type names, which are an integration detail.
pub fn action_reference_id(command: PaneCommand) -> String {
    command.id()
}

/// Failure while compiling a portable keymap into GPUI bindings.
#[derive(Debug)]
pub enum KeymapCompileError {
    /// GPUI rejected the supplied context predicate.
    InvalidContext(
        /// GPUI's human-readable predicate parse error.
        String,
    ),
    /// GPUI rejected a normalized key sequence from the portable keymap.
    InvalidKeystroke(
        /// The original GPUI key parser error.
        gpui::InvalidKeystrokeError,
    ),
}

impl fmt::Display for KeymapCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContext(error) => write!(f, "invalid GPUI key context: {error}"),
            Self::InvalidKeystroke(error) => write!(f, "invalid GPUI key sequence: {error}"),
        }
    }
}

impl Error for KeymapCompileError {}

/// Compiles a portable keymap into GPUI bindings under an explicit context.
///
/// Each normalized sequence is paired with [`action_for_command`]. Unless the
/// keymap explicitly captures editable targets, the compiled predicate also
/// excludes `MullionEditable` so pane shortcuts do not consume text input.
///
/// # Errors
///
/// Returns [`KeymapCompileError::InvalidContext`] when `context` cannot be
/// parsed as a GPUI predicate, or [`KeymapCompileError::InvalidKeystroke`] when
/// any normalized sequence is not a valid GPUI key sequence.
pub fn compile_keymap(
    keymap: &MullionKeymap,
    context: &str,
) -> Result<Vec<KeyBinding>, KeymapCompileError> {
    let context = if keymap.captures_editable_targets() {
        context.to_owned()
    } else {
        format!("({context}) && !MullionEditable")
    };
    let context = KeyBindingContextPredicate::parse(&context)
        .map_err(|error| KeymapCompileError::InvalidContext(error.to_string()))?;
    let context = Rc::new(context);

    keymap
        .normalized_sequences()
        .into_iter()
        .map(|(sequence, command)| {
            KeyBinding::load(
                &sequence,
                action_for_command(command),
                Some(context.clone()),
                false,
                None,
                &DummyKeyboardMapper,
            )
            .map_err(KeymapCompileError::InvalidKeystroke)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_static_command_has_one_distinct_round_tripping_action() {
        let catalog = PaneCommand::catalog();
        assert_eq!(catalog.len(), 37);
        let mut action_names = HashSet::new();
        for command in catalog {
            let action = action_for_command(command);
            assert_eq!(command_for_action(action.as_ref()), Some(command));
            assert!(
                action_names.insert(action.name()),
                "duplicate action for {command:?}"
            );
            assert_eq!(action_reference_id(command), command.id());
        }
        assert_eq!(action_names.len(), 37);
    }

    #[test]
    fn dynamic_focus_action_round_trips() {
        let command = PaneCommand::FocusIndex(41);
        let action = action_for_command(command);
        assert_eq!(command_for_action(action.as_ref()), Some(command));
    }

    #[test]
    fn direct_and_tmux_bindings_all_compile_and_round_trip() {
        for map in [MullionKeymap::mullion(), MullionKeymap::tmux()] {
            let expected = map.normalized_sequences();
            let compiled = compile_keymap(&map, MULLION_KEY_CONTEXT).unwrap();
            assert_eq!(compiled.len(), expected.len());
            for (binding, (_, command)) in compiled.iter().zip(expected) {
                assert_eq!(command_for_action(binding.action()), Some(command));
                assert!(binding.predicate().is_some());
            }
        }
    }

    #[test]
    fn editable_context_is_suppressed_unless_capture_is_explicit() {
        let suppressed = compile_keymap(&MullionKeymap::mullion(), MULLION_KEY_CONTEXT).unwrap();
        let captured = compile_keymap(
            &MullionKeymap::mullion().capture_editable_targets(true),
            MULLION_KEY_CONTEXT,
        )
        .unwrap();
        let suppressed = suppressed[0].predicate().unwrap().to_string();
        let captured = captured[0].predicate().unwrap().to_string();
        assert!(suppressed.contains("Mullion"));
        assert!(suppressed.contains("MullionEditable"));
        assert!(suppressed.contains('!'));
        assert_eq!(captured, MULLION_KEY_CONTEXT);
    }

    #[test]
    fn invalid_sequence_and_context_are_reported() {
        let map = MullionKeymap::unprefixed()
            .with_binding(crate::KeyChord::new("bad-key"), PaneCommand::FocusFirst);
        assert!(matches!(
            compile_keymap(&map, MULLION_KEY_CONTEXT),
            Err(KeymapCompileError::InvalidKeystroke(_))
        ));
        assert!(matches!(
            compile_keymap(&MullionKeymap::mullion(), "Mullion &&"),
            Err(KeymapCompileError::InvalidContext(_))
        ));
    }
}
