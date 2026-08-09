use gpui_mullion::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Data {
    project: String,
}
#[test]
fn reference_json_shape_round_trips() {
    let tree = PaneNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.6,
        first: Box::new(PaneNode::leaf_with_activity(
            PaneId::new("a"),
            ActivityId::new("files"),
            Data {
                project: "rship".into(),
            },
        )),
        second: Box::new(PaneNode::leaf(
            PaneId::new("b"),
            Data {
                project: "rship".into(),
            },
        )),
    };
    let value = serde_json::to_value(&tree).unwrap();
    assert_eq!(
        value,
        json!({"Split":{"direction":"Horizontal","ratio":0.6,"first":{"Leaf":{"id":"a","active_activity":"files","data":{"project":"rship"}}},"second":{"Leaf":{"id":"b","active_activity":null,"data":{"project":"rship"}}}}})
    );
    assert_eq!(
        serde_json::from_value::<PaneNode<Data>>(value).unwrap(),
        tree
    );
}
#[test]
fn workspace_switch_returns_portable_snapshot() {
    let tree = PaneNode::leaf(
        PaneId::new("a"),
        Data {
            project: "one".into(),
        },
    );
    let mut set = WorkspaceSet {
        active: WorkspaceId("one".into()),
        workspaces: vec![Workspace {
            id: WorkspaceId("one".into()),
            name: "One".into(),
            tree: tree.clone(),
        }],
    };
    assert_eq!(set.switch(&WorkspaceId("one".into())), Some(tree));
}
