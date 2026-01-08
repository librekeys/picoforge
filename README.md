# PicoForge

<div align="center">

![PicoForge Logo](static/pico-forge.svg)

**An open source commissioning tool for Pico FIDO security keys**

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![GitHub issues](https://img.shields.io/github/issues/librekeys/picoforge)](https://github.com/librekeys/picoforge/issues)
[![GitHub stars](https://img.shields.io/github/stars/librekeys/picoforge)](https://github.com/librekeys/picoforge/stargazers)

</div>

## About

PicoForge is a modern desktop application for configuring and managing Pico FIDO security keys. Built with Rust, Tauri, and Svelte, it provides an intuitive interface for:

- Reading device information and firmware details
- Configuring USB VID/PID and product names
- Adjusting LED settings (GPIO, brightness, driver)
- Managing security features (secure boot, firmware locking)
- Real-time system logging and diagnostics
- Support for multiple hardware variants and vendors

## Features

- **Device Configuration** - Customize USB identifiers, LED behavior, and hardware settings
- **Security Management** - Enable secure boot and firmware verification (experimental)
- **Real-time Monitoring** - View flash usage, connection status, and system logs
- **Modern UI** - Clean, responsive interface built with Svelte and shadcn-svelte
- **Multi-Vendor Support** - Preset configurations for YubiKey, Nitrokey, SoloKeys, and more
- **Cross-Platform** - Works on Windows, macOS, and Linux

## Requirements

### Runtime Requirements

- Smart Card Reader (PC/SC compatible)
- Pico FIDO device in Rescue mode

### Development Requirements

To contribute to PicoForge, you'll need:

- **[Deno](https://deno.land/)** - JavaScript/TypeScript runtime
- **[Rust](https://www.rust-lang.org/)** - System programming language (1.70+)
- **PC/SC Middleware**:
  - Linux: `pcscd` (usually pre-installed)
  - macOS: Built-in
  - Windows: Built-in

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/librekeys/picoforge.git
cd picoforge
```

### 2. Install Dependencies

```bash
deno install
```

### 3. Build the Application

#### Development Build

```bash
deno task tauri dev
```

#### Production Build

```bash
deno task tauri build
```

The compiled binaries will be available in:
- **Linux**: `src-tauri/target/release/bundle/`
- **macOS**: `src-tauri/target/release/bundle/dmg/`
- **Windows**: `src-tauri/target/release/bundle/`

### Platform-Specific Notes

#### Linux

Install PC/SC dependencies:

```bash
sudo apt install libpcsclite-dev pcscd
```

Start the PC/SC daemon:

```bash
sudo systemctl start pcscd
sudo systemctl enable pcscd
```

#### macOS

No additional setup required. PC/SC framework is built-in.

#### Windows

Ensure Smart Card service is running:

```powershell
Get-Service SCardSvr | Start-Service
```

## Usage

1. Connect your smart card reader
2. Insert your Pico FIDO device (in Rescue mode)
3. Launch PicoForge
4. Click **Connect Device** to detect your key
5. Navigate through the sidebar to configure settings:
   - **Home** - Device overview and quick actions
   - **Configuration** - USB settings, LED options
   - **Security** - Secure boot management (experimental)
   - **Logs** - Real-time event monitoring
   - **About** - Application information

### Configuration Options

#### USB Identity
- **VID/PID** - Vendor and Product IDs (hex format)
- **Product Name** - Device name shown to host system
- **Vendor Presets** - Quick selection for common manufacturers

#### LED Settings
- **GPIO Pin** - Hardware pin for LED control
- **Brightness** - Intensity level (0-15)
- **Driver Type** - Hardware-specific LED drivers
- **Options** - Dimmable, steady mode, power cycle behavior

#### Advanced
- **Touch Timeout** - User presence button timeout (seconds)
- **Secp256k1** - Enable secp256k1 curve support
- **Secure Boot** - Firmware signature verification (⚠️ experimental)

## Project Structure

```
picoforge/
├── src/                      # Svelte frontend
│   ├── lib/                  # Reusable components & utilities
│   ├── routes/               # SvelteKit pages
│   ├── app.css               # Global styles
│   └── app.html              # HTML template
├── src-tauri/                # Rust backend
│   ├── src/                  # Rust source code
│   │   └── lib.rs            # Tauri commands & PC/SC logic
│   ├── icons/                # Application icons
│   ├── capabilities/         # Tauri permissions
│   ├── Cargo.toml            # Rust dependencies
│   ├── tauri.conf.json       # Tauri configuration
│   └── build.rs              # Build script
├── static/                   # Static assets
│   ├── build-configure-symbolic.svg
│   └── favicon.png
├── node_modules/             # Deno node compatibility modules
├── components.json           # shadcn-svelte config
├── package.json              # Node package manifest
├── deno.lock                 # Deno lock file
├── svelte.config.js          # SvelteKit configuration
├── vite.config.js            # Vite bundler config
├── tsconfig.json             # TypeScript configuration
└── LICENSE                   # AGPL-3.0 license
```

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust and TypeScript best practices
- Write clear commit messages
- Update documentation for new features
- Test on multiple platforms when possible

## License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0-only)**.

See [LICENSE](LICENSE) for full details.

## Maintainers

- **Suyog Tandel** ([@lockedmutex](https://github.com/lockedmutex))

## Acknowledgments

- [Pico FIDO](https://github.com/polhenarejos/pico-fido) - The firmware this tool configures
- [Tauri](https://tauri.app/) - Desktop application framework
- [Svelte](https://svelte.dev/) - Reactive UI framework
- [shadcn-svelte](https://www.shadcn-svelte.com/) - UI component library
- [pcsc-rust](https://github.com/bluetech/pcsc-rust) - Smart card interface

## Support

- **Issues**: [GitHub Issues](https://github.com/librekeys/picoforge/issues)
- **Discussions**: [GitHub Discussions](https://github.com/librekeys/picoforge/discussions)

## Disclaimer

PicoForge is experimental software. The secure boot feature can permanently lock devices if misconfigured. Always understand the implications before enabling security features.

---

<div align="center">

**Made by the LibreKeys community**

Copyright © 2026 Suyog Tandel

</div>