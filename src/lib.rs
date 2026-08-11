//! Native, cross-platform GPUI split panes.
//!
//! Mullion provides a validated portable pane model and one shared GPUI view for
//! native and WebAssembly hosts. Start with [`prelude`], [`MullionView`], and
//! [`Activity::new`]; advanced integration APIs are organized in named modules.

#![deny(missing_docs, unsafe_code)]
#![deny(rustdoc::broken_intra_doc_links)]
/// Semantic accessibility metadata derived from Mullion state.
pub mod accessibility;
/// Activity definitions, durable GPUI instances, factories, and cache ownership.
pub mod activity;
/// Activity-bar placement, behavior, chrome, and hover policy.
pub mod activity_bar;
/// Validated recursive activity catalogs and per-pane projections.
pub mod activity_catalog;
/// Portable pane commands, metadata, results, and errors.
pub mod command;
/// Typed GPUI actions and keymap compilation for pane commands.
pub mod command_actions;
/// Typed pane/activity docking payloads and drop geometry.
pub mod drag;
/// Persistence and transient events emitted by pane and workspace mutations.
pub mod events;
/// Pointer-to-pane focus policies.
pub mod focus;
/// Serializable Mullion keymaps and normalized key sequences.
pub mod keybindings;
/// Toolkit-independent pane state machine and event production.
pub mod model;
/// Window-level overlay policy, stacks, and host integration.
pub mod overlay;
/// Command-palette projection, search, and invocation adapters.
pub mod palette;
/// Optional host-owned platform capabilities such as detached windows.
pub mod platform;
/// Runtime settings, controlled values, and serializable configuration.
pub mod settings;
/// Typed geometry and color tokens for Mullion chrome.
pub mod styles;
/// Light, dark, and system-resolved Mullion palettes.
pub mod theme;
/// Validated pane-tree data, mutations, geometry, and navigation algorithms.
pub mod tree;
/// The shared native/WebAssembly GPUI view and interaction adapters.
pub mod view;
/// Validated workspace snapshots and workspace-set operations.
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
