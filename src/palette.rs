//! Host-agnostic command and activity palette projections.
//!
//! This module contains no palette widget or registration lifecycle. Hosts can
//! project Mullion state, search it, and translate the selected invocation back
//! into the model or view layer they own.

use crate::{
    ActivityCatalog, ActivityCatalogGroup, ActivityId, PaneCommand, PaneData, PaneId,
    VisibleActivityNode,
};
use serde::{Deserialize, Serialize};

/// A stable operation represented by a palette entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteInvocation {
    PaneCommand(PaneCommand),
    SelectActivity { pane: PaneId, activity: ActivityId },
}

impl PaletteInvocation {
    pub fn pane_command(&self) -> Option<PaneCommand> {
        match self {
            Self::PaneCommand(command) => Some(*command),
            Self::SelectActivity { .. } => None,
        }
    }

    pub fn activity(&self) -> Option<(&PaneId, &ActivityId)> {
        match self {
            Self::SelectActivity { pane, activity } => Some((pane, activity)),
            Self::PaneCommand(_) => None,
        }
    }
}

/// Serializable HSLA metadata, independent of a renderer's color type.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteColor {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl From<gpui::Hsla> for PaletteColor {
    fn from(color: gpui::Hsla) -> Self {
        Self {
            h: color.h,
            s: color.s,
            l: color.l,
            a: color.a,
        }
    }
}

/// One enclosing category, ordered from the outermost category inward.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteBreadcrumb {
    pub id: crate::CategoryId,
    pub name: String,
    pub color: PaletteColor,
}

/// Portable metadata suitable for Rship or any other palette implementation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group: String,
    pub catalog_group: Option<ActivityCatalogGroup>,
    pub breadcrumbs: Vec<PaletteBreadcrumb>,
    pub color: Option<PaletteColor>,
    pub invocation: PaletteInvocation,
}

impl PaletteEntry {
    fn command(command: PaneCommand) -> Self {
        Self {
            id: command.id(),
            name: command.name(),
            description: command.description().into(),
            group: command.group().label().into(),
            catalog_group: None,
            breadcrumbs: Vec::new(),
            color: None,
            invocation: PaletteInvocation::PaneCommand(command),
        }
    }
}

/// Project the exact static [`PaneCommand`] catalog.
///
/// Split commands are omitted when the host has no split factory, matching the
/// reference adapter. Dynamic focus entries are provided separately by
/// [`focus_index_palette_entries`].
pub fn pane_command_palette_entries(can_split: bool) -> Vec<PaletteEntry> {
    PaneCommand::catalog()
        .into_iter()
        .filter(|command| can_split || !matches!(command, PaneCommand::Split(_)))
        .map(PaletteEntry::command)
        .collect()
}

/// Project the complete live command palette: focus-index entries followed by
/// the exact static command catalog.
pub fn mullion_palette_entries(panes: &[PaneId], can_split: bool) -> Vec<PaletteEntry> {
    let mut entries = focus_index_palette_entries(panes);
    entries.extend(pane_command_palette_entries(can_split));
    entries
}

/// Project the live pane traversal order as dynamic `FocusIndex` commands.
pub fn focus_index_palette_entries(panes: &[PaneId]) -> Vec<PaletteEntry> {
    panes
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            let command = PaneCommand::FocusIndex(index);
            PaletteEntry {
                id: command.id(),
                name: format!("{} · {}", index + 1, pane.0),
                description: "Focus this pane".into(),
                group: command.group().label().into(),
                catalog_group: None,
                breadcrumbs: Vec::new(),
                color: None,
                invocation: PaletteInvocation::PaneCommand(command),
            }
        })
        .collect()
}

/// Recursively flatten the activities visible for one pane.
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
    breadcrumbs: &mut Vec<PaletteBreadcrumb>,
    entries: &mut Vec<PaletteEntry>,
) {
    for node in nodes {
        match node {
            VisibleActivityNode::Activity(visible) => {
                let activity = &visible.activity;
                let category_path = breadcrumbs
                    .iter()
                    .map(|category| category.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" › ");
                let group = if category_path.is_empty() {
                    match catalog_group {
                        ActivityCatalogGroup::Primary => "Mullion · Activities".into(),
                        ActivityCatalogGroup::Trailing => "Mullion · Activities · Trailing".into(),
                    }
                } else {
                    format!("Mullion · Activities · {category_path}")
                };
                entries.push(PaletteEntry {
                    id: format!("mullion.activity.{}.{}", pane.0, activity.id.0),
                    name: activity.name.to_string(),
                    description: if category_path.is_empty() {
                        format!("Select this activity in pane {}", pane.0)
                    } else {
                        format!("Select from {category_path} in pane {}", pane.0)
                    },
                    group,
                    catalog_group: Some(catalog_group),
                    breadcrumbs: breadcrumbs.clone(),
                    color: visible.inherited_color.map(Into::into),
                    invocation: PaletteInvocation::SelectActivity {
                        pane: pane.clone(),
                        activity: activity.id.clone(),
                    },
                });
            }
            VisibleActivityNode::Category(category) => {
                breadcrumbs.push(PaletteBreadcrumb {
                    id: category.id.clone(),
                    name: category.name.to_string(),
                    color: category.color.into(),
                });
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

/// One owned search result. `catalog_index` makes equal-score ordering explicit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteSearchResult {
    pub entry: PaletteEntry,
    pub score: u32,
    pub catalog_index: usize,
}

/// Tokenized, case-insensitive deterministic search.
///
/// Every token must occur in at least one metadata field. Exact and prefix
/// matches rank above word-prefix and substring matches; equal scores retain
/// input catalog order.
pub fn search_palette(entries: &[PaletteEntry], query: &str) -> Vec<PaletteSearchResult> {
    let tokens = query
        .split_whitespace()
        .map(|token| token.to_lowercase())
        .collect::<Vec<_>>();
    let mut results = entries
        .iter()
        .enumerate()
        .filter_map(|(catalog_index, entry)| {
            let score = score_entry(entry, &tokens)?;
            Some(PaletteSearchResult {
                entry: entry.clone(),
                score,
                catalog_index,
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.catalog_index.cmp(&b.catalog_index))
    });
    results
}

fn score_entry(entry: &PaletteEntry, tokens: &[String]) -> Option<u32> {
    if tokens.is_empty() {
        return Some(0);
    }
    let mut fields = vec![
        (entry.name.to_lowercase(), 400),
        (entry.id.to_lowercase(), 250),
        (entry.description.to_lowercase(), 100),
        (entry.group.to_lowercase(), 75),
    ];
    fields.extend(
        entry
            .breadcrumbs
            .iter()
            .map(|category| (category.name.to_lowercase(), 150)),
    );
    tokens.iter().try_fold(0_u32, |total, token| {
        fields
            .iter()
            .filter_map(|(field, weight)| match_score(field, token).map(|score| score + weight))
            .max()
            .map(|score| total + score)
    })
}

fn match_score(field: &str, token: &str) -> Option<u32> {
    if field == token {
        return Some(1_000);
    }
    if field.starts_with(token) {
        return Some(700);
    }
    if field
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(token))
    {
        return Some(500);
    }
    field
        .find(token)
        .map(|position| 250_u32.saturating_sub(position.min(200) as u32))
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
    fn value(value: &bool) -> bool {
        *value
    }

    #[test]
    fn command_catalog_metadata_is_exact_and_dynamic_focus_is_stable() {
        let entries = pane_command_palette_entries(true);
        assert_eq!(entries.len(), PaneCommand::catalog().len());
        assert_eq!(entries[0].id, "mullion.focus.left");
        assert_eq!(entries[0].name, "Focus Pane Left");
        assert_eq!(entries[0].group, "Mullion · Focus");
        assert_eq!(entries.last().unwrap().id, "mullion.zoom.toggle");
        assert!(!pane_command_palette_entries(false)
            .iter()
            .any(|entry| matches!(
                entry.invocation,
                PaletteInvocation::PaneCommand(PaneCommand::Split(_))
            )));

        let focus = focus_index_palette_entries(&[PaneId::new("α"), PaneId::new("two")]);
        assert_eq!(focus[1].id, "mullion.focus.index.1");
        assert_eq!(focus[1].name, "2 · two");
        assert_eq!(
            focus[1].invocation,
            PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(1))
        );
    }

    #[test]
    fn visible_activities_flatten_with_group_breadcrumb_color_and_invocation() {
        let catalog = ActivityCatalog::new(vec![ActivityNode::Category(ActivityCategory {
            id: CategoryId::new("media"),
            name: "Media".into(),
            color: rgb(0xff0000).into(),
            children: vec![
                activity("hidden", "Hidden", value),
                ActivityNode::Category(ActivityCategory {
                    id: CategoryId::new("audio"),
                    name: "Audio".into(),
                    color: rgb(0x00ff00).into(),
                    children: vec![activity("meters", "Meters", yes)],
                }),
            ],
        })])
        .with_trailing(vec![activity("settings", "Settings", yes)]);
        let entries = activity_palette_entries(&catalog, &PaneId::new("main"), &false);
        assert_eq!(entries.len(), 2);
        let meters = &entries[0];
        assert_eq!(meters.id, "mullion.activity.main.meters");
        assert_eq!(meters.catalog_group, Some(ActivityCatalogGroup::Primary));
        assert_eq!(
            meters
                .breadcrumbs
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["Media", "Audio"]
        );
        assert_eq!(
            meters.color,
            meters.breadcrumbs.last().map(|item| item.color)
        );
        assert_eq!(
            meters.invocation,
            PaletteInvocation::SelectActivity {
                pane: PaneId::new("main"),
                activity: ActivityId::new("meters")
            }
        );
        assert_eq!(
            entries[1].catalog_group,
            Some(ActivityCatalogGroup::Trailing)
        );
    }

    #[test]
    fn search_is_case_insensitive_tokenized_ranked_and_stable() {
        let entries = pane_command_palette_entries(true);
        let lower = search_palette(&entries, "focus LEFT");
        let upper = search_palette(&entries, "FOCUS left");
        assert_eq!(lower, upper);
        assert_eq!(lower[0].entry.id, "mullion.focus.left");
        assert!(search_palette(&entries, "left missing").is_empty());
        let all = search_palette(&entries[..3], "");
        assert_eq!(
            all.iter()
                .map(|result| result.catalog_index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn invocation_serde_is_exhaustive_for_both_adapter_paths() {
        let values = [
            PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(2)),
            PaletteInvocation::SelectActivity {
                pane: PaneId::new("窗格"),
                activity: ActivityId::new("音频"),
            },
        ];
        for value in values {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(
                serde_json::from_str::<PaletteInvocation>(&json).unwrap(),
                value
            );
        }
    }
}
