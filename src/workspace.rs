use crate::{PaneData, PaneNode, PaneValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Workspace<D: PaneData> {
    pub id: WorkspaceId,
    pub name: String,
    pub tree: PaneNode<D>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct WorkspaceSet<D: PaneData> {
    pub active: WorkspaceId,
    pub workspaces: Vec<Workspace<D>>,
}

/// An invariant or requested-operation error for a [`WorkspaceSet`].
#[derive(Clone, Debug, PartialEq)]
pub enum WorkspaceSetError {
    Empty,
    DuplicateWorkspaceId {
        id: WorkspaceId,
        first_index: usize,
        duplicate_index: usize,
    },
    ActiveWorkspaceNotFound {
        active: WorkspaceId,
    },
    InvalidPaneTree {
        workspace_id: WorkspaceId,
        source: PaneValidationError,
    },
    WorkspaceNotFound {
        id: WorkspaceId,
    },
    CannotRemoveActive {
        id: WorkspaceId,
    },
    ReorderIndexOutOfBounds {
        index: usize,
        len: usize,
    },
}

/// Shorter name for callers which use one error type for workspace operations.
pub type WorkspaceError = WorkspaceSetError;
/// Validation-specific name for persistence-loading call sites.
pub type WorkspaceValidationError = WorkspaceSetError;

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
    pub fn try_new(
        active: WorkspaceId,
        workspaces: Vec<Workspace<D>>,
    ) -> Result<Self, WorkspaceSetError> {
        let set = Self { active, workspaces };
        set.validate()?;
        Ok(set)
    }

    /// Validate the set without changing it.
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

    pub fn active(&self) -> Option<&Workspace<D>> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == self.active)
    }

    /// Append a valid workspace and return its index.
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

    /// Move a workspace to `index`, returning its previous index.
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

    /// Typed switching API. The returned tree is the new active snapshot.
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

    /// Switch while retaining the original optional API.
    pub fn switch(&mut self, id: &WorkspaceId) -> Option<PaneNode<D>> {
        self.try_switch(id).ok()
    }

    /// Typed persistence API for the active workspace.
    pub fn try_persist_active(
        &mut self,
        tree: PaneNode<D>,
    ) -> Result<PaneNode<D>, WorkspaceSetError> {
        let active = self.active.clone();
        self.update_tree(&active, tree)
    }

    /// Replace the stored tree of the active workspace, retaining the original bool API.
    pub fn persist_active(&mut self, tree: PaneNode<D>) -> bool {
        self.try_persist_active(tree).is_ok()
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
        assert!(set.switch(&WorkspaceId("missing".into())).is_none());
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
        assert!(set.persist_active(PaneNode::leaf(PaneId::new("c"), "c".into())));
        assert_eq!(
            set.switch(&WorkspaceId("two".into())).unwrap().leaf_ids(),
            vec![PaneId::new("b")]
        );
        assert_eq!(set.workspaces[0].tree.leaf_ids(), vec![PaneId::new("c")]);
        let before = set.clone();
        assert!(!set.persist_active(invalid_tree()));
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
