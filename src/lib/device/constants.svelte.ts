import type { DeviceConfig } from "$lib/device/types.svelte.ts";

export const VENDORS = [
  { value: "custom", label: "Custom (Manual Entry)", vid: "", pid: "" },
  { value: "generic", label: "Generic (FEFF:FCFD)", vid: "FEFF", pid: "FCFD" },
  { value: "pico", label: "Pico (2E8A:0003)", vid: "2E8A", pid: "0003" },
  { value: "solokeys", label: "SoloKeys (0483:A2CA)", vid: "0483", pid: "A2CA" },
  { value: "nitrohsm", label: "NitroHSM (20A0:4230)", vid: "20A0", pid: "4230" },
  { value: "nitrofido2", label: "NitroFIDO2 (20A0:42D4)", vid: "20A0", pid: "42D4" },
  { value: "nitrostart", label: "NitroStart (20A0:4211)", vid: "20A0", pid: "4211" },
  { value: "nitropro", label: "NitroPro (20A0:4108)", vid: "20A0", pid: "4108" },
  { value: "nitro3", label: "Nitrokey 3 (20A0:42B2)", vid: "20A0", pid: "42B2" },
  { value: "yubikey5", label: "YubiKey 5 (1050:0407)", vid: "1050", pid: "0407" },
  { value: "yubikeyneo", label: "YubiKey Neo (1050:0116)", vid: "1050", pid: "0116" },
  { value: "yubihsm", label: "YubiHSM 2 (1050:0030)", vid: "1050", pid: "0030" },
  { value: "gnuk", label: "Gnuk Token (234B:0000)", vid: "234B", pid: "0000" },
  { value: "gnupg", label: "GnuPG (234B:0000)", vid: "234B", pid: "0000" },
];

export const LED_DRIVERS = [
  { value: "1", label: "Pico (Standard GPIO)" },
  { value: "2", label: "Pimoroni (RGB)" },
  { value: "3", label: "WS2812 (Neopixel)" },
  { value: "5", label: "ESP32 Neopixel" },
];

export const DEFAULT_CONFIG: DeviceConfig = {
  vid: "CAFE",
  pid: "4242",
  productName: "Pico FIDO Key",
  ledGpio: 2,
  ledBrightness: 15,
  touchTimeout: 30,
  ledDimmable: false,
  powerCycleOnReset: false,
  ledSteady: false,
  enableSecp256k1: false,
  ledDriver: "1",
};

export const DEFAULT_DEVICE_INFO = {
  serial: "---",
  flashUsed: 0,
  flashTotal: 0,
  firmwareVersion: "---",
};
