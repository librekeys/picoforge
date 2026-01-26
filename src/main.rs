use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, IconName, Root, TitleBar, h_flex, v_flex};
use gpui_component::{Side, sidebar::*};
use gpui_component::{Theme, ThemeMode};
use ui::views::{
	about::AboutView, config::ConfigView, home::HomeView, logs::LogsView, passkeys::PasskeysView,
	security::SecurityView,
};

mod device;
mod ui;

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
					.header(SidebarHeader::new().child("PicoForge"))
					.child(
						SidebarGroup::new("Menu").child(
							SidebarMenu::new()
								.child(
									SidebarMenuItem::new("Home")
										.icon(IconName::LayoutDashboard)
										.active(self.active_view == ActiveView::Home)
										.on_click(cx.listener(|this, _, _, _| {
											this.active_view = ActiveView::Home;
										})),
								)
								.child(
									SidebarMenuItem::new("Passkeys")
										.icon(IconName::Settings)
										.active(self.active_view == ActiveView::Passkeys)
										.on_click(cx.listener(|this, _, _, _| {
											this.active_view = ActiveView::Passkeys;
										})),
								)
								.child(
									SidebarMenuItem::new("Configuration")
										.icon(IconName::Settings)
										.active(self.active_view == ActiveView::Configuration)
										.on_click(cx.listener(|this, _, _, _| {
											this.active_view = ActiveView::Configuration;
										})),
								)
								// TODO: Replace these icons with correct ones from lucide
								.child(
									SidebarMenuItem::new("Security")
										.icon(IconName::Eye)
										.active(self.active_view == ActiveView::Security)
										.on_click(cx.listener(|this, _, _, _| {
											this.active_view = ActiveView::Security;
										})),
								)
								.child(
									SidebarMenuItem::new("Logs")
										.icon(IconName::File)
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
					)
					.footer(SidebarFooter::new().child("User Profile")),
			)
			.child(
				v_flex()
					.size_full()
					.child(
						TitleBar::new().child(
							h_flex()
								.w_full()
								.justify_between()
								// .px_4()
								.items_center()
								.child(
									// CORRECTED BUTTON SYNTAX
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
					.child(div().flex_grow().bg(cx.theme().background).child(
						// Switch on the Enum to decide what to render
						match self.active_view {
							ActiveView::Home => HomeView::build(cx.theme()).into_any_element(),
							ActiveView::Passkeys => PasskeysView::build().into_any_element(),
							ActiveView::Configuration => ConfigView::build().into_any_element(),
							ActiveView::Security => SecurityView::build().into_any_element(),
							ActiveView::Logs => LogsView::build().into_any_element(),
							ActiveView::About => AboutView::build().into_any_element(),
						},
					)),
			)
	}
}

fn main() {
	let app = Application::new().with_assets(gpui_component_assets::Assets);

	app.run(move |cx| {
		gpui_component::init(cx);
		Theme::change(ThemeMode::Dark, None, cx);
		// Theme::change(ThemeMode::Dark, Some(ui::theme::dark_theme()), cx);

		cx.activate(true);

		let mut window_size = size(px(1280.0), px(720.0));
		if let Some(display) = cx.primary_display() {
			let display_size = display.bounds().size;
			window_size.width = window_size.width.min(display_size.width * 0.85);
			window_size.height = window_size.height.min(display_size.height * 0.85);
		}
		let window_bounds = Bounds::centered(None, window_size, cx);

		cx.spawn(async move |cx| {
			let window_options = WindowOptions {
				window_bounds: Some(WindowBounds::Windowed(window_bounds)),
				titlebar: Some(TitleBar::title_bar_options()),
				window_min_size: Some(gpui::Size {
					width: px(800.),
					height: px(600.),
				}),
				kind: WindowKind::Normal,
				#[cfg(target_os = "linux")]
				window_background: gpui::WindowBackgroundAppearance::Transparent,
				#[cfg(target_os = "linux")]
				window_decorations: Some(gpui::WindowDecorations::Client),
				..Default::default()
			};

			cx.open_window(window_options, |window, cx| {
				let view = cx.new(|_| ApplicationRoot::new());
				cx.new(|cx| Root::new(view, window, cx))
			})?;

			Ok::<_, anyhow::Error>(())
		})
		.detach();
	});
}
