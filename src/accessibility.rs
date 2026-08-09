//! Additive accessibility metadata for GPUI and other view adapters.
//!
//! Mullion does not assume a particular accessibility tree API. These portable
//! descriptions give views consistent labels, roles, and state text while
//! leaving rendering and event wiring to the host.

use crate::{ActivityId, CategoryId, DropEdge, PaneId, SplitDirection, WorkspaceId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MullionAccessibilityRole {
    Pane,
    Activity,
    ActivityCategory,
    SplitHandle,
    DropTarget,
    Workspace,
}

/// Common interactive state, kept separate so a view can map each flag to the
/// native accessibility API as well as expose the combined description.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MullionAccessibilityState {
    pub focused: bool,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub active: bool,
    pub disabled: bool,
}

impl MullionAccessibilityState {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MullionAccessibilityNode {
    pub role: MullionAccessibilityRole,
    /// Stable domain identity when the represented object has one.
    pub id: Option<String>,
    pub label: String,
    pub description: String,
    pub state: MullionAccessibilityState,
}

impl MullionAccessibilityNode {
    pub fn state_description(&self) -> String {
        self.state.description()
    }

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
        ];
        let json = serde_json::to_string(&roles).unwrap();
        assert_eq!(
            json,
            r#"["Pane","Activity","ActivityCategory","SplitHandle","DropTarget","Workspace"]"#
        );
        assert_eq!(
            serde_json::from_str::<Vec<MullionAccessibilityRole>>(&json).unwrap(),
            roles
        );
    }
}
