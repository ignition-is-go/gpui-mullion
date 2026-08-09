use crate::{PaneDirection, PaneLayout, PaneRotation, SplitDirection};
use serde::{Deserialize, Serialize};

/// Focus-relative command understood by every frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneCommand {
    Focus(PaneDirection),
    FocusNext,
    FocusPrevious,
    FocusFirst,
    FocusLast,
    FocusIndex(usize),
    Split(SplitDirection),
    Close,
    Move(PaneDirection),
    Swap(PaneDirection),
    SwapNext,
    SwapPrevious,
    Resize(PaneDirection),
    SetParentSplitDirection(SplitDirection),
    ToggleParentSplitDirection,
    Balance,
    Rotate(PaneRotation),
    ApplyLayout(PaneLayout),
    ToggleZoom,
}

impl PaneCommand {
    /// Stable id suitable for command registries.
    ///
    /// The spelling is intentionally explicit rather than derived from
    /// `Debug`, so changing a Rust variant name cannot silently break saved
    /// keymaps or command-palette integrations.
    pub fn id(self) -> String {
        use PaneCommand::*;
        match self {
            Focus(direction) => format!("mullion.focus.{}", pane_direction_slug(direction)),
            FocusNext => "mullion.focus.next".into(),
            FocusPrevious => "mullion.focus.previous".into(),
            FocusFirst => "mullion.focus.first".into(),
            FocusLast => "mullion.focus.last".into(),
            FocusIndex(index) => format!("mullion.focus.index.{index}"),
            Split(direction) => format!("mullion.split.{}", split_direction_slug(direction)),
            Close => "mullion.close".into(),
            Move(direction) => format!("mullion.move.{}", pane_direction_slug(direction)),
            Swap(direction) => format!("mullion.swap.{}", pane_direction_slug(direction)),
            SwapNext => "mullion.swap.next".into(),
            SwapPrevious => "mullion.swap.previous".into(),
            Resize(direction) => format!("mullion.resize.{}", pane_direction_slug(direction)),
            SetParentSplitDirection(direction) => {
                format!("mullion.parent-split.{}", split_direction_slug(direction))
            }
            ToggleParentSplitDirection => "mullion.parent-split.toggle".into(),
            Balance => "mullion.layout.balance".into(),
            Rotate(PaneRotation::Forward) => "mullion.rotate.forward".into(),
            Rotate(PaneRotation::Backward) => "mullion.rotate.backward".into(),
            ApplyLayout(layout) => format!("mullion.layout.{}", layout_slug(layout)),
            ToggleZoom => "mullion.zoom.toggle".into(),
        }
    }
}

fn pane_direction_slug(direction: PaneDirection) -> &'static str {
    match direction {
        PaneDirection::Left => "left",
        PaneDirection::Right => "right",
        PaneDirection::Up => "up",
        PaneDirection::Down => "down",
    }
}

fn split_direction_slug(direction: SplitDirection) -> &'static str {
    match direction {
        SplitDirection::Horizontal => "horizontal",
        SplitDirection::Vertical => "vertical",
    }
}

fn layout_slug(layout: PaneLayout) -> &'static str {
    match layout {
        PaneLayout::EvenHorizontal => "even-horizontal",
        PaneLayout::EvenVertical => "even-vertical",
        PaneLayout::MainHorizontal => "main-horizontal",
        PaneLayout::MainVertical => "main-vertical",
        PaneLayout::Tiled => "tiled",
    }
}

/// Why a pane command could not be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneCommandError {
    NoFocusedPane,
    NoNeighbor,
    SplitUnavailable,
    SplitRefused,
    CannotCloseLastPane,
    InvalidPaneIndex,
    NotApplicable,
}
impl std::fmt::Display for PaneCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NoFocusedPane => "the layout has no focused pane",
            Self::NoNeighbor => "there is no pane in that direction",
            Self::SplitUnavailable => "no split-pane factory is configured",
            Self::SplitRefused => "the split-pane factory refused the split",
            Self::CannotCloseLastPane => "the last pane cannot be closed",
            Self::InvalidPaneIndex => "there is no pane at that index",
            Self::NotApplicable => "the command does not apply to this layout",
        };
        f.write_str(message)
    }
}
impl std::error::Error for PaneCommandError {}
pub type PaneCommandResult = Result<(), PaneCommandError>;
