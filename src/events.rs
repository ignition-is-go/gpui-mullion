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
    Split {
        target: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        new_data: D,
    },
    Closed {
        id: PaneId,
        data: D,
    },
    Resized {
        split_key: PaneId,
        ratio: f64,
    },
    Moved {
        source: PaneId,
        destination: PaneId,
        edge: DropEdge,
    },
    /// A host-minted pane was inserted for an activity drop.
    ///
    /// This persistence event is followed by `TreeChanged`; the transient
    /// focus change for the inserted pane follows the tree snapshot.
    ActivityDropped {
        activity: ActivityId,
        destination: PaneId,
        edge: DropEdge,
        new_id: PaneId,
        new_data: D,
    },
    DirectionChanged {
        pane: PaneId,
        direction: SplitDirection,
    },
    ActivityChanged {
        pane: PaneId,
        activity: Option<ActivityId>,
    },
    DataChanged {
        pane: PaneId,
        data: D,
    },
    /// Transient view state; not part of the persisted pane tree.
    FocusChanged {
        pane: Option<PaneId>,
    },
    /// Transient view state; not part of the persisted pane tree.
    ZoomChanged {
        pane: Option<PaneId>,
    },
    /// Complete persisted tree after a successful local tree mutation.
    TreeChanged {
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
    SnapshotChanged { workspaces: crate::WorkspaceSet<D> },
}

/// Emitted by [`crate::MullionView`] after its active internal workspace changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceChanged {
    pub previous: WorkspaceId,
    pub active: WorkspaceId,
}
