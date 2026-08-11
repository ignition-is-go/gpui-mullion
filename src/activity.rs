use crate::{ActivityId, CategoryId, PaneData, PaneId, WorkspaceId};
use gpui::{AnyElement, AnyView, App, SharedString, Window};
use std::{collections::HashMap, rc::Rc, sync::Arc};

/// Legacy stateless renderer. It remains `Send + Sync` for source compatibility.
pub type ActivityRenderer<D> = Arc<dyn Fn(&PaneId, &D) -> AnyElement + Send + Sync>;

/// Creates one durable activity instance for a pane.
///
/// Factories run lazily, when their activity is first selected. They are UI-local
/// (`Rc`, with no `Send`/`Sync` requirement) and may create GPUI entities through
/// the supplied window and app contexts.
pub type ActivityFactory<D> = Rc<dyn Fn(&PaneId, &D, &mut Window, &mut App) -> ActivityInstance<D>>;

/// Callback invoked when the owning pane's data changes.
pub type ActivityUpdate<D> = Rc<dyn Fn(&D, &mut Window, &mut App)>;
/// Callback invoked immediately before a cached instance is removed.
pub type ActivityDispose = Rc<dyn Fn(&mut App)>;

/// The stable GPUI views and lifecycle hooks produced by an [`ActivityFactory`].
///
/// `body` and `header` retain their entity identity until the pane, activity, or
/// workspace disappears from the view's model. `update` is called only after a
/// `PartialEq`-observable change to that pane's data. `dispose` is called once
/// when Mullion explicitly evicts the instance. Cache eviction includes GPUI
/// releasing the Mullion root entity.
pub struct ActivityInstance<D: PaneData> {
    /// The persistent main content view for this activity instance.
    pub body: AnyView,
    /// An optional persistent view rendered as activity-specific header chrome.
    pub header: Option<AnyView>,
    /// An optional hook that receives new pane data after it changes.
    pub update: Option<ActivityUpdate<D>>,
    /// An optional hook invoked once when Mullion explicitly evicts the instance.
    pub dispose: Option<ActivityDispose>,
}

impl<D: PaneData> ActivityInstance<D> {
    /// Creates an instance whose durable body is `body` and whose optional hooks are unset.
    pub fn new(body: impl Into<AnyView>) -> Self {
        Self {
            body: body.into(),
            header: None,
            update: None,
            dispose: None,
        }
    }

    /// Adds a durable header view that shares this instance's cache lifetime.
    pub fn with_header(mut self, header: impl Into<AnyView>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Sets the callback run when this instance's pane data changes by [`PartialEq`].
    ///
    /// The callback runs with the new data and may update the cached GPUI entities. It
    /// is not called for the initial value or for equal values.
    pub fn with_update(mut self, update: impl Fn(&D, &mut Window, &mut App) + 'static) -> Self {
        self.update = Some(Rc::new(update));
        self
    }

    /// Sets the callback run immediately before Mullion discards this cached instance.
    ///
    /// Use this to detach application-level resources that are not owned by the GPUI
    /// views themselves. Dropping an instance outside Mullion cannot trigger this hook.
    pub fn with_dispose(mut self, dispose: impl Fn(&mut App) + 'static) -> Self {
        self.dispose = Some(Rc::new(dispose));
        self
    }
}

/// Additive stateful renderer registry. Legacy [`Activity`] values need no changes.
pub struct ActivityFactoryRegistry<D: PaneData> {
    factories: HashMap<ActivityId, ActivityFactory<D>>,
}

impl<D: PaneData> Default for ActivityFactoryRegistry<D> {
    fn default() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }
}

impl<D: PaneData> ActivityFactoryRegistry<D> {
    /// Creates an empty registry; activities continue to use their legacy renderers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the lazy factory for `id`, returning the factory it replaces.
    ///
    /// Registration affects only future cache misses. An already-created instance keeps
    /// its entity identity and lifecycle hooks until normal eviction.
    pub fn register(
        &mut self,
        id: ActivityId,
        factory: impl Fn(&PaneId, &D, &mut Window, &mut App) -> ActivityInstance<D> + 'static,
    ) -> Option<ActivityFactory<D>> {
        self.factories.insert(id, Rc::new(factory))
    }

    /// Registers a factory and returns the registry for builder-style construction.
    ///
    /// As with [`Self::register`], replacing a factory does not evict existing instances.
    pub fn with_factory(
        mut self,
        id: ActivityId,
        factory: impl Fn(&PaneId, &D, &mut Window, &mut App) -> ActivityInstance<D> + 'static,
    ) -> Self {
        self.register(id, factory);
        self
    }

    /// Returns the factory used for future instances of `id`, if one is registered.
    pub fn get(&self, id: &ActivityId) -> Option<&ActivityFactory<D>> {
        self.factories.get(id)
    }

    /// Reports whether `id` has a stateful factory registered.
    pub fn contains(&self, id: &ActivityId) -> bool {
        self.factories.contains_key(id)
    }

    /// Stop using a factory for future instances and return it to the caller.
    /// Existing cached instances remain mounted until their normal eviction.
    pub fn unregister(&mut self, id: &ActivityId) -> Option<ActivityFactory<D>> {
        self.factories.remove(id)
    }
}

/// Stable namespace for a cached per-pane activity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActivityCacheKey {
    /// The workspace namespace, or `None` for panes outside a workspace.
    pub workspace: Option<WorkspaceId>,
    /// The pane that owns the cached instance.
    pub pane: PaneId,
    /// The activity definition instantiated in the pane.
    pub activity: ActivityId,
}

impl ActivityCacheKey {
    /// Creates a key from all identity scopes that determine an instance's lifetime.
    pub fn new(workspace: Option<WorkspaceId>, pane: PaneId, activity: ActivityId) -> Self {
        Self {
            workspace,
            pane,
            activity,
        }
    }
}

pub(crate) struct CachedActivity<D: PaneData> {
    pub instance: ActivityInstance<D>,
    pub data: D,
}

/// Lifecycle bookkeeping is independent of rendering so its eviction and data
/// transition rules can be tested without a platform window.
pub(crate) struct ActivityCache<D: PaneData> {
    entries: HashMap<ActivityCacheKey, CachedActivity<D>>,
}

impl<D: PaneData> Default for ActivityCache<D> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<D: PaneData> ActivityCache<D> {
    pub fn get(&self, key: &ActivityCacheKey) -> Option<&CachedActivity<D>> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: ActivityCacheKey, instance: ActivityInstance<D>, data: D) {
        self.entries.insert(key, CachedActivity { instance, data });
    }

    pub fn changed_callbacks(
        &mut self,
        pane_data: &HashMap<(Option<WorkspaceId>, PaneId), D>,
    ) -> Vec<(ActivityUpdate<D>, D)> {
        let mut updates = Vec::new();
        for (key, cached) in &mut self.entries {
            if let Some(data) = pane_data.get(&(key.workspace.clone(), key.pane.clone())) {
                if data != &cached.data {
                    cached.data = data.clone();
                    if let Some(update) = &cached.instance.update {
                        updates.push((update.clone(), data.clone()));
                    }
                }
            }
        }
        updates
    }

    pub fn remove_invalid(
        &mut self,
        valid: impl Fn(&ActivityCacheKey) -> bool,
    ) -> Vec<ActivityInstance<D>> {
        let stale = self
            .entries
            .keys()
            .filter(|key| !valid(key))
            .cloned()
            .collect::<Vec<_>>();
        stale
            .into_iter()
            .filter_map(|key| self.entries.remove(&key).map(|entry| entry.instance))
            .collect()
    }

    pub fn drain(&mut self) -> Vec<ActivityInstance<D>> {
        self.entries
            .drain()
            .map(|(_, entry)| entry.instance)
            .collect()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A native activity definition. Rendering stays adapter-specific; IDs and pane state remain portable.
#[derive(Clone)]
pub struct Activity<D: PaneData> {
    /// Stable identity used by selection, factories, chrome, and cache keys.
    pub id: ActivityId,
    /// Human-readable label presented by host navigation.
    pub name: SharedString,
    /// Predicate deciding whether this activity is visible for a pane's current data.
    pub filter: fn(&D) -> bool,
    /// Stateless compatibility renderer used when no stateful factory is registered.
    pub render: ActivityRenderer<D>,
}

impl<D: PaneData> Activity<D> {
    /// Define an activity with a stable identity and a legacy stateless renderer.
    ///
    /// Stateful GPUI content should register an [`ActivityFactoryRegistry`]
    /// factory for the same id; this renderer remains the compatibility fallback.
    pub fn new(
        id: impl Into<ActivityId>,
        name: impl Into<SharedString>,
        render: impl Fn(&PaneId, &D) -> AnyElement + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            filter: |_| true,
            render: Arc::new(render),
        }
    }

    /// Restrict visibility to panes accepted by `filter`.
    pub fn with_filter(mut self, filter: fn(&D) -> bool) -> Self {
        self.filter = filter;
        self
    }
}

/// A named, recursively nestable group of activities.
///
/// Category identity must be unique across an [`ActivityCatalog`](crate::ActivityCatalog),
/// including both primary and trailing trees. Empty categories are pruned from projections.
#[derive(Clone)]
pub struct ActivityCategory<D: PaneData> {
    /// Stable identity used by catalog validation, chrome, and active ancestry.
    pub id: CategoryId,
    /// Human-readable label presented by host navigation.
    pub name: SharedString,
    /// Legacy category color, used unless catalog chrome overrides it.
    pub color: gpui::Hsla,
    /// Ordered activities and nested categories in this group.
    pub children: Vec<ActivityNode<D>>,
}

impl<D: PaneData> ActivityCategory<D> {
    /// Define a recursive activity category.
    pub fn new(
        id: impl Into<CategoryId>,
        name: impl Into<SharedString>,
        color: gpui::Hsla,
        children: impl IntoIterator<Item = ActivityNode<D>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color,
            children: children.into_iter().collect(),
        }
    }
}

/// A node in the recursive activity information architecture.
#[derive(Clone)]
pub enum ActivityNode<D: PaneData> {
    /// A selectable leaf activity.
    Activity(Activity<D>),
    /// A nested category whose children are projected recursively.
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{div, prelude::*, Context, Render, TestAppContext};
    use std::cell::Cell;

    struct StatefulView(usize);

    impl Render for StatefulView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.to_string())
        }
    }

    fn key(workspace: Option<&str>, pane: &str, activity: &str) -> ActivityCacheKey {
        ActivityCacheKey::new(
            workspace.map(|id| WorkspaceId(id.into())),
            PaneId::new(pane),
            ActivityId::new(activity),
        )
    }

    #[gpui::test]
    fn cache_keeps_stateful_entity_identity_and_updates_only_changed_data(cx: &mut TestAppContext) {
        let body = cx.new(|_| StatefulView(1));
        let any_body: AnyView = body.clone().into();
        let updates = Rc::new(Cell::new(0));
        let update_count = updates.clone();
        let instance = ActivityInstance::new(body).with_update(move |_: &String, _, _| {
            update_count.set(update_count.get() + 1);
        });
        let mut cache = ActivityCache::default();
        let cache_key = key(Some("one"), "pane", "activity");
        cache.insert(cache_key.clone(), instance, "old".into());

        let unchanged = HashMap::from([(
            (Some(WorkspaceId("one".into())), PaneId::new("pane")),
            "old".to_string(),
        )]);
        assert!(cache.changed_callbacks(&unchanged).is_empty());
        assert_eq!(cache.get(&cache_key).unwrap().instance.body, any_body);

        let changed = HashMap::from([(
            (Some(WorkspaceId("one".into())), PaneId::new("pane")),
            "new".to_string(),
        )]);
        let callbacks = cache.changed_callbacks(&changed);
        assert_eq!(callbacks.len(), 1);
        assert_eq!(cache.get(&cache_key).unwrap().instance.body, any_body);
        // A repeated render of identical pane data schedules no second update.
        assert!(cache.changed_callbacks(&changed).is_empty());
    }

    #[gpui::test]
    fn cache_keys_separate_workspaces_and_eviction_returns_each_instance(cx: &mut TestAppContext) {
        let mut cache = ActivityCache::default();
        for workspace in ["one", "two"] {
            let body = cx.new(|_| StatefulView(0));
            cache.insert(
                key(Some(workspace), "same-pane", "same-activity"),
                ActivityInstance::new(body),
                workspace.to_string(),
            );
        }
        assert_eq!(cache.len(), 2);
        let removed =
            cache.remove_invalid(|key| key.workspace.as_ref() == Some(&WorkspaceId("two".into())));
        assert_eq!(removed.len(), 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn registry_is_additive_and_duplicate_registration_returns_previous_factory() {
        let captured = Rc::new(Cell::new(0));
        let mut registry = ActivityFactoryRegistry::<String>::new();
        let first_capture = captured.clone();
        assert!(registry
            .register(ActivityId::new("stateful"), move |_, _, _, cx| {
                first_capture.set(1);
                ActivityInstance::new(cx.new(|_| StatefulView(1)))
            })
            .is_none());
        assert!(registry.contains(&ActivityId::new("stateful")));
        assert!(registry
            .register(ActivityId::new("stateful"), |_, _, _, cx| {
                ActivityInstance::new(cx.new(|_| StatefulView(2)))
            })
            .is_some());
    }

    #[test]
    fn activity_builder_accepts_string_ids_and_defaults_visible() {
        let activity = Activity::new("files", "Files", |_, _: &String| div().into_any_element());
        assert_eq!(activity.id.as_ref(), "files");
        assert_eq!(activity.name.as_ref(), "Files");
        assert!((activity.filter)(&"data".to_owned()));
    }

    #[test]
    fn stable_key_includes_all_three_identity_components() {
        assert_eq!(key(Some("w"), "p", "a"), key(Some("w"), "p", "a"));
        assert_ne!(key(Some("w"), "p", "a"), key(Some("x"), "p", "a"));
        assert_ne!(key(Some("w"), "p", "a"), key(Some("w"), "q", "a"));
        assert_ne!(key(Some("w"), "p", "a"), key(Some("w"), "p", "b"));
        assert_ne!(key(None, "p", "a"), key(Some("w"), "p", "a"));
    }
}
