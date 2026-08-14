use crate::{PaneData, PaneNode, PaneValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Stable, host-defined identity for a workspace.
///
/// Mullion treats the wrapped string as opaque and compares it exactly; hosts
/// are responsible for choosing values that remain stable across persistence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

/// A named pane-tree snapshot with a stable identity.
///
/// Within a [`WorkspaceSet`], `id` must be unique and `tree` must satisfy the
/// invariants checked by [`PaneNode::validate`]. Direct construction and
/// deserialization do not enforce either condition; call [`WorkspaceSet::validate`]
/// after loading untrusted persisted data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace<D: PaneData> {
    /// Stable identity used by workspace operations and persistence.
    pub id: WorkspaceId,
    /// User-facing display name; it need not be unique.
    pub name: String,
    /// Pane-tree snapshot restored when this workspace becomes active.
    pub tree: PaneNode<D>,
}

impl WorkspaceId {
    /// Construct a stable workspace identity.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying stable identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WorkspaceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkspaceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for WorkspaceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<D: PaneData> Workspace<D> {
    /// Construct a workspace around a pane-tree snapshot.
    pub fn new(id: impl Into<WorkspaceId>, name: impl Into<String>, tree: PaneNode<D>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            tree,
        }
    }
}

/// An ordered collection of workspaces and its active selection.
///
/// A valid set is nonempty, has unique workspace identifiers, names an
/// existing workspace as active, and contains only valid pane trees. The
/// fields remain public for serialization compatibility, so direct mutation
/// can violate these invariants; use the provided operations or call
/// [`Self::validate`] before relying on the set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSet<D: PaneData> {
    /// Identity of the currently selected workspace.
    pub active: WorkspaceId,
    /// Workspaces in their user-visible order.
    pub workspaces: Vec<Workspace<D>>,
}

/// Consumer-owned persistence hook for mounted workspace changes.
///
/// Mullion owns no storage or async runtime. The callback receives each complete,
/// validated snapshot after a user or view API mutation. Consumers can map it to
/// their server DTO and spawn an asynchronous save through the supplied context.
pub type WorkspaceChangedCallback<D> =
    std::rc::Rc<dyn Fn(WorkspaceSet<D>, &mut gpui::Context<crate::MullionView<D>>)>;

/// Consumer factory invoked by the workspace switcher's add control.
///
/// The application receives the latest complete snapshot and may return a new
/// workspace built from its own IDs, pane defaults, and domain state. Returning
/// `None` cancels the request. Accepted workspaces flow through the ordinary
/// `on_changed` persistence callback.
pub type WorkspaceAddCallback<D> = std::rc::Rc<
    dyn Fn(&WorkspaceSet<D>, &mut gpui::Window, &mut gpui::App) -> Option<Workspace<D>>,
>;

/// Styled workspace-switcher behavior and consumer persistence integration.
pub struct WorkspaceControls<D: PaneData> {
    rename_enabled: bool,
    on_add: Option<WorkspaceAddCallback<D>>,
    on_changed: Option<WorkspaceChangedCallback<D>>,
}

impl<D: PaneData> Clone for WorkspaceControls<D> {
    fn clone(&self) -> Self {
        Self {
            rename_enabled: self.rename_enabled,
            on_add: self.on_add.clone(),
            on_changed: self.on_changed.clone(),
        }
    }
}

impl<D: PaneData> Default for WorkspaceControls<D> {
    fn default() -> Self {
        Self {
            rename_enabled: false,
            on_add: None,
            on_changed: None,
        }
    }
}

impl<D: PaneData> WorkspaceControls<D> {
    /// Enable styled inline renaming by double-click, Enter/F2, and keyboard editing.
    pub fn editable() -> Self {
        Self {
            rename_enabled: true,
            on_add: None,
            on_changed: None,
        }
    }

    /// Enable or disable inline workspace renaming.
    pub fn with_rename_enabled(mut self, enabled: bool) -> Self {
        self.rename_enabled = enabled;
        self
    }

    /// Show an add control and delegate new-workspace construction to the application.
    pub fn on_add_workspace(
        mut self,
        callback: impl Fn(&WorkspaceSet<D>, &mut gpui::Window, &mut gpui::App) -> Option<Workspace<D>>
            + 'static,
    ) -> Self {
        self.on_add = Some(std::rc::Rc::new(callback));
        self
    }

    /// Receive complete validated snapshots after accepted local mutations.
    pub fn on_changed(
        mut self,
        callback: impl Fn(WorkspaceSet<D>, &mut gpui::Context<crate::MullionView<D>>) + 'static,
    ) -> Self {
        self.on_changed = Some(std::rc::Rc::new(callback));
        self
    }

    /// Return whether inline renaming is enabled.
    pub const fn rename_enabled(&self) -> bool {
        self.rename_enabled
    }

    pub(crate) fn add_callback(&self) -> Option<&WorkspaceAddCallback<D>> {
        self.on_add.as_ref()
    }

    pub(crate) fn changed_callback(&self) -> Option<&WorkspaceChangedCallback<D>> {
        self.on_changed.as_ref()
    }
}

/// An invariant or requested-operation error for a [`WorkspaceSet`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WorkspaceSetError {
    /// The set contains no workspaces.
    Empty,
    /// Two workspaces use the same stable identifier.
    DuplicateWorkspaceId {
        /// Identifier shared by both workspaces.
        id: WorkspaceId,
        /// Zero-based position of the first occurrence.
        first_index: usize,
        /// Zero-based position of the later occurrence.
        duplicate_index: usize,
    },
    /// The selected workspace identifier is absent from the set.
    ActiveWorkspaceNotFound {
        /// Missing identifier stored in [`WorkspaceSet::active`].
        active: WorkspaceId,
    },
    /// A workspace contains a pane tree that failed validation.
    InvalidPaneTree {
        /// Identifier of the workspace containing the invalid tree.
        workspace_id: WorkspaceId,
        /// Underlying pane-tree validation failure.
        source: PaneValidationError,
    },
    /// An operation named an identifier that is absent from the set.
    WorkspaceNotFound {
        /// Identifier requested by the operation.
        id: WorkspaceId,
    },
    /// Removal was rejected because the requested workspace is active.
    CannotRemoveActive {
        /// Identifier of the active workspace.
        id: WorkspaceId,
    },
    /// A requested reorder position lies outside the workspace vector.
    ReorderIndexOutOfBounds {
        /// Requested zero-based destination position.
        index: usize,
        /// Number of workspaces, and therefore the exclusive upper bound.
        len: usize,
    },
}

impl fmt::Display for WorkspaceSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a workspace set must contain at least one workspace"),
            Self::DuplicateWorkspaceId {
                id,
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "duplicate workspace id {:?} at index {duplicate_index} (first seen at index {first_index})",
                id.0
            ),
            Self::ActiveWorkspaceNotFound { active } => {
                write!(formatter, "active workspace {:?} is not in the set", active.0)
            }
            Self::InvalidPaneTree {
                workspace_id,
                source,
            } => write!(
                formatter,
                "workspace {:?} has an invalid pane tree: {source}",
                workspace_id.0
            ),
            Self::WorkspaceNotFound { id } => {
                write!(formatter, "workspace {:?} is not in the set", id.0)
            }
            Self::CannotRemoveActive { id } => {
                write!(formatter, "cannot remove active workspace {:?}", id.0)
            }
            Self::ReorderIndexOutOfBounds { index, len } => write!(
                formatter,
                "workspace reorder index {index} is out of bounds for {len} workspaces"
            ),
        }
    }
}

impl std::error::Error for WorkspaceSetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPaneTree { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl<D: PaneData> WorkspaceSet<D> {
    /// Construct a workspace set after validating all persistence invariants.
    ///
    /// # Errors
    ///
    /// Returns the applicable invariant error when the collection is empty,
    /// identifiers are duplicated, `active` is absent, or a pane tree is invalid.
    pub fn try_new(
        active: WorkspaceId,
        workspaces: Vec<Workspace<D>>,
    ) -> Result<Self, WorkspaceSetError> {
        let set = Self { active, workspaces };
        set.validate()?;
        Ok(set)
    }

    /// Validate the set without changing it.
    ///
    /// # Errors
    ///
    /// Returns the first violated set or pane-tree invariant. Workspaces and
    /// duplicate identifiers are examined in vector order.
    pub fn validate(&self) -> Result<(), WorkspaceSetError> {
        if self.workspaces.is_empty() {
            return Err(WorkspaceSetError::Empty);
        }

        let mut ids = HashMap::with_capacity(self.workspaces.len());
        for (index, workspace) in self.workspaces.iter().enumerate() {
            if let Some(first_index) = ids.insert(&workspace.id, index) {
                return Err(WorkspaceSetError::DuplicateWorkspaceId {
                    id: workspace.id.clone(),
                    first_index,
                    duplicate_index: index,
                });
            }
            workspace
                .tree
                .validate()
                .map_err(|source| WorkspaceSetError::InvalidPaneTree {
                    workspace_id: workspace.id.clone(),
                    source,
                })?;
        }

        if !ids.contains_key(&self.active) {
            return Err(WorkspaceSetError::ActiveWorkspaceNotFound {
                active: self.active.clone(),
            });
        }
        Ok(())
    }

    /// Return the selected workspace, if `active` names an entry in the set.
    ///
    /// A validated set always returns `Some`; `None` is possible because the
    /// public fields and deserialization can construct an invalid set.
    pub fn active(&self) -> Option<&Workspace<D>> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == self.active)
    }

    /// Append a valid workspace and return its zero-based index.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error, [`WorkspaceSetError::DuplicateWorkspaceId`],
    /// or [`WorkspaceSetError::InvalidPaneTree`]. The set is unchanged on error.
    pub fn add(&mut self, workspace: Workspace<D>) -> Result<usize, WorkspaceSetError> {
        self.validate()?;
        if let Some((first_index, _)) = self
            .workspaces
            .iter()
            .enumerate()
            .find(|(_, existing)| existing.id == workspace.id)
        {
            return Err(WorkspaceSetError::DuplicateWorkspaceId {
                id: workspace.id,
                first_index,
                duplicate_index: self.workspaces.len(),
            });
        }
        workspace
            .tree
            .validate()
            .map_err(|source| WorkspaceSetError::InvalidPaneTree {
                workspace_id: workspace.id.clone(),
                source,
            })?;
        let index = self.workspaces.len();
        self.workspaces.push(workspace);
        Ok(index)
    }

    /// Remove a non-active workspace, returning the complete removed value.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error, [`WorkspaceSetError::CannotRemoveActive`],
    /// or [`WorkspaceSetError::WorkspaceNotFound`]. The set is unchanged on error.
    pub fn remove(&mut self, id: &WorkspaceId) -> Result<Workspace<D>, WorkspaceSetError> {
        self.validate()?;
        if id == &self.active {
            return Err(WorkspaceSetError::CannotRemoveActive { id: id.clone() });
        }
        let index = self
            .workspaces
            .iter()
            .position(|workspace| &workspace.id == id)
            .ok_or_else(|| WorkspaceSetError::WorkspaceNotFound { id: id.clone() })?;
        Ok(self.workspaces.remove(index))
    }

    /// Rename a workspace, returning its previous name.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error or [`WorkspaceSetError::WorkspaceNotFound`].
    /// The set is unchanged on error.
    pub fn rename(
        &mut self,
        id: &WorkspaceId,
        name: impl Into<String>,
    ) -> Result<String, WorkspaceSetError> {
        self.validate()?;
        let workspace = self
            .workspaces
            .iter_mut()
            .find(|workspace| &workspace.id == id)
            .ok_or_else(|| WorkspaceSetError::WorkspaceNotFound { id: id.clone() })?;
        Ok(std::mem::replace(&mut workspace.name, name.into()))
    }

    /// Replace any workspace tree after validation, returning the old tree.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error, [`WorkspaceSetError::InvalidPaneTree`],
    /// or [`WorkspaceSetError::WorkspaceNotFound`]. The set is unchanged on error.
    pub fn update_tree(
        &mut self,
        id: &WorkspaceId,
        tree: PaneNode<D>,
    ) -> Result<PaneNode<D>, WorkspaceSetError> {
        self.validate()?;
        tree.validate()
            .map_err(|source| WorkspaceSetError::InvalidPaneTree {
                workspace_id: id.clone(),
                source,
            })?;
        let workspace = self
            .workspaces
            .iter_mut()
            .find(|workspace| &workspace.id == id)
            .ok_or_else(|| WorkspaceSetError::WorkspaceNotFound { id: id.clone() })?;
        Ok(std::mem::replace(&mut workspace.tree, tree))
    }

    /// Move a workspace to zero-based `index`, returning its previous index.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error,
    /// [`WorkspaceSetError::ReorderIndexOutOfBounds`] when `index >= len`, or
    /// [`WorkspaceSetError::WorkspaceNotFound`]. The order is unchanged on error.
    pub fn reorder(&mut self, id: &WorkspaceId, index: usize) -> Result<usize, WorkspaceSetError> {
        self.validate()?;
        let len = self.workspaces.len();
        if index >= len {
            return Err(WorkspaceSetError::ReorderIndexOutOfBounds { index, len });
        }
        let previous = self
            .workspaces
            .iter()
            .position(|workspace| &workspace.id == id)
            .ok_or_else(|| WorkspaceSetError::WorkspaceNotFound { id: id.clone() })?;
        if previous != index {
            let workspace = self.workspaces.remove(previous);
            self.workspaces.insert(index, workspace);
        }
        Ok(previous)
    }

    /// Select a workspace and return a clone of its stored pane-tree snapshot.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error or [`WorkspaceSetError::WorkspaceNotFound`].
    /// The active selection is unchanged on error.
    pub fn try_switch(&mut self, id: &WorkspaceId) -> Result<PaneNode<D>, WorkspaceSetError> {
        self.validate()?;
        let tree = self
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .ok_or_else(|| WorkspaceSetError::WorkspaceNotFound { id: id.clone() })?
            .tree
            .clone();
        self.active = id.clone();
        Ok(tree)
    }

    /// Replace the active workspace's tree and return its previous snapshot.
    ///
    /// # Errors
    ///
    /// Returns an existing-set validation error or a tree validation error.
    /// The stored snapshot is unchanged on error.
    pub fn try_persist_active(
        &mut self,
        tree: PaneNode<D>,
    ) -> Result<PaneNode<D>, WorkspaceSetError> {
        let active = self.active.clone();
        self.update_tree(&active, tree)
    }

    /// Persist a snapshot produced by an already-validated [`crate::MullionModel`].
    ///
    /// This deliberately skips whole-set and tree validation. It is crate-private
    /// because accepting arbitrary host data here would violate `WorkspaceSet`'s
    /// invariants; the view uses it only for snapshots from model mutations, which
    /// preserve those invariants by construction.
    pub(crate) fn persist_model_snapshot(&mut self, tree: PaneNode<D>) -> bool {
        let Some(workspace) = self
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.id == self.active)
        else {
            return false;
        };
        workspace.tree = tree;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PaneId, SplitDirection};

    fn workspace(id: &str, pane: &str) -> Workspace<String> {
        Workspace {
            id: WorkspaceId(id.into()),
            name: id.into(),
            tree: PaneNode::leaf(PaneId::new(pane), pane.into()),
        }
    }

    fn set() -> WorkspaceSet<String> {
        WorkspaceSet::try_new(
            WorkspaceId("one".into()),
            vec![workspace("one", "a"), workspace("two", "b")],
        )
        .unwrap()
    }

    fn invalid_tree() -> PaneNode<String> {
        PaneNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(PaneId::new("duplicate"), "a".into())),
            second: Box::new(PaneNode::leaf(PaneId::new("duplicate"), "b".into())),
        }
    }

    #[test]
    fn workspace_builder_accepts_string_identity() {
        let workspace = Workspace::new(
            "one",
            "One",
            PaneNode::leaf(PaneId::new("pane"), "data".to_owned()),
        );
        assert_eq!(workspace.id.as_str(), "one");
        assert_eq!(workspace.id.to_string(), "one");
        assert_eq!(workspace.name, "One");
    }

    #[test]
    fn validation_rejects_empty_invalid_active_duplicates_and_bad_trees() {
        assert_eq!(
            WorkspaceSet::<String>::try_new(WorkspaceId("none".into()), vec![]),
            Err(WorkspaceSetError::Empty)
        );
        assert!(matches!(
            WorkspaceSet::try_new(WorkspaceId("missing".into()), vec![workspace("one", "a")]),
            Err(WorkspaceSetError::ActiveWorkspaceNotFound { .. })
        ));
        assert!(matches!(
            WorkspaceSet::try_new(
                WorkspaceId("one".into()),
                vec![workspace("one", "a"), workspace("one", "b")]
            ),
            Err(WorkspaceSetError::DuplicateWorkspaceId {
                first_index: 0,
                duplicate_index: 1,
                ..
            })
        ));
        let mut bad = workspace("one", "a");
        bad.tree = invalid_tree();
        assert!(matches!(
            WorkspaceSet::try_new(WorkspaceId("one".into()), vec![bad]),
            Err(WorkspaceSetError::InvalidPaneTree { .. })
        ));
    }

    #[test]
    fn switch_same_and_invalid_are_atomic() {
        let mut set = set();
        let before = set.clone();
        assert_eq!(
            set.try_switch(&WorkspaceId("one".into()))
                .unwrap()
                .leaf_ids(),
            vec![PaneId::new("a")]
        );
        assert_eq!(set, before);
        assert!(matches!(
            set.try_switch(&WorkspaceId("missing".into())),
            Err(WorkspaceSetError::WorkspaceNotFound { .. })
        ));
        assert_eq!(set, before);
        assert!(set.try_switch(&WorkspaceId("missing".into())).is_err());
        assert_eq!(set, before);
    }

    #[test]
    fn active_remove_is_refused_without_mutation() {
        let mut set = set();
        let before = set.clone();
        assert!(matches!(
            set.remove(&WorkspaceId("one".into())),
            Err(WorkspaceSetError::CannotRemoveActive { .. })
        ));
        assert_eq!(set, before);
    }

    #[test]
    fn add_remove_rename_reorder_and_update_return_previous_values() {
        let mut set = set();
        assert_eq!(set.add(workspace("three", "c")), Ok(2));
        assert_eq!(
            set.rename(&WorkspaceId("three".into()), "renamed").unwrap(),
            "three"
        );
        assert_eq!(set.reorder(&WorkspaceId("three".into()), 0), Ok(2));
        assert_eq!(set.workspaces[0].name, "renamed");

        let replacement = PaneNode::leaf(PaneId::new("new"), "new".into());
        let old = set
            .update_tree(&WorkspaceId("three".into()), replacement)
            .unwrap();
        assert_eq!(old.leaf_ids(), vec![PaneId::new("c")]);
        let removed = set.remove(&WorkspaceId("three".into())).unwrap();
        assert_eq!(removed.id, WorkspaceId("three".into()));
        set.validate().unwrap();
    }

    #[test]
    fn mutations_reject_invalid_input_atomically() {
        let mut set = set();
        let before = set.clone();
        assert!(matches!(
            set.add(workspace("one", "c")),
            Err(WorkspaceSetError::DuplicateWorkspaceId { .. })
        ));
        assert_eq!(set, before);
        assert!(matches!(
            set.update_tree(&WorkspaceId("one".into()), invalid_tree()),
            Err(WorkspaceSetError::InvalidPaneTree { .. })
        ));
        assert_eq!(set, before);
        assert!(matches!(
            set.reorder(&WorkspaceId("one".into()), 2),
            Err(WorkspaceSetError::ReorderIndexOutOfBounds { .. })
        ));
        assert_eq!(set, before);
    }

    #[test]
    fn switching_and_persisting_are_internal_set_operations() {
        let mut set = set();
        assert!(set
            .try_persist_active(PaneNode::leaf(PaneId::new("c"), "c".into()))
            .is_ok());
        assert_eq!(
            set.try_switch(&WorkspaceId("two".into()))
                .unwrap()
                .leaf_ids(),
            vec![PaneId::new("b")]
        );
        assert_eq!(set.workspaces[0].tree.leaf_ids(), vec![PaneId::new("c")]);
        let before = set.clone();
        assert!(set.try_persist_active(invalid_tree()).is_err());
        assert_eq!(set, before);
    }

    #[test]
    fn serde_round_trip_preserves_the_public_shape() {
        let set = set();
        let json = serde_json::to_value(&set).unwrap();
        assert_eq!(json["active"], serde_json::json!("one"));
        assert!(json.get("workspaces").is_some());
        let round_trip: WorkspaceSet<String> = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip, set);
        round_trip.validate().unwrap();
    }
}
