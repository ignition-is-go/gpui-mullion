use gpui::{div, IntoElement, ParentElement, TestAppContext};
use gpui_mullion::{
    Activity, ActivityId, ActivityNode, MullionView, PaneId, PaneNode, SplitDirection, Workspace,
    WorkspaceId, WorkspaceSet,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

static FILTER_CALLS: AtomicUsize = AtomicUsize::new(0);

fn counted_visible(_: &String) -> bool {
    FILTER_CALLS.fetch_add(1, Ordering::SeqCst);
    true
}

fn pane_tree(workspace: usize, start: usize, count: usize, depth: usize) -> PaneNode<String> {
    if count == 1 {
        return PaneNode::leaf_with_activity(
            PaneId::new(format!("w{workspace}-p{start}")),
            ActivityId::new("activity-0"),
            format!("data-{workspace}-{start}"),
        );
    }

    let first_count = count / 2;
    PaneNode::Split {
        direction: if depth.is_multiple_of(2) {
            SplitDirection::Horizontal
        } else {
            SplitDirection::Vertical
        },
        ratio: 0.5,
        first: Box::new(pane_tree(workspace, start, first_count, depth + 1)),
        second: Box::new(pane_tree(
            workspace,
            start + first_count,
            count - first_count,
            depth + 1,
        )),
    }
}

fn workspace_set(panes: usize, workspace_count: usize) -> WorkspaceSet<String> {
    let base = panes / workspace_count;
    let remainder = panes % workspace_count;
    let workspaces = (0..workspace_count)
        .map(|workspace| {
            let count = base + usize::from(workspace < remainder);
            Workspace::new(
                WorkspaceId::new(format!("workspace-{workspace}")),
                format!("Workspace {workspace}"),
                pane_tree(workspace, 0, count, 0),
            )
        })
        .collect();
    WorkspaceSet::try_new(WorkspaceId::new("workspace-0"), workspaces).unwrap()
}

fn activities(count: usize) -> Vec<ActivityNode<String>> {
    (0..count)
        .map(|index| {
            ActivityNode::Activity(Activity {
                id: ActivityId::new(format!("activity-{index}")),
                name: format!("Activity {index}").into(),
                filter: counted_visible,
                render: Arc::new(|_, _| div().child("stable").into_any_element()),
            })
        })
        .collect()
}

/// Deterministic work accounting for activity-cache reconciliation. Each dirty
/// synchronization visits every pane in every workspace and applies each
/// activity filter twice: once while collecting valid cache keys and once while
/// building the render projection. The first render also projects the active
/// panes once before the deferred cache exists. Clean renders and unattached
/// command-palette resynchronization must do neither.
#[gpui::test]
fn render_cache_synchronization_scales_with_panes_activities_and_workspaces(
    cx: &mut TestAppContext,
) {
    for (panes, workspace_count, activity_count) in [(1, 1, 1), (8, 2, 3), (29, 1, 5), (128, 4, 2)]
    {
        FILTER_CALLS.store(0, Ordering::SeqCst);
        let workspaces = workspace_set(panes, workspace_count);
        let catalog = activities(activity_count);
        let (view, cx) = cx.add_window_view(move |_, cx| {
            MullionView::try_new_with_workspaces(workspaces, catalog, cx).unwrap()
        });
        cx.run_until_parked();

        let one_sync = panes * activity_count * 2;
        let active_panes = panes / workspace_count + usize::from(panes % workspace_count > 0);
        let initial_work = one_sync + active_panes * activity_count;
        assert_eq!(
            FILTER_CALLS.load(Ordering::SeqCst),
            initial_work,
            "initial synchronization for {panes} panes, {workspace_count} workspaces, {activity_count} activities"
        );

        for _ in 0..3 {
            view.update(cx, |_, cx| cx.notify());
            cx.run_until_parked();
        }
        assert_eq!(
            FILTER_CALLS.load(Ordering::SeqCst),
            initial_work,
            "clean renders revisited the catalog for {panes} panes"
        );

        if workspace_count > 1 {
            view.update(cx, |view, cx| {
                assert!(view
                    .try_switch_workspace(&WorkspaceId::new("workspace-1"), cx)
                    .unwrap());
            });
            cx.run_until_parked();
            assert_eq!(
                FILTER_CALLS.load(Ordering::SeqCst),
                initial_work + one_sync,
                "workspace switch did not perform exactly one full synchronization for {panes} panes"
            );
        }
    }
}
