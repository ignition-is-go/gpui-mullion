use crate::{ChromeRenderer, PaneData, PaneId};
use gpui::{App, Hsla, Window};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Main/cross axis used by activity-bar layout code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarAxis {
    /// The main axis runs left-to-right and the cross axis runs top-to-bottom.
    Horizontal,
    /// The main axis runs top-to-bottom and the cross axis runs left-to-right.
    Vertical,
}

/// Edge of a pane occupied by its activity bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarEdge {
    /// A vertical bar attached to the pane's left edge.
    #[default]
    Left,
    /// A vertical bar attached to the pane's right edge.
    Right,
    /// A horizontal bar attached to the pane's top edge.
    Top,
    /// A horizontal bar attached to the pane's bottom edge.
    Bottom,
}

impl ActivityBarEdge {
    /// Returns the bar's main layout axis.
    ///
    /// Left and right edges produce a vertical bar; top and bottom edges produce
    /// a horizontal bar.
    pub const fn axis(self) -> ActivityBarAxis {
        match self {
            Self::Left | Self::Right => ActivityBarAxis::Vertical,
            Self::Top | Self::Bottom => ActivityBarAxis::Horizontal,
        }
    }

    /// Returns whether items along this edge are laid out left-to-right.
    pub const fn is_horizontal(self) -> bool {
        matches!(self.axis(), ActivityBarAxis::Horizontal)
    }

    /// Whether the bar is on the trailing side of its cross axis.
    pub const fn is_trailing(self) -> bool {
        matches!(self, Self::Right | Self::Bottom)
    }

    /// Returns the corresponding edge on the other side of the pane.
    pub const fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// Built-in pane management controls, in their reference render order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneControl {
    /// The pane drag handle used to move or dock the pane.
    Move,
    /// The command that splits the pane horizontally.
    SplitHorizontal,
    /// The command that splits the pane vertically.
    SplitVertical,
    /// The command that closes the pane.
    Close,
}

impl PaneControl {
    /// All built-in controls in their canonical render and traversal order.
    pub const ORDER: [Self; 4] = [
        Self::Move,
        Self::SplitHorizontal,
        Self::SplitVertical,
        Self::Close,
    ];

    /// Returns the stable, lowercase key used in selectors and internal item ids.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::SplitHorizontal => "split-h",
            Self::SplitVertical => "split-v",
            Self::Close => "close",
        }
    }

    /// Returns the short human-readable label used for accessibility.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::SplitHorizontal => "Split H",
            Self::SplitVertical => "Split V",
            Self::Close => "Close",
        }
    }

    /// Builds the GPUI debug selector for this control in `pane`.
    ///
    /// The result has the form `pane-control:<control-key>:<pane-id>`.
    pub fn debug_selector(self, pane: &PaneId) -> String {
        format!("pane-control:{}:{}", self.key(), pane.0)
    }

    /// Builds the accessibility element id for this control in `pane`.
    ///
    /// The result has the form `mullion-pane-<control-key>-<pane-id>`.
    pub fn accessibility_id(self, pane: &PaneId) -> String {
        format!("mullion-pane-{}-{}", self.key(), pane.0)
    }
}

/// Per-pane visibility policy for an activity bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarMode {
    /// The bar remains visible and occupies layout space beside pane content.
    #[default]
    Pinned,
    /// The bar is not rendered.
    Hidden,
    /// The bar overlays pane content and reveals from its configured edge on hover.
    AutoHide,
}

/// Hover flyout timing. The delay applies only when opening; leaving cancels
/// a pending open immediately and animates closed over the configured duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityBarHoverIntent {
    /// Time the pointer must remain over the rail before the flyout opens.
    pub expand_delay_ms: u32,
    /// Duration of both the opening and closing flyout transitions.
    pub transition_duration_ms: u32,
}

impl Default for ActivityBarHoverIntent {
    fn default() -> Self {
        Self {
            expand_delay_ms: 0,
            transition_duration_ms: 150,
        }
    }
}

impl ActivityBarHoverIntent {
    /// Sets the opening hover-intent delay, in milliseconds.
    ///
    /// A value of zero makes a valid pointer entry logically expand immediately.
    pub const fn with_expand_delay_ms(mut self, delay_ms: u32) -> Self {
        self.expand_delay_ms = delay_ms;
        self
    }

    /// Sets the opening and closing animation duration, in milliseconds.
    ///
    /// This does not delay logical collapse on pointer leave; it controls only
    /// the visual transition to the collapsed endpoint.
    pub const fn with_transition_duration_ms(mut self, duration_ms: u32) -> Self {
        self.transition_duration_ms = duration_ms;
        self
    }
}

/// Interaction semantics independent of styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityBarBehavior {
    /// Whether hovering a collapsed rail may reveal its expanded labels or flyout.
    pub hover_expand: bool,
    /// Delay and animation timing applied to hover-driven expansion.
    pub hover_intent: ActivityBarHoverIntent,
}

impl Default for ActivityBarBehavior {
    fn default() -> Self {
        Self {
            hover_expand: true,
            hover_intent: ActivityBarHoverIntent::default(),
        }
    }
}

/// Serializable activity-bar policy shared by hosts and views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityBarConfig {
    /// Pane edge to which the bar is attached, determining its layout axis and side.
    pub edge: ActivityBarEdge,
    /// Base visibility and overlay policy used when no per-pane resolver overrides it.
    pub mode: ActivityBarMode,
    /// Hover interaction policy for rendered bars.
    pub behavior: ActivityBarBehavior,
}

/// Host predicate selecting a visibility policy for each pane.
pub type ActivityBarModeResolver<D> = Arc<dyn Fn(&PaneId, &D) -> ActivityBarMode + Send + Sync>;
/// Host predicate selecting an optional pane border color.
pub type PaneBorderColor<D> = Arc<dyn Fn(&PaneId, &D) -> Option<Hsla> + Send + Sync>;

/// Host-owned positions around the trailing activity group.
#[derive(Clone)]
pub struct ActivityBarSlots<D: PaneData> {
    /// Optional application icon placed in the bar's primary group.
    pub app_icon: Option<crate::ActivityIcon>,
    /// Host renderer inserted at the start of the trailing activity group.
    pub leading: Option<ChromeRenderer<D>>,
    /// Host renderer inserted after the trailing activity nodes.
    pub trailing: Option<ChromeRenderer<D>>,
    /// Pane-specific host renderer inserted after `trailing` and before pane controls.
    pub pane_accessory: Option<ChromeRenderer<D>>,
}

impl<D: PaneData> Default for ActivityBarSlots<D> {
    fn default() -> Self {
        Self {
            app_icon: None,
            leading: None,
            trailing: None,
            pane_accessory: None,
        }
    }
}

impl<D: PaneData> ActivityBarSlots<D> {
    /// Creates empty host slots.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the optional application icon rendered in the bar's primary group.
    pub fn with_app_icon(mut self, icon: crate::ActivityIcon) -> Self {
        self.app_icon = Some(icon);
        self
    }

    /// Installs content immediately before the trailing activity nodes.
    ///
    /// The callback runs while each pane is rendered and receives that pane's
    /// stable id and data plus the current GPUI window and application contexts.
    pub fn with_leading(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.leading = Some(std::rc::Rc::new(render));
        self
    }

    /// Installs content immediately after the trailing activity nodes.
    ///
    /// The callback runs while each pane is rendered with the pane id and data.
    pub fn with_trailing(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.trailing = Some(std::rc::Rc::new(render));
        self
    }

    /// Installs pane-specific content between the trailing slot and pane controls.
    ///
    /// The callback is invoked during rendering for every pane; it may derive the
    /// returned element from both the pane id and its host-owned data.
    pub fn with_pane_accessory(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.pane_accessory = Some(std::rc::Rc::new(render));
        self
    }
}

/// Header-band behavior and an optional host-wide accessory.
#[derive(Clone)]
pub struct PaneHeaderConfig<D: PaneData> {
    /// Whether the fixed-height header band is rendered for panes with a selected activity.
    pub visible: bool,
    /// Optional host renderer appended after activity-provided header content.
    pub accessory: Option<ChromeRenderer<D>>,
}

impl<D: PaneData> PaneHeaderConfig<D> {
    /// Creates a visible header with no host accessory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a hidden header with no host accessory.
    pub fn hidden() -> Self {
        Self {
            visible: false,
            accessory: None,
        }
    }

    /// Sets whether the header band is rendered.
    ///
    /// The band is still omitted when a pane has no selected activity.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Installs host content after the selected activity's header content.
    ///
    /// The callback runs during each pane render with that pane's id and data.
    pub fn with_accessory(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.accessory = Some(std::rc::Rc::new(render));
        self
    }
}

impl<D: PaneData> Default for PaneHeaderConfig<D> {
    fn default() -> Self {
        Self {
            visible: true,
            accessory: None,
        }
    }
}

/// Non-view host configuration for pane-specific activity chrome.
#[derive(Clone)]
pub struct ActivityBarHostConfig<D: PaneData> {
    /// Default edge, mode, and hover behavior for all panes.
    pub activity_bar: ActivityBarConfig,
    /// Optional per-pane mode callback, taking precedence over `activity_bar.mode`.
    pub mode: Option<ActivityBarModeResolver<D>>,
    /// Host-provided elements placed around built-in activity and pane controls.
    pub slots: ActivityBarSlots<D>,
    /// Pane header visibility and host accessory configuration.
    pub header: PaneHeaderConfig<D>,
    /// Optional per-pane border-color callback; `None` preserves the normal style color.
    pub pane_border_color: Option<PaneBorderColor<D>>,
}

impl<D: PaneData> Default for ActivityBarHostConfig<D> {
    fn default() -> Self {
        Self {
            activity_bar: ActivityBarConfig::default(),
            mode: None,
            slots: ActivityBarSlots::default(),
            header: PaneHeaderConfig::default(),
            pane_border_color: None,
        }
    }
}

impl<D: PaneData> ActivityBarHostConfig<D> {
    /// Creates the default pinned-left host configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the default activity-bar policy.
    pub fn with_activity_bar(mut self, config: ActivityBarConfig) -> Self {
        self.activity_bar = config;
        self
    }

    /// Installs a thread-safe callback that selects a mode for each pane.
    ///
    /// The resolver receives the pane id and borrowed host data whenever its
    /// effective mode is requested, and overrides [`ActivityBarConfig::mode`].
    pub fn with_mode_resolver(
        mut self,
        resolver: impl Fn(&PaneId, &D) -> ActivityBarMode + Send + Sync + 'static,
    ) -> Self {
        self.mode = Some(Arc::new(resolver));
        self
    }

    /// Replaces all host-rendered activity-bar slots.
    pub fn with_slots(mut self, slots: ActivityBarSlots<D>) -> Self {
        self.slots = slots;
        self
    }

    /// Replaces pane-header visibility and accessory configuration.
    pub fn with_header(mut self, header: PaneHeaderConfig<D>) -> Self {
        self.header = header;
        self
    }

    /// Installs a thread-safe callback that selects an optional pane border color.
    ///
    /// Returning `None` requests the normal theme/style border for that pane.
    pub fn with_pane_border_color(
        mut self,
        color: impl Fn(&PaneId, &D) -> Option<Hsla> + Send + Sync + 'static,
    ) -> Self {
        self.pane_border_color = Some(Arc::new(color));
        self
    }

    /// Resolves the effective activity-bar mode for a pane.
    ///
    /// Calls the configured resolver when present; otherwise returns the base
    /// [`ActivityBarConfig::mode`].
    pub fn mode_for(&self, pane: &PaneId, data: &D) -> ActivityBarMode {
        self.mode
            .as_ref()
            .map_or(self.activity_bar.mode, |resolve| resolve(pane, data))
    }
}

/// Pure category expansion state, keyed only by stable category ids.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivityExpansionState {
    expanded: HashSet<crate::CategoryId>,
}

impl ActivityExpansionState {
    /// Returns whether the category id is currently expanded.
    pub fn is_expanded(&self, id: &crate::CategoryId) -> bool {
        self.expanded.contains(id)
    }

    /// Toggles a category and returns its new expansion state.
    pub fn toggle(&mut self, id: crate::CategoryId) -> bool {
        if self.expanded.remove(&id) {
            false
        } else {
            self.expanded.insert(id);
            true
        }
    }

    /// Expand every active ancestor without collapsing user-opened categories.
    pub fn reveal_active<'a>(&mut self, path: impl IntoIterator<Item = &'a crate::CategoryId>) {
        self.expanded.extend(path.into_iter().cloned());
    }

    /// Collapses every category, including categories expanded by active-path reveal.
    pub fn clear(&mut self) {
        self.expanded.clear();
    }

    /// Borrows the complete set of expanded stable category ids.
    pub fn expanded(&self) -> &HashSet<crate::CategoryId> {
        &self.expanded
    }
}

/// Token returned on pointer entry. A delayed callback may expand only while
/// its token is still current, preventing stale timers from reopening the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverGeneration(
    /// Monotonically wrapping token value used to reject obsolete callbacks.
    pub u64,
);

/// Logical hover-intent state for one bar or bar item.
///
/// Entry issues a generation token for a possibly delayed open. Leaving
/// immediately clears logical expansion and invalidates all outstanding tokens;
/// animation timing is intentionally owned by the caller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityBarHoverState {
    generation: u64,
    hovered: bool,
    expanded: bool,
}

impl ActivityBarHoverState {
    /// Records pointer entry and returns the only token currently allowed to open.
    ///
    /// Calling this again invalidates any token returned by an earlier entry.
    pub fn enter(&mut self) -> HoverGeneration {
        self.generation = self.generation.wrapping_add(1);
        self.hovered = true;
        HoverGeneration(self.generation)
    }

    /// Collapse immediately and invalidate all delayed entry callbacks.
    pub fn leave(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.hovered = false;
        self.expanded = false;
    }

    /// Applies a delayed open only if its token is current and the pointer remains inside.
    ///
    /// Returns `true` when the open is accepted (including an already-expanded
    /// state), and `false` for stale tokens or after pointer leave.
    pub fn apply_open(&mut self, generation: HoverGeneration) -> bool {
        if self.hovered && generation.0 == self.generation {
            self.expanded = true;
            true
        } else {
            false
        }
    }

    /// Returns whether the logical pointer state is inside the tracked region.
    pub const fn is_hovered(self) -> bool {
        self.hovered
    }

    /// Returns whether a valid open callback has expanded this logical state.
    pub const fn is_expanded(self) -> bool {
        self.expanded
    }

    /// Returns the current token, primarily for diagnostics and state inspection.
    pub const fn generation(self) -> HoverGeneration {
        HoverGeneration(self.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CategoryId;

    #[test]
    fn every_edge_maps_to_the_expected_axis_and_side() {
        assert_eq!(ActivityBarEdge::Left.axis(), ActivityBarAxis::Vertical);
        assert_eq!(ActivityBarEdge::Right.axis(), ActivityBarAxis::Vertical);
        assert_eq!(ActivityBarEdge::Top.axis(), ActivityBarAxis::Horizontal);
        assert_eq!(ActivityBarEdge::Bottom.axis(), ActivityBarAxis::Horizontal);
        assert!(!ActivityBarEdge::Left.is_trailing());
        assert!(ActivityBarEdge::Right.is_trailing());
        assert!(!ActivityBarEdge::Top.is_trailing());
        assert!(ActivityBarEdge::Bottom.is_trailing());
        assert_eq!(ActivityBarEdge::Left.opposite(), ActivityBarEdge::Right);
        assert_eq!(ActivityBarEdge::Top.opposite(), ActivityBarEdge::Bottom);
    }

    #[test]
    fn config_defaults_and_serde_are_stable() {
        let default: ActivityBarConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default.edge, ActivityBarEdge::Left);
        assert_eq!(default.mode, ActivityBarMode::Pinned);
        assert!(default.behavior.hover_expand);
        assert_eq!(default.behavior.hover_intent.expand_delay_ms, 0);
        assert_eq!(default.behavior.hover_intent.transition_duration_ms, 150);

        let config = ActivityBarConfig {
            edge: ActivityBarEdge::Bottom,
            mode: ActivityBarMode::AutoHide,
            behavior: ActivityBarBehavior {
                hover_expand: false,
                hover_intent: ActivityBarHoverIntent {
                    expand_delay_ms: 175,
                    transition_duration_ms: 240,
                },
            },
        };
        let json = serde_json::to_value(config).unwrap();
        assert_eq!(json["edge"], "Bottom");
        assert_eq!(json["mode"], "AutoHide");
        assert_eq!(json["behavior"]["hover_intent"]["expand_delay_ms"], 175);
        assert_eq!(
            json["behavior"]["hover_intent"]["transition_duration_ms"],
            240
        );
        assert_eq!(
            serde_json::from_value::<ActivityBarConfig>(json).unwrap(),
            config
        );
    }

    #[test]
    fn expansion_reveals_active_path_without_forgetting_manual_state() {
        let mut state = ActivityExpansionState::default();
        assert!(state.toggle(CategoryId::new("manual")));
        let active = [CategoryId::new("outer"), CategoryId::new("inner")];
        state.reveal_active(&active);
        assert!(state.is_expanded(&CategoryId::new("manual")));
        assert!(state.is_expanded(&CategoryId::new("outer")));
        assert!(state.is_expanded(&CategoryId::new("inner")));
        assert!(!state.toggle(CategoryId::new("outer")));
    }

    #[test]
    fn stale_hover_generations_cannot_reopen_after_leave_or_reentry() {
        let mut hover = ActivityBarHoverState::default();
        let first = hover.enter();
        hover.leave();
        assert!(!hover.apply_open(first));
        assert!(!hover.is_expanded());

        let old = hover.enter();
        let current = hover.enter();
        assert!(!hover.apply_open(old));
        assert!(hover.apply_open(current));
        assert!(hover.is_hovered());
        assert!(hover.is_expanded());
        hover.leave();
        assert!(!hover.is_expanded(), "leave is intentionally zero-delay");
    }
}
