use crate::{
    Activity, ActivityId, ActivityNode, DropEdge, MullionModel, MullionTheme, PaneData,
    PaneDirection, PaneEvent, PaneId, PaneNode, SplitDirection,
};
use gpui::{
    actions, div, prelude::*, px, relative, AnyElement, App, Context, EventEmitter, Hsla,
    MouseButton, SharedString, Window,
};

actions!(
    mullion,
    [
        FocusLeft,
        FocusRight,
        FocusUp,
        FocusDown,
        FocusNext,
        FocusPrevious,
        ClosePane,
        ToggleZoom,
        BalancePanes
    ]
);

#[derive(Clone)]
struct PaneDrag {
    id: PaneId,
}
impl Render for PaneDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(gpui::blue().opacity(0.75))
            .text_color(gpui::white())
            .child(self.id.0.clone())
    }
}

/// Native GPUI view over the portable model.
pub struct MullionView<D: PaneData> {
    model: MullionModel<D>,
    activities: Vec<ActivityNode<D>>,
    theme: MullionTheme,
    activity_bar_width: gpui::Pixels,
    show_headers: bool,
}

impl<D: PaneData> EventEmitter<PaneEvent<D>> for MullionView<D> {}

impl<D: PaneData> MullionView<D> {
    pub fn new(tree: PaneNode<D>, activities: Vec<ActivityNode<D>>) -> Self {
        Self {
            model: MullionModel::new(tree),
            activities,
            theme: MullionTheme::default(),
            activity_bar_width: px(42.),
            show_headers: true,
        }
    }
    pub fn with_theme(mut self, theme: MullionTheme) -> Self {
        self.theme = theme;
        self
    }
    pub fn with_headers(mut self, visible: bool) -> Self {
        self.show_headers = visible;
        self
    }
    pub fn model(&self) -> &MullionModel<D> {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut MullionModel<D> {
        &mut self.model
    }
    fn finish(&mut self, cx: &mut Context<Self>) {
        for event in self.model.take_events() {
            cx.emit(event)
        }
        cx.notify();
    }
    fn command(&mut self, command: crate::PaneCommand, cx: &mut Context<Self>) {
        let _ = self.model.execute(command, |_, _, _| None);
        self.finish(cx);
    }
    fn all_activities<'a>(&'a self, data: &D) -> Vec<&'a Activity<D>> {
        let mut out = Vec::new();
        for node in &self.activities {
            node.activities(data, &mut out)
        }
        out
    }
    fn render_node(&self, node: &PaneNode<D>, cx: &mut Context<Self>) -> AnyElement {
        match node {
            PaneNode::Leaf {
                id,
                active_activity,
                data,
            } => self.render_leaf(id, active_activity.as_ref(), data, cx),
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let key = second.leftmost_leaf_id().clone();
                let first_el = self.render_node(first, cx);
                let second_el = self.render_node(second, cx);
                let amount = *ratio;
                let handle_color = self.theme.border;
                let key_inc = key.clone();
                let key_dec = key;
                let handle = div()
                    .id(SharedString::from(format!("split-handle:{}", key_inc.0)))
                    .flex_shrink_0()
                    .when(*direction == SplitDirection::Horizontal, |e| {
                        e.w(px(5.)).h_full().cursor_col_resize()
                    })
                    .when(*direction == SplitDirection::Vertical, |e| {
                        e.h(px(5.)).w_full().cursor_row_resize()
                    })
                    .bg(handle_color)
                    .hover(|e| e.bg(self.theme.focused))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.model.resize(&key_inc, amount + 0.04);
                            this.finish(cx)
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, _, cx| {
                            this.model.resize(&key_dec, amount - 0.04);
                            this.finish(cx)
                        }),
                    );
                div()
                    .size_full()
                    .flex()
                    .overflow_hidden()
                    .when(*direction == SplitDirection::Vertical, |e| e.flex_col())
                    .child(
                        div()
                            .flex_none()
                            .when(*direction == SplitDirection::Horizontal, |e| {
                                e.w(relative(*ratio as f32)).h_full()
                            })
                            .when(*direction == SplitDirection::Vertical, |e| {
                                e.h(relative(*ratio as f32)).w_full()
                            })
                            .child(first_el),
                    )
                    .child(handle)
                    .child(div().flex_1().min_w_0().min_h_0().child(second_el))
                    .into_any_element()
            }
        }
    }
    fn render_leaf(
        &self,
        id: &PaneId,
        active: Option<&ActivityId>,
        data: &D,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let focused = self.model.focused() == Some(id);
        let theme = self.theme;
        let id_focus = id.clone();
        let id_drop = id.clone();
        let id_drag = id.clone();
        let id_close = id.clone();
        let activities = self.all_activities(data);
        let selected = active
            .and_then(|a| activities.iter().find(|x| &x.id == a).copied())
            .or_else(|| activities.first().copied());
        let tabs = activities.iter().map(|activity| {
            let pane = id.clone();
            let activity_id = activity.id.clone();
            let is_active = selected.is_some_and(|a| a.id == activity.id);
            div()
                .id(SharedString::from(format!(
                    "activity:{}:{}",
                    id.0, activity.id.0
                )))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_xs()
                .text_color(if is_active {
                    theme.text
                } else {
                    theme.muted_text
                })
                .bg(if is_active {
                    theme.accent
                } else {
                    Hsla::transparent_black()
                })
                .hover(|e| e.bg(theme.accent))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model.set_activity(&pane, Some(activity_id.clone()));
                    this.finish(cx)
                }))
                .child(activity.name.clone())
        });
        let body = selected.map(|a| (a.render)(id, data)).unwrap_or_else(|| {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_text)
                .child("No activity")
                .into_any_element()
        });
        let header = self.show_headers.then(|| {
            div()
                .h(px(30.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .border_b_1()
                .border_color(theme.border)
                .text_sm()
                .child(
                    selected
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| SharedString::from("Pane")),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("close:{}", id.0)))
                        .px_2()
                        .cursor_pointer()
                        .hover(|e| e.bg(theme.accent))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.model.close(&id_close);
                            this.finish(cx)
                        }))
                        .child("×"),
                )
        });
        div()
            .id(SharedString::from(format!("pane:{}", id.0)))
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .bg(theme.surface)
            .text_color(theme.text)
            .border_1()
            .border_color(if focused { theme.focused } else { theme.border })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.model.focus(&id_focus);
                    this.finish(cx)
                }),
            )
            .on_drag(PaneDrag { id: id_drag }, |drag, _, _, cx| {
                cx.new(|_| drag.clone())
            })
            .on_drop(cx.listener(move |this, drag: &PaneDrag, _, cx| {
                if drag.id != id_drop {
                    this.model.move_pane(&drag.id, &id_drop, DropEdge::Center);
                    this.finish(cx)
                }
            }))
            .child(
                div()
                    .w(self.activity_bar_width)
                    .h_full()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .py_1()
                    .border_r_1()
                    .border_color(theme.border)
                    .children(tabs),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .when_some(header, |e, h| e.child(h))
                    .child(div().flex_1().min_h_0().overflow_hidden().child(body)),
            )
            .into_any_element()
    }
}

impl<D: PaneData> Render for MullionView<D> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tree = self
            .model
            .zoomed()
            .and_then(|id| self.model.tree().find(id))
            .unwrap_or(self.model.tree())
            .clone();
        div()
            .key_context("Mullion")
            .track_focus(&cx.focus_handle())
            .size_full()
            .bg(self.theme.background)
            .on_action(cx.listener(|this, _: &FocusLeft, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Left), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusRight, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Right), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusUp, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Up), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusDown, _, cx| {
                this.command(crate::PaneCommand::Focus(PaneDirection::Down), cx)
            }))
            .on_action(cx.listener(|this, _: &FocusNext, _, cx| {
                this.command(crate::PaneCommand::FocusNext, cx)
            }))
            .on_action(cx.listener(|this, _: &FocusPrevious, _, cx| {
                this.command(crate::PaneCommand::FocusPrevious, cx)
            }))
            .on_action(
                cx.listener(|this, _: &ClosePane, _, cx| {
                    this.command(crate::PaneCommand::Close, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleZoom, _, cx| {
                this.command(crate::PaneCommand::ToggleZoom, cx)
            }))
            .on_action(cx.listener(|this, _: &BalancePanes, _, cx| {
                this.command(crate::PaneCommand::Balance, cx)
            }))
            .child(self.render_node(&tree, cx))
    }
}

/// Register the default, direct native key bindings. Hosts may override them later.
pub fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("alt-left", FocusLeft, None),
        gpui::KeyBinding::new("alt-right", FocusRight, None),
        gpui::KeyBinding::new("alt-up", FocusUp, None),
        gpui::KeyBinding::new("alt-down", FocusDown, None),
        gpui::KeyBinding::new("alt-pagedown", FocusNext, None),
        gpui::KeyBinding::new("alt-pageup", FocusPrevious, None),
        gpui::KeyBinding::new("ctrl-shift-backspace", ClosePane, None),
        gpui::KeyBinding::new("ctrl-shift-enter", ToggleZoom, None),
        gpui::KeyBinding::new("ctrl-alt-=", BalancePanes, None),
    ]);
}
