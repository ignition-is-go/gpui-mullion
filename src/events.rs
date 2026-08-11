use serde::{Deserialize, Serialize};

use crate::{ActivityId, DropEdge, PaneData, PaneId, PaneNode, SplitDirection, WorkspaceId};

/// An observable change made by [`crate::MullionModel`].
///
/// Layout and pane-data variants are **persistence events**. Hosts may use
/// them to update durable state; every successful local tree mutation ends
/// its persistence trace with [`PaneEvent::TreeChanged`]. [`PaneEvent::FocusChanged`] and
/// [`PaneEvent::ZoomChanged`] are **transient view events** and must not be
/// persisted as part of the pane tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaneEvent<D: PaneData> {
    /// A new leaf was inserted beside an existing pane.
    Split {
        /// Pane that received the split.
        target: PaneId,
        /// Axis used by the new parent split.
        direction: SplitDirection,
        /// Host-provided identity of the inserted pane.
        new_id: PaneId,
        /// Initial data stored in the inserted pane.
        new_data: D,
    },
    /// A leaf was removed from the tree.
    Closed {
        /// Identity of the removed pane.
        id: PaneId,
        /// Data returned from the removed pane.
        data: D,
    },
    /// A split ratio changed.
    Resized {
        /// Stable split key, equal to the second subtree's leftmost leaf id.
        split_key: PaneId,
        /// Stored first-child fraction after clamping to `0.1..=0.9`.
        ratio: f64,
    },
    /// An existing pane moved relative to another pane.
    Moved {
        /// Pane removed from its previous layout slot.
        source: PaneId,
        /// Pane used as the drop destination.
        destination: PaneId,
        /// Destination edge or center operation.
        edge: DropEdge,
    },
    /// A host-minted pane was inserted for an activity drop.
    ///
    /// This persistence event is followed by `TreeChanged`; the transient
    /// focus change for the inserted pane follows the tree snapshot.
    ActivityDropped {
        /// Activity selected in the inserted pane.
        activity: ActivityId,
        /// Existing pane used as the drop destination.
        destination: PaneId,
        /// Destination edge used for insertion.
        edge: DropEdge,
        /// Host-provided identity of the inserted pane.
        new_id: PaneId,
        /// Initial data stored in the inserted pane.
        new_data: D,
    },
    /// A pane's parent split changed axis.
    DirectionChanged {
        /// Pane identifying the affected parent split.
        pane: PaneId,
        /// Newly stored split axis.
        direction: SplitDirection,
    },
    /// A pane selected a different activity.
    ActivityChanged {
        /// Pane whose selection changed.
        pane: PaneId,
        /// Selected activity, or `None` when cleared.
        activity: Option<ActivityId>,
    },
    /// Consumer-defined pane data changed.
    DataChanged {
        /// Pane whose data changed.
        pane: PaneId,
        /// Complete replacement data.
        data: D,
    },
    /// Transient view state; not part of the persisted pane tree.
    FocusChanged {
        /// Focused pane, or `None` if no pane owns model focus.
        pane: Option<PaneId>,
    },
    /// Transient view state; not part of the persisted pane tree.
    ZoomChanged {
        /// Zoomed pane, or `None` when zoom is disabled.
        pane: Option<PaneId>,
    },
    /// Complete persisted tree after a successful local tree mutation.
    TreeChanged {
        /// Validated post-mutation snapshot.
        tree: PaneNode<D>,
    },
}

impl<D: PaneData> PaneEvent<D> {
    /// Whether this event describes durable pane state.
    pub fn is_persistence(&self) -> bool {
        !matches!(self, Self::FocusChanged { .. } | Self::ZoomChanged { .. })
    }

    /// Whether this event only describes transient model view state.
    pub fn is_transient(&self) -> bool {
        !self.is_persistence()
    }
}

/// A durable, typed workspace-management event.
///
/// The full validated snapshot deliberately avoids requiring persistence consumers to
/// replay a second workspace mutation language. Transient pane focus/zoom events are
/// emitted separately and are not represented here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceEvent<D: PaneData> {
    /// The complete validated workspace collection changed.
    SnapshotChanged {
        /// Post-mutation workspace snapshot.
        workspaces: crate::WorkspaceSet<D>,
    },
}

/// Emitted by [`crate::MullionView`] after its active internal workspace changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceChanged {
    /// Workspace that was active before the switch.
    pub previous: WorkspaceId,
    /// Workspace active after the switch.
    pub active: WorkspaceId,
}
