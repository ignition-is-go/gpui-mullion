use crate::{Activity, ActivityId, ActivityNode, CategoryId, PaneData, PaneId};
use gpui::{AnyElement, App, Hsla, Window};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

/// A GPUI-native activity icon.
///
/// Hosts provide an element factory rather than CSS classes, SVG markup, or a
/// web URL. This keeps Mullion independent of Zed's private `ui` crate and lets
/// applications use GPUI's `svg`, `img`, text, or a custom element.
type ActivityIconRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Cloneable UI-local factory for one activity or category icon element.
/// Cloneable UI-local factory for one activity or category icon element.
/// Cloneable UI-local factory for one activity or category icon element.
#[derive(Clone)]
pub struct ActivityIcon(ActivityIconRenderer);

impl ActivityIcon {
    /// Wrap an element factory invoked when icon chrome is projected.
    /// Wrap an element factory invoked when icon chrome is projected.
    /// Wrap an element factory invoked when icon chrome is projected.
    pub fn new(render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self(Rc::new(render))
    }

    /// Invoke the host factory in the current window and application contexts.
    /// Invoke the host factory in the current window and application contexts.
    /// Invoke the host factory in the current window and application contexts.
    pub fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        (self.0)(window, cx)
    }
}

impl fmt::Debug for ActivityIcon {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ActivityIcon(..)")
    }
}

/// UI-local host chrome rendered with the current pane and its data.
pub type ChromeRenderer<D> = Rc<dyn Fn(&PaneId, &D, &mut Window, &mut App) -> AnyElement>;

/// Additive visual metadata for a legacy [`Activity`].
#[derive(Clone)]
pub struct ActivityChrome<D: PaneData> {
    /// Optional icon displayed by the activity bar.
    pub icon: Option<ActivityIcon>,
    /// Optional pane-header chrome rendered with current pane data.
    /// Optional pane-header chrome rendered with current pane data.
    pub header: Option<ChromeRenderer<D>>,
}

impl<D: PaneData> Default for ActivityChrome<D> {
    fn default() -> Self {
        Self {
            icon: None,
            header: None,
        }
    }
}

impl<D: PaneData> ActivityChrome<D> {
    /// Construct chrome with an icon and no custom header.
    pub fn new(icon: ActivityIcon) -> Self {
        Self {
            icon: Some(icon),
            header: None,
        }
    }

    /// Set or replace the icon factory.
    pub fn with_icon(mut self, icon: ActivityIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Install UI-local header chrome rendered for each owning pane.
    pub fn with_header(
        mut self,
        render: impl Fn(&PaneId, &D, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.header = Some(Rc::new(render));
        self
    }
}

/// Additive visual metadata for a legacy [`crate::ActivityCategory`].
#[derive(Clone, Default)]
pub struct CategoryChrome {
    /// Optional icon displayed beside the category label.
    pub icon: Option<ActivityIcon>,
    /// Overrides the category's legacy `color` when present.
    pub color: Option<Hsla>,
}

impl CategoryChrome {
    /// Construct chrome with an icon and no color override.
    pub fn new(icon: ActivityIcon) -> Self {
        Self {
            icon: Some(icon),
            color: None,
        }
    }

    /// Set or replace the icon factory.
    pub fn with_icon(mut self, icon: ActivityIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Override the legacy category color used by descendants.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Ordered activity information architecture plus additive GPUI chrome.
///
/// The existing `Vec<ActivityNode<D>>` constructor path remains valid. Hosts
/// may migrate by wrapping that vector with [`Self::new`] and adding chrome one
/// entry at a time.
#[derive(Clone)]
pub struct ActivityCatalog<D: PaneData> {
    primary: Vec<ActivityNode<D>>,
    trailing: Vec<ActivityNode<D>>,
    activity_chrome: HashMap<ActivityId, ActivityChrome<D>>,
    category_chrome: HashMap<CategoryId, CategoryChrome>,
}

impl<D: PaneData> Default for ActivityCatalog<D> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<D: PaneData> From<Vec<ActivityNode<D>>> for ActivityCatalog<D> {
    fn from(primary: Vec<ActivityNode<D>>) -> Self {
        Self::new(primary)
    }
}

impl<D: PaneData> ActivityCatalog<D> {
    /// Construct a catalog whose nodes occupy the leading group.
    pub fn new(primary: Vec<ActivityNode<D>>) -> Self {
        Self {
            primary,
            trailing: Vec::new(),
            activity_chrome: HashMap::new(),
            category_chrome: HashMap::new(),
        }
    }

    /// Set nodes anchored at the trailing end of the activity bar.
    pub fn with_trailing(mut self, trailing: Vec<ActivityNode<D>>) -> Self {
        self.trailing = trailing;
        self
    }

    /// Associate additive chrome with a registered activity id.
    pub fn with_activity_chrome(mut self, id: ActivityId, chrome: ActivityChrome<D>) -> Self {
        self.activity_chrome.insert(id, chrome);
        self
    }

    /// Associate additive chrome with a registered category id.
    pub fn with_category_chrome(mut self, id: CategoryId, chrome: CategoryChrome) -> Self {
        self.category_chrome.insert(id, chrome);
        self
    }

    /// Insert activity chrome, returning the previous entry for the id.
    pub fn insert_activity_chrome(
        &mut self,
        id: ActivityId,
        chrome: ActivityChrome<D>,
    ) -> Option<ActivityChrome<D>> {
        self.activity_chrome.insert(id, chrome)
    }

    /// Insert category chrome, returning the previous entry for the id.
    pub fn insert_category_chrome(
        &mut self,
        id: CategoryId,
        chrome: CategoryChrome,
    ) -> Option<CategoryChrome> {
        self.category_chrome.insert(id, chrome)
    }

    /// Borrow the recursively ordered leading group.
    pub fn primary(&self) -> &[ActivityNode<D>] {
        &self.primary
    }

    /// Borrow the recursively ordered trailing group.
    pub fn trailing(&self) -> &[ActivityNode<D>] {
        &self.trailing
    }

    /// Return additive chrome registered for an activity.
    pub fn activity_chrome(&self, id: &ActivityId) -> Option<&ActivityChrome<D>> {
        self.activity_chrome.get(id)
    }

    /// Return additive chrome registered for a category.
    pub fn category_chrome(&self, id: &CategoryId) -> Option<&CategoryChrome> {
        self.category_chrome.get(id)
    }

    /// Validate stable identity across both groups and every chrome map key.
    ///
    /// Chrome is additive, so a registered node need not have a chrome entry.
    /// A chrome entry, however, must name a registered node; otherwise it can
    /// never be rendered and usually indicates a renamed or mistyped id.
    pub fn validate(&self) -> Result<(), ActivityCatalogValidationError> {
        let mut activities = HashSet::new();
        let mut categories = HashSet::new();
        for (group, nodes) in [
            (ActivityCatalogGroup::Primary, self.primary.as_slice()),
            (ActivityCatalogGroup::Trailing, self.trailing.as_slice()),
        ] {
            validate_nodes(nodes, group, &mut activities, &mut categories)?;
        }
        for id in self.activity_chrome.keys() {
            if !activities.contains(id) {
                return Err(ActivityCatalogValidationError::MissingActivityChromeKey {
                    activity_id: id.clone(),
                });
            }
        }
        for id in self.category_chrome.keys() {
            if !categories.contains(id) {
                return Err(ActivityCatalogValidationError::MissingCategoryChromeKey {
                    category_id: id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Project both ordered trees for one pane.
    pub fn visible(&self, data: &D, active: Option<&ActivityId>) -> ActivityProjection<D> {
        let mut active_ancestors = Vec::new();
        let primary = project_nodes(
            &self.primary,
            data,
            active,
            None,
            &mut Vec::new(),
            &mut active_ancestors,
            &self.category_chrome,
        );
        let trailing = project_nodes(
            &self.trailing,
            data,
            active,
            None,
            &mut Vec::new(),
            &mut active_ancestors,
            &self.category_chrome,
        );
        ActivityProjection {
            primary,
            trailing,
            active_ancestors,
        }
    }
}

/// Top-level placement group containing a catalog node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActivityCatalogGroup {
    /// Leading group, following the configured activity-bar direction.
    Primary,
    /// Group anchored at the opposite end of the bar.
    Trailing,
}

/// Stable-identity or chrome-reference validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityCatalogValidationError {
    /// An activity id occurs more than once across the recursive catalog.
    DuplicateActivityId {
        /// Repeated stable activity identity.
        activity_id: ActivityId,
        /// Top-level group containing the later occurrence.
        duplicate_group: ActivityCatalogGroup,
    },
    /// A category id occurs more than once across the recursive catalog.
    DuplicateCategoryId {
        /// Repeated stable category identity.
        category_id: CategoryId,
        /// Top-level group containing the later occurrence.
        duplicate_group: ActivityCatalogGroup,
    },
    /// An activity chrome map key has no corresponding registered activity.
    MissingActivityChromeKey {
        /// Chrome key without a corresponding activity node.
        activity_id: ActivityId,
    },
    /// A category chrome map key has no corresponding registered category.
    MissingCategoryChromeKey {
        /// Chrome key without a corresponding category node.
        category_id: CategoryId,
    },
}

impl fmt::Display for ActivityCatalogValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateActivityId { activity_id, .. } => {
                write!(formatter, "duplicate activity id {:?}", activity_id.0)
            }
            Self::DuplicateCategoryId { category_id, .. } => {
                write!(formatter, "duplicate category id {:?}", category_id.0)
            }
            Self::MissingActivityChromeKey { activity_id } => write!(
                formatter,
                "activity chrome key {:?} has no registered activity",
                activity_id.0
            ),
            Self::MissingCategoryChromeKey { category_id } => write!(
                formatter,
                "category chrome key {:?} has no registered category",
                category_id.0
            ),
        }
    }
}

impl std::error::Error for ActivityCatalogValidationError {}

fn validate_nodes<D: PaneData>(
    nodes: &[ActivityNode<D>],
    group: ActivityCatalogGroup,
    activities: &mut HashSet<ActivityId>,
    categories: &mut HashSet<CategoryId>,
) -> Result<(), ActivityCatalogValidationError> {
    for node in nodes {
        match node {
            ActivityNode::Activity(activity) => {
                if !activities.insert(activity.id.clone()) {
                    return Err(ActivityCatalogValidationError::DuplicateActivityId {
                        activity_id: activity.id.clone(),
                        duplicate_group: group,
                    });
                }
            }
            ActivityNode::Category(category) => {
                if !categories.insert(category.id.clone()) {
                    return Err(ActivityCatalogValidationError::DuplicateCategoryId {
                        category_id: category.id.clone(),
                        duplicate_group: group,
                    });
                }
                validate_nodes(&category.children, group, activities, categories)?;
            }
        }
    }
    Ok(())
}

/// Per-pane, recursively filtered projection of an activity catalog.
#[derive(Clone)]
pub struct ActivityProjection<D: PaneData> {
    /// Visible leading nodes in catalog order.
    pub primary: Vec<VisibleActivityNode<D>>,
    /// Visible trailing nodes in catalog order.
    pub trailing: Vec<VisibleActivityNode<D>>,
    /// Ancestor categories of the active visible activity, outermost first.
    pub active_ancestors: Vec<CategoryId>,
}

/// Visible recursive node produced for one pane's data.
#[derive(Clone)]
pub enum VisibleActivityNode<D: PaneData> {
    /// Selectable activity leaf.
    Activity(VisibleActivity<D>),
    /// Nonempty visible category branch.
    Category(VisibleCategory<D>),
}

/// Activity leaf retained by a per-pane projection.
#[derive(Clone)]
pub struct VisibleActivity<D: PaneData> {
    /// Complete host activity descriptor.
    pub activity: Activity<D>,
    /// The nearest enclosing category color, after chrome overrides.
    pub inherited_color: Option<Hsla>,
}

/// Visible category whose filtered child list is nonempty.
#[derive(Clone)]
pub struct VisibleCategory<D: PaneData> {
    /// Stable category identity.
    pub id: CategoryId,
    /// Display label.
    pub name: gpui::SharedString,
    /// Effective color after applying catalog chrome overrides.
    pub color: Hsla,
    /// Recursively visible children in source order.
    pub children: Vec<VisibleActivityNode<D>>,
}

fn project_nodes<D: PaneData>(
    nodes: &[ActivityNode<D>],
    data: &D,
    active: Option<&ActivityId>,
    inherited_color: Option<Hsla>,
    path: &mut Vec<CategoryId>,
    active_ancestors: &mut Vec<CategoryId>,
    category_chrome: &HashMap<CategoryId, CategoryChrome>,
) -> Vec<VisibleActivityNode<D>> {
    nodes
        .iter()
        .filter_map(|node| match node {
            ActivityNode::Activity(activity) => {
                if !(activity.filter)(data) {
                    return None;
                }
                if active == Some(&activity.id) {
                    *active_ancestors = path.clone();
                }
                Some(VisibleActivityNode::Activity(VisibleActivity {
                    activity: activity.clone(),
                    inherited_color,
                }))
            }
            ActivityNode::Category(category) => {
                let color = category_chrome
                    .get(&category.id)
                    .and_then(|chrome| chrome.color)
                    .unwrap_or(category.color);
                path.push(category.id.clone());
                let children = project_nodes(
                    &category.children,
                    data,
                    active,
                    Some(color),
                    path,
                    active_ancestors,
                    category_chrome,
                );
                path.pop();
                (!children.is_empty()).then(|| {
                    VisibleActivityNode::Category(VisibleCategory {
                        id: category.id.clone(),
                        name: category.name.clone(),
                        color,
                        children,
                    })
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActivityCategory;
    use gpui::{div, prelude::*, rgb, SharedString};
    use std::sync::Arc;

    fn visible(value: &bool) -> bool {
        *value
    }

    fn always(_: &bool) -> bool {
        true
    }

    fn activity(id: &str, filter: fn(&bool) -> bool) -> ActivityNode<bool> {
        ActivityNode::Activity(Activity {
            id: ActivityId::new(id),
            name: SharedString::from(id.to_owned()),
            filter,
            render: Arc::new(|_, _| div().into_any_element()),
        })
    }

    fn category(id: &str, color: u32, children: Vec<ActivityNode<bool>>) -> ActivityNode<bool> {
        ActivityNode::Category(ActivityCategory {
            id: CategoryId::new(id),
            name: SharedString::from(id.to_owned()),
            color: rgb(color).into(),
            children,
        })
    }

    fn activity_ids(nodes: &[VisibleActivityNode<bool>], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                VisibleActivityNode::Activity(activity) => {
                    out.push(activity.activity.id.0.clone());
                }
                VisibleActivityNode::Category(category) => activity_ids(&category.children, out),
            }
        }
    }

    #[test]
    fn recursive_projection_preserves_groups_order_prunes_and_tracks_active_path() {
        let catalog = ActivityCatalog::new(vec![
            activity("root", always),
            category(
                "outer",
                0xff0000,
                vec![
                    activity("filtered", visible),
                    category("empty", 0x00ff00, vec![activity("gone", visible)]),
                    category("inner", 0x0000ff, vec![activity("active", always)]),
                ],
            ),
        ])
        .with_trailing(vec![activity("settings", always)]);

        let projection = catalog.visible(&false, Some(&ActivityId::new("active")));
        let mut primary = Vec::new();
        activity_ids(&projection.primary, &mut primary);
        let mut trailing = Vec::new();
        activity_ids(&projection.trailing, &mut trailing);
        assert_eq!(primary, ["root", "active"]);
        assert_eq!(trailing, ["settings"]);
        assert_eq!(
            projection.active_ancestors,
            [CategoryId::new("outer"), CategoryId::new("inner")]
        );

        let VisibleActivityNode::Category(outer) = &projection.primary[1] else {
            panic!("outer category retained");
        };
        assert_eq!(outer.children.len(), 1, "both empty branches are pruned");
        let VisibleActivityNode::Category(inner) = &outer.children[0] else {
            panic!("inner category retained");
        };
        let VisibleActivityNode::Activity(active) = &inner.children[0] else {
            panic!("active activity retained");
        };
        assert_eq!(active.inherited_color, Some(rgb(0x0000ff).into()));
    }

    #[test]
    fn nearest_category_chrome_color_overrides_legacy_inheritance() {
        let override_color: Hsla = rgb(0xabcdef).into();
        let catalog = ActivityCatalog::new(vec![category(
            "outer",
            0xff0000,
            vec![category("inner", 0x00ff00, vec![activity("leaf", always)])],
        )])
        .with_category_chrome(
            CategoryId::new("inner"),
            CategoryChrome::default().with_color(override_color),
        );
        let projection = catalog.visible(&true, None);
        let VisibleActivityNode::Category(outer) = &projection.primary[0] else {
            panic!()
        };
        let VisibleActivityNode::Category(inner) = &outer.children[0] else {
            panic!()
        };
        let VisibleActivityNode::Activity(leaf) = &inner.children[0] else {
            panic!()
        };
        assert_eq!(leaf.inherited_color, Some(override_color));
    }

    #[test]
    fn validation_finds_duplicates_across_depth_and_groups_and_orphan_chrome() {
        let duplicate_activity = ActivityCatalog::new(vec![activity("same", always)])
            .with_trailing(vec![category(
                "container",
                0,
                vec![activity("same", always)],
            )]);
        assert!(matches!(
            duplicate_activity.validate(),
            Err(ActivityCatalogValidationError::DuplicateActivityId {
                duplicate_group: ActivityCatalogGroup::Trailing,
                ..
            })
        ));

        let duplicate_category = ActivityCatalog::new(vec![category(
            "same",
            0,
            vec![category("same", 0, vec![activity("leaf", always)])],
        )]);
        assert!(matches!(
            duplicate_category.validate(),
            Err(ActivityCatalogValidationError::DuplicateCategoryId { .. })
        ));

        let orphan = ActivityCatalog::new(vec![activity("known", always)])
            .with_activity_chrome(ActivityId::new("missing"), ActivityChrome::default());
        assert_eq!(
            orphan.validate(),
            Err(ActivityCatalogValidationError::MissingActivityChromeKey {
                activity_id: ActivityId::new("missing")
            })
        );
    }
}
