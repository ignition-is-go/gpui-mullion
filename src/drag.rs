//! Portable drag-and-drop types and five-zone docking geometry.
//!
//! This module deliberately contains no GPUI types. A view adapter can translate
//! its pointer coordinates into [`DockPoint`] and [`DockBounds`], then retain a
//! typed [`DockDrag`] for the duration of the native drag.

use crate::{ActivityId, DropEdge, MullionModel, PaneData, PaneId, PaneNode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The operation represented by a dock drag.
///
/// Serde uses its default externally tagged enum representation: the variant
/// name (`Pane` or `NewActivity`) wraps the corresponding identifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPayload {
    /// Relocate an existing pane.
    Pane(PaneId),
    /// Create a new pane whose selected activity is this activity.
    NewActivity(ActivityId),
}

impl DockPayload {
    /// Returns the source pane for a pane relocation.
    pub fn pane(&self) -> Option<&PaneId> {
        match self {
            Self::Pane(pane) => Some(pane),
            Self::NewActivity(_) => None,
        }
    }

    /// Returns the requested activity for a new-pane drag.
    pub fn activity(&self) -> Option<&ActivityId> {
        match self {
            Self::Pane(_) => None,
            Self::NewActivity(activity) => Some(activity),
        }
    }

    /// Whether a drop on `destination` would target the pane being moved.
    ///
    /// A new-activity drag has no source pane and is therefore valid even in a
    /// single-pane layout.
    pub fn is_self_drop(&self, destination: &PaneId) -> bool {
        self.pane() == Some(destination)
    }

    /// Returns whether `destination` is not the source pane.
    pub fn can_drop_on(&self, destination: &PaneId) -> bool {
        !self.is_self_drop(destination)
    }
}

/// Typed state placed in a frontend's native drag system.
///
/// Its serialized form is an object containing a `payload` value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockDrag {
    /// Portable operation carried by the frontend's drag object.
    ///
    /// Serde represents `DockDrag` as an object with this `payload` field.
    pub payload: DockPayload,
}

impl DockDrag {
    /// Creates a drag that relocates `pane`.
    pub fn pane(pane: PaneId) -> Self {
        Self {
            payload: DockPayload::Pane(pane),
        }
    }

    /// Creates a drag that requests a new pane for `activity`.
    pub fn new_activity(activity: ActivityId) -> Self {
        Self {
            payload: DockPayload::NewActivity(activity),
        }
    }

    /// Returns whether dropping here would target the relocated source pane.
    pub fn is_self_drop(&self, destination: &PaneId) -> bool {
        self.payload.is_self_drop(destination)
    }

    /// Returns whether the drag can target `destination` under self-drop rules.
    pub fn can_drop_on(&self, destination: &PaneId) -> bool {
        self.payload.can_drop_on(destination)
    }
}

/// The resolved pane and zone currently under a dock drag.
///
/// Serde stores this as an object with `destination` and `edge` fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockHover {
    /// Pane currently beneath the pointer.
    pub destination: PaneId,
    /// Five-zone docking region within the destination.
    pub edge: DropEdge,
}

impl DockHover {
    /// Resolves a hover from target-normalized coordinates.
    ///
    /// Coordinates use fractions where `(0, 0)` is the target's top-left and
    /// `(1, 1)` its bottom-right. Returns `None` for non-finite coordinates.
    pub fn from_normalized(destination: PaneId, x: f64, y: f64) -> Option<Self> {
        Some(Self {
            destination,
            edge: DropEdge::from_normalized(x, y)?,
        })
    }

    /// Resolves a hover from a point and bounds in the same pixel coordinate space.
    ///
    /// Returns `None` for non-finite values or non-positive bounds extents.
    pub fn from_point(destination: PaneId, point: DockPoint, bounds: DockBounds) -> Option<Self> {
        Some(Self {
            destination,
            edge: DropEdge::from_point(point, bounds)?,
        })
    }

    /// Returns whether `drag` may be dropped on this hover's destination.
    pub fn accepts(&self, drag: &DockDrag) -> bool {
        drag.can_drop_on(&self.destination)
    }
}

/// A pointer position in an adapter-defined pixel coordinate space.
///
/// Serde stores the two floating-point components as `x` and `y`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockPoint {
    /// Horizontal pixel coordinate in the adapter's coordinate space.
    pub x: f64,
    /// Vertical pixel coordinate in the adapter's coordinate space.
    pub y: f64,
}

impl DockPoint {
    /// Creates a point in adapter-defined pixel units.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Pixel bounds for a pane drop target.
///
/// Serde stores `left`, `top`, `width`, and `height` without validation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockBounds {
    /// Horizontal coordinate of the target's left edge, in adapter pixels.
    pub left: f64,
    /// Vertical coordinate of the target's top edge, in adapter pixels.
    pub top: f64,
    /// Target width in adapter pixels; geometry operations require it to be positive.
    pub width: f64,
    /// Target height in adapter pixels; geometry operations require it to be positive.
    pub height: f64,
}

impl DockBounds {
    /// Creates bounds in adapter-defined pixel units.
    ///
    /// Construction does not validate values; geometry methods return `None`
    /// when a component is non-finite or an extent is not positive.
    pub const fn new(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    fn is_usable(self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
    }
}

/// Rectangle used to paint the active drop indicator.
///
/// Serde stores `left`, `top`, `width`, and `height`; their units depend on
/// whether the rectangle is normalized or translated into target pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockIndicator {
    /// Horizontal coordinate of the indicator's left edge.
    pub left: f64,
    /// Vertical coordinate of the indicator's top edge.
    pub top: f64,
    /// Indicator width.
    pub width: f64,
    /// Indicator height.
    pub height: f64,
}

impl DockIndicator {
    /// Creates indicator geometry.
    ///
    /// Values are normalized fractions when returned by
    /// [`DropEdge::normalized_indicator`] and adapter pixels when returned by
    /// [`DropEdge::indicator_in`].
    pub const fn new(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }
}

impl DropEdge {
    /// Resolve the reference five-zone algorithm from normalized coordinates.
    ///
    /// Left and right deliberately take priority in the corner regions. The
    /// quarter boundaries are strict: `x == 0.25` and `x == 0.75` remain in
    /// the middle column, as do the equivalent y boundaries. Non-finite input
    /// has no meaningful zone and returns `None`.
    pub fn from_normalized(x: f64, y: f64) -> Option<Self> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        Some(if x < 0.25 {
            Self::Left
        } else if x > 0.75 {
            Self::Right
        } else if y < 0.25 {
            Self::Top
        } else if y > 0.75 {
            Self::Bottom
        } else {
            Self::Center
        })
    }

    /// Resolve a zone from a pointer and pixel bounds.
    ///
    /// Zero/negative extents and any non-finite component are rejected rather
    /// than allowing division to produce a misleading edge.
    pub fn from_point(point: DockPoint, bounds: DockBounds) -> Option<Self> {
        if !point.x.is_finite() || !point.y.is_finite() || !bounds.is_usable() {
            return None;
        }
        Self::from_normalized(
            (point.x - bounds.left) / bounds.width,
            (point.y - bounds.top) / bounds.height,
        )
    }

    /// Indicator geometry in normalized target coordinates.
    ///
    /// Edge indicators cover the corresponding half; center covers the full
    /// target, exactly matching the reference overlay.
    pub const fn normalized_indicator(self) -> DockIndicator {
        match self {
            Self::Left => DockIndicator::new(0.0, 0.0, 0.5, 1.0),
            Self::Right => DockIndicator::new(0.5, 0.0, 0.5, 1.0),
            Self::Top => DockIndicator::new(0.0, 0.0, 1.0, 0.5),
            Self::Bottom => DockIndicator::new(0.0, 0.5, 1.0, 0.5),
            Self::Center => DockIndicator::new(0.0, 0.0, 1.0, 1.0),
        }
    }

    /// Indicator geometry translated into pixel bounds.
    pub fn indicator_in(self, bounds: DockBounds) -> Option<DockIndicator> {
        if !bounds.is_usable() {
            return None;
        }
        let indicator = self.normalized_indicator();
        Some(DockIndicator::new(
            bounds.left + indicator.left * bounds.width,
            bounds.top + indicator.top * bounds.height,
            indicator.width * bounds.width,
            indicator.height * bounds.height,
        ))
    }
}

/// Host hook that mints a pane for a dropped activity.
///
/// The destination and edge let the host inherit nearby context. Returning
/// `None` refuses the drop without mutating the model.
pub type NewPaneFactory<D> =
    Arc<dyn Fn(&ActivityId, &PaneId, DropEdge) -> Option<(PaneId, D)> + Send + Sync>;

/// Activity-to-new-pane docking policy.
///
/// Without a host override, Mullion clones the destination pane data and mints
/// a collision-free internal id. Hosts can override this for durable identity
/// or custom pane initialization.
#[derive(Clone)]
pub struct DockConfig<D: PaneData> {
    new_pane_factory: Option<NewPaneFactory<D>>,
}

impl<D: PaneData> Default for DockConfig<D> {
    fn default() -> Self {
        Self {
            new_pane_factory: None,
        }
    }
}

impl<D: PaneData> DockConfig<D> {
    /// Creates a configuration using Mullion's default cloning policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs a host factory and returns the configuration.
    ///
    /// Returning `None` from the factory refuses the drop without mutation.
    pub fn with_new_pane_factory(
        mut self,
        factory: impl Fn(&ActivityId, &PaneId, DropEdge) -> Option<(PaneId, D)> + Send + Sync + 'static,
    ) -> Self {
        self.new_pane_factory = Some(Arc::new(factory));
        self
    }

    /// Returns the installed host factory override, if present.
    pub fn new_pane_factory(&self) -> Option<&NewPaneFactory<D>> {
        self.new_pane_factory.as_ref()
    }

    /// Replaces the host override. Passing `None` restores default cloning.
    pub fn set_new_pane_factory(&mut self, factory: Option<NewPaneFactory<D>>) {
        self.new_pane_factory = factory;
    }

    /// Returns whether activity drops can create panes.
    ///
    /// This is always true because default docking clones destination pane data.
    pub const fn can_create_panes(&self) -> bool {
        true
    }

    /// Ask the host to mint a pane, then pass it to
    /// [`MullionModel::drop_activity`].
    ///
    /// A configured factory may refuse by returning `None`. Without an override,
    /// the destination pane's data is cloned and an internal collision-free id is
    /// generated. Model validation remains the final error channel.
    pub fn drop_activity(
        &self,
        model: &mut MullionModel<D>,
        activity: &ActivityId,
        destination: &PaneId,
        edge: DropEdge,
    ) -> bool {
        let generated = if let Some(factory) = &self.new_pane_factory {
            factory(activity, destination, edge)
        } else {
            let data = match model.tree().find(destination) {
                Some(PaneNode::Leaf { data, .. }) => data.clone(),
                _ => return false,
            };
            Some((
                crate::tree::internal_pane_id(model.tree(), destination, "activity"),
                data,
            ))
        };
        let Some((new_id, new_data)) = generated else {
            return false;
        };
        model.drop_activity(activity, destination, edge, new_id, new_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaneNode;

    const EDGES: [DropEdge; 5] = [
        DropEdge::Left,
        DropEdge::Right,
        DropEdge::Top,
        DropEdge::Bottom,
        DropEdge::Center,
    ];

    #[test]
    fn five_zone_table_has_strict_boundaries_and_horizontal_corner_priority() {
        let cases = [
            ((0.249_999, 0.0), DropEdge::Left),
            ((0.25, 0.0), DropEdge::Top),
            ((0.75, 0.0), DropEdge::Top),
            ((0.750_001, 0.0), DropEdge::Right),
            ((0.0, 1.0), DropEdge::Left),
            ((1.0, 1.0), DropEdge::Right),
            ((0.5, 0.249_999), DropEdge::Top),
            ((0.5, 0.25), DropEdge::Center),
            ((0.5, 0.75), DropEdge::Center),
            ((0.5, 0.750_001), DropEdge::Bottom),
            ((0.5, 0.5), DropEdge::Center),
        ];
        for ((x, y), expected) in cases {
            assert_eq!(DropEdge::from_normalized(x, y), Some(expected), "{x}, {y}");
        }
    }

    #[test]
    fn normalized_grid_exhaustively_matches_reference_ordering() {
        let values = [-1.0, 0.0, 0.25, 0.5, 0.75, 1.0, 2.0];
        for x in values {
            for y in values {
                let expected = if x < 0.25 {
                    DropEdge::Left
                } else if x > 0.75 {
                    DropEdge::Right
                } else if y < 0.25 {
                    DropEdge::Top
                } else if y > 0.75 {
                    DropEdge::Bottom
                } else {
                    DropEdge::Center
                };
                assert_eq!(DropEdge::from_normalized(x, y), Some(expected));
            }
        }
    }

    #[test]
    fn pixel_and_normalized_calculations_are_equivalent_under_translation_and_scale() {
        for left in [-100.0, 0.0, 37.5] {
            for top in [-20.0, 0.0, 91.0] {
                for (width, height) in [(1.0, 1.0), (320.0, 180.0), (0.5, 9.0)] {
                    let bounds = DockBounds::new(left, top, width, height);
                    for x in [-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
                        for y in [-0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
                            let point = DockPoint::new(left + x * width, top + y * height);
                            assert_eq!(
                                DropEdge::from_point(point, bounds),
                                DropEdge::from_normalized(x, y)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_coordinates_and_bounds_never_produce_a_zone_or_indicator() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(DropEdge::from_normalized(invalid, 0.5), None);
            assert_eq!(DropEdge::from_normalized(0.5, invalid), None);
            assert_eq!(
                DropEdge::from_point(
                    DockPoint::new(invalid, 0.0),
                    DockBounds::new(0.0, 0.0, 10.0, 10.0)
                ),
                None
            );
            assert_eq!(
                DropEdge::Center.indicator_in(DockBounds::new(invalid, 0.0, 1.0, 1.0)),
                None
            );
        }
        for (width, height) in [(0.0, 1.0), (-1.0, 1.0), (1.0, 0.0), (1.0, -1.0)] {
            let bounds = DockBounds::new(0.0, 0.0, width, height);
            assert_eq!(DropEdge::from_point(DockPoint::new(0.0, 0.0), bounds), None);
            assert_eq!(DropEdge::Center.indicator_in(bounds), None);
        }
    }

    #[test]
    fn every_indicator_is_the_reference_half_or_whole_rectangle() {
        let normalized = [
            DockIndicator::new(0.0, 0.0, 0.5, 1.0),
            DockIndicator::new(0.5, 0.0, 0.5, 1.0),
            DockIndicator::new(0.0, 0.0, 1.0, 0.5),
            DockIndicator::new(0.0, 0.5, 1.0, 0.5),
            DockIndicator::new(0.0, 0.0, 1.0, 1.0),
        ];
        let bounds = DockBounds::new(10.0, 20.0, 80.0, 40.0);
        let pixels = [
            DockIndicator::new(10.0, 20.0, 40.0, 40.0),
            DockIndicator::new(50.0, 20.0, 40.0, 40.0),
            DockIndicator::new(10.0, 20.0, 80.0, 20.0),
            DockIndicator::new(10.0, 40.0, 80.0, 20.0),
            DockIndicator::new(10.0, 20.0, 80.0, 40.0),
        ];
        for ((edge, normalized), pixels) in EDGES.into_iter().zip(normalized).zip(pixels) {
            assert_eq!(edge.normalized_indicator(), normalized);
            assert_eq!(edge.indicator_in(bounds), Some(pixels));
        }
    }

    #[test]
    fn payload_accessors_and_self_drop_rules_distinguish_both_operations() {
        let pane = PaneId::new("pane");
        let other = PaneId::new("other");
        let activity = ActivityId::new("activity");
        let moving = DockDrag::pane(pane.clone());
        assert_eq!(moving.payload.pane(), Some(&pane));
        assert_eq!(moving.payload.activity(), None);
        assert!(moving.is_self_drop(&pane));
        assert!(!moving.can_drop_on(&pane));
        assert!(moving.can_drop_on(&other));

        let creating = DockDrag::new_activity(activity.clone());
        assert_eq!(creating.payload.activity(), Some(&activity));
        assert_eq!(creating.payload.pane(), None);
        assert!(!creating.is_self_drop(&pane));
        assert!(creating.can_drop_on(&pane));
        let hover = DockHover::from_normalized(pane, 0.5, 0.5).unwrap();
        assert!(hover.accepts(&creating));
    }

    #[test]
    fn payload_and_hover_round_trip_through_serde() {
        let drag = DockDrag::new_activity(ActivityId::new("logs"));
        let hover = DockHover {
            destination: PaneId::new("main"),
            edge: DropEdge::Bottom,
        };
        assert_eq!(
            serde_json::from_str::<DockDrag>(&serde_json::to_string(&drag).unwrap()).unwrap(),
            drag
        );
        assert_eq!(
            serde_json::from_str::<DockHover>(&serde_json::to_string(&hover).unwrap()).unwrap(),
            hover
        );
    }

    #[test]
    fn dock_config_refusal_is_inert_and_success_delegates_to_model() {
        let destination = PaneId::new("only");
        let activity = ActivityId::new("logs");
        let mut default_model =
            MullionModel::new(PaneNode::leaf(destination.clone(), "old".to_string()));
        let defaults = DockConfig::new();
        assert!(defaults.drop_activity(
            &mut default_model,
            &activity,
            &destination,
            DropEdge::Right
        ));
        assert!(matches!(
            default_model.tree().find(&PaneId::new("only--activity")),
            Some(PaneNode::Leaf {
                active_activity: Some(active),
                data,
                ..
            }) if active == &activity && data == "old"
        ));
        assert!(defaults.drop_activity(
            &mut default_model,
            &activity,
            &destination,
            DropEdge::Left
        ));
        assert!(default_model
            .tree()
            .find(&PaneId::new("only--activity-2"))
            .is_some());

        let mut model = MullionModel::new(PaneNode::leaf(destination.clone(), "old".to_string()));
        let refusing = DockConfig::new().with_new_pane_factory(|_, _, _| None);
        assert!(!refusing.drop_activity(&mut model, &activity, &destination, DropEdge::Right));
        assert_eq!(model.tree().leaf_ids(), vec![destination.clone()]);

        let config = DockConfig::new().with_new_pane_factory(|activity, destination, edge| {
            assert_eq!(activity, &ActivityId::new("logs"));
            assert_eq!(destination, &PaneId::new("only"));
            assert_eq!(edge, DropEdge::Left);
            Some((PaneId::new("new"), "new-data".to_string()))
        });
        assert!(config.can_create_panes());
        assert!(config.drop_activity(&mut model, &activity, &destination, DropEdge::Left));
        assert_eq!(
            model.tree().leaf_ids(),
            vec![PaneId::new("new"), destination]
        );
        assert_eq!(model.focused(), Some(&PaneId::new("new")));
    }
}
