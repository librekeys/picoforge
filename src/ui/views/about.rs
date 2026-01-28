// src/views/about.rs
use gpui::*;

pub struct AboutView;

impl AboutView {
    pub fn build() -> impl IntoElement {
        div().size_full().p_8().child("About goes here...")
    }
}
