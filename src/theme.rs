use gpui::{rgb, Hsla, WindowAppearance};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct MullionTheme {
    pub background: Hsla,
    pub surface: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub text: Hsla,
    pub muted_text: Hsla,
    pub focused: Hsla,
    /// Dedicated focus fallback: 65% #00a4ef mixed with #1a1a1a in sRGB.
    pub focus_indicator: Hsla,
    pub drop_target: Hsla,
}

impl MullionTheme {
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
    Light,
    Dark,
    #[default]
    System,
}

impl MullionThemeMode {
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
