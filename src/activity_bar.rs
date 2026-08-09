use crate::{ChromeRenderer, PaneData, PaneId};
use gpui::{App, Hsla, Window};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Main/cross axis used by activity-bar layout code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarAxis {
    Horizontal,
    Vertical,
}

/// Edge of a pane occupied by its activity bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarEdge {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

impl ActivityBarEdge {
    pub const fn axis(self) -> ActivityBarAxis {
        match self {
            Self::Left | Self::Right => ActivityBarAxis::Vertical,
            Self::Top | Self::Bottom => ActivityBarAxis::Horizontal,
        }
    }

    pub const fn is_horizontal(self) -> bool {
        matches!(self.axis(), ActivityBarAxis::Horizontal)
    }

    /// Whether the bar is on the trailing side of its cross axis.
    pub const fn is_trailing(self) -> bool {
        matches!(self, Self::Right | Self::Bottom)
    }

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
    Move,
    SplitHorizontal,
    SplitVertical,
    Close,
}

impl PaneControl {
    pub const ORDER: [Self; 4] = [
        Self::Move,
        Self::SplitHorizontal,
        Self::SplitVertical,
        Self::Close,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::SplitHorizontal => "split-h",
            Self::SplitVertical => "split-v",
            Self::Close => "close",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::SplitHorizontal => "Split H",
            Self::SplitVertical => "Split V",
            Self::Close => "Close",
        }
    }

    pub fn debug_selector(self, pane: &PaneId) -> String {
        format!("pane-control:{}:{}", self.key(), pane.0)
    }

    pub fn accessibility_id(self, pane: &PaneId) -> String {
        format!("mullion-pane-{}-{}", self.key(), pane.0)
    }
}

/// Per-pane visibility policy for an activity bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarMode {
    #[default]
    Pinned,
    Hidden,
    AutoHide,
}

/// Open-only hover intent. Leaving always invalidates a pending open and
/// collapses immediately.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityBarHoverIntent {
    pub expand_delay_ms: u32,
}

/// Interaction semantics independent of styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityBarBehavior {
    pub hover_expand: bool,
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
    pub edge: ActivityBarEdge,
    pub mode: ActivityBarMode,
    pub behavior: ActivityBarBehavior,
}

/// Host predicate selecting a visibility policy for each pane.
pub type ActivityBarModeResolver<D> = Arc<dyn Fn(&PaneId, &D) -> ActivityBarMode + Send + Sync>;
/// Host predicate selecting an optional pane border color.
pub type PaneBorderColor<D> = Arc<dyn Fn(&PaneId, &D) -> Option<Hsla> + Send + Sync>;

/// Host-owned positions around the trailing activity group.
#[derive(Clone)]
pub struct ActivityBarSlots<D: PaneData> {
    pub app_icon: Option<crate::ActivityIcon>,
    pub leading: Option<ChromeRenderer<D>>,
    pub trailing: Option<ChromeRenderer<D>>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_app_icon(mut self, icon: crate::ActivityIcon) -> Self {
        self.app_icon = Some(icon);
        self
    }

    pub fn with_leading(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.leading = Some(std::rc::Rc::new(render));
        self
    }

    pub fn with_trailing(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> gpui::AnyElement + 'static,
    ) -> Self {
        self.trailing = Some(std::rc::Rc::new(render));
        self
    }

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
    pub visible: bool,
    pub accessory: Option<ChromeRenderer<D>>,
}

impl<D: PaneData> PaneHeaderConfig<D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hidden() -> Self {
        Self {
            visible: false,
            accessory: None,
        }
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

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
    pub activity_bar: ActivityBarConfig,
    pub mode: Option<ActivityBarModeResolver<D>>,
    pub slots: ActivityBarSlots<D>,
    pub header: PaneHeaderConfig<D>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_activity_bar(mut self, config: ActivityBarConfig) -> Self {
        self.activity_bar = config;
        self
    }

    pub fn with_mode_resolver(
        mut self,
        resolver: impl Fn(&PaneId, &D) -> ActivityBarMode + Send + Sync + 'static,
    ) -> Self {
        self.mode = Some(Arc::new(resolver));
        self
    }

    pub fn with_slots(mut self, slots: ActivityBarSlots<D>) -> Self {
        self.slots = slots;
        self
    }

    pub fn with_header(mut self, header: PaneHeaderConfig<D>) -> Self {
        self.header = header;
        self
    }

    pub fn with_pane_border_color(
        mut self,
        color: impl Fn(&PaneId, &D) -> Option<Hsla> + Send + Sync + 'static,
    ) -> Self {
        self.pane_border_color = Some(Arc::new(color));
        self
    }

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
    pub fn is_expanded(&self, id: &crate::CategoryId) -> bool {
        self.expanded.contains(id)
    }

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

    pub fn clear(&mut self) {
        self.expanded.clear();
    }

    pub fn expanded(&self) -> &HashSet<crate::CategoryId> {
        &self.expanded
    }
}

/// Token returned on pointer entry. A delayed callback may expand only while
/// its token is still current, preventing stale timers from reopening the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverGeneration(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityBarHoverState {
    generation: u64,
    hovered: bool,
    expanded: bool,
}

impl ActivityBarHoverState {
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

    pub fn apply_open(&mut self, generation: HoverGeneration) -> bool {
        if self.hovered && generation.0 == self.generation {
            self.expanded = true;
            true
        } else {
            false
        }
    }

    pub const fn is_hovered(self) -> bool {
        self.hovered
    }

    pub const fn is_expanded(self) -> bool {
        self.expanded
    }

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

        let config = ActivityBarConfig {
            edge: ActivityBarEdge::Bottom,
            mode: ActivityBarMode::AutoHide,
            behavior: ActivityBarBehavior {
                hover_expand: false,
                hover_intent: ActivityBarHoverIntent {
                    expand_delay_ms: 175,
                },
            },
        };
        let json = serde_json::to_value(config).unwrap();
        assert_eq!(json["edge"], "Bottom");
        assert_eq!(json["mode"], "AutoHide");
        assert_eq!(json["behavior"]["hover_intent"]["expand_delay_ms"], 175);
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
