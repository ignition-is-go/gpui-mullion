use crate::{PaneDirection, PaneLayout, PaneRotation, SplitDirection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    /// Static command catalog used by keymaps and command-palette adapters.
    /// `FocusIndex` is dynamic and therefore omitted.
    pub fn catalog() -> Vec<Self> {
        use PaneCommand::*;
        use PaneDirection::*;
        vec![
            Focus(Left),
            Focus(Right),
            Focus(Up),
            Focus(Down),
            FocusNext,
            FocusPrevious,
            FocusFirst,
            FocusLast,
            Split(SplitDirection::Horizontal),
            Split(SplitDirection::Vertical),
            Close,
            Move(Left),
            Move(Right),
            Move(Up),
            Move(Down),
            Swap(Left),
            Swap(Right),
            Swap(Up),
            Swap(Down),
            SwapNext,
            SwapPrevious,
            Resize(Left),
            Resize(Right),
            Resize(Up),
            Resize(Down),
            SetParentSplitDirection(SplitDirection::Horizontal),
            SetParentSplitDirection(SplitDirection::Vertical),
            ToggleParentSplitDirection,
            Balance,
            Rotate(PaneRotation::Forward),
            Rotate(PaneRotation::Backward),
            ApplyLayout(PaneLayout::EvenHorizontal),
            ApplyLayout(PaneLayout::EvenVertical),
            ApplyLayout(PaneLayout::MainHorizontal),
            ApplyLayout(PaneLayout::MainVertical),
            ApplyLayout(PaneLayout::Tiled),
            ToggleZoom,
        ]
    }

    /// Stable id suitable for command registries.
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

    pub fn name(self) -> String {
        use PaneCommand::*;
        match self {
            Focus(direction) => format!("Focus Pane {}", pane_direction_label(direction)),
            FocusNext => "Focus Next Pane".into(),
            FocusPrevious => "Focus Previous Pane".into(),
            FocusFirst => "Focus First Pane".into(),
            FocusLast => "Focus Last Pane".into(),
            FocusIndex(index) => format!("Focus Pane {}", index + 1),
            Split(SplitDirection::Horizontal) => "Split Pane Left/Right".into(),
            Split(SplitDirection::Vertical) => "Split Pane Top/Bottom".into(),
            Close => "Close Focused Pane".into(),
            Move(direction) => format!("Move Pane {}", pane_direction_label(direction)),
            Swap(direction) => format!("Swap with Pane {}", pane_direction_label(direction)),
            SwapNext => "Swap with Next Pane".into(),
            SwapPrevious => "Swap with Previous Pane".into(),
            Resize(direction) => format!("Grow Pane {}", pane_direction_label(direction)),
            SetParentSplitDirection(SplitDirection::Horizontal) => {
                "Set Parent Split Left/Right".into()
            }
            SetParentSplitDirection(SplitDirection::Vertical) => {
                "Set Parent Split Top/Bottom".into()
            }
            ToggleParentSplitDirection => "Toggle Parent Split Direction".into(),
            Balance => "Balance Pane Splits".into(),
            Rotate(PaneRotation::Forward) => "Rotate Panes Forward".into(),
            Rotate(PaneRotation::Backward) => "Rotate Panes Backward".into(),
            ApplyLayout(layout) => format!("Apply {} Layout", layout_label(layout)),
            ToggleZoom => "Toggle Focused Pane Zoom".into(),
        }
    }

    pub fn group(self) -> PaneCommandGroup {
        use PaneCommand::*;
        match self {
            Focus(..) | FocusNext | FocusPrevious | FocusFirst | FocusLast | FocusIndex(..) => {
                PaneCommandGroup::Focus
            }
            Split(..) | Close => PaneCommandGroup::Pane,
            Move(..) | Swap(..) | SwapNext | SwapPrevious | Rotate(..) => PaneCommandGroup::Arrange,
            Resize(..) => PaneCommandGroup::Resize,
            SetParentSplitDirection(..)
            | ToggleParentSplitDirection
            | Balance
            | ApplyLayout(..) => PaneCommandGroup::Layout,
            ToggleZoom => PaneCommandGroup::View,
        }
    }

    pub fn description(self) -> &'static str {
        use PaneCommand::*;
        match self {
            Focus(..) => "Focus the nearest pane in this direction",
            FocusNext | FocusPrevious => "Cycle focus through the pane layout",
            FocusFirst | FocusLast | FocusIndex(..) => "Focus a pane by layout order",
            Split(..) => "Create and focus a pane beside the focused pane",
            Close => "Close the focused pane and focus an adjacent pane",
            Move(..) => "Move the focused pane beside its directional neighbor",
            Swap(..) | SwapNext | SwapPrevious => {
                "Exchange panes without changing the split topology"
            }
            Resize(..) => "Grow the focused pane toward its nearest boundary",
            SetParentSplitDirection(..) => "Set the focused pane's parent split axis",
            ToggleParentSplitDirection => "Flip the focused pane's parent split axis",
            Balance => "Reset every split ratio to an equal half",
            Rotate(..) => "Rotate panes through the existing layout slots",
            ApplyLayout(..) => "Rebuild the split topology using a standard layout",
            ToggleZoom => "Temporarily fill Mullion with the focused pane",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneCommandGroup {
    Focus,
    Pane,
    Arrange,
    Resize,
    Layout,
    View,
}

impl PaneCommandGroup {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Focus => "Mullion · Focus",
            Self::Pane => "Mullion · Pane",
            Self::Arrange => "Mullion · Arrange",
            Self::Resize => "Mullion · Resize",
            Self::Layout => "Mullion · Layout",
            Self::View => "Mullion · View",
        }
    }
}
impl std::fmt::Display for PaneCommandGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
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

fn pane_direction_label(direction: PaneDirection) -> &'static str {
    match direction {
        PaneDirection::Left => "Left",
        PaneDirection::Right => "Right",
        PaneDirection::Up => "Up",
        PaneDirection::Down => "Down",
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

fn layout_label(layout: PaneLayout) -> &'static str {
    match layout {
        PaneLayout::EvenHorizontal => "Even Horizontal",
        PaneLayout::EvenVertical => "Even Vertical",
        PaneLayout::MainHorizontal => "Main Horizontal",
        PaneLayout::MainVertical => "Main Vertical",
        PaneLayout::Tiled => "Tiled",
    }
}

/// Host hook used by focus-relative split commands.
pub type PaneSplitFactory<D> =
    Arc<dyn Fn(&crate::PaneId, SplitDirection, &D) -> Option<(crate::PaneId, D)> + Send + Sync>;

/// Host-configurable behavior for command execution.
#[derive(Clone)]
pub struct PaneCommandExecutionOptions<D> {
    pub(crate) split_factory: Option<PaneSplitFactory<D>>,
    pub(crate) resize_step: f64,
}

impl<D> Default for PaneCommandExecutionOptions<D> {
    fn default() -> Self {
        Self {
            split_factory: None,
            resize_step: 0.05,
        }
    }
}

impl<D> PaneCommandExecutionOptions<D> {
    pub fn with_split_factory(mut self, factory: PaneSplitFactory<D>) -> Self {
        self.split_factory = Some(factory);
        self
    }

    pub fn with_split_factory_fn(
        mut self,
        factory: impl Fn(&crate::PaneId, SplitDirection, &D) -> Option<(crate::PaneId, D)>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.split_factory = Some(Arc::new(factory));
        self
    }

    pub fn with_resize_step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.resize_step = step;
        }
        self
    }

    pub fn can_split(&self) -> bool {
        self.split_factory.is_some()
    }
    pub fn resize_step(&self) -> f64 {
        self.resize_step
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
