use crate::{
    Activity, ActivityBarEdge, ActivityBarHostConfig, ActivityBarHoverState, ActivityBarMode,
    ActivityCache, ActivityCacheKey, ActivityCatalog, ActivityCatalogValidationError,
    ActivityExpansionState, ActivityFactoryRegistry, ActivityId, ActivityNode, ActivityProjection,
    DockBounds, DockConfig, DockDrag, DockHover, DockPayload, DropEdge, FocusPresentation,
    MullionModel, MullionOverlay, MullionSettings, MullionStyles, MullionTheme, MullionThemeMode,
    NewPaneFactory, OverlayAlignment, OverlayError, OverlayHostConfig, OverlayLength, PaletteEntry,
    PaletteInvocation, PaletteInvocationError, PaletteSearchResult, PaneCommandExecutionOptions,
    PaneControl, PaneData, PaneDirection, PaneEvent, PaneFocusBehavior, PaneId, PaneNode,
    PaneSplitFactory, SplitDirection, VisibleActivityNode, WorkspaceChanged, WorkspaceEvent,
    WorkspaceId, WorkspaceSet, WorkspaceSetError,
};
use gpui::{
    actions, canvas, div, ease_in_out, point, prelude::*, px, relative, AnyElement, App, Bounds,
    Context, DragMoveEvent, Element, ElementId, EventEmitter, FocusHandle, GlobalElementId, Hsla,
    InspectorElementId, LayoutId, MouseButton, PathBuilder, Pixels, Point, SharedString,
    StyleRefinement, Window,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

actions!(
    mullion,
    [
        FocusLeft,
        FocusRight,
        FocusUp,
        FocusDown,
        FocusNext,
        FocusPrevious,
        ClosePane,
        ToggleZoom,
        BalancePanes,
        ResizeSplitDecrease,
        ResizeSplitIncrease,
        CancelSplitResize
    ]
);

const KEYBOARD_RESIZE_STEP: f64 = 0.05;
/// Key context activated after a splitter is directly selected.
pub const MULLION_SPLITTER_KEY_CONTEXT: &str = "MullionSplitter";

#[derive(Clone, Copy, Default)]
struct InternalEdges {
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
}

#[derive(Clone, Copy)]
struct PaneRenderPosition<'a> {
    pane_ids: &'a [PaneId],
    edges: InternalEdges,
}

type PaneActivityKey = (Option<WorkspaceId>, PaneId);
type PaneActivitySource<D> = (Option<ActivityId>, D);

#[derive(Clone, Copy)]
struct PaneMoveRenderStyle {
    size: Pixels,
    row_extent: Pixels,
    icon_size: Pixels,
    end_padding: Pixels,
    focus_progress: f32,
    theme: MullionTheme,
    horizontal: bool,
}

struct PaneControlRenderStyle {
    size: Pixels,
    row_extent: Pixels,
    icon_size: Pixels,
    label_size: Pixels,
    show_label: bool,
    label_opacity: f32,
    end_padding: Pixels,
    theme: MullionTheme,
}

type SplitBounds = Rc<RefCell<HashMap<PaneId, Bounds<Pixels>>>>;
type ActiveSplit = Rc<RefCell<Option<(PaneId, f64)>>>;

const ACTIVITY_BAR_TRANSITION: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActivityMotion {
    progress: f32,
    from: f32,
    target: bool,
    started_at: Option<scheduler::Instant>,
    generation: u64,
}

impl Default for ActivityMotion {
    fn default() -> Self {
        Self {
            progress: 0.0,
            from: 0.0,
            target: false,
            started_at: None,
            generation: 0,
        }
    }
}

impl ActivityMotion {
    fn endpoint(target: bool) -> f32 {
        if target {
            1.0
        } else {
            0.0
        }
    }

    fn resolved(&self, reduce_motion: bool) -> f32 {
        if reduce_motion {
            Self::endpoint(self.target)
        } else {
            self.progress
        }
    }

    fn advance(&mut self, now: scheduler::Instant, duration: Duration) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };
        let delta =
            now.saturating_duration_since(started_at).as_secs_f32() / duration.as_secs_f32();
        self.progress = self.from
            + (Self::endpoint(self.target) - self.from) * ease_in_out(delta.clamp(0.0, 1.0));
        if delta >= 1.0 {
            self.progress = Self::endpoint(self.target);
            self.started_at = None;
        }
        self.started_at.is_some()
    }

    fn start(
        &mut self,
        target: bool,
        now: scheduler::Instant,
        duration: Duration,
        immediate: bool,
        from: Option<f32>,
    ) -> Option<u64> {
        self.advance(now, duration);
        if let Some(from) = from {
            self.progress = from.clamp(0.0, 1.0);
        }
        self.target = target;
        self.generation = self.generation.wrapping_add(1);
        let endpoint = Self::endpoint(target);
        if immediate || (self.progress - endpoint).abs() <= f32::EPSILON {
            self.progress = endpoint;
            self.from = endpoint;
            self.started_at = None;
            None
        } else {
            self.from = self.progress;
            self.started_at = Some(now);
            Some(self.generation)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MotionKey {
    Bar(PaneId),
    Item(PaneId, String),
    Focus(PaneId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActivityMotionSample {
    progress: f32,
    vertical_extent: f32,
    edge_padding: f32,
    row_extent: f32,
    label_opacity: f32,
    hidden_translation: f32,
}

fn activity_motion_sample(progress: f32) -> ActivityMotionSample {
    let progress = progress.clamp(0.0, 1.0);
    ActivityMotionSample {
        progress,
        vertical_extent: 28.0 + (158.0 - 28.0) * progress,
        edge_padding: 8.0 * progress,
        row_extent: 28.0 + (150.0 - 28.0) * progress,
        label_opacity: progress,
        hidden_translation: 1.0 - progress,
    }
}

fn interpolate_hsla(from: Hsla, to: Hsla, progress: f32) -> Hsla {
    let from = from.to_rgb();
    let to = to.to_rgb();
    let progress = progress.clamp(0.0, 1.0);
    gpui::Rgba {
        r: from.r + (to.r - from.r) * progress,
        g: from.g + (to.g - from.g) * progress,
        b: from.b + (to.b - from.b) * progress,
        a: from.a + (to.a - from.a) * progress,
    }
    .into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReferencePaneIcon {
    SplitHorizontal,
    SplitVertical,
    Close,
}

fn reference_icon_polygons(icon: ReferencePaneIcon) -> &'static [&'static [(f32, f32)]] {
    match icon {
        ReferencePaneIcon::SplitHorizontal => &[
            &[(1., 1.), (15., 1.), (15., 2.), (1., 2.)],
            &[(1., 14.), (15., 14.), (15., 15.), (1., 15.)],
            &[(1., 2.), (2., 2.), (2., 14.), (1., 14.)],
            &[(14., 2.), (15., 2.), (15., 14.), (14., 14.)],
            &[(7.5, 2.), (8.5, 2.), (8.5, 14.), (7.5, 14.)],
        ],
        ReferencePaneIcon::SplitVertical => &[
            &[(1., 1.), (15., 1.), (15., 2.), (1., 2.)],
            &[(1., 14.), (15., 14.), (15., 15.), (1., 15.)],
            &[(1., 2.), (2., 2.), (2., 14.), (1., 14.)],
            &[(14., 2.), (15., 2.), (15., 14.), (14., 14.)],
            &[(2., 7.5), (14., 7.5), (14., 8.5), (2., 8.5)],
        ],
        ReferencePaneIcon::Close => &[
            &[
                (3.647, 4.354),
                (4.354, 3.646),
                (12.354, 11.647),
                (11.646, 12.354),
            ],
            &[
                (11.647, 3.646),
                (12.354, 4.354),
                (4.354, 12.354),
                (3.647, 11.646),
            ],
        ],
    }
}

fn reference_pane_icon(icon: ReferencePaneIcon, size: Pixels, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let scale = size.as_f32() / 16.0;
            let at = |x: f32, y: f32| {
                point(
                    bounds.origin.x + px(x * scale),
                    bounds.origin.y + px(y * scale),
                )
            };
            let polygons = reference_icon_polygons(icon);
            for polygon in polygons {
                let mut builder = PathBuilder::fill();
                let points = polygon.iter().map(|&(x, y)| at(x, y)).collect::<Vec<_>>();
                builder.add_polygon(&points, true);
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .size(size)
    .into_any_element()
}

fn chevron_rotation(edge: ActivityBarEdge, expanded: bool) -> i32 {
    if !expanded {
        0
    } else if edge.is_horizontal() {
        180
    } else {
        90
    }
}

fn chevron_icon(edge: ActivityBarEdge, expanded: bool, size: Pixels, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let rotation = chevron_rotation(edge, expanded);
            let base = [(5.5_f32, 3.5_f32), (10.5, 8.), (5.5, 12.5)];
            let rotate = |(x, y): (f32, f32)| match rotation {
                90 => (16. - y, x),
                180 => (16. - x, 16. - y),
                _ => (x, y),
            };
            let scale = size.as_f32() / 16.0;
            let mut builder = PathBuilder::stroke(px(scale));
            let (x, y) = rotate(base[0]);
            builder.move_to(point(
                bounds.origin.x + px(x * scale),
                bounds.origin.y + px(y * scale),
            ));
            for point_value in base[1..].iter().copied().map(rotate) {
                builder.line_to(point(
                    bounds.origin.x + px(point_value.0 * scale),
                    bounds.origin.y + px(point_value.1 * scale),
                ));
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(size)
    .into_any_element()
}

struct SplitBoundsRecorder {
    key: PaneId,
    bounds: SplitBounds,
    child: AnyElement,
}

impl IntoElement for SplitBoundsRecorder {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SplitBoundsRecorder {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.bounds.borrow_mut().insert(self.key.clone(), bounds);
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

struct SplitDrag {
    split_key: PaneId,
    direction: SplitDirection,
    start_ratio: f64,
    start_cursor: Rc<Cell<Option<Point<Pixels>>>>,
    parent_bounds: Cell<Option<Bounds<Pixels>>>,
}

struct SplitDragPreview;
impl Render for SplitDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct LegacyActivityBody<D: PaneData> {
    pane: PaneId,
    data: D,
    render: crate::ActivityRenderer<D>,
}

impl<D: PaneData> Render for LegacyActivityBody<D> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        (self.render)(&self.pane, &self.data)
    }
}

#[derive(Clone)]
struct PaneActivityRenderData<D: PaneData> {
    activities: Vec<Activity<D>>,
    projection: ActivityProjection<D>,
}

impl Render for DockDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label = match &self.payload {
            DockPayload::Pane(id) => id.0.clone(),
            DockPayload::NewActivity(id) => id.0.clone(),
        };
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(gpui::blue().opacity(0.75))
            .text_color(gpui::white())
            .child(label)
    }
}

/// Shared native/WebAssembly GPUI view over the portable model.
pub struct MullionView<D: PaneData> {
    model: MullionModel<D>,
    dock_config: DockConfig<D>,
    command_options: PaneCommandExecutionOptions<D>,
    catalog: ActivityCatalog<D>,
    theme: MullionTheme,
    theme_mode: Option<MullionThemeMode>,
    styles: Option<MullionStyles>,
    host: ActivityBarHostConfig<D>,
    settings: MullionSettings,
    focus_presentation: FocusPresentation,
    expansion: HashMap<PaneId, ActivityExpansionState>,
    expansion_active: HashMap<PaneId, Option<ActivityId>>,
    hover: HashMap<PaneId, ActivityBarHoverState>,
    /// Horizontal rails expand only the row under the pointer.
    hovered_bar_items: HashSet<(PaneId, String)>,
    bar_motion: HashMap<PaneId, ActivityMotion>,
    item_motion: HashMap<(PaneId, String), ActivityMotion>,
    focus_motion: HashMap<PaneId, ActivityMotion>,
    motion_focus: Option<PaneId>,
    dock_drag_active: bool,
    focus_handle: FocusHandle,
    workspaces: Option<WorkspaceSet<D>>,
    activity_factories: ActivityFactoryRegistry<D>,
    activity_cache: ActivityCache<D>,
    activity_render_cache: HashMap<(Option<WorkspaceId>, PaneId), PaneActivityRenderData<D>>,
    activity_cache_dirty: bool,
    split_bounds: SplitBounds,
    split_starts: Rc<RefCell<HashMap<PaneId, Point<Pixels>>>>,
    active_split: ActiveSplit,
    keyboard_split: Option<PaneId>,
    dock_hover: Option<DockHover>,
    overlay_host: Option<OverlayHostConfig>,
    last_overlay_error: Option<OverlayError>,
    #[cfg(test)]
    routed_commands: Vec<crate::PaneCommand>,
    #[cfg(test)]
    activity_cache_syncs: usize,
    #[cfg(test)]
    split_move_mutations: usize,
    #[cfg(test)]
    notifications: usize,
}

impl<D: PaneData> EventEmitter<PaneEvent<D>> for MullionView<D> {}
impl<D: PaneData> EventEmitter<WorkspaceChanged> for MullionView<D> {}
impl<D: PaneData> EventEmitter<WorkspaceEvent<D>> for MullionView<D> {}

impl<D: PaneData> MullionView<D> {
    pub fn new(
        tree: PaneNode<D>,
        activities: Vec<ActivityNode<D>>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.on_release(|this, cx| this.dispose_all_activities(cx))
            .detach();
        Self {
            model: MullionModel::new(tree),
            dock_config: DockConfig::default(),
            command_options: PaneCommandExecutionOptions::default(),
            catalog: ActivityCatalog::new(activities),
            theme: MullionTheme::default(),
            theme_mode: None,
            styles: None,
            host: ActivityBarHostConfig::default(),
            settings: MullionSettings::default(),
            focus_presentation: FocusPresentation::default(),
            expansion: HashMap::new(),
            expansion_active: HashMap::new(),
            hover: HashMap::new(),
            hovered_bar_items: HashSet::new(),
            bar_motion: HashMap::new(),
            item_motion: HashMap::new(),
            focus_motion: HashMap::new(),
            motion_focus: None,
            dock_drag_active: false,
            focus_handle: cx.focus_handle(),
            workspaces: None,
            activity_factories: ActivityFactoryRegistry::new(),
            activity_cache: ActivityCache::default(),
            activity_render_cache: HashMap::new(),
            activity_cache_dirty: true,
            split_bounds: Rc::default(),
            split_starts: Rc::default(),
            active_split: Rc::default(),
            keyboard_split: None,
            dock_hover: None,
            overlay_host: None,
            last_overlay_error: None,
            #[cfg(test)]
            routed_commands: Vec::new(),
            #[cfg(test)]
            activity_cache_syncs: 0,
            #[cfg(test)]
            split_move_mutations: 0,
            #[cfg(test)]
            notifications: 0,
        }
    }

    /// Construct from a validated catalog while keeping [`Self::new`] source-compatible.
    pub fn try_new_with_catalog(
        tree: PaneNode<D>,
        catalog: ActivityCatalog<D>,
        cx: &mut Context<Self>,
    ) -> Result<Self, ActivityCatalogValidationError> {
        catalog.validate()?;
        let mut view = Self::new(tree, Vec::new(), cx);
        view.catalog = catalog;
        Ok(view)
    }

    /// Construct a view which owns and renders a set of internal workspaces.
    /// Returns `None` when validation fails, preserving the original optional API.
    pub fn new_with_workspaces(
        workspaces: WorkspaceSet<D>,
        activities: Vec<ActivityNode<D>>,
        cx: &mut Context<Self>,
    ) -> Option<Self> {
        Self::try_new_with_workspaces(workspaces, activities, cx).ok()
    }

    /// Typed constructor for persisted workspace input.
    pub fn try_new_with_workspaces(
        workspaces: WorkspaceSet<D>,
        activities: Vec<ActivityNode<D>>,
        cx: &mut Context<Self>,
    ) -> Result<Self, WorkspaceSetError> {
        workspaces.validate()?;
        let tree = workspaces
            .active()
            .expect("validated workspace set has an active workspace")
            .tree
            .clone();
        let mut view = Self::new(tree, activities, cx);
        view.workspaces = Some(workspaces);
        Ok(view)
    }
    pub fn with_theme(mut self, theme: MullionTheme) -> Self {
        self.theme = theme;
        self.theme_mode = None;
        self
    }
    /// Resolve the palette from the window appearance on every render.
    pub fn with_theme_mode(mut self, mode: MullionThemeMode) -> Self {
        self.theme_mode = Some(mode);
        self
    }
    pub fn theme_mode(&self) -> Option<MullionThemeMode> {
        self.theme_mode
    }
    pub fn with_styles(mut self, styles: MullionStyles) -> Self {
        self.styles = Some(styles);
        self
    }
    pub fn styles(&self) -> Option<&MullionStyles> {
        self.styles.as_ref()
    }
    pub fn with_activity_catalog(
        mut self,
        catalog: ActivityCatalog<D>,
    ) -> Result<Self, ActivityCatalogValidationError> {
        catalog.validate()?;
        self.catalog = catalog;
        Ok(self)
    }
    pub fn activity_catalog(&self) -> &ActivityCatalog<D> {
        &self.catalog
    }
    pub fn with_activity_bar_host(mut self, host: ActivityBarHostConfig<D>) -> Self {
        self.host = host;
        self
    }
    pub fn activity_bar_host(&self) -> &ActivityBarHostConfig<D> {
        &self.host
    }
    /// Install a controlled window-level overlay host.
    pub fn with_overlay_host(mut self, host: OverlayHostConfig) -> Self {
        self.overlay_host = Some(host);
        self.last_overlay_error = None;
        self
    }
    /// Replace or remove the controlled overlay host.
    pub fn set_overlay_host(&mut self, host: Option<OverlayHostConfig>, cx: &mut Context<Self>) {
        self.overlay_host = host;
        self.last_overlay_error = None;
        cx.notify();
    }
    /// Return the current controlled overlay host.
    pub fn overlay_host(&self) -> Option<&OverlayHostConfig> {
        self.overlay_host.as_ref()
    }
    /// Return the validation error from the most recent overlay snapshot.
    pub fn last_overlay_error(&self) -> Option<&OverlayError> {
        self.last_overlay_error.as_ref()
    }
    /// Configure activity-to-new-pane docking.
    pub fn with_dock_config(mut self, config: DockConfig<D>) -> Self {
        self.dock_config = config;
        self
    }
    /// Replace the activity docking configuration used by subsequent drags.
    pub fn set_dock_config(&mut self, config: DockConfig<D>, cx: &mut Context<Self>) {
        self.dock_config = config;
        self.dock_hover = None;
        cx.notify();
    }
    /// Return the activity docking configuration.
    pub fn dock_config(&self) -> &DockConfig<D> {
        &self.dock_config
    }
    /// Configure the host factory which mints panes for dropped activities.
    pub fn with_new_pane_factory(
        mut self,
        factory: impl Fn(&ActivityId, &PaneId, DropEdge) -> Option<(PaneId, D)> + Send + Sync + 'static,
    ) -> Self {
        self.dock_config = self.dock_config.with_new_pane_factory(factory);
        self
    }
    /// Replace the shared host factory used by activity docking.
    pub fn set_new_pane_factory(
        &mut self,
        factory: Option<NewPaneFactory<D>>,
        cx: &mut Context<Self>,
    ) {
        self.dock_config.set_new_pane_factory(factory);
        self.dock_hover = None;
        cx.notify();
    }
    /// Return the host pane factory, if configured.
    pub fn new_pane_factory(&self) -> Option<&NewPaneFactory<D>> {
        self.dock_config.new_pane_factory()
    }
    /// Install live focus settings. The callbacks are read on every render and
    /// pointer interaction, so host-controlled changes do not require rebuilding the view.
    pub fn with_settings(mut self, settings: MullionSettings) -> Self {
        self.settings = settings;
        self
    }
    /// Return the live settings handle used by this view.
    pub fn settings(&self) -> &MullionSettings {
        &self.settings
    }
    /// Configure the dispatcher used by all Mullion GPUI command actions.
    pub fn with_command_execution_options(
        mut self,
        options: PaneCommandExecutionOptions<D>,
    ) -> Self {
        self.command_options = options;
        self
    }
    /// Replace the dispatcher configuration used by subsequent actions.
    pub fn set_command_execution_options(&mut self, options: PaneCommandExecutionOptions<D>) {
        self.command_options = options;
    }
    /// Return the dispatcher configuration shared by every command action.
    pub fn command_execution_options(&self) -> &PaneCommandExecutionOptions<D> {
        &self.command_options
    }
    /// Configure the host callback used by split actions.
    pub fn with_split_factory_fn(
        mut self,
        factory: impl Fn(&PaneId, SplitDirection, &D) -> Option<(PaneId, D)> + Send + Sync + 'static,
    ) -> Self {
        self.command_options = self.command_options.with_split_factory_fn(factory);
        self
    }
    /// Configure a shared host callback used by split actions.
    pub fn with_split_factory(mut self, factory: PaneSplitFactory<D>) -> Self {
        self.command_options = self.command_options.with_split_factory(factory);
        self
    }
    /// Replace the host callback used by split actions.
    pub fn set_split_factory(&mut self, factory: Option<PaneSplitFactory<D>>) {
        self.command_options.set_split_factory(factory);
    }
    /// Return the host callback used by split actions, if configured.
    pub fn split_factory(&self) -> Option<&PaneSplitFactory<D>> {
        self.command_options.split_factory()
    }
    /// Configure the proportional step used by command-level resize actions.
    pub fn with_resize_step(mut self, step: f64) -> Self {
        self.command_options = self.command_options.with_resize_step(step);
        self
    }
    /// Replace the proportional step used by command-level resize actions.
    pub fn set_resize_step(&mut self, step: f64) {
        self.command_options.set_resize_step(step);
    }
    /// Return the proportional step used by command-level resize actions.
    pub fn resize_step(&self) -> f64 {
        self.command_options.resize_step()
    }
    pub fn with_focus_behavior(mut self, behavior: PaneFocusBehavior) -> Self {
        self.settings = MullionSettings::local(behavior);
        self
    }
    pub fn focus_behavior(&self) -> PaneFocusBehavior {
        self.settings.focus_behavior()
    }
    pub fn set_focus_behavior(&mut self, behavior: PaneFocusBehavior, cx: &mut Context<Self>) {
        self.settings.set_focus_behavior(behavior);
        cx.notify();
    }
    /// Configure opt-in focus chrome and inactive-pane treatment.
    pub fn with_focus_presentation(mut self, presentation: FocusPresentation) -> Self {
        self.focus_presentation = presentation;
        self
    }
    /// Compatibility alias for hosts that call the visual configuration simply presentation.
    pub fn with_presentation(self, presentation: FocusPresentation) -> Self {
        self.with_focus_presentation(presentation)
    }
    pub const fn focus_presentation(&self) -> FocusPresentation {
        self.focus_presentation
    }
    pub const fn presentation(&self) -> FocusPresentation {
        self.focus_presentation()
    }
    pub fn set_focus_presentation(
        &mut self,
        presentation: FocusPresentation,
        cx: &mut Context<Self>,
    ) {
        self.focus_presentation = presentation;
        cx.notify();
    }
    pub fn with_headers(mut self, visible: bool) -> Self {
        self.host.header.visible = visible;
        self
    }
    /// Install stateful activity factories while preserving legacy renderers as fallback.
    pub fn with_activity_factories(mut self, factories: ActivityFactoryRegistry<D>) -> Self {
        self.activity_factories = factories;
        self
    }
    /// Register a stateful activity factory on an existing view.
    pub fn register_activity_factory(
        &mut self,
        id: ActivityId,
        factory: impl Fn(&PaneId, &D, &mut Window, &mut App) -> crate::ActivityInstance<D> + 'static,
    ) -> Option<crate::ActivityFactory<D>> {
        self.activity_factories.register(id, factory)
    }
    /// Explicitly dispose all cached activities. GPUI also calls this automatically
    /// when the Mullion root entity is released.
    pub fn clear_activity_cache(&mut self, cx: &mut App) {
        self.dispose_all_activities(cx);
    }
    fn dispose_all_activities(&mut self, cx: &mut App) {
        for instance in self.activity_cache.drain() {
            if let Some(dispose) = instance.dispose {
                dispose(cx);
            }
        }
    }
    pub fn model(&self) -> &MullionModel<D> {
        &self.model
    }
    /// Return a fresh host-palette projection of the mounted view.
    ///
    /// The projection always contains all 37 stable commands, one dynamic focus
    /// command per live pane, and the activities currently visible for the
    /// focused pane. Unsupported split commands remain discoverable and report
    /// `SplitUnavailable` when invoked.
    pub fn palette_entries(&self) -> Vec<PaletteEntry> {
        let panes = self.model.tree().leaf_ids();
        let mut entries = crate::mullion_palette_entries(&panes, true);
        if let Some(focused) = self.model.focused() {
            if let Some(PaneNode::Leaf { data, .. }) = self.model.tree().find(focused) {
                entries.extend(crate::activity_palette_entries(
                    &self.catalog,
                    focused,
                    data,
                ));
            }
        }
        entries
    }

    /// Search the current live palette projection.
    pub fn search_palette(&self, query: &str) -> Vec<PaletteSearchResult> {
        crate::search_palette(&self.palette_entries(), query)
    }

    /// Execute a typed palette invocation through the same configured command
    /// dispatcher and event forwarding path used by GPUI actions.
    pub fn invoke_palette(
        &mut self,
        invocation: PaletteInvocation,
        cx: &mut Context<Self>,
    ) -> Result<(), PaletteInvocationError> {
        let result = match invocation {
            PaletteInvocation::PaneCommand(command) => self
                .model
                .execute_with_options(command, &self.command_options)
                .map_err(PaletteInvocationError::Command),
            PaletteInvocation::SelectActivity { pane, activity } => {
                let Some(PaneNode::Leaf { data, .. }) = self.model.tree().find(&pane) else {
                    return Err(PaletteInvocationError::PaneNotFound(pane));
                };
                let visible = crate::activity_palette_entries(&self.catalog, &pane, data)
                    .into_iter()
                    .any(|entry| {
                        entry.invocation
                            == PaletteInvocation::SelectActivity {
                                pane: pane.clone(),
                                activity: activity.clone(),
                            }
                    });
                if !visible {
                    return Err(PaletteInvocationError::ActivityNotVisible { pane, activity });
                }
                debug_assert!(self.model.set_activity(&pane, Some(activity)));
                Ok(())
            }
        };
        self.finish(cx);
        result
    }
    /// The internally owned workspace set, when this view was constructed with one.
    pub fn workspaces(&self) -> Option<&WorkspaceSet<D>> {
        self.workspaces.as_ref()
    }
    /// Compatibility-explicit alias for hosts which call the aggregate a workspace set.
    pub fn workspace_set(&self) -> Option<&WorkspaceSet<D>> {
        self.workspaces()
    }
    pub fn active_workspace(&self) -> Option<&crate::Workspace<D>> {
        self.workspaces.as_ref()?.active()
    }
    pub fn workspace(&self, id: &WorkspaceId) -> Option<&crate::Workspace<D>> {
        self.workspaces
            .as_ref()?
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
    }

    fn emit_workspace_snapshot(&self, cx: &mut Context<Self>) {
        if let Some(workspaces) = &self.workspaces {
            cx.emit(WorkspaceEvent::SnapshotChanged {
                workspaces: workspaces.clone(),
            });
        }
    }

    /// Add a workspace to this mounted view.
    pub fn add_workspace(
        &mut self,
        workspace: crate::Workspace<D>,
        cx: &mut Context<Self>,
    ) -> Result<usize, WorkspaceSetError> {
        let workspaces = self.workspaces.as_mut().ok_or(WorkspaceSetError::Empty)?;
        let index = workspaces.add(workspace)?;
        self.activity_cache_dirty = true;
        self.emit_workspace_snapshot(cx);
        cx.notify();
        Ok(index)
    }

    /// Remove a non-active workspace and dispose every cached activity in its namespace.
    pub fn remove_workspace(
        &mut self,
        id: &WorkspaceId,
        cx: &mut Context<Self>,
    ) -> Result<crate::Workspace<D>, WorkspaceSetError> {
        let workspaces = self.workspaces.as_mut().ok_or(WorkspaceSetError::Empty)?;
        let removed = workspaces.remove(id)?;
        self.activity_cache_dirty = true;
        for instance in self
            .activity_cache
            .remove_invalid(|key| key.workspace.as_ref() != Some(id))
        {
            if let Some(dispose) = instance.dispose {
                dispose(cx);
            }
        }
        self.emit_workspace_snapshot(cx);
        cx.notify();
        Ok(removed)
    }

    pub fn rename_workspace(
        &mut self,
        id: &WorkspaceId,
        name: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Result<String, WorkspaceSetError> {
        let workspaces = self.workspaces.as_mut().ok_or(WorkspaceSetError::Empty)?;
        let previous = workspaces.rename(id, name)?;
        self.emit_workspace_snapshot(cx);
        cx.notify();
        Ok(previous)
    }

    pub fn reorder_workspace(
        &mut self,
        id: &WorkspaceId,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Result<usize, WorkspaceSetError> {
        let workspaces = self.workspaces.as_mut().ok_or(WorkspaceSetError::Empty)?;
        let previous = workspaces.reorder(id, index)?;
        self.emit_workspace_snapshot(cx);
        cx.notify();
        Ok(previous)
    }

    /// Replace a stored tree. Updating the active tree uses the model's non-echo path.
    pub fn update_workspace_tree(
        &mut self,
        id: &WorkspaceId,
        tree: PaneNode<D>,
        cx: &mut Context<Self>,
    ) -> Result<PaneNode<D>, WorkspaceSetError> {
        let workspaces = self.workspaces.as_mut().ok_or(WorkspaceSetError::Empty)?;
        let old = workspaces.update_tree(id, tree.clone())?;
        self.activity_cache_dirty = true;
        if &workspaces.active == id {
            self.model.set_tree(tree);
            for event in self.model.take_events() {
                cx.emit(event);
            }
        }
        self.emit_workspace_snapshot(cx);
        cx.notify();
        Ok(old)
    }

    /// Switch the tree displayed in this same GPUI window/canvas.
    ///
    /// The operation is staged: invalid targets cannot persist or otherwise mutate the
    /// outgoing workspace. The outgoing model tree is persisted before the staged active
    /// id changes, and the incoming tree is installed through the non-echo model path.
    /// Focus and zoom use shared-ID reconciliation: each survives iff that pane id exists
    /// in the incoming tree; a surviving zoom owns focus, otherwise invalid focus falls
    /// back to the incoming tree's first leaf. Their transient events precede both the
    /// durable snapshot and `WorkspaceChanged`.
    pub fn try_switch_workspace(
        &mut self,
        id: &WorkspaceId,
        cx: &mut Context<Self>,
    ) -> Result<bool, WorkspaceSetError> {
        let current = self.workspaces.as_ref().ok_or(WorkspaceSetError::Empty)?;
        if &current.active == id {
            return Ok(false);
        }
        let previous = current.active.clone();
        let mut staged = current.clone();
        staged.try_persist_active(self.model.snapshot())?;
        let tree = staged.try_switch(id)?;

        self.model.set_tree(tree);
        self.workspaces = Some(staged);
        self.activity_cache_dirty = true;
        for event in self.model.take_events() {
            cx.emit(event);
        }
        self.emit_workspace_snapshot(cx);
        cx.emit(WorkspaceChanged {
            previous,
            active: id.clone(),
        });
        cx.notify();
        Ok(true)
    }

    /// Compatibility boolean switching API. A same-workspace request succeeds without
    /// mutation or notification.
    pub fn switch_workspace(&mut self, id: &WorkspaceId, cx: &mut Context<Self>) -> bool {
        match self.try_switch_workspace(id, cx) {
            Ok(changed) => {
                changed
                    || self
                        .workspaces
                        .as_ref()
                        .is_some_and(|set| &set.active == id)
            }
            Err(_) => false,
        }
    }
    /// The stable focus handle used for key-action dispatch. Hosts should focus it
    /// after creating the view (and pane pointer interaction does so automatically).
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }
    /// Mutate the portable model while safely forwarding every resulting event
    /// through GPUI and scheduling a repaint.
    pub fn update_model<R>(
        &mut self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut MullionModel<D>) -> R,
    ) -> R {
        let result = update(&mut self.model);
        self.finish(cx);
        result
    }
    fn finish(&mut self, cx: &mut Context<Self>) {
        let events = self.model.take_events();
        let tree_changed = events
            .iter()
            .any(|event| matches!(event, PaneEvent::TreeChanged { .. }));
        let ratio_only = !events.is_empty()
            && events.len().is_multiple_of(2)
            && events.as_chunks::<2>().0.iter().all(|pair| {
                matches!(
                    pair,
                    [PaneEvent::Resized { .. }, PaneEvent::TreeChanged { .. }]
                )
            });
        if tree_changed {
            if !ratio_only {
                self.activity_cache_dirty = true;
            }
            if let Some(workspaces) = &mut self.workspaces {
                workspaces.persist_active(self.model.snapshot());
            }
        }
        let notify = !events.is_empty();
        for event in events {
            cx.emit(event)
        }
        if notify {
            #[cfg(test)]
            {
                self.notifications += 1;
            }
            cx.notify();
        }
    }
    /// Execute a command through the view, forwarding events and repainting.
    /// The factory is consulted only for [`crate::PaneCommand::Split`].
    pub fn execute<F>(
        &mut self,
        command: crate::PaneCommand,
        split_factory: F,
        cx: &mut Context<Self>,
    ) -> crate::PaneCommandResult
    where
        F: FnMut(&PaneId, SplitDirection, &D) -> Option<(PaneId, D)>,
    {
        let result = self.model.execute(command, split_factory);
        self.finish(cx);
        result
    }
    fn command(&mut self, command: crate::PaneCommand, cx: &mut Context<Self>) {
        #[cfg(test)]
        self.routed_commands.push(command);
        let _ = self
            .model
            .execute_with_options(command, &self.command_options);
        self.finish(cx);
    }
    fn resize_keyboard_split(&mut self, delta: f64, cx: &mut Context<Self>) {
        let Some(key) = self.keyboard_split.clone() else {
            return;
        };
        if let Some(ratio) = crate::tree::find_ratio(self.model.tree(), &key) {
            self.model.resize(&key, ratio + delta);
            self.finish(cx);
        }
    }
    fn cancel_split_resize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let split = self.active_split.borrow_mut().take();
        if !cx.stop_active_drag(window) {
            return;
        }
        self.dock_hover = None;
        if let Some((key, start_ratio)) = split {
            self.model.resize(&key, start_ratio);
            self.finish(cx);
        } else {
            cx.notify();
        }
    }
    fn all_activities(&self, data: &D) -> Vec<Activity<D>> {
        let mut out = Vec::new();
        for node in self.catalog.primary().iter().chain(self.catalog.trailing()) {
            let mut borrowed = Vec::new();
            node.activities(data, &mut borrowed);
            out.extend(borrowed.into_iter().cloned());
        }
        out
    }
    fn workspace_namespace(&self) -> Option<WorkspaceId> {
        self.workspaces.as_ref().map(|set| set.active.clone())
    }
    fn collect_panes(
        node: &PaneNode<D>,
        workspace: Option<WorkspaceId>,
        out: &mut HashMap<PaneActivityKey, PaneActivitySource<D>>,
    ) {
        match node {
            PaneNode::Leaf {
                id,
                active_activity,
                data,
            } => {
                out.insert(
                    (workspace, id.clone()),
                    (active_activity.clone(), data.clone()),
                );
            }
            PaneNode::Split { first, second, .. } => {
                Self::collect_panes(first, workspace.clone(), out);
                Self::collect_panes(second, workspace, out);
            }
        }
    }
    fn sync_activity_cache(&mut self, window: &mut Window, cx: &mut App) {
        if !self.activity_cache_dirty {
            return;
        }
        self.activity_cache_dirty = false;
        #[cfg(test)]
        {
            self.activity_cache_syncs += 1;
        }
        let mut panes = HashMap::new();
        if let Some(workspaces) = &self.workspaces {
            for workspace in &workspaces.workspaces {
                let tree = if workspace.id == workspaces.active {
                    self.model.tree()
                } else {
                    &workspace.tree
                };
                Self::collect_panes(tree, Some(workspace.id.clone()), &mut panes);
            }
        } else {
            Self::collect_panes(self.model.tree(), None, &mut panes);
        }

        let mut render_cache = HashMap::with_capacity(panes.len());
        let mut valid = HashSet::new();
        let mut pane_data = HashMap::with_capacity(panes.len());
        for ((workspace, pane), (active, data)) in &panes {
            let activities = self.all_activities(data);
            valid.extend(activities.iter().map(|activity| {
                ActivityCacheKey::new(workspace.clone(), pane.clone(), activity.id.clone())
            }));
            let projection = self.catalog.visible(data, active.as_ref());
            render_cache.insert(
                (workspace.clone(), pane.clone()),
                PaneActivityRenderData {
                    activities,
                    projection,
                },
            );
            pane_data.insert((workspace.clone(), pane.clone()), data.clone());
        }
        self.activity_render_cache = render_cache;

        for instance in self
            .activity_cache
            .remove_invalid(|key| valid.contains(key))
        {
            if let Some(dispose) = instance.dispose {
                dispose(cx);
            }
        }
        // Do not update an instance that became filtered out on this same data
        // change; eviction and disposal are its only lifecycle transitions.
        for (update, data) in self.activity_cache.changed_callbacks(&pane_data) {
            update(&data, window, cx);
        }
    }

    fn render_node(
        &mut self,
        node: &PaneNode<D>,
        pane_ids: &[PaneId],
        edges: InternalEdges,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let styles = self
            .styles
            .unwrap_or_else(|| MullionStyles::from_theme(self.theme));
        match node {
            PaneNode::Leaf {
                id,
                active_activity,
                data,
            } => self.render_leaf(
                id,
                active_activity.as_ref(),
                data,
                PaneRenderPosition { pane_ids, edges },
                window,
                cx,
            ),
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                // The first leaf of the second subtree is a collision-free key that
                // survives ratio changes and rerenders.
                let key = second.leftmost_leaf_id().clone();
                let (first_edges, second_edges) = match direction {
                    SplitDirection::Horizontal => (
                        InternalEdges {
                            top: edges.top,
                            right: true,
                            bottom: edges.bottom,
                            left: edges.left,
                        },
                        InternalEdges {
                            top: edges.top,
                            right: edges.right,
                            bottom: edges.bottom,
                            left: true,
                        },
                    ),
                    SplitDirection::Vertical => (
                        InternalEdges {
                            top: edges.top,
                            right: edges.right,
                            bottom: true,
                            left: edges.left,
                        },
                        InternalEdges {
                            top: true,
                            right: edges.right,
                            bottom: edges.bottom,
                            left: edges.left,
                        },
                    ),
                };
                let first_el = self.render_node(first, pane_ids, first_edges, window, cx);
                let second_el = self.render_node(second, pane_ids, second_edges, window, cx);
                let handle_color = styles.split_handle.color;
                let focused_color = styles.split_handle.hover_color;
                let handle_thickness = styles.split_handle.thickness;
                let hit_target_thickness = styles.split_handle.hover_target_thickness;
                let drag_bounds = self.split_bounds.clone();
                let drag_starts = self.split_starts.clone();
                let drag_active = self.active_split.clone();
                let drag_key = key.clone();
                let drag_direction = *direction;
                let drag_ratio = *ratio;
                let mouse_key = key.clone();
                let drag_start_cursor = Rc::new(Cell::new(None));
                let mouse_starts = self.split_starts.clone();
                let decrement_key = key.clone();
                let increment_key = key.clone();
                let arrow_key = key.clone();
                let split_accessibility =
                    crate::MullionAccessibilityNode::split(*direction, *ratio, false);

                // Keep the actual layout separator one pixel wide. Its absolutely
                // positioned child supplies an eight-pixel, centered hit target.
                let handle = div()
                    .id(SharedString::from(format!("split-handle:{}", key.0)))
                    .debug_selector({
                        let key = key.clone();
                        move || format!("split-handle:{}", key.0)
                    })
                    .relative()
                    .flex_shrink_0()
                    .when(*direction == SplitDirection::Horizontal, |element| {
                        element.w(handle_thickness).h_full()
                    })
                    .when(*direction == SplitDirection::Vertical, |element| {
                        element.h(handle_thickness).w_full()
                    })
                    .bg(handle_color)
                    .child(
                        div()
                            .id(SharedString::from(format!("split-hit-target:{}", key.0)))
                            .debug_selector({
                                let key = key.clone();
                                move || format!("split-hit-target:{}", key.0)
                            })
                            .absolute()
                            .focusable()
                            .tab_stop(true)
                            .role(gpui::Role::Splitter)
                            .accessibility_id(format!("mullion-splitter-{}", key.0))
                            .aria_label(split_accessibility.label)
                            .aria_description(split_accessibility.description)
                            .aria_min_numeric_value(0.1)
                            .aria_max_numeric_value(0.9)
                            .aria_numeric_value(*ratio)
                            .aria_numeric_value_step(KEYBOARD_RESIZE_STEP)
                            .aria_orientation(match direction {
                                SplitDirection::Horizontal => gpui::Orientation::Vertical,
                                SplitDirection::Vertical => gpui::Orientation::Horizontal,
                            })
                            .aria_keyshortcuts("Ctrl+Alt+[ Ctrl+Alt+]")
                            .when(*direction == SplitDirection::Horizontal, |element| {
                                element
                                    .left(-(hit_target_thickness - handle_thickness) / 2.)
                                    .w(hit_target_thickness)
                                    .h_full()
                                    .cursor_col_resize()
                            })
                            .when(*direction == SplitDirection::Vertical, |element| {
                                element
                                    .top(-(hit_target_thickness - handle_thickness) / 2.)
                                    .h(hit_target_thickness)
                                    .w_full()
                                    .cursor_row_resize()
                            })
                            .hover(move |element| element.bg(focused_color))
                            .block_mouse_except_scroll()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, _| {
                                    mouse_starts
                                        .borrow_mut()
                                        .insert(mouse_key.clone(), event.position);
                                    this.keyboard_split = Some(mouse_key.clone());
                                }),
                            )
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    let delta = match event.keystroke.key.as_str() {
                                        "left" | "up" => -KEYBOARD_RESIZE_STEP,
                                        "right" | "down" => KEYBOARD_RESIZE_STEP,
                                        _ => return,
                                    };
                                    if let Some(current) =
                                        crate::tree::find_ratio(this.model.tree(), &arrow_key)
                                    {
                                        this.model.resize(&arrow_key, current + delta);
                                        this.finish(cx);
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                            .on_a11y_action(gpui::AccessibleAction::Decrement, {
                                let view = cx.entity().downgrade();
                                move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        if let Some(current) = crate::tree::find_ratio(
                                            this.model.tree(),
                                            &decrement_key,
                                        ) {
                                            this.model.resize(
                                                &decrement_key,
                                                current - KEYBOARD_RESIZE_STEP,
                                            );
                                            this.finish(cx);
                                        }
                                    })
                                    .ok();
                                }
                            })
                            .on_a11y_action(gpui::AccessibleAction::Increment, {
                                let view = cx.entity().downgrade();
                                move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        if let Some(current) = crate::tree::find_ratio(
                                            this.model.tree(),
                                            &increment_key,
                                        ) {
                                            this.model.resize(
                                                &increment_key,
                                                current + KEYBOARD_RESIZE_STEP,
                                            );
                                            this.finish(cx);
                                        }
                                    })
                                    .ok();
                                }
                            })
                            .on_drag(
                                SplitDrag {
                                    split_key: drag_key,
                                    direction: drag_direction,
                                    start_ratio: drag_ratio,
                                    start_cursor: drag_start_cursor,
                                    parent_bounds: Cell::new(None),
                                },
                                move |drag, _, window, cx| {
                                    drag.start_cursor.set(Some(
                                        drag_starts
                                            .borrow()
                                            .get(&drag.split_key)
                                            .copied()
                                            .unwrap_or_else(|| window.mouse_position()),
                                    ));
                                    drag.parent_bounds
                                        .set(drag_bounds.borrow().get(&drag.split_key).copied());
                                    *drag_active.borrow_mut() =
                                        Some((drag.split_key.clone(), drag.start_ratio));
                                    cx.new(|_| SplitDragPreview)
                                },
                            ),
                    );

                let active_on_drop = self.active_split.clone();
                let move_handler_key = key.clone();
                let parent = div()
                    .id(SharedString::from(format!("split-container:{}", key.0)))
                    .debug_selector({
                        let key = key.clone();
                        move || format!("split-container:{}", key.0)
                    })
                    .size_full()
                    .flex()
                    .overflow_hidden()
                    .when(*direction == SplitDirection::Vertical, |element| {
                        element.flex_col()
                    })
                    .on_drag_move::<SplitDrag>(cx.listener(
                        move |this, event: &gpui::DragMoveEvent<SplitDrag>, _, cx| {
                            let drag = event.drag(cx);
                            // Drag-move bubbles through every ancestor split container. Only
                            // the physical splitter that created this drag may mutate it.
                            if drag.split_key != move_handler_key {
                                return;
                            }
                            let Some(start) = drag.start_cursor.get() else {
                                return;
                            };
                            let bounds = drag.parent_bounds.get().unwrap_or(event.bounds);
                            let (delta, extent) = match drag.direction {
                                SplitDirection::Horizontal => {
                                    (event.event.position.x - start.x, bounds.size.width)
                                }
                                SplitDirection::Vertical => {
                                    (event.event.position.y - start.y, bounds.size.height)
                                }
                            };
                            if extent > px(0.)
                                && this.model.resize(
                                    &drag.split_key,
                                    drag.start_ratio + f64::from(delta / extent),
                                )
                            {
                                #[cfg(test)]
                                {
                                    this.split_move_mutations += 1;
                                }
                                this.finish(cx);
                            }
                        },
                    ))
                    .on_drop::<SplitDrag>(move |_, _, _| {
                        active_on_drop.borrow_mut().take();
                    })
                    .child(
                        div()
                            .flex_none()
                            .when(*direction == SplitDirection::Horizontal, |element| {
                                element.w(relative(*ratio as f32)).h_full()
                            })
                            .when(*direction == SplitDirection::Vertical, |element| {
                                element.h(relative(*ratio as f32)).w_full()
                            })
                            .child(first_el),
                    )
                    .child(handle)
                    .child(div().flex_1().min_w_0().min_h_0().child(second_el));

                SplitBoundsRecorder {
                    key,
                    bounds: self.split_bounds.clone(),
                    child: parent.into_any_element(),
                }
                .into_any_element()
            }
        }
    }
    fn handle_dock_move(
        &mut self,
        destination: &PaneId,
        event: &DragMoveEvent<DockDrag>,
        cx: &mut Context<Self>,
    ) {
        self.set_dock_drag_active(true, cx);
        let drag = event.drag(cx);
        let can_drop = match &drag.payload {
            DockPayload::Pane(source) => source != destination,
            DockPayload::NewActivity(_) => self.dock_config.can_create_panes(),
        };
        let hover = (can_drop && event.bounds.contains(&event.event.position))
            .then(|| {
                let point = crate::DockPoint::new(
                    event.event.position.x.into(),
                    event.event.position.y.into(),
                );
                let bounds = DockBounds::new(
                    event.bounds.left().into(),
                    event.bounds.top().into(),
                    event.bounds.size.width.into(),
                    event.bounds.size.height.into(),
                );
                DockHover::from_point(destination.clone(), point, bounds)
                    .filter(|hover| hover.accepts(drag))
            })
            .flatten();
        if let Some(hover) = hover {
            if self.dock_hover.as_ref() != Some(&hover) {
                self.dock_hover = Some(hover);
                cx.notify();
            }
        } else if self
            .dock_hover
            .as_ref()
            .is_some_and(|hover| &hover.destination == destination)
        {
            self.dock_hover = None;
            cx.notify();
        }
    }

    fn handle_dock_drop(&mut self, drag: &DockDrag, destination: &PaneId, cx: &mut Context<Self>) {
        self.set_dock_drag_active(false, cx);
        let hover = self
            .dock_hover
            .take()
            .filter(|hover| &hover.destination == destination);
        let Some(hover) = hover.filter(|hover| hover.accepts(drag)) else {
            cx.notify();
            return;
        };
        match &drag.payload {
            DockPayload::Pane(source) if hover.edge == DropEdge::Center => {
                self.model.swap(source, destination);
            }
            DockPayload::Pane(source) => {
                self.model.move_pane(source, destination, hover.edge);
            }
            DockPayload::NewActivity(activity) => {
                self.dock_config
                    .drop_activity(&mut self.model, activity, destination, hover.edge);
            }
        }
        self.finish(cx);
    }

    fn motion_mut(&mut self, key: &MotionKey) -> &mut ActivityMotion {
        match key {
            MotionKey::Bar(pane) => self.bar_motion.entry(pane.clone()).or_default(),
            MotionKey::Item(pane, item) => self
                .item_motion
                .entry((pane.clone(), item.clone()))
                .or_default(),
            MotionKey::Focus(pane) => self.focus_motion.entry(pane.clone()).or_default(),
        }
    }

    fn start_motion(
        &mut self,
        key: MotionKey,
        target: bool,
        duration: Duration,
        from: Option<f32>,
        cx: &mut Context<Self>,
    ) {
        let immediate =
            cx.reduce_motion() || (self.dock_drag_active && !matches!(key, MotionKey::Focus(_)));
        let now = cx.background_executor().now();
        let Some(generation) = self
            .motion_mut(&key)
            .start(target, now, duration, immediate, from)
        else {
            return;
        };
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(15))
                .await;
            let keep_running = this
                .update(cx, |this, cx| {
                    let force_endpoint = cx.reduce_motion()
                        || (this.dock_drag_active && !matches!(key, MotionKey::Focus(_)));
                    let now = cx.background_executor().now();
                    let motion = this.motion_mut(&key);
                    if motion.generation != generation {
                        return false;
                    }
                    if force_endpoint {
                        motion.progress = ActivityMotion::endpoint(motion.target);
                        motion.from = motion.progress;
                        motion.started_at = None;
                    } else {
                        motion.advance(now, duration);
                    }
                    cx.notify();
                    motion.started_at.is_some()
                })
                .unwrap_or(false);
            if !keep_running {
                break;
            }
        })
        .detach();
    }

    fn set_dock_drag_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.dock_drag_active == active {
            return;
        }
        self.dock_drag_active = active;
        if active {
            for motion in self
                .bar_motion
                .values_mut()
                .chain(self.item_motion.values_mut())
            {
                motion.generation = motion.generation.wrapping_add(1);
                motion.progress = 1.0;
                motion.from = 1.0;
                motion.started_at = None;
            }
            cx.notify();
            return;
        }
        let bars = self
            .bar_motion
            .iter()
            .map(|(pane, motion)| (MotionKey::Bar(pane.clone()), motion.target))
            .collect::<Vec<_>>();
        let items = self
            .item_motion
            .iter()
            .map(|((pane, item), motion)| {
                (MotionKey::Item(pane.clone(), item.clone()), motion.target)
            })
            .collect::<Vec<_>>();
        for (key, target) in bars.into_iter().chain(items) {
            let motion = self.motion_mut(&key);
            motion.progress = 1.0;
            motion.from = 1.0;
            motion.started_at = None;
            self.start_motion(key, target, ACTIVITY_BAR_TRANSITION, Some(1.0), cx);
        }
        cx.notify();
    }

    fn sync_focus_motion(&mut self, cx: &mut Context<Self>) {
        let focused = self.model.focused().cloned();
        if self.motion_focus == focused {
            return;
        }
        let initial = self.motion_focus.is_none() && self.focus_motion.is_empty();
        if let Some(previous) = self.motion_focus.clone() {
            self.start_motion(
                MotionKey::Focus(previous),
                false,
                Duration::from_millis(125),
                None,
                cx,
            );
        }
        if let Some(next) = focused.clone() {
            let from = initial.then_some(1.0);
            self.start_motion(
                MotionKey::Focus(next),
                true,
                Duration::from_millis(125),
                from,
                cx,
            );
        }
        self.motion_focus = focused;
    }

    fn handle_activity_bar_hover(
        &mut self,
        pane: &PaneId,
        hovered: bool,
        delay_ms: u32,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            let state = self.hover.entry(pane.clone()).or_default();
            if state.is_hovered() {
                return;
            }
            let generation = state.enter();
            let pane = pane.clone();
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay_ms.into()))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this
                        .hover
                        .entry(pane.clone())
                        .or_default()
                        .apply_open(generation)
                    {
                        this.start_motion(
                            MotionKey::Bar(pane),
                            true,
                            ACTIVITY_BAR_TRANSITION,
                            None,
                            cx,
                        );
                        cx.notify();
                    }
                });
            })
            .detach();
        } else {
            let state = self.hover.entry(pane.clone()).or_default();
            if !state.is_hovered() {
                return;
            }
            state.leave();
            self.start_motion(
                MotionKey::Bar(pane.clone()),
                false,
                ACTIVITY_BAR_TRANSITION,
                None,
                cx,
            );
            let removed = self
                .hovered_bar_items
                .iter()
                .filter(|(item_pane, _)| item_pane == pane)
                .cloned()
                .collect::<Vec<_>>();
            self.hovered_bar_items
                .retain(|(item_pane, _)| item_pane != pane);
            for key in removed {
                self.start_motion(
                    MotionKey::Item(key.0, key.1),
                    false,
                    ACTIVITY_BAR_TRANSITION,
                    None,
                    cx,
                );
            }
            cx.notify();
        }
    }

    fn render_activity_nodes(
        &mut self,
        pane: &PaneId,
        nodes: &[VisibleActivityNode<D>],
        selected: Option<&ActivityId>,
        depth: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        fn contains_active<D: PaneData>(
            node: &VisibleActivityNode<D>,
            selected: Option<&ActivityId>,
        ) -> bool {
            match node {
                VisibleActivityNode::Activity(activity) => selected == Some(&activity.activity.id),
                VisibleActivityNode::Category(category) => category
                    .children
                    .iter()
                    .any(|child| contains_active(child, selected)),
            }
        }

        let styles = self
            .styles
            .unwrap_or_else(|| MullionStyles::from_theme(self.theme));
        let edge = self.host.activity_bar.edge;
        let horizontal = edge.is_horizontal();
        let mut rendered = Vec::new();
        for node in nodes {
            match node {
                VisibleActivityNode::Activity(visible) => {
                    let activity = &visible.activity;
                    let active = selected == Some(&activity.id);
                    let item_key = format!("activity:{}", activity.id.0);
                    let stored_item_progress = self
                        .item_motion
                        .entry((pane.clone(), item_key.clone()))
                        .or_default()
                        .progress;
                    let stored_bar_progress =
                        self.bar_motion.entry(pane.clone()).or_default().progress;
                    let item_progress = if self.dock_drag_active {
                        1.0
                    } else if horizontal {
                        stored_item_progress
                    } else {
                        stored_bar_progress
                    };
                    let item_sample = activity_motion_sample(item_progress);
                    let pane_id = pane.clone();
                    let down_pane_id = pane.clone();
                    let key_pane_id = pane.clone();
                    let activity_id = activity.id.clone();
                    let down_activity_id = activity.id.clone();
                    let key_activity_id = activity.id.clone();
                    let a11y_pane_id = pane.clone();
                    let a11y_activity_id = activity.id.clone();
                    let drag_activity_id = activity.id.clone();
                    let can_drag_activity = self.dock_config.can_create_panes();
                    let drag_arm_view = cx.entity().downgrade();
                    let drag_release_view = cx.entity().downgrade();
                    let drag_release_inside_view = cx.entity().downgrade();
                    let accessibility = crate::MullionAccessibilityNode::activity(
                        &activity.id,
                        activity.name.as_ref(),
                        active,
                    );
                    let icon = self
                        .catalog
                        .activity_chrome(&activity.id)
                        .and_then(|chrome| chrome.icon.clone());
                    let child = icon.map(|icon| icon.render(window, cx)).unwrap_or_else(|| {
                        div()
                            .text_size(styles.activity_bar.font_size)
                            .child(activity.name.clone())
                            .into_any_element()
                    });
                    let hover_pane = pane.clone();
                    let hover_key = item_key.clone();
                    let foreground = if active {
                        visible.inherited_color.unwrap_or(styles.activity_bar.icon)
                    } else {
                        styles.activity_bar.icon
                    };
                    let activity_label = div()
                        .debug_selector({
                            let pane = pane.clone();
                            let id = activity.id.clone();
                            move || format!("activity-label:{}:{}", pane.0, id.0)
                        })
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .opacity(item_sample.label_opacity)
                        .child(activity.name.clone());
                    let activity_label = activity_label.into_any_element();
                    let item = div()
                        .id(SharedString::from(format!(
                            "activity:{}:{}",
                            pane.0, activity.id.0
                        )))
                        .debug_selector({
                            let pane = pane.clone();
                            let activity = activity.id.clone();
                            move || format!("activity:{}:{}", pane.0, activity.0)
                        })
                        .role(gpui::Role::Button)
                        .accessibility_id(format!("mullion-activity-{}-{}", pane.0, activity.id.0))
                        .aria_label(accessibility.label)
                        .aria_description(if can_drag_activity {
                            format!("{}; drag to create pane", accessibility.description)
                        } else {
                            accessibility.description
                        })
                        .aria_selected(active)
                        .focusable()
                        .tab_stop(true)
                        .cursor_pointer()
                        .relative()
                        .flex()
                        .items_center()
                        .h(styles.activity_bar.thickness)
                        .flex_shrink_0()
                        .when(horizontal, |item| item.w(px(item_sample.row_extent)))
                        .when(!horizontal, |item| item.w_full())
                        .overflow_hidden()
                        .text_size(styles.activity_bar.font_size)
                        .text_color(foreground)
                        .opacity(if active {
                            styles.activity_bar.active_icon_opacity
                        } else {
                            styles.activity_bar.inactive_icon_opacity
                        })
                        .when(horizontal, |item| {
                            item.on_hover(cx.listener(move |this, hovered, _, cx| {
                                let key = (hover_pane.clone(), hover_key.clone());
                                let changed = if *hovered {
                                    this.hovered_bar_items.insert(key.clone())
                                } else {
                                    this.hovered_bar_items.remove(&key)
                                };
                                if changed {
                                    this.start_motion(
                                        MotionKey::Item(key.0, key.1),
                                        *hovered,
                                        ACTIVITY_BAR_TRANSITION,
                                        None,
                                        cx,
                                    );
                                    cx.notify();
                                }
                            }))
                        })
                        .when(!can_drag_activity, |item| {
                            item.on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.model.focus(&down_pane_id);
                                    this.model.set_activity(
                                        &down_pane_id,
                                        Some(down_activity_id.clone()),
                                    );
                                    this.finish(cx);
                                }),
                            )
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.model.focus(&pane_id);
                            this.model.set_activity(&pane_id, Some(activity_id.clone()));
                            this.finish(cx);
                        }))
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                                this.model.focus(&key_pane_id);
                                this.model
                                    .set_activity(&key_pane_id, Some(key_activity_id.clone()));
                                this.finish(cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_a11y_action(gpui::AccessibleAction::Click, {
                            let view = cx.entity().downgrade();
                            move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.model.focus(&a11y_pane_id);
                                    this.model.set_activity(
                                        &a11y_pane_id,
                                        Some(a11y_activity_id.clone()),
                                    );
                                    this.finish(cx);
                                })
                                .ok();
                            }
                        })
                        .when(can_drag_activity, |item| {
                            item.cursor_copy()
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    drag_arm_view
                                        .update(cx, |this, cx| this.set_dock_drag_active(true, cx))
                                        .ok();
                                })
                                .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                                    drag_release_inside_view
                                        .update(cx, |this, cx| {
                                            this.set_dock_drag_active(false, cx);
                                        })
                                        .ok();
                                })
                                .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                                    drag_release_view
                                        .update(cx, |this, cx| {
                                            this.set_dock_drag_active(false, cx);
                                        })
                                        .ok();
                                })
                                .on_drag(
                                    DockDrag::new_activity(drag_activity_id),
                                    |drag, _, _, cx| cx.new(|_| drag.clone()),
                                )
                        })
                        .child(
                            div()
                                .debug_selector({
                                    let pane = pane.clone();
                                    let id = activity.id.clone();
                                    move || format!("activity-icon-slot:{}:{}", pane.0, id.0)
                                })
                                .w(styles.activity_bar.thickness)
                                .h(styles.activity_bar.thickness)
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .size(styles.activity_bar.icon_size)
                                        .overflow_hidden()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(child),
                                ),
                        )
                        .child(activity_label);
                    let item = item.into_any_element();
                    rendered.push(item);
                }
                VisibleActivityNode::Category(category) => {
                    let expanded = self
                        .expansion
                        .get(pane)
                        .is_some_and(|state| state.is_expanded(&category.id));
                    let has_active = contains_active(node, selected);
                    let show_dot = !expanded && has_active;
                    let item_key = format!("category:{}", category.id.0);
                    let stored_item_progress = self
                        .item_motion
                        .entry((pane.clone(), item_key.clone()))
                        .or_default()
                        .progress;
                    let stored_bar_progress =
                        self.bar_motion.entry(pane.clone()).or_default().progress;
                    let item_progress = if self.dock_drag_active {
                        1.0
                    } else if horizontal {
                        stored_item_progress
                    } else {
                        stored_bar_progress
                    };
                    let item_sample = activity_motion_sample(item_progress);
                    let pane_id = pane.clone();
                    let category_id = category.id.clone();
                    let key_pane_id = pane.clone();
                    let key_category_id = category.id.clone();
                    let a11y_pane_id = pane.clone();
                    let a11y_category_id = category.id.clone();
                    let hover_pane = pane.clone();
                    let hover_key = item_key;
                    let accessibility = crate::MullionAccessibilityNode::category(
                        &category.id,
                        category.name.as_ref(),
                        expanded,
                    );
                    let icon = self
                        .catalog
                        .category_chrome(&category.id)
                        .and_then(|chrome| chrome.icon.clone());
                    let child = icon.map(|icon| icon.render(window, cx)).unwrap_or_else(|| {
                        div()
                            .text_size(styles.activity_bar.font_size)
                            .child(category.name.clone())
                            .into_any_element()
                    });
                    let dot = show_dot.then(|| {
                        div()
                            .debug_selector({
                                let pane = pane.clone();
                                let id = category.id.clone();
                                move || format!("activity-category-dot:{}:{}", pane.0, id.0)
                            })
                            .absolute()
                            .size(px(4.))
                            .rounded(px(2.))
                            .bg(category.color)
                            .when(edge == ActivityBarEdge::Left, |dot| {
                                dot.left(px(2.)).top(px(12.))
                            })
                            .when(edge == ActivityBarEdge::Right, |dot| {
                                dot.right(px(2.)).top(px(12.))
                            })
                            .when(edge == ActivityBarEdge::Top, |dot| {
                                dot.bottom(px(2.)).left(px(12.))
                            })
                            .when(edge == ActivityBarEdge::Bottom, |dot| {
                                dot.top(px(2.)).left(px(12.))
                            })
                    });
                    let label = div()
                        .id(SharedString::from(format!(
                            "activity-category:{}:{}",
                            pane.0, category.id.0
                        )))
                        .debug_selector({
                            let pane = pane.clone();
                            let category = category.id.clone();
                            move || format!("activity-category:{}:{}", pane.0, category.0)
                        })
                        .role(gpui::Role::Button)
                        .accessibility_id(format!(
                            "mullion-activity-category-{}-{}",
                            pane.0, category.id.0
                        ))
                        .aria_label(accessibility.label)
                        .aria_description(accessibility.description)
                        .aria_expanded(expanded)
                        .focusable()
                        .tab_stop(true)
                        .cursor_pointer()
                        .relative()
                        .flex()
                        .items_center()
                        .h(styles.activity_bar.thickness)
                        .flex_shrink_0()
                        .when(horizontal, |item| item.w(px(item_sample.row_extent)))
                        .when(!horizontal, |item| item.w_full())
                        .overflow_hidden()
                        .text_size(styles.activity_bar.font_size)
                        .font_weight(gpui::FontWeight(600.))
                        .text_color(styles.activity_bar.category_label)
                        .opacity(if expanded || has_active {
                            1.0
                        } else {
                            styles.activity_bar.inactive_icon_opacity
                        })
                        .when(horizontal, |item| {
                            item.on_hover(cx.listener(move |this, hovered, _, cx| {
                                let key = (hover_pane.clone(), hover_key.clone());
                                let changed = if *hovered {
                                    this.hovered_bar_items.insert(key.clone())
                                } else {
                                    this.hovered_bar_items.remove(&key)
                                };
                                if changed {
                                    this.start_motion(
                                        MotionKey::Item(key.0, key.1),
                                        *hovered,
                                        ACTIVITY_BAR_TRANSITION,
                                        None,
                                        cx,
                                    );
                                    cx.notify();
                                }
                            }))
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.model.focus(&pane_id);
                                this.expansion
                                    .entry(pane_id.clone())
                                    .or_default()
                                    .toggle(category_id.clone());
                                cx.notify();
                            }),
                        )
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                                this.model.focus(&key_pane_id);
                                this.expansion
                                    .entry(key_pane_id.clone())
                                    .or_default()
                                    .toggle(key_category_id.clone());
                                cx.notify();
                                cx.stop_propagation();
                            }
                        }))
                        .on_a11y_action(gpui::AccessibleAction::Click, {
                            let view = cx.entity().downgrade();
                            move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.model.focus(&a11y_pane_id);
                                    this.expansion
                                        .entry(a11y_pane_id.clone())
                                        .or_default()
                                        .toggle(a11y_category_id.clone());
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .child(
                            div()
                                .relative()
                                .w(styles.activity_bar.thickness)
                                .h(styles.activity_bar.thickness)
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .when_some(dot, |slot, dot| slot.child(dot))
                                .child(
                                    div()
                                        .size(styles.activity_bar.icon_size)
                                        .overflow_hidden()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(child),
                                ),
                        )
                        .child({
                            let text = div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .opacity(item_sample.label_opacity)
                                .child(category.name.clone());
                            text.into_any_element()
                        })
                        .child({
                            let chevron = div()
                                .ml_auto()
                                .w(px(14.))
                                .h(px(14.))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(0.5 * item_sample.label_opacity)
                                .child(chevron_icon(
                                    edge,
                                    expanded,
                                    px(9.),
                                    styles.activity_bar.category_label,
                                ));
                            chevron.into_any_element()
                        });

                    let label = label.into_any_element();

                    let mut wrapper = div()
                        .debug_selector({
                            let pane = pane.clone();
                            let id = category.id.clone();
                            move || format!("activity-category-card:{}:{}", pane.0, id.0)
                        })
                        .relative()
                        .flex_shrink_0()
                        .when(horizontal, |wrapper| wrapper.flex().flex_row().h_full())
                        .when(!horizontal, |wrapper| wrapper.flex().flex_col().w_full())
                        .when(depth == 0 && edge == ActivityBarEdge::Left, |wrapper| {
                            wrapper.mr(-styles.activity_bar.expanded_padding)
                        })
                        .when(depth == 0 && edge == ActivityBarEdge::Right, |wrapper| {
                            wrapper.ml(-styles.activity_bar.expanded_padding)
                        })
                        .bg(if expanded {
                            styles.activity_bar.category_card_background
                        } else {
                            Hsla::transparent_black()
                        })
                        .border_color(if expanded {
                            styles.activity_bar.category_edge
                        } else {
                            Hsla::transparent_black()
                        })
                        .when(horizontal, |wrapper| wrapper.border_l(px(1.)))
                        .when(!horizontal, |wrapper| wrapper.border_t(px(1.)))
                        .child(label);
                    if expanded {
                        let border = div()
                            .debug_selector({
                                let pane = pane.clone();
                                let id = category.id.clone();
                                move || format!("activity-category-stripe:{}:{}", pane.0, id.0)
                            })
                            .absolute()
                            .bg(category.color)
                            .when(edge == ActivityBarEdge::Left, |b| {
                                b.left_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(styles.activity_bar.category_border_width)
                            })
                            .when(edge == ActivityBarEdge::Right, |b| {
                                b.right_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(styles.activity_bar.category_border_width)
                            })
                            .when(edge == ActivityBarEdge::Top, |b| {
                                b.left_0()
                                    .right_0()
                                    .bottom_0()
                                    .h(styles.activity_bar.category_border_width)
                            })
                            .when(edge == ActivityBarEdge::Bottom, |b| {
                                b.left_0()
                                    .right_0()
                                    .top_0()
                                    .h(styles.activity_bar.category_border_width)
                            });
                        let children = self.render_activity_nodes(
                            pane,
                            &category.children,
                            selected,
                            depth + 1,
                            window,
                            cx,
                        );
                        wrapper = wrapper.child(
                            div()
                                .debug_selector({
                                    let pane = pane.clone();
                                    let id = category.id.clone();
                                    move || {
                                        format!("activity-category-children:{}:{}", pane.0, id.0)
                                    }
                                })
                                .relative()
                                .when(horizontal, |c| c.flex().flex_row().h_full())
                                .when(!horizontal, |c| c.flex().flex_col().w_full())
                                .child(border)
                                .children(children),
                        );
                    }
                    rendered.push(wrapper.into_any_element());
                }
            }
        }
        rendered
    }

    fn render_pane_command_control(
        pane: &PaneId,
        control: PaneControl,
        command: crate::PaneCommand,
        enabled: bool,
        style: PaneControlRenderStyle,
        horizontal: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let icon = match control {
            PaneControl::Move => None,
            PaneControl::SplitHorizontal => Some(ReferencePaneIcon::SplitHorizontal),
            PaneControl::SplitVertical => Some(ReferencePaneIcon::SplitVertical),
            PaneControl::Close => Some(ReferencePaneIcon::Close),
        };
        let selector = control.debug_selector(pane);
        let accessibility_id = control.accessibility_id(pane);
        let pane_click = pane.clone();
        let pane_key = pane.clone();
        let pane_a11y = pane.clone();
        let click_command = command;
        let key_command = command;
        let a11y_command = command;
        let click_view = cx.entity().downgrade();
        let key_view = cx.entity().downgrade();
        let a11y_view = cx.entity().downgrade();
        let hover_pane = pane.clone();
        let hover_key = format!("control:{}", control.key());
        let mut element = div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .role(gpui::Role::Button)
            .accessibility_id(accessibility_id)
            .aria_label(control.label())
            .aria_description(if enabled {
                format!("{} pane {}", control.label(), pane.0)
            } else {
                format!("{} pane {} (unavailable)", control.label(), pane.0)
            })
            .focusable()
            .tab_stop(enabled)
            .h(style.size)
            .w(style.row_extent)
            .flex_shrink_0()
            .flex()
            .items_center()
            .overflow_hidden()
            .pr(style.end_padding)
            .text_color(style.theme.text)
            .when(horizontal, |element| {
                element.on_hover(cx.listener(move |this, hovered, _, cx| {
                    let key = (hover_pane.clone(), hover_key.clone());
                    let changed = if *hovered {
                        this.hovered_bar_items.insert(key.clone())
                    } else {
                        this.hovered_bar_items.remove(&key)
                    };
                    if changed {
                        this.start_motion(
                            MotionKey::Item(key.0, key.1),
                            *hovered,
                            ACTIVITY_BAR_TRANSITION,
                            None,
                            cx,
                        );
                        cx.notify();
                    }
                }))
            })
            .when(enabled, |element| {
                element
                    .cursor_pointer()
                    .hover(|element| element.bg(style.theme.accent))
            })
            .when(!enabled, |element| {
                element.cursor_not_allowed().opacity(0.35)
            })
            .child(
                div()
                    .w(style.size)
                    .h(style.size)
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(style.icon_size)
                    .child(icon.map_or_else(
                        || div().child("⠿").into_any_element(),
                        |icon| reference_pane_icon(icon, style.icon_size, style.theme.text),
                    )),
            )
            .when(style.show_label, |element| {
                element.child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(style.label_size)
                        .whitespace_nowrap()
                        .opacity(style.label_opacity)
                        .child(control.label()),
                )
            });
        if enabled {
            element = element
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    click_view
                        .update(cx, |this, cx| {
                            this.model.focus(&pane_click);
                            this.command(click_command, cx);
                        })
                        .ok();
                })
                .on_key_down(move |event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                        key_view
                            .update(cx, |this, cx| {
                                this.model.focus(&pane_key);
                                this.command(key_command, cx);
                            })
                            .ok();
                        cx.stop_propagation();
                    }
                })
                .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                    a11y_view
                        .update(cx, |this, cx| {
                            this.model.focus(&pane_a11y);
                            this.command(a11y_command, cx);
                        })
                        .ok();
                });
        }
        element.into_any_element()
    }

    fn render_pane_move_control(
        pane: &PaneId,
        icon: Option<AnyElement>,
        style: PaneMoveRenderStyle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selector = PaneControl::Move.debug_selector(pane);
        let pane_drag = pane.clone();
        let pane_key = pane.clone();
        let drag_selector = format!("pane-drag-handle:{}", pane.0);
        let view = cx.entity().downgrade();
        let drag_arm_view = view.clone();
        let drag_release_view = view.clone();
        let drag_release_inside_view = view.clone();
        let hover_pane = pane.clone();
        let hover_key = "control:move".to_owned();
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .role(gpui::Role::Button)
            .accessibility_id(PaneControl::Move.accessibility_id(pane))
            .aria_label(PaneControl::Move.label())
            .aria_description(format!("Move pane {}", pane.0))
            .aria_keyshortcuts("Mullion move-pane commands")
            .focusable()
            .tab_stop(true)
            .h(style.size)
            .w(style.row_extent)
            .flex_shrink_0()
            .flex()
            .items_center()
            .pr(style.end_padding)
            .cursor_move()
            .text_color(style.theme.text)
            .when(style.horizontal, |element| {
                element.on_hover(cx.listener(move |this, hovered, _, cx| {
                    let key = (hover_pane.clone(), hover_key.clone());
                    let changed = if *hovered {
                        this.hovered_bar_items.insert(key.clone())
                    } else {
                        this.hovered_bar_items.remove(&key)
                    };
                    if changed {
                        this.start_motion(
                            MotionKey::Item(key.0, key.1),
                            *hovered,
                            ACTIVITY_BAR_TRANSITION,
                            None,
                            cx,
                        );
                        cx.notify();
                    }
                }))
            })
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                drag_arm_view
                    .update(cx, |this, cx| this.set_dock_drag_active(true, cx))
                    .ok();
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                drag_release_inside_view
                    .update(cx, |this, cx| {
                        this.set_dock_drag_active(false, cx);
                    })
                    .ok();
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                drag_release_view
                    .update(cx, |this, cx| {
                        this.set_dock_drag_active(false, cx);
                    })
                    .ok();
            })
            .on_drag(DockDrag::pane(pane_drag), |drag, _, _, cx| {
                cx.new(|_| drag.clone())
            })
            .on_key_down(move |event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                    view.update(cx, |this, cx| {
                        this.model.focus(&pane_key);
                        this.finish(cx);
                    })
                    .ok();
                    cx.stop_propagation();
                }
            })
            .child(
                div()
                    .w(style.size)
                    .h(style.size)
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .debug_selector(move || drag_selector.clone())
                            .size(style.icon_size)
                            .text_color(interpolate_hsla(
                                style.theme.text,
                                gpui::rgb(0x0974a4).into(),
                                style.focus_progress,
                            ))
                            .opacity(0.5 + 0.5 * style.focus_progress)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(style.icon_size)
                            .child(icon.unwrap_or_else(|| div().child("⠿").into_any_element())),
                    ),
            )
            .child(div().min_w_0().overflow_hidden().child(""))
            .into_any_element()
    }

    fn render_leaf(
        &mut self,
        id: &PaneId,
        active: Option<&ActivityId>,
        data: &D,
        position: PaneRenderPosition<'_>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane_ids = position.pane_ids;
        let edges = position.edges;
        let focused = self.model.focused() == Some(id);
        let pane_index = pane_ids.iter().position(|pane| pane == id).unwrap_or(0);
        let render_data = self
            .activity_render_cache
            .get(&(self.workspace_namespace(), id.clone()))
            .cloned()
            .unwrap_or_else(|| PaneActivityRenderData {
                activities: Vec::new(),
                projection: self.catalog.visible(data, active),
            });
        let active_name = active.and_then(|active| {
            render_data
                .activities
                .iter()
                .find(|activity| &activity.id == active)
                .map(|activity| activity.name.clone())
        });
        let pane_accessibility = crate::MullionAccessibilityNode::pane(
            id,
            pane_index,
            pane_ids.len(),
            active_name.as_deref(),
            focused,
            self.model.zoomed() == Some(id),
        );
        let theme = self.theme;
        let styles = self
            .styles
            .unwrap_or_else(|| MullionStyles::from_theme(theme));
        let pane_border = self
            .host
            .pane_border_color
            .as_ref()
            .and_then(|resolve| resolve(id, data))
            .unwrap_or(styles.pane.border);
        let id_focus_click = id.clone();
        let id_focus_hover = id.clone();
        let click_focus_handle = self.focus_handle.clone();
        let hover_focus_handle = self.focus_handle.clone();
        let id_drop = id.clone();
        let id_move = id.clone();
        let can_create_panes = self.dock_config.can_create_panes();
        let selected = active
            .and_then(|a| {
                render_data
                    .activities
                    .iter()
                    .find(|activity| &activity.id == a)
                    .cloned()
            })
            .or_else(|| render_data.activities.first().cloned());
        let projection = render_data.projection;
        if self.expansion_active.get(id) != Some(&active.cloned()) {
            self.expansion
                .entry(id.clone())
                .or_default()
                .reveal_active(&projection.active_ancestors);
            self.expansion_active.insert(id.clone(), active.cloned());
        }
        let horizontal = self.host.activity_bar.edge.is_horizontal();
        let mode = self.host.mode_for(id, data);
        let hover_expanded = self.hover.get(id).is_some_and(|state| state.is_expanded());
        let panel_expanded = self.dock_drag_active
            || (hover_expanded && self.host.activity_bar.behavior.hover_expand);
        let selected_id = selected.as_ref().map(|activity| &activity.id);
        self.bar_motion.entry(id.clone()).or_default();
        for key in ["move", "split-h", "split-v", "close"] {
            self.item_motion
                .entry((id.clone(), format!("control:{key}")))
                .or_default();
        }
        let primary_tabs =
            self.render_activity_nodes(id, &projection.primary, selected_id, 0, window, cx);
        let trailing_tabs =
            self.render_activity_nodes(id, &projection.trailing, selected_id, 0, window, cx);
        let cached = selected.as_ref().and_then(|activity| {
            let key =
                ActivityCacheKey::new(self.workspace_namespace(), id.clone(), activity.id.clone());
            if self.activity_cache.get(&key).is_none() {
                let instance = if let Some(factory) = self.activity_factories.get(&activity.id) {
                    factory(id, data, window, cx)
                } else {
                    let body = cx.new(|_| LegacyActivityBody {
                        pane: id.clone(),
                        data: data.clone(),
                        render: activity.render.clone(),
                    });
                    let update_body = body.clone();
                    crate::ActivityInstance::new(body).with_update(move |data: &D, _, cx| {
                        update_body.update(cx, |body, cx| {
                            if body.data != *data {
                                body.data = data.clone();
                                cx.notify();
                            }
                        });
                    })
                };
                self.activity_cache
                    .insert(key.clone(), instance, data.clone());
            }
            self.activity_cache
                .get(&key)
                .map(|entry| (entry.instance.body.clone(), entry.instance.header.clone()))
        });
        let body = cached
            .as_ref()
            .map(|(body, _)| {
                body.clone()
                    .cached(StyleRefinement::default().size_full())
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_text)
                    .child("No activity")
                    .into_any_element()
            });
        let mut custom_headers = Vec::new();
        if let Some(custom) = cached.and_then(|(_, header)| header) {
            custom_headers.push(custom.into_any_element());
        }
        if let Some(render) = selected
            .as_ref()
            .and_then(|activity| self.catalog.activity_chrome(&activity.id))
            .and_then(|chrome| chrome.header.clone())
        {
            custom_headers.push(render(id, data, window, cx));
        }
        if let Some(render) = self.host.header.accessory.clone() {
            custom_headers.push(render(id, data, window, cx));
        }
        let header = (self.host.header.visible && selected.is_some()).then(|| {
            div()
                .h(styles.header.height)
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(styles.header.horizontal_padding)
                .gap(styles.header.gap)
                .border_b(styles.header.border_width)
                .border_color(styles.header.border)
                .bg(styles.header.background)
                .text_color(styles.header.text)
                .text_size(styles.header.font_size)
                .child(
                    div()
                        .text_color(styles.header.title)
                        .font_weight(gpui::FontWeight(styles.header.title_weight.into()))
                        .child(selected.as_ref().unwrap().name.clone()),
                )
                .children(custom_headers)
        });
        let app_icon = self
            .host
            .slots
            .app_icon
            .clone()
            .map(|icon| icon.render(window, cx));
        let leading_slot = self
            .host
            .slots
            .leading
            .clone()
            .map(|render| render(id, data, window, cx));
        let trailing_slot = self
            .host
            .slots
            .trailing
            .clone()
            .map(|render| render(id, data, window, cx));
        let pane_accessory = self
            .host
            .slots
            .pane_accessory
            .clone()
            .map(|render| render(id, data, window, cx));
        let can_split = self.command_options.split_factory().is_some();
        let can_close = pane_ids.len() > 1;
        let row_expanded = |key: &str| {
            panel_expanded
                && (!horizontal
                    || self.dock_drag_active
                    || self
                        .hovered_bar_items
                        .contains(&(id.clone(), format!("control:{key}"))))
        };
        let move_expanded = row_expanded("move");
        let split_h_expanded = row_expanded("split-h");
        let split_v_expanded = row_expanded("split-v");
        let close_expanded = row_expanded("close");
        let row_sample = |key: &str, expanded: bool| {
            activity_motion_sample(if self.dock_drag_active {
                1.0
            } else if horizontal {
                self.item_motion
                    .get(&(id.clone(), format!("control:{key}")))
                    .map_or(ActivityMotion::endpoint(expanded), |motion| {
                        motion.resolved(cx.reduce_motion())
                    })
            } else {
                self.bar_motion
                    .get(id)
                    .map_or(ActivityMotion::endpoint(expanded), |motion| {
                        motion.resolved(cx.reduce_motion())
                    })
            })
        };
        let move_sample = row_sample("move", move_expanded);
        let split_h_sample = row_sample("split-h", split_h_expanded);
        let split_v_sample = row_sample("split-v", split_v_expanded);
        let close_sample = row_sample("close", close_expanded);
        let focus_progress = self
            .focus_motion
            .get(id)
            .map_or(ActivityMotion::endpoint(focused), |motion| {
                motion.resolved(cx.reduce_motion())
            });
        let compact_move = (mode != ActivityBarMode::Hidden).then(|| {
            Self::render_pane_move_control(
                id,
                app_icon,
                PaneMoveRenderStyle {
                    size: styles.pane_controls.compact_size,
                    row_extent: px(move_sample.row_extent),
                    icon_size: styles.pane_controls.compact_icon_size,
                    end_padding: px(move_sample.edge_padding),
                    focus_progress,
                    theme,
                    horizontal,
                },
                cx,
            )
        });
        let compact_split_h = (mode != ActivityBarMode::Hidden).then(|| {
            Self::render_pane_command_control(
                id,
                PaneControl::SplitHorizontal,
                crate::PaneCommand::Split(SplitDirection::Horizontal),
                can_split,
                PaneControlRenderStyle {
                    size: styles.pane_controls.compact_size,
                    row_extent: px(split_h_sample.row_extent),
                    icon_size: styles.pane_controls.compact_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: true,
                    label_opacity: split_h_sample.label_opacity,
                    end_padding: px(split_h_sample.edge_padding),
                    theme,
                },
                horizontal,
                cx,
            )
        });
        let compact_split_v = (mode != ActivityBarMode::Hidden).then(|| {
            Self::render_pane_command_control(
                id,
                PaneControl::SplitVertical,
                crate::PaneCommand::Split(SplitDirection::Vertical),
                can_split,
                PaneControlRenderStyle {
                    size: styles.pane_controls.compact_size,
                    row_extent: px(split_v_sample.row_extent),
                    icon_size: styles.pane_controls.compact_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: true,
                    label_opacity: split_v_sample.label_opacity,
                    end_padding: px(split_v_sample.edge_padding),
                    theme,
                },
                horizontal,
                cx,
            )
        });
        let compact_close = (mode != ActivityBarMode::Hidden).then(|| {
            Self::render_pane_command_control(
                id,
                PaneControl::Close,
                crate::PaneCommand::Close,
                can_close,
                PaneControlRenderStyle {
                    size: styles.pane_controls.compact_size,
                    row_extent: px(close_sample.row_extent),
                    icon_size: styles.pane_controls.compact_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: true,
                    label_opacity: close_sample.label_opacity,
                    end_padding: px(close_sample.edge_padding),
                    theme,
                },
                horizontal,
                cx,
            )
        });
        let bar = (mode != ActivityBarMode::Hidden).then(|| {
            let pane = id.clone();
            let delay = self.host.activity_bar.behavior.hover_intent.expand_delay_ms;
            let auto_hide = mode == ActivityBarMode::AutoHide;
            let edge = self.host.activity_bar.edge;
            let motion_progress = if self.dock_drag_active {
                1.0
            } else {
                self.bar_motion
                    .get(id)
                    .map_or(ActivityMotion::endpoint(hover_expanded), |motion| {
                        motion.resolved(cx.reduce_motion())
                    })
            };
            let reveal_progress = if auto_hide { motion_progress } else { 1.0 };
            let expansion_progress = if self.host.activity_bar.behavior.hover_expand {
                motion_progress
            } else if self.dock_drag_active {
                1.0
            } else {
                0.0
            };
            let panel_sample = activity_motion_sample(expansion_progress);
            let panel_extent = px(panel_sample.vertical_extent);
            let edge_padding = px(panel_sample.edge_padding);

            let primary_group = div()
                .debug_selector({
                    let pane = id.clone();
                    move || format!("activity-bar-primary:{}", pane.0)
                })
                .flex()
                .flex_shrink_0()
                .when(horizontal, |group| group.flex_row().h_full())
                .when(!horizontal, |group| group.flex_col().w_full())
                .when_some(compact_move, |group, control| group.child(control))
                .children(primary_tabs);
            let trailing_group = div()
                .debug_selector({
                    let pane = id.clone();
                    move || format!("activity-bar-trailing:{}", pane.0)
                })
                .flex()
                .flex_shrink_0()
                .when(horizontal, |group| group.flex_row().h_full().ml_auto())
                .when(!horizontal, |group| group.flex_col().w_full().mt_auto())
                .when_some(leading_slot, |group, slot| group.child(slot))
                .children(trailing_tabs)
                .when_some(trailing_slot, |group, slot| group.child(slot))
                .when_some(pane_accessory, |group, accessory| group.child(accessory))
                .when_some(compact_split_h, |group, control| group.child(control))
                .when_some(compact_split_v, |group, control| group.child(control))
                .when_some(compact_close, |group, control| group.child(control));

            let panel_hover_pane = id.clone();
            let panel = div()
                .id(SharedString::from(format!("activity-bar-panel:{}", id.0)))
                .debug_selector({
                    let pane = id.clone();
                    move || format!("activity-bar-panel:{}", pane.0)
                })
                .absolute()
                .flex()
                .overflow_hidden()
                .bg(styles.activity_bar.background)
                .border_color(styles.activity_bar.border)
                .when(horizontal, |panel| {
                    panel
                        .left_0()
                        .right_0()
                        .h(styles.activity_bar.thickness)
                        .flex_row()
                })
                .when(!horizontal, |panel| {
                    panel.top_0().bottom_0().w(panel_extent).flex_col()
                })
                .when(edge == ActivityBarEdge::Left, |panel| {
                    panel
                        .left(-panel_extent * (1.0 - reveal_progress))
                        .border_r(styles.activity_bar.border_width)
                        .pr(edge_padding)
                })
                .when(edge == ActivityBarEdge::Right, |panel| {
                    panel
                        .right(-panel_extent * (1.0 - reveal_progress))
                        .border_l(styles.activity_bar.border_width)
                        .pl(edge_padding)
                })
                .when(edge == ActivityBarEdge::Top, |panel| {
                    panel
                        .top(-styles.activity_bar.thickness * (1.0 - reveal_progress))
                        .border_b(styles.activity_bar.border_width)
                })
                .when(edge == ActivityBarEdge::Bottom, |panel| {
                    panel
                        .bottom(-styles.activity_bar.thickness * (1.0 - reveal_progress))
                        .border_t(styles.activity_bar.border_width)
                })
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    this.handle_activity_bar_hover(&panel_hover_pane, *hovered, delay, cx);
                }))
                .child(primary_group)
                .child(trailing_group);
            let panel = panel.into_any_element();

            let edge_trigger = auto_hide.then(|| {
                let trigger_pane = id.clone();
                div()
                    .id(SharedString::from(format!("activity-bar-trigger:{}", id.0)))
                    .debug_selector({
                        let pane = id.clone();
                        move || format!("activity-bar-trigger:{}", pane.0)
                    })
                    .absolute()
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        this.handle_activity_bar_hover(&trigger_pane, *hovered, delay, cx);
                    }))
                    .when(edge == ActivityBarEdge::Left, |trigger| {
                        trigger.left_0().top_0().bottom_0().w(px(12.))
                    })
                    .when(edge == ActivityBarEdge::Right, |trigger| {
                        trigger.right_0().top_0().bottom_0().w(px(12.))
                    })
                    .when(edge == ActivityBarEdge::Top, |trigger| {
                        trigger.left_0().right_0().top_0().h(px(12.))
                    })
                    .when(edge == ActivityBarEdge::Bottom, |trigger| {
                        trigger.left_0().right_0().bottom_0().h(px(12.))
                    })
            });

            div()
                .id(SharedString::from(format!("activity-bar:{}", id.0)))
                .debug_selector({
                    let pane = id.clone();
                    move || format!("activity-bar:{}", pane.0)
                })
                .aria_label(format!("Activity bar for pane {}", id.0))
                .relative()
                .flex_shrink_0()
                .when(horizontal, |scope| {
                    scope.w_full().h(if auto_hide {
                        px(0.)
                    } else {
                        styles.activity_bar.thickness
                    })
                })
                .when(!horizontal, |scope| {
                    scope.h_full().w(if auto_hide {
                        px(0.)
                    } else {
                        styles.activity_bar.thickness
                    })
                })
                .on_hover(cx.listener(move |this, hovered, _, cx| {
                    this.handle_activity_bar_hover(&pane, *hovered, delay, cx);
                }))
                .when_some(edge_trigger, |scope, trigger| scope.child(trigger))
                .child(panel)
                .into_any_element()
        });

        let hidden_controls = (mode == ActivityBarMode::Hidden && focused).then(|| {
            let move_control = Self::render_pane_move_control(
                id,
                None,
                PaneMoveRenderStyle {
                    size: styles.pane_controls.hidden_size,
                    row_extent: styles.pane_controls.hidden_size,
                    icon_size: styles.pane_controls.hidden_icon_size,
                    end_padding: px(0.),
                    focus_progress,
                    theme,
                    horizontal: false,
                },
                cx,
            );
            let split_h = Self::render_pane_command_control(
                id,
                PaneControl::SplitHorizontal,
                crate::PaneCommand::Split(SplitDirection::Horizontal),
                can_split,
                PaneControlRenderStyle {
                    size: styles.pane_controls.hidden_size,
                    row_extent: styles.pane_controls.hidden_size,
                    icon_size: styles.pane_controls.hidden_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: false,
                    label_opacity: 0.0,
                    end_padding: px(0.),
                    theme,
                },
                false,
                cx,
            );
            let split_v = Self::render_pane_command_control(
                id,
                PaneControl::SplitVertical,
                crate::PaneCommand::Split(SplitDirection::Vertical),
                can_split,
                PaneControlRenderStyle {
                    size: styles.pane_controls.hidden_size,
                    row_extent: styles.pane_controls.hidden_size,
                    icon_size: styles.pane_controls.hidden_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: false,
                    label_opacity: 0.0,
                    end_padding: px(0.),
                    theme,
                },
                false,
                cx,
            );
            let close = Self::render_pane_command_control(
                id,
                PaneControl::Close,
                crate::PaneCommand::Close,
                can_close,
                PaneControlRenderStyle {
                    size: styles.pane_controls.hidden_size,
                    row_extent: styles.pane_controls.hidden_size,
                    icon_size: styles.pane_controls.hidden_icon_size,
                    label_size: styles.pane_controls.expanded_label_size,
                    show_label: false,
                    label_opacity: 0.0,
                    end_padding: px(0.),
                    theme,
                },
                false,
                cx,
            );
            div()
                .id(SharedString::from(format!("pane-controls:{}", id.0)))
                .debug_selector({
                    let id = id.clone();
                    move || format!("pane-controls:{}", id.0)
                })
                .aria_label(format!("Pane controls for {}", id.0))
                .absolute()
                .top(styles.pane_controls.capsule_inset - styles.pane.border_width)
                .right(styles.pane_controls.capsule_inset - styles.pane.border_width)
                .p(styles.pane_controls.capsule_padding)
                .gap(styles.pane_controls.capsule_gap)
                .rounded(styles.pane_controls.capsule_radius)
                .border(styles.pane_controls.capsule_border_width)
                .border_color(styles.pane_controls.capsule_border)
                .bg(styles.pane_controls.capsule_background)
                .opacity(styles.pane_controls.capsule_opacity)
                .flex()
                .items_center()
                .child(move_control)
                .child(split_h)
                .child(split_v)
                .child(close)
        });
        let unfocused_opacity = if focused {
            1.0
        } else {
            self.focus_presentation.unfocused_pane_opacity() as f32
        };
        let mut focus_edges = Vec::new();
        if focused && self.focus_presentation.show_focus_indicator() {
            let edge = |name: &'static str| {
                div()
                    .id(SharedString::from(format!("focus-edge:{}:{name}", id.0)))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("focus-edge:{}:{name}", id.0)
                    })
                    .absolute()
                    .bg(pane_border)
            };
            if edges.top {
                focus_edges.push(
                    edge("top")
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(styles.pane.focus_indicator_width),
                );
            }
            if edges.right {
                focus_edges.push(
                    edge("right")
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .w(styles.pane.focus_indicator_width),
                );
            }
            if edges.bottom {
                focus_edges.push(
                    edge("bottom")
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(styles.pane.focus_indicator_width),
                );
            }
            if edges.left {
                focus_edges.push(
                    edge("left")
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(styles.pane.focus_indicator_width),
                );
            }
        }
        div()
            .id(SharedString::from(format!("pane:{}", id.0)))
            .debug_selector({
                let id = id.clone();
                move || format!("pane:{}", id.0)
            })
            .role(gpui::Role::Pane)
            .accessibility_id(format!("mullion-pane-{}", id.0))
            .aria_label(pane_accessibility.label)
            .aria_description(pane_accessibility.description)
            .aria_selected(focused)
            .focusable()
            .tab_stop(true)
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .bg(styles.pane.background)
            .text_color(styles.pane.text)
            .border(styles.pane.border_width)
            .border_color(pane_border)
            .on_hover(cx.listener(move |this, hovered, window, cx| {
                if *hovered && this.settings.focus_behavior() == PaneFocusBehavior::Hover {
                    hover_focus_handle.focus(window, cx);
                    this.model.focus(&id_focus_hover);
                    this.finish(cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    if this.settings.focus_behavior() == PaneFocusBehavior::Click {
                        click_focus_handle.focus(window, cx);
                        this.model.focus(&id_focus_click);
                        this.finish(cx);
                    }
                }),
            )
            .can_drop({
                let destination = id_drop.clone();
                move |value, _, _| {
                    value
                        .downcast_ref::<DockDrag>()
                        .is_some_and(|drag| match &drag.payload {
                            DockPayload::Pane(source) => source != &destination,
                            DockPayload::NewActivity(_) => can_create_panes,
                        })
                }
            })
            .on_drag_move::<DockDrag>(cx.listener(move |this, event, _, cx| {
                this.handle_dock_move(&id_move, event, cx);
            }))
            .on_drop(cx.listener(move |this, drag: &DockDrag, _, cx| {
                this.handle_dock_drop(drag, &id_drop, cx);
            }))
            .child({
                let content = div()
                    .id(SharedString::from(format!("pane-content:{}", id.0)))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("pane-content:{}", id.0)
                    })
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .when_some(header, |content, header| content.child(header))
                    .child(div().flex_1().min_h_0().overflow_hidden().child(body));
                let visual = div()
                    .id(SharedString::from(format!("pane-visual:{}", id.0)))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("pane-visual:{}", id.0)
                    })
                    .size_full()
                    .relative()
                    .flex()
                    .when(horizontal, |visual| visual.flex_col())
                    .when(!horizontal, |visual| visual.flex_row())
                    .opacity(unfocused_opacity);
                if self.host.activity_bar.edge.is_trailing() {
                    visual
                        .child(content)
                        .when_some(bar, |visual, bar| visual.child(bar))
                } else {
                    visual
                        .when_some(bar, |visual, bar| visual.child(bar))
                        .child(content)
                }
            })
            .when_some(hidden_controls, |pane, controls| pane.child(controls))
            .when_some(
                self.dock_hover
                    .as_ref()
                    .filter(|hover| &hover.destination == id)
                    .map(|hover| hover.edge.normalized_indicator()),
                |pane, indicator| {
                    let active_edge = self.dock_hover.as_ref().unwrap().edge;
                    let zones = [
                        (DropEdge::Top, 0.25_f32, 0.0_f32, 0.5_f32, 0.25_f32),
                        (DropEdge::Bottom, 0.25, 0.75, 0.5, 0.25),
                        (DropEdge::Left, 0.0, 0.0, 0.25, 1.0),
                        (DropEdge::Right, 0.75, 0.0, 0.25, 1.0),
                        (DropEdge::Center, 0.25, 0.25, 0.5, 0.5),
                    ];
                    pane.children(zones.into_iter().map(|(edge, left, top, width, height)| {
                        let accessibility =
                            crate::MullionAccessibilityNode::drop_target(edge, edge == active_edge);
                        div()
                            .id(SharedString::from(format!("dock-target:{}:{edge:?}", id.0)))
                            .debug_selector({
                                let id = id.clone();
                                move || format!("dock-target:{}:{edge:?}", id.0)
                            })
                            .role(gpui::Role::Button)
                            .accessibility_id(format!("mullion-dock-{}-{edge:?}", id.0))
                            .aria_label(accessibility.label)
                            .aria_description(accessibility.description)
                            .aria_selected(edge == active_edge)
                            .focusable()
                            .tab_stop(true)
                            .absolute()
                            .left(relative(left))
                            .top(relative(top))
                            .w(relative(width))
                            .h(relative(height))
                    }))
                    .child(
                        div()
                            .debug_selector({
                                let id = id.clone();
                                move || format!("dock-indicator:{}:{active_edge:?}", id.0)
                            })
                            .absolute()
                            .left(relative(indicator.left as f32))
                            .top(relative(indicator.top as f32))
                            .w(relative(indicator.width as f32))
                            .h(relative(indicator.height as f32))
                            .bg(styles.drop_overlay.indicator_color),
                    )
                },
            )
            .children(focus_edges)
            .into_any_element()
    }

    fn render_overlay(
        overlay: MullionOverlay,
        host: &OverlayHostConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let policy = overlay.policy().clone();
        let id = policy.id.clone();
        let content = overlay.render(window, cx);

        let content = div()
            .id(SharedString::from(format!(
                "mullion-overlay-content:{}",
                id
            )))
            .debug_selector({
                let id = id.clone();
                move || format!("mullion-overlay-content:{}", id)
            })
            .when(
                matches!(policy.size.width, OverlayLength::Fill),
                |element| element.w_full(),
            )
            .when_some(
                match policy.size.width {
                    OverlayLength::Pixels(value) => Some(px(value)),
                    _ => None,
                },
                |element, width| element.w(width),
            )
            .when_some(
                match policy.size.width {
                    OverlayLength::Fraction(value) => Some(relative(value)),
                    _ => None,
                },
                |element, width| element.w(width),
            )
            .when(
                matches!(policy.size.height, OverlayLength::Fill),
                |element| element.h_full(),
            )
            .when_some(
                match policy.size.height {
                    OverlayLength::Pixels(value) => Some(px(value)),
                    _ => None,
                },
                |element, height| element.h(height),
            )
            .when_some(
                match policy.size.height {
                    OverlayLength::Fraction(value) => Some(relative(value)),
                    _ => None,
                },
                |element, height| element.h(height),
            )
            .when(
                policy.placement.horizontal == OverlayAlignment::Stretch,
                |element| element.w_full(),
            )
            .when(
                policy.placement.vertical == OverlayAlignment::Stretch,
                |element| element.h_full(),
            )
            .when(!policy.click_through, |element| {
                element
                    .block_mouse_except_scroll()
                    .on_click(|_, _, cx| cx.stop_propagation())
            })
            .when(policy.a11y_modal, |element| {
                element.role(gpui::Role::Dialog)
            })
            .when_some(policy.a11y_label.clone(), |element, label| {
                element.aria_label(label)
            })
            .accessibility_id(format!("mullion-overlay-{}", id))
            .child(content);

        let dismiss = host.on_dismiss().cloned();
        let blocks_input = !policy.click_through
            && (policy.tier == crate::OverlayTier::Modal || policy.backdrop.is_some());
        div()
            .id(SharedString::from(format!("mullion-overlay:{}", id)))
            .debug_selector({
                let id = id.clone();
                move || format!("mullion-overlay:{}", id)
            })
            .absolute()
            .inset_0()
            .flex()
            .when(
                policy.placement.horizontal == OverlayAlignment::Start,
                |element| element.justify_start(),
            )
            .when(
                policy.placement.horizontal == OverlayAlignment::Center,
                |element| element.justify_center(),
            )
            .when(
                policy.placement.horizontal == OverlayAlignment::End,
                |element| element.justify_end(),
            )
            .when(
                policy.placement.vertical == OverlayAlignment::Start,
                |element| element.items_start(),
            )
            .when(
                policy.placement.vertical == OverlayAlignment::Center,
                |element| element.items_center(),
            )
            .when(
                policy.placement.vertical == OverlayAlignment::End,
                |element| element.items_end(),
            )
            .when_some(policy.backdrop, |element, backdrop| {
                element.bg(gpui::Rgba {
                    r: backdrop.rgba[0],
                    g: backdrop.rgba[1],
                    b: backdrop.rgba[2],
                    a: backdrop.rgba[3],
                })
            })
            .when(blocks_input, |element| element.block_mouse_except_scroll())
            .when(policy.backdrop.is_some(), |element| {
                element.child(
                    div()
                        .debug_selector({
                            let id = id.clone();
                            move || format!("mullion-overlay-backdrop:{}", id)
                        })
                        .absolute()
                        .inset_0(),
                )
            })
            .when_some(policy.dismiss_on_backdrop.then_some(dismiss).flatten(), {
                let id = id.clone();
                move |element, dismiss| {
                    element.on_click(move |_, window, cx| dismiss(&id, window, cx))
                }
            })
            .child(content)
            .into_any_element()
    }

    fn render_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(host) = self.overlay_host.clone() else {
            self.last_overlay_error = None;
            return Vec::new();
        };
        match host.sorted_render_snapshot() {
            Ok(snapshot) => {
                self.last_overlay_error = None;
                snapshot
                    .into_iter()
                    .map(|overlay| Self::render_overlay(overlay, &host, window, cx))
                    .collect()
            }
            Err(error) => {
                self.last_overlay_error = Some(error);
                Vec::new()
            }
        }
    }
}

impl<D: PaneData> Render for MullionView<D> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(mode) = self.theme_mode {
            self.theme = mode.resolve(window.appearance());
        }
        let styles = self
            .styles
            .unwrap_or_else(|| MullionStyles::from_theme(self.theme));
        if !cx.has_active_drag() {
            self.dock_hover = None;
            self.set_dock_drag_active(false, cx);
        }
        self.sync_focus_motion(cx);
        self.sync_activity_cache(window, cx);
        let tree = self
            .model
            .zoomed()
            .and_then(|id| self.model.tree().find(id))
            .unwrap_or(self.model.tree())
            .clone();
        let pane_ids = tree.leaf_ids();
        let workspace_tabs = self.workspaces.as_ref().map(|set| {
            let active = set.active.clone();
            let count = set.workspaces.len();
            set.workspaces
                .iter()
                .enumerate()
                .map(|(index, workspace)| {
                    let id = workspace.id.clone();
                    let key_id = workspace.id.clone();
                    let a11y_id = workspace.id.clone();
                    let selected = id == active;
                    let accessibility = crate::MullionAccessibilityNode::workspace(
                        &workspace.id,
                        &workspace.name,
                        index,
                        count,
                        selected,
                    );
                    div()
                        .id(SharedString::from(format!("workspace:{}", id.0)))
                        .debug_selector({
                            let id = id.clone();
                            move || format!("workspace:{}", id.0)
                        })
                        .role(gpui::Role::Tab)
                        .accessibility_id(format!("mullion-workspace-{}", workspace.id.0))
                        .aria_label(accessibility.label)
                        .aria_description(accessibility.description)
                        .aria_selected(selected)
                        .focusable()
                        .tab_stop(true)
                        .px(styles.workspace_switcher.horizontal_padding)
                        .py(styles.workspace_switcher.vertical_padding)
                        .rounded(styles.workspace_switcher.border_radius)
                        .cursor_pointer()
                        .text_size(styles.workspace_switcher.font_size)
                        .text_color(if selected {
                            styles.workspace_switcher.active_text
                        } else {
                            styles.workspace_switcher.text
                        })
                        .bg(if selected {
                            styles.workspace_switcher.active_background
                        } else {
                            styles.workspace_switcher.background
                        })
                        .hover(|element| element.bg(styles.workspace_switcher.active_background))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.switch_workspace(&id, cx);
                        }))
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                                this.switch_workspace(&key_id, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_a11y_action(gpui::AccessibleAction::Click, {
                            let view = cx.entity().downgrade();
                            move |_, _, cx| {
                                view.update(cx, |this, cx| this.switch_workspace(&a11y_id, cx))
                                    .ok();
                            }
                        })
                        .child(workspace.name.clone())
                })
                .collect::<Vec<_>>()
        });
        let overlays = self.render_overlays(window, cx);
        let key_context = if self.keyboard_split.is_some() {
            "Mullion MullionSplitter"
        } else {
            crate::MULLION_KEY_CONTEXT
        };
        div()
            .key_context(key_context)
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(styles.root.background)
            .on_action(cx.listener(|this, _: &crate::FocusLeft, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusRight, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusUp, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusDown, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Down), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusNext, _, cx| {
                this.command(crate::PaneCommand::FocusNext, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusPrevious, _, cx| {
                this.command(crate::PaneCommand::FocusPrevious, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusFirst, _, cx| {
                this.command(crate::PaneCommand::FocusFirst, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::FocusLast, _, cx| {
                this.command(crate::PaneCommand::FocusLast, cx)
            }))
            .on_action(cx.listener(|this, action: &crate::FocusPane, _, cx| {
                this.command(crate::PaneCommand::FocusIndex(action.index), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SplitPaneHorizontal, _, cx| {
                this.command(crate::PaneCommand::Split(SplitDirection::Horizontal), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SplitPaneVertical, _, cx| {
                this.command(crate::PaneCommand::Split(SplitDirection::Vertical), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::ClosePane, _, cx| {
                this.command(crate::PaneCommand::Close, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::MovePaneLeft, _, cx| {
                this.command(crate::PaneCommand::Move(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::MovePaneRight, _, cx| {
                this.command(crate::PaneCommand::Move(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::MovePaneUp, _, cx| {
                this.command(crate::PaneCommand::Move(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::MovePaneDown, _, cx| {
                this.command(crate::PaneCommand::Move(PaneDirection::Down), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPaneLeft, _, cx| {
                this.command(crate::PaneCommand::Swap(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPaneRight, _, cx| {
                this.command(crate::PaneCommand::Swap(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPaneUp, _, cx| {
                this.command(crate::PaneCommand::Swap(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPaneDown, _, cx| {
                this.command(crate::PaneCommand::Swap(PaneDirection::Down), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPaneNext, _, cx| {
                this.command(crate::PaneCommand::SwapNext, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::SwapPanePrevious, _, cx| {
                this.command(crate::PaneCommand::SwapPrevious, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::ResizePaneLeft, _, cx| {
                this.command(crate::PaneCommand::Resize(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::ResizePaneRight, _, cx| {
                this.command(crate::PaneCommand::Resize(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::ResizePaneUp, _, cx| {
                this.command(crate::PaneCommand::Resize(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::ResizePaneDown, _, cx| {
                this.command(crate::PaneCommand::Resize(PaneDirection::Down), cx)
            }))
            .on_action(
                cx.listener(|this, _: &crate::SetParentSplitHorizontal, _, cx| {
                    this.command(
                        crate::PaneCommand::SetParentSplitDirection(SplitDirection::Horizontal),
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::SetParentSplitVertical, _, cx| {
                    this.command(
                        crate::PaneCommand::SetParentSplitDirection(SplitDirection::Vertical),
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::ToggleParentSplitDirection, _, cx| {
                    this.command(crate::PaneCommand::ToggleParentSplitDirection, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &crate::BalancePanes, _, cx| {
                this.command(crate::PaneCommand::Balance, cx)
            }))
            .on_action(cx.listener(|this, _: &crate::RotatePanesForward, _, cx| {
                this.command(crate::PaneCommand::Rotate(crate::PaneRotation::Forward), cx)
            }))
            .on_action(cx.listener(|this, _: &crate::RotatePanesBackward, _, cx| {
                this.command(
                    crate::PaneCommand::Rotate(crate::PaneRotation::Backward),
                    cx,
                )
            }))
            .on_action(
                cx.listener(|this, _: &crate::ApplyEvenHorizontalLayout, _, cx| {
                    this.command(
                        crate::PaneCommand::ApplyLayout(crate::PaneLayout::EvenHorizontal),
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::ApplyEvenVerticalLayout, _, cx| {
                    this.command(
                        crate::PaneCommand::ApplyLayout(crate::PaneLayout::EvenVertical),
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::ApplyMainHorizontalLayout, _, cx| {
                    this.command(
                        crate::PaneCommand::ApplyLayout(crate::PaneLayout::MainHorizontal),
                        cx,
                    )
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::ApplyMainVerticalLayout, _, cx| {
                    this.command(
                        crate::PaneCommand::ApplyLayout(crate::PaneLayout::MainVertical),
                        cx,
                    )
                }),
            )
            .on_action(cx.listener(|this, _: &crate::ApplyTiledLayout, _, cx| {
                this.command(
                    crate::PaneCommand::ApplyLayout(crate::PaneLayout::Tiled),
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &crate::ToggleZoom, _, cx| {
                this.command(crate::PaneCommand::ToggleZoom, cx)
            }))
            .on_action(cx.listener(|this, _: &ResizeSplitDecrease, _, cx| {
                this.resize_keyboard_split(-KEYBOARD_RESIZE_STEP, cx)
            }))
            .on_action(cx.listener(|this, _: &ResizeSplitIncrease, _, cx| {
                this.resize_keyboard_split(KEYBOARD_RESIZE_STEP, cx)
            }))
            .on_action(cx.listener(|this, _: &CancelSplitResize, window, cx| {
                this.cancel_split_resize(window, cx)
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.active_split.borrow_mut().take();
                    this.split_starts.borrow_mut().clear();
                }),
            )
            .when_some(workspace_tabs, |element, tabs| {
                element.child(
                    div()
                        .id("mullion-workspace-tabs")
                        .role(gpui::Role::TabList)
                        .aria_label("Mullion workspaces")
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(styles.workspace_switcher.gap)
                        .border_b(styles.pane.border_width)
                        .border_color(styles.pane.border)
                        .bg(styles.workspace_switcher.background)
                        .children(tabs),
                )
            })
            .child(div().flex_1().min_w_0().min_h_0().child(self.render_node(
                &tree,
                &pane_ids,
                InternalEdges::default(),
                window,
                cx,
            )))
            .child(
                div()
                    .debug_selector(|| "mullion-overlay-layer".to_owned())
                    .absolute()
                    .inset_0()
                    .children(overlays),
            )
    }
}

/// Compile and register a custom Mullion keymap in the standard [`crate::MULLION_KEY_CONTEXT`].
///
/// By default, compiled bindings require `Mullion && !MullionEditable`. A host
/// should add `MullionEditable` with `key_context` around text inputs or other
/// editable descendants. This cooperative marker lets editing shortcuts win.
/// Maps created with [`crate::MullionKeymap::capture_editable_targets`] opt into
/// capture and are bound to `Mullion` without that suppression predicate.
pub fn try_register_key_bindings(
    cx: &mut App,
    keymap: &crate::MullionKeymap,
) -> Result<(), crate::KeymapCompileError> {
    let bindings = crate::compile_keymap(keymap, crate::MULLION_KEY_CONTEXT)?;
    cx.bind_keys(bindings);
    Ok(())
}

/// Alias for [`try_register_key_bindings`] for hosts that name the custom map explicitly.
pub fn register_keymap(
    cx: &mut App,
    keymap: &crate::MullionKeymap,
) -> Result<(), crate::KeymapCompileError> {
    try_register_key_bindings(cx, keymap)
}

/// Register the complete default [`crate::MullionKeymap`].
///
/// This compatibility entry point is infallible because the built-in map and
/// context are crate-owned constants. Use [`try_register_key_bindings`] for
/// user-provided maps so invalid keystrokes can be reported.
pub fn register_key_bindings(cx: &mut App) {
    try_register_key_bindings(cx, &crate::MullionKeymap::default())
        .expect("the built-in Mullion keymap must compile");

    // Splitter-local manipulation is intentionally not part of PaneCommand or
    // MullionKeymap: these actions operate on the last directly manipulated bar.
    cx.bind_keys([
        gpui::KeyBinding::new(
            "ctrl-alt-[",
            ResizeSplitDecrease,
            Some(MULLION_SPLITTER_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "ctrl-alt-]",
            ResizeSplitIncrease,
            Some(MULLION_SPLITTER_KEY_CONTEXT),
        ),
        gpui::KeyBinding::new(
            "escape",
            CancelSplitResize,
            Some(MULLION_SPLITTER_KEY_CONTEXT),
        ),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{div, TestAppContext};
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            atomic::{AtomicU8, AtomicUsize, Ordering},
            Arc,
        },
    };

    fn test_overlay(id: &str, tier: crate::OverlayTier) -> MullionOverlay {
        let selector = id.to_owned();
        MullionOverlay::new(id, move |_, _| {
            div()
                .debug_selector({
                    let selector = selector.clone();
                    move || format!("overlay-renderer:{selector}")
                })
                .child(selector.clone())
                .into_any_element()
        })
        .with_tier(tier)
    }

    #[gpui::test]
    fn rendered_overlay_layer_escapes_panes_and_preserves_tier_order(cx: &mut TestAppContext) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let stack = crate::OverlayStack::from_overlays([
            {
                let calls = calls.clone();
                MullionOverlay::new("drag", move |_, _| {
                    calls.borrow_mut().push("drag");
                    div().into_any_element()
                })
                .with_tier(crate::OverlayTier::Drag)
            },
            {
                let calls = calls.clone();
                MullionOverlay::new("modal", move |_, _| {
                    calls.borrow_mut().push("modal");
                    div().into_any_element()
                })
            },
            {
                let calls = calls.clone();
                MullionOverlay::new("toast", move |_, _| {
                    calls.borrow_mut().push("toast");
                    div().into_any_element()
                })
                .with_tier(crate::OverlayTier::Toast)
            },
        ])
        .unwrap();
        let (_, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx)
                .with_overlay_host(OverlayHostConfig::controlled(move || stack.clone()))
        });
        cx.run_until_parked();

        assert!(calls
            .borrow()
            .chunks_exact(3)
            .all(|chunk| chunk == ["modal", "toast", "drag"]));
        assert_eq!(
            cx.debug_bounds("mullion-overlay-layer"),
            cx.debug_bounds("pane:pane")
                .map(|_| cx.debug_bounds("mullion-overlay-layer").unwrap())
        );
        for selector in [
            "mullion-overlay:modal",
            "mullion-overlay:toast",
            "mullion-overlay:drag",
        ] {
            assert!(cx.debug_bounds(selector).is_some());
        }
    }

    #[gpui::test]
    fn rendered_overlay_supports_every_alignment_and_length(cx: &mut TestAppContext) {
        let overlays = [
            MullionOverlay::new("start-pixels", |_, _| div().into_any_element())
                .with_tier(crate::OverlayTier::Toast)
                .with_placement(crate::OverlayPlacement::new(
                    OverlayAlignment::Start,
                    OverlayAlignment::Start,
                ))
                .with_size(crate::OverlaySize::new(
                    OverlayLength::Pixels(40.0),
                    OverlayLength::Pixels(30.0),
                )),
            MullionOverlay::new("center-fraction", |_, _| div().into_any_element())
                .with_tier(crate::OverlayTier::Toast)
                .with_size(crate::OverlaySize::new(
                    OverlayLength::Fraction(0.5),
                    OverlayLength::Fraction(0.5),
                )),
            MullionOverlay::new("end-content", |_, _| {
                div().w(px(25.0)).h(px(20.0)).into_any_element()
            })
            .with_tier(crate::OverlayTier::Toast)
            .with_placement(crate::OverlayPlacement::new(
                OverlayAlignment::End,
                OverlayAlignment::End,
            )),
            MullionOverlay::new("stretch-content", |_, _| div().into_any_element())
                .with_tier(crate::OverlayTier::Toast)
                .with_placement(crate::OverlayPlacement::FILL),
            MullionOverlay::new("start-fill", |_, _| div().into_any_element())
                .with_tier(crate::OverlayTier::Toast)
                .with_placement(crate::OverlayPlacement::new(
                    OverlayAlignment::Start,
                    OverlayAlignment::Start,
                ))
                .with_size(crate::OverlaySize::FILL),
        ];
        let stack = crate::OverlayStack::from_overlays(overlays).unwrap();
        let (_, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx)
                .with_overlay_host(OverlayHostConfig::controlled(move || stack.clone()))
        });
        cx.run_until_parked();

        let layer = cx.debug_bounds("mullion-overlay-layer").unwrap();
        let start = cx
            .debug_bounds("mullion-overlay-content:start-pixels")
            .unwrap();
        assert_eq!(start.origin, layer.origin);
        assert_eq!(start.size, gpui::size(px(40.0), px(30.0)));

        let center = cx
            .debug_bounds("mullion-overlay-content:center-fraction")
            .unwrap();
        assert_eq!(center.center(), layer.center());
        assert_eq!(center.size.width, layer.size.width * 0.5);
        assert_eq!(center.size.height, layer.size.height * 0.5);

        let end = cx
            .debug_bounds("mullion-overlay-content:end-content")
            .unwrap();
        assert_eq!(end.right(), layer.right());
        assert_eq!(end.bottom(), layer.bottom());
        assert_eq!(end.size, gpui::size(px(25.0), px(20.0)));

        for selector in [
            "mullion-overlay-content:stretch-content",
            "mullion-overlay-content:start-fill",
        ] {
            assert_eq!(cx.debug_bounds(selector).unwrap(), layer);
        }
    }

    #[gpui::test]
    fn rendered_overlay_geometry_backdrop_and_true_outside_dismiss(cx: &mut TestAppContext) {
        let dismissed = Rc::new(Cell::new(0));
        let count = dismissed.clone();
        let overlay = test_overlay("dialog", crate::OverlayTier::Modal)
            .with_placement(crate::OverlayPlacement::CENTER)
            .with_size(crate::OverlaySize::new(
                OverlayLength::Pixels(120.0),
                OverlayLength::Pixels(80.0),
            ))
            .with_backdrop(crate::OverlayBackdrop::default())
            .dismiss_on_backdrop(true)
            .a11y_modal(true)
            .with_a11y_label("Dialog");
        let stack = crate::OverlayStack::from_overlays([overlay]).unwrap();
        let (_, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_overlay_host(
                OverlayHostConfig::controlled(move || stack.clone())
                    .with_dismiss_handler(move |_, _, _| count.set(count.get() + 1)),
            )
        });
        cx.run_until_parked();

        let content = cx.debug_bounds("mullion-overlay-content:dialog").unwrap();
        assert_eq!(content.size.width, px(120.0));
        assert_eq!(content.size.height, px(80.0));
        cx.simulate_click(content.center(), gpui::Modifiers::none());
        assert_eq!(dismissed.get(), 0);
        let layer = cx.debug_bounds("mullion-overlay-layer").unwrap();
        cx.simulate_click(layer.origin, gpui::Modifiers::none());
        assert_eq!(dismissed.get(), 1);
    }

    #[gpui::test]
    fn click_through_overlay_preserves_workspace_input(cx: &mut TestAppContext) {
        let overlay = MullionOverlay::new("pass-through", |_, _| div().into_any_element())
            .with_tier(crate::OverlayTier::Drag)
            .with_placement(crate::OverlayPlacement::FILL)
            .with_size(crate::OverlaySize::FILL)
            .click_through(true);
        let stack = crate::OverlayStack::from_overlays([overlay]).unwrap();
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspace_set(leaf("c")), vec![], cx)
                .unwrap()
                .with_overlay_host(OverlayHostConfig::controlled(move || stack.clone()))
        });
        cx.run_until_parked();

        let tab = cx.debug_bounds("workspace:two").unwrap().center();
        cx.simulate_click(tab, gpui::Modifiers::none());
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.active_workspace().unwrap().id,
                WorkspaceId("two".into())
            );
        });
    }

    #[gpui::test]
    fn controlled_overlay_updates_and_invalid_snapshot_fails_safe(cx: &mut TestAppContext) {
        let snapshot = Rc::new(RefCell::new(
            crate::OverlayStack::from_overlays([test_overlay("first", crate::OverlayTier::Toast)])
                .unwrap(),
        ));
        let source = snapshot.clone();
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_overlay_host(
                OverlayHostConfig::controlled(move || source.borrow().clone()),
            )
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("mullion-overlay:first").is_some());

        *snapshot.borrow_mut() =
            crate::OverlayStack::from_unchecked([MullionOverlay::from_policy(
                crate::OverlayPolicy::new("bad").with_size(crate::OverlaySize::new(
                    OverlayLength::Fraction(2.0),
                    OverlayLength::Content,
                )),
                |_, _| div().into_any_element(),
            )]);
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        assert!(cx.debug_bounds("mullion-overlay:first").is_none());
        assert!(cx.debug_bounds("mullion-overlay:bad").is_none());
        assert!(matches!(
            view.read_with(cx, |view, _| view.last_overlay_error().cloned()),
            Some(OverlayError::InvalidDimension { .. })
        ));
    }

    #[gpui::test]
    fn every_gpui_command_action_routes_through_the_configured_dispatcher(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            MullionView::new(
                PaneNode::leaf(PaneId::new("only"), "data".to_owned()),
                vec![],
                cx,
            )
        });
        cx.run_until_parked();

        cx.dispatch_action(crate::FocusLeft);
        cx.dispatch_action(crate::FocusRight);
        cx.dispatch_action(crate::FocusUp);
        cx.dispatch_action(crate::FocusDown);
        cx.dispatch_action(crate::FocusNext);
        cx.dispatch_action(crate::FocusPrevious);
        cx.dispatch_action(crate::FocusFirst);
        cx.dispatch_action(crate::FocusLast);
        cx.dispatch_action(crate::SplitPaneHorizontal);
        cx.dispatch_action(crate::SplitPaneVertical);
        cx.dispatch_action(crate::ClosePane);
        cx.dispatch_action(crate::MovePaneLeft);
        cx.dispatch_action(crate::MovePaneRight);
        cx.dispatch_action(crate::MovePaneUp);
        cx.dispatch_action(crate::MovePaneDown);
        cx.dispatch_action(crate::SwapPaneLeft);
        cx.dispatch_action(crate::SwapPaneRight);
        cx.dispatch_action(crate::SwapPaneUp);
        cx.dispatch_action(crate::SwapPaneDown);
        cx.dispatch_action(crate::SwapPaneNext);
        cx.dispatch_action(crate::SwapPanePrevious);
        cx.dispatch_action(crate::ResizePaneLeft);
        cx.dispatch_action(crate::ResizePaneRight);
        cx.dispatch_action(crate::ResizePaneUp);
        cx.dispatch_action(crate::ResizePaneDown);
        cx.dispatch_action(crate::SetParentSplitHorizontal);
        cx.dispatch_action(crate::SetParentSplitVertical);
        cx.dispatch_action(crate::ToggleParentSplitDirection);
        cx.dispatch_action(crate::BalancePanes);
        cx.dispatch_action(crate::RotatePanesForward);
        cx.dispatch_action(crate::RotatePanesBackward);
        cx.dispatch_action(crate::ApplyEvenHorizontalLayout);
        cx.dispatch_action(crate::ApplyEvenVerticalLayout);
        cx.dispatch_action(crate::ApplyMainHorizontalLayout);
        cx.dispatch_action(crate::ApplyMainVerticalLayout);
        cx.dispatch_action(crate::ApplyTiledLayout);
        cx.dispatch_action(crate::ToggleZoom);
        cx.dispatch_action(crate::FocusPane { index: 23 });

        view.read_with(cx, |view, _| {
            let mut expected = crate::PaneCommand::catalog();
            expected.push(crate::PaneCommand::FocusIndex(23));
            assert_eq!(view.routed_commands, expected);
        });
    }

    #[gpui::test]
    fn complete_default_keymap_dispatches_through_the_rendered_view(cx: &mut TestAppContext) {
        cx.update(register_key_bindings);
        let keymap = crate::MullionKeymap::default();
        let expected = keymap.normalized_sequences();
        let (view, cx) = cx.add_window_view(|_, cx| {
            MullionView::new(
                PaneNode::leaf(PaneId::new("only"), "data".to_owned()),
                vec![],
                cx,
            )
        });
        cx.run_until_parked();
        let pane = cx.debug_bounds("pane:only").unwrap().center();
        cx.simulate_mouse_move(pane, None, gpui::Modifiers::none());
        for (sequence, _) in &expected {
            cx.simulate_keystrokes(sequence);
        }
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.routed_commands,
                expected
                    .into_iter()
                    .map(|(_, command)| command)
                    .collect::<Vec<_>>()
            );
        });
    }

    #[gpui::test]
    fn configured_split_factory_and_resize_step_drive_actions(cx: &mut TestAppContext) {
        let calls = Arc::new(AtomicU8::new(0));
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx
            .add_window_view(move |_, cx| MullionView::new(tree, vec![], cx).with_resize_step(0.2));
        cx.run_until_parked();

        // No factory means unavailable and is inert.
        cx.dispatch_action(crate::SplitPaneVertical);
        assert_eq!(
            view.read_with(cx, |view, _| view.model().tree().leaf_ids().len()),
            2
        );

        let refusal_calls = calls.clone();
        view.update(cx, |view, _| {
            view.set_split_factory(Some(Arc::new(move |_, _, _| {
                refusal_calls.fetch_add(1, Ordering::SeqCst);
                None
            })));
        });
        cx.dispatch_action(crate::SplitPaneVertical);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            view.read_with(cx, |view, _| view.model().tree().leaf_ids().len()),
            2
        );

        view.update(cx, |view, _| {
            view.set_split_factory(Some(Arc::new(|_, _, data: &String| {
                Some((PaneId::new("created"), format!("{data}-split")))
            })));
        });
        cx.dispatch_action(crate::SplitPaneVertical);
        assert_eq!(
            view.read_with(cx, |view, _| view.model().tree().leaf_ids().len()),
            3
        );

        // Focus the original first pane, then grow it toward the right boundary.
        cx.dispatch_action(crate::FocusPane { index: 0 });
        cx.dispatch_action(crate::ResizePaneRight);
        assert_eq!(
            view.read_with(cx, |view, _| crate::tree::find_ratio(
                view.model().tree(),
                &PaneId::new("b")
            )),
            Some(0.7)
        );
        assert_eq!(
            view.read_with(cx, |view, _| view.model().focused().cloned()),
            Some(PaneId::new("a"))
        );
    }

    struct StatefulBody;

    impl Render for StatefulBody {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child("stateful")
        }
    }

    fn legacy_activity() -> Activity<String> {
        Activity {
            id: ActivityId::new("legacy"),
            name: "Legacy".into(),
            filter: |_| true,
            render: Arc::new(|_, _| div().child("legacy fallback").into_any_element()),
        }
    }

    fn visible_data(data: &String) -> bool {
        data != "hidden"
    }

    #[gpui::test]
    fn rendered_stateful_activity_is_lazy_stable_updated_and_filtered(cx: &mut TestAppContext) {
        let factory_calls = Rc::new(Cell::new(0));
        let updates = Rc::new(Cell::new(0));
        let disposals = Rc::new(Cell::new(0));
        let factory_count = factory_calls.clone();
        let update_count = updates.clone();
        let dispose_count = disposals.clone();
        let registry = ActivityFactoryRegistry::new().with_factory(
            ActivityId::new("stateful"),
            move |_, _, _, cx| {
                factory_count.set(factory_count.get() + 1);
                let body = cx.new(|_| StatefulBody);
                crate::ActivityInstance::new(body)
                    .with_header(cx.new(|_| StatefulBody))
                    .with_update({
                        let updates = update_count.clone();
                        move |_, _, _| updates.set(updates.get() + 1)
                    })
                    .with_dispose({
                        let disposals = dispose_count.clone();
                        move |_| disposals.set(disposals.get() + 1)
                    })
            },
        );
        let activity = Activity {
            id: ActivityId::new("stateful"),
            name: "Stateful".into(),
            filter: visible_data,
            render: Arc::new(|_, _| div().child("legacy").into_any_element()),
        };
        let tree = PaneNode::leaf_with_activity(
            PaneId::new("pane"),
            ActivityId::new("stateful"),
            "one".to_string(),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(tree, vec![ActivityNode::Activity(activity)], cx)
                .with_activity_factories(registry)
        });

        assert_eq!(factory_calls.get(), 1);
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(factory_calls.get(), 1);
        assert_eq!(updates.get(), 0);

        view.update(cx, |view, cx| {
            assert!(view.update_model(cx, |model| {
                model.update_data(&PaneId::new("pane"), "two".to_string())
            }));
        });
        cx.run_until_parked();
        assert_eq!(factory_calls.get(), 1);
        assert_eq!(updates.get(), 1);
        assert_eq!(disposals.get(), 0);

        view.update(cx, |view, cx| {
            assert!(view.update_model(cx, |model| {
                model.update_data(&PaneId::new("pane"), "hidden".to_string())
            }));
        });
        cx.run_until_parked();
        assert_eq!(updates.get(), 1);
        assert_eq!(disposals.get(), 1);
    }

    #[gpui::test]
    fn root_release_disposes_cached_instance_exactly_once(cx: &mut TestAppContext) {
        let disposals = Rc::new(Cell::new(0));
        let view = cx.new(|cx| {
            MullionView::new(
                PaneNode::leaf(PaneId::new("pane"), "data".to_string()),
                vec![ActivityNode::Activity(legacy_activity())],
                cx,
            )
        });
        let count = disposals.clone();
        view.update(cx, |view, cx| {
            let body = cx.new(|_| StatefulBody);
            view.activity_cache.insert(
                ActivityCacheKey::new(None, PaneId::new("pane"), ActivityId::new("legacy")),
                crate::ActivityInstance::new(body).with_dispose(move |_| {
                    count.set(count.get() + 1);
                }),
                "data".to_string(),
            );
        });
        drop(view);
        cx.update(|_| {});
        cx.run_until_parked();
        assert_eq!(disposals.get(), 1);
    }

    #[gpui::test]
    fn explicit_clear_then_root_release_does_not_double_dispose(cx: &mut TestAppContext) {
        let disposals = Rc::new(Cell::new(0));
        let view = cx.new(|cx| {
            MullionView::new(
                PaneNode::leaf(PaneId::new("pane"), "data".to_string()),
                vec![ActivityNode::Activity(legacy_activity())],
                cx,
            )
        });
        let count = disposals.clone();
        view.update(cx, |view, cx| {
            let body = cx.new(|_| StatefulBody);
            view.activity_cache.insert(
                ActivityCacheKey::new(None, PaneId::new("pane"), ActivityId::new("legacy")),
                crate::ActivityInstance::new(body).with_dispose(move |_| {
                    count.set(count.get() + 1);
                }),
                "data".to_string(),
            );
            view.clear_activity_cache(cx);
        });
        assert_eq!(disposals.get(), 1);
        drop(view);
        cx.update(|_| {});
        cx.run_until_parked();
        assert_eq!(disposals.get(), 1);
    }

    fn split(
        direction: SplitDirection,
        ratio: f64,
        first: PaneNode<String>,
        second: PaneNode<String>,
    ) -> PaneNode<String> {
        PaneNode::Split {
            direction,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    fn leaf(id: &str) -> PaneNode<String> {
        PaneNode::leaf(PaneId::new(id), id.to_string())
    }

    #[gpui::test]
    fn typed_nested_and_trailing_activity_drags_create_panes_in_all_five_zones(
        cx: &mut TestAppContext,
    ) {
        let nested = rendered_activity("nested-drag", show_activity);
        let trailing = rendered_activity("trailing-drag", show_activity);
        let catalog = ActivityCatalog::new(vec![ActivityNode::Category(crate::ActivityCategory {
            id: crate::CategoryId::new("drag-category"),
            name: "Drag category".into(),
            color: gpui::rgb(0x112233).into(),
            children: vec![ActivityNode::Activity(nested)],
        })])
        .with_trailing(vec![ActivityNode::Activity(trailing)]);
        let base = split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf_with_activity(
                PaneId::new("a"),
                ActivityId::new("nested-drag"),
                "a".to_owned(),
            ),
            leaf("b"),
        );
        let factory_calls = Arc::new(AtomicU8::new(0));
        let calls = factory_calls.clone();
        let (view, cx) = cx.add_window_view({
            let base = base.clone();
            move |_, cx| {
                MullionView::try_new_with_catalog(base, catalog, cx)
                    .unwrap()
                    .with_new_pane_factory(move |_, _, _| {
                        let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        Some((PaneId::new(format!("created-{n}")), format!("created-{n}")))
                    })
            }
        });
        cx.simulate_resize(gpui::size(px(1000.), px(700.)));
        cx.run_until_parked();
        let events = Rc::new(RefCell::new(Vec::<PaneEvent<String>>::new()));
        let event_log = events.clone();
        let observed = view.clone();
        view.update(cx, move |_, cx| {
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                event_log.borrow_mut().push(event.clone());
            })
            .detach();
        });

        for (index, (edge, x, y)) in [
            (DropEdge::Left, 0.1, 0.5),
            (DropEdge::Right, 0.9, 0.5),
            (DropEdge::Top, 0.5, 0.1),
            (DropEdge::Bottom, 0.5, 0.9),
            (DropEdge::Center, 0.5, 0.5),
        ]
        .into_iter()
        .enumerate()
        {
            let reset = base.clone();
            view.update(cx, |view, cx| {
                view.update_model(cx, |model| model.replace_tree(reset))
            });
            cx.run_until_parked();
            events.borrow_mut().clear();
            cx.update(|_, _| {});
            cx.run_until_parked();
            events.borrow_mut().clear();
            let activity = if index % 2 == 0 {
                "nested-drag"
            } else {
                "trailing-drag"
            };
            let activity_selector = if index % 2 == 0 {
                "activity:a:nested-drag"
            } else {
                "activity:a:trailing-drag"
            };
            let start = cx.debug_bounds(activity_selector).unwrap().center();
            let destination_pane = if edge == DropEdge::Center { "a" } else { "b" };
            let target = if destination_pane == "a" {
                cx.debug_bounds("pane:a").unwrap()
            } else {
                cx.debug_bounds("pane:b").unwrap()
            };
            let destination = gpui::point(
                target.left() + target.size.width * x,
                target.top() + target.size.height * y,
            );
            cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
            cx.simulate_mouse_move(
                gpui::point(start.x + px(4.), start.y),
                Some(MouseButton::Left),
                gpui::Modifiers::none(),
            );
            cx.simulate_mouse_move(
                destination,
                Some(MouseButton::Left),
                gpui::Modifiers::none(),
            );
            cx.run_until_parked();
            let indicator_selector = match edge {
                DropEdge::Left => "dock-indicator:b:Left",
                DropEdge::Right => "dock-indicator:b:Right",
                DropEdge::Top => "dock-indicator:b:Top",
                DropEdge::Bottom => "dock-indicator:b:Bottom",
                DropEdge::Center => "dock-indicator:a:Center",
            };
            assert!(cx.debug_bounds(indicator_selector).is_some());
            cx.simulate_mouse_up(destination, MouseButton::Left, gpui::Modifiers::none());
            cx.run_until_parked();

            let new_id = PaneId::new(format!("created-{}", index + 1));
            let mut expected = base.clone();
            assert!(expected.insert_leaf(
                &PaneId::new(destination_pane),
                edge,
                new_id.clone(),
                format!("created-{}", index + 1),
                Some(ActivityId::new(activity)),
            ));
            view.read_with(cx, |view, _| {
                assert_eq!(view.model().tree(), &expected);
                assert_eq!(view.model().focused(), Some(&new_id));
            });
            let mut expected_events = Vec::new();
            if index > 0 {
                expected_events.push(PaneEvent::FocusChanged {
                    pane: Some(PaneId::new("a")),
                });
            }
            expected_events.extend([
                PaneEvent::ActivityDropped {
                    activity: ActivityId::new(activity),
                    destination: PaneId::new(destination_pane),
                    edge,
                    new_id: new_id.clone(),
                    new_data: format!("created-{}", index + 1),
                },
                PaneEvent::TreeChanged { tree: expected },
                PaneEvent::FocusChanged { pane: Some(new_id) },
            ]);
            assert_eq!(&*events.borrow(), &expected_events);
        }
        assert_eq!(factory_calls.load(Ordering::SeqCst), 5);
    }

    fn ratio(
        view: &gpui::Entity<MullionView<String>>,
        key: &str,
        cx: &mut gpui::VisualTestContext,
    ) -> f64 {
        view.read_with(cx, |view, _| {
            crate::tree::find_ratio(view.model().tree(), &PaneId::new(key)).unwrap()
        })
    }

    #[gpui::test]
    fn typed_pane_drag_drives_all_five_zones_with_exact_events_and_indicators(
        cx: &mut TestAppContext,
    ) {
        let base = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view({
            let base = base.clone();
            move |_, cx| {
                MullionView::new(base, vec![], cx).with_focus_behavior(PaneFocusBehavior::Click)
            }
        });
        cx.simulate_resize(gpui::size(px(1000.), px(700.)));
        cx.run_until_parked();
        let events = Rc::new(RefCell::new(Vec::<PaneEvent<String>>::new()));
        let event_log = events.clone();
        let observed = view.clone();
        view.update(cx, move |_, cx| {
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                event_log.borrow_mut().push(event.clone());
            })
            .detach();
        });

        let cases = [
            (DropEdge::Left, 0.1, 0.1),
            (DropEdge::Right, 0.9, 0.1),
            (DropEdge::Top, 0.5, 0.1),
            (DropEdge::Bottom, 0.5, 0.9),
            (DropEdge::Center, 0.5, 0.5),
        ];
        for (edge, x, y) in cases {
            let selector = match edge {
                DropEdge::Left => "dock-indicator:b:Left",
                DropEdge::Right => "dock-indicator:b:Right",
                DropEdge::Top => "dock-indicator:b:Top",
                DropEdge::Bottom => "dock-indicator:b:Bottom",
                DropEdge::Center => "dock-indicator:b:Center",
            };
            let reset = base.clone();
            view.update(cx, |view, cx| {
                view.update_model(cx, |model| model.replace_tree(reset));
            });
            cx.run_until_parked();
            events.borrow_mut().clear();

            let start = cx.debug_bounds("pane-drag-handle:a").unwrap().center();
            let target = cx.debug_bounds("pane:b").unwrap();
            let destination = gpui::point(
                target.left() + target.size.width * x,
                target.top() + target.size.height * y,
            );
            cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
            cx.simulate_mouse_move(
                gpui::point(start.x + px(4.), start.y),
                Some(MouseButton::Left),
                gpui::Modifiers::none(),
            );
            cx.simulate_mouse_move(
                destination,
                Some(MouseButton::Left),
                gpui::Modifiers::none(),
            );
            cx.run_until_parked();
            let indicator = cx.debug_bounds(selector).unwrap();
            let expected_indicator = edge
                .indicator_in(DockBounds::new(
                    target.left().into(),
                    target.top().into(),
                    target.size.width.into(),
                    target.size.height.into(),
                ))
                .unwrap();
            assert!((f64::from(indicator.left()) - expected_indicator.left).abs() <= 1.0);
            assert!((f64::from(indicator.top()) - expected_indicator.top).abs() <= 1.0);
            assert!((f64::from(indicator.size.width) - expected_indicator.width).abs() <= 2.0);
            assert!((f64::from(indicator.size.height) - expected_indicator.height).abs() <= 2.0);

            cx.simulate_mouse_up(destination, MouseButton::Left, gpui::Modifiers::none());
            cx.run_until_parked();
            assert!(cx.debug_bounds(selector).is_none());
            let expected_tree = match edge {
                DropEdge::Left => split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b")),
                DropEdge::Right | DropEdge::Center => {
                    split(SplitDirection::Horizontal, 0.5, leaf("b"), leaf("a"))
                }
                DropEdge::Top => split(SplitDirection::Vertical, 0.5, leaf("a"), leaf("b")),
                DropEdge::Bottom => split(SplitDirection::Vertical, 0.5, leaf("b"), leaf("a")),
            };
            view.read_with(cx, |view, _| {
                assert_eq!(view.model().tree(), &expected_tree)
            });
            let expected_events = if edge == DropEdge::Center {
                vec![PaneEvent::TreeChanged {
                    tree: expected_tree,
                }]
            } else {
                vec![
                    PaneEvent::Moved {
                        source: PaneId::new("a"),
                        destination: PaneId::new("b"),
                        edge,
                    },
                    PaneEvent::TreeChanged {
                        tree: expected_tree,
                    },
                ]
            };
            assert_eq!(&*events.borrow(), &expected_events);
        }
    }

    #[gpui::test]
    fn dock_drag_self_right_click_nested_cancel_and_release_are_no_ops_until_valid_drop(
        cx: &mut TestAppContext,
    ) {
        let nested = split(SplitDirection::Vertical, 0.5, leaf("b"), leaf("c"));
        let base = split(SplitDirection::Horizontal, 0.4, leaf("a"), nested);
        let (view, cx) = cx.add_window_view({
            let base = base.clone();
            move |_, cx| {
                MullionView::new(base, vec![], cx).with_focus_behavior(PaneFocusBehavior::Click)
            }
        });
        cx.simulate_resize(gpui::size(px(1000.), px(700.)));
        cx.run_until_parked();
        let start = cx.debug_bounds("pane-drag-handle:a").unwrap().center();
        let a = cx.debug_bounds("pane:a").unwrap().center();
        let c_bounds = cx.debug_bounds("pane:c").unwrap();
        let c = gpui::point(
            c_bounds.left() + c_bounds.size.width * 0.5,
            c_bounds.top() + c_bounds.size.height * 0.9,
        );

        cx.simulate_mouse_down(start, MouseButton::Right, gpui::Modifiers::none());
        cx.simulate_mouse_move(c, Some(MouseButton::Right), gpui::Modifiers::none());
        cx.simulate_mouse_up(c, MouseButton::Right, gpui::Modifiers::none());
        assert!(cx.debug_bounds("dock-indicator:c:Bottom").is_none());

        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(a, Some(MouseButton::Left), gpui::Modifiers::none());
        assert!(cx.debug_bounds("dock-indicator:a:Center").is_none());
        cx.simulate_mouse_up(a, MouseButton::Left, gpui::Modifiers::none());
        view.read_with(cx, |view, _| assert_eq!(view.model().tree(), &base));

        let start = cx.debug_bounds("pane-drag-handle:a").unwrap().center();
        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(c, Some(MouseButton::Left), gpui::Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds("dock-indicator:c:Bottom").is_some());
        cx.dispatch_action(CancelSplitResize);
        cx.run_until_parked();
        assert!(cx.debug_bounds("dock-indicator:c:Bottom").is_none());
        cx.simulate_mouse_up(c, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(c, None, gpui::Modifiers::none());
        view.read_with(cx, |view, _| assert_eq!(view.model().tree(), &base));

        let start = cx.debug_bounds("pane-drag-handle:a").unwrap().center();
        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(c, Some(MouseButton::Left), gpui::Modifiers::none());
        cx.simulate_mouse_up(c, MouseButton::Left, gpui::Modifiers::none());
        cx.run_until_parked();
        let mut expected = base.clone();
        assert!(expected.move_pane(&PaneId::new("a"), &PaneId::new("c"), DropEdge::Bottom));
        view.read_with(cx, |view, _| assert_eq!(view.model().tree(), &expected));
        assert!(cx.debug_bounds("dock-indicator:c:Bottom").is_none());
    }

    static PERF_FILTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn counted_visible(_: &String) -> bool {
        PERF_FILTER_CALLS.fetch_add(1, Ordering::SeqCst);
        true
    }

    fn pane_fixture(start: usize, count: usize, depth: usize) -> PaneNode<String> {
        if count == 1 {
            return PaneNode::leaf_with_activity(
                PaneId::new(format!("p{start}")),
                ActivityId::new("perf"),
                format!("data-{start}"),
            );
        }
        let first_count = count / 2;
        split(
            if depth.is_multiple_of(2) {
                SplitDirection::Horizontal
            } else {
                SplitDirection::Vertical
            },
            0.5,
            pane_fixture(start, first_count, depth + 1),
            pane_fixture(start + first_count, count - first_count, depth + 1),
        )
    }

    #[gpui::test]
    fn twenty_nine_pane_drag_has_constant_lifecycle_and_exact_event_budget(
        cx: &mut TestAppContext,
    ) {
        PERF_FILTER_CALLS.store(0, Ordering::SeqCst);
        let body_renders = Arc::new(AtomicUsize::new(0));
        let renders = body_renders.clone();
        let activity = Activity {
            id: ActivityId::new("perf"),
            name: "Performance".into(),
            filter: counted_visible,
            render: Arc::new(move |_, _| {
                renders.fetch_add(1, Ordering::SeqCst);
                div().child("stable").into_any_element()
            }),
        };
        let tree = pane_fixture(0, 29, 0);
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(tree, vec![ActivityNode::Activity(activity)], cx)
        });
        cx.simulate_resize(gpui::size(px(1800.), px(1200.)));
        cx.run_until_parked();

        let initial_filters = PERF_FILTER_CALLS.load(Ordering::SeqCst);
        let initial_bodies = body_renders.load(Ordering::SeqCst);
        assert!((29..=29 * 3).contains(&initial_bodies));
        assert_eq!(view.read_with(cx, |view, _| view.activity_cache_syncs), 1);
        for _ in 0..4 {
            view.update(cx, |_, cx| cx.notify());
            cx.run_until_parked();
        }
        assert_eq!(PERF_FILTER_CALLS.load(Ordering::SeqCst), initial_filters);
        assert_eq!(body_renders.load(Ordering::SeqCst), initial_bodies);
        assert_eq!(view.read_with(cx, |view, _| view.activity_cache_syncs), 1);

        // p28 keys the deepest right-hand split, so its drag event bubbles through
        // several split containers and exercises the strict split-key guard.
        let handle = cx.debug_bounds("split-hit-target:p28").unwrap();
        let parent = cx.debug_bounds("split-container:p28").unwrap();
        let start = handle.center();
        let events = Rc::new(RefCell::new(Vec::<PaneEvent<String>>::new()));
        let event_log = events.clone();
        let observed = view.clone();
        view.update(cx, move |_, cx| {
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                event_log.borrow_mut().push(event.clone());
            })
            .detach();
        });

        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        events.borrow_mut().clear();
        let (base_mutations, base_notifications) = view.read_with(cx, |view, _| {
            (view.split_move_mutations, view.notifications)
        });
        for step in 1..=4 {
            cx.simulate_mouse_move(
                gpui::point(start.x + parent.size.width * (step as f32 * 0.05), start.y),
                Some(MouseButton::Left),
                gpui::Modifiers::none(),
            );
        }
        cx.run_until_parked();

        let logged = events.borrow();
        assert_eq!(logged.len(), 8);
        for pair in logged.chunks_exact(2) {
            assert!(matches!(pair[0], PaneEvent::Resized { .. }));
            assert!(matches!(pair[1], PaneEvent::TreeChanged { .. }));
        }
        drop(logged);
        view.read_with(cx, |view, _| {
            assert_eq!(view.split_move_mutations - base_mutations, 4);
            assert_eq!(view.notifications - base_notifications, 4);
        });
        assert_eq!(PERF_FILTER_CALLS.load(Ordering::SeqCst), initial_filters);
        assert_eq!(view.read_with(cx, |view, _| view.activity_cache_syncs), 1);

        // The first out-of-range move reaches the clamp; identical clamped moves
        // are silent and cannot schedule extra root work.
        events.borrow_mut().clear();
        let (base_mutations, base_notifications) = view.read_with(cx, |view, _| {
            (view.split_move_mutations, view.notifications)
        });
        let clamped = gpui::point(parent.right() + parent.size.width, start.y);
        cx.simulate_mouse_move(clamped, Some(MouseButton::Left), gpui::Modifiers::none());
        for _ in 0..4 {
            cx.simulate_mouse_move(clamped, Some(MouseButton::Left), gpui::Modifiers::none());
        }
        cx.run_until_parked();
        assert_eq!(events.borrow().len(), 2);
        view.read_with(cx, |view, _| {
            assert_eq!(view.split_move_mutations - base_mutations, 1);
            assert_eq!(view.notifications - base_notifications, 1);
        });
        assert_eq!(PERF_FILTER_CALLS.load(Ordering::SeqCst), initial_filters);
        assert_eq!(view.read_with(cx, |view, _| view.activity_cache_syncs), 1);
    }

    #[gpui::test]
    fn horizontal_split_drag_is_proportional_clamped_exact_and_released(cx: &mut TestAppContext) {
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, vec![], cx));
        cx.simulate_resize(gpui::size(px(1000.), px(700.)));
        cx.run_until_parked();

        let events = Rc::new(RefCell::new(Vec::<PaneEvent<String>>::new()));
        let event_log = events.clone();
        let observed = view.clone();
        view.update(cx, move |_, cx| {
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                event_log.borrow_mut().push(event.clone());
            })
            .detach();
        });

        let handle = cx.debug_bounds("split-hit-target:b").unwrap();
        let parent = cx.debug_bounds("split-container:b").unwrap();
        let bar = cx.debug_bounds("split-handle:b").unwrap();
        assert_eq!(handle.size.width, px(8.));
        assert_eq!(bar.size.width, px(4.));
        assert_eq!(handle.center().x, bar.center().x);
        let start = handle.center();
        cx.simulate_mouse_down(start, MouseButton::Right, gpui::Modifiers::none());
        cx.simulate_mouse_up(start, MouseButton::Right, gpui::Modifiers::none());
        assert_eq!(ratio(&view, "b", cx), 0.5);

        cx.simulate_click(start, gpui::Modifiers::none());
        cx.dispatch_action(ResizeSplitIncrease);
        assert_eq!(ratio(&view, "b", cx), 0.55);
        cx.dispatch_action(ResizeSplitDecrease);
        assert_eq!(ratio(&view, "b", cx), 0.5);

        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(
            gpui::point(start.x + parent.size.width / 4., start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        assert_eq!(ratio(&view, "b", cx), 0.75);

        let logged = events.borrow();
        let resized = logged.iter().rev().find_map(|event| match event {
            PaneEvent::Resized { split_key, ratio } => Some((split_key, *ratio)),
            _ => None,
        });
        assert_eq!(resized, Some((&PaneId::new("b"), 0.75)));
        let snapshot_ratio = logged.iter().rev().find_map(|event| match event {
            PaneEvent::TreeChanged { tree } => crate::tree::find_ratio(tree, &PaneId::new("b")),
            _ => None,
        });
        assert_eq!(snapshot_ratio, Some(0.75));
        drop(logged);

        cx.simulate_mouse_move(
            gpui::point(parent.right() + parent.size.width, start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        assert_eq!(ratio(&view, "b", cx), 0.9);
        cx.simulate_mouse_up(
            gpui::point(parent.right() + parent.size.width, start.y),
            MouseButton::Left,
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(start, None, gpui::Modifiers::none());
        assert_eq!(ratio(&view, "b", cx), 0.9);
    }

    #[gpui::test]
    fn nested_vertical_drag_uses_its_parent_bounds_and_cancels(cx: &mut TestAppContext) {
        let nested = split(SplitDirection::Vertical, 0.5, leaf("b"), leaf("c"));
        let tree = split(SplitDirection::Horizontal, 0.4, leaf("a"), nested);
        let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, vec![], cx));
        cx.simulate_resize(gpui::size(px(1000.), px(800.)));
        cx.run_until_parked();

        let handle = cx.debug_bounds("split-hit-target:c").unwrap();
        let parent = cx.debug_bounds("split-container:c").unwrap();
        assert_eq!(handle.size.height, px(8.));
        assert!(parent.size.width < cx.debug_bounds("split-container:b").unwrap().size.width);
        let start = handle.center();
        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x, start.y + px(4.)),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(
            gpui::point(start.x, start.y + parent.size.height / 4.),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        assert_eq!(ratio(&view, "c", cx), 0.75);
        assert_eq!(ratio(&view, "b", cx), 0.4);

        cx.dispatch_action(CancelSplitResize);
        assert_eq!(ratio(&view, "c", cx), 0.5);
        cx.simulate_mouse_move(
            gpui::point(start.x, start.y + parent.size.height / 3.),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        assert_eq!(ratio(&view, "c", cx), 0.5);
    }

    fn focused(
        view: &gpui::Entity<MullionView<String>>,
        cx: &mut gpui::VisualTestContext,
    ) -> Option<PaneId> {
        view.read_with(cx, |view, _| view.model().focused().cloned())
    }

    #[gpui::test]
    fn default_hover_and_click_only_left_press_follow_reference_policy(cx: &mut TestAppContext) {
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, vec![], cx));
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.focus_behavior()),
            PaneFocusBehavior::Hover
        );

        let b = cx.debug_bounds("pane:b").unwrap().center();
        cx.simulate_mouse_move(b, None, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));

        view.update(cx, |view, cx| {
            view.set_focus_behavior(PaneFocusBehavior::Click, cx)
        });
        cx.run_until_parked();
        let a = cx.debug_bounds("pane:a").unwrap().center();
        cx.simulate_mouse_move(a, None, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));
        cx.simulate_mouse_down(a, MouseButton::Right, gpui::Modifiers::none());
        cx.simulate_mouse_up(a, MouseButton::Right, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));
        cx.simulate_mouse_down(a, MouseButton::Left, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("a")));
    }

    #[gpui::test]
    fn controlled_focus_setting_is_read_live(cx: &mut TestAppContext) {
        let host = Arc::new(AtomicU8::new(0));
        let reader = host.clone();
        let writer = host.clone();
        let settings = MullionSettings::controlled(
            move || {
                if reader.load(Ordering::SeqCst) == 0 {
                    PaneFocusBehavior::Hover
                } else {
                    PaneFocusBehavior::Click
                }
            },
            move |behavior| {
                writer.store(
                    u8::from(behavior == PaneFocusBehavior::Click),
                    Ordering::SeqCst,
                )
            },
        );
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(tree, vec![], cx).with_settings(settings)
        });
        cx.run_until_parked();
        let b = cx.debug_bounds("pane:b").unwrap().center();
        cx.simulate_mouse_move(b, None, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));

        host.store(1, Ordering::SeqCst);
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        let a = cx.debug_bounds("pane:a").unwrap().center();
        cx.simulate_mouse_move(a, None, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));
        cx.simulate_mouse_down(a, MouseButton::Left, gpui::Modifiers::none());
        assert_eq!(focused(&view, cx), Some(PaneId::new("a")));
    }

    #[gpui::test]
    fn focus_presentation_is_opt_in_internal_and_keeps_washed_panes_interactive(
        cx: &mut TestAppContext,
    ) {
        let left = split(SplitDirection::Vertical, 0.5, leaf("a"), leaf("c"));
        let tree = split(SplitDirection::Horizontal, 0.5, left, leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(tree, vec![], cx).with_focus_behavior(PaneFocusBehavior::Click)
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.focus_presentation()),
            FocusPresentation::default()
        );
        assert!(cx.debug_bounds("focus-edge:a:right").is_none());

        view.update(cx, |view, cx| {
            view.set_focus_presentation(
                FocusPresentation::new()
                    .with_focus_indicator(true)
                    .with_unfocused_pane_opacity(-4.0),
                cx,
            );
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.presentation().unfocused_pane_opacity()),
            0.0
        );
        assert!(cx.debug_bounds("focus-edge:a:right").is_some());
        assert!(cx.debug_bounds("focus-edge:a:bottom").is_some());
        assert!(cx.debug_bounds("focus-edge:a:left").is_none());
        assert!(cx.debug_bounds("focus-edge:a:top").is_none());
        assert!(cx.debug_bounds("pane-visual:b").is_some());

        let b = cx.debug_bounds("pane-visual:b").unwrap().center();
        cx.simulate_mouse_down(b, MouseButton::Left, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(focused(&view, cx), Some(PaneId::new("b")));
        assert!(cx.debug_bounds("focus-edge:b:left").is_some());
        assert!(cx.debug_bounds("focus-edge:b:right").is_none());
        assert!(cx.debug_bounds("focus-edge:b:top").is_none());
        assert!(cx.debug_bounds("focus-edge:b:bottom").is_none());
    }

    #[gpui::test]
    fn keyboard_focus_zoom_close_and_tree_replacement_stay_coherent(cx: &mut TestAppContext) {
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, vec![], cx));
        cx.run_until_parked();
        let a = cx.debug_bounds("pane:a").unwrap().center();
        cx.simulate_mouse_move(a, None, gpui::Modifiers::none());
        cx.dispatch_action(ToggleZoom);
        cx.dispatch_action(FocusNext);
        view.read_with(cx, |view, _| {
            assert_eq!(view.model().focused(), Some(&PaneId::new("b")));
            assert_eq!(view.model().zoomed(), view.model().focused());
        });
        cx.dispatch_action(ClosePane);
        view.read_with(cx, |view, _| {
            assert_eq!(view.model().focused(), Some(&PaneId::new("a")));
            assert_eq!(view.model().zoomed(), None);
        });

        view.update(cx, |view, cx| {
            view.update_model(cx, |model| {
                model.replace_tree(split(SplitDirection::Vertical, 0.5, leaf("c"), leaf("d")));
            });
        });
        view.read_with(cx, |view, _| {
            assert_eq!(view.model().focused(), Some(&PaneId::new("c")));
            assert_eq!(view.model().zoomed(), None);
        });
    }

    #[gpui::test]
    fn workspace_switch_reconciles_focus_and_zoom(cx: &mut TestAppContext) {
        let workspaces = WorkspaceSet::try_new(
            WorkspaceId("one".into()),
            vec![
                crate::Workspace {
                    id: WorkspaceId("one".into()),
                    name: "One".into(),
                    tree: split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b")),
                },
                crate::Workspace {
                    id: WorkspaceId("two".into()),
                    name: "Two".into(),
                    tree: leaf("c"),
                },
            ],
        )
        .unwrap();
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspaces, vec![], cx).unwrap()
        });
        view.update(cx, |view, cx| {
            view.update_model(cx, |model| {
                model.focus(&PaneId::new("b"));
                model.toggle_zoom();
            });
            assert!(view.switch_workspace(&WorkspaceId("two".into()), cx));
        });
        view.read_with(cx, |view, _| {
            assert_eq!(view.model().focused(), Some(&PaneId::new("c")));
            assert_eq!(view.model().zoomed(), None);
        });
    }

    fn workspace_set(second_tree: PaneNode<String>) -> WorkspaceSet<String> {
        WorkspaceSet::try_new(
            WorkspaceId("one".into()),
            vec![
                crate::Workspace {
                    id: WorkspaceId("one".into()),
                    name: "One".into(),
                    tree: split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b")),
                },
                crate::Workspace {
                    id: WorkspaceId("two".into()),
                    name: "Two".into(),
                    tree: second_tree,
                },
            ],
        )
        .unwrap()
    }

    #[gpui::test]
    fn mounted_workspace_operations_emit_complete_snapshots_and_update_active_model(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspace_set(leaf("c")), vec![], cx).unwrap()
        });
        let snapshots = Rc::new(RefCell::new(Vec::new()));
        let log = snapshots.clone();
        let observed = view.clone();
        view.update(cx, move |_, cx| {
            cx.subscribe(&observed, move |_, _, event: &WorkspaceEvent<String>, _| {
                log.borrow_mut().push(event.clone());
            })
            .detach();
        });

        view.update(cx, |view, cx| {
            assert_eq!(
                view.add_workspace(
                    crate::Workspace {
                        id: WorkspaceId("three".into()),
                        name: "Three".into(),
                        tree: leaf("d"),
                    },
                    cx,
                ),
                Ok(2)
            );
            assert_eq!(
                view.rename_workspace(&WorkspaceId("three".into()), "Third", cx),
                Ok("Three".into())
            );
            assert_eq!(
                view.reorder_workspace(&WorkspaceId("three".into()), 1, cx),
                Ok(2)
            );
            assert_eq!(
                view.update_workspace_tree(&WorkspaceId("one".into()), leaf("z"), cx)
                    .unwrap()
                    .leaf_ids(),
                vec![PaneId::new("a"), PaneId::new("b")]
            );
            assert_eq!(view.model().tree().leaf_ids(), vec![PaneId::new("z")]);
            assert_eq!(
                view.remove_workspace(&WorkspaceId("three".into()), cx)
                    .unwrap()
                    .name,
                "Third"
            );
        });
        assert_eq!(snapshots.borrow().len(), 5);
        for event in snapshots.borrow().iter() {
            let WorkspaceEvent::SnapshotChanged { workspaces } = event;
            workspaces.validate().unwrap();
            serde_json::to_string(event).unwrap();
        }
    }

    #[gpui::test]
    fn workspace_switch_is_atomic_persists_outgoing_and_orders_transient_events(
        cx: &mut TestAppContext,
    ) {
        #[derive(Debug, PartialEq)]
        enum Seen {
            Focus(Option<PaneId>),
            Zoom(Option<PaneId>),
            Snapshot,
            Changed,
        }
        let incoming = split(SplitDirection::Horizontal, 0.5, leaf("b"), leaf("c"));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspace_set(incoming), vec![], cx).unwrap()
        });
        let seen = Rc::new(RefCell::new(Vec::new()));
        let observed = view.clone();
        view.update(cx, |_, cx| {
            let log = seen.clone();
            cx.subscribe(
                &observed,
                move |_, _, event: &PaneEvent<String>, _| match event {
                    PaneEvent::FocusChanged { pane } => {
                        log.borrow_mut().push(Seen::Focus(pane.clone()))
                    }
                    PaneEvent::ZoomChanged { pane } => {
                        log.borrow_mut().push(Seen::Zoom(pane.clone()))
                    }
                    _ => {}
                },
            )
            .detach();
            let log = seen.clone();
            cx.subscribe(&observed, move |_, _, _: &WorkspaceEvent<String>, _| {
                log.borrow_mut().push(Seen::Snapshot);
            })
            .detach();
            let log = seen.clone();
            cx.subscribe(&observed, move |_, _, _: &WorkspaceChanged, _| {
                log.borrow_mut().push(Seen::Changed);
            })
            .detach();
        });
        view.update(cx, |view, cx| {
            view.update_model(cx, |model| {
                model.focus(&PaneId::new("a"));
                model.toggle_zoom();
                model.update_data(&PaneId::new("a"), "persisted".into());
            });
        });
        cx.run_until_parked();
        seen.borrow_mut().clear();
        view.update(cx, |view, cx| {
            let before = view.workspaces().unwrap().clone();
            assert!(matches!(
                view.try_switch_workspace(&WorkspaceId("missing".into()), cx),
                Err(WorkspaceSetError::WorkspaceNotFound { .. })
            ));
            assert_eq!(view.workspaces().unwrap(), &before);
            assert!(view
                .try_switch_workspace(&WorkspaceId("two".into()), cx)
                .unwrap());
            assert_eq!(
                view.workspaces().unwrap().workspaces[0].tree.leaf_ids(),
                vec![PaneId::new("a"), PaneId::new("b")]
            );
            assert!(matches!(
                view.workspaces().unwrap().workspaces[0]
                    .tree
                    .find(&PaneId::new("a")),
                Some(PaneNode::Leaf { data, .. }) if data == "persisted"
            ));
            let switched = view.workspaces().unwrap().clone();
            assert!(!view
                .try_switch_workspace(&WorkspaceId("two".into()), cx)
                .unwrap());
            assert_eq!(view.workspaces().unwrap(), &switched);
        });
        assert_eq!(
            *seen.borrow(),
            vec![
                Seen::Focus(Some(PaneId::new("b"))),
                Seen::Zoom(None),
                Seen::Snapshot,
                Seen::Changed,
            ]
        );
    }

    #[gpui::test]
    fn typed_workspace_constructor_rejects_invalid_persistence(cx: &mut TestAppContext) {
        let invalid = WorkspaceSet {
            active: WorkspaceId("missing".into()),
            workspaces: vec![crate::Workspace {
                id: WorkspaceId("one".into()),
                name: "One".into(),
                tree: leaf("a"),
            }],
        };
        let captured = Rc::new(RefCell::new(None));
        let error = captured.clone();
        let invalid_for_typed = invalid.clone();
        let _ = cx.add_window_view(move |_, cx| {
            match MullionView::try_new_with_workspaces(invalid_for_typed, vec![], cx) {
                Ok(view) => view,
                Err(found) => {
                    *error.borrow_mut() = Some(found);
                    MullionView::new(leaf("fallback"), vec![], cx)
                }
            }
        });
        assert!(matches!(
            captured.borrow().as_ref(),
            Some(WorkspaceSetError::ActiveWorkspaceNotFound { .. })
        ));
        let optional_was_none = Rc::new(Cell::new(false));
        let was_none = optional_was_none.clone();
        let _ = cx.add_window_view(move |_, cx| {
            let result = MullionView::new_with_workspaces(invalid, vec![], cx);
            was_none.set(result.is_none());
            result.unwrap_or_else(|| MullionView::new(leaf("fallback"), vec![], cx))
        });
        assert!(optional_was_none.get());
    }

    #[gpui::test]
    fn workspace_switch_preserves_overlapping_focus_and_zoom_without_transient_events(
        cx: &mut TestAppContext,
    ) {
        let incoming = split(SplitDirection::Horizontal, 0.5, leaf("b"), leaf("c"));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspace_set(incoming), vec![], cx).unwrap()
        });
        let pane_events = Rc::new(RefCell::new(Vec::new()));
        let observed = view.clone();
        view.update(cx, |_, cx| {
            let log = pane_events.clone();
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                log.borrow_mut().push(event.clone());
            })
            .detach();
        });
        view.update(cx, |view, cx| {
            view.update_model(cx, |model| {
                model.focus(&PaneId::new("b"));
                model.toggle_zoom();
            });
        });
        cx.run_until_parked();
        pane_events.borrow_mut().clear();
        view.update(cx, |view, cx| {
            assert!(view.switch_workspace(&WorkspaceId("two".into()), cx));
            assert_eq!(view.model().focused(), Some(&PaneId::new("b")));
            assert_eq!(view.model().zoomed(), Some(&PaneId::new("b")));
        });
        cx.run_until_parked();
        assert!(pane_events.borrow().is_empty());
    }

    #[gpui::test]
    fn removing_workspace_disposes_only_its_activity_cache_namespace(cx: &mut TestAppContext) {
        let disposals = Rc::new(Cell::new(0));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new_with_workspaces(workspace_set(leaf("c")), vec![], cx).unwrap()
        });
        view.update(cx, |view, cx| {
            for workspace in ["one", "two"] {
                let count = disposals.clone();
                let body = cx.new(|_| StatefulBody);
                view.activity_cache.insert(
                    ActivityCacheKey::new(
                        Some(WorkspaceId(workspace.into())),
                        PaneId::new("pane"),
                        ActivityId::new("activity"),
                    ),
                    crate::ActivityInstance::new(body)
                        .with_dispose(move |_| count.set(count.get() + 1)),
                    "data".into(),
                );
            }
            view.remove_workspace(&WorkspaceId("two".into()), cx)
                .unwrap();
            assert_eq!(disposals.get(), 1);
        });
    }

    #[test]
    fn legacy_activity_struct_literal_remains_source_compatible() {
        let activity = legacy_activity();
        let rendered = (activity.render)(&PaneId::new("pane"), &"data".to_string());
        drop(rendered);
    }

    macro_rules! pinned_edge_test {
        ($name:ident, $edge:expr) => {
            #[gpui::test]
            fn $name(cx: &mut TestAppContext) {
                let edge = $edge;
                let host =
                    ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
                        edge,
                        ..crate::ActivityBarConfig::default()
                    });
                let (_, cx) = cx.add_window_view(move |_, cx| {
                    MullionView::new(leaf("pane"), vec![], cx).with_activity_bar_host(host)
                });
                cx.simulate_resize(gpui::size(px(500.), px(320.)));
                cx.run_until_parked();
                let visual = cx.debug_bounds("pane-visual:pane").unwrap();
                let content = cx.debug_bounds("pane-content:pane").unwrap();
                let bar = cx.debug_bounds("activity-bar:pane").unwrap();
                match edge {
                    ActivityBarEdge::Left => {
                        assert_eq!(bar.left(), visual.left());
                        assert_eq!(bar.size.width, px(28.));
                        assert_eq!(content.left(), bar.right());
                    }
                    ActivityBarEdge::Right => {
                        assert_eq!(bar.right(), visual.right());
                        assert_eq!(bar.size.width, px(28.));
                        assert_eq!(content.right(), bar.left());
                    }
                    ActivityBarEdge::Top => {
                        assert_eq!(bar.top(), visual.top());
                        assert_eq!(bar.size.height, px(28.));
                        assert_eq!(content.top(), bar.bottom());
                    }
                    ActivityBarEdge::Bottom => {
                        assert_eq!(bar.bottom(), visual.bottom());
                        assert_eq!(bar.size.height, px(28.));
                        assert_eq!(content.bottom(), bar.top());
                    }
                }
            }
        };
    }

    pinned_edge_test!(pinned_left_rail_precedes_content, ActivityBarEdge::Left);
    pinned_edge_test!(pinned_right_rail_follows_content, ActivityBarEdge::Right);
    pinned_edge_test!(pinned_top_rail_precedes_content, ActivityBarEdge::Top);
    pinned_edge_test!(pinned_bottom_rail_follows_content, ActivityBarEdge::Bottom);

    #[gpui::test]
    fn pinned_pane_controls_have_exact_bounds_secondary_order_and_split_dispatch(
        cx: &mut TestAppContext,
    ) {
        let slots = crate::ActivityBarSlots::new()
            .with_leading(|_, _, _, _| {
                div()
                    .size(px(6.))
                    .debug_selector(|| "bottom-leading".into())
                    .into_any_element()
            })
            .with_trailing(|_, _, _, _| {
                div()
                    .size(px(6.))
                    .debug_selector(|| "bottom-trailing".into())
                    .into_any_element()
            })
            .with_pane_accessory(|_, _, _, _| {
                div()
                    .size(px(6.))
                    .debug_selector(|| "pane-accessory-order".into())
                    .into_any_element()
            });
        let catalog = ActivityCatalog::new(Vec::new()).with_trailing(vec![ActivityNode::Activity(
            rendered_activity("trailing-control", show_activity),
        )]);
        let tree = PaneNode::leaf_with_activity(
            PaneId::new("pane"),
            ActivityId::new("trailing-control"),
            "data".to_owned(),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_catalog(tree, catalog, cx)
                .unwrap()
                .with_activity_bar_host(ActivityBarHostConfig::new().with_slots(slots))
                .with_split_factory_fn(|pane, direction, data| {
                    let suffix = match direction {
                        SplitDirection::Horizontal => "h",
                        SplitDirection::Vertical => "v",
                    };
                    Some((PaneId::new(format!("{}-{suffix}", pane.0)), data.clone()))
                })
        });
        cx.simulate_resize(gpui::size(px(500.), px(400.)));
        cx.run_until_parked();

        let ordered = [
            cx.debug_bounds("bottom-leading").unwrap(),
            cx.debug_bounds("activity:pane:trailing-control").unwrap(),
            cx.debug_bounds("bottom-trailing").unwrap(),
            cx.debug_bounds("pane-accessory-order").unwrap(),
            cx.debug_bounds("pane-control:split-h:pane").unwrap(),
            cx.debug_bounds("pane-control:split-v:pane").unwrap(),
            cx.debug_bounds("pane-control:close:pane").unwrap(),
        ];
        assert!(ordered
            .windows(2)
            .all(|pair| pair[0].bottom() <= pair[1].top()));
        for selector in [
            "pane-control:move:pane",
            "pane-control:split-h:pane",
            "pane-control:split-v:pane",
            "pane-control:close:pane",
        ] {
            assert_eq!(
                cx.debug_bounds(selector).unwrap().size,
                gpui::size(px(28.), px(28.))
            );
        }

        let split_h = cx
            .debug_bounds("pane-control:split-h:pane")
            .unwrap()
            .center();
        cx.simulate_click(split_h, gpui::Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.model().tree().find(&PaneId::new("pane-h")).is_some());
            assert_eq!(
                view.routed_commands.last(),
                Some(&crate::PaneCommand::Split(SplitDirection::Horizontal))
            );
        });
        let split_v = cx
            .debug_bounds("pane-control:split-v:pane")
            .unwrap()
            .center();
        cx.simulate_click(split_v, gpui::Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.model().tree().find(&PaneId::new("pane-v")).is_some());
            assert_eq!(
                view.routed_commands.last(),
                Some(&crate::PaneCommand::Split(SplitDirection::Vertical))
            );
        });
    }

    #[gpui::test]
    fn hidden_capsule_has_exact_inset_bounds_order_and_disabled_controls_refuse_clicks(
        cx: &mut TestAppContext,
    ) {
        let host = ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
            mode: ActivityBarMode::Hidden,
            ..crate::ActivityBarConfig::default()
        });
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_activity_bar_host(host)
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();

        let pane = cx.debug_bounds("pane:pane").unwrap();
        let capsule = cx.debug_bounds("pane-controls:pane").unwrap();
        assert_eq!(capsule.top(), pane.top() + px(6.));
        assert_eq!(capsule.right(), pane.right() - px(6.));
        assert_eq!(capsule.size, gpui::size(px(100.), px(28.)));
        let selectors = [
            "pane-control:move:pane",
            "pane-control:split-h:pane",
            "pane-control:split-v:pane",
            "pane-control:close:pane",
        ];
        let bounds = selectors.map(|selector| cx.debug_bounds(selector).unwrap());
        assert!(bounds
            .windows(2)
            .all(|pair| pair[0].right() + px(2.) == pair[1].left()));
        assert!(bounds
            .iter()
            .all(|bounds| bounds.size == gpui::size(px(22.), px(22.))));

        cx.simulate_click(bounds[1].center(), gpui::Modifiers::none());
        cx.simulate_click(bounds[3].center(), gpui::Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.model().tree().leaf_ids(), vec![PaneId::new("pane")]);
            assert!(view.routed_commands.is_empty());
        });
    }

    #[gpui::test]
    fn hidden_and_autohide_rails_have_exact_overlay_geometry_and_cancel_stale_intent(
        cx: &mut TestAppContext,
    ) {
        let hidden = ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
            mode: ActivityBarMode::Hidden,
            ..crate::ActivityBarConfig::default()
        });
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_activity_bar_host(hidden)
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();
        assert!(cx.debug_bounds("activity-bar:pane").is_none());

        view.update(cx, |view, cx| {
            view.host.activity_bar = crate::ActivityBarConfig {
                edge: ActivityBarEdge::Left,
                mode: ActivityBarMode::AutoHide,
                behavior: crate::ActivityBarBehavior {
                    hover_expand: true,
                    hover_intent: crate::ActivityBarHoverIntent {
                        expand_delay_ms: 50,
                    },
                },
            };
            cx.notify();
        });
        cx.run_until_parked();
        let content_before = cx.debug_bounds("pane-content:pane").unwrap();
        let strip = cx.debug_bounds("activity-bar:pane").unwrap();
        assert_eq!(strip.size.width, px(0.));
        let trigger = cx.debug_bounds("activity-bar-trigger:pane").unwrap();
        assert_eq!(trigger.size.width, px(12.));
        cx.simulate_mouse_move(trigger.center(), None, gpui::Modifiers::none());
        cx.simulate_mouse_move(content_before.center(), None, gpui::Modifiers::none());
        cx.executor().advance_clock(Duration::from_millis(60));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity-bar:pane").unwrap().size.width,
            px(0.)
        );

        view.update(cx, |view, cx| {
            view.host.activity_bar.behavior.hover_intent.expand_delay_ms = 0;
            cx.notify();
        });
        cx.run_until_parked();
        let trigger = cx.debug_bounds("activity-bar-trigger:pane").unwrap();
        cx.simulate_mouse_move(trigger.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        let expanded = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(
            cx.debug_bounds("activity-bar:pane").unwrap().size.width,
            px(0.)
        );
        assert_eq!(expanded.size.width, px(158.));
        assert_eq!(
            expanded.left(),
            cx.debug_bounds("pane-visual:pane").unwrap().left()
        );
        assert_eq!(
            cx.debug_bounds("pane-content:pane").unwrap(),
            content_before
        );
    }

    fn rendered_activity(id: &str, filter: fn(&String) -> bool) -> Activity<String> {
        Activity {
            id: ActivityId::new(id),
            name: id.to_owned().into(),
            filter,
            render: Arc::new(|_, _| div().child("legacy body").into_any_element()),
        }
    }

    fn show_activity(_: &String) -> bool {
        true
    }

    fn hide_activity(_: &String) -> bool {
        false
    }

    #[gpui::test]
    fn pinned_vertical_panel_and_horizontal_item_expand_without_resizing_content(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let catalog = ActivityCatalog::new(vec![ActivityNode::Activity(rendered_activity(
            "activity",
            show_activity,
        ))]);
        let tree = PaneNode::leaf_with_activity(
            PaneId::new("pane"),
            ActivityId::new("activity"),
            "data".to_owned(),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_catalog(tree, catalog, cx).unwrap()
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();
        let content = cx.debug_bounds("pane-content:pane").unwrap();
        assert!(cx.debug_bounds("activity-label:pane:activity").is_some());
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(28.));
        cx.simulate_mouse_move(panel.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity-bar-panel:pane")
                .unwrap()
                .size
                .width,
            px(158.)
        );
        assert_eq!(cx.debug_bounds("pane-content:pane").unwrap(), content);

        view.update(cx, |view, cx| {
            view.host.activity_bar.edge = ActivityBarEdge::Top;
            view.hover.clear();
            view.hovered_bar_items.clear();
            cx.notify();
        });
        cx.run_until_parked();
        let content = cx.debug_bounds("pane-content:pane").unwrap();
        let item = cx.debug_bounds("activity:pane:activity").unwrap();
        assert_eq!(item.size.width, px(28.));
        cx.simulate_mouse_move(item.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.simulate_mouse_move(item.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity:pane:activity")
                .unwrap()
                .size
                .width,
            px(150.)
        );
        assert_eq!(cx.debug_bounds("pane-content:pane").unwrap(), content);
    }

    #[gpui::test]
    fn live_palette_projects_searches_and_executes_typed_invocations(cx: &mut TestAppContext) {
        let activities = vec![
            ActivityNode::Activity(rendered_activity("visible", show_activity)),
            ActivityNode::Activity(rendered_activity("hidden", hide_activity)),
        ];
        let tree = split(
            SplitDirection::Horizontal,
            0.5,
            PaneNode::leaf_with_activity(PaneId::new("a"), ActivityId::new("visible"), "a".into()),
            leaf("b"),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| MullionView::new(tree, activities, cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = view.clone();
        view.update(cx, |_, cx| {
            let events = events.clone();
            cx.subscribe(&observed, move |_, _, event: &PaneEvent<String>, _| {
                events.borrow_mut().push(event.clone());
            })
            .detach();
        });

        view.update(cx, |view, _| {
            let entries = view.palette_entries();
            assert_eq!(
                entries
                    .iter()
                    .filter(|entry| matches!(entry.invocation, PaletteInvocation::PaneCommand(_)))
                    .count(),
                39
            );
            assert!(entries
                .iter()
                .any(|entry| entry.id == "mullion.activity.a.visible"));
            assert!(!entries.iter().any(|entry| entry.id.contains("hidden")));
            assert_eq!(view.search_palette("visible")[0].entry.name, "visible");
        });
        view.update(cx, |view, cx| {
            view.invoke_palette(
                PaletteInvocation::PaneCommand(crate::PaneCommand::FocusIndex(1)),
                cx,
            )
            .unwrap();
            assert_eq!(view.model().focused(), Some(&PaneId::new("b")));
            assert!(matches!(
                view.invoke_palette(
                    PaletteInvocation::PaneCommand(crate::PaneCommand::Split(
                        SplitDirection::Horizontal
                    )),
                    cx,
                ),
                Err(PaletteInvocationError::Command(
                    crate::PaneCommandError::SplitUnavailable
                ))
            ));
            assert!(matches!(
                view.invoke_palette(
                    PaletteInvocation::SelectActivity {
                        pane: PaneId::new("missing"),
                        activity: ActivityId::new("visible"),
                    },
                    cx,
                ),
                Err(PaletteInvocationError::PaneNotFound(_))
            ));
            view.invoke_palette(
                PaletteInvocation::SelectActivity {
                    pane: PaneId::new("b"),
                    activity: ActivityId::new("visible"),
                },
                cx,
            )
            .unwrap();
        });
        cx.run_until_parked();
        assert!(events.borrow().iter().any(|event| matches!(
            event,
            PaneEvent::ActivityChanged { pane, activity }
                if pane == &PaneId::new("b")
                    && activity.as_ref() == Some(&ActivityId::new("visible"))
        )));
    }

    #[gpui::test]
    fn rendered_catalog_composes_recursive_chrome_slots_activation_and_trailing_cache(
        cx: &mut TestAppContext,
    ) {
        let primary = rendered_activity("primary", show_activity);
        let nested = rendered_activity("nested", show_activity);
        let filtered = rendered_activity("filtered", hide_activity);
        let trailing = rendered_activity("trailing", show_activity);
        let catalog = ActivityCatalog::new(vec![
            ActivityNode::Activity(primary),
            ActivityNode::Category(crate::ActivityCategory {
                id: crate::CategoryId::new("category"),
                name: "Category".into(),
                color: gpui::rgb(0x112233).into(),
                children: vec![ActivityNode::Activity(nested)],
            }),
            ActivityNode::Category(crate::ActivityCategory {
                id: crate::CategoryId::new("pruned"),
                name: "Pruned".into(),
                color: gpui::rgb(0x334455).into(),
                children: vec![ActivityNode::Activity(filtered)],
            }),
        ])
        .with_trailing(vec![ActivityNode::Activity(trailing)])
        .with_activity_chrome(
            ActivityId::new("primary"),
            crate::ActivityChrome::new(crate::ActivityIcon::new(|_, _| {
                div()
                    .debug_selector(|| "primary-icon".into())
                    .into_any_element()
            }))
            .with_header(|_, _, _, _| {
                div()
                    .debug_selector(|| "activity-header".into())
                    .into_any_element()
            }),
        )
        .with_activity_chrome(
            ActivityId::new("trailing"),
            crate::ActivityChrome::new(crate::ActivityIcon::new(|_, _| {
                div()
                    .debug_selector(|| "trailing-icon".into())
                    .into_any_element()
            })),
        )
        .with_category_chrome(
            crate::CategoryId::new("category"),
            crate::CategoryChrome::new(crate::ActivityIcon::new(|_, _| {
                div()
                    .debug_selector(|| "category-icon".into())
                    .into_any_element()
            })),
        );
        let slots = crate::ActivityBarSlots::new()
            .with_app_icon(crate::ActivityIcon::new(|_, _| {
                div()
                    .debug_selector(|| "app-icon".into())
                    .into_any_element()
            }))
            .with_leading(|_, _, _, _| {
                div()
                    .debug_selector(|| "leading-slot".into())
                    .into_any_element()
            })
            .with_trailing(|_, _, _, _| {
                div()
                    .debug_selector(|| "trailing-slot".into())
                    .into_any_element()
            })
            .with_pane_accessory(|_, _, _, _| {
                div()
                    .debug_selector(|| "pane-accessory".into())
                    .into_any_element()
            });
        let host = ActivityBarHostConfig::new().with_slots(slots).with_header(
            crate::PaneHeaderConfig::new().with_accessory(|_, _, _, _| {
                div()
                    .debug_selector(|| "header-accessory".into())
                    .into_any_element()
            }),
        );
        let registry = ActivityFactoryRegistry::new()
            .with_factory(ActivityId::new("trailing"), |_, _, _, cx| {
                crate::ActivityInstance::new(cx.new(|_| StatefulBody))
            });
        let tree = PaneNode::leaf_with_activity(
            PaneId::new("pane"),
            ActivityId::new("primary"),
            "data".to_owned(),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_catalog(tree, catalog, cx)
                .unwrap()
                .with_activity_bar_host(host)
                .with_activity_factories(registry)
        });
        cx.run_until_parked();
        for selector in [
            "primary-icon",
            "trailing-icon",
            "category-icon",
            "app-icon",
            "leading-slot",
            "trailing-slot",
            "activity-header",
            "header-accessory",
            "pane-accessory",
        ] {
            assert!(cx.debug_bounds(selector).is_some(), "missing {selector}");
        }
        assert!(cx.debug_bounds("activity-category:pane:pruned").is_none());
        assert!(cx.debug_bounds("activity:pane:nested").is_none());
        let category = cx
            .debug_bounds("activity-category:pane:category")
            .unwrap()
            .center();
        cx.simulate_click(category, gpui::Modifiers::none());
        cx.run_until_parked();
        let nested = cx.debug_bounds("activity:pane:nested").unwrap().center();
        cx.simulate_click(nested, gpui::Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.model()
                    .tree()
                    .find(&PaneId::new("pane"))
                    .and_then(|node| {
                        match node {
                            PaneNode::Leaf {
                                active_activity, ..
                            } => active_activity.as_ref(),
                            _ => None,
                        }
                    }),
                Some(&ActivityId::new("nested"))
            );
        });
        // Manual collapse keeps the active descendant represented by the exact dot.
        let category = cx
            .debug_bounds("activity-category:pane:category")
            .unwrap()
            .center();
        cx.simulate_click(category, gpui::Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds("activity:pane:nested").is_none());
        assert!(cx
            .debug_bounds("activity-category-dot:pane:category")
            .is_some());

        let trailing = cx.debug_bounds("activity:pane:trailing").unwrap().center();
        cx.simulate_click(trailing, gpui::Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view
                .activity_cache
                .get(&ActivityCacheKey::new(
                    None,
                    PaneId::new("pane"),
                    ActivityId::new("trailing"),
                ))
                .is_some());
        });
        assert!(cx.debug_bounds("pane-drag-handle:pane").is_some());
    }

    #[gpui::test]
    fn autohide_all_edges_reserve_zero_use_exact_trigger_and_reveal_when_collapsed(
        cx: &mut TestAppContext,
    ) {
        let host = ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
            mode: ActivityBarMode::AutoHide,
            behavior: crate::ActivityBarBehavior {
                hover_expand: false,
                hover_intent: crate::ActivityBarHoverIntent::default(),
            },
            ..crate::ActivityBarConfig::default()
        });
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_activity_bar_host(host)
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        for edge in [
            ActivityBarEdge::Left,
            ActivityBarEdge::Right,
            ActivityBarEdge::Top,
            ActivityBarEdge::Bottom,
        ] {
            view.update(cx, |view, cx| {
                view.host.activity_bar.edge = edge;
                view.hover.clear();
                cx.notify();
            });
            cx.run_until_parked();
            let pane = cx.debug_bounds("pane-visual:pane").unwrap();
            let scope = cx.debug_bounds("activity-bar:pane").unwrap();
            let trigger = cx.debug_bounds("activity-bar-trigger:pane").unwrap();
            match edge.axis() {
                crate::ActivityBarAxis::Vertical => {
                    assert_eq!(scope.size.width, px(0.));
                    assert_eq!(trigger.size.width, px(12.));
                    assert_eq!(trigger.size.height, pane.size.height);
                }
                crate::ActivityBarAxis::Horizontal => {
                    assert_eq!(scope.size.height, px(0.));
                    assert_eq!(trigger.size.height, px(12.));
                    assert_eq!(trigger.size.width, pane.size.width);
                }
            }
            cx.simulate_mouse_move(trigger.center(), None, gpui::Modifiers::none());
            cx.run_until_parked();
            cx.executor().advance_clock(Duration::from_millis(200));
            cx.run_until_parked();
            let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
            match edge {
                ActivityBarEdge::Left => assert_eq!(panel.left(), pane.left()),
                ActivityBarEdge::Right => assert_eq!(panel.right(), pane.right()),
                ActivityBarEdge::Top => assert_eq!(panel.top(), pane.top()),
                ActivityBarEdge::Bottom => assert_eq!(panel.bottom(), pane.bottom()),
            }
            if edge.is_horizontal() {
                assert_eq!(panel.size.height, px(28.));
            } else {
                assert_eq!(panel.size.width, px(28.));
            }
        }
    }

    #[gpui::test]
    fn custom_style_geometry_and_theme_mode_are_composed_independently(cx: &mut TestAppContext) {
        let mut styles = MullionStyles::default();
        styles.activity_bar.thickness = px(47.);
        styles.split_handle.thickness = px(9.);
        styles.split_handle.hover_target_thickness = px(13.);
        styles.root.background = gpui::rgb(0x010203).into();
        styles.pane.border = gpui::rgb(0x040506).into();
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(tree, vec![], cx)
                .with_styles(styles)
                .with_theme_mode(MullionThemeMode::System)
        });
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity-bar:a").unwrap().size.width,
            px(47.)
        );
        assert_eq!(
            cx.debug_bounds("split-handle:b").unwrap().size.width,
            px(9.)
        );
        assert_eq!(
            cx.debug_bounds("split-hit-target:b").unwrap().size.width,
            px(13.)
        );
        view.read_with(cx, |view, _| {
            assert_eq!(view.styles(), Some(&styles));
            assert_eq!(view.theme_mode(), Some(MullionThemeMode::System));
        });
    }

    #[gpui::test]
    fn typed_catalog_constructor_rejects_invalid_identity(cx: &mut TestAppContext) {
        let duplicate = rendered_activity("duplicate", show_activity);
        let catalog = ActivityCatalog::new(vec![ActivityNode::Activity(duplicate.clone())])
            .with_trailing(vec![ActivityNode::Activity(duplicate)]);
        let error = Rc::new(RefCell::new(None));
        let captured = error.clone();
        let _ = cx.add_window_view(move |_, cx| {
            match MullionView::try_new_with_catalog(leaf("pane"), catalog, cx) {
                Ok(view) => view,
                Err(found) => {
                    *captured.borrow_mut() = Some(found);
                    MullionView::new(leaf("fallback"), vec![], cx)
                }
            }
        });
        assert!(matches!(
            error.borrow().as_ref(),
            Some(ActivityCatalogValidationError::DuplicateActivityId { .. })
        ));
    }
    #[test]
    fn activity_motion_has_exact_linear_geometry_at_eased_progress_endpoints_and_midpoint() {
        let start = activity_motion_sample(ease_in_out(0.0));
        let mid = activity_motion_sample(ease_in_out(0.5));
        let end = activity_motion_sample(ease_in_out(1.0));
        assert_eq!(
            (
                start.vertical_extent,
                start.edge_padding,
                start.row_extent,
                start.label_opacity
            ),
            (28.0, 0.0, 28.0, 0.0)
        );
        assert_eq!(
            (
                mid.vertical_extent,
                mid.edge_padding,
                mid.row_extent,
                mid.label_opacity
            ),
            (93.0, 4.0, 89.0, 0.5)
        );
        assert_eq!(
            (
                end.vertical_extent,
                end.edge_padding,
                end.row_extent,
                end.label_opacity
            ),
            (158.0, 8.0, 150.0, 1.0)
        );
        assert_eq!(start.hidden_translation, 1.0);
        assert_eq!(mid.hidden_translation, 0.5);
        assert_eq!(end.hidden_translation, 0.0);
        assert_eq!(activity_motion_sample(1.0 - ease_in_out(0.0)), end);
        assert_eq!(activity_motion_sample(1.0 - ease_in_out(0.5)), mid);
        assert_eq!(activity_motion_sample(1.0 - ease_in_out(1.0)), start);
    }

    #[test]
    fn drag_and_reduced_motion_use_exact_state_endpoints() {
        let forced = activity_motion_sample(ActivityMotion::endpoint(true));
        assert_eq!(
            (
                forced.vertical_extent,
                forced.row_extent,
                forced.edge_padding,
                forced.label_opacity
            ),
            (158.0, 150.0, 8.0, 1.0)
        );
        let restored = activity_motion_sample(ActivityMotion::endpoint(false));
        assert_eq!(
            (
                restored.vertical_extent,
                restored.row_extent,
                restored.edge_padding,
                restored.label_opacity
            ),
            (28.0, 28.0, 0.0, 0.0)
        );
    }

    #[test]
    fn reference_vector_icons_stay_inside_the_exact_sixteen_unit_viewbox() {
        for icon in [
            ReferencePaneIcon::SplitHorizontal,
            ReferencePaneIcon::SplitVertical,
            ReferencePaneIcon::Close,
        ] {
            let polygons = reference_icon_polygons(icon);
            assert_eq!(
                polygons.len(),
                if icon == ReferencePaneIcon::Close {
                    2
                } else {
                    5
                }
            );
            for &(x, y) in polygons.iter().flat_map(|polygon| polygon.iter()) {
                assert!((0.0..=16.0).contains(&x));
                assert!((0.0..=16.0).contains(&y));
            }
        }
        assert_eq!(
            reference_icon_polygons(ReferencePaneIcon::SplitHorizontal)[4],
            &[(7.5, 2.), (8.5, 2.), (8.5, 14.), (7.5, 14.)]
        );
        assert_eq!(
            reference_icon_polygons(ReferencePaneIcon::SplitVertical)[4],
            &[(2., 7.5), (14., 7.5), (14., 8.5), (2., 8.5)]
        );
    }

    #[test]
    fn chevron_rotation_states_match_each_edge() {
        for edge in [ActivityBarEdge::Left, ActivityBarEdge::Right] {
            assert_eq!(chevron_rotation(edge, false), 0);
            assert_eq!(chevron_rotation(edge, true), 90);
        }
        for edge in [ActivityBarEdge::Top, ActivityBarEdge::Bottom] {
            assert_eq!(chevron_rotation(edge, false), 0);
            assert_eq!(chevron_rotation(edge, true), 180);
        }
    }

    #[gpui::test]
    fn rendered_autohide_translation_reverses_through_the_exact_midpoint(cx: &mut TestAppContext) {
        let host = ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
            mode: ActivityBarMode::AutoHide,
            behavior: crate::ActivityBarBehavior {
                hover_expand: true,
                hover_intent: crate::ActivityBarHoverIntent { expand_delay_ms: 0 },
            },
            ..crate::ActivityBarConfig::default()
        });
        let (_, cx) = cx.add_window_view(move |_, cx| {
            MullionView::new(leaf("pane"), vec![], cx).with_activity_bar_host(host)
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();
        let pane = cx.debug_bounds("pane-visual:pane").unwrap();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(28.));
        assert_eq!(panel.right(), pane.left());
        let trigger = cx.debug_bounds("activity-bar-trigger:pane").unwrap();
        cx.simulate_mouse_move(trigger.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(93.));
        assert_eq!(panel.left(), pane.left() - px(46.5));
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity-bar-panel:pane").unwrap().left(),
            pane.left()
        );
        cx.simulate_mouse_move(pane.center(), None, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity-bar-panel:pane").unwrap().left(),
            pane.left()
        );
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(93.));
        assert_eq!(panel.left(), pane.left() - px(46.5));
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(28.));
        assert_eq!(panel.right(), pane.left());
    }

    #[gpui::test]
    fn rendered_state_motion_reaches_vertical_and_horizontal_midpoints_and_endpoints(
        cx: &mut TestAppContext,
    ) {
        let catalog = ActivityCatalog::new(vec![ActivityNode::Activity(rendered_activity(
            "activity",
            show_activity,
        ))]);
        let tree = PaneNode::leaf_with_activity(
            PaneId::new("pane"),
            ActivityId::new("activity"),
            "data".to_owned(),
        );
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_catalog(tree, catalog, cx).unwrap()
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        assert_eq!(panel.size.width, px(28.));
        cx.simulate_mouse_move(panel.center(), None, gpui::Modifiers::none());
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        let row = cx.debug_bounds("activity:pane:activity").unwrap();
        assert_eq!(panel.size.width, px(93.));
        assert_eq!(row.size.width, px(88.));
        assert_eq!(panel.size.width - row.size.width - px(1.), px(4.));
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        let panel = cx.debug_bounds("activity-bar-panel:pane").unwrap();
        let row = cx.debug_bounds("activity:pane:activity").unwrap();
        assert_eq!(panel.size.width, px(158.));
        assert_eq!(row.size.width, px(149.));
        assert_eq!(panel.size.width - row.size.width - px(1.), px(8.));

        view.update(cx, |view, cx| {
            view.host.activity_bar.edge = ActivityBarEdge::Top;
            view.hover.clear();
            view.bar_motion.clear();
            view.item_motion.clear();
            cx.notify();
        });
        cx.run_until_parked();
        let control = cx.debug_bounds("pane-control:split-h:pane").unwrap();
        assert_eq!(control.size.width, px(28.));
        cx.simulate_mouse_move(control.center(), None, gpui::Modifiers::none());
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("pane-control:split-h:pane")
                .unwrap()
                .size
                .width,
            px(89.)
        );
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("pane-control:split-h:pane")
                .unwrap()
                .size
                .width,
            px(150.)
        );
    }

    #[gpui::test]
    fn rendered_dock_drag_forces_all_horizontal_rows_then_restores_on_cancel(
        cx: &mut TestAppContext,
    ) {
        let catalog = ActivityCatalog::new(vec![ActivityNode::Activity(rendered_activity(
            "activity",
            show_activity,
        ))]);
        let tree = split(SplitDirection::Horizontal, 0.5, leaf("a"), leaf("b"));
        let host = ActivityBarHostConfig::new().with_activity_bar(crate::ActivityBarConfig {
            edge: ActivityBarEdge::Top,
            ..crate::ActivityBarConfig::default()
        });
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_catalog(tree, catalog, cx)
                .unwrap()
                .with_activity_bar_host(host)
        });
        cx.simulate_resize(gpui::size(px(500.), px(320.)));
        cx.run_until_parked();
        let start = cx.debug_bounds("pane-drag-handle:a").unwrap().center();
        let target = cx.debug_bounds("pane:b").unwrap().center();
        cx.simulate_mouse_down(start, MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(
            gpui::point(start.x + px(4.), start.y),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        cx.simulate_mouse_move(target, Some(MouseButton::Left), gpui::Modifiers::none());
        cx.run_until_parked();
        for selector in [
            "activity:a:activity",
            "activity:b:activity",
            "pane-control:split-h:a",
            "pane-control:split-v:b",
        ] {
            assert_eq!(
                cx.debug_bounds(selector).unwrap().size.width,
                px(150.),
                "{selector}"
            );
        }
        cx.simulate_mouse_move(start, Some(MouseButton::Left), gpui::Modifiers::none());
        cx.simulate_mouse_up(start, MouseButton::Left, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.model().tree().leaf_ids()),
            vec![PaneId::new("a"), PaneId::new("b")]
        );
        assert_eq!(
            cx.debug_bounds("activity:a:activity").unwrap().size.width,
            px(150.)
        );
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity:a:activity").unwrap().size.width,
            px(89.)
        );
        cx.executor().advance_clock(Duration::from_millis(75));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("activity:a:activity").unwrap().size.width,
            px(28.)
        );
    }
}
