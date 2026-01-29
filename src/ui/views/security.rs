// src/views/security.rs
use gpui::*;

pub struct SecurityView;

impl SecurityView {
    pub fn build() -> impl IntoElement {
        div()
            .size_full()
            .p_8()
            .child("Security Management List goes here...")
    }
}
