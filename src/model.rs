use crate::tree::{directional_neighbor, find_ratio, resize_boundary};
use crate::{
    ActivityId, DropEdge, PaneCommand, PaneCommandError, PaneCommandResult, PaneData,
    PaneDirection, PaneEvent, PaneId, PaneLayout, PaneNode, PaneRotation, SplitDirection,
};

/// Toolkit-independent state machine. UI adapters translate input into these methods.
pub struct MullionModel<D: PaneData> {
    tree: PaneNode<D>,
    focused: Option<PaneId>,
    zoomed: Option<PaneId>,
    events: Vec<PaneEvent<D>>,
}

impl<D: PaneData> MullionModel<D> {
    pub fn new(tree: PaneNode<D>) -> Self {
        let focused = tree.leaf_ids().into_iter().next();
        Self {
            tree,
            focused,
            zoomed: None,
            events: Vec::new(),
        }
    }
    pub fn tree(&self) -> &PaneNode<D> {
        &self.tree
    }
    pub fn snapshot(&self) -> PaneNode<D> {
        self.tree.clone()
    }
    pub fn focused(&self) -> Option<&PaneId> {
        self.focused.as_ref()
    }
    pub fn zoomed(&self) -> Option<&PaneId> {
        self.zoomed.as_ref()
    }
    pub fn take_events(&mut self) -> Vec<PaneEvent<D>> {
        std::mem::take(&mut self.events)
    }
    fn changed(&mut self) {
        self.events.push(PaneEvent::TreeChanged {
            tree: self.tree.clone(),
        });
    }
    pub fn replace_tree(&mut self, tree: PaneNode<D>) {
        self.tree = tree;
        if self.focused.as_ref().is_none_or(|p| !self.tree.contains(p)) {
            self.focused = self.tree.leaf_ids().into_iter().next();
        }
        if self.zoomed.as_ref().is_some_and(|p| !self.tree.contains(p)) {
            self.zoomed = None;
        }
        self.changed();
    }
    pub fn focus(&mut self, pane: &PaneId) -> bool {
        if !self.tree.contains(pane) {
            return false;
        }
        if self.focused.as_ref() != Some(pane) {
            self.focused = Some(pane.clone());
            self.events.push(PaneEvent::FocusChanged {
                pane: self.focused.clone(),
            });
        }
        true
    }
    pub fn focus_neighbor(&mut self, direction: PaneDirection) -> bool {
        let Some(current) = self.focused.clone() else {
            return false;
        };
        let next = directional_neighbor(&self.tree, &current, direction, |key| {
            find_ratio(&self.tree, key).unwrap_or(0.5)
        });
        next.is_some_and(|pane| self.focus(&pane))
    }
    pub fn focus_index(&mut self, index: usize) -> bool {
        self.tree
            .leaf_ids()
            .get(index)
            .cloned()
            .is_some_and(|p| self.focus(&p))
    }
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
    pub fn split(
        &mut self,
        target: &PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        data: D,
    ) -> bool {
        if self.tree.contains(&new_id)
            || !self
                .tree
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
    pub fn close(&mut self, pane: &PaneId) -> Option<D> {
        if self.tree.leaf_ids().len() <= 1 {
            return None;
        }
        let ids = self.tree.leaf_ids();
        let at = ids.iter().position(|id| id == pane)?;
        let data = self.tree.close(pane)?;
        self.events.push(PaneEvent::Closed {
            id: pane.clone(),
            data: data.clone(),
        });
        if self.focused.as_ref() == Some(pane) {
            let left = at.saturating_sub(1).min(self.tree.leaf_ids().len() - 1);
            let next = self.tree.leaf_ids()[left].clone();
            self.focus(&next);
        }
        if self.zoomed.as_ref() == Some(pane) {
            self.zoomed = None;
        }
        self.changed();
        Some(data)
    }
    pub fn resize(&mut self, split_key: &PaneId, ratio: f64) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        let ratio = ratio.clamp(0.05, 0.95);
        if !self.tree.set_split_ratio(split_key, ratio) {
            return false;
        }
        self.events.push(PaneEvent::Resized {
            split_key: split_key.clone(),
            ratio,
        });
        self.changed();
        true
    }
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
    pub fn move_pane(&mut self, source: &PaneId, destination: &PaneId, edge: DropEdge) -> bool {
        if !self.tree.move_pane(source, destination, edge) {
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
    pub fn swap(&mut self, a: &PaneId, b: &PaneId) -> bool {
        if !self.tree.swap_panes(a, b) {
            false
        } else {
            self.changed();
            true
        }
    }
    pub fn set_direction(&mut self, pane: &PaneId, direction: SplitDirection) -> bool {
        if !self.tree.change_direction(pane, direction) {
            return false;
        }
        self.events.push(PaneEvent::DirectionChanged {
            pane: pane.clone(),
            direction,
        });
        self.changed();
        true
    }
    pub fn balance(&mut self) -> bool {
        if self.tree.balance_splits() == 0 {
            false
        } else {
            self.changed();
            true
        }
    }
    pub fn rotate(&mut self, rotation: PaneRotation) -> bool {
        if !self.tree.rotate_panes(rotation) {
            false
        } else {
            self.changed();
            true
        }
    }
    pub fn apply_layout(&mut self, layout: PaneLayout) -> bool {
        let focus = self.focused.clone();
        if !self.tree.apply_layout(layout, focus.as_ref()) {
            false
        } else {
            self.changed();
            true
        }
    }
    pub fn set_activity(&mut self, pane: &PaneId, activity: Option<ActivityId>) -> bool {
        let Some(PaneNode::Leaf {
            active_activity, ..
        }) = self.tree.find_mut(pane)
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
    pub fn update_data(&mut self, pane: &PaneId, data: D) -> bool {
        let Some(PaneNode::Leaf { data: old, .. }) = self.tree.find_mut(pane) else {
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

    pub fn execute<F>(&mut self, command: PaneCommand, mut split_factory: F) -> PaneCommandResult
    where
        F: FnMut(&PaneId, SplitDirection, &D) -> Option<(PaneId, D)>,
    {
        use PaneCommand::*;
        let focused = || self.focused.clone().ok_or(PaneCommandError::NoFocusedPane);
        let ok = match command {
            Focus(d) => self.focus_neighbor(d),
            FocusNext => self.cycle_focus(1),
            FocusPrevious => self.cycle_focus(-1),
            FocusFirst => self.focus_index(0),
            FocusLast => self
                .tree
                .leaf_ids()
                .len()
                .checked_sub(1)
                .is_some_and(|i| self.focus_index(i)),
            FocusIndex(i) => self.focus_index(i),
            Split(d) => {
                let p = focused()?;
                let data = match self.tree.find(&p) {
                    Some(PaneNode::Leaf { data, .. }) => data.clone(),
                    _ => return Err(PaneCommandError::PaneNotFound),
                };
                let Some((id, data)) = split_factory(&p, d, &data) else {
                    return Err(PaneCommandError::SplitRefused);
                };
                self.split(&p, d, id, data)
            }
            Close => {
                let p = focused()?;
                if self.tree.leaf_ids().len() == 1 {
                    return Err(PaneCommandError::CannotCloseLastPane);
                }
                self.close(&p).is_some()
            }
            Move(d) => {
                let p = focused()?;
                let n = directional_neighbor(&self.tree, &p, d, |k| {
                    find_ratio(&self.tree, k).unwrap_or(0.5)
                });
                n.is_some_and(|n| self.move_pane(&p, &n, d.drop_edge()))
            }
            Swap(d) => {
                let p = focused()?;
                let n = directional_neighbor(&self.tree, &p, d, |k| {
                    find_ratio(&self.tree, k).unwrap_or(0.5)
                });
                n.is_some_and(|n| self.swap(&p, &n))
            }
            SwapNext | SwapPrevious => {
                let p = focused()?;
                let ids = self.tree.leaf_ids();
                let at = ids.iter().position(|x| x == &p).unwrap();
                let n = if matches!(command, SwapNext) {
                    (at + 1) % ids.len()
                } else {
                    (at + ids.len() - 1) % ids.len()
                };
                self.swap(&p, &ids[n])
            }
            Resize(d) => self.resize_focused(d, 0.05),
            SetParentSplitDirection(d) => {
                let p = focused()?;
                self.set_direction(&p, d)
            }
            ToggleParentSplitDirection => {
                let p = focused()?;
                let Some(d) = self.tree.parent_split_direction(&p) else {
                    return Err(PaneCommandError::InvalidOperation);
                };
                self.set_direction(
                    &p,
                    if d == SplitDirection::Horizontal {
                        SplitDirection::Vertical
                    } else {
                        SplitDirection::Horizontal
                    },
                )
            }
            Balance => self.balance(),
            Rotate(r) => self.rotate(r),
            ApplyLayout(l) => self.apply_layout(l),
            ToggleZoom => self.toggle_zoom(),
        };
        if ok {
            Ok(())
        } else {
            Err(PaneCommandError::InvalidOperation)
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
    fn command_split_uses_host_factory() {
        let mut m = model();
        m.execute(PaneCommand::Split(SplitDirection::Vertical), |_, _, _| {
            Some((PaneId::new("c"), D(3)))
        })
        .unwrap();
        assert!(m.tree().contains(&PaneId::new("c")));
    }
}
