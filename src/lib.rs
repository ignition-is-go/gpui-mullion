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
        register_key_bindings, Activity, ActivityBarConfig, ActivityCatalog, ActivityCategory,
        ActivityFactoryRegistry, ActivityId, ActivityInstance, ActivityNode, CategoryId,
        FocusPresentation, MullionAppearance, MullionConfig, MullionModel, MullionSettings,
        MullionTheme, MullionThemeMode, MullionThemeProvider, MullionView, PaneData, PaneEvent,
        PaneFocusBehavior, PaneId, PaneNode, SplitDirection, Workspace, WorkspaceId, WorkspaceSet,
    };
}
pub use accessibility::{
    MullionAccessibilityNode, MullionAccessibilityRole, MullionAccessibilityState,
};
pub use activity::{
    Activity, ActivityCacheKey, ActivityCategory, ActivityDispose, ActivityFactory,
    ActivityFactoryRegistry, ActivityInstance, ActivityNode, ActivityRenderer, ActivityUpdate,
};
pub use activity_bar::{
    ActivityBarAxis, ActivityBarBehavior, ActivityBarConfig, ActivityBarEdge,
    ActivityBarHostConfig, ActivityBarHoverIntent, ActivityBarHoverState, ActivityBarMode,
    ActivityBarModeResolver, ActivityBarSlots, ActivityExpansionState, HoverGeneration,
    PaneBorderColor, PaneControl, PaneHeaderConfig,
};
pub use activity_catalog::{
    ActivityCatalog, ActivityCatalogGroup, ActivityCatalogValidationError, ActivityChrome,
    ActivityIcon, ActivityProjection, CategoryChrome, ChromeRenderer, VisibleActivity,
    VisibleActivityNode, VisibleCategory,
};
pub use command::{
    PaneCommand, PaneCommandError, PaneCommandExecutionOptions, PaneCommandGroup,
    PaneCommandResult, PaneSplitFactory,
};
pub use command_actions::{
    action_for_command, action_reference_id, command_for_action, compile_keymap,
    ApplyEvenHorizontalLayout, ApplyEvenVerticalLayout, ApplyMainHorizontalLayout,
    ApplyMainVerticalLayout, ApplyTiledLayout, FocusFirst, FocusLast, FocusPane,
    KeymapCompileError, MovePaneDown, MovePaneLeft, MovePaneRight, MovePaneUp, ResizePaneDown,
    ResizePaneLeft, ResizePaneRight, ResizePaneUp, RotatePanesBackward, RotatePanesForward,
    SetParentSplitHorizontal, SetParentSplitVertical, SplitPaneHorizontal, SplitPaneVertical,
    SwapPaneDown, SwapPaneLeft, SwapPaneNext, SwapPanePrevious, SwapPaneRight, SwapPaneUp,
    ToggleParentSplitDirection, MULLION_KEY_CONTEXT,
};
pub use drag::{
    DockBounds, DockConfig, DockDrag, DockHover, DockIndicator, DockPayload, DockPoint,
    NewPaneFactory,
};
pub use events::{PaneEvent, WorkspaceChanged, WorkspaceEvent};
pub use focus::PaneFocusBehavior;
/// The exact GPUI revision used by Mullion, re-exported to keep public types identical.
pub use gpui;
/// The exact shared palette crate used by Mullion, re-exported so hosts cannot
/// accidentally link a second revision and register duplicate GPUI actions.
pub use gpui_command_palette;
pub use keybindings::{
    normalize_key, KeyChord, KeySequenceMatch, KeyStroke, MullionKeyBinding, MullionKeymap,
};
pub use model::MullionModel;
pub use overlay::{
    ControlledOverlaySource, MullionOverlay, OverlayAlignment, OverlayBackdrop,
    OverlayDismissHandler, OverlayError, OverlayHostConfig, OverlayId, OverlayLength,
    OverlayMutation, OverlayPlacement, OverlayPolicy, OverlayRenderer, OverlaySize, OverlayStack,
    OverlayTier,
};
pub use palette::{
    activity_palette_entries, attach_command_palette, command_palette_for_view,
    focus_index_palette_entries, install_command_palette_for_view, mullion_palette_entries,
    pane_command_palette_entries, search_palette, FocusedPaneCommandProvider,
    MullionPaletteBinding, PaletteEntry, PaletteInvocation, PaletteInvocationError,
    PaletteSearchResult,
};
#[cfg(not(target_family = "wasm"))]
pub use platform::NativeDetachedWindowService;
pub use platform::{
    DetachError, DetachedWindowService, UnavailableDetachedWindows, WindowCapabilities,
};
pub use settings::{
    FocusPresentation, MullionConfig, MullionConfiguration, MullionPresentation, MullionSetting,
    MullionSettingOption, MullionSettings, MullionSettingsConfig,
};
pub use styles::{
    ActivityBarStyle, DropOverlayStyle, MullionAppearance, MullionRootStyle, PaneControlStyle,
    PaneHeaderStyle, PaneStyle, SplitHandleStyle, WorkspaceSwitcherStyle,
};
pub use theme::{MullionAppearanceProvider, MullionTheme, MullionThemeMode, MullionThemeProvider};
pub use tree::{
    collect_split_keys, collect_split_ratios, directional_neighbor, find_ratio,
    find_split_direction, leaf_rect, resize_boundary, split_parent_rect, ActivityId, CategoryId,
    DropEdge, PaneData, PaneDirection, PaneId, PaneLayout, PaneNode, PaneNodeBranch, PaneNodePath,
    PaneRotation, PaneValidationError, Rect, SplitDirection,
};
pub use view::{
    register_key_bindings, register_keymap, try_register_key_bindings, BalancePanes,
    CancelSplitResize, ClosePane, FocusDown, FocusLeft, FocusNext, FocusPrevious, FocusRight,
    FocusUp, MullionView, MullionViewConstructionError, ResizeSplitDecrease, ResizeSplitIncrease,
    ToggleZoom,
};
pub use workspace::{
    Workspace, WorkspaceError, WorkspaceId, WorkspaceSet, WorkspaceSetError,
    WorkspaceValidationError,
};
