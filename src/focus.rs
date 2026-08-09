use serde::{Deserialize, Serialize};

/// How pointer interaction changes the focused pane.
///
/// Programmatic focus is independent of this policy. UI adapters should use
/// this value only when translating pointer input into a focus request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneFocusBehavior {
    /// Focus follows the pointer as it enters panes.
    #[default]
    Hover,
    /// Focus changes on pointer press and remains until another focus request.
    Click,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_is_the_compatibility_default() {
        assert_eq!(PaneFocusBehavior::default(), PaneFocusBehavior::Hover);
    }

    #[test]
    fn every_behavior_has_a_stable_serde_representation() {
        for (behavior, encoded) in [
            (PaneFocusBehavior::Hover, r#""Hover""#),
            (PaneFocusBehavior::Click, r#""Click""#),
        ] {
            assert_eq!(serde_json::to_string(&behavior).unwrap(), encoded);
            assert_eq!(
                serde_json::from_str::<PaneFocusBehavior>(encoded).unwrap(),
                behavior
            );
        }
    }
}
