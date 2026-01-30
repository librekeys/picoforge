use crate::device::types::{FidoDeviceInfo, FullDeviceStatus};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveView {
    Home,
    Passkeys,
    Configuration,
    Security,
    Logs,
    About,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobalDeviceState {
    pub device_status: Option<FullDeviceStatus>,
    pub fido_info: Option<FidoDeviceInfo>,
    pub error: Option<String>,
}

impl GlobalDeviceState {
    pub fn new() -> Self {
        Self {
            device_status: None,
            fido_info: None,
            error: None,
        }
    }
}

pub struct VendorData {
    pub value: &'static str,
    pub label: &'static str,
    pub vid: &'static str,
    pub pid: &'static str,
}

pub const VENDORS: &[VendorData] = &[
    VendorData {
        value: "custom",
        label: "Custom (Manual Entry)",
        vid: "",
        pid: "",
    },
    VendorData {
        value: "generic",
        label: "Generic (FEFF:FCFD)",
        vid: "FEFF",
        pid: "FCFD",
    },
    VendorData {
        value: "pico-hsm",
        label: "Pico Keys HSM (2E8A:10FD)",
        vid: "2E8A",
        pid: "10FD",
    },
    VendorData {
        value: "pico-fido",
        label: "Pico Keys Fido (2E8A:10FE)",
        vid: "2E8A",
        pid: "10FE",
    },
    VendorData {
        value: "pico-openpgp",
        label: "Pico Keys OpenPGP (2E8A:10FF)",
        vid: "2E8A",
        pid: "10FF",
    },
    VendorData {
        value: "pico",
        label: "Pico (2E8A:0003)",
        vid: "2E8A",
        pid: "0003",
    },
    VendorData {
        value: "solokeys",
        label: "SoloKeys (0483:A2CA)",
        vid: "0483",
        pid: "A2CA",
    },
    VendorData {
        value: "nitrohsm",
        label: "NitroHSM (20A0:4230)",
        vid: "20A0",
        pid: "4230",
    },
    VendorData {
        value: "nitrofido2",
        label: "NitroFIDO2 (20A0:42D4)",
        vid: "20A0",
        pid: "42D4",
    },
    VendorData {
        value: "nitrostart",
        label: "NitroStart (20A0:4211)",
        vid: "20A0",
        pid: "4211",
    },
    VendorData {
        value: "nitropro",
        label: "NitroPro (20A0:4108)",
        vid: "20A0",
        pid: "4108",
    },
    VendorData {
        value: "nitro3",
        label: "Nitrokey 3 (20A0:42B2)",
        vid: "20A0",
        pid: "42B2",
    },
    VendorData {
        value: "yubikey5",
        label: "YubiKey 5 (1050:0407)",
        vid: "1050",
        pid: "0407",
    },
    VendorData {
        value: "yubikeyneo",
        label: "YubiKey Neo (1050:0116)",
        vid: "1050",
        pid: "0116",
    },
    VendorData {
        value: "yubihsm",
        label: "YubiHSM 2 (1050:0030)",
        vid: "1050",
        pid: "0030",
    },
    VendorData {
        value: "gnuk",
        label: "Gnuk Token (234B:0000)",
        vid: "234B",
        pid: "0000",
    },
    VendorData {
        value: "gnupg",
        label: "GnuPG (234B:0000)",
        vid: "234B",
        pid: "0000",
    },
];
