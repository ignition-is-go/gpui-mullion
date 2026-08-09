use crate::{PaneData, PaneNode};
use serde::{Deserialize, Serialize};
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
impl<D: PaneData> WorkspaceSet<D> {
    pub fn active(&self) -> Option<&Workspace<D>> {
        self.workspaces.iter().find(|w| w.id == self.active)
    }
    pub fn switch(&mut self, id: &WorkspaceId) -> Option<PaneNode<D>> {
        let tree = self.workspaces.iter().find(|w| &w.id == id)?.tree.clone();
        self.active = id.clone();
        Some(tree)
    }

    /// Replace the stored tree of the active workspace.
    pub fn persist_active(&mut self, tree: PaneNode<D>) -> bool {
        let Some(active) = self.workspaces.iter_mut().find(|w| w.id == self.active) else {
            return false;
        };
        active.tree = tree;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(id: &str, pane: &str) -> Workspace<String> {
        Workspace {
            id: WorkspaceId(id.into()),
            name: id.into(),
            tree: PaneNode::leaf(crate::PaneId::new(pane), pane.into()),
        }
    }

    #[test]
    fn switching_and_persisting_are_internal_set_operations() {
        let mut set = WorkspaceSet {
            active: WorkspaceId("one".into()),
            workspaces: vec![workspace("one", "a"), workspace("two", "b")],
        };
        assert!(set.persist_active(PaneNode::leaf(crate::PaneId::new("c"), "c".into())));
        assert_eq!(
            set.switch(&WorkspaceId("two".into())).unwrap().leaf_ids(),
            vec![crate::PaneId::new("b")]
        );
        assert_eq!(
            set.workspaces[0].tree.leaf_ids(),
            vec![crate::PaneId::new("c")]
        );
    }
}
