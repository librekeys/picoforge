//! Small form helpers shared by the applet screens' dialogs.

use gpui::*;
use gpui_component::select::{SelectItem, SelectState};

/// A labelled dropdown option carrying a small integer key (a wire value where
/// one exists — algorithm byte, period, digits — or a 0/1 flag otherwise).
#[derive(Clone, PartialEq)]
pub struct LabeledU8 {
    label: SharedString,
    key: u8,
}

impl SelectItem for LabeledU8 {
    type Value = u8;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.key
    }
}

/// Build a `Select` state from `(label, key)` options with a default row.
pub fn select_state(
    window: &mut Window,
    cx: &mut App,
    options: &[(&str, u8)],
    default_row: usize,
) -> Entity<SelectState<Vec<LabeledU8>>> {
    let opts: Vec<LabeledU8> = options
        .iter()
        .map(|(label, key)| LabeledU8 {
            label: crate::ui::i18n::translate(label).into(),
            key: *key,
        })
        .collect();
    cx.new(|cx| {
        SelectState::new(
            opts,
            Some(gpui_component::IndexPath::default().row(default_row)),
            window,
            cx,
        )
    })
}

/// Read a select's chosen key by mapping its row back through `options`.
pub fn selected_key(
    sel: &Entity<SelectState<Vec<LabeledU8>>>,
    options: &[(&str, u8)],
    cx: &App,
) -> u8 {
    let row = sel.read(cx).selected_index(cx).map(|p| p.row).unwrap_or(0);
    options.get(row).map(|(_, k)| *k).unwrap_or(options[0].1)
}
