use crate::{ActivityId, CategoryId, PaneData, PaneId};
use gpui::{AnyElement, SharedString};
use std::sync::Arc;

pub type ActivityRenderer<D> = Arc<dyn Fn(&PaneId, &D) -> AnyElement + Send + Sync>;

/// A native activity definition. Rendering stays adapter-specific; IDs and pane state remain portable.
#[derive(Clone)]
pub struct Activity<D: PaneData> {
    pub id: ActivityId,
    pub name: SharedString,
    pub filter: fn(&D) -> bool,
    pub render: ActivityRenderer<D>,
}

#[derive(Clone)]
pub struct ActivityCategory<D: PaneData> {
    pub id: CategoryId,
    pub name: SharedString,
    pub color: gpui::Hsla,
    pub children: Vec<ActivityNode<D>>,
}

#[derive(Clone)]
pub enum ActivityNode<D: PaneData> {
    Activity(Activity<D>),
    Category(ActivityCategory<D>),
}

impl<D: PaneData> ActivityNode<D> {
    pub(crate) fn activities<'a>(&'a self, data: &D, out: &mut Vec<&'a Activity<D>>) {
        match self {
            Self::Activity(a) if (a.filter)(data) => out.push(a),
            Self::Category(c) => {
                for n in &c.children {
                    n.activities(data, out)
                }
            }
            _ => {}
        }
    }
}
