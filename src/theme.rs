use gpui::{rgb, App, Hsla, WindowAppearance};
use serde::{Deserialize, Serialize};

/// Colors used to render Mullion chrome and pane state.
///
/// All values are GPUI HSLA colors, including their alpha component. The
/// built-in palettes are fully opaque except where a translucent overlay is
/// explicitly useful, such as [`Self::drop_target`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MullionTheme {
    /// Background behind the workspace and its panes.
    pub background: Hsla,
    /// Background of pane surfaces and other raised content.
    pub surface: Hsla,
    /// Color of pane borders and separators.
    pub border: Hsla,
    /// Background used for emphasized controls and chrome.
    pub accent: Hsla,
    /// Primary foreground color for labels and content.
    pub text: Hsla,
    /// Secondary foreground color for de-emphasized labels.
    pub muted_text: Hsla,
    /// Background used to distinguish a focused pane.
    pub focused: Hsla,
    /// Dedicated focus fallback: 65% #00a4ef mixed with #1a1a1a in sRGB.
    pub focus_indicator: Hsla,
    /// Overlay used to preview a drag-and-drop destination.
    pub drop_target: Hsla,
}

impl MullionTheme {
    /// Return the built-in dark palette.
    pub fn dark() -> Self {
        Self {
            background: rgb(0x0e0e0e).into(),
            surface: rgb(0x111111).into(),
            border: rgb(0x1a1a1a).into(),
            accent: rgb(0x222222).into(),
            text: rgb(0xeeeeee).into(),
            muted_text: rgb(0x888888).into(),
            focused: rgb(0x333333).into(),
            focus_indicator: rgb(0x0974a4).into(),
            drop_target: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.06,
            },
        }
    }

    /// Return the built-in light palette.
    pub fn light() -> Self {
        Self {
            background: rgb(0xf4f4f4).into(),
            surface: rgb(0xffffff).into(),
            border: rgb(0xd0d0d0).into(),
            accent: rgb(0xe8e8e8).into(),
            text: rgb(0x202020).into(),
            muted_text: rgb(0x666666).into(),
            focused: rgb(0x0067c0).into(),
            focus_indicator: rgb(0x0067c0).into(),
            drop_target: rgb(0xb8d8f2).into(),
        }
    }
}

impl Default for MullionTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Persistable theme policy. `System` resolves against GPUI's window
/// appearance, including vibrant light/dark variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MullionThemeMode {
    /// Always use the built-in light palette.
    Light,
    /// Always use the built-in dark palette.
    Dark,
    /// Follow the current GPUI window appearance.
    #[default]
    System,
}

impl MullionThemeMode {
    /// Resolve this policy to a concrete palette for `appearance`.
    ///
    /// Explicit light and dark policies ignore `appearance`; [`Self::System`]
    /// maps both regular and vibrant appearances to their matching palette.
    pub fn resolve(self, appearance: WindowAppearance) -> MullionTheme {
        match self {
            Self::Light => MullionTheme::light(),
            Self::Dark => MullionTheme::dark(),
            Self::System => match appearance {
                WindowAppearance::Light | WindowAppearance::VibrantLight => MullionTheme::light(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark => MullionTheme::dark(),
            },
        }
    }
}

/// UI-local semantic theme source evaluated once per Mullion root render.
///
/// The host is responsible for invalidation when provider state changes. Pulse,
/// for example, refreshes windows after replacing its app-global theme.
pub type MullionThemeProvider = std::rc::Rc<dyn Fn(&App) -> MullionTheme>;

/// UI-local resolved-appearance source evaluated once per Mullion root render.
///
/// Most hosts should prefer [`MullionThemeProvider`] so Mullion derives all
/// component tokens consistently. This advanced provider exists for applications
/// that dynamically replace exact geometry or component-specific colors.
pub type MullionAppearanceProvider = std::rc::Rc<dyn Fn(&App) -> crate::MullionAppearance>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_palette_matches_reference_literals() {
        let theme = MullionTheme::dark();
        assert_eq!(theme.background, rgb(0x0e0e0e).into());
        assert_eq!(theme.surface, rgb(0x111111).into());
        assert_eq!(theme.border, rgb(0x1a1a1a).into());
        assert_eq!(theme.accent, rgb(0x222222).into());
        assert_eq!(theme.text, rgb(0xeeeeee).into());
        assert_eq!(theme.muted_text, rgb(0x888888).into());
        assert_eq!(theme.focused, rgb(0x333333).into());
        assert_eq!(theme.focus_indicator, rgb(0x0974a4).into());
        assert_eq!(theme.drop_target.a, 0.06);
    }

    #[test]
    fn system_mode_tracks_all_gpui_appearances() {
        let light = MullionThemeMode::System.resolve(WindowAppearance::VibrantLight);
        let dark = MullionThemeMode::System.resolve(WindowAppearance::VibrantDark);
        assert_eq!(light.background, MullionTheme::light().background);
        assert_eq!(dark.background, MullionTheme::dark().background);
    }

    #[test]
    fn theme_mode_has_stable_serde() {
        assert_eq!(
            serde_json::to_string(&MullionThemeMode::System).unwrap(),
            r#""System""#
        );
        assert_eq!(
            serde_json::from_str::<MullionThemeMode>(r#""Light""#).unwrap(),
            MullionThemeMode::Light
        );
    }
}
