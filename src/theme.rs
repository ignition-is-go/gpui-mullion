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

/// UI-local source evaluated once per Mullion root render.
///
/// The host is responsible for invalidation when provider state changes. Pulse,
/// for example, refreshes windows after replacing its app-global theme.
pub type MullionAppearanceProvider = std::rc::Rc<dyn Fn(&App) -> MullionAppearance>;

/// Complete source used to resolve Mullion's visual appearance.
///
/// This is the canonical look configuration accepted by
/// [`crate::MullionView::with_appearance`]. A theme mode follows a built-in
/// light/dark policy, a theme derives standard component geometry from a custom
/// palette, and styles preserve an exact fully resolved component snapshot.
#[derive(Clone, Debug, PartialEq)]
pub enum MullionAppearance {
    /// Resolve a built-in palette from a fixed or system-following policy.
    Mode(MullionThemeMode),
    /// Derive standard component styles from this exact palette.
    Theme(MullionTheme),
    /// Use a complete resolved style snapshot and semantic palette without merging.
    Styles {
        /// Semantic colors supplied to host-rendered chrome callbacks.
        theme: MullionTheme,
        /// Exact resolved Mullion component tokens.
        styles: std::rc::Rc<crate::styles::MullionStyles>,
    },
}

impl MullionAppearance {
    /// Follow the current GPUI window appearance.
    pub const fn system() -> Self {
        Self::Mode(MullionThemeMode::System)
    }

    /// Always derive styles from the built-in light palette.
    pub const fn light() -> Self {
        Self::Mode(MullionThemeMode::Light)
    }

    /// Always derive styles from the built-in dark palette.
    pub const fn dark() -> Self {
        Self::Mode(MullionThemeMode::Dark)
    }

    /// Derive standard component styles from a custom palette.
    pub const fn theme(theme: MullionTheme) -> Self {
        Self::Theme(theme)
    }

    /// Use a fully resolved component-style snapshot.
    pub fn styles(styles: crate::styles::MullionStyles) -> Self {
        Self::Styles {
            theme: MullionTheme::default(),
            styles: std::rc::Rc::new(styles),
        }
    }

    /// Use a custom semantic palette and exact resolved component tokens together.
    pub fn custom(theme: MullionTheme, styles: crate::styles::MullionStyles) -> Self {
        Self::Styles {
            theme,
            styles: std::rc::Rc::new(styles),
        }
    }

    /// Resolve the semantic palette and complete tokens for a GPUI window appearance.
    pub fn resolve(
        &self,
        appearance: WindowAppearance,
    ) -> (MullionTheme, crate::styles::MullionStyles) {
        let theme = match self {
            Self::Mode(mode) => mode.resolve(appearance),
            Self::Theme(theme) => *theme,
            Self::Styles { theme, styles } => return (*theme, **styles),
        };
        (theme, crate::styles::MullionStyles::from_theme(theme))
    }
}

impl Default for MullionAppearance {
    fn default() -> Self {
        Self::Theme(MullionTheme::default())
    }
}

impl From<MullionThemeMode> for MullionAppearance {
    fn from(mode: MullionThemeMode) -> Self {
        Self::Mode(mode)
    }
}

impl From<MullionTheme> for MullionAppearance {
    fn from(theme: MullionTheme) -> Self {
        Self::Theme(theme)
    }
}

impl From<crate::styles::MullionStyles> for MullionAppearance {
    fn from(styles: crate::styles::MullionStyles) -> Self {
        Self::styles(styles)
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

    #[test]
    fn appearance_resolves_one_complete_look_source() {
        let (light_theme, light_styles) =
            MullionAppearance::system().resolve(WindowAppearance::VibrantLight);
        assert_eq!(light_theme, MullionTheme::light());
        assert_eq!(light_styles, crate::MullionStyles::from_theme(light_theme));

        let theme = MullionTheme::dark();
        let mut styles = crate::MullionStyles::from_theme(theme);
        styles.activity_bar.thickness = gpui::px(51.0);
        let (resolved_theme, resolved_styles) =
            MullionAppearance::custom(theme, styles).resolve(WindowAppearance::Light);
        assert_eq!(resolved_theme, theme);
        assert_eq!(resolved_styles, styles);
    }
}
