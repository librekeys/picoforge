// src/views/passkeys.rs
use gpui::*;

pub struct PasskeysView;

impl PasskeysView {
	pub fn build() -> impl IntoElement {
		div()
			.size_full()
			.p_8()
			.child("Passkey Management List goes here...")
	}
}
