//! Mullion-specific adapters for the standalone `gpui-command-palette` crate.
use crate::{
    ActivityCatalog, ActivityCatalogGroup, ActivityId, PaneCommand, PaneData, PaneId,
    VisibleActivityNode,
};
use gpui::AppContext as _;
use gpui_command_palette::Command;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteInvocation {
    PaneCommand(PaneCommand),
    SelectActivity { pane: PaneId, activity: ActivityId },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteInvocationError {
    Command(crate::PaneCommandError),
    PaneNotFound(PaneId),
    ActivityNotVisible { pane: PaneId, activity: ActivityId },
}
impl std::fmt::Display for PaletteInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(e) => e.fmt(f),
            Self::PaneNotFound(p) => write!(f, "pane `{}` does not exist", p.0),
            Self::ActivityNotVisible { pane, activity } => write!(
                f,
                "activity `{}` is not visible in pane `{}`",
                activity.0, pane.0
            ),
        }
    }
}
impl std::error::Error for PaletteInvocationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Command(e) => Some(e),
            _ => None,
        }
    }
}
impl PaletteInvocation {
    pub fn pane_command(&self) -> Option<PaneCommand> {
        match self {
            Self::PaneCommand(c) => Some(*c),
            _ => None,
        }
    }
    pub fn activity(&self) -> Option<(&PaneId, &ActivityId)> {
        match self {
            Self::SelectActivity { pane, activity } => Some((pane, activity)),
            _ => None,
        }
    }
}

/// Compatibility alias: generic entry/search behavior is owned by the external crate.
pub type PaletteEntry = Command<PaletteInvocation>;
pub type PaletteSearchResult = gpui_command_palette::SearchResult<PaletteInvocation>;
fn entry(
    id: String,
    name: String,
    description: String,
    group: String,
    invocation: PaletteInvocation,
) -> PaletteEntry {
    Command::with_metadata(id, name, invocation, |_, _| {})
        .description(description)
        .group(group)
}

pub fn pane_command_palette_entries(can_split: bool) -> Vec<PaletteEntry> {
    PaneCommand::catalog()
        .into_iter()
        .filter(|c| can_split || !matches!(c, PaneCommand::Split(_)))
        .map(|command| {
            entry(
                command.id(),
                command.name(),
                command.description().into(),
                command.group().label().into(),
                PaletteInvocation::PaneCommand(command),
            )
        })
        .collect()
}
pub fn mullion_palette_entries(panes: &[PaneId], can_split: bool) -> Vec<PaletteEntry> {
    let mut entries = focus_index_palette_entries(panes);
    entries.extend(pane_command_palette_entries(can_split));
    entries
}
pub fn focus_index_palette_entries(panes: &[PaneId]) -> Vec<PaletteEntry> {
    panes
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            let command = PaneCommand::FocusIndex(index);
            entry(
                command.id(),
                format!("{} · {}", index + 1, pane.0),
                "Focus this pane".into(),
                command.group().label().into(),
                PaletteInvocation::PaneCommand(command),
            )
        })
        .collect()
}
pub fn activity_palette_entries<D: PaneData>(
    catalog: &ActivityCatalog<D>,
    pane: &PaneId,
    data: &D,
) -> Vec<PaletteEntry> {
    let projection = catalog.visible(data, None);
    let mut entries = Vec::new();
    flatten_activities(
        &projection.primary,
        ActivityCatalogGroup::Primary,
        pane,
        &mut Vec::new(),
        &mut entries,
    );
    flatten_activities(
        &projection.trailing,
        ActivityCatalogGroup::Trailing,
        pane,
        &mut Vec::new(),
        &mut entries,
    );
    entries
}
fn flatten_activities<D: PaneData>(
    nodes: &[VisibleActivityNode<D>],
    catalog_group: ActivityCatalogGroup,
    pane: &PaneId,
    breadcrumbs: &mut Vec<String>,
    entries: &mut Vec<PaletteEntry>,
) {
    for node in nodes {
        match node {
            VisibleActivityNode::Activity(visible) => {
                let activity = &visible.activity;
                let path = breadcrumbs.join(" › ");
                let group = if path.is_empty() {
                    match catalog_group {
                        ActivityCatalogGroup::Primary => "Mullion · Activities".into(),
                        ActivityCatalogGroup::Trailing => "Mullion · Activities · Trailing".into(),
                    }
                } else {
                    format!("Mullion · Activities · {path}")
                };
                entries.push(entry(
                    format!("mullion.activity.{}.{}", pane.0, activity.id.0),
                    activity.name.to_string(),
                    if path.is_empty() {
                        format!("Select this activity in pane {}", pane.0)
                    } else {
                        format!("Select from {path} in pane {}", pane.0)
                    },
                    group,
                    PaletteInvocation::SelectActivity {
                        pane: pane.clone(),
                        activity: activity.id.clone(),
                    },
                ));
            }
            VisibleActivityNode::Category(category) => {
                breadcrumbs.push(category.name.to_string());
                flatten_activities(
                    &category.children,
                    catalog_group,
                    pane,
                    breadcrumbs,
                    entries,
                );
                breadcrumbs.pop();
            }
        }
    }
}
/// Create, wire, and mount the shared palette widget for a Mullion view.
pub fn command_palette_for_view<D: PaneData>(
    view: &gpui::Entity<crate::MullionView<D>>,
    cx: &mut gpui::App,
) -> gpui::Entity<gpui_command_palette::CommandPalette<PaletteInvocation>> {
    let weak = view.downgrade();
    let palette = cx.new(|cx| {
        gpui_command_palette::CommandPalette::new(cx).with_on_execute(
            move |invocation: &PaletteInvocation, _, cx| {
                let invocation = invocation.clone();
                weak.update(cx, |view, cx| {
                    let _ = view.invoke_palette(invocation, cx);
                })
                .ok();
            },
        )
    });
    view.update(cx, |view, cx| {
        view.set_command_palette(Some(palette.clone()), cx)
    });
    palette
}

/// Source-compatible forwarding shim; ranking lives in `gpui-command-palette`.
pub fn search_palette(entries: &[PaletteEntry], query: &str) -> Vec<PaletteSearchResult> {
    gpui_command_palette::search_commands(entries, query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Activity, ActivityCategory, ActivityNode, CategoryId};
    use gpui::{div, prelude::*, rgb, SharedString};
    use std::sync::Arc;
    fn activity(id: &str, name: &str, visible: fn(&bool) -> bool) -> ActivityNode<bool> {
        ActivityNode::Activity(Activity {
            id: ActivityId::new(id),
            name: SharedString::from(name.to_owned()),
            filter: visible,
            render: Arc::new(|_, _| div().into_any_element()),
        })
    }
    fn yes(_: &bool) -> bool {
        true
    }
    fn value(v: &bool) -> bool {
        *v
    }
    #[test]
    fn command_and_focus_adapters_are_stable() {
        let entries = pane_command_palette_entries(true);
        assert_eq!(entries.len(), PaneCommand::catalog().len());
        assert_eq!(entries[0].id, "mullion.focus.left");
        assert_eq!(entries[0].group.as_deref(), Some("Mullion · Focus"));
        let focus = focus_index_palette_entries(&[PaneId::new("one"), PaneId::new("two")]);
        assert_eq!(focus[1].name, "2 · two");
        assert_eq!(
            focus[1].metadata,
            PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(1))
        );
    }
    #[test]
    fn dynamic_activity_metadata_and_invocation() {
        let catalog = ActivityCatalog::new(vec![ActivityNode::Category(ActivityCategory {
            id: CategoryId::new("media"),
            name: "Media".into(),
            color: rgb(0xff0000).into(),
            children: vec![
                activity("hidden", "Hidden", value),
                activity("meters", "Meters", yes),
            ],
        })]);
        let entries = activity_palette_entries(&catalog, &PaneId::new("main"), &false);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].group.as_deref(),
            Some("Mullion · Activities · Media")
        );
        assert_eq!(
            entries[0].metadata,
            PaletteInvocation::SelectActivity {
                pane: PaneId::new("main"),
                activity: ActivityId::new("meters")
            }
        );
    }
    #[test]
    fn external_search_is_tokenized_and_stable() {
        let entries = pane_command_palette_entries(true);
        assert_eq!(
            search_palette(&entries, "focus LEFT")[0].entry.id,
            "mullion.focus.left"
        );
        assert!(search_palette(&entries, "left missing").is_empty());
    }
    #[test]
    fn invocation_serde_round_trips() {
        let value = PaletteInvocation::SelectActivity {
            pane: PaneId::new("窗格"),
            activity: ActivityId::new("音频"),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<PaletteInvocation>(&json).unwrap(),
            value
        );
    }
}
