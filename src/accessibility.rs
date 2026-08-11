//! Additive accessibility metadata for GPUI and other view adapters.
//!
//! Mullion does not assume a particular accessibility tree API. These portable
//! descriptions give views consistent labels, roles, and state text while
//! leaving rendering and event wiring to the host.

use crate::{ActivityId, CategoryId, DropEdge, PaneId, SplitDirection, WorkspaceId};
use serde::{Deserialize, Serialize};

/// The semantic role of an element in a Mullion pane workspace.
///
/// Hosts can translate these portable roles into the closest roles offered by
/// their native accessibility API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MullionAccessibilityRole {
    /// A pane which contains one active activity.
    Pane,
    /// An activity that can be selected for display in a pane.
    Activity,
    /// An expandable group of activities.
    ActivityCategory,
    /// A handle that adjusts the relative sizes of two pane groups.
    SplitHandle,
    /// A docking destination shown while a pane is being moved.
    DropTarget,
    /// A selectable workspace in a collection of workspaces.
    Workspace,
    /// A control that closes a pane.
    CloseButton,
    /// A control used to drag and dock a pane.
    DragHandle,
}

/// Common interactive state, kept separate so a view can map each flag to the
/// native accessibility API as well as expose the combined description.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MullionAccessibilityState {
    /// Whether this element currently holds keyboard focus.
    pub focused: bool,
    /// Whether this element is the selected item in its containing collection.
    pub selected: bool,
    /// The expansion state of an expandable element, or `None` when expansion
    /// does not apply to the element.
    pub expanded: Option<bool>,
    /// Whether this element is the current active target or mode.
    pub active: bool,
    /// Whether interaction with this element is unavailable.
    pub disabled: bool,
}

impl MullionAccessibilityState {
    /// Returns a human-readable, comma-separated summary of the asserted state.
    ///
    /// States are emitted in field order. When no state is asserted, this
    /// returns `"available"`; an explicit `expanded: Some(false)` is described
    /// as `"collapsed"`.
    pub fn description(self) -> String {
        let mut states = Vec::new();
        if self.focused {
            states.push("focused");
        }
        if self.selected {
            states.push("selected");
        }
        if let Some(expanded) = self.expanded {
            states.push(if expanded { "expanded" } else { "collapsed" });
        }
        if self.active {
            states.push("active");
        }
        if self.disabled {
            states.push("disabled");
        }
        if states.is_empty() {
            "available".into()
        } else {
            states.join(", ")
        }
    }
}

/// Portable accessibility metadata for one Mullion UI element.
///
/// The node is descriptive only: the host remains responsible for attaching
/// it to a native accessibility tree and wiring the corresponding actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MullionAccessibilityNode {
    /// The element's portable semantic role.
    pub role: MullionAccessibilityRole,
    /// Stable domain identity when the represented object has one.
    pub id: Option<String>,
    /// The short, user-facing accessible name of the element.
    pub label: String,
    /// Additional user-facing context about the element or its action.
    pub description: String,
    /// The element's current interactive state.
    pub state: MullionAccessibilityState,
}

impl MullionAccessibilityNode {
    /// Returns the human-readable summary produced by this node's [`state`](Self::state).
    pub fn state_description(&self) -> String {
        self.state.description()
    }

    /// Describes a pane within an ordered pane collection.
    ///
    /// `index` is zero-based and is rendered as a one-based ordinal out of
    /// `count`. `activity_name`, when present, identifies the pane's visible
    /// activity. `focused` controls the focused state, while `zoomed` both
    /// annotates the description and marks the pane active.
    pub fn pane(
        id: &PaneId,
        index: usize,
        count: usize,
        activity_name: Option<&str>,
        focused: bool,
        zoomed: bool,
    ) -> Self {
        let label = format!("Pane {} of {}", index + 1, count);
        let mut description = activity_name.map_or_else(
            || format!("Pane {}", id.0),
            |activity| format!("{activity} in pane {}", id.0),
        );
        if zoomed {
            description.push_str(", zoomed");
        }
        Self {
            role: MullionAccessibilityRole::Pane,
            id: Some(id.0.clone()),
            label,
            description,
            state: MullionAccessibilityState {
                focused,
                active: zoomed,
                ..Default::default()
            },
        }
    }

    /// Describes an activity selector.
    ///
    /// `selected` indicates that the activity is currently displayed in its
    /// pane and controls both the state and action-oriented description.
    pub fn activity(id: &ActivityId, name: &str, selected: bool) -> Self {
        Self {
            role: MullionAccessibilityRole::Activity,
            id: Some(id.0.clone()),
            label: name.into(),
            description: if selected {
                "Current pane activity".into()
            } else {
                "Switch to this pane activity".into()
            },
            state: MullionAccessibilityState {
                selected,
                ..Default::default()
            },
        }
    }

    /// Describes an expandable activity category.
    ///
    /// `expanded` is represented explicitly as either expanded or collapsed,
    /// rather than as a state that does not apply.
    pub fn category(id: &CategoryId, name: &str, expanded: bool) -> Self {
        Self {
            role: MullionAccessibilityRole::ActivityCategory,
            id: Some(id.0.clone()),
            label: format!("{name} activities"),
            description: if expanded {
                "Activity category is expanded".into()
            } else {
                "Activity category is collapsed".into()
            },
            state: MullionAccessibilityState {
                expanded: Some(expanded),
                ..Default::default()
            },
        }
    }

    /// Describes a handle for a split with the given axis and first-group ratio.
    ///
    /// `ratio` is a unitless fraction of the available split extent assigned to
    /// the first pane group. Finite values are clamped to `0.0..=1.0` and
    /// announced as a rounded percentage; non-finite values are announced as
    /// 50 percent. `disabled` reports whether the handle can currently resize.
    pub fn split(direction: SplitDirection, ratio: f64, disabled: bool) -> Self {
        let axis = match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        };
        let percent = if ratio.is_finite() {
            (ratio.clamp(0.0, 1.0) * 100.0).round() as u8
        } else {
            50
        };
        Self {
            role: MullionAccessibilityRole::SplitHandle,
            id: None,
            label: format!("Resize {axis} pane split"),
            description: format!("First pane group uses {percent} percent"),
            state: MullionAccessibilityState {
                disabled,
                ..Default::default()
            },
        }
    }

    /// Describes a pane docking target at `edge`.
    ///
    /// A center target moves content into an existing pane; every other edge
    /// creates a split. `active` indicates the currently highlighted target.
    pub fn drop_target(edge: DropEdge, active: bool) -> Self {
        let edge_label = match edge {
            DropEdge::Top => "top",
            DropEdge::Bottom => "bottom",
            DropEdge::Left => "left",
            DropEdge::Right => "right",
            DropEdge::Center => "center",
        };
        Self {
            role: MullionAccessibilityRole::DropTarget,
            id: None,
            label: format!("Dock at {edge_label}"),
            description: if edge == DropEdge::Center {
                "Move into this pane".into()
            } else {
                format!("Create a split at the {edge_label} edge")
            },
            state: MullionAccessibilityState {
                active,
                ..Default::default()
            },
        }
    }

    /// Describes the control that closes the pane identified by `id`.
    pub fn close_pane(id: &PaneId) -> Self {
        Self {
            role: MullionAccessibilityRole::CloseButton,
            id: Some(id.0.clone()),
            label: format!("Close pane {}", id.0),
            description: "Close this pane".into(),
            state: MullionAccessibilityState::default(),
        }
    }

    /// Describes the drag handle used to move and dock the pane identified by `id`.
    pub fn drag_handle(id: &PaneId) -> Self {
        Self {
            role: MullionAccessibilityRole::DragHandle,
            id: Some(id.0.clone()),
            label: format!("Move pane {}", id.0),
            description: "Drag to dock this pane in another location".into(),
            state: MullionAccessibilityState::default(),
        }
    }

    /// Describes a workspace within an ordered workspace collection.
    ///
    /// `index` is zero-based and is rendered as a one-based ordinal out of
    /// `count`. An `active` workspace is marked both selected and active.
    pub fn workspace(
        id: &WorkspaceId,
        name: &str,
        index: usize,
        count: usize,
        active: bool,
    ) -> Self {
        Self {
            role: MullionAccessibilityRole::Workspace,
            id: Some(id.0.clone()),
            label: name.into(),
            description: format!("Workspace {} of {}", index + 1, count),
            state: MullionAccessibilityState {
                selected: active,
                active,
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_produce_role_labels_and_state_descriptions() {
        let pane =
            MullionAccessibilityNode::pane(&PaneId::new("main"), 0, 3, Some("Meters"), true, true);
        assert_eq!(pane.label, "Pane 1 of 3");
        assert_eq!(pane.description, "Meters in pane main, zoomed");
        assert_eq!(pane.state_description(), "focused, active");

        let category =
            MullionAccessibilityNode::category(&CategoryId::new("media"), "Media", false);
        assert_eq!(category.label, "Media activities");
        assert_eq!(category.state_description(), "collapsed");

        let split = MullionAccessibilityNode::split(SplitDirection::Horizontal, f64::NAN, false);
        assert_eq!(split.label, "Resize horizontal pane split");
        assert_eq!(split.description, "First pane group uses 50 percent");

        let drop = MullionAccessibilityNode::drop_target(DropEdge::Left, true);
        assert_eq!(drop.label, "Dock at left");
        assert_eq!(drop.state_description(), "active");
    }

    #[test]
    fn every_role_has_a_stable_serde_representation() {
        let roles = [
            MullionAccessibilityRole::Pane,
            MullionAccessibilityRole::Activity,
            MullionAccessibilityRole::ActivityCategory,
            MullionAccessibilityRole::SplitHandle,
            MullionAccessibilityRole::DropTarget,
            MullionAccessibilityRole::Workspace,
            MullionAccessibilityRole::CloseButton,
            MullionAccessibilityRole::DragHandle,
        ];
        let json = serde_json::to_string(&roles).unwrap();
        assert_eq!(
            json,
            r#"["Pane","Activity","ActivityCategory","SplitHandle","DropTarget","Workspace","CloseButton","DragHandle"]"#
        );
        assert_eq!(
            serde_json::from_str::<Vec<MullionAccessibilityRole>>(&json).unwrap(),
            roles
        );
    }
}
