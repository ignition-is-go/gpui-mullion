//! Window-level overlay policy and host integration primitives.
//!
//! This module deliberately does not render the overlay layer. [`MullionOverlay`]
//! carries GPUI-local content while [`OverlayPolicy`] is portable application
//! state. A root view can consume [`OverlayHostConfig`] and paint its sorted
//! snapshot after the pane tree, outside any `overflow_hidden` activity content.

use gpui::{AnyElement, App, Window};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// Stable application-provided identity for an overlay.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverlayId(String);

impl OverlayId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for OverlayId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for OverlayId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for OverlayId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Deterministic stacking bands above Mullion's pane chrome.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OverlayTier {
    #[default]
    Modal,
    Toast,
    Drag,
}

impl OverlayTier {
    /// A stable sort key. Hosts may map this to their own concrete z values.
    pub const fn z_order(self) -> u8 {
        match self {
            Self::Modal => 10,
            Self::Toast => 20,
            Self::Drag => 30,
        }
    }
}

/// Alignment along one viewport axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayAlignment {
    Start,
    #[default]
    Center,
    End,
    Stretch,
}

/// Viewport placement for overlay content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayPlacement {
    pub horizontal: OverlayAlignment,
    pub vertical: OverlayAlignment,
}

impl OverlayPlacement {
    pub const CENTER: Self = Self::new(OverlayAlignment::Center, OverlayAlignment::Center);
    pub const FILL: Self = Self::new(OverlayAlignment::Stretch, OverlayAlignment::Stretch);

    pub const fn new(horizontal: OverlayAlignment, vertical: OverlayAlignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

/// A viewport-independent dimension interpreted by the future host renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum OverlayLength {
    #[default]
    Content,
    Fill,
    Pixels(f32),
    Fraction(f32),
}

impl OverlayLength {
    fn validate(self, field: &'static str) -> Result<(), OverlayError> {
        match self {
            Self::Pixels(value) if !value.is_finite() || value < 0.0 => {
                Err(OverlayError::InvalidDimension { field, value })
            }
            Self::Fraction(value) if !value.is_finite() || !(0.0..=1.0).contains(&value) => {
                Err(OverlayError::InvalidDimension { field, value })
            }
            _ => Ok(()),
        }
    }
}

/// Requested overlay content size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlaySize {
    pub width: OverlayLength,
    pub height: OverlayLength,
}

impl OverlaySize {
    pub const CONTENT: Self = Self::new(OverlayLength::Content, OverlayLength::Content);
    pub const FILL: Self = Self::new(OverlayLength::Fill, OverlayLength::Fill);

    pub const fn new(width: OverlayLength, height: OverlayLength) -> Self {
        Self { width, height }
    }

    fn validate(self) -> Result<(), OverlayError> {
        self.width.validate("width")?;
        self.height.validate("height")
    }
}

/// Serializable backdrop color in straight-alpha sRGB.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayBackdrop {
    pub rgba: [f32; 4],
}

impl Default for OverlayBackdrop {
    fn default() -> Self {
        Self {
            rgba: [0.0, 0.0, 0.0, 0.5],
        }
    }
}

impl OverlayBackdrop {
    pub const fn new(rgba: [f32; 4]) -> Self {
        Self { rgba }
    }

    fn validate(self) -> Result<(), OverlayError> {
        if self
            .rgba
            .iter()
            .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
        {
            Ok(())
        } else {
            Err(OverlayError::InvalidBackdrop)
        }
    }
}

/// Cloneable and serializable overlay behavior, separate from GPUI content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayPolicy {
    pub id: OverlayId,
    pub tier: OverlayTier,
    pub placement: OverlayPlacement,
    pub size: OverlaySize,
    pub backdrop: Option<OverlayBackdrop>,
    pub dismiss_on_backdrop: bool,
    pub click_through: bool,
    pub a11y_modal: bool,
    pub a11y_label: Option<String>,
}

impl OverlayPolicy {
    pub fn new(id: impl Into<OverlayId>) -> Self {
        Self {
            id: id.into(),
            tier: OverlayTier::default(),
            placement: OverlayPlacement::default(),
            size: OverlaySize::default(),
            backdrop: None,
            dismiss_on_backdrop: false,
            click_through: false,
            a11y_modal: false,
            a11y_label: None,
        }
    }

    pub fn with_tier(mut self, tier: OverlayTier) -> Self {
        self.tier = tier;
        self
    }

    pub fn with_placement(mut self, placement: OverlayPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn with_size(mut self, size: OverlaySize) -> Self {
        self.size = size;
        self
    }

    pub fn with_backdrop(mut self, backdrop: OverlayBackdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    pub fn click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    pub fn a11y_modal(mut self, modal: bool) -> Self {
        self.a11y_modal = modal;
        self
    }

    pub fn with_a11y_label(mut self, label: impl Into<String>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        if self.id.as_str().is_empty() {
            return Err(OverlayError::EmptyId);
        }
        self.size.validate()?;
        if let Some(backdrop) = self.backdrop {
            backdrop.validate()?;
        }
        if self.dismiss_on_backdrop && self.backdrop.is_none() {
            return Err(OverlayError::DismissWithoutBackdrop(self.id.clone()));
        }
        if self.click_through && self.dismiss_on_backdrop {
            return Err(OverlayError::ClickThroughDismiss(self.id.clone()));
        }
        if self.a11y_modal && self.click_through {
            return Err(OverlayError::ClickThroughModal(self.id.clone()));
        }
        if self.a11y_label.as_deref().is_some_and(str::is_empty) {
            return Err(OverlayError::EmptyA11yLabel(self.id.clone()));
        }
        Ok(())
    }
}

/// UI-local renderer. `Rc` intentionally keeps GPUI closures on the UI thread.
pub type OverlayRenderer = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Portable policy paired with GPUI-local content.
#[derive(Clone)]
pub struct MullionOverlay {
    policy: OverlayPolicy,
    renderer: OverlayRenderer,
}

impl MullionOverlay {
    pub fn new(
        id: impl Into<OverlayId>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self::from_policy(OverlayPolicy::new(id), render)
    }

    pub fn from_policy(
        policy: OverlayPolicy,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            policy,
            renderer: Rc::new(render),
        }
    }

    pub fn policy(&self) -> &OverlayPolicy {
        &self.policy
    }

    pub fn renderer(&self) -> &OverlayRenderer {
        &self.renderer
    }

    pub fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        (self.renderer)(window, cx)
    }

    pub fn with_tier(mut self, tier: OverlayTier) -> Self {
        self.policy.tier = tier;
        self
    }

    pub fn with_placement(mut self, placement: OverlayPlacement) -> Self {
        self.policy.placement = placement;
        self
    }

    pub fn with_size(mut self, size: OverlaySize) -> Self {
        self.policy.size = size;
        self
    }

    pub fn with_backdrop(mut self, backdrop: OverlayBackdrop) -> Self {
        self.policy.backdrop = Some(backdrop);
        self
    }

    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.policy.dismiss_on_backdrop = dismiss;
        self
    }

    pub fn click_through(mut self, click_through: bool) -> Self {
        self.policy.click_through = click_through;
        self
    }

    pub fn a11y_modal(mut self, modal: bool) -> Self {
        self.policy.a11y_modal = modal;
        self
    }

    pub fn with_a11y_label(mut self, label: impl Into<String>) -> Self {
        self.policy.a11y_label = Some(label.into());
        self
    }
}

impl fmt::Debug for MullionOverlay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MullionOverlay")
            .field("policy", &self.policy)
            .field("renderer", &"OverlayRenderer(..)")
            .finish()
    }
}

/// Ordered changes accepted by [`OverlayStack::apply_atomic`].
#[derive(Clone, Debug)]
pub enum OverlayMutation {
    Push(MullionOverlay),
    Remove(OverlayId),
    Replace(MullionOverlay),
    MoveToFront(OverlayId),
    Clear,
}

/// An insertion-ordered, uniquely identified overlay collection.
#[derive(Clone, Debug, Default)]
pub struct OverlayStack {
    overlays: Vec<MullionOverlay>,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_overlays(
        overlays: impl IntoIterator<Item = MullionOverlay>,
    ) -> Result<Self, OverlayError> {
        let stack = Self {
            overlays: overlays.into_iter().collect(),
        };
        stack.validate()?;
        Ok(stack)
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn get(&self, id: &OverlayId) -> Option<&MullionOverlay> {
        self.overlays
            .iter()
            .find(|overlay| &overlay.policy.id == id)
    }

    pub fn insertion_order(&self) -> &[MullionOverlay] {
        &self.overlays
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        for (index, overlay) in self.overlays.iter().enumerate() {
            overlay.policy.validate()?;
            if self.overlays[..index]
                .iter()
                .any(|prior| prior.policy.id == overlay.policy.id)
            {
                return Err(OverlayError::DuplicateId(overlay.policy.id.clone()));
            }
        }
        Ok(())
    }

    /// Clones overlays in paint order: tier first, insertion order within a tier.
    pub fn sorted_render_snapshot(&self) -> Vec<MullionOverlay> {
        let mut snapshot = self.overlays.clone();
        snapshot.sort_by_key(|overlay| overlay.policy.tier.z_order());
        snapshot
    }

    /// Applies all changes or none. Mutations are evaluated in the supplied order.
    pub fn apply_atomic(
        &mut self,
        mutations: impl IntoIterator<Item = OverlayMutation>,
    ) -> Result<(), OverlayError> {
        let mut next = self.clone();
        for mutation in mutations {
            next.apply_one(mutation)?;
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn push(&mut self, overlay: MullionOverlay) -> Result<(), OverlayError> {
        self.apply_atomic([OverlayMutation::Push(overlay)])
    }

    pub fn remove(&mut self, id: impl Into<OverlayId>) -> Result<(), OverlayError> {
        self.apply_atomic([OverlayMutation::Remove(id.into())])
    }

    fn apply_one(&mut self, mutation: OverlayMutation) -> Result<(), OverlayError> {
        match mutation {
            OverlayMutation::Push(overlay) => {
                overlay.policy.validate()?;
                if self.get(&overlay.policy.id).is_some() {
                    return Err(OverlayError::DuplicateId(overlay.policy.id.clone()));
                }
                self.overlays.push(overlay);
            }
            OverlayMutation::Remove(id) => {
                let Some(index) = self
                    .overlays
                    .iter()
                    .position(|overlay| overlay.policy.id == id)
                else {
                    return Err(OverlayError::UnknownId(id));
                };
                self.overlays.remove(index);
            }
            OverlayMutation::Replace(overlay) => {
                overlay.policy.validate()?;
                let Some(index) = self
                    .overlays
                    .iter()
                    .position(|current| current.policy.id == overlay.policy.id)
                else {
                    return Err(OverlayError::UnknownId(overlay.policy.id.clone()));
                };
                self.overlays[index] = overlay;
            }
            OverlayMutation::MoveToFront(id) => {
                let Some(index) = self
                    .overlays
                    .iter()
                    .position(|overlay| overlay.policy.id == id)
                else {
                    return Err(OverlayError::UnknownId(id));
                };
                let overlay = self.overlays.remove(index);
                self.overlays.push(overlay);
            }
            OverlayMutation::Clear => self.overlays.clear(),
        }
        Ok(())
    }
}

/// Pull-based controlled source owned by the host application.
#[derive(Clone)]
pub struct ControlledOverlaySource(Rc<dyn Fn() -> OverlayStack>);

impl ControlledOverlaySource {
    pub fn new(source: impl Fn() -> OverlayStack + 'static) -> Self {
        Self(Rc::new(source))
    }

    pub fn snapshot(&self) -> OverlayStack {
        (self.0)()
    }
}

impl fmt::Debug for ControlledOverlaySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ControlledOverlaySource(..)")
    }
}

/// Callback used by a root renderer after a dismissible backdrop is clicked.
pub type OverlayDismissHandler = Rc<dyn Fn(&OverlayId, &mut Window, &mut App)>;

/// Host-side inputs for the future root overlay layer.
#[derive(Clone)]
pub struct OverlayHostConfig {
    source: ControlledOverlaySource,
    on_dismiss: Option<OverlayDismissHandler>,
}

impl OverlayHostConfig {
    pub fn new(source: ControlledOverlaySource) -> Self {
        Self {
            source,
            on_dismiss: None,
        }
    }

    pub fn controlled(source: impl Fn() -> OverlayStack + 'static) -> Self {
        Self::new(ControlledOverlaySource::new(source))
    }

    pub fn with_dismiss_handler(
        mut self,
        handler: impl Fn(&OverlayId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    pub fn source(&self) -> &ControlledOverlaySource {
        &self.source
    }

    pub fn on_dismiss(&self) -> Option<&OverlayDismissHandler> {
        self.on_dismiss.as_ref()
    }

    pub fn sorted_render_snapshot(&self) -> Result<Vec<MullionOverlay>, OverlayError> {
        let snapshot = self.source.snapshot();
        snapshot.validate()?;
        Ok(snapshot.sorted_render_snapshot())
    }
}

impl fmt::Debug for OverlayHostConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OverlayHostConfig")
            .field("source", &self.source)
            .field("on_dismiss", &self.on_dismiss.as_ref().map(|_| "(..)"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayError {
    EmptyId,
    DuplicateId(OverlayId),
    UnknownId(OverlayId),
    InvalidDimension { field: &'static str, value: f32 },
    InvalidBackdrop,
    DismissWithoutBackdrop(OverlayId),
    ClickThroughDismiss(OverlayId),
    ClickThroughModal(OverlayId),
    EmptyA11yLabel(OverlayId),
}

impl fmt::Display for OverlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("overlay id must not be empty"),
            Self::DuplicateId(id) => write!(formatter, "duplicate overlay id `{id}`"),
            Self::UnknownId(id) => write!(formatter, "unknown overlay id `{id}`"),
            Self::InvalidDimension { field, value } => {
                write!(formatter, "invalid overlay {field} dimension `{value}`")
            }
            Self::InvalidBackdrop => {
                formatter.write_str("backdrop channels must be finite and between zero and one")
            }
            Self::DismissWithoutBackdrop(id) => write!(
                formatter,
                "overlay `{id}` cannot dismiss on a missing backdrop"
            ),
            Self::ClickThroughDismiss(id) => write!(
                formatter,
                "click-through overlay `{id}` cannot dismiss on backdrop"
            ),
            Self::ClickThroughModal(id) => {
                write!(formatter, "accessible modal `{id}` cannot be click-through")
            }
            Self::EmptyA11yLabel(id) => {
                write!(formatter, "overlay `{id}` has an empty accessibility label")
            }
        }
    }
}

impl Error for OverlayError {}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{div, IntoElement};

    fn overlay(id: &str, tier: OverlayTier) -> MullionOverlay {
        MullionOverlay::new(id, |_, _| div().into_any_element()).with_tier(tier)
    }

    #[test]
    fn policy_has_stable_serde_shape_and_round_trips() {
        let policy = OverlayPolicy::new("save-dialog")
            .with_tier(OverlayTier::Toast)
            .with_placement(OverlayPlacement::new(
                OverlayAlignment::End,
                OverlayAlignment::Start,
            ))
            .with_size(OverlaySize::new(
                OverlayLength::Pixels(480.0),
                OverlayLength::Content,
            ))
            .with_backdrop(OverlayBackdrop::default())
            .dismiss_on_backdrop(true)
            .a11y_modal(true)
            .with_a11y_label("Save project");

        let json = serde_json::to_value(&policy).unwrap();
        assert_eq!(json["id"], "save-dialog");
        assert_eq!(json["tier"], "toast");
        assert_eq!(json["placement"]["horizontal"], "end");
        assert_eq!(json["size"]["width"]["kind"], "pixels");
        assert_eq!(
            serde_json::from_value::<OverlayPolicy>(json).unwrap(),
            policy
        );
    }

    #[test]
    fn tier_order_is_explicit_and_snapshot_is_stable() {
        assert!(OverlayTier::Modal.z_order() < OverlayTier::Toast.z_order());
        assert!(OverlayTier::Toast.z_order() < OverlayTier::Drag.z_order());

        let stack = OverlayStack::from_overlays([
            overlay("drag", OverlayTier::Drag),
            overlay("modal-a", OverlayTier::Modal),
            overlay("toast", OverlayTier::Toast),
            overlay("modal-b", OverlayTier::Modal),
        ])
        .unwrap();
        let ids: Vec<_> = stack
            .sorted_render_snapshot()
            .iter()
            .map(|item| item.policy().id.as_str().to_owned())
            .collect();
        assert_eq!(ids, ["modal-a", "modal-b", "toast", "drag"]);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let result = OverlayStack::from_overlays([
            overlay("same", OverlayTier::Modal),
            overlay("same", OverlayTier::Toast),
        ]);
        assert_eq!(
            result.unwrap_err(),
            OverlayError::DuplicateId("same".into())
        );
    }

    #[test]
    fn mutations_are_ordered_and_atomic() {
        let mut stack = OverlayStack::from_overlays([
            overlay("first", OverlayTier::Modal),
            overlay("second", OverlayTier::Modal),
        ])
        .unwrap();

        let result = stack.apply_atomic([
            OverlayMutation::Remove("first".into()),
            OverlayMutation::Push(overlay("third", OverlayTier::Toast)),
            OverlayMutation::Push(overlay("second", OverlayTier::Drag)),
        ]);
        assert_eq!(
            result.unwrap_err(),
            OverlayError::DuplicateId("second".into())
        );
        let ids: Vec<_> = stack
            .insertion_order()
            .iter()
            .map(|item| item.policy().id.as_str())
            .collect();
        assert_eq!(ids, ["first", "second"]);
    }

    #[test]
    fn policy_validation_rejects_incoherent_interaction() {
        let policy = OverlayPolicy::new("bad").dismiss_on_backdrop(true);
        assert_eq!(
            policy.validate().unwrap_err(),
            OverlayError::DismissWithoutBackdrop("bad".into())
        );

        let policy = OverlayPolicy::new("bad-modal")
            .a11y_modal(true)
            .click_through(true);
        assert_eq!(
            policy.validate().unwrap_err(),
            OverlayError::ClickThroughModal("bad-modal".into())
        );
    }

    #[test]
    fn policy_validation_rejects_invalid_geometry_and_backdrop() {
        let policy = OverlayPolicy::new("bad-size").with_size(OverlaySize::new(
            OverlayLength::Fraction(1.5),
            OverlayLength::Pixels(f32::NAN),
        ));
        assert_eq!(
            policy.validate().unwrap_err(),
            OverlayError::InvalidDimension {
                field: "width",
                value: 1.5,
            }
        );

        let policy = OverlayPolicy::new("bad-backdrop")
            .with_backdrop(OverlayBackdrop::new([0.0, 0.0, 0.0, 1.1]));
        assert_eq!(
            policy.validate().unwrap_err(),
            OverlayError::InvalidBackdrop
        );
    }

    #[test]
    fn controlled_host_reads_live_state_and_sorts_each_snapshot() {
        use std::cell::RefCell;

        let source = Rc::new(RefCell::new(
            OverlayStack::from_overlays([overlay("toast", OverlayTier::Toast)]).unwrap(),
        ));
        let config = OverlayHostConfig::controlled({
            let source = Rc::clone(&source);
            move || source.borrow().clone()
        });

        source
            .borrow_mut()
            .push(overlay("modal", OverlayTier::Modal))
            .unwrap();
        let ids: Vec<_> = config
            .sorted_render_snapshot()
            .unwrap()
            .iter()
            .map(|item| item.policy().id.as_str().to_owned())
            .collect();
        assert_eq!(ids, ["modal", "toast"]);
    }
}
