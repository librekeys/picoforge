use crate::ui::colors;
use crate::ui::views::{
    about::AboutView, config::ConfigView, home::HomeView, logs::LogsView, passkeys::PasskeysView,
    security::SecurityView,
};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{ActiveTheme, Icon, IconName, StyledExt, TitleBar, h_flex, v_flex};
use gpui_component::{Side, sidebar::*};

#[derive(Clone, Copy, PartialEq)]
enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    Logs,
    About,
}

pub struct ApplicationRoot {
    active_view: ActiveView,
    collapsed: bool,
}

impl ApplicationRoot {
    pub fn new() -> Self {
        Self {
            active_view: ActiveView::Home,
            collapsed: false,
        }
    }
}

impl Render for ApplicationRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .child(
                Sidebar::new(Side::Left)
                    .collapsed(self.collapsed)
                    .collapsible(true)
                    .h_full()
                    .bg(rgb(0x18181b))
                    .child(
                        SidebarGroup::new("Menu").child(
                            SidebarMenu::new()
                                .child(
                                    SidebarMenuItem::new("Home")
                                        .icon(Icon::default().path("icons/house.svg"))
                                        .active(self.active_view == ActiveView::Home)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::Home;
                                        })),
                                )
                                .child(
                                    SidebarMenuItem::new("Passkeys")
                                        .icon(Icon::default().path("icons/key-round.svg"))
                                        .active(self.active_view == ActiveView::Passkeys)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::Passkeys;
                                        })),
                                )
                                .child(
                                    SidebarMenuItem::new("Configuration")
                                        .icon(Icon::default().path("icons/settings.svg"))
                                        .active(self.active_view == ActiveView::Configuration)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::Configuration;
                                        })),
                                )
                                // TODO: Replace these icons with correct ones from lucide
                                .child(
                                    SidebarMenuItem::new("Security")
                                        .icon(Icon::default().path("icons/shield-check.svg"))
                                        .active(self.active_view == ActiveView::Security)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::Security;
                                        })),
                                )
                                .child(
                                    SidebarMenuItem::new("Logs")
                                        .icon(Icon::default().path("icons/scroll-text.svg"))
                                        .active(self.active_view == ActiveView::Logs)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::Logs;
                                        })),
                                )
                                .child(
                                    SidebarMenuItem::new("About")
                                        .icon(IconName::Info)
                                        .active(self.active_view == ActiveView::About)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.active_view = ActiveView::About;
                                        })),
                                ),
                        ),
                    ),
            )
            .child(
                v_flex()
                    .size_full()
                    .child(
                        TitleBar::new().bg(rgba(colors::zinc::ZINC900)).child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .bg(rgba(colors::zinc::ZINC900))
                                .items_center()
                                .cursor(gpui::CursorStyle::OpenHand)
                                .child(
                                    Button::new("sidebar_toggle")
                                        .ghost()
                                        .icon(IconName::PanelLeft)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.collapsed = !this.collapsed;
                                        }))
                                        .tooltip("Toggle Sidebar"),
                                ),
                        ),
                    )
                    .child(
                        v_flex()
                            .min_h(px(0.))
                            .min_w(px(0.))
                            .overflow_y_scrollbar()
                            .flex_grow()
                            .bg(cx.theme().background)
                            .child(match self.active_view {
                                ActiveView::Home => HomeView::build(cx.theme()).into_any_element(),
                                ActiveView::Passkeys => PasskeysView::build().into_any_element(),
                                ActiveView::Configuration => ConfigView::build().into_any_element(),
                                ActiveView::Security => SecurityView::build().into_any_element(),
                                ActiveView::Logs => LogsView::build().into_any_element(),
                                ActiveView::About => AboutView::build().into_any_element(),
                            }),
                    ),
            )
    }
}
