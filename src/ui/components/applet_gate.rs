//! Shared empty-state gating for the CCID applet screens (Accounts, Slots,
//! PIV, OpenPGP).
//!
//! Three orthogonal questions decide whether a screen can show its content, in
//! priority order: is the CCID interface on, is the applet enabled on the
//! device, does this firmware expose it. Each screen computes an [`AppletGate`]
//! and, unless [`AppletGate::Ready`], renders the message below.

/// Why an applet screen cannot show its content — or `Ready` to proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppletGate {
    /// The applet is reachable; render the real UI.
    Ready,
    /// The CCID / smart-card USB interface is turned off.
    CcidOff,
    /// The applet is disabled in USB Applications (carries its display name).
    Disabled(&'static str),
    /// This firmware does not expose the applet.
    Unsupported,
}

impl AppletGate {
    /// Heading + body copy for the empty state, or `None` when [`Self::Ready`].
    ///
    /// Copy is firmware-neutral by design — it never names "pico-fido".
    pub fn message(&self) -> Option<(&'static str, String)> {
        match self {
            Self::Ready => None,
            Self::CcidOff => Some((
                crate::tr!("Smart-card interface off"),
                crate::tr!("Enable the CCID interface in Configuration → Hardware Endpoints, then reconnect the device.")
                    .into(),
            )),
            Self::Disabled(name) => Some((
                crate::tr!("Applet disabled"),
                crate::tr!("{name} is turned off. Enable it in Configuration → USB Applications.", name = name),
            )),
            Self::Unsupported => Some((
                crate::tr!("Not available"),
                crate::tr!("This firmware does not expose this applet.").into(),
            )),
        }
    }
}
