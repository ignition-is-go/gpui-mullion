use crate::tree::{collect_split_ratios, directional_neighbor, find_ratio, resize_boundary};
use crate::{
    ActivityId, DropEdge, PaneCommand, PaneCommandError, PaneCommandExecutionOptions,
    PaneCommandResult, PaneData, PaneDirection, PaneEvent, PaneId, PaneLayout, PaneNode,
    PaneRotation, PaneValidationError, SplitDirection,
};
use std::rc::Rc;

type MutablePaneSplitFactory<'a, D> =
    dyn FnMut(&PaneId, SplitDirection, &D) -> Option<(PaneId, D)> + 'a;

/// Toolkit-independent state machine. UI adapters translate input into these methods.
pub struct MullionModel<D: PaneData> {
    tree: Rc<PaneNode<D>>,
    focused: Option<PaneId>,
    zoomed: Option<PaneId>,
    events: Vec<PaneEvent<D>>,
}

impl<D: PaneData> MullionModel<D> {
    /// Construct a model after validating layout invariants.
    pub fn try_new(tree: PaneNode<D>) -> Result<Self, PaneValidationError> {
        tree.validate()?;
        let focused = tree.leaf_ids().into_iter().next();
        Ok(Self {
            tree: Rc::new(tree),
            focused,
            zoomed: None,
            events: Vec::new(),
        })
    }

    /// Construct a model from a layout already trusted by the caller.
    ///
    /// Invalid persisted input must use [`Self::try_new`] so it can report a
    /// typed error rather than panic.
    pub fn new(tree: PaneNode<D>) -> Self {
        Self::try_new(tree).expect("MullionModel::new requires a valid pane tree")
    }
    /// Borrow the current validated pane tree.
    pub fn tree(&self) -> &PaneNode<D> {
        self.tree.as_ref()
    }
    /// Clone the complete tree for persistence or event transport.
    pub fn snapshot(&self) -> PaneNode<D> {
        self.tree.as_ref().clone()
    }
    /// Share the immutable tree for render projection without cloning pane data.
    pub(crate) fn shared_tree(&self) -> Rc<PaneNode<D>> {
        self.tree.clone()
    }
    fn tree_mut(&mut self) -> &mut PaneNode<D> {
        Rc::make_mut(&mut self.tree)
    }
    /// Return the pane that owns model focus.
    pub fn focused(&self) -> Option<&PaneId> {
        self.focused.as_ref()
    }
    /// Return the pane occupying the zoom viewport, if zoom is enabled.
    pub fn zoomed(&self) -> Option<&PaneId> {
        self.zoomed.as_ref()
    }
    /// Drain pending events in mutation order.
    pub fn take_events(&mut self) -> Vec<PaneEvent<D>> {
        std::mem::take(&mut self.events)
    }
    fn changed(&mut self) {
        self.events.push(PaneEvent::TreeChanged {
            tree: self.tree.as_ref().clone(),
        });
    }
    fn try_replace_tree_inner(
        &mut self,
        tree: PaneNode<D>,
        emit_tree_changed: bool,
    ) -> Result<(), PaneValidationError> {
        tree.validate()?;
        let previous_focus = self.focused.clone();
        let previous_zoom = self.zoomed.clone();
        self.tree = Rc::new(tree);

        if self
            .zoomed
            .as_ref()
            .is_some_and(|pane| !self.tree.contains(pane))
        {
            self.zoomed = None;
        }
        if let Some(zoomed) = self.zoomed.clone() {
            // Zoom is a view of the focused pane, so a surviving zoom takes
            // precedence when a replacement invalidates the old focus.
            self.focused = Some(zoomed);
        } else if self
            .focused
            .as_ref()
            .is_none_or(|pane| !self.tree.contains(pane))
        {
            self.focused = self.tree.leaf_ids().into_iter().next();
        }

        if self.focused != previous_focus {
            self.events.push(PaneEvent::FocusChanged {
                pane: self.focused.clone(),
            });
        }
        if self.zoomed != previous_zoom {
            self.events.push(PaneEvent::ZoomChanged {
                pane: self.zoomed.clone(),
            });
        }
        if emit_tree_changed {
            self.changed();
        }
        Ok(())
    }

    /// Apply a validated upstream tree without echoing `TreeChanged`.
    ///
    /// Focus and zoom are reconciled and their transient events are still
    /// emitted. Use this path for snapshots received from persistence or a
    /// remote owner, where echoing the snapshot could create a feedback loop.
    pub fn try_set_tree(&mut self, tree: PaneNode<D>) -> Result<(), PaneValidationError> {
        self.try_replace_tree_inner(tree, false)
    }

    /// Apply an upstream tree trusted by the caller without echoing it.
    pub fn set_tree(&mut self, tree: PaneNode<D>) {
        self.try_set_tree(tree)
            .expect("MullionModel::set_tree requires a valid pane tree");
    }

    /// Replace the tree as a local mutation, emitting `TreeChanged`.
    pub fn try_replace_tree(&mut self, tree: PaneNode<D>) -> Result<(), PaneValidationError> {
        self.try_replace_tree_inner(tree, true)
    }

    /// Replace the tree as a trusted local mutation, emitting `TreeChanged`.
    pub fn replace_tree(&mut self, tree: PaneNode<D>) {
        self.try_replace_tree(tree)
            .expect("MullionModel::replace_tree requires a valid pane tree");
    }

    /// Focus an existing pane, returning `false` when the id is unknown.
    pub fn focus(&mut self, pane: &PaneId) -> bool {
        if !self.tree.contains(pane) {
            return false;
        }
        if self.focused.as_ref() != Some(pane) {
            self.focused = Some(pane.clone());
            self.events.push(PaneEvent::FocusChanged {
                pane: self.focused.clone(),
            });
            // Coherent navigation semantics: navigating while zoomed keeps
            // zoom enabled and moves the zoom viewport to the new focus.
            if self.zoomed.is_some() {
                self.zoomed = Some(pane.clone());
                self.events.push(PaneEvent::ZoomChanged {
                    pane: self.zoomed.clone(),
                });
            }
        }
        true
    }
    /// Focus the visually nearest pane in a screen-space direction.
    pub fn focus_neighbor(&mut self, direction: PaneDirection) -> bool {
        let Some(current) = self.focused.clone() else {
            return false;
        };
        let next = directional_neighbor(&self.tree, &current, direction, |key| {
            find_ratio(&self.tree, key).unwrap_or(0.5)
        });
        next.is_some_and(|pane| self.focus(&pane))
    }
    /// Focus a leaf by depth-first traversal index.
    pub fn focus_index(&mut self, index: usize) -> bool {
        self.tree
            .leaf_ids()
            .get(index)
            .cloned()
            .is_some_and(|p| self.focus(&p))
    }
    /// Move focus cyclically by a signed depth-first traversal offset.
    pub fn cycle_focus(&mut self, delta: isize) -> bool {
        let ids = self.tree.leaf_ids();
        if ids.is_empty() {
            return false;
        }
        let at = self
            .focused
            .as_ref()
            .and_then(|p| ids.iter().position(|id| id == p))
            .unwrap_or(0) as isize;
        let next = (at + delta).rem_euclid(ids.len() as isize) as usize;
        self.focus(&ids[next])
    }
    /// Insert a host-identified pane beside `target` and focus it.
    ///
    /// Returns `false` for an unknown target, duplicate id, or refused tree mutation.
    pub fn split(
        &mut self,
        target: &PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        data: D,
    ) -> bool {
        if self.tree.contains(&new_id)
            || !self
                .tree_mut()
                .split(target, direction, new_id.clone(), data.clone())
        {
            return false;
        }
        self.events.push(PaneEvent::Split {
            target: target.clone(),
            direction,
            new_id: new_id.clone(),
            new_data: data,
        });
        self.focus(&new_id);
        self.changed();
        true
    }
    /// Remove a pane and return its data; the final pane cannot be closed.
    pub fn close(&mut self, pane: &PaneId) -> Option<D> {
        if self.tree.leaf_ids().len() <= 1 {
            return None;
        }
        let ids = self.tree.leaf_ids();
        let at = ids.iter().position(|id| id == pane)?;
        // Match Mullion's traversal semantics: prefer the pane that followed
        // the closed pane, falling back to its predecessor only at the end.
        let preferred_focus = ids
            .get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|previous| ids.get(previous)))
            .cloned();
        let data = self.tree_mut().close(pane)?;
        self.events.push(PaneEvent::Closed {
            id: pane.clone(),
            data: data.clone(),
        });
        // Closing a zoomed pane exits zoom before choosing replacement focus;
        // otherwise `focus` would coherently migrate zoom to that replacement.
        if self.zoomed.as_ref() == Some(pane) {
            self.zoomed = None;
            self.events.push(PaneEvent::ZoomChanged { pane: None });
        }
        if self.focused.as_ref() == Some(pane) {
            // A successful close always has a successor because the final pane
            // cannot be closed and `preferred_focus` was chosen beforehand.
            if let Some(next) = preferred_focus {
                self.focus(&next);
            }
        }
        self.changed();
        Some(data)
    }
    /// Store a finite first-child split fraction, clamped to `0.1..=0.9`.
    pub fn resize(&mut self, split_key: &PaneId, ratio: f64) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        if !self.tree_mut().set_split_ratio(split_key, ratio) {
            return false;
        }
        // Report the ratio actually persisted by PaneNode (including its
        // public 0.1..=0.9 clamp), never merely the requested value.
        let ratio = find_ratio(&self.tree, split_key)
            .expect("a successfully resized split remains addressable");
        self.events.push(PaneEvent::Resized {
            split_key: split_key.clone(),
            ratio,
        });
        self.changed();
        true
    }
    /// Resize the nearest ancestor boundary of the focused pane.
    pub fn resize_focused(&mut self, direction: PaneDirection, amount: f64) -> bool {
        let Some(pane) = self.focused.clone() else {
            return false;
        };
        let Some((key, sign)) = resize_boundary(&self.tree, &pane, direction) else {
            return false;
        };
        let old = find_ratio(&self.tree, &key).unwrap_or(0.5);
        self.resize(&key, old + amount.abs() * sign)
    }
    /// Move a pane to an edge of another pane, preserving its complete leaf state.
    pub fn move_pane(&mut self, source: &PaneId, destination: &PaneId, edge: DropEdge) -> bool {
        if !self.tree_mut().move_pane(source, destination, edge) {
            return false;
        }
        self.events.push(PaneEvent::Moved {
            source: source.clone(),
            destination: destination.clone(),
            edge,
        });
        self.changed();
        true
    }
    /// Insert a pane minted by the host for a dropped activity.
    ///
    /// The host owns id/data creation. On success this emits
    /// `ActivityDropped`, `TreeChanged`, then the transient `FocusChanged`.
    pub fn drop_activity(
        &mut self,
        activity: &ActivityId,
        destination: &PaneId,
        edge: DropEdge,
        new_id: PaneId,
        new_data: D,
    ) -> bool {
        if self.tree.contains(&new_id)
            || !self.tree_mut().insert_leaf(
                destination,
                edge,
                new_id.clone(),
                new_data.clone(),
                Some(activity.clone()),
            )
        {
            return false;
        }
        self.events.push(PaneEvent::ActivityDropped {
            activity: activity.clone(),
            destination: destination.clone(),
            edge,
            new_id: new_id.clone(),
            new_data,
        });
        self.changed();
        self.focus(&new_id);
        true
    }

    /// Swap two complete leaves without changing split topology.
    pub fn swap(&mut self, a: &PaneId, b: &PaneId) -> bool {
        if !self.tree_mut().swap_panes(a, b) {
            false
        } else {
            self.changed();
            true
        }
    }
    /// Change the nearest parent split axis identified by `pane`.
    pub fn set_direction(&mut self, pane: &PaneId, direction: SplitDirection) -> bool {
        if !self.tree_mut().change_direction(pane, direction) {
            return false;
        }
        self.events.push(PaneEvent::DirectionChanged {
            pane: pane.clone(),
            direction,
        });
        self.changed();
        true
    }
    /// Reset every non-balanced split to an even ratio.
    pub fn balance(&mut self) -> bool {
        if self.tree_mut().balance_splits() == 0 {
            return false;
        }
        let mut splits = Vec::new();
        collect_split_ratios(&self.tree, &mut splits);
        self.events.extend(
            splits
                .into_iter()
                .map(|(split_key, ratio)| PaneEvent::Resized { split_key, ratio }),
        );
        self.changed();
        true
    }
    /// Rotate leaves through existing layout slots.
    pub fn rotate(&mut self, rotation: PaneRotation) -> bool {
        if !self.tree_mut().rotate_panes(rotation) {
            false
        } else {
            self.changed();
            true
        }
    }
    /// Rebuild split topology using a standard whole-tree layout.
    pub fn apply_layout(&mut self, layout: PaneLayout) -> bool {
        let focus = self.focused.clone();
        if !self.tree_mut().apply_layout(layout, focus.as_ref()) {
            false
        } else {
            self.changed();
            true
        }
    }
    /// Replace a pane's selected activity.
    pub fn set_activity(&mut self, pane: &PaneId, activity: Option<ActivityId>) -> bool {
        let Some(PaneNode::Leaf {
            active_activity, ..
        }) = self.tree_mut().find_mut(pane)
        else {
            return false;
        };
        *active_activity = activity.clone();
        self.events.push(PaneEvent::ActivityChanged {
            pane: pane.clone(),
            activity,
        });
        self.changed();
        true
    }
    /// Replace pane data when it differs by `PartialEq`.
    pub fn update_data(&mut self, pane: &PaneId, data: D) -> bool {
        let Some(PaneNode::Leaf { data: old, .. }) = self.tree_mut().find_mut(pane) else {
            return false;
        };
        *old = data.clone();
        self.events.push(PaneEvent::DataChanged {
            pane: pane.clone(),
            data,
        });
        self.changed();
        true
    }
    /// Toggle zoom for the focused pane.
    pub fn toggle_zoom(&mut self) -> bool {
        let Some(focus) = self.focused.clone() else {
            return false;
        };
        self.zoomed = if self.zoomed.as_ref() == Some(&focus) {
            None
        } else {
            Some(focus)
        };
        self.events.push(PaneEvent::ZoomChanged {
            pane: self.zoomed.clone(),
        });
        true
    }

    /// Execute a portable command using `split_factory` only for split commands.
    pub fn execute<F>(&mut self, command: PaneCommand, mut split_factory: F) -> PaneCommandResult
    where
        F: FnMut(&PaneId, SplitDirection, &D) -> Option<(PaneId, D)>,
    {
        self.execute_inner(command, Some(&mut split_factory), 0.05)
    }

    /// Execute with reusable host split and resize configuration.
    pub fn execute_with_options(
        &mut self,
        command: PaneCommand,
        options: &PaneCommandExecutionOptions<D>,
    ) -> PaneCommandResult {
        match options.split_factory.as_ref() {
            Some(factory) => {
                let mut split_factory =
                    |id: &PaneId, direction, data: &D| factory(id, direction, data);
                self.execute_inner(command, Some(&mut split_factory), options.resize_step)
            }
            None => self.execute_inner(command, None, options.resize_step),
        }
    }

    fn execute_inner(
        &mut self,
        command: PaneCommand,
        mut split_factory: Option<&mut MutablePaneSplitFactory<'_, D>>,
        resize_step: f64,
    ) -> PaneCommandResult {
        use PaneCommand::*;

        match command {
            Focus(direction) => {
                self.focused
                    .as_ref()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.focus_neighbor(direction)
                    .then_some(())
                    .ok_or(PaneCommandError::NoNeighbor)
            }
            FocusNext => {
                self.focused
                    .as_ref()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.cycle_focus(1)
                    .then_some(())
                    .ok_or(PaneCommandError::NoFocusedPane)
            }
            FocusPrevious => {
                self.focused
                    .as_ref()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.cycle_focus(-1)
                    .then_some(())
                    .ok_or(PaneCommandError::NoFocusedPane)
            }
            FocusFirst => self
                .focus_index(0)
                .then_some(())
                .ok_or(PaneCommandError::InvalidPaneIndex),
            FocusLast => self
                .tree
                .leaf_ids()
                .len()
                .checked_sub(1)
                .ok_or(PaneCommandError::NoFocusedPane)
                .and_then(|index| {
                    self.focus_index(index)
                        .then_some(())
                        .ok_or(PaneCommandError::InvalidPaneIndex)
                }),
            FocusIndex(index) => self
                .focus_index(index)
                .then_some(())
                .ok_or(PaneCommandError::InvalidPaneIndex),
            Split(direction) => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let data = match self.tree.find(&focused) {
                    Some(PaneNode::Leaf { data, .. }) => data.clone(),
                    _ => return Err(PaneCommandError::NoFocusedPane),
                };
                let factory = split_factory
                    .as_mut()
                    .ok_or(PaneCommandError::SplitUnavailable)?;
                let (new_id, new_data) =
                    factory(&focused, direction, &data).ok_or(PaneCommandError::SplitRefused)?;
                self.split(&focused, direction, new_id, new_data)
                    .then_some(())
                    .ok_or(PaneCommandError::SplitRefused)
            }
            Close => {
                if self.tree.leaf_ids().len() <= 1 {
                    return Err(PaneCommandError::CannotCloseLastPane);
                }
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.close(&focused)
                    .map(|_| ())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            Move(direction) => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let neighbor = directional_neighbor(&self.tree, &focused, direction, |key| {
                    find_ratio(&self.tree, key).unwrap_or(0.5)
                })
                .ok_or(PaneCommandError::NoNeighbor)?;
                self.move_pane(&focused, &neighbor, direction.drop_edge())
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            Swap(direction) => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let neighbor = directional_neighbor(&self.tree, &focused, direction, |key| {
                    find_ratio(&self.tree, key).unwrap_or(0.5)
                })
                .ok_or(PaneCommandError::NoNeighbor)?;
                self.swap(&focused, &neighbor)
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            SwapNext | SwapPrevious => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let ids = self.tree.leaf_ids();
                if ids.len() < 2 {
                    return Err(PaneCommandError::NoNeighbor);
                }
                let at = ids
                    .iter()
                    .position(|pane| pane == &focused)
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let neighbor = if matches!(command, SwapNext) {
                    (at + 1) % ids.len()
                } else {
                    (at + ids.len() - 1) % ids.len()
                };
                self.swap(&focused, &ids[neighbor])
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            Resize(direction) => {
                self.focused
                    .as_ref()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.resize_focused(direction, resize_step)
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            SetParentSplitDirection(direction) => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.set_direction(&focused, direction)
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            ToggleParentSplitDirection => {
                let focused = self
                    .focused
                    .clone()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                let direction = self
                    .tree
                    .parent_split_direction(&focused)
                    .ok_or(PaneCommandError::NotApplicable)?;
                let toggled = match direction {
                    SplitDirection::Horizontal => SplitDirection::Vertical,
                    SplitDirection::Vertical => SplitDirection::Horizontal,
                };
                self.set_direction(&focused, toggled)
                    .then_some(())
                    .ok_or(PaneCommandError::NotApplicable)
            }
            Balance => self
                .balance()
                .then_some(())
                .ok_or(PaneCommandError::NotApplicable),
            Rotate(rotation) => self
                .rotate(rotation)
                .then_some(())
                .ok_or(PaneCommandError::NotApplicable),
            ApplyLayout(layout) => self
                .apply_layout(layout)
                .then_some(())
                .ok_or(PaneCommandError::NotApplicable),
            ToggleZoom => {
                self.focused
                    .as_ref()
                    .ok_or(PaneCommandError::NoFocusedPane)?;
                self.toggle_zoom()
                    .then_some(())
                    .ok_or(PaneCommandError::NoFocusedPane)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
    struct D(u8);
    fn model() -> MullionModel<D> {
        MullionModel::new(PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
        })
    }
    #[test]
    fn mutations_emit_specific_then_snapshot() {
        let mut m = model();
        assert!(m.resize(&PaneId::new("b"), 0.7));
        let e = m.take_events();
        assert!(matches!(e[0],PaneEvent::Resized{ratio,..} if ratio==0.7));
        assert!(matches!(e[1], PaneEvent::TreeChanged { .. }));
    }
    #[test]
    fn close_preserves_valid_focus() {
        let mut m = model();
        m.focus(&PaneId::new("b"));
        m.take_events();
        assert_eq!(m.close(&PaneId::new("b")), Some(D(2)));
        assert_eq!(m.focused(), Some(&PaneId::new("a")));
    }
    #[test]
    fn replace_tree_emits_focus_and_zoom_changes_only_when_they_change() {
        let mut m = model();
        m.focus(&PaneId::new("b"));
        m.toggle_zoom();
        m.take_events();

        m.replace_tree(PaneNode::leaf(PaneId::new("c"), D(3)));
        assert_eq!(m.focused(), Some(&PaneId::new("c")));
        assert_eq!(m.zoomed(), None);
        let events = m.take_events();
        assert!(matches!(events.as_slice(), [
            PaneEvent::FocusChanged { pane: Some(p) },
            PaneEvent::ZoomChanged { pane: None },
            PaneEvent::TreeChanged { .. }
        ] if p == &PaneId::new("c")));

        m.replace_tree(PaneNode::leaf(PaneId::new("c"), D(4)));
        assert!(matches!(
            m.take_events().as_slice(),
            [PaneEvent::TreeChanged { .. }]
        ));
    }

    #[test]
    fn close_emits_complete_focus_and_zoom_changes() {
        let mut m = model();
        m.focus(&PaneId::new("b"));
        m.toggle_zoom();
        m.take_events();

        assert_eq!(m.close(&PaneId::new("b")), Some(D(2)));
        let events = m.take_events();
        assert!(matches!(events.as_slice(), [
            PaneEvent::Closed { id, .. },
            PaneEvent::ZoomChanged { pane: None },
            PaneEvent::FocusChanged { pane: Some(focus) },
            PaneEvent::TreeChanged { .. }
        ] if id == &PaneId::new("b") && focus == &PaneId::new("a")));
    }

    #[test]
    fn focus_navigation_moves_zoom_to_the_new_focus() {
        let mut m = model();
        m.toggle_zoom();
        m.take_events();

        assert!(m.cycle_focus(1));
        assert_eq!(m.focused(), Some(&PaneId::new("b")));
        assert_eq!(m.zoomed(), m.focused());
        assert!(matches!(m.take_events().as_slice(), [
            PaneEvent::FocusChanged { pane: Some(focus) },
            PaneEvent::ZoomChanged { pane: Some(zoom) }
        ] if focus == zoom && focus == &PaneId::new("b")));
    }

    #[test]
    fn resize_event_reports_the_stored_clamped_ratio() {
        let mut m = model();
        let key = PaneId::new("b");
        assert!(m.resize(&key, 0.01));
        let events = m.take_events();
        assert!(matches!(events.as_slice(), [
            PaneEvent::Resized { split_key, ratio },
            PaneEvent::TreeChanged { .. },
        ] if split_key == &key && *ratio == 0.1));
        assert_eq!(find_ratio(m.tree(), &key), Some(0.1));

        // Both the stored ratio and any request which clamps back to it are no-ops.
        assert!(!m.resize(&key, 0.1));
        assert!(!m.resize(&key, -100.0));
        assert!(m.take_events().is_empty());
    }
    #[test]
    fn try_new_rejects_invalid_persisted_layouts() {
        let invalid = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: f64::NAN,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
        };
        assert!(matches!(
            MullionModel::try_new(invalid),
            Err(PaneValidationError::NonFiniteSplitRatio { .. })
        ));
    }

    #[test]
    fn failed_validated_replacement_preserves_model_state_and_events() {
        let mut model = model();
        let original = model.snapshot();
        let duplicate = PaneNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("duplicate"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("duplicate"), D(2))),
        };

        assert!(matches!(
            model.try_replace_tree(duplicate),
            Err(PaneValidationError::DuplicatePaneId { .. })
        ));
        assert_eq!(model.snapshot(), original);
        assert!(model.take_events().is_empty());
    }

    #[test]
    fn command_split_uses_host_factory() {
        let mut m = model();
        m.execute(PaneCommand::Split(SplitDirection::Vertical), |_, _, _| {
            Some((PaneId::new("c"), D(3)))
        })
        .unwrap();
        assert!(m.tree().contains(&PaneId::new("c")));
    }
}
