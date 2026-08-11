//! Native, cross-platform GPUI split panes.
pub mod accessibility;
pub mod activity;
pub mod activity_bar;
pub mod activity_catalog;
pub mod command;
pub mod command_actions;
pub mod drag;
pub mod events;
pub mod focus;
pub mod keybindings;
pub mod model;
pub mod overlay;
pub mod palette;
pub mod platform;
pub mod settings;
pub mod styles;
pub mod theme;
pub mod tree;
pub mod view;
pub mod workspace;

/// Common imports for applications embedding Mullion.
///
/// The prelude intentionally contains only the primary model, activity, view,
/// workspace, configuration, and event types. Advanced adapters remain
/// available from their named modules.
pub mod prelude {
    pub use crate::{
        Activity, ActivityBarConfig, ActivityCatalog, ActivityFactoryRegistry, ActivityId,
        ActivityInstance, ActivityNode, MullionConfig, MullionModel, MullionSettings,
        MullionStyles, MullionTheme, MullionThemeMode, MullionView, PaneData, PaneEvent, PaneId,
        PaneNode, SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
    };
}
pub use accessibility::*;
pub use activity::*;
pub use activity_bar::*;
pub use activity_catalog::*;
pub use command::*;
pub use command_actions::*;
pub use drag::*;
pub use events::*;
pub use focus::*;
/// The exact shared palette crate used by Mullion, re-exported so hosts cannot
/// accidentally link a second revision and register duplicate GPUI actions.
pub use gpui_command_palette;
pub use keybindings::*;
pub use model::*;
pub use overlay::*;
pub use palette::*;
pub use platform::*;
pub use settings::*;
pub use styles::*;
pub use theme::*;
pub use tree::*;
pub use view::*;
pub use workspace::*;
