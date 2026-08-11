//! Window-level overlay policy and host integration primitives.
//!
//! This module deliberately does not render the overlay layer. [`MullionOverlay`]
//! carries GPUI-local content while [`OverlayPolicy`] is portable application
//! state. A root view can consume [`OverlayHostConfig`] and paint its sorted
//! snapshot after the pane tree, outside any `overflow_hidden` activity content.

use gpui::{AnyElement, App, Window};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// Stable application-provided identity for an overlay.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverlayId(String);

impl OverlayId {
    /// Creates an identity from an application-defined string.
    ///
    /// The value is preserved verbatim. In particular, this constructor does not reject an
    /// empty string; call [`OverlayPolicy::validate`] (directly or through an
    /// [`OverlayStack`]) before rendering.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the application-defined identity as a string slice.
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
    /// Modal content, painted below notifications and drag affordances.
    #[default]
    Modal,
    /// Transient notification content, painted above modals and below drag affordances.
    Toast,
    /// Drag-and-drop affordances, painted above every other tier.
    Drag,
}

impl OverlayTier {
    /// Returns the stable, unitless primary paint-order key for this tier.
    ///
    /// The keys are `10` for [`Self::Modal`], `20` for [`Self::Toast`], and `30` for
    /// [`Self::Drag`]. Hosts sort ascending, painting larger keys above smaller keys, and may map
    /// these keys to their own concrete z values.
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
    /// Aligns the content with the beginning of the axis.
    Start,
    /// Centers the content on the axis.
    #[default]
    Center,
    /// Aligns the content with the end of the axis.
    End,
    /// Expands the content to the available extent on the axis.
    Stretch,
}

/// Viewport placement for overlay content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayPlacement {
    /// Alignment along the viewport's horizontal axis.
    pub horizontal: OverlayAlignment,
    /// Alignment along the viewport's vertical axis.
    pub vertical: OverlayAlignment,
}

impl OverlayPlacement {
    /// Placement centered on both viewport axes.
    pub const CENTER: Self = Self::new(OverlayAlignment::Center, OverlayAlignment::Center);
    /// Placement stretched across both viewport axes.
    pub const FILL: Self = Self::new(OverlayAlignment::Stretch, OverlayAlignment::Stretch);

    /// Creates a placement from independent horizontal and vertical alignments.
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
    /// Uses the content's intrinsic extent on the axis.
    #[default]
    Content,
    /// Uses all space available from the viewport on the axis.
    Fill,
    /// Requests a logical-pixel extent.
    ///
    /// Validation accepts only finite, non-negative values.
    Pixels(f32),
    /// Requests a fraction of the available viewport extent.
    ///
    /// Validation accepts only finite values in the inclusive range `0.0..=1.0`.
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
    /// Requested horizontal extent.
    pub width: OverlayLength,
    /// Requested vertical extent.
    pub height: OverlayLength,
}

impl OverlaySize {
    /// A size using the content's intrinsic extent on both axes.
    pub const CONTENT: Self = Self::new(OverlayLength::Content, OverlayLength::Content);
    /// A size using all available viewport space on both axes.
    pub const FILL: Self = Self::new(OverlayLength::Fill, OverlayLength::Fill);

    /// Creates a size from independent width and height requests.
    ///
    /// Values are not checked here; [`OverlayPolicy::validate`] enforces the finite and range
    /// requirements of [`OverlayLength::Pixels`] and [`OverlayLength::Fraction`].
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
    /// Red, green, blue, and alpha channels, each in the inclusive range `0.0..=1.0`.
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
    /// Creates a backdrop from straight-alpha sRGB channels.
    ///
    /// Channels are not checked here; [`OverlayPolicy::validate`] rejects non-finite values and
    /// values outside the inclusive range `0.0..=1.0`.
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
    /// Stable identity used for lookup, mutation, and dismissal callbacks.
    pub id: OverlayId,
    /// Stacking band used as the primary paint-order key.
    pub tier: OverlayTier,
    /// Alignment of the overlay content within the viewport.
    pub placement: OverlayPlacement,
    /// Requested content dimensions.
    pub size: OverlaySize,
    /// Optional straight-alpha sRGB backdrop painted behind the content.
    pub backdrop: Option<OverlayBackdrop>,
    /// Whether a click on the backdrop outside the content requests dismissal through the host
    /// callback.
    ///
    /// This requires [`Self::backdrop`] to be present and [`Self::click_through`] to be `false`.
    /// A backdrop click is only a request: the controlled source must remove the overlay.
    pub dismiss_on_backdrop: bool,
    /// Whether the host wrapper permits otherwise-unhandled pointer input to reach underlying UI.
    ///
    /// Interactive elements supplied by the renderer may still handle their own input.
    /// Click-through is incompatible with backdrop dismissal and with [`Self::a11y_modal`].
    pub click_through: bool,
    /// Whether assistive technology should treat the overlay as modal.
    ///
    /// An accessible modal cannot be click-through.
    pub a11y_modal: bool,
    /// Optional non-empty accessible label for the overlay.
    pub a11y_label: Option<String>,
}

impl OverlayPolicy {
    /// Creates a policy with centered, intrinsic-size modal defaults and no backdrop.
    ///
    /// Backdrop dismissal, click-through, and accessible-modal behavior are disabled. The
    /// identity is preserved without validation; call [`Self::validate`] before rendering.
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

    /// Sets the stacking tier used to order the overlay for painting.
    pub fn with_tier(mut self, tier: OverlayTier) -> Self {
        self.tier = tier;
        self
    }

    /// Sets the overlay's alignment within the viewport.
    pub fn with_placement(mut self, placement: OverlayPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the requested content size.
    ///
    /// Dimension ranges are checked by [`Self::validate`], not by this builder.
    pub fn with_size(mut self, size: OverlaySize) -> Self {
        self.size = size;
        self
    }

    /// Sets the backdrop painted behind the overlay content.
    ///
    /// Channel ranges are checked by [`Self::validate`], not by this builder.
    pub fn with_backdrop(mut self, backdrop: OverlayBackdrop) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    /// Enables or disables dismissal requests when the backdrop outside the content is clicked.
    ///
    /// Enabling this requires a backdrop and forbids click-through. The host invokes its
    /// dismissal callback; removal remains the controlled source's responsibility.
    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    /// Enables or disables passing otherwise-unhandled pointer input through the host wrapper.
    ///
    /// Renderer-provided interactive elements may still handle input. Click-through cannot be
    /// combined with backdrop dismissal or accessible-modal behavior.
    pub fn click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    /// Sets whether assistive technology should treat the overlay as modal.
    ///
    /// Accessible-modal behavior cannot be combined with click-through.
    pub fn a11y_modal(mut self, modal: bool) -> Self {
        self.a11y_modal = modal;
        self
    }

    /// Sets the overlay's accessible label.
    ///
    /// Empty labels are rejected by [`Self::validate`].
    pub fn with_a11y_label(mut self, label: impl Into<String>) -> Self {
        self.a11y_label = Some(label.into());
        self
    }

    /// Validates identity, dimensions, backdrop channels, and interaction coherence.
    ///
    /// Validation rejects empty identities and labels; invalid pixel, fractional, or color
    /// values; dismissal without a backdrop; and incompatible click-through combinations.
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
    /// Creates an overlay with [`OverlayPolicy::new`] defaults and a UI-local renderer.
    ///
    /// The resulting policy is not validated until explicitly validated or inserted into a
    /// validated stack.
    pub fn new(
        id: impl Into<OverlayId>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self::from_policy(OverlayPolicy::new(id), render)
    }

    /// Pairs an existing portable policy with a UI-local renderer.
    ///
    /// This constructor does not validate `policy`; controlled hosts validate their snapshots
    /// before returning them for painting.
    pub fn from_policy(
        policy: OverlayPolicy,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            policy,
            renderer: Rc::new(render),
        }
    }

    /// Returns the portable policy associated with this renderer.
    pub fn policy(&self) -> &OverlayPolicy {
        &self.policy
    }

    /// Returns the UI-local rendering callback.
    pub fn renderer(&self) -> &OverlayRenderer {
        &self.renderer
    }

    /// Invokes the rendering callback for the given GPUI window and application.
    pub fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        (self.renderer)(window, cx)
    }

    /// Sets the policy's stacking tier used as the primary paint-order key.
    pub fn with_tier(mut self, tier: OverlayTier) -> Self {
        self.policy.tier = tier;
        self
    }

    /// Sets the policy's viewport alignment.
    pub fn with_placement(mut self, placement: OverlayPlacement) -> Self {
        self.policy.placement = placement;
        self
    }

    /// Sets the requested content size; validation is deferred.
    pub fn with_size(mut self, size: OverlaySize) -> Self {
        self.policy.size = size;
        self
    }

    /// Sets the straight-alpha sRGB backdrop; channel validation is deferred.
    pub fn with_backdrop(mut self, backdrop: OverlayBackdrop) -> Self {
        self.policy.backdrop = Some(backdrop);
        self
    }

    /// Enables or disables dismissal requests on backdrop clicks outside the content.
    ///
    /// Enabling this requires a backdrop and disables no fields automatically; validation rejects
    /// a conflicting click-through policy. A dismissal callback only requests that controlled
    /// application state remove the overlay.
    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.policy.dismiss_on_backdrop = dismiss;
        self
    }

    /// Enables or disables passing otherwise-unhandled pointer input through the host wrapper.
    ///
    /// Renderer-provided interactive elements may still handle input. Validation rejects
    /// click-through combined with backdrop dismissal or accessible modality.
    pub fn click_through(mut self, click_through: bool) -> Self {
        self.policy.click_through = click_through;
        self
    }

    /// Sets whether assistive technology should treat the overlay as modal.
    ///
    /// Validation rejects accessible modality combined with click-through.
    pub fn a11y_modal(mut self, modal: bool) -> Self {
        self.policy.a11y_modal = modal;
        self
    }

    /// Sets the accessible label; empty labels are rejected during validation.
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
    /// Appends a new, uniquely identified overlay to insertion order.
    Push(MullionOverlay),
    /// Removes the overlay with the given identity, or fails if it is absent.
    Remove(OverlayId),
    /// Replaces the matching overlay in place, preserving its insertion-order position.
    Replace(MullionOverlay),
    /// Moves the matching overlay to the end of insertion order.
    ///
    /// This brings it to the front only among overlays in the same tier; tier remains the primary
    /// paint-order key.
    MoveToFront(OverlayId),
    /// Removes every overlay.
    Clear,
}

/// An insertion-ordered, uniquely identified overlay collection.
#[derive(Clone, Debug, Default)]
pub struct OverlayStack {
    overlays: Vec<MullionOverlay>,
}

impl OverlayStack {
    /// Creates an empty overlay stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a stack in iterator order and validates all policies and identity uniqueness.
    ///
    /// Returns the first policy error in insertion order, or a duplicate-identity error.
    pub fn from_overlays(
        overlays: impl IntoIterator<Item = MullionOverlay>,
    ) -> Result<Self, OverlayError> {
        let stack = Self::from_unchecked(overlays);
        stack.validate()?;
        Ok(stack)
    }

    /// Build a controlled snapshot without validating it immediately.
    ///
    /// This is useful at application state boundaries: the root host validates every
    /// snapshot and exposes any [`OverlayError`] instead of panicking or painting a
    /// partially valid stack.
    pub fn from_unchecked(overlays: impl IntoIterator<Item = MullionOverlay>) -> Self {
        Self {
            overlays: overlays.into_iter().collect(),
        }
    }

    /// Returns the number of overlays in the stack.
    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    /// Returns `true` when the stack contains no overlays.
    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    /// Returns the overlay with `id`, if present.
    pub fn get(&self, id: &OverlayId) -> Option<&MullionOverlay> {
        self.overlays
            .iter()
            .find(|overlay| &overlay.policy.id == id)
    }

    /// Returns overlays in insertion order, before tier-based paint sorting.
    pub fn insertion_order(&self) -> &[MullionOverlay] {
        &self.overlays
    }

    /// Validates every policy and requires identities to be unique.
    ///
    /// Policies are checked in insertion order; the first error is returned.
    pub fn validate(&self) -> Result<(), OverlayError> {
        let mut ids = HashSet::with_capacity(self.overlays.len());
        for overlay in &self.overlays {
            overlay.policy.validate()?;
            if !ids.insert(&overlay.policy.id) {
                return Err(OverlayError::DuplicateId(overlay.policy.id.clone()));
            }
        }
        Ok(())
    }

    /// Clones overlays in paint order: ascending tier z-order first, then insertion order.
    ///
    /// Sorting is stable, so overlays in the same [`OverlayTier`] retain insertion order. Later
    /// items in the returned vector are intended to paint above earlier items.
    pub fn sorted_render_snapshot(&self) -> Vec<MullionOverlay> {
        let mut snapshot = self.overlays.clone();
        snapshot.sort_by_key(|overlay| overlay.policy.tier.z_order());
        snapshot
    }

    /// Applies all changes or none. Mutations are evaluated in the supplied order.
    ///
    /// Each mutation observes preceding mutations. If any mutation fails, or final validation
    /// fails, the original stack and its insertion order remain unchanged.
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

    /// Atomically appends a validated overlay with a unique identity.
    ///
    /// On error the stack is unchanged.
    pub fn push(&mut self, overlay: MullionOverlay) -> Result<(), OverlayError> {
        self.apply_atomic([OverlayMutation::Push(overlay)])
    }

    /// Atomically removes the overlay with `id` while preserving all other relative order.
    ///
    /// Returns [`OverlayError::UnknownId`] and leaves the stack unchanged if `id` is absent.
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
    /// Creates a pull-based source from a UI-thread callback.
    ///
    /// The callback is invoked for each requested snapshot; returned stacks are not validated by
    /// this type itself.
    pub fn new(source: impl Fn() -> OverlayStack + 'static) -> Self {
        Self(Rc::new(source))
    }

    /// Invokes the source and returns its current insertion-ordered stack.
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
    /// Creates host inputs from a controlled source with no dismissal handler.
    pub fn new(source: ControlledOverlaySource) -> Self {
        Self {
            source,
            on_dismiss: None,
        }
    }

    /// Creates host inputs directly from a pull-based stack callback.
    pub fn controlled(source: impl Fn() -> OverlayStack + 'static) -> Self {
        Self::new(ControlledOverlaySource::new(source))
    }

    /// Installs the callback invoked after a dismissible backdrop is clicked.
    ///
    /// Invocation requests a state change only: because the source is controlled, the handler is
    /// responsible for arranging removal of the identified overlay. Click-through overlays cannot
    /// be backdrop-dismissible and therefore never produce this backdrop-dismiss interaction.
    pub fn with_dismiss_handler(
        mut self,
        handler: impl Fn(&OverlayId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    /// Returns the pull-based controlled source.
    pub fn source(&self) -> &ControlledOverlaySource {
        &self.source
    }

    /// Returns the optional backdrop-dismiss request callback.
    pub fn on_dismiss(&self) -> Option<&OverlayDismissHandler> {
        self.on_dismiss.as_ref()
    }

    /// Pulls, validates, and sorts the current stack into paint order.
    ///
    /// The primary key is ascending tier z-order and the stable secondary key is insertion order.
    /// No overlays are returned if any policy or identity fails validation.
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

/// Failure to validate or mutate an overlay policy or stack.
#[derive(Clone, Debug, PartialEq)]
pub enum OverlayError {
    /// An overlay identity is empty.
    EmptyId,
    /// More than one overlay has the contained identity.
    DuplicateId(OverlayId),
    /// A mutation refers to an identity not present in the stack.
    UnknownId(OverlayId),
    /// A requested dimension is non-finite or outside the range allowed for its kind.
    InvalidDimension {
        /// The invalid policy field, currently `"width"` or `"height"`.
        field: &'static str,
        /// The rejected pixel or fractional value.
        value: f32,
    },
    /// A backdrop contains a non-finite channel or a channel outside `0.0..=1.0`.
    InvalidBackdrop,
    /// The identified overlay requests backdrop dismissal without having a backdrop.
    DismissWithoutBackdrop(OverlayId),
    /// The identified overlay combines click-through with backdrop dismissal.
    ClickThroughDismiss(OverlayId),
    /// The identified overlay combines click-through with accessible-modal behavior.
    ClickThroughModal(OverlayId),
    /// The identified overlay has an empty accessible label.
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
