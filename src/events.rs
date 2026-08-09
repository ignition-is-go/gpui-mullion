use serde::{Deserialize, Serialize};

use crate::{ActivityId, DropEdge, PaneData, PaneId, PaneNode, SplitDirection, WorkspaceId};

/// An observable change made by [`crate::MullionModel`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
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
    FocusChanged {
        pane: Option<PaneId>,
    },
    ZoomChanged {
        pane: Option<PaneId>,
    },
    TreeChanged {
        tree: PaneNode<D>,
    },
}

/// Emitted by [`crate::MullionView`] after its active internal workspace changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceChanged {
    pub previous: WorkspaceId,
    pub active: WorkspaceId,
}
