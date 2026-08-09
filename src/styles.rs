use crate::MullionTheme;
use gpui::{px, rgba, Hsla, Pixels};

/// Root surface tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MullionRootStyle {
    pub background: Hsla,
}

/// Pane surface tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneStyle {
    pub background: Hsla,
    pub text: Hsla,
    pub border: Hsla,
    pub border_width: Pixels,
    pub focus_indicator_width: Pixels,
}

/// Reference activity-bar geometry and state colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActivityBarStyle {
    pub thickness: Pixels,
    pub expanded_extent: Pixels,
    pub icon_size: Pixels,
    pub expanded_padding: Pixels,
    pub font_size: Pixels,
    pub border_width: Pixels,
    pub border_radius: Pixels,
    pub category_border_width: Pixels,
    pub background: Hsla,
    pub border: Hsla,
    pub icon: Hsla,
    pub category_label: Hsla,
    pub category_card_background: Hsla,
    pub category_edge: Hsla,
    pub inactive_icon_opacity: f32,
    pub active_icon_opacity: f32,
}

/// Split grabber geometry separates the painted line from its pointer target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitHandleStyle {
    pub thickness: Pixels,
    pub hover_target_thickness: Pixels,
    pub color: Hsla,
    pub hover_color: Hsla,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropOverlayStyle {
    pub indicator_color: Hsla,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneHeaderStyle {
    pub height: Pixels,
    pub horizontal_padding: Pixels,
    pub gap: Pixels,
    pub font_size: Pixels,
    pub border_width: Pixels,
    pub title_weight: u16,
    pub background: Hsla,
    pub text: Hsla,
    pub title: Hsla,
    pub border: Hsla,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceSwitcherStyle {
    pub gap: Pixels,
    pub font_size: Pixels,
    pub vertical_padding: Pixels,
    pub horizontal_padding: Pixels,
    pub border_radius: Pixels,
    pub background: Hsla,
    pub text: Hsla,
    pub active_background: Hsla,
    pub active_text: Hsla,
}

/// Complete typed GPUI style set. Constructing it from a theme keeps color
/// ownership in Rust while component geometry remains independently tunable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MullionStyles {
    pub root: MullionRootStyle,
    pub pane: PaneStyle,
    pub activity_bar: ActivityBarStyle,
    pub split_handle: SplitHandleStyle,
    pub drop_overlay: DropOverlayStyle,
    pub header: PaneHeaderStyle,
    pub workspace_switcher: WorkspaceSwitcherStyle,
}

impl MullionStyles {
    pub fn from_theme(theme: MullionTheme) -> Self {
        Self {
            root: MullionRootStyle {
                background: theme.background,
            },
            pane: PaneStyle {
                background: theme.surface,
                text: theme.text,
                border: theme.border,
                border_width: px(1.),
                focus_indicator_width: px(1.),
            },
            activity_bar: ActivityBarStyle {
                thickness: px(28.),
                expanded_extent: px(150.),
                icon_size: px(14.),
                expanded_padding: px(8.),
                font_size: px(11.),
                border_width: px(1.),
                border_radius: px(0.),
                category_border_width: px(2.),
                background: theme.surface,
                border: theme.border,
                icon: theme.text,
                category_label: theme.muted_text,
                category_card_background: rgba(0xffffff0b).into(),
                category_edge: rgba(0xffffff14).into(),
                inactive_icon_opacity: 0.5,
                active_icon_opacity: 1.0,
            },
            split_handle: SplitHandleStyle {
                thickness: px(4.),
                hover_target_thickness: px(8.),
                color: theme.border,
                hover_color: theme.focused,
            },
            drop_overlay: DropOverlayStyle {
                indicator_color: theme.drop_target,
            },
            header: PaneHeaderStyle {
                height: px(28.),
                horizontal_padding: px(8.),
                gap: px(8.),
                font_size: px(11.),
                border_width: px(1.),
                title_weight: 600,
                background: theme.surface,
                text: theme.text,
                title: theme.text,
                border: theme.border,
            },
            workspace_switcher: WorkspaceSwitcherStyle {
                gap: px(4.),
                font_size: px(12.),
                vertical_padding: px(4.),
                horizontal_padding: px(12.),
                border_radius: px(3.),
                background: theme.accent,
                text: theme.muted_text,
                active_background: theme.focused,
                active_text: theme.text,
            },
        }
    }
}

impl Default for MullionStyles {
    fn default() -> Self {
        Self::from_theme(MullionTheme::default())
    }
}

macro_rules! component_default {
    ($component:ident, $field:ident) => {
        impl Default for $component {
            fn default() -> Self {
                MullionStyles::default().$field
            }
        }
    };
}

component_default!(MullionRootStyle, root);
component_default!(PaneStyle, pane);
component_default!(ActivityBarStyle, activity_bar);
component_default!(SplitHandleStyle, split_handle);
component_default!(DropOverlayStyle, drop_overlay);
component_default!(PaneHeaderStyle, header);
component_default!(WorkspaceSwitcherStyle, workspace_switcher);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_reference_geometry() {
        let styles = MullionStyles::default();
        assert_eq!(styles.activity_bar.thickness, px(28.));
        assert_eq!(styles.activity_bar.expanded_extent, px(150.));
        assert_eq!(styles.activity_bar.icon_size, px(14.));
        assert_eq!(styles.activity_bar.font_size, px(11.));
        assert_eq!(styles.split_handle.thickness, px(4.));
        assert_eq!(styles.split_handle.hover_target_thickness, px(8.));
        assert_eq!(styles.header.height, px(28.));
        assert_eq!(styles.header.horizontal_padding, px(8.));
        assert_eq!(styles.workspace_switcher.font_size, px(12.));
    }

    #[test]
    fn theme_colors_flow_into_every_component() {
        let theme = MullionTheme::default();
        let styles = MullionStyles::from_theme(theme);
        assert_eq!(styles.root.background, theme.background);
        assert_eq!(styles.pane.background, theme.surface);
        assert_eq!(styles.activity_bar.category_label, theme.muted_text);
        assert_eq!(styles.drop_overlay.indicator_color, theme.drop_target);
    }
}
