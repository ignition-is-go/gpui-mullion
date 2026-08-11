use crate::MullionTheme;
use gpui::{px, Hsla, Pixels};

/// Style tokens for the root Mullion surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MullionRootStyle {
    /// Fill color behind the entire Mullion layout.
    pub background: Hsla,
}

/// Style tokens for pane surfaces, borders, and focus presentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneStyle {
    /// Fill color of each pane's content surface.
    pub background: Hsla,
    /// Default foreground color for pane content.
    pub text: Hsla,
    /// Color used for pane and host boundary lines.
    pub border: Hsla,
    /// Thickness of ordinary pane borders, in logical pixels.
    ///
    /// A value of zero disables the ordinary border.
    pub border_width: Pixels,
    /// Thickness of the host-provided bottom-edge border, in logical pixels.
    pub host_border_width: Pixels,
    /// Thickness of the focused-pane indicator, in logical pixels.
    pub focus_indicator_width: Pixels,
    /// Color of the focused-pane indicator.
    pub focus_indicator: Hsla,
    /// Color composited over panes when unfocused-pane washing is enabled.
    pub unfocused_wash: Hsla,
}

/// Geometry and state colors for activity bars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActivityBarStyle {
    /// Width of a vertical bar or height of a horizontal bar, in logical pixels.
    pub thickness: Pixels,
    /// Width or height of the bar while expanded, in logical pixels.
    pub expanded_extent: Pixels,
    /// Width and height of activity icons, in logical pixels.
    pub icon_size: Pixels,
    /// Inset around expanded activity-bar content, in logical pixels.
    pub expanded_padding: Pixels,
    /// Category-label font size, in logical pixels.
    pub font_size: Pixels,
    /// Thickness of the bar's boundary line, in logical pixels.
    pub border_width: Pixels,
    /// Corner radius of the activity-bar surface, in logical pixels.
    pub border_radius: Pixels,
    /// Thickness of the selected category edge, in logical pixels.
    pub category_border_width: Pixels,
    /// Fill color of the activity bar.
    pub background: Hsla,
    /// Color of the activity-bar boundary line.
    pub border: Hsla,
    /// Base color of activity icons.
    pub icon: Hsla,
    /// Foreground color of category labels.
    pub category_label: Hsla,
    /// Fill color of category cards in the expanded bar.
    pub category_card_background: Hsla,
    /// Color used to distinguish category edges.
    pub category_edge: Hsla,
    /// Alpha multiplier for inactive icons, normally in the inclusive range `0.0..=1.0`.
    pub inactive_icon_opacity: f32,
    /// Alpha multiplier for active icons, normally in the inclusive range `0.0..=1.0`.
    pub active_icon_opacity: f32,
}

/// Geometry and colors for the built-in pane management affordances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneControlStyle {
    /// Width and height of a compact control, in logical pixels.
    pub compact_size: Pixels,
    /// Width and height of an icon in a compact control, in logical pixels.
    pub compact_icon_size: Pixels,
    /// Font size of labels in expanded controls, in logical pixels.
    pub expanded_label_size: Pixels,
    /// Width and height of a control in its hidden presentation, in logical pixels.
    pub hidden_size: Pixels,
    /// Width and height of its hidden-presentation icon, in logical pixels.
    pub hidden_icon_size: Pixels,
    /// Distance from pane edges to the control capsule, in logical pixels.
    pub capsule_inset: Pixels,
    /// Inner padding of the control capsule, in logical pixels.
    pub capsule_padding: Pixels,
    /// Space between controls within the capsule, in logical pixels.
    pub capsule_gap: Pixels,
    /// Corner radius of the control capsule, in logical pixels.
    pub capsule_radius: Pixels,
    /// Thickness of the capsule outline, in logical pixels.
    pub capsule_border_width: Pixels,
    /// Alpha multiplier applied to the capsule, normally in `0.0..=1.0`.
    pub capsule_opacity: f32,
    /// Fill color of the control capsule.
    pub capsule_background: Hsla,
    /// Outline color of the control capsule.
    pub capsule_border: Hsla,
}

/// Geometry and colors for split grabbers.
///
/// The painted line and its larger pointer target are configured separately
/// so a thin divider can remain easy to grab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitHandleStyle {
    /// Thickness of the painted divider line, in logical pixels.
    pub thickness: Pixels,
    /// Thickness of the pointer hit target, in logical pixels.
    pub hover_target_thickness: Pixels,
    /// Divider color in its resting state.
    pub color: Hsla,
    /// Divider color while its pointer target is hovered.
    pub hover_color: Hsla,
}

/// Style tokens for drag-and-drop destination previews.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropOverlayStyle {
    /// Color composited over the prospective drop region.
    pub indicator_color: Hsla,
}

/// Geometry, typography, and colors for pane headers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneHeaderStyle {
    /// Total header height, in logical pixels.
    pub height: Pixels,
    /// Left and right content inset, in logical pixels.
    pub horizontal_padding: Pixels,
    /// Space between header children, in logical pixels.
    pub gap: Pixels,
    /// Header text size, in logical pixels.
    pub font_size: Pixels,
    /// Thickness of the header boundary line, in logical pixels.
    pub border_width: Pixels,
    /// Numeric font weight used for the pane title.
    pub title_weight: u16,
    /// Fill color of the header.
    pub background: Hsla,
    /// Default foreground color for header content.
    pub text: Hsla,
    /// Foreground color of the pane title.
    pub title: Hsla,
    /// Color of the header boundary line.
    pub border: Hsla,
}

/// Geometry, typography, and colors for workspace-switcher buttons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceSwitcherStyle {
    /// Space between workspace buttons, in logical pixels.
    pub gap: Pixels,
    /// Workspace-label font size, in logical pixels.
    pub font_size: Pixels,
    /// Workspace-label line-box height, in logical pixels.
    pub line_height: Pixels,
    /// Top and bottom button inset, in logical pixels.
    pub vertical_padding: Pixels,
    /// Left and right button inset, in logical pixels.
    pub horizontal_padding: Pixels,
    /// Button corner radius, in logical pixels.
    pub border_radius: Pixels,
    /// Fill color of an inactive workspace button.
    pub background: Hsla,
    /// Foreground color of an inactive workspace button.
    pub text: Hsla,
    /// Fill color of the active workspace button.
    pub active_background: Hsla,
    /// Foreground color of the active workspace button.
    pub active_text: Hsla,
}

/// Complete typed GPUI style set.
///
/// Constructing the set from a [`MullionTheme`] keeps color ownership in Rust
/// while leaving each component's geometry independently tunable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MullionAppearance {
    /// Semantic colors exposed to host-rendered Mullion chrome.
    ///
    /// Component-specific colors below are fully resolved and may intentionally
    /// diverge from this source palette.
    pub theme: MullionTheme,
    /// Tokens for the root surface.
    pub root: MullionRootStyle,
    /// Tokens for pane surfaces and focus state.
    pub pane: PaneStyle,
    /// Tokens for activity bars.
    pub activity_bar: ActivityBarStyle,
    /// Tokens for pane management controls.
    pub pane_controls: PaneControlStyle,
    /// Tokens for split dividers and their hit targets.
    pub split_handle: SplitHandleStyle,
    /// Tokens for drag-and-drop previews.
    pub drop_overlay: DropOverlayStyle,
    /// Tokens for pane headers.
    pub header: PaneHeaderStyle,
    /// Tokens for the workspace switcher.
    pub workspace_switcher: WorkspaceSwitcherStyle,
}

impl MullionAppearance {
    /// Build the reference component geometry using colors from `theme`.
    ///
    /// Geometry values are expressed in GPUI logical pixels. The supplied
    /// theme provides component colors without otherwise changing sizing.
    pub fn from_theme(theme: MullionTheme) -> Self {
        Self {
            theme,
            root: MullionRootStyle {
                background: theme.background,
            },
            pane: PaneStyle {
                background: theme.surface,
                text: theme.text,
                border: theme.border,
                border_width: px(0.),
                host_border_width: px(2.),
                focus_indicator_width: px(1.),
                focus_indicator: theme.focus_indicator,
                unfocused_wash: theme.background,
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
                category_card_background: Hsla {
                    h: 0.0,
                    s: 0.0,
                    l: 1.0,
                    a: 0.045,
                },
                category_edge: Hsla {
                    h: 0.0,
                    s: 0.0,
                    l: 1.0,
                    a: 0.08,
                },
                inactive_icon_opacity: 0.5,
                active_icon_opacity: 1.0,
            },
            pane_controls: PaneControlStyle {
                compact_size: px(28.),
                compact_icon_size: px(14.),
                expanded_label_size: px(11.),
                hidden_size: px(22.),
                hidden_icon_size: px(13.),
                capsule_inset: px(6.),
                capsule_padding: px(2.),
                capsule_gap: px(2.),
                capsule_radius: px(6.),
                capsule_border_width: px(1.),
                capsule_opacity: 0.95,
                capsule_background: theme.surface,
                capsule_border: theme.border,
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
                line_height: px(14.),
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

impl Default for MullionAppearance {
    fn default() -> Self {
        Self::from_theme(MullionTheme::default())
    }
}

macro_rules! component_default {
    ($component:ident, $field:ident) => {
        impl Default for $component {
            fn default() -> Self {
                MullionAppearance::default().$field
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
        let styles = MullionAppearance::default();
        assert_eq!(styles.activity_bar.thickness, px(28.));
        assert_eq!(styles.activity_bar.expanded_extent, px(150.));
        assert_eq!(styles.activity_bar.icon_size, px(14.));
        assert_eq!(styles.activity_bar.font_size, px(11.));
        assert_eq!(styles.split_handle.thickness, px(4.));
        assert_eq!(styles.split_handle.hover_target_thickness, px(8.));
        assert_eq!(styles.pane.border_width, px(0.));
        assert_eq!(styles.pane.host_border_width, px(2.));
        assert_eq!(styles.pane.focus_indicator_width, px(1.));
        assert_eq!(styles.header.height, px(28.));
        assert_eq!(styles.header.border_width, px(1.));
        assert_eq!(styles.header.horizontal_padding, px(8.));
        assert_eq!(styles.workspace_switcher.font_size, px(12.));
        assert_eq!(styles.workspace_switcher.line_height, px(14.));
        assert_eq!(styles.workspace_switcher.vertical_padding, px(4.));
        assert_eq!(styles.workspace_switcher.horizontal_padding, px(12.));
    }

    #[test]
    fn theme_colors_flow_into_every_component() {
        let theme = MullionTheme::default();
        let styles = MullionAppearance::from_theme(theme);
        assert_eq!(styles.root.background, theme.background);
        assert_eq!(styles.pane.background, theme.surface);
        assert_eq!(styles.activity_bar.category_label, theme.muted_text);
        assert_eq!(styles.drop_overlay.indicator_color, theme.drop_target);
        assert_eq!(styles.split_handle.color, theme.border);
        assert_eq!(styles.split_handle.hover_color, theme.focused);
        assert_eq!(styles.pane.focus_indicator, theme.focus_indicator);
        assert_eq!(styles.pane.unfocused_wash, theme.background);
    }
}
