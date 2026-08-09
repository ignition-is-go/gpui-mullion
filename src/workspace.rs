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
}
