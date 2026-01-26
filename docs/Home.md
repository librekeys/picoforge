# Welcome to the PicoForge Wiki

**PicoForge** is a modern desktop application for configuring and managing **Pico FIDO** security keys. Built with Rust, Tauri, and Svelte, it provides an intuitive interface.

> [!WARNING]
> **Beta Status**: This application is currently under active development and in beta stage. Users should expect bugs and are encouraged to report them. The app has been tested on Linux and Windows 10/11 with the official Raspberry Pi Pico2, WaveShare RP2350 One & ESP32-S3 and, currently supports Pico FIDO firmware version 7.2 only.
>
> It does not support all the features exposed by the `pico-fido` firmware and `pico-hsm`.

## Features

- **Device Configuration** - Customize USB identifiers, LED behavior, and hardware settings
- **Security Management** - Enable secure boot and firmware verification (experimental and WIP)
- **Real-time Monitoring** - View flash usage, connection status, and system logs
- **Modern UI** - Clean, responsive interface built with Svelte and shadcn-svelte
- **Multi-Vendor Support** - Compatible with multiple hardware variants
- **Cross-Platform** - Works on Windows, macOS, and Linux
- **Detailed Capabilities**:
    - Reading device information and firmware details
    - Configuring USB VID/PID and product names
    - Adjusting LED settings (GPIO, brightness, driver)
    - Managing security features (secure boot, firmware locking) (WIP)
    - Real-time system logging and diagnostics
    - Support for multiple hardware variants and vendors

## Usage

1. Connect your smart card reader
2. Insert your Pico FIDO device
3. Launch PicoForge
4. Click **Refresh** button at top right corner to detect your key
5. Navigate through the sidebar to configure settings:
   - **Home** - Device overview and quick actions
   - **PassKeys** - Passkey management
   - **Configuration** - USB settings, LED options
   - **Security** - Secure boot management (experimental)
   - **Logs** - Real-time event monitoring
   - **About** - Application information

## Disclaimer

> [!CAUTION]
> **USB VID/PID Notice**: The vendor presets provided in this software include USB Vendor IDs (VID) and Product IDs (PID) that are the intellectual property of their respective owners. These identifiers are included for testing and educational purposes only. You are NOT authorized to distribute or commercially market devices using VID/PID combinations you do not own or license. Commercial distribution requires obtaining your own VID from the USB Implementers Forum ([usb.org](https://www.usb.org/getting-vendor-id)) and complying with all applicable trademark and certification requirements. Unauthorized use may violate USB-IF policies and intellectual property laws. The PicoForge developers assume no liability for misuse of USB identifiers.

## Documentation Index

### Getting Started
*   **[Installation](Installation)** – Detailed setup instructions for all supported operating systems (Windows, macOS, Linux).
<!-- *   **[First Setup](First-Setup)** – How to initialize and personalize your new Pico-Key. -->

### User Guide
<!-- *   **[Device Management](Device-Management)** – configuring PINs, resetting the device, and firmware updates. -->
*   **[Troubleshooting](Troubleshooting)** – Solutions for common connection and detection issues.
<!-- *   **[FAQ](FAQ)** – Frequently asked questions about PicoForge and Pico-Key. -->

### Development
*   **[Building from Source](Building)** – Instructions for compiling PicoForge locally.
<!-- *   **[Contributing](Contributing)** – Guidelines for reporting bugs and submitting code changes. -->

---
*Return to the [LibreKeys PicoForge Repository](https://github.com/librekeys/picoforge)*
