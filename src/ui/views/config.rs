// src/views/config.rs
use gpui::*;

pub struct ConfigView;

impl ConfigView {
    pub fn build() -> impl IntoElement {
        div()
            .size_full()
            .p_8()
            .child("Passkey Management List goes here...")
    }
}
