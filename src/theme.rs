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
    pub drop_target: Hsla,
}

impl MullionTheme {
    pub fn dark() -> Self {
        Self {
            background: rgb(0x0e0e0e).into(),
            surface: rgb(0x151515).into(),
            border: rgb(0x303030).into(),
            accent: rgb(0x242424).into(),
            text: rgb(0xeeeeee).into(),
            muted_text: rgb(0x909090).into(),
            focused: rgb(0x62a0ea).into(),
            drop_target: rgb(0x355070).into(),
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
