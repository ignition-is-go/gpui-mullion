use crate::{
    Activity, ActivityCache, ActivityCacheKey, ActivityFactoryRegistry, ActivityId, ActivityNode,
    DropEdge, FocusPresentation, MullionModel, MullionSettings, MullionTheme, PaneData,
    PaneDirection, PaneEvent, PaneFocusBehavior, PaneId, PaneNode, SplitDirection,
    WorkspaceChanged, WorkspaceEvent, WorkspaceId, WorkspaceSet, WorkspaceSetError,
};
use gpui::{
    actions, div, prelude::*, px, relative, AnyElement, App, Bounds, Context, Element, ElementId,
    EventEmitter, FocusHandle, GlobalElementId, Hsla, InspectorElementId, LayoutId, MouseButton,
    Pixels, Point, SharedString, Window,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
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

const SPLIT_HIT_TARGET: f32 = 8.0;
const SPLIT_BAR_WIDTH: f32 = 1.0;
const KEYBOARD_RESIZE_STEP: f64 = 0.05;

#[derive(Clone, Copy, Default)]
struct InternalEdges {
    top: bool,
    right: bool,
    bottom: bool,
    left: bool,
}

type SplitBounds = Rc<RefCell<HashMap<PaneId, Bounds<Pixels>>>>;
type ActiveSplit = Rc<RefCell<Option<(PaneId, f64)>>>;

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

#[derive(Clone)]
struct PaneDrag {
    id: PaneId,
}
impl Render for PaneDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(gpui::blue().opacity(0.75))
            .text_color(gpui::white())
            .child(self.id.0.clone())
    }
}

/// Shared native/WebAssembly GPUI view over the portable model.
pub struct MullionView<D: PaneData> {
    model: MullionModel<D>,
    activities: Vec<ActivityNode<D>>,
    theme: MullionTheme,
    settings: MullionSettings,
    focus_presentation: FocusPresentation,
    activity_bar_width: gpui::Pixels,
    show_headers: bool,
    focus_handle: FocusHandle,
    workspaces: Option<WorkspaceSet<D>>,
    activity_factories: ActivityFactoryRegistry<D>,
    activity_cache: ActivityCache<D>,
    split_bounds: SplitBounds,
    split_starts: Rc<RefCell<HashMap<PaneId, Point<Pixels>>>>,
    active_split: ActiveSplit,
    keyboard_split: Option<PaneId>,
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
            activities,
            theme: MullionTheme::default(),
            settings: MullionSettings::default(),
            focus_presentation: FocusPresentation::default(),
            activity_bar_width: px(42.),
            show_headers: true,
            focus_handle: cx.focus_handle(),
            workspaces: None,
            activity_factories: ActivityFactoryRegistry::new(),
            activity_cache: ActivityCache::default(),
            split_bounds: Rc::default(),
            split_starts: Rc::default(),
            active_split: Rc::default(),
            keyboard_split: None,
        }
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
        self
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
        self.show_headers = visible;
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
        if events
            .iter()
            .any(|event| matches!(event, PaneEvent::TreeChanged { .. }))
        {
            if let Some(workspaces) = &mut self.workspaces {
                workspaces.persist_active(self.model.snapshot());
            }
        }
        for event in events {
            cx.emit(event)
        }
        cx.notify();
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
        let _ = self.execute(command, |_, _, _| None, cx);
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
        let Some((key, start_ratio)) = self.active_split.borrow_mut().take() else {
            return;
        };
        if cx.stop_active_drag(window) {
            self.model.resize(&key, start_ratio);
            self.finish(cx);
        }
    }
    fn all_activities(&self, data: &D) -> Vec<Activity<D>> {
        let mut out = Vec::new();
        for node in &self.activities {
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
        out: &mut HashMap<(Option<WorkspaceId>, PaneId), D>,
    ) {
        match node {
            PaneNode::Leaf { id, data, .. } => {
                out.insert((workspace, id.clone()), data.clone());
            }
            PaneNode::Split { first, second, .. } => {
                Self::collect_panes(first, workspace.clone(), out);
                Self::collect_panes(second, workspace, out);
            }
        }
    }
    fn sync_activity_cache(&mut self, window: &mut Window, cx: &mut App) {
        let mut pane_data = HashMap::new();
        if let Some(workspaces) = &self.workspaces {
            for workspace in &workspaces.workspaces {
                let tree = if workspace.id == workspaces.active {
                    self.model.tree()
                } else {
                    &workspace.tree
                };
                Self::collect_panes(tree, Some(workspace.id.clone()), &mut pane_data);
            }
        } else {
            Self::collect_panes(self.model.tree(), None, &mut pane_data);
        }
        let valid = pane_data
            .iter()
            .flat_map(|((workspace, pane), data)| {
                self.all_activities(data).into_iter().map(move |activity| {
                    ActivityCacheKey::new(workspace.clone(), pane.clone(), activity.id)
                })
            })
            .collect::<HashSet<_>>();
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
        edges: InternalEdges,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            PaneNode::Leaf {
                id,
                active_activity,
                data,
            } => self.render_leaf(id, active_activity.as_ref(), data, edges, window, cx),
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
                let first_el = self.render_node(first, first_edges, window, cx);
                let second_el = self.render_node(second, second_edges, window, cx);
                let handle_color = self.theme.border;
                let focused_color = self.theme.focused;
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
                        element.w(px(SPLIT_BAR_WIDTH)).h_full()
                    })
                    .when(*direction == SplitDirection::Vertical, |element| {
                        element.h(px(SPLIT_BAR_WIDTH)).w_full()
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
                            .aria_label("Resize panes")
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
                                    .left(px(-(SPLIT_HIT_TARGET - SPLIT_BAR_WIDTH) / 2.0))
                                    .w(px(SPLIT_HIT_TARGET))
                                    .h_full()
                                    .cursor_col_resize()
                            })
                            .when(*direction == SplitDirection::Vertical, |element| {
                                element
                                    .top(px(-(SPLIT_HIT_TARGET - SPLIT_BAR_WIDTH) / 2.0))
                                    .h(px(SPLIT_HIT_TARGET))
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
                            if extent > px(0.) {
                                this.model.resize(
                                    &drag.split_key,
                                    drag.start_ratio + f64::from(delta / extent),
                                );
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
    fn render_leaf(
        &mut self,
        id: &PaneId,
        active: Option<&ActivityId>,
        data: &D,
        edges: InternalEdges,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let focused = self.model.focused() == Some(id);
        let theme = self.theme;
        let id_focus_click = id.clone();
        let id_focus_hover = id.clone();
        let click_focus_handle = self.focus_handle.clone();
        let hover_focus_handle = self.focus_handle.clone();
        let id_drop = id.clone();
        let id_drag = id.clone();
        let id_close = id.clone();
        let activities = self.all_activities(data);
        let selected = active
            .and_then(|a| activities.iter().find(|x| &x.id == a).cloned())
            .or_else(|| activities.first().cloned());
        let tabs = activities
            .iter()
            .map(|activity| {
                let pane = id.clone();
                let activity_id = activity.id.clone();
                let is_active = selected.as_ref().is_some_and(|a| a.id == activity.id);
                div()
                    .id(SharedString::from(format!(
                        "activity:{}:{}",
                        id.0, activity.id.0
                    )))
                    .size(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if is_active {
                        theme.text
                    } else {
                        theme.muted_text
                    })
                    .bg(if is_active {
                        theme.accent
                    } else {
                        Hsla::transparent_black()
                    })
                    .hover(|e| e.bg(theme.accent))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.model.set_activity(&pane, Some(activity_id.clone()));
                        this.finish(cx)
                    }))
                    .child(activity.name.clone())
            })
            .collect::<Vec<_>>();
        let cached = selected.as_ref().and_then(|activity| {
            let factory = self.activity_factories.get(&activity.id)?.clone();
            let key =
                ActivityCacheKey::new(self.workspace_namespace(), id.clone(), activity.id.clone());
            if self.activity_cache.get(&key).is_none() {
                let instance = factory(id, data, window, cx);
                self.activity_cache
                    .insert(key.clone(), instance, data.clone());
            }
            self.activity_cache
                .get(&key)
                .map(|entry| (entry.instance.body.clone(), entry.instance.header.clone()))
        });
        let body = cached
            .as_ref()
            .map(|(body, _)| body.clone().into_any_element())
            .or_else(|| {
                selected
                    .as_ref()
                    .map(|activity| (activity.render)(id, data))
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
        let custom_header = cached.and_then(|(_, header)| header);
        let header = self.show_headers.then(|| {
            div()
                .h(px(30.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .border_b_1()
                .border_color(theme.border)
                .text_sm()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            selected
                                .as_ref()
                                .map(|a| a.name.clone())
                                .unwrap_or_else(|| SharedString::from("Pane")),
                        )
                        .when_some(custom_header, |header, custom| {
                            header.child(custom.into_any_element())
                        }),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("close:{}", id.0)))
                        .px_2()
                        .cursor_pointer()
                        .hover(|e| e.bg(theme.accent))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.model.close(&id_close);
                            this.finish(cx)
                        }))
                        .child("×"),
                )
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
                    .bg(theme.focused)
            };
            if edges.top {
                focus_edges.push(edge("top").top_0().left_0().right_0().h(px(1.)));
            }
            if edges.right {
                focus_edges.push(edge("right").top_0().right_0().bottom_0().w(px(1.)));
            }
            if edges.bottom {
                focus_edges.push(edge("bottom").bottom_0().left_0().right_0().h(px(1.)));
            }
            if edges.left {
                focus_edges.push(edge("left").top_0().bottom_0().left_0().w(px(1.)));
            }
        }
        div()
            .id(SharedString::from(format!("pane:{}", id.0)))
            .debug_selector({
                let id = id.clone();
                move || format!("pane:{}", id.0)
            })
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .bg(theme.surface)
            .text_color(theme.text)
            .border_1()
            .border_color(theme.border)
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
            .on_drag(PaneDrag { id: id_drag }, |drag, _, _, cx| {
                cx.new(|_| drag.clone())
            })
            .on_drop(cx.listener(move |this, drag: &PaneDrag, _, cx| {
                if drag.id != id_drop {
                    this.model.move_pane(&drag.id, &id_drop, DropEdge::Center);
                    this.finish(cx)
                }
            }))
            .child(
                div()
                    .id(SharedString::from(format!("pane-visual:{}", id.0)))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("pane-visual:{}", id.0)
                    })
                    .size_full()
                    .flex()
                    .opacity(unfocused_opacity)
                    .child(
                        div()
                            .w(self.activity_bar_width)
                            .h_full()
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .py_1()
                            .border_r_1()
                            .border_color(theme.border)
                            .children(tabs),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .when_some(header, |e, h| e.child(h))
                            .child(div().flex_1().min_h_0().overflow_hidden().child(body)),
                    ),
            )
            .children(focus_edges)
            .into_any_element()
    }
}

impl<D: PaneData> Render for MullionView<D> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_activity_cache(window, cx);
        let tree = self
            .model
            .zoomed()
            .and_then(|id| self.model.tree().find(id))
            .unwrap_or(self.model.tree())
            .clone();
        let workspace_tabs = self.workspaces.as_ref().map(|set| {
            let active = set.active.clone();
            set.workspaces
                .iter()
                .map(|workspace| {
                    let id = workspace.id.clone();
                    let selected = id == active;
                    div()
                        .id(SharedString::from(format!("workspace:{}", id.0)))
                        .px_3()
                        .py_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(if selected {
                            self.theme.text
                        } else {
                            self.theme.muted_text
                        })
                        .bg(if selected {
                            self.theme.accent
                        } else {
                            Hsla::transparent_black()
                        })
                        .hover(|element| element.bg(self.theme.accent))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.switch_workspace(&id, cx);
                        }))
                        .child(workspace.name.clone())
                })
                .collect::<Vec<_>>()
        });
        div()
            .key_context("Mullion")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(self.theme.background)
            .on_action(cx.listener(|this, _: &FocusLeft, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusRight, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusUp, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusDown, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Down), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusNext, _, cx| {
                this.command(crate::PaneCommand::FocusNext, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusPrevious, _, cx| {
                this.command(crate::PaneCommand::FocusPrevious, cx)
            }))
            .on_action(
                cx.listener(|this, _: &ClosePane, _, cx| {
                    this.command(crate::PaneCommand::Close, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleZoom, _, cx| {
                this.command(crate::PaneCommand::ToggleZoom, cx)
            }))
            .on_action(cx.listener(|this, _: &BalancePanes, _, cx| {
                this.command(crate::PaneCommand::Balance, cx)
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
                        .h(px(36.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .border_b_1()
                        .border_color(self.theme.border)
                        .children(tabs),
                )
            })
            .child(div().flex_1().min_w_0().min_h_0().child(self.render_node(
                &tree,
                InternalEdges::default(),
                window,
                cx,
            )))
    }
}

/// Register the default, direct native key bindings. Hosts may override them later.
pub fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("alt-left", FocusLeft, None),
        gpui::KeyBinding::new("alt-right", FocusRight, None),
        gpui::KeyBinding::new("alt-up", FocusUp, None),
        gpui::KeyBinding::new("alt-down", FocusDown, None),
        gpui::KeyBinding::new("alt-pagedown", FocusNext, None),
        gpui::KeyBinding::new("alt-pageup", FocusPrevious, None),
        gpui::KeyBinding::new("ctrl-shift-backspace", ClosePane, None),
        gpui::KeyBinding::new("ctrl-shift-enter", ToggleZoom, None),
        gpui::KeyBinding::new("ctrl-alt-=", BalancePanes, None),
        gpui::KeyBinding::new("ctrl-alt-[", ResizeSplitDecrease, Some("Mullion")),
        gpui::KeyBinding::new("ctrl-alt-]", ResizeSplitIncrease, Some("Mullion")),
        gpui::KeyBinding::new("escape", CancelSplitResize, Some("Mullion")),
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
            atomic::{AtomicU8, Ordering},
            Arc,
        },
    };

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
        assert_eq!(bar.size.width, px(1.));
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
}
