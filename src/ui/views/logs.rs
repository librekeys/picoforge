// src/views/logs.rs
use gpui::*;

pub struct LogsView;

impl LogsView {
	pub fn build() -> impl IntoElement {
		div().size_full().p_8().child("Logs goes here...")
	}
}
