//! Mullion-specific adapters for the standalone `gpui-command-palette` crate.
use crate::{
    ActivityCatalog, ActivityCatalogGroup, ActivityId, PaneCommand, PaneData, PaneId,
    VisibleActivityNode,
};
use gpui::AppContext as _;
use gpui_command_palette::Command;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

/// UI-local commands derived from the currently focused pane and its data.
pub type FocusedPaneCommandProvider<D> = Rc<dyn Fn(&PaneId, &D) -> Vec<Command<()>>>;

/// Retained attachment between one Mullion view and a shared application palette.
///
/// Clones share the same RAII registration set. Mullion also retains the binding
/// so ignoring the returned value does not detach commands. Reattaching, explicit
/// detachment, or releasing the Mullion view removes only Mullion-owned entries.
#[derive(Clone)]
pub struct MullionPaletteBinding {
    core: Rc<RefCell<MullionPaletteBindingCore>>,
}

struct MullionPaletteBindingCore {
    palette: gpui::Entity<gpui_command_palette::CommandPalette<()>>,
    registrations: Vec<gpui_command_palette::Registration<()>>,
    active: bool,
}

impl MullionPaletteBinding {
    pub(crate) fn new(palette: gpui::Entity<gpui_command_palette::CommandPalette<()>>) -> Self {
        Self {
            core: Rc::new(RefCell::new(MullionPaletteBindingCore {
                palette,
                registrations: Vec::new(),
                active: true,
            })),
        }
    }

    /// Return the shared palette receiving Mullion registrations.
    pub fn palette(&self) -> gpui::Entity<gpui_command_palette::CommandPalette<()>> {
        self.core.borrow().palette.clone()
    }

    pub(crate) fn replace(&self, commands: Vec<Command<()>>, cx: &mut gpui::App) {
        let palette = {
            let mut core = self.core.borrow_mut();
            if !core.active {
                return;
            }
            core.registrations.clear();
            core.palette.clone()
        };
        let registry = palette.read(cx).registry().clone();
        let registrations = registry.register_many(commands);
        self.core.borrow_mut().registrations = registrations;
        palette.update(cx, |_, cx| cx.notify());
    }

    /// Remove this binding's registrations and notify the shared palette.
    pub fn detach(&self, cx: &mut gpui::App) {
        let palette = {
            let mut core = self.core.borrow_mut();
            if !core.active {
                return;
            }
            core.active = false;
            core.registrations.clear();
            core.palette.clone()
        };
        palette.update(cx, |_, cx| cx.notify());
    }
}

/// Action metadata attached to a Mullion command-palette entry.
///
/// Serde uses the default externally tagged representation. `PaneCommand`
/// wraps a serialized [`PaneCommand`], while `SelectActivity` wraps an object
/// with `pane` and `activity` fields. Consumers may persist or transmit this
/// metadata, but should still validate it against current view state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteInvocation {
    /// Dispatch a pane-management command.
    PaneCommand(PaneCommand),
    /// Select an activity in a specific pane.
    SelectActivity {
        /// Pane that should receive the activity selection.
        pane: PaneId,
        /// Activity to select.
        activity: ActivityId,
    },
}
/// Failure returned when executing palette metadata against the current view.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaletteInvocationError {
    /// The pane command was rejected; the wrapped error is exposed as the source.
    Command(crate::PaneCommandError),
    /// The invocation names a pane that no longer exists.
    PaneNotFound(PaneId),
    /// The activity is absent from the pane's current visible catalog projection.
    ActivityNotVisible {
        /// Pane whose visible activities were checked.
        pane: PaneId,
        /// Requested activity that was not visible.
        activity: ActivityId,
    },
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
    /// Returns the pane command, or `None` for an activity selection.
    pub fn pane_command(&self) -> Option<PaneCommand> {
        match self {
            Self::PaneCommand(c) => Some(*c),
            _ => None,
        }
    }
    /// Returns the target pane and activity, or `None` for a pane command.
    pub fn activity(&self) -> Option<(&PaneId, &ActivityId)> {
        match self {
            Self::SelectActivity { pane, activity } => Some((pane, activity)),
            _ => None,
        }
    }
}

/// Compatibility alias for an external command carrying [`PaletteInvocation`].
///
/// Generic entry and child-resolution behavior is owned by
/// `gpui-command-palette`.
pub type PaletteEntry = Command<PaletteInvocation>;
/// Compatibility alias for a ranked external search result.
///
/// Match scoring and result fields follow `gpui-command-palette`'s contract.
pub type PaletteSearchResult = gpui_command_palette::SearchResult<PaletteInvocation>;
fn entry(
    id: String,
    name: String,
    description: String,
    group: String,
    invocation: PaletteInvocation,
) -> PaletteEntry {
    Command::with_metadata(id, name, invocation, || {})
        .description(description)
        .group(group)
}

/// Projects the static pane-command catalog into palette entries.
///
/// When `can_split` is `false`, both split-direction commands are omitted.
/// Catalog order and command identifiers are preserved.
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
/// Builds the top-level Mullion palette, including the live pane picker.
///
/// The first entry is a searchable child entry for `panes`; remaining entries
/// are produced by [`pane_command_palette_entries`].
pub fn mullion_palette_entries(panes: &[PaneId], can_split: bool) -> Vec<PaletteEntry> {
    let focus_children = focus_index_palette_entries(panes);
    let fallback = PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(0));
    let focus = entry(
        "mullion.focus.pane".into(),
        "Focus Pane…".into(),
        "Choose a pane from the live Mullion layout".into(),
        "Mullion · Focus".into(),
        fallback,
    )
    .children(move || focus_children.clone())
    .searchable_children();
    let mut entries = vec![focus];
    entries.extend(pane_command_palette_entries(can_split));
    entries
}
/// Creates focus commands for `panes` in slice order.
///
/// Each command uses its zero-based slice position as [`PaneCommand::FocusIndex`]
/// while its displayed ordinal is one-based.
pub fn focus_index_palette_entries(panes: &[PaneId]) -> Vec<PaletteEntry> {
    panes
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            let command = PaneCommand::FocusIndex(index);
            entry(
                format!("mullion.focus.pane.{}", pane.0),
                format!("{} · {}", index + 1, pane.0),
                "Focus this pane".into(),
                "Mullion · Focus".into(),
                PaletteInvocation::PaneCommand(command),
            )
        })
        .collect()
}
/// Projects activities visible for one pane's current data into palette entries.
///
/// Catalog filters are evaluated with no search query. Category breadcrumbs
/// become entry groups, primary entries precede trailing entries, and hidden
/// activities are omitted.
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
fn direct_command<D: PaneData>(
    entry: PaletteEntry,
    view: gpui::WeakEntity<crate::MullionView<D>>,
) -> Command<()> {
    let id = entry.id.clone();
    let name = entry.name.clone();
    let description = entry.description.clone();
    let group = entry.group.clone();
    let shortcut = entry.shortcut.clone();
    let invocation = entry.metadata.clone();
    let handler_view = view.clone();
    let mut command = Command::with_handler(id, name, move |_, cx| {
        let invocation = invocation.clone();
        handler_view
            .update(cx, |view, cx| {
                let _ = view.invoke_palette(invocation, cx);
            })
            .ok();
    });
    if let Some(description) = description {
        command = command.description(description);
    }
    if let Some(group) = group {
        command = command.group(group);
    }
    if let Some(shortcut) = shortcut {
        command = command.shortcut(shortcut.modifiers, shortcut.key);
    }
    if let Some(children) = entry.resolve_children() {
        let child_view = view.clone();
        command = command.children(move || {
            children
                .clone()
                .into_iter()
                .map(|child| direct_command(child, child_view.clone()))
                .collect()
        });
        if entry.searches_children() {
            command = command.searchable_children();
        }
    }
    command
}

pub(crate) fn direct_palette_commands<D: PaneData>(
    entries: Vec<PaletteEntry>,
    view: gpui::WeakEntity<crate::MullionView<D>>,
) -> Vec<Command<()>> {
    entries
        .into_iter()
        .map(|entry| direct_command(entry, view.clone()))
        .collect()
}

/// Attach Mullion command registrations to a shared application palette.
///
/// This function does not render or install the palette. The application owns
/// one window/root-level palette and retains it independently of Mullion layout.
pub fn attach_command_palette<D: PaneData>(
    view: &gpui::Entity<crate::MullionView<D>>,
    palette: gpui::Entity<gpui_command_palette::CommandPalette<()>>,
    cx: &mut gpui::App,
) -> MullionPaletteBinding {
    view.update(cx, |view, cx| view.attach_command_palette(palette, cx))
}

/// Creates, wires, and attaches the Mullion-owned convenience palette widget.
///
/// Execution delegates to `MullionView::invoke_palette`; invocation errors are
/// intentionally ignored by this event callback because entries may become
/// stale between projection and selection. The returned entity is not given an
/// application action route; use [`install_command_palette_for_view`] for that.
pub fn command_palette_for_view<D: PaneData>(
    view: &gpui::Entity<crate::MullionView<D>>,
    cx: &mut gpui::App,
) -> gpui::Entity<gpui_command_palette::CommandPalette<()>> {
    let palette = cx.new(|cx| {
        gpui_command_palette::CommandPalette::new(cx)
            // GPUI resolves percentage padding against width. Use positioned
            // percentages so the shared widget matches the reference's top:20%.
            .with_position(gpui_command_palette::CommandPalettePosition::Custom {
                top: Some(gpui_command_palette::PaletteLength::percent(20.0)),
                right: None,
                bottom: None,
                left: Some(gpui_command_palette::PaletteLength::percent(50.0)),
                transform: Some(gpui_command_palette::PaletteTransform::pixels(-250.0, 0.0)),
            })
            // Match CSS line boxes after GPUI's taller font metrics.
            .with_input_theme(gpui_command_palette::CommandPaletteInputTheme {
                padding_y: gpui::px(4.5),
                ..Default::default()
            })
            .with_item_theme(gpui_command_palette::CommandPaletteItemTheme {
                padding_y: gpui::px(2.0),
                ..Default::default()
            })
    });
    view.update(cx, |view, cx| {
        view.set_command_palette(Some(palette.clone()), cx)
    });
    palette
}

/// Create the Mullion adapter palette, attach it to `view`, and install its
/// application-level action route for `window`.
///
/// The returned entity is the same palette attached to the view and may be
/// retained by the host for further palette configuration.
pub fn install_command_palette_for_view<D: PaneData>(
    view: &gpui::Entity<crate::MullionView<D>>,
    window: &gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Entity<gpui_command_palette::CommandPalette<()>> {
    let palette = command_palette_for_view(view, cx);
    gpui_command_palette::install_palette(&palette, window, cx);
    palette
}

/// Searches projected entries using `gpui-command-palette` ranking.
///
/// This source-compatible forwarding shim returns results in the external
/// crate's stable ranking order; Mullion does not interpret the query itself.
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
        let panes = [PaneId::new("one"), PaneId::new("two")];
        let focus = focus_index_palette_entries(&panes);
        assert_eq!(focus[1].id, "mullion.focus.pane.two");
        assert_eq!(focus[1].name, "2 · two");
        assert_eq!(
            focus[1].metadata,
            PaletteInvocation::PaneCommand(PaneCommand::FocusIndex(1))
        );
        let projected = mullion_palette_entries(&panes, true);
        assert_eq!(projected[0].id, "mullion.focus.pane");
        assert_eq!(projected[0].name, "Focus Pane…");
        assert_eq!(projected[0].resolve_children().unwrap().len(), 2);
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
    fn external_search_uses_case_insensitive_substrings_and_stable_order() {
        let entries = pane_command_palette_entries(true);
        assert_eq!(
            search_palette(&entries, "PANE LEFT")[0].entry.id,
            "mullion.focus.left"
        );
        assert!(search_palette(&entries, "focus missing").is_empty());
        assert!(search_palette(&entries, "mullion.focus.left").is_empty());
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
