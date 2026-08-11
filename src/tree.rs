use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PaneId(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ActivityId(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct CategoryId(pub String);

impl PaneId {
    pub fn new(id: impl Into<String>) -> Self {
        PaneId(id.into())
    }
}

impl ActivityId {
    pub fn new(id: impl Into<String>) -> Self {
        ActivityId(id.into())
    }
}

impl CategoryId {
    pub fn new(id: impl Into<String>) -> Self {
        CategoryId(id.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A direction relative to a pane's rendered position.
///
/// Unlike [`SplitDirection`], this describes navigation and manipulation in
/// screen space rather than the axis of a split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
}

impl PaneDirection {
    /// The drop edge used when moving a pane in this direction.
    pub fn drop_edge(self) -> DropEdge {
        match self {
            PaneDirection::Left => DropEdge::Left,
            PaneDirection::Right => DropEdge::Right,
            PaneDirection::Up => DropEdge::Top,
            PaneDirection::Down => DropEdge::Bottom,
        }
    }
}

/// Standard whole-tree layouts available to pane commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneLayout {
    /// Equal-width columns.
    EvenHorizontal,
    /// Equal-height rows.
    EvenVertical,
    /// A large focused pane above equal-width secondary panes.
    MainHorizontal,
    /// A large focused pane left of equal-height secondary panes.
    MainVertical,
    /// A balanced, alternating grid.
    Tiled,
}

/// Direction used when rotating pane contents through the current layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneRotation {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropEdge {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl DropEdge {
    pub fn split_direction(&self) -> SplitDirection {
        match self {
            DropEdge::Top | DropEdge::Bottom => SplitDirection::Vertical,
            DropEdge::Left | DropEdge::Right | DropEdge::Center => SplitDirection::Horizontal,
        }
    }

    pub fn source_is_first(&self) -> bool {
        matches!(self, DropEdge::Top | DropEdge::Left)
    }
}

/// Minimal trait bound for consumer-defined pane data.
///
/// Mullion keeps pane state on GPUI's foreground thread, so UI-local data does
/// not need to be `Send`, `Sync`, or serializable. Serialization bounds are
/// required only when a pane tree or workspace snapshot is actually encoded.
pub trait PaneData: Clone + PartialEq + 'static {}

impl<T> PaneData for T where T: Clone + PartialEq + 'static {}

/// One step from a split node to one of its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaneNodeBranch {
    First,
    Second,
}

/// A stable structural location in a [`PaneNode`] tree.
///
/// An empty path names the root. Each branch identifies the child selected at
/// the corresponding split, making validation errors actionable even when a
/// pane id itself is duplicated.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PaneNodePath(pub Vec<PaneNodeBranch>);

impl fmt::Display for PaneNodePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("root")?;
        for branch in &self.0 {
            match branch {
                PaneNodeBranch::First => formatter.write_str(".first")?,
                PaneNodeBranch::Second => formatter.write_str(".second")?,
            }
        }
        Ok(())
    }
}

/// An invariant violation in a pane tree intended for persistence.
#[derive(Clone, Debug, PartialEq)]
pub enum PaneValidationError {
    /// Two leaf nodes use the same pane id.
    DuplicatePaneId {
        pane_id: PaneId,
        first_path: PaneNodePath,
        duplicate_path: PaneNodePath,
    },
    /// A split ratio is NaN or positive/negative infinity.
    NonFiniteSplitRatio {
        split_key: PaneId,
        path: PaneNodePath,
        ratio: f64,
    },
    /// A finite split ratio is outside the supported inclusive range.
    SplitRatioOutOfRange {
        split_key: PaneId,
        path: PaneNodePath,
        ratio: f64,
    },
}

impl fmt::Display for PaneValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneValidationError::DuplicatePaneId {
                pane_id,
                first_path,
                duplicate_path,
            } => write!(
                formatter,
                "duplicate pane id {:?} at {duplicate_path} (first seen at {first_path})",
                pane_id.0
            ),
            PaneValidationError::NonFiniteSplitRatio {
                split_key,
                path,
                ratio,
            } => write!(
                formatter,
                "split keyed by {:?} at {path} has non-finite ratio {ratio}",
                split_key.0
            ),
            PaneValidationError::SplitRatioOutOfRange {
                split_key,
                path,
                ratio,
            } => write!(
                formatter,
                "split keyed by {:?} at {path} has ratio {ratio} outside 0.1..=0.9",
                split_key.0
            ),
        }
    }
}

impl std::error::Error for PaneValidationError {}

/// A node in the pane tree — either a leaf pane or a split containing two children.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaneNode<D: PaneData> {
    Leaf {
        id: PaneId,
        active_activity: Option<ActivityId>,
        data: D,
    },
    Split {
        direction: SplitDirection,
        /// Fraction (0.0..1.0) of space allocated to `first`.
        ratio: f64,
        first: Box<PaneNode<D>>,
        second: Box<PaneNode<D>>,
    },
}

impl<D: PaneData> PaneNode<D> {
    /// Create a new leaf node.
    pub fn leaf(id: PaneId, data: D) -> Self {
        PaneNode::Leaf {
            id,
            active_activity: None,
            data,
        }
    }

    /// Create a leaf with an initial active activity.
    pub fn leaf_with_activity(id: PaneId, activity: ActivityId, data: D) -> Self {
        PaneNode::Leaf {
            id,
            active_activity: Some(activity),
            data,
        }
    }

    /// Find a leaf by id (immutable).
    pub fn find(&self, target: &PaneId) -> Option<&PaneNode<D>> {
        match self {
            PaneNode::Leaf { id, .. } if id == target => Some(self),
            PaneNode::Split { first, second, .. } => {
                first.find(target).or_else(|| second.find(target))
            }
            _ => None,
        }
    }

    /// Find a leaf by id (mutable).
    pub fn find_mut(&mut self, target: &PaneId) -> Option<&mut PaneNode<D>> {
        match self {
            PaneNode::Leaf { id, .. } if id == target => Some(self),
            PaneNode::Split { first, second, .. } => {
                first.find_mut(target).or_else(|| second.find_mut(target))
            }
            _ => None,
        }
    }

    /// Split a leaf pane. The original becomes `first`, new pane becomes `second`.
    pub fn split(
        &mut self,
        target: &PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        new_data: D,
    ) -> bool {
        if let PaneNode::Leaf {
            id,
            active_activity,
            ..
        } = self
        {
            if id == target {
                let inherit_activity = active_activity.clone();
                let original = std::mem::replace(
                    self,
                    PaneNode::leaf(PaneId::new("__temp__"), new_data.clone()),
                );
                let new_leaf = PaneNode::Leaf {
                    id: new_id,
                    active_activity: inherit_activity,
                    data: new_data,
                };
                *self = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(original),
                    second: Box::new(new_leaf),
                };
                return true;
            }
        }
        if let PaneNode::Split { first, second, .. } = self {
            if first.split(target, direction, new_id.clone(), new_data.clone()) {
                return true;
            }
            return second.split(target, direction, new_id, new_data);
        }
        false
    }

    /// Remove a pane, collapsing its parent split.
    pub fn close(&mut self, target: &PaneId) -> Option<D> {
        self.close_inner(target).map(|(data, _)| data)
    }

    fn close_inner(&mut self, target: &PaneId) -> Option<(D, bool)> {
        match self {
            PaneNode::Leaf { id, .. } if id == target => None,
            PaneNode::Split { first, second, .. } => {
                if let PaneNode::Leaf { id, data, .. } = first.as_ref() {
                    if id == target {
                        let data = data.clone();
                        let sibling = *second.clone();
                        *self = sibling;
                        return Some((data, true));
                    }
                }
                if let PaneNode::Leaf { id, data, .. } = second.as_ref() {
                    if id == target {
                        let data = data.clone();
                        let sibling = *first.clone();
                        *self = sibling;
                        return Some((data, true));
                    }
                }
                if let Some(result) = first.close_inner(target) {
                    return Some(result);
                }
                second.close_inner(target)
            }
            _ => None,
        }
    }

    /// Change the split direction of the immediate parent of `target`.
    pub fn change_direction(&mut self, target: &PaneId, new_direction: SplitDirection) -> bool {
        match self {
            PaneNode::Split {
                direction,
                first,
                second,
                ..
            } => {
                let first_contains = first.contains(target);
                let second_contains = second.contains(target);
                if first_contains || second_contains {
                    let is_direct_child = match (first.as_ref(), second.as_ref()) {
                        (PaneNode::Leaf { id, .. }, _) if id == target => true,
                        (_, PaneNode::Leaf { id, .. }) if id == target => true,
                        _ => false,
                    };
                    if is_direct_child {
                        *direction = new_direction;
                        return true;
                    }
                    if first_contains {
                        return first.change_direction(target, new_direction);
                    }
                    return second.change_direction(target, new_direction);
                }
                false
            }
            _ => false,
        }
    }

    /// Set the ratio of the split identified by `split_key` — the first leaf
    /// id under the split's `second` subtree.
    ///
    /// Keying splits by "first leaf of second" (rather than any leaf under
    /// `first`) is what makes splits addressable without collisions: every
    /// leaf lives in exactly one place in the tree, so it can be the
    /// "leftmost of second" for at most one ancestor split. Using "leftmost
    /// of first" collides the moment you split a pane in place, because the
    /// original pane stays leftmost of both the new outer and inner splits.
    ///
    /// Returns `true` if a matching split was found and updated. Non-finite
    /// ratios (`NaN`, `±inf`) are rejected.
    pub fn set_split_ratio(&mut self, split_key: &PaneId, new_ratio: f64) -> bool {
        if !new_ratio.is_finite() {
            return false;
        }
        let clamped = new_ratio.clamp(0.1, 0.9);
        match self {
            PaneNode::Split {
                ratio,
                first,
                second,
                ..
            } => {
                if second.leftmost_leaf_id() == split_key {
                    if *ratio == clamped {
                        return false;
                    }
                    *ratio = clamped;
                    return true;
                }
                first.set_split_ratio(split_key, new_ratio)
                    || second.set_split_ratio(split_key, new_ratio)
            }
            _ => false,
        }
    }

    /// Move a pane from one position to another.
    pub fn move_pane(&mut self, source: &PaneId, destination: &PaneId, edge: DropEdge) -> bool {
        if source == destination {
            return false;
        }
        let (id, data, active_activity) = match self.find(source) {
            Some(PaneNode::Leaf {
                id,
                data,
                active_activity,
                ..
            }) => (id.clone(), data.clone(), active_activity.clone()),
            _ => return false,
        };
        if self.close(source).is_none() {
            return false;
        }
        // `close` may have collapsed the destination's parent, but never the
        // destination leaf itself, so it is still addressable.
        self.insert_leaf(destination, edge, id, data, active_activity)
    }

    /// Insert a brand-new leaf beside `destination`, splitting it at `edge`.
    ///
    /// This is the second half of [`PaneNode::move_pane`] on its own: the
    /// destination leaf is replaced by a split holding the original and the new
    /// leaf, ordered by [`DropEdge::source_is_first`]. Use it for drop-to-create
    /// (an activity dragged out of the activity bar), where there is no existing
    /// pane to relocate.
    ///
    /// Unlike [`PaneNode::split`], this honours the drop edge, so the new leaf
    /// can land *before* the destination (`Top`/`Left`) as well as after it.
    ///
    /// Returns `false` if `destination` is not a leaf in this tree.
    pub fn insert_leaf(
        &mut self,
        destination: &PaneId,
        edge: DropEdge,
        new_id: PaneId,
        new_data: D,
        active_activity: Option<ActivityId>,
    ) -> bool {
        let Some(dest_node) = self.find_mut(destination) else {
            return false;
        };
        let original = std::mem::replace(
            dest_node,
            PaneNode::leaf(PaneId::new("__temp__"), new_data.clone()),
        );
        let new_leaf = PaneNode::Leaf {
            id: new_id,
            active_activity,
            data: new_data,
        };
        let (first, second) = if edge.source_is_first() {
            (Box::new(new_leaf), Box::new(original))
        } else {
            (Box::new(original), Box::new(new_leaf))
        };
        *dest_node = PaneNode::Split {
            direction: edge.split_direction(),
            ratio: 0.5,
            first,
            second,
        };
        true
    }

    /// Check if this subtree contains a pane with the given id.
    pub fn contains(&self, target: &PaneId) -> bool {
        self.find(target).is_some()
    }

    /// Validate the invariants required to safely persist and restore this tree.
    ///
    /// Pane ids must be unique, and every split ratio must be finite and in
    /// the inclusive `0.1..=0.9` range used by pane resizing. The error
    /// includes structural paths and, for splits, the same key used by the
    /// renderer and resize APIs.
    pub fn validate(&self) -> Result<(), PaneValidationError> {
        fn walk<D: PaneData>(
            node: &PaneNode<D>,
            path: &mut Vec<PaneNodeBranch>,
            pane_paths: &mut HashMap<PaneId, PaneNodePath>,
        ) -> Result<(), PaneValidationError> {
            match node {
                PaneNode::Leaf { id, .. } => {
                    let current_path = PaneNodePath(path.clone());
                    if let Some(first_path) = pane_paths.get(id) {
                        return Err(PaneValidationError::DuplicatePaneId {
                            pane_id: id.clone(),
                            first_path: first_path.clone(),
                            duplicate_path: current_path,
                        });
                    }
                    pane_paths.insert(id.clone(), current_path);
                    Ok(())
                }
                PaneNode::Split {
                    ratio,
                    first,
                    second,
                    ..
                } => {
                    let split_key = second.leftmost_leaf_id().clone();
                    let split_path = PaneNodePath(path.clone());
                    if !ratio.is_finite() {
                        return Err(PaneValidationError::NonFiniteSplitRatio {
                            split_key,
                            path: split_path,
                            ratio: *ratio,
                        });
                    }
                    if !(0.1..=0.9).contains(ratio) {
                        return Err(PaneValidationError::SplitRatioOutOfRange {
                            split_key,
                            path: split_path,
                            ratio: *ratio,
                        });
                    }

                    path.push(PaneNodeBranch::First);
                    walk(first, path, pane_paths)?;
                    path.pop();
                    path.push(PaneNodeBranch::Second);
                    let result = walk(second, path, pane_paths);
                    path.pop();
                    result
                }
            }
        }

        walk(self, &mut Vec::new(), &mut HashMap::new())
    }

    /// Collect all leaf PaneIds.
    pub fn leaf_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    fn collect_ids(&self, ids: &mut Vec<PaneId>) {
        match self {
            PaneNode::Leaf { id, .. } => ids.push(id.clone()),
            PaneNode::Split { first, second, .. } => {
                first.collect_ids(ids);
                second.collect_ids(ids);
            }
        }
    }

    /// Swap two leaf panes while keeping the split topology and ratios intact.
    ///
    /// The entire leaf (id, active activity, and data) moves, which lets the
    /// flat renderer preserve each keyed pane component while changing its
    /// position.
    pub fn swap_panes(&mut self, first_id: &PaneId, second_id: &PaneId) -> bool {
        if first_id == second_id {
            return false;
        }
        let Some(first_path) = leaf_path(self, first_id) else {
            return false;
        };
        let Some(second_path) = leaf_path(self, second_id) else {
            return false;
        };
        let first = node_at_path(self, &first_path).clone();
        let second = node_at_path(self, &second_path).clone();
        *node_at_path_mut(self, &first_path) = second;
        *node_at_path_mut(self, &second_path) = first;
        true
    }

    /// Rotate every leaf through the existing layout slots.
    ///
    /// Split directions and ratios remain unchanged. Returns `false` when
    /// there are fewer than two panes.
    pub fn rotate_panes(&mut self, rotation: PaneRotation) -> bool {
        let paths = leaf_paths(self);
        if paths.len() < 2 {
            return false;
        }
        let leaves: Vec<_> = paths
            .iter()
            .map(|path| node_at_path(self, path).clone())
            .collect();
        let count = leaves.len();
        for (index, path) in paths.iter().enumerate() {
            let source = match rotation {
                // Each pane advances to the next layout slot.
                PaneRotation::Forward => (index + count - 1) % count,
                PaneRotation::Backward => (index + 1) % count,
            };
            *node_at_path_mut(self, path) = leaves[source].clone();
        }
        true
    }

    /// Set every split ratio in the tree to `0.5`.
    ///
    /// Returns the number of splits encountered, including splits that were
    /// already balanced.
    pub fn balance_splits(&mut self) -> usize {
        match self {
            PaneNode::Leaf { .. } => 0,
            PaneNode::Split {
                ratio,
                first,
                second,
                ..
            } => {
                *ratio = 0.5;
                1 + first.balance_splits() + second.balance_splits()
            }
        }
    }

    /// Rebuild the split topology using a standard layout.
    ///
    /// Leaf panes are retained intact and in their current traversal order.
    /// `main` selects the large pane for the two main-pane layouts; if it is
    /// absent or unknown, the first pane is used. Returns `false` for a
    /// single-pane tree, where every layout is equivalent.
    pub fn apply_layout(&mut self, layout: PaneLayout, main: Option<&PaneId>) -> bool {
        let mut leaves: Vec<_> = leaf_paths(self)
            .iter()
            .map(|path| node_at_path(self, path).clone())
            .collect();
        if leaves.len() < 2 {
            return false;
        }

        if matches!(
            layout,
            PaneLayout::MainHorizontal | PaneLayout::MainVertical
        ) {
            if let Some(main) = main {
                if let Some(index) = leaves
                    .iter()
                    .position(|leaf| matches!(leaf, PaneNode::Leaf { id, .. } if id == main))
                {
                    leaves.swap(0, index);
                }
            }
        }

        *self = match layout {
            PaneLayout::EvenHorizontal => build_even_layout(leaves, SplitDirection::Horizontal),
            PaneLayout::EvenVertical => build_even_layout(leaves, SplitDirection::Vertical),
            PaneLayout::MainHorizontal => {
                build_main_layout(leaves, SplitDirection::Vertical, SplitDirection::Horizontal)
            }
            PaneLayout::MainVertical => {
                build_main_layout(leaves, SplitDirection::Horizontal, SplitDirection::Vertical)
            }
            PaneLayout::Tiled => build_tiled_layout(leaves, SplitDirection::Horizontal),
        };
        true
    }

    /// Direction of the split immediately containing `target`.
    pub fn parent_split_direction(&self, target: &PaneId) -> Option<SplitDirection> {
        match self {
            PaneNode::Leaf { .. } => None,
            PaneNode::Split {
                direction,
                first,
                second,
                ..
            } => {
                let direct = matches!(first.as_ref(), PaneNode::Leaf { id, .. } if id == target)
                    || matches!(second.as_ref(), PaneNode::Leaf { id, .. } if id == target);
                if direct {
                    Some(*direction)
                } else if first.contains(target) {
                    first.parent_split_direction(target)
                } else if second.contains(target) {
                    second.parent_split_direction(target)
                } else {
                    None
                }
            }
        }
    }

    /// Returns a reference to the id of the leftmost leaf under this node.
    ///
    /// O(depth), unlike `leaf_ids().into_iter().next()` which is O(subtree
    /// size) because it allocates the full leaf-id vector. Used heavily by
    /// the renderer's rect walks — those fire once per leaf per structural
    /// mutation, so using the O(subtree-size) form there turns a single
    /// structural change into O(N³) work.
    pub fn leftmost_leaf_id(&self) -> &PaneId {
        let mut node = self;
        loop {
            match node {
                PaneNode::Leaf { id, .. } => return id,
                PaneNode::Split { first, .. } => node = first,
            }
        }
    }
}

/// `false` selects a split's first child and `true` its second child.
type PanePath = Vec<bool>;

fn leaf_path<D: PaneData>(tree: &PaneNode<D>, target: &PaneId) -> Option<PanePath> {
    fn walk<D: PaneData>(node: &PaneNode<D>, target: &PaneId, path: &mut PanePath) -> bool {
        match node {
            PaneNode::Leaf { id, .. } => id == target,
            PaneNode::Split { first, second, .. } => {
                path.push(false);
                if walk(first, target, path) {
                    return true;
                }
                path.pop();
                path.push(true);
                if walk(second, target, path) {
                    return true;
                }
                path.pop();
                false
            }
        }
    }

    let mut path = Vec::new();
    walk(tree, target, &mut path).then_some(path)
}

fn leaf_paths<D: PaneData>(tree: &PaneNode<D>) -> Vec<PanePath> {
    fn walk<D: PaneData>(node: &PaneNode<D>, path: &mut PanePath, out: &mut Vec<PanePath>) {
        match node {
            PaneNode::Leaf { .. } => out.push(path.clone()),
            PaneNode::Split { first, second, .. } => {
                path.push(false);
                walk(first, path, out);
                path.pop();
                path.push(true);
                walk(second, path, out);
                path.pop();
            }
        }
    }

    let mut out = Vec::new();
    walk(tree, &mut Vec::new(), &mut out);
    out
}

fn node_at_path<'a, D: PaneData>(mut node: &'a PaneNode<D>, path: &[bool]) -> &'a PaneNode<D> {
    for second_child in path {
        node = match node {
            PaneNode::Split { first, second, .. } => {
                if *second_child {
                    second
                } else {
                    first
                }
            }
            PaneNode::Leaf { .. } => unreachable!("leaf path cannot descend through a leaf"),
        };
    }
    node
}

fn node_at_path_mut<'a, D: PaneData>(
    mut node: &'a mut PaneNode<D>,
    path: &[bool],
) -> &'a mut PaneNode<D> {
    for second_child in path {
        node = match node {
            PaneNode::Split { first, second, .. } => {
                if *second_child {
                    second
                } else {
                    first
                }
            }
            PaneNode::Leaf { .. } => unreachable!("leaf path cannot descend through a leaf"),
        };
    }
    node
}

fn build_even_layout<D: PaneData>(
    mut leaves: Vec<PaneNode<D>>,
    direction: SplitDirection,
) -> PaneNode<D> {
    if leaves.len() == 1 {
        return leaves.pop().expect("one leaf");
    }
    let count = leaves.len();
    let first = leaves.remove(0);
    let second = build_even_layout(leaves, direction);
    PaneNode::Split {
        direction,
        ratio: 1.0 / count as f64,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn build_main_layout<D: PaneData>(
    mut leaves: Vec<PaneNode<D>>,
    main_direction: SplitDirection,
    secondary_direction: SplitDirection,
) -> PaneNode<D> {
    let main = leaves.remove(0);
    let secondary = build_even_layout(leaves, secondary_direction);
    PaneNode::Split {
        direction: main_direction,
        ratio: 0.6,
        first: Box::new(main),
        second: Box::new(secondary),
    }
}

fn build_tiled_layout<D: PaneData>(
    mut leaves: Vec<PaneNode<D>>,
    direction: SplitDirection,
) -> PaneNode<D> {
    if leaves.len() == 1 {
        return leaves.pop().expect("one leaf");
    }
    let count = leaves.len();
    let first_count = count.div_ceil(2);
    let second = leaves.split_off(first_count);
    let next_direction = match direction {
        SplitDirection::Horizontal => SplitDirection::Vertical,
        SplitDirection::Vertical => SplitDirection::Horizontal,
    };
    PaneNode::Split {
        direction,
        ratio: first_count as f64 / count as f64,
        first: Box::new(build_tiled_layout(leaves, next_direction)),
        second: Box::new(build_tiled_layout(second, next_direction)),
    }
}

/// Look up the ratio of the split whose `split_key` (first-leaf-of-second) equals `key`.
pub fn find_ratio<D: PaneData>(node: &PaneNode<D>, key: &PaneId) -> Option<f64> {
    match node {
        PaneNode::Leaf { .. } => None,
        PaneNode::Split {
            ratio,
            first,
            second,
            ..
        } => {
            if second.leftmost_leaf_id() == key {
                Some(*ratio)
            } else {
                find_ratio(first, key).or_else(|| find_ratio(second, key))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
    struct D(u32);

    #[derive(Clone, PartialEq)]
    struct UiLocalData(std::rc::Rc<String>);

    #[test]
    fn pane_data_can_be_ui_local_and_non_serializable() {
        let tree = PaneNode::leaf(
            PaneId::new("local"),
            UiLocalData(std::rc::Rc::new("state".into())),
        );
        assert!(tree.validate().is_ok());
    }

    fn sample() -> PaneNode<D> {
        PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
        }
    }

    #[test]
    fn validation_accepts_single_and_nested_valid_layouts() {
        assert_eq!(PaneNode::leaf(PaneId::new("only"), D(1)).validate(), Ok(()));

        let tree = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.1,
            first: Box::new(PaneNode::leaf(PaneId::new("pane/雪?x=1&y=two"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.9,
                first: Box::new(PaneNode::leaf(PaneId::new("spaces and \"quotes\""), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("emoji-🚀\nnewline"), D(3))),
            }),
        };
        assert_eq!(tree.validate(), Ok(()));

        let json = serde_json::to_string(&tree).expect("valid special ids serialize");
        let restored: PaneNode<D> = serde_json::from_str(&json).expect("valid special ids restore");
        assert_eq!(restored, tree);
        assert_eq!(restored.validate(), Ok(()));
    }

    #[test]
    fn validation_reports_nested_duplicate_with_both_locations() {
        let json = r#"{
            "Split": {
                "direction": "Horizontal",
                "ratio": 0.5,
                "first": { "Split": {
                    "direction": "Vertical",
                    "ratio": 0.4,
                    "first": { "Leaf": {
                        "id": "unique", "active_activity": null, "data": 1
                    } },
                    "second": { "Leaf": {
                        "id": "duplicate/雪", "active_activity": null, "data": 2
                    } }
                } },
                "second": { "Leaf": {
                    "id": "duplicate/雪", "active_activity": null, "data": 3
                } }
            }
        }"#;
        let tree: PaneNode<D> =
            serde_json::from_str(json).expect("hostile tree still deserializes");

        assert_eq!(
            tree.validate(),
            Err(PaneValidationError::DuplicatePaneId {
                pane_id: PaneId::new("duplicate/雪"),
                first_path: PaneNodePath(vec![PaneNodeBranch::First, PaneNodeBranch::Second,]),
                duplicate_path: PaneNodePath(vec![PaneNodeBranch::Second]),
            })
        );
    }

    #[test]
    fn validation_rejects_every_non_finite_ratio() {
        for ratio in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut tree = sample();
            let PaneNode::Split {
                ratio: tree_ratio, ..
            } = &mut tree
            else {
                unreachable!()
            };
            *tree_ratio = ratio;

            match tree.validate() {
                Err(PaneValidationError::NonFiniteSplitRatio {
                    split_key,
                    path,
                    ratio: reported,
                }) => {
                    assert_eq!(split_key, PaneId::new("b"));
                    assert_eq!(path, PaneNodePath::default());
                    if ratio.is_nan() {
                        assert!(reported.is_nan());
                    } else {
                        assert_eq!(reported, ratio);
                    }
                }
                other => panic!("unexpected validation result: {other:?}"),
            }
        }
    }

    #[test]
    fn validation_rejects_ratios_outside_inclusive_limits() {
        for ratio in [-1.0, 0.0, 0.1 - f64::EPSILON, 0.9 + f64::EPSILON, 1.0, 2.0] {
            let json = format!(
                r#"{{"Split":{{"direction":"Horizontal","ratio":{ratio},"first":{{"Leaf":{{"id":"a","active_activity":null,"data":1}}}},"second":{{"Leaf":{{"id":"split-key","active_activity":null,"data":2}}}}}}}}"#
            );
            let tree: PaneNode<D> =
                serde_json::from_str(&json).expect("finite hostile ratio deserializes");
            assert_eq!(
                tree.validate(),
                Err(PaneValidationError::SplitRatioOutOfRange {
                    split_key: PaneId::new("split-key"),
                    path: PaneNodePath::default(),
                    ratio,
                })
            );
        }
    }

    #[test]
    fn validation_reports_nested_ratio_location_and_split_key() {
        let tree = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.91,
                first: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("nested-key"), D(3))),
            }),
        };
        assert_eq!(
            tree.validate(),
            Err(PaneValidationError::SplitRatioOutOfRange {
                split_key: PaneId::new("nested-key"),
                path: PaneNodePath(vec![PaneNodeBranch::Second]),
                ratio: 0.91,
            })
        );
    }

    // These tests verify the *structural* view that the renderer subscribes
    // to (`leaf_ids()` and `collect_split_keys()`) is stable across
    // non-structural mutations. The per-leaf render path subscribes only to
    // this structural view plus per-leaf/per-split signals, so any
    // non-structural mutation that DIDN'T change this view also cannot force
    // a re-mount of existing leaves.

    #[test]
    fn leaf_ids_and_split_keys_are_stable_across_ratio_changes() {
        let mut t = sample();
        let ids_before = t.leaf_ids();
        let keys_before = collect_split_keys(&t);
        // `sample()` is Split { first: leaf("a"), second: leaf("b") }.
        // The split's key is "b" (first leaf of second subtree).
        assert!(t.set_split_ratio(&PaneId::new("b"), 0.8));
        assert_eq!(t.leaf_ids(), ids_before);
        assert_eq!(collect_split_keys(&t), keys_before);
    }

    #[test]
    fn leaf_ids_and_split_keys_are_stable_across_data_changes() {
        let mut t = sample();
        let ids_before = t.leaf_ids();
        let keys_before = collect_split_keys(&t);
        if let Some(PaneNode::Leaf { data, .. }) = t.find_mut(&PaneId::new("a")) {
            *data = D(99);
        }
        assert_eq!(t.leaf_ids(), ids_before);
        assert_eq!(collect_split_keys(&t), keys_before);
    }

    #[test]
    fn leaf_ids_and_split_keys_are_stable_across_active_activity_changes() {
        let mut t = sample();
        let ids_before = t.leaf_ids();
        let keys_before = collect_split_keys(&t);
        if let Some(PaneNode::Leaf {
            active_activity, ..
        }) = t.find_mut(&PaneId::new("a"))
        {
            *active_activity = Some(ActivityId::new("foo"));
        }
        assert_eq!(t.leaf_ids(), ids_before);
        assert_eq!(collect_split_keys(&t), keys_before);
    }

    #[test]
    fn split_keys_are_stable_across_direction_changes() {
        // `change_direction` changes a split's direction but not its set of
        // leaves, so its `split_key` (first leaf of `second`) must stay the
        // same — the renderer depends on this to reuse the existing handle
        // instance rather than remounting it.
        let mut t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
        };
        let keys_before = collect_split_keys(&t);
        assert_eq!(
            find_split_direction(&t, &PaneId::new("b")),
            Some(SplitDirection::Horizontal),
        );
        assert!(t.change_direction(&PaneId::new("a"), SplitDirection::Vertical));
        assert_eq!(collect_split_keys(&t), keys_before);
        assert_eq!(
            find_split_direction(&t, &PaneId::new("b")),
            Some(SplitDirection::Vertical),
        );
    }

    #[test]
    fn leaf_ids_change_on_split() {
        let mut t = sample();
        let before = t.leaf_ids();
        t.split(
            &PaneId::new("a"),
            SplitDirection::Vertical,
            PaneId::new("c"),
            D(3),
        );
        let after = t.leaf_ids();
        assert_ne!(before, after);
        assert!(after.contains(&PaneId::new("c")));
    }

    #[test]
    fn leaf_ids_change_on_close() {
        let mut t = sample();
        t.close(&PaneId::new("a"));
        let after = t.leaf_ids();
        assert!(!after.contains(&PaneId::new("a")));
    }

    #[test]
    fn collect_split_ratios_walks_nested_splits() {
        // Key each split by the first leaf of its `second` subtree — that
        // value is unique across nested splits (unlike `first`'s leftmost
        // leaf, which collides when splitting a pane in place).
        let t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.3,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.7,
                first: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("c"), D(3))),
            }),
        };
        let mut out = Vec::new();
        collect_split_ratios(&t, &mut out);
        assert_eq!(out.len(), 2);
        // Outer: second is the inner split; inner's first leaf is "b".
        assert_eq!(out[0], (PaneId::new("b"), 0.3));
        // Inner: second is leaf("c").
        assert_eq!(out[1], (PaneId::new("c"), 0.7));
    }

    #[test]
    fn split_keys_are_unique_when_splitting_in_place() {
        // Regression: the previous keying (first-leaf-of-first) collided
        // when a pane was split in place, because the original pane stayed
        // leftmost of both the outer and inner splits.
        let mut t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.25,
            first: Box::new(PaneNode::leaf(PaneId::new("sidebar"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("main"), D(2))),
        };
        t.split(
            &PaneId::new("sidebar"),
            SplitDirection::Horizontal,
            PaneId::new("new"),
            D(3),
        );

        let mut keys = Vec::new();
        collect_split_ratios(&t, &mut keys);
        let key_set: std::collections::HashSet<_> = keys.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(
            key_set.len(),
            keys.len(),
            "split keys must be unique, got {:?}",
            keys,
        );
    }

    // ---- insert_leaf (drop-to-create) ----

    fn leaf_order(node: &PaneNode<D>) -> Vec<PaneId> {
        node.leaf_ids()
    }

    #[test]
    fn insert_leaf_before_destination_on_leading_edges() {
        // Top/Left mean "the new pane goes first".
        for edge in [DropEdge::Left, DropEdge::Top] {
            let mut t = sample();
            assert!(t.insert_leaf(&PaneId::new("b"), edge, PaneId::new("new"), D(9), None));
            assert_eq!(
                leaf_order(&t),
                vec![PaneId::new("a"), PaneId::new("new"), PaneId::new("b")],
                "edge {edge:?} should place the new leaf before the destination",
            );
        }
    }

    #[test]
    fn insert_leaf_after_destination_on_trailing_edges() {
        for edge in [DropEdge::Right, DropEdge::Bottom, DropEdge::Center] {
            let mut t = sample();
            assert!(t.insert_leaf(&PaneId::new("a"), edge, PaneId::new("new"), D(9), None));
            assert_eq!(
                leaf_order(&t),
                vec![PaneId::new("a"), PaneId::new("new"), PaneId::new("b")],
                "edge {edge:?} should place the new leaf after the destination",
            );
        }
    }

    #[test]
    fn insert_leaf_uses_the_edges_split_direction() {
        let mut t = sample();
        t.insert_leaf(
            &PaneId::new("a"),
            DropEdge::Bottom,
            PaneId::new("new"),
            D(9),
            None,
        );
        // "a" was `first` of a horizontal split; it is now a vertical split.
        let PaneNode::Split { first, .. } = &t else {
            panic!("root should still be a split")
        };
        assert!(matches!(
            **first,
            PaneNode::Split {
                direction: SplitDirection::Vertical,
                ..
            }
        ));
    }

    #[test]
    fn insert_leaf_carries_the_dropped_activity() {
        let mut t = sample();
        let act = ActivityId::new("files");
        t.insert_leaf(
            &PaneId::new("a"),
            DropEdge::Right,
            PaneId::new("new"),
            D(9),
            Some(act.clone()),
        );
        let Some(PaneNode::Leaf {
            active_activity, ..
        }) = t.find(&PaneId::new("new"))
        else {
            panic!("new leaf should exist")
        };
        assert_eq!(active_activity.as_ref(), Some(&act));
    }

    #[test]
    fn insert_leaf_on_unknown_destination_is_a_no_op() {
        let mut t = sample();
        let before = t.clone();
        assert!(!t.insert_leaf(
            &PaneId::new("nope"),
            DropEdge::Right,
            PaneId::new("new"),
            D(9),
            None
        ));
        assert_eq!(t, before, "a refused insert must not touch the tree");
    }

    #[test]
    fn split_keys_stay_unique_after_insert_leaf_on_every_edge() {
        // The hazard: dropping into an edge mints a new split node, and split
        // ratios (and the `<For>` over split handles) are keyed by split_key.
        // A collision would make two splits share a ratio signal and a render
        // key. Keys are derived (first leaf of the `second` subtree), not
        // generated, so this asserts the derivation survives insert_leaf at
        // every edge and at every position in a nested tree.
        for edge in [
            DropEdge::Left,
            DropEdge::Right,
            DropEdge::Top,
            DropEdge::Bottom,
            DropEdge::Center,
        ] {
            for dest in ["a", "b", "c"] {
                let mut t = PaneNode::Split {
                    direction: SplitDirection::Horizontal,
                    ratio: 0.25,
                    first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
                    second: Box::new(PaneNode::Split {
                        direction: SplitDirection::Vertical,
                        ratio: 0.5,
                        first: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
                        second: Box::new(PaneNode::leaf(PaneId::new("c"), D(3))),
                    }),
                };
                assert!(t.insert_leaf(&PaneId::new(dest), edge, PaneId::new("new"), D(9), None));

                let keys = collect_split_keys(&t);
                let unique: std::collections::HashSet<_> = keys.iter().cloned().collect();
                assert_eq!(
                    unique.len(),
                    keys.len(),
                    "duplicate split key after {edge:?} drop on {dest}: {keys:?}",
                );
                // One insert adds exactly one split.
                assert_eq!(keys.len(), 3, "{edge:?} on {dest}");
                // And no pane was lost or duplicated.
                let leaves = t.leaf_ids();
                let unique_leaves: std::collections::HashSet<_> = leaves.iter().cloned().collect();
                assert_eq!(unique_leaves.len(), 4, "{edge:?} on {dest}: {leaves:?}");
            }
        }
    }

    #[test]
    fn move_pane_still_relocates_after_insert_leaf_refactor() {
        let mut t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("c"), D(3))),
            }),
        };
        assert!(t.move_pane(&PaneId::new("a"), &PaneId::new("c"), DropEdge::Bottom));
        // "a" left its slot and now sits below "c"; no pane lost.
        assert_eq!(
            leaf_order(&t),
            vec![PaneId::new("b"), PaneId::new("c"), PaneId::new("a")],
        );
    }

    #[test]
    fn move_pane_onto_itself_is_rejected() {
        let mut t = sample();
        let before = t.clone();
        assert!(!t.move_pane(&PaneId::new("a"), &PaneId::new("a"), DropEdge::Right));
        assert_eq!(t, before);
    }

    #[test]
    fn rect_split_horizontal_preserves_total_width() {
        let r = Rect::FULL;
        let (a, b) = r.split(SplitDirection::Horizontal, 0.3);
        assert!((a.width + b.width - r.width).abs() < 1e-9);
        assert_eq!(a.height, r.height);
        assert_eq!(b.height, r.height);
        assert_eq!(b.left, r.left + a.width);
        assert!((a.width - 0.3).abs() < 1e-9);
    }

    #[test]
    fn rect_split_vertical_preserves_total_height() {
        let r = Rect::FULL;
        let (a, b) = r.split(SplitDirection::Vertical, 0.75);
        assert!((a.height + b.height - r.height).abs() < 1e-9);
        assert_eq!(a.width, r.width);
        assert_eq!(b.width, r.width);
        assert_eq!(b.top, r.top + a.height);
        assert!((a.height - 0.75).abs() < 1e-9);
    }

    #[test]
    fn leaf_rect_walks_nested_splits() {
        // sidebar(25%) | main-top(60% of right) / main-bottom(40% of right)
        let t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.25,
            first: Box::new(PaneNode::leaf(PaneId::new("sidebar"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.6,
                first: Box::new(PaneNode::leaf(PaneId::new("main-top"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("main-bottom"), D(3))),
            }),
        };
        let read = |_: &PaneId| 0.25; // unused because we'll use tree ratios
                                      // Actually use the tree's stored ratios via a closure
        let read_from_tree = |k: &PaneId| {
            let mut out = Vec::new();
            collect_split_ratios(&t, &mut out);
            out.iter()
                .find(|(key, _)| key == k)
                .map(|(_, r)| *r)
                .unwrap_or(0.5)
        };

        let sidebar_rect = leaf_rect(&t, &PaneId::new("sidebar"), read_from_tree).unwrap();
        assert_eq!(
            sidebar_rect,
            Rect {
                left: 0.0,
                top: 0.0,
                width: 0.25,
                height: 1.0
            }
        );

        let top_rect = leaf_rect(&t, &PaneId::new("main-top"), read_from_tree).unwrap();
        assert!((top_rect.left - 0.25).abs() < 1e-9);
        assert_eq!(top_rect.top, 0.0);
        assert!((top_rect.width - 0.75).abs() < 1e-9);
        assert!((top_rect.height - 0.6).abs() < 1e-9);

        let bot_rect = leaf_rect(&t, &PaneId::new("main-bottom"), read_from_tree).unwrap();
        assert!((bot_rect.left - 0.25).abs() < 1e-9);
        assert!((bot_rect.top - 0.6).abs() < 1e-9);
        assert!((bot_rect.width - 0.75).abs() < 1e-9);
        assert!((bot_rect.height - 0.4).abs() < 1e-9);

        let _ = read; // silence unused
    }

    #[test]
    fn split_parent_rect_returns_enclosing_area() {
        let t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.25,
            first: Box::new(PaneNode::leaf(PaneId::new("sidebar"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.6,
                first: Box::new(PaneNode::leaf(PaneId::new("main-top"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("main-bottom"), D(3))),
            }),
        };
        let read_from_tree = |k: &PaneId| {
            let mut out = Vec::new();
            collect_split_ratios(&t, &mut out);
            out.iter()
                .find(|(key, _)| key == k)
                .map(|(_, r)| *r)
                .unwrap_or(0.5)
        };

        // Outer split is at the root — parent rect is FULL.
        let outer = split_parent_rect(&t, &PaneId::new("main-top"), read_from_tree).unwrap();
        assert_eq!(outer, Rect::FULL);

        // Inner split occupies the right 75%.
        let inner = split_parent_rect(&t, &PaneId::new("main-bottom"), read_from_tree).unwrap();
        assert!((inner.left - 0.25).abs() < 1e-9);
        assert_eq!(inner.top, 0.0);
        assert!((inner.width - 0.75).abs() < 1e-9);
        assert_eq!(inner.height, 1.0);
    }

    #[test]
    fn find_ratio_returns_correct_value() {
        let t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.42,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
        };
        // Keyed by first-leaf-of-second: "b".
        assert_eq!(find_ratio(&t, &PaneId::new("b")), Some(0.42));
        assert_eq!(find_ratio(&t, &PaneId::new("a")), None);
        assert_eq!(find_ratio(&t, &PaneId::new("nonexistent")), None);
    }

    #[test]
    fn set_split_ratio_rejects_non_finite() {
        let mut t = sample();
        assert!(!t.set_split_ratio(&PaneId::new("b"), f64::NAN));
        assert!(!t.set_split_ratio(&PaneId::new("b"), f64::INFINITY));
        assert!(!t.set_split_ratio(&PaneId::new("b"), f64::NEG_INFINITY));
        // Ratio unchanged
        if let PaneNode::Split { ratio, .. } = &t {
            assert_eq!(*ratio, 0.5);
        } else {
            panic!("sample should be a split");
        }
    }

    #[test]
    fn set_split_ratio_updates_correct_split_when_nested() {
        let mut t = PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.25,
            first: Box::new(PaneNode::leaf(PaneId::new("sidebar"), D(1))),
            second: Box::new(PaneNode::leaf(PaneId::new("main"), D(2))),
        };
        t.split(
            &PaneId::new("sidebar"),
            SplitDirection::Horizontal,
            PaneId::new("new"),
            D(3),
        );
        // After split:
        //   outer = Split{ first=Split{ first=leaf(sidebar), second=leaf(new) },
        //                  second=leaf(main) }
        // Split keys (first-leaf-of-second):
        //   outer.second is leaf(main) → outer key = "main"
        //   inner.second is leaf(new)  → inner key = "new"
        assert!(t.set_split_ratio(&PaneId::new("main"), 0.6));
        assert!(t.set_split_ratio(&PaneId::new("new"), 0.3));

        // A leaf id that isn't any split's key should not match.
        assert!(!t.set_split_ratio(&PaneId::new("sidebar"), 0.9));

        // Verify the right splits got the right ratios.
        let mut collected = Vec::new();
        collect_split_ratios(&t, &mut collected);
        let outer = collected
            .iter()
            .find(|(k, _)| k == &PaneId::new("main"))
            .expect("outer split");
        let inner = collected
            .iter()
            .find(|(k, _)| k == &PaneId::new("new"))
            .expect("inner split");
        assert!((outer.1 - 0.6).abs() < 1e-9);
        assert!((inner.1 - 0.3).abs() < 1e-9);
    }

    fn three_pane_layout() -> PaneNode<D> {
        PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.4,
            first: Box::new(PaneNode::leaf(PaneId::new("a"), D(1))),
            second: Box::new(PaneNode::Split {
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(PaneNode::leaf(PaneId::new("b"), D(2))),
                second: Box::new(PaneNode::leaf(PaneId::new("c"), D(3))),
            }),
        }
    }

    fn stored_ratio(tree: &PaneNode<D>, key: &PaneId) -> f64 {
        find_ratio(tree, key).unwrap_or(0.5)
    }

    #[test]
    fn directional_navigation_follows_rendered_geometry() {
        let tree = three_pane_layout();
        assert_eq!(
            directional_neighbor(&tree, &PaneId::new("a"), PaneDirection::Right, |key| {
                stored_ratio(&tree, key)
            }),
            Some(PaneId::new("b"))
        );
        assert_eq!(
            directional_neighbor(&tree, &PaneId::new("b"), PaneDirection::Down, |key| {
                stored_ratio(&tree, key)
            }),
            Some(PaneId::new("c"))
        );
        assert_eq!(
            directional_neighbor(&tree, &PaneId::new("c"), PaneDirection::Left, |key| {
                stored_ratio(&tree, key)
            }),
            Some(PaneId::new("a"))
        );
        assert_eq!(
            directional_neighbor(&tree, &PaneId::new("a"), PaneDirection::Left, |key| {
                stored_ratio(&tree, key)
            }),
            None
        );
    }

    #[test]
    fn swap_moves_whole_leaves_without_changing_topology() {
        let mut tree = three_pane_layout();
        let directions_before: Vec<_> = collect_split_keys(&tree)
            .into_iter()
            .filter_map(|key| find_split_direction(&tree, &key))
            .collect();
        assert!(tree.swap_panes(&PaneId::new("a"), &PaneId::new("c")));
        assert_eq!(
            tree.leaf_ids(),
            vec![PaneId::new("c"), PaneId::new("b"), PaneId::new("a")]
        );
        assert!(matches!(
            tree.find(&PaneId::new("a")),
            Some(PaneNode::Leaf { data: D(1), .. })
        ));
        let directions_after: Vec<_> = collect_split_keys(&tree)
            .into_iter()
            .filter_map(|key| find_split_direction(&tree, &key))
            .collect();
        assert_eq!(directions_after, directions_before);
    }

    #[test]
    fn rotate_preserves_every_leaf_and_layout_slot() {
        let mut tree = three_pane_layout();
        assert!(tree.rotate_panes(PaneRotation::Forward));
        assert_eq!(
            tree.leaf_ids(),
            vec![PaneId::new("c"), PaneId::new("a"), PaneId::new("b")]
        );
        assert!(tree.rotate_panes(PaneRotation::Backward));
        assert_eq!(
            tree.leaf_ids(),
            vec![PaneId::new("a"), PaneId::new("b"), PaneId::new("c")]
        );
    }

    #[test]
    fn balance_resets_all_ratios() {
        let mut tree = three_pane_layout();
        assert_eq!(tree.balance_splits(), 2);
        let mut ratios = Vec::new();
        collect_split_ratios(&tree, &mut ratios);
        assert!(ratios.iter().all(|(_, ratio)| *ratio == 0.5));
    }

    #[test]
    fn even_layouts_give_every_pane_equal_space() {
        for layout in [PaneLayout::EvenHorizontal, PaneLayout::EvenVertical] {
            let mut tree = three_pane_layout();
            assert!(tree.apply_layout(layout, None));
            for id in tree.leaf_ids() {
                let rect = leaf_rect(&tree, &id, |key| stored_ratio(&tree, key)).unwrap();
                let dimension = match layout {
                    PaneLayout::EvenHorizontal => rect.width,
                    PaneLayout::EvenVertical => rect.height,
                    _ => unreachable!(),
                };
                assert!((dimension - 1.0 / 3.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn main_layout_promotes_the_selected_pane() {
        let mut tree = three_pane_layout();
        assert!(tree.apply_layout(PaneLayout::MainVertical, Some(&PaneId::new("c"))));
        let main = leaf_rect(&tree, &PaneId::new("c"), |key| stored_ratio(&tree, key)).unwrap();
        assert!((main.width - 0.6).abs() < 1e-9);
        assert_eq!(main.height, 1.0);
    }

    #[test]
    fn resize_boundary_uses_the_nearest_matching_ancestor() {
        let tree = three_pane_layout();
        assert_eq!(
            resize_boundary(&tree, &PaneId::new("b"), PaneDirection::Down),
            Some((PaneId::new("c"), 1.0))
        );
        assert_eq!(
            resize_boundary(&tree, &PaneId::new("b"), PaneDirection::Left),
            Some((PaneId::new("b"), -1.0))
        );
        assert_eq!(
            resize_boundary(&tree, &PaneId::new("a"), PaneDirection::Left),
            None
        );
    }
}

/// An axis-aligned rectangle in unit (0.0..=1.0) coordinates, representing a
/// fractional area of the root pane container. Used internally by the
/// flat-layout renderer to position leaves and split handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// The rectangle covering the entire container.
    pub(crate) const FULL: Rect = Rect {
        left: 0.0,
        top: 0.0,
        width: 1.0,
        height: 1.0,
    };

    /// Divide this rect by a split at the given direction and ratio. Returns
    /// `(first, second)` — `first` gets `ratio` of the axis that `direction`
    /// splits, `second` gets the remainder.
    pub fn split(&self, direction: SplitDirection, ratio: f64) -> (Rect, Rect) {
        match direction {
            SplitDirection::Horizontal => {
                let first_w = self.width * ratio;
                let first = Rect {
                    left: self.left,
                    top: self.top,
                    width: first_w,
                    height: self.height,
                };
                let second = Rect {
                    left: self.left + first_w,
                    top: self.top,
                    width: self.width - first_w,
                    height: self.height,
                };
                (first, second)
            }
            SplitDirection::Vertical => {
                let first_h = self.height * ratio;
                let first = Rect {
                    left: self.left,
                    top: self.top,
                    width: self.width,
                    height: first_h,
                };
                let second = Rect {
                    left: self.left,
                    top: self.top + first_h,
                    width: self.width,
                    height: self.height - first_h,
                };
                (first, second)
            }
        }
    }
}

/// Walk the tree to find `target`'s rect, given a function that resolves a
/// split's live ratio by its `split_key`.
///
/// The walk only reads ratios on the leaf's ancestor chain, so when this is
/// called from a `Memo`, only those ratios are tracked as dependencies —
/// resizing an unrelated split will not invalidate the memo.
pub fn leaf_rect<D: PaneData>(
    tree: &PaneNode<D>,
    target: &PaneId,
    mut read_ratio: impl FnMut(&PaneId) -> f64,
) -> Option<Rect> {
    fn walk<D: PaneData>(
        node: &PaneNode<D>,
        target: &PaneId,
        rect: Rect,
        read_ratio: &mut dyn FnMut(&PaneId) -> f64,
    ) -> Option<Rect> {
        match node {
            PaneNode::Leaf { id, .. } if id == target => Some(rect),
            PaneNode::Leaf { .. } => None,
            PaneNode::Split {
                direction,
                first,
                second,
                ..
            } => {
                let ratio = read_ratio(second.leftmost_leaf_id());
                let (first_rect, second_rect) = rect.split(*direction, ratio);
                walk(first, target, first_rect, read_ratio)
                    .or_else(|| walk(second, target, second_rect, read_ratio))
            }
        }
    }
    walk(tree, target, Rect::FULL, &mut read_ratio)
}

/// Find the visually nearest pane in `direction` from `target`.
///
/// Candidates that overlap the target on the perpendicular axis are preferred
/// over diagonal candidates, then ranked by distance. Ratios are supplied by
/// the caller so navigation follows live drag-resize state rather than a stale
/// persisted snapshot.
pub fn directional_neighbor<D: PaneData>(
    tree: &PaneNode<D>,
    target: &PaneId,
    direction: PaneDirection,
    mut read_ratio: impl FnMut(&PaneId) -> f64,
) -> Option<PaneId> {
    let ids = tree.leaf_ids();
    let rects: Vec<_> = ids
        .iter()
        .filter_map(|id| leaf_rect(tree, id, &mut read_ratio).map(|rect| (id, rect)))
        .collect();
    let target_rect = rects
        .iter()
        .find(|(id, _)| *id == target)
        .map(|(_, rect)| *rect)?;
    let target_x = target_rect.left + target_rect.width / 2.0;
    let target_y = target_rect.top + target_rect.height / 2.0;

    rects
        .into_iter()
        .filter(|(id, _)| *id != target)
        .filter_map(|(id, rect)| {
            let x = rect.left + rect.width / 2.0;
            let y = rect.top + rect.height / 2.0;
            let eligible = match direction {
                PaneDirection::Left => x < target_x,
                PaneDirection::Right => x > target_x,
                PaneDirection::Up => y < target_y,
                PaneDirection::Down => y > target_y,
            };
            if !eligible {
                return None;
            }

            let (orthogonal_overlap, primary_distance, orthogonal_distance) = match direction {
                PaneDirection::Left | PaneDirection::Right => (
                    interval_overlap(
                        target_rect.top,
                        target_rect.top + target_rect.height,
                        rect.top,
                        rect.top + rect.height,
                    ),
                    (x - target_x).abs(),
                    (y - target_y).abs(),
                ),
                PaneDirection::Up | PaneDirection::Down => (
                    interval_overlap(
                        target_rect.left,
                        target_rect.left + target_rect.width,
                        rect.left,
                        rect.left + rect.width,
                    ),
                    (y - target_y).abs(),
                    (x - target_x).abs(),
                ),
            };
            let diagonal_penalty = usize::from(orthogonal_overlap <= f64::EPSILON);
            Some((
                id.clone(),
                diagonal_penalty,
                primary_distance,
                orthogonal_distance,
            ))
        })
        .min_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.2.total_cmp(&b.2))
                .then_with(|| a.3.total_cmp(&b.3))
        })
        .map(|(id, ..)| id)
}

fn interval_overlap(a_start: f64, a_end: f64, b_start: f64, b_end: f64) -> f64 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0.0)
}

/// Find the nearest ancestor boundary that can grow `target` toward
/// `direction`. The returned sign is applied to that split's ratio.
pub fn resize_boundary<D: PaneData>(
    tree: &PaneNode<D>,
    target: &PaneId,
    direction: PaneDirection,
) -> Option<(PaneId, f64)> {
    fn walk<D: PaneData>(
        node: &PaneNode<D>,
        target: &PaneId,
        direction: PaneDirection,
    ) -> Option<(PaneId, f64)> {
        let PaneNode::Split {
            direction: split_direction,
            first,
            second,
            ..
        } = node
        else {
            return None;
        };

        if first.contains(target) {
            if let Some(inner) = walk(first, target, direction) {
                return Some(inner);
            }
            let grows_toward_boundary = matches!(
                (split_direction, direction),
                (SplitDirection::Horizontal, PaneDirection::Right)
                    | (SplitDirection::Vertical, PaneDirection::Down)
            );
            grows_toward_boundary.then(|| (second.leftmost_leaf_id().clone(), 1.0))
        } else if second.contains(target) {
            if let Some(inner) = walk(second, target, direction) {
                return Some(inner);
            }
            let grows_toward_boundary = matches!(
                (split_direction, direction),
                (SplitDirection::Horizontal, PaneDirection::Left)
                    | (SplitDirection::Vertical, PaneDirection::Up)
            );
            grows_toward_boundary.then(|| (second.leftmost_leaf_id().clone(), -1.0))
        } else {
            None
        }
    }

    walk(tree, target, direction)
}

/// Walk the tree to find the parent rect of the split identified by `split_key`
/// (the first leaf id under the split's `second` subtree).
pub fn split_parent_rect<D: PaneData>(
    tree: &PaneNode<D>,
    split_key: &PaneId,
    mut read_ratio: impl FnMut(&PaneId) -> f64,
) -> Option<Rect> {
    fn walk<D: PaneData>(
        node: &PaneNode<D>,
        split_key: &PaneId,
        rect: Rect,
        read_ratio: &mut dyn FnMut(&PaneId) -> f64,
    ) -> Option<Rect> {
        match node {
            PaneNode::Leaf { .. } => None,
            PaneNode::Split {
                direction,
                first,
                second,
                ..
            } => {
                let this_key = second.leftmost_leaf_id();
                if this_key == split_key {
                    return Some(rect);
                }
                let ratio = read_ratio(this_key);
                let (first_rect, second_rect) = rect.split(*direction, ratio);
                walk(first, split_key, first_rect, read_ratio)
                    .or_else(|| walk(second, split_key, second_rect, read_ratio))
            }
        }
    }
    walk(tree, split_key, Rect::FULL, &mut read_ratio)
}

/// Walk the tree collecting the `split_key` of every split (first leaf id
/// under each split's `second` subtree).
///
/// Used by the renderer's flat layout to enumerate split handles, keyed
/// stably by `split_key` regardless of their direction. Direction is read
/// reactively per-handle via [`find_split_direction`] so a direction change
/// updates the existing handle rather than remounting it.
pub fn collect_split_keys<D: PaneData>(node: &PaneNode<D>) -> Vec<PaneId> {
    let mut out = Vec::new();
    fn walk<D: PaneData>(node: &PaneNode<D>, out: &mut Vec<PaneId>) {
        if let PaneNode::Split { first, second, .. } = node {
            out.push(second.leftmost_leaf_id().clone());
            walk(first, out);
            walk(second, out);
        }
    }
    walk(node, &mut out);
    out
}

/// Find the direction of the split identified by `split_key`.
pub fn find_split_direction<D: PaneData>(
    node: &PaneNode<D>,
    split_key: &PaneId,
) -> Option<SplitDirection> {
    match node {
        PaneNode::Leaf { .. } => None,
        PaneNode::Split {
            direction,
            first,
            second,
            ..
        } => {
            if second.leftmost_leaf_id() == split_key {
                Some(*direction)
            } else {
                find_split_direction(first, split_key)
                    .or_else(|| find_split_direction(second, split_key))
            }
        }
    }
}

/// Walk the tree collecting `(split_key, ratio)` for every split, where
/// `split_key` is the first leaf id under the split's `second` subtree.
pub fn collect_split_ratios<D: PaneData>(node: &PaneNode<D>, out: &mut Vec<(PaneId, f64)>) {
    if let PaneNode::Split {
        ratio,
        first,
        second,
        ..
    } = node
    {
        out.push((second.leftmost_leaf_id().clone(), *ratio));
        collect_split_ratios(first, out);
        collect_split_ratios(second, out);
    }
}
