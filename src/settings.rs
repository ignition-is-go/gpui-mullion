//! Typed live settings and serializable focus presentation preferences.
//!
//! [`MullionSettings`] can either be controlled by a host application's source
//! of truth or backed by local shared state. [`MullionConfig`] combines the
//! behavior setting with visual presentation values suitable for persistence.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Deserializer, Serialize};

use crate::PaneFocusBehavior;

/// Stable identifier used by host settings registries.
pub const FOCUS_BEHAVIOR_SETTING_ID: &str = "mullion.focus_behavior";

/// One allowed value in a typed Mullion setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MullionSettingOption<T: 'static> {
    value: T,
    label: &'static str,
    description: &'static str,
}

impl<T: 'static> MullionSettingOption<T> {
    /// Creates an option with its typed value and static user-facing metadata.
    pub const fn new(value: T, label: &'static str, description: &'static str) -> Self {
        Self {
            value,
            label,
            description,
        }
    }

    /// Returns the typed value selected by this option.
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the short user-facing name shown for this option.
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// Returns the user-facing explanation of this option's behavior.
    pub const fn description(&self) -> &'static str {
        self.description
    }
}

/// A typed, live setting suitable for registration in a host application's UI.
///
/// The callbacks deliberately do not depend on a particular GPUI settings
/// store. A host can read an application global in `get_value` and dispatch an
/// action or persist a preference in `set_value`.
pub struct MullionSetting<T: Clone + Send + Sync + 'static> {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    options: &'static [MullionSettingOption<T>],
    get_value: Arc<dyn Fn() -> T + Send + Sync>,
    set_value: Arc<dyn Fn(T) + Send + Sync>,
}

impl<T: Clone + Send + Sync + 'static> Clone for MullionSetting<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            label: self.label,
            description: self.description,
            options: self.options,
            get_value: self.get_value.clone(),
            set_value: self.set_value.clone(),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> MullionSetting<T> {
    fn new(
        id: &'static str,
        label: &'static str,
        description: &'static str,
        options: &'static [MullionSettingOption<T>],
        get_value: impl Fn() -> T + Send + Sync + 'static,
        set_value: impl Fn(T) + Send + Sync + 'static,
    ) -> Self {
        Self {
            id,
            label,
            description,
            options,
            get_value: Arc::new(get_value),
            set_value: Arc::new(set_value),
        }
    }

    /// Returns the stable identifier used to register or persist the setting.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Returns the user-facing name of the setting.
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// Returns the user-facing explanation of what the setting controls.
    pub const fn description(&self) -> &'static str {
        self.description
    }

    /// Returns every value offered by this setting, in display order.
    pub const fn options(&self) -> &'static [MullionSettingOption<T>] {
        self.options
    }

    /// Read the latest value from the host's source of truth.
    pub fn get(&self) -> T {
        (self.get_value)()
    }

    /// Pass a new value to the host-provided callback.
    pub fn set(&self, value: T) {
        (self.set_value)(value);
    }
}

const FOCUS_BEHAVIOR_OPTIONS: [MullionSettingOption<PaneFocusBehavior>; 2] = [
    MullionSettingOption::new(
        PaneFocusBehavior::Click,
        "Click",
        "Focus a pane when it is clicked and keep focus there.",
    ),
    MullionSettingOption::new(
        PaneFocusBehavior::Hover,
        "Hover",
        "Move focus whenever the pointer enters another pane.",
    ),
];

/// Persistable preferences understood by Mullion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MullionSettingsConfig {
    /// Controls whether clicking or pointer hover transfers focus between panes.
    pub focus_behavior: PaneFocusBehavior,
}

/// Live preferences shared by the pane system and its GPUI host.
///
/// Use [`Self::controlled`] for application-owned state and [`Self::local`]
/// for a self-contained handle. Clones stay connected to the same callbacks.
#[derive(Clone)]
pub struct MullionSettings {
    focus_behavior: MullionSetting<PaneFocusBehavior>,
}

impl MullionSettings {
    /// Creates settings controlled by a host-owned source of truth.
    ///
    /// Every read invokes `get_focus_behavior`, and every write invokes
    /// `set_focus_behavior`. Calling a setter does not itself retain the value;
    /// the getter must subsequently expose any update accepted by the host.
    pub fn controlled(
        get_focus_behavior: impl Fn() -> PaneFocusBehavior + Send + Sync + 'static,
        set_focus_behavior: impl Fn(PaneFocusBehavior) + Send + Sync + 'static,
    ) -> Self {
        Self {
            focus_behavior: MullionSetting::new(
                FOCUS_BEHAVIOR_SETTING_ID,
                "Pane focus behavior",
                "Choose whether pointer hover or click changes the focused pane.",
                &FOCUS_BEHAVIOR_OPTIONS,
                get_focus_behavior,
                set_focus_behavior,
            ),
        }
    }

    /// Creates a self-contained setting initialized to `focus_behavior`.
    ///
    /// Clones share the same thread-safe in-memory value. This is convenient
    /// when no application-level settings store controls the value.
    pub fn local(focus_behavior: PaneFocusBehavior) -> Self {
        let value = Arc::new(RwLock::new(focus_behavior));
        let read_value = value.clone();
        Self::controlled(
            move || *read_value.read().unwrap_or_else(|error| error.into_inner()),
            move |next| {
                *value.write().unwrap_or_else(|error| error.into_inner()) = next;
            },
        )
    }

    /// Creates locally backed live settings initialized from persisted `config`.
    pub fn from_config(config: MullionSettingsConfig) -> Self {
        Self::local(config.focus_behavior)
    }

    /// Returns the live descriptor for the pane focus behavior setting.
    ///
    /// The returned clone remains connected to the same getter and setter.
    pub fn focus_behavior_setting(&self) -> MullionSetting<PaneFocusBehavior> {
        self.focus_behavior.clone()
    }

    /// Reads the current pane focus behavior from the backing source of truth.
    pub fn focus_behavior(&self) -> PaneFocusBehavior {
        self.focus_behavior.get()
    }

    /// Requests that the backing source set the pane focus behavior.
    ///
    /// For controlled settings, whether the request is retained is determined
    /// by the host-provided callback.
    pub fn set_focus_behavior(&self, focus_behavior: PaneFocusBehavior) {
        self.focus_behavior.set(focus_behavior);
    }

    /// Snapshot the current live values for persistence.
    pub fn config(&self) -> MullionSettingsConfig {
        MullionSettingsConfig {
            focus_behavior: self.focus_behavior(),
        }
    }

    /// Apply persisted values through the host callback.
    pub fn apply_config(&self, config: MullionSettingsConfig) {
        self.set_focus_behavior(config.focus_behavior);
    }
}

impl Default for MullionSettings {
    fn default() -> Self {
        Self::from_config(MullionSettingsConfig::default())
    }
}

/// Visual focus treatment. Defaults are compatibility-safe and add no chrome.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct FocusPresentation {
    show_focus_indicator: bool,
    unfocused_pane_opacity: f64,
}

impl FocusPresentation {
    /// Creates the compatibility-safe presentation with no indicator or dimming.
    pub const fn new() -> Self {
        Self {
            show_focus_indicator: false,
            unfocused_pane_opacity: 1.0,
        }
    }

    /// Returns whether the focused pane should display an internal-edge indicator.
    pub const fn show_focus_indicator(&self) -> bool {
        self.show_focus_indicator
    }

    /// Returns the opacity applied to panes that do not hold focus.
    ///
    /// The value is a unitless alpha fraction in `0.0..=1.0`, where `0.0` is
    /// fully transparent and `1.0` is fully opaque (no dimming).
    pub const fn unfocused_pane_opacity(&self) -> f64 {
        self.unfocused_pane_opacity
    }

    /// Opt in to an internal-edge indicator around the focused pane.
    pub const fn with_focus_indicator(mut self, visible: bool) -> Self {
        self.show_focus_indicator = visible;
        self
    }

    /// Set inactive pane opacity. Finite values are clamped to `0.0..=1.0`;
    /// non-finite values restore the disabled value of `1.0`.
    pub fn with_unfocused_pane_opacity(mut self, opacity: f64) -> Self {
        self.unfocused_pane_opacity = normalize_pane_opacity(opacity);
        self
    }
}

impl Default for FocusPresentation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct FocusPresentationWire {
    show_focus_indicator: bool,
    unfocused_pane_opacity: f64,
}

impl Default for FocusPresentationWire {
    fn default() -> Self {
        Self {
            show_focus_indicator: false,
            unfocused_pane_opacity: 1.0,
        }
    }
}

impl<'de> Deserialize<'de> for FocusPresentation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = FocusPresentationWire::deserialize(deserializer)?;
        Ok(Self::new()
            .with_focus_indicator(wire.show_focus_indicator)
            .with_unfocused_pane_opacity(wire.unfocused_pane_opacity))
    }
}

/// Complete serializable focus configuration for a Mullion view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MullionConfig {
    /// Persistable interaction behavior controlled through [`MullionSettings`].
    pub settings: MullionSettingsConfig,
    /// Visual treatment of focused and unfocused panes.
    pub presentation: FocusPresentation,
}

fn normalize_pane_opacity(opacity: f64) -> f64 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[test]
    fn local_setting_updates_every_clone() {
        let settings = MullionSettings::local(PaneFocusBehavior::Click);
        let clone = settings.clone();
        clone.set_focus_behavior(PaneFocusBehavior::Hover);
        assert_eq!(settings.focus_behavior(), PaneFocusBehavior::Hover);
    }

    #[test]
    fn controlled_setting_uses_typed_host_callbacks() {
        let host = Arc::new(AtomicU8::new(1));
        let reader = host.clone();
        let writer = host.clone();
        let settings = MullionSettings::controlled(
            move || match reader.load(Ordering::SeqCst) {
                0 => PaneFocusBehavior::Hover,
                _ => PaneFocusBehavior::Click,
            },
            move |value| {
                writer.store(
                    u8::from(value == PaneFocusBehavior::Click),
                    Ordering::SeqCst,
                );
            },
        );

        assert_eq!(settings.focus_behavior(), PaneFocusBehavior::Click);
        settings.set_focus_behavior(PaneFocusBehavior::Hover);
        assert_eq!(settings.focus_behavior(), PaneFocusBehavior::Hover);
        assert_eq!(host.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn descriptor_metadata_and_all_options_are_stable() {
        let setting = MullionSettings::default().focus_behavior_setting();
        assert_eq!(setting.id(), FOCUS_BEHAVIOR_SETTING_ID);
        assert_eq!(setting.label(), "Pane focus behavior");
        assert!(!setting.description().is_empty());
        assert_eq!(
            setting
                .options()
                .iter()
                .map(|option| (*option.value(), option.label()))
                .collect::<Vec<_>>(),
            vec![
                (PaneFocusBehavior::Click, "Click"),
                (PaneFocusBehavior::Hover, "Hover"),
            ]
        );
        assert!(setting
            .options()
            .iter()
            .all(|option| !option.description().is_empty()));
    }

    #[test]
    fn settings_config_defaults_round_trips_and_applies() {
        let default: MullionSettingsConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default.focus_behavior, PaneFocusBehavior::Hover);
        let encoded = serde_json::to_string(&MullionSettingsConfig {
            focus_behavior: PaneFocusBehavior::Click,
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<MullionSettingsConfig>(&encoded).unwrap(),
            MullionSettingsConfig {
                focus_behavior: PaneFocusBehavior::Click,
            }
        );
        let settings = MullionSettings::default();
        settings.apply_config(serde_json::from_str(&encoded).unwrap());
        assert_eq!(settings.config().focus_behavior, PaneFocusBehavior::Click);
    }

    #[test]
    fn presentation_is_opt_in_and_clamps_every_finite_range() {
        let default = FocusPresentation::default();
        assert!(!default.show_focus_indicator());
        assert_eq!(default.unfocused_pane_opacity(), 1.0);
        assert!(default.with_focus_indicator(true).show_focus_indicator());
        for (input, expected) in [
            (-1.0, 0.0),
            (0.0, 0.0),
            (0.75, 0.75),
            (1.0, 1.0),
            (2.0, 1.0),
        ] {
            assert_eq!(
                default
                    .with_unfocused_pane_opacity(input)
                    .unfocused_pane_opacity(),
                expected
            );
        }
        for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                default
                    .with_unfocused_pane_opacity(input)
                    .unfocused_pane_opacity(),
                1.0
            );
        }
    }

    #[test]
    fn deserialization_applies_defaults_and_normalization() {
        let presentation: FocusPresentation = serde_json::from_str("{}").unwrap();
        assert_eq!(presentation, FocusPresentation::default());
        let low: FocusPresentation =
            serde_json::from_str(r#"{"show_focus_indicator":true,"unfocused_pane_opacity":-3.0}"#)
                .unwrap();
        assert!(low.show_focus_indicator());
        assert_eq!(low.unfocused_pane_opacity(), 0.0);
        let high: FocusPresentation =
            serde_json::from_str(r#"{"unfocused_pane_opacity":3.0}"#).unwrap();
        assert_eq!(high.unfocused_pane_opacity(), 1.0);
    }

    #[test]
    fn complete_config_has_stable_serialized_shape() {
        let config = MullionConfig {
            settings: MullionSettingsConfig {
                focus_behavior: PaneFocusBehavior::Click,
            },
            presentation: FocusPresentation::new()
                .with_focus_indicator(true)
                .with_unfocused_pane_opacity(0.75),
        };
        let json = serde_json::to_value(config).unwrap();
        assert_eq!(json["settings"]["focus_behavior"], "Click");
        assert_eq!(json["presentation"]["show_focus_indicator"], true);
        assert_eq!(json["presentation"]["unfocused_pane_opacity"], 0.75);
        assert_eq!(
            serde_json::from_value::<MullionConfig>(json).unwrap(),
            config
        );
    }
}
