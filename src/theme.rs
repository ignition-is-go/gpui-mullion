use gpui::{px, rgb, App, Hsla, Pixels, WindowAppearance};
use std::rc::Rc;

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

/// The complete Mullion look: semantic colors and resolved component geometry.
///
/// This is the only public look configuration. Component-specific colors may
/// intentionally diverge from the semantic fields when a host needs exact control.
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
    /// Dedicated focused-pane indicator color.
    pub focus_indicator: Hsla,
    /// Overlay used to preview drag-and-drop destinations.
    pub drop_target: Hsla,
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

#[derive(Clone, Copy)]
struct ThemeColors {
    background: Hsla,
    surface: Hsla,
    border: Hsla,
    accent: Hsla,
    text: Hsla,
    muted_text: Hsla,
    focused: Hsla,
    focus_indicator: Hsla,
    drop_target: Hsla,
}

impl MullionTheme {
    /// Return the complete built-in dark theme.
    pub fn dark() -> Self {
        Self::from_colors(ThemeColors {
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
        })
    }

    /// Return the complete built-in light theme.
    pub fn light() -> Self {
        Self::from_colors(ThemeColors {
            background: rgb(0xf4f4f4).into(),
            surface: rgb(0xffffff).into(),
            border: rgb(0xd0d0d0).into(),
            accent: rgb(0xe8e8e8).into(),
            text: rgb(0x202020).into(),
            muted_text: rgb(0x666666).into(),
            focused: rgb(0x0067c0).into(),
            focus_indicator: rgb(0x0067c0).into(),
            drop_target: rgb(0xb8d8f2).into(),
        })
    }

    /// Resolve the complete built-in theme for a GPUI window appearance.
    pub fn system(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
        }
    }

    /// Build a complete theme from nine semantic colors.
    ///
    /// Mullion maps these colors into every component token while preserving
    /// the reference geometry. Callers may then override individual public
    /// component fields when they need an intentional exception.
    #[allow(clippy::too_many_arguments)]
    pub fn custom(
        background: Hsla,
        surface: Hsla,
        border: Hsla,
        accent: Hsla,
        text: Hsla,
        muted_text: Hsla,
        focused: Hsla,
        focus_indicator: Hsla,
        drop_target: Hsla,
    ) -> Self {
        Self::from_colors(ThemeColors {
            background,
            surface,
            border,
            accent,
            text,
            muted_text,
            focused,
            focus_indicator,
            drop_target,
        })
    }

    fn from_colors(colors: ThemeColors) -> Self {
        Self {
            background: colors.background,
            surface: colors.surface,
            border: colors.border,
            accent: colors.accent,
            text: colors.text,
            muted_text: colors.muted_text,
            focused: colors.focused,
            focus_indicator: colors.focus_indicator,
            drop_target: colors.drop_target,
            root: MullionRootStyle {
                background: colors.background,
            },
            pane: PaneStyle {
                background: colors.surface,
                text: colors.text,
                border: colors.border,
                border_width: px(0.),
                host_border_width: px(2.),
                focus_indicator_width: px(1.),
                focus_indicator: colors.focus_indicator,
                unfocused_wash: colors.background,
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
                background: colors.surface,
                border: colors.border,
                icon: colors.text,
                category_label: colors.muted_text,
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
                capsule_background: colors.surface,
                capsule_border: colors.border,
            },
            split_handle: SplitHandleStyle {
                thickness: px(4.),
                hover_target_thickness: px(8.),
                color: colors.border,
                hover_color: colors.focused,
            },
            drop_overlay: DropOverlayStyle {
                indicator_color: colors.drop_target,
            },
            header: PaneHeaderStyle {
                height: px(28.),
                horizontal_padding: px(8.),
                gap: px(8.),
                font_size: px(11.),
                border_width: px(1.),
                title_weight: 600,
                background: colors.surface,
                text: colors.text,
                title: colors.text,
                border: colors.border,
            },
            workspace_switcher: WorkspaceSwitcherStyle {
                gap: px(4.),
                font_size: px(12.),
                line_height: px(14.),
                vertical_padding: px(4.),
                horizontal_padding: px(12.),
                border_radius: px(3.),
                background: colors.accent,
                text: colors.muted_text,
                active_background: colors.focused,
                active_text: colors.text,
            },
        }
    }
}

impl Default for MullionTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// UI-local theme source evaluated once per Mullion root render.
///
/// Hosts must invalidate their windows when provider state changes.
pub type MullionThemeProvider = Rc<dyn Fn(&App) -> MullionTheme>;

macro_rules! component_default {
    ($component:ident, $field:ident) => {
        impl Default for $component {
            fn default() -> Self {
                MullionTheme::default().$field
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
        let styles = MullionTheme::default();
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
        let styles = theme;
        assert_eq!(styles.root.background, theme.background);
        assert_eq!(styles.pane.background, theme.surface);
        assert_eq!(styles.activity_bar.category_label, theme.muted_text);
        assert_eq!(styles.drop_overlay.indicator_color, theme.drop_target);
        assert_eq!(styles.split_handle.color, theme.border);
        assert_eq!(styles.split_handle.hover_color, theme.focused);
        assert_eq!(styles.pane.focus_indicator, theme.focus_indicator);
        assert_eq!(styles.pane.unfocused_wash, theme.background);
    }
    #[test]
    fn custom_semantic_colors_flow_into_every_component_color() {
        let background = rgb(0x010101).into();
        let surface = rgb(0x020202).into();
        let border = rgb(0x030303).into();
        let accent = rgb(0x040404).into();
        let text = rgb(0x050505).into();
        let muted_text = rgb(0x060606).into();
        let focused = rgb(0x070707).into();
        let focus_indicator = rgb(0x080808).into();
        let drop_target = rgb(0x090909).into();

        let theme = MullionTheme::custom(
            background,
            surface,
            border,
            accent,
            text,
            muted_text,
            focused,
            focus_indicator,
            drop_target,
        );

        assert_eq!(theme.background, background);
        assert_eq!(theme.surface, surface);
        assert_eq!(theme.border, border);
        assert_eq!(theme.accent, accent);
        assert_eq!(theme.text, text);
        assert_eq!(theme.muted_text, muted_text);
        assert_eq!(theme.focused, focused);
        assert_eq!(theme.focus_indicator, focus_indicator);
        assert_eq!(theme.drop_target, drop_target);
        assert_eq!(theme.root.background, background);
        assert_eq!(theme.pane.background, surface);
        assert_eq!(theme.pane.text, text);
        assert_eq!(theme.pane.border, border);
        assert_eq!(theme.pane.focus_indicator, focus_indicator);
        assert_eq!(theme.pane.unfocused_wash, background);
        assert_eq!(theme.activity_bar.background, surface);
        assert_eq!(theme.activity_bar.border, border);
        assert_eq!(theme.activity_bar.icon, text);
        assert_eq!(theme.activity_bar.category_label, muted_text);
        assert_eq!(theme.pane_controls.capsule_background, surface);
        assert_eq!(theme.pane_controls.capsule_border, border);
        assert_eq!(theme.split_handle.color, border);
        assert_eq!(theme.split_handle.hover_color, focused);
        assert_eq!(theme.drop_overlay.indicator_color, drop_target);
        assert_eq!(theme.header.background, surface);
        assert_eq!(theme.header.text, text);
        assert_eq!(theme.header.title, text);
        assert_eq!(theme.header.border, border);
        assert_eq!(theme.workspace_switcher.background, accent);
        assert_eq!(theme.workspace_switcher.text, muted_text);
        assert_eq!(theme.workspace_switcher.active_background, focused);
        assert_eq!(theme.workspace_switcher.active_text, text);
    }
}
