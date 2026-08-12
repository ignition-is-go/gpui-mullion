//! Product-neutral Lucide icons for GPUI.
//!
//! The icon font is embedded in this crate through [`lucide_icons`], so both
//! native and WebAssembly applications render the same glyphs without a network
//! request or host asset source. Call [`crate::icons::install`] before opening a window.

use gpui::{
    div, prelude::*, App, Div, ElementId, Global, Hsla, Pixels, RenderOnce, Result, SharedString,
    Window,
};
use std::borrow::Cow;

/// The canonical typed icon value accepted by Mullion visual APIs.
pub use lucide_icons::Icon as LucideIcon;

/// Font family recorded in the bundled Lucide typeface.
pub const FAMILY: &str = "lucide";

struct Installed;
impl Global for Installed {}

/// Register the bundled Lucide font with GPUI.
///
/// Registration is idempotent for each [`App`]. The font bytes are compiled
/// into native and WebAssembly binaries; this performs no network or filesystem
/// I/O at runtime.
pub fn install(cx: &mut App) -> Result<()> {
    if cx.has_global::<Installed>() {
        return Ok(());
    }
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(lucide_icons::LUCIDE_FONT_BYTES)])?;
    cx.set_global(Installed);
    Ok(())
}

fn glyph(icon: LucideIcon) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .font_family(FAMILY)
        .child(icon.unicode().to_string())
}

/// A typed Lucide glyph rendered as an ordinary, caller-owned GPUI element.
///
/// Icons are decorative by default and inherit their surrounding size and
/// color. Use [`Self::aria_label`] only when the glyph itself conveys meaning;
/// interactive icon controls should put their label on the control instead.
#[derive(IntoElement)]
pub struct IconElement {
    icon: LucideIcon,
    id: Option<ElementId>,
    size: Option<Pixels>,
    color: Option<Hsla>,
    aria_label: Option<SharedString>,
}

impl IconElement {
    /// Construct an inheriting, decorative icon element.
    pub fn new(icon: LucideIcon) -> Self {
        Self {
            icon,
            id: None,
            size: None,
            color: None,
            aria_label: None,
        }
    }

    /// Assign a stable element identity.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set an explicit square glyph size.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }

    /// Set an explicit glyph color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Give a meaningful, non-interactive icon an accessible name.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl RenderOnce for IconElement {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let icon = glyph(self.icon)
            .when_some(self.size, |element, size| {
                element.size(size).text_size(size)
            })
            .when_some(self.color, |element, color| element.text_color(color));
        match (self.id, self.aria_label) {
            (Some(id), Some(label)) => icon.id(id).aria_label(label).into_any_element(),
            (Some(id), None) => icon.id(id).into_any_element(),
            (None, _) => icon.into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_a_valid_truetype_font() {
        let bytes = lucide_icons::LUCIDE_FONT_BYTES;
        assert!(bytes.len() > 1_000);
        assert!(matches!(&bytes[..4], b"\0\x01\0\0" | b"OTTO"));
    }

    #[test]
    fn icon_element_is_an_ordinary_into_element_builder() {
        fn assert_into_element(_: impl gpui::IntoElement) {}
        assert_into_element(
            IconElement::new(LucideIcon::X)
                .id("close-icon")
                .size(gpui::px(16.))
                .aria_label("Close"),
        );
    }

    #[gpui::test]
    fn installation_is_app_global_and_idempotent(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            assert!(!cx.has_global::<Installed>());
            install(cx).expect("bundled Lucide font registers");
            assert!(cx.has_global::<Installed>());
            install(cx).expect("a second installation is a no-op");
        });
    }
}
