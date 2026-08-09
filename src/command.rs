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
    pub fn id(self) -> String {
        use PaneCommand::*;
        match self {
            Focus(d) => format!("mullion.focus.{d:?}").to_lowercase(),
            FocusNext => "mullion.focus.next".into(),
            FocusPrevious => "mullion.focus.previous".into(),
            FocusFirst => "mullion.focus.first".into(),
            FocusLast => "mullion.focus.last".into(),
            FocusIndex(i) => format!("mullion.focus.index.{i}"),
            Split(d) => format!("mullion.split.{d:?}").to_lowercase(),
            Close => "mullion.close".into(),
            Move(d) => format!("mullion.move.{d:?}").to_lowercase(),
            Swap(d) => format!("mullion.swap.{d:?}").to_lowercase(),
            SwapNext => "mullion.swap.next".into(),
            SwapPrevious => "mullion.swap.previous".into(),
            Resize(d) => format!("mullion.resize.{d:?}").to_lowercase(),
            SetParentSplitDirection(d) => format!("mullion.parent-split.{d:?}").to_lowercase(),
            ToggleParentSplitDirection => "mullion.parent-split.toggle".into(),
            Balance => "mullion.layout.balance".into(),
            Rotate(d) => format!("mullion.rotate.{d:?}").to_lowercase(),
            ApplyLayout(d) => format!("mullion.layout.{d:?}").to_lowercase(),
            ToggleZoom => "mullion.zoom.toggle".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneCommandError {
    NoFocusedPane,
    PaneNotFound,
    NoNeighbor,
    CannotCloseLastPane,
    SplitRefused,
    InvalidOperation,
}
impl std::fmt::Display for PaneCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PaneCommandError {}
pub type PaneCommandResult = Result<(), PaneCommandError>;
