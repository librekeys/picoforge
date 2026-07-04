//! # PicoForge
//!
//! An open-source commissioning and management tool for **Pico FIDO** hardware security keys.
//! Built with Rust and the GPUI framework, PicoForge provides a native desktop GUI for
//! configuring, managing, and monitoring FIDO2/CTAP2 security keys running the
//! [pico-fido](https://github.com/polhenarejos/pico-fido) or
//! [RS-Key](https://github.com/TheMaxMur/RS-Key) firmware on RP2040, RP2350, and ESP32-S3
//! microcontrollers.
//!
//! ## Table of Contents
//!
//! - [Project Overview](#project-overview)
//! - [Quick Start for Contributors](#quick-start-for-contributors)
//! - [Project Structure](#project-structure)
//! - [Technology Stack](#technology-stack)
//! - [Architecture Overview](#architecture-overview)
//! - [Data Flow](#data-flow)
//! - [Device Communication Protocols](#device-communication-protocols)
//! - [UI Architecture](#ui-architecture)
//! - [Code Style and Conventions](#code-style-and-conventions)
//! - [Error Handling](#error-handling)
//! - [Firmware Compatibility](#firmware-compatibility)
//! - [Build and Development](#build-and-development)
//! - [Testing](#testing)
//! - [Deployment and Packaging](#deployment-and-packaging)
//! - [Contributing Guidelines](#contributing-guidelines)
//! - [License](#license)
//!
//! ---
//!
//! <div class="warning">
//!
//! **Documentation Notice**: This documentation is a work in progress and may contain
//! inaccuracies, incomplete information, or descriptions that don't perfectly reflect the
//! current state of the codebase or API behavior. Contributions to improve or correct this
//! documentation are very welcome — please see the [Contributing Guidelines](#contributing-guidelines)
//! section or check [CONTRIBUTING.md](.github/CONTRIBUTING.md) for details.
//!
//! </div>
//!
//! ## Project Overview
//!
//! PicoForge is a desktop application that allows users to:
//!
//! - **View device information**: Serial number, firmware version, flash usage, USB identifiers
//! - **Configure hardware settings**: USB VID/PID, LED GPIO pins, brightness, touch timeout
//! - **Manage FIDO2 credentials**: List, delete, and factory-reset passkeys
//! - **PIN management**: Set, change, and configure minimum PIN length
//! - **Security features**: Enable/disable secure boot, enterprise attestation
//! - **LED customization**: Configure LED colors and behavior for different device states (RS-Key)
//! - **USB interface management**: Enable/disable FIDO2, OpenPGP, PIV, OATH, OTP applets (RS-Key)
//! - **Device reboot**: Normal reboot or enter BOOTSEL/UF2 bootloader mode for firmware updates
//!
//! The application communicates with security keys through two protocols:
//!
//! 1. **Rescue Protocol (PC/SC)**: Low-level APDU-based communication for hardware configuration
//! 2. **FIDO2 Protocol (CTAP2)**: Standard authenticator protocol for credential and PIN management
//!
//! PicoForge automatically detects the connected device and selects the appropriate communication
//! method, falling back gracefully between protocols when needed.
//!
//! ---
//!
//! ## Quick Start for Contributors
//!
//! ### Prerequisites
//!
//! - **Rust toolchain** (edition 2024, MSRV 1.80+)
//! - **Linux**: `pcscd` daemon running (`sudo systemctl start pcscd`)
//! - **NixOS**: Use `nix develop` for a complete development environment
//!
//! ### Getting Started
//!
//! 1. **Clone and enter the project**:
//!    ```bash
//!    git clone <repo-url>
//!    cd pico-forge
//!    ```
//!
//! 2. **Enter the development shell** (NixOS):
//!    ```bash
//!    nix develop
//!    ```
//!
//! 3. **Build and run**:
//!    ```bash
//!    cargo run
//!    ```
//!
//! 4. **Connect a Pico FIDO device** and the application will auto-detect it.
//!
//! ### Where to Start
//!
//! - **New to the codebase?** Start with `src/main.rs` (this file) and `src/ui/rootview.rs`
//!   to understand the application entry point and main window structure.
//!
//! - **Want to understand device communication?** Read `src/device/mod.rs` and
//!   `src/device/io.rs` for the high-level API, then dive into `src/device/rescue/mod.rs`
//!   (PC/SC protocol) or `src/device/fido/mod.rs` (FIDO2/CTAP2 protocol).
//!
//! - **Working on UI features?** Explore `src/ui/views/` for page implementations and
//!   `src/ui/components/` for reusable widgets. See `src/ui/types.rs` for state types.
//!
//! - **Adding new device commands?** Check `src/device/fido/constants.rs` and
//!   `src/device/rescue/constants.rs` for protocol constants, then implement in the
//!   appropriate protocol module.
//!
//! ---
//!
//! ## Project Structure
//!
//! ```text
//! pico-forge/
//! ├── build.rs                            # Build script (Windows resource embedding)
//! ├── Cargo.lock                          # Dependency lockfile
//! ├── Cargo.toml                          # Package manifest and dependencies
//! ├── ci.nix                              # CI configuration for cachix
//! ├── CREDITS.md                          # Credits
//! ├── data/
//! │   ├── in.suyogtandel.picoforge.desktop    # Linux desktop entry
//! │   ├── in.suyogtandel.picoforge.metainfo.xml  # AppStream metadata
//! │   └── screenshots/                    # Screenshots for documentation
//! ├── default.nix                         # Nix package fallback
//! ├── docs/                               # Project documentation/wiki files
//! │   ├── Building.md
//! │   ├── Home.md
//! │   ├── Installation.md
//! │   └── Troubleshooting.md
//! ├── flake.lock                          # Nix flake lock file
//! ├── flake.nix                           # Nix flake configuration
//! ├── Info.plist                          # macOS bundle metadata
//! ├── LICENSE                             # AGPL-3.0
//! ├── maintainers/                        # Package maintainer scripts
//! │   └── scripts/
//! │       ├── update.nix
//! │       └── update.py
//! ├── package.nix                         # Nix package definition
//! ├── picoforge.spec                      # RPM spec file
//! ├── README.md
//! ├── rustfmt.toml                        # Rust formatting configuration
//! ├── shell.nix                           # Nix development shell
//! ├── src/                                # Source code
//! │   ├── main.rs                         # ← THIS FILE: Application entry point
//! │   ├── error.rs                        # Application-wide error types (PFError)
//! │   ├── logging.rs                      # log4rs configuration
//! │   ├── device/                         # Hardware communication layer
//! │   │   ├── mod.rs                      # Module root, re-exports
//! │   │   ├── io.rs                       # High-level API bridging rescue and FIDO
//! │   │   ├── types.rs                    # Shared data structures
//! │   │   ├── rescue/                     # Rescue applet (PC/SC protocol)
//! │   │   │   ├── mod.rs
//! │   │   │   └── constants.rs
//! │   │   └── fido/                       # FIDO2/CTAP2 protocol
//! │   │       ├── mod.rs
//! │   │       ├── constants.rs
//! │   │       └── hid.rs                  # USB HID transport
//! │   └── ui/                             # GPUI frontend
//! │       ├── mod.rs
//! │       ├── rootview.rs                 # Main window, sidebar routing
//! │       ├── types.rs                    # UI state types
//! │       ├── assets.rs                   # rust-embed asset loader
//! │       ├── colors.rs                   # Theme color constants
//! │       ├── views/                      # Page views (sidebar sections)
//! │       │   ├── mod.rs
//! │       │   ├── home.rs
//! │       │   ├── passkeys.rs
//! │       │   ├── config.rs
//! │       │   ├── security.rs
//! │       │   └── about.rs
//! │       └── components/                 # Reusable UI widgets
//! │           ├── mod.rs
//! │           ├── button.rs
//! │           ├── card.rs
//! │           ├── dialog.rs
//! │           ├── page_view.rs
//! │           ├── sidebar.rs
//! │           └── tag.rs
//! ├── static/
//! │   ├── appIcons/                       # Application icons (SVG, PNG, ICO, ICNS)
//! │   └── icons/                          # UI icons (SVG, loaded via rust-embed)
//! ├── themes/
//! │   └── picoforge-zinc.json             # Application theme (Zinc dark palette)
//! ├── .cargo/
//! │   └── config.toml                     # Cargo configuration
//! ├── .envrc                              # direnv integration
//! ├── .github/                            # CI/CD and contribution templates
//! │   ├── workflows/
//! │   │   ├── ci.yml
//! │   │   ├── docs.yml
//! │   │   ├── release.yml
//! │   │   ├── release-nightly.yml
//! │   │   ├── nix-binary-cache.yml
//! │   │   ├── nix-check-package.yml
//! │   │   ├── nix-update-package.yml
//! │   │   └── wiki-sync.yml
//! │   ├── scripts/
//! │   ├── ISSUE_TEMPLATE/
//! │   ├── manifests/
//! │   ├── CONTRIBUTING.md
//! │   ├── FUNDING.yml
//! │   └── PULL_REQUEST_TEMPLATE.md
//! ├── .gitignore
//! └── .tito/                              # RPM/Tito release tooling
//!     ├── packages/
//!     └── tito.props
//!```
//!
//! ---
//!
//! ## Technology Stack
//!
//! ### Core Dependencies
//!
//! | Crate | Version | Purpose |
//! |-------|---------|---------|
//! | `gpui` | 0.2.2 | Zed's GPU-accelerated UI framework |
//! | `gpui-component` | git (librekeys fork) | Reusable UI components (buttons, inputs, dialogs, etc.) |
//! | `pcsc` | 2.x | PC/SC smart card API for Rescue protocol |
//! | `hidapi` | 2.6 | USB HID API for FIDO2/CTAPHID transport |
//! | `serde` / `serde_json` | 1.x | Serialization/deserialization |
//! | `serde_cbor_2` | 0.13 | CBOR encoding for CTAP2 messages |
//! | `ring` | 0.17 | Cryptographic operations (ECDH, HMAC, SHA-256) |
//! | `aes` / `cbc` | 0.9 / 0.2 | AES-256-CBC encryption for PIN tokens |
//! | `rand` | 0.10 | Cryptographic random number generation |
//! | `byteorder` | 1.5 | Big-endian byte encoding (firmware protocol requirement) |
//! | `hex` | 0.4 | Hex encoding/decoding for VID/PID, AAGUIDs |
//! | `base64` | 0.22 | PEM encoding for certificates |
//! | `bitflags` | 2.13 | Type-safe bitflags for configuration options |
//! | `thiserror` | 2.x | Derive macro for error types |
//! | `anyhow` | 1.x | Error propagation with context |
//! | `log` / `log4rs` | 0.4 / 1.x | Logging facade and implementation |
//! | `directories` | 6.x | Cross-platform config/data directory paths |
//! | `rust-embed` | 8.11 | Embed static assets in binary |
//!
//! ### UI Framework: GPUI
//!
//! GPUI is a GPU-accelerated UI framework developed by Zed Industries. Key concepts:
//!
//! - **Entity**: Stateful component with `Context<Self>` for mutations
//! - **Render**: Trait for components that produce element trees
//! - **RenderOnce**: Stateless elements that consume self on render
//! - **Context (cx)**: Mutable access to entity state and window/app APIs
//! - **Window**: Window-level operations (dialogs, notifications, focus)
//! - **App**: Application-level state (global theme, asset loading)
//!
//! The project uses the **librekeys fork** of `gpui-component` (`fix/client-window-linux`
//! branch) which applies bug fixes for Linux/FreeBSD window management. The upstream
//! component API is the same, but check the fork's branch for any platform-specific fixes.
//!
//! ---
//!
//! ## Architecture Overview
//!
//! ### Layer Diagram
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    UI Layer (src/ui/)                       │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
//! │  │ HomeView │  │Passkeys  │  │ConfigView│  │Security  │     │
//! │  │          │  │View      │  │          │  │View      │     │
//! │  └────┬─────┘  └─────┬────┘  └──────┬───┘  └───────┬──┘     │
//! │       │              │              │              │        │
//! │       └──────────────┴──────┬───────┴──────────────┘        │
//! │                             │                               │
//! │                    ┌────────▼────────┐                      │
//! │                    │ ApplicationRoot │ (rootview.rs)        │
//! │                    │  DeviceState    │                      │
//! │                    │  LayoutState    │                      │
//! │                    │  ViewCache      │                      │
//! │                    └────────┬────────┘                      │
//! └─────────────────────────────┼───────────────────────────────┘
//!                               │
//! ┌─────────────────────────────▼───────────────────────────────┐
//! │                  Device I/O Layer (src/device/io.rs)        │
//! │         High-level API: read_device_details()               │
//! │                         write_config()                      │
//! │                         get_credentials()                   │
//! │                         reboot()                            │
//! └─────────────────────────────┬───────────────────────────────┘
//!                               │
//!              ┌────────────────┴────────────────┐
//!              │                                 │
//! ┌────────────▼────────────┐  ┌─────────────────▼─────────────┐
//! │  Rescue Protocol        │  │  FIDO2 Protocol               │
//! │  (src/device/rescue/)   │  │  (src/device/fido/)           │
//! │                         │  │                               │
//! │  PC/SC + ISO 7816-4     │  │  CTAPHID + CTAP2              │
//! │  APDU commands          │  │  CBOR messages                │
//! │  TLV configuration      │  │  ECDH key agreement           │
//! └────────────┬────────────┘  └─────────────────┬─────────────┘
//!              │                                 │
//!              └────────────────┬────────────────┘
//!                               │
//! ┌─────────────────────────────▼───────────────────────────────┐
//! │              Hardware (USB Composite Device)                │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐                   │
//! │  │ CCID     │  │ HID      │  │ Keyboard │                   │
//! │  │ (Rescue) │  │ (FIDO2)  │  │ (OTP)    │                   │
//! │  └──────────┘  └──────────┘  └──────────┘                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Key Design Principles
//!
//! 1. **Separation of Concerns**: Device communication (`src/device/`) is completely
//!    separate from UI code (`src/ui/`). No GPUI imports exist in device modules.
//!
//! 2. **Protocol Abstraction**: The `io.rs` layer provides a unified API that
//!    automatically selects Rescue or FIDO2 based on device capabilities and
//!    falls back gracefully between protocols.
//!
//! 3. **Firmware Agnosticism**: The codebase supports both pico-fido (C firmware)
//!    and RS-Key (Rust firmware) with the same UI, detecting firmware type via
//!    AAGUID and adapting behavior accordingly.
//!
//! 4. **State-Driven UI**: The `ApplicationRoot` maintains device state and
//!    propagates changes to views via the `DeviceConnectionState` struct.
//!
//! ---
//!
//! ## Data Flow
//!
//! ### Device Detection and Status Refresh
//!
//! ```text
//! ApplicationRoot::new()
//!       │
//!       ▼
//! refresh_device_status()
//!       │
//!       ▼
//! io::read_device_details()
//!       │
//!       ├──► rescue::read_device_details()
//!       │         │
//!       │         ├── connect_and_select()          [PC/SC]
//!       │         ├── READ(FlashInfo)
//!       │         ├── READ(SecureBootStatus)
//!       │         └── READ(PhyConfig)               [TLV parsing]
//!       │
//!       └──► fido::read_device_details()            [fallback]
//!                 │
//!                 ├── HidTransport::open()
//!                 ├── GetInfo (CTAP2 0x04)
//!                 └── Vendor commands (0xC1/0xC2)
//!
//!       ▼
//! Update DeviceConnectionState
//!       │
//!       ├── status: FullDeviceStatus
//!       ├── fido_info: FidoDeviceInfo
//!       ├── led_status: LedStatusConfig             [RS-Key only]
//!       └── management_apps: ManagementAppConfig    [RS-Key only]
//!       │
//!       ▼
//! cx.notify() → UI re-renders with new state
//! ```
//!
//! ### Configuration Write Flow
//!
//! ```text
//! User edits config form (ConfigView)
//!       │
//!       ▼
//! io::write_config(config, method, pin)
//!       │
//!       ├──► rescue::write_config(config)
//!       │         │
//!       │         ├── Build TLV payload
//!       │         ├── connect_and_select()
//!       │         └── WRITE(PhyConfig, tlv_data)
//!       │
//!       └──► fido::write_config(config, pin)    [if FIDO mode]
//!                 │
//!                 ├── HidTransport::open()
//!                 ├── get_pin_token_with_permission()
//!                 └── send_vendor_config()
//!
//!       ▼
//! Return success/error message
//!       │
//!       ▼
//! Dialog shows status to user
//! ```
//!
//! ### Credential Management Flow
//!
//! ```text
//! PasskeysView::unlock_storage(pin)
//!       │
//!       ▼
//! io::get_credentials(pin)
//!       │
//!       ▼
//! fido::get_credentials(pin)
//!       │
//!       ├── HidTransport::open()
//!       ├── credential_management_enumerate_rps(pin)
//!       │       → Vec<EnumerateRpResponse>
//!       │
//!       └── For each RP:
//!               credential_management_enumerate_credentials(pin, rp_id_hash)
//!                       → Vec<EnumerateCredentialResponse>
//!
//!       ▼
//! Parse CBOR responses → Vec<StoredCredential>
//!       │
//!       ▼
//! Display credentials in PasskeysView table
//! ```
//!
//! ---
//!
//! ## Device Communication Protocols
//!
//! ### Rescue Protocol (PC/SC)
//!
//! The Rescue applet provides low-level hardware access via the CCID (smart card)
//! USB interface. Communication uses ISO 7816-4 APDUs:
//!
//! ```text
//! APDU Structure:
//! ┌─────┬─────┬─────┬─────┬─────┬─────────────┐
//! │ CLA │ INS │  P1 │  P2 │ Lc  │    Data     │
//! └─────┴─────┴─────┴─────┴─────┴─────────────┘
//!   0x80  cmd   param param len   payload
//!
//! Response: [Data...] [SW1 SW2]  (SW 9000 = success)
//! ```
//!
//! **Key Operations**:
//! - `SELECT A0 58 3F C1 9B 7E 4F 21` — Select Rescue applet
//! - `READ (0x1E)` — Read flash info, secure boot status, PHY config
//! - `WRITE (0x1C)` — Write PHY configuration (TLV format)
//! - `REBOOT (0x1F)` — Reboot device (normal or BOOTSEL mode)
//!
//! **PHY TLV Tags** (hardware configuration):
//! - `0x00`: VID:PID (4 bytes, big-endian)
//! - `0x04`: LED GPIO pin
//! - `0x05`: LED brightness
//! - `0x06`: Options bitmask (LED_DIMMABLE, DISABLE_POWER_RESET, LED_STEADY)
//! - `0x07`: Elliptic curves bitmask (SECP256K1, etc.)
//! - `0x08`: Touch/presence timeout
//! - `0x09`: USB product name (null-terminated)
//! - `0x0B`: Enabled USB interfaces
//! - `0x0C`: LED driver selection
//! - `0x0D`: LED order (RS-Key extension)
//!
//! ### FIDO2 Protocol (CTAP2)
//!
//! FIDO2 communication uses USB HID (CTAPHID) with 64-byte reports:
//!
//! ```text
//! CTAPHID Framing:
//! Init Packet (64 bytes):
//!   CID(4) | CMD(1) | BCNT_HI(1) | BCNT_LO(1) | payload[..57]
//!
//! Continuation Packets:
//!   CID(4) | SEQ(1) | payload[..59]
//! ```
//!
//! **Key Operations**:
//! - `GetInfo (0x04)` — Device metadata (versions, AAGUID, options)
//! - `ClientPin (0x06)` — PIN/UV token management
//! - `CredentialMgmt (0x0A)` — Credential enumeration/deletion
//! - `Config (0x0D)` — Authenticator configuration
//! - Vendor commands (`0xC1`, `0xC2`) — Hardware config (pico-fido)
//!
//! **PIN Token Flow** (ECDH + AES-CBC):
//! 1. Host requests device's P-256 public key (GetKeyAgreement)
//! 2. Host generates ephemeral P-256 key pair
//! 3. Host computes ECDH shared secret → SHA-256(shared_secret)
//! 4. PIN hash encrypted with AES-256-CBC (key = shared_secret, IV = 0)
//! 5. Token decrypted with same key for permission-gated operations
//!
//! ---
//!
//! ## UI Architecture
//!
//! ### GPUI Component Pattern
//!
//! PicoForge uses two patterns from gpui-component:
//!
//! 1. **Stateless RenderOnce elements** (e.g., `Card`, `Tag`, `PageView`):
//!    ```rust
//!    Card::new()
//!        .title("Device Info")
//!        .icon(Icon::default().path("icons/cpu.svg"))
//!        .child(content)
//!    ```
//!
//! 2. **Stateful Entity components** (e.g., `InputState`, `SelectState`):
//!    ```rust
//!    let input = cx.new(|cx| InputState::new(window, cx)
//!        .placeholder("Enter VID...")
//!        .default_value("FEFF"));
//!    Input::new(&input)
//!    ```
//!
//! ### View Lifecycle
//!
//! ```text
//! ApplicationRoot::new(cx)
//!       │
//!       ├── DeviceConnectionState::new()
//!       ├── LayoutState::new()
//!       ├── ViewCache::new()
//!       │
//!       └── refresh_device_status() → io::read_device_details()
//!
//! Render cycle:
//!       │
//!       ├── Render sidebar (AppSidebar)
//!       │     └── on_select → updates LayoutState.active_view
//!       │
//!       └── Render content based on active_view:
//!             ├── Home → HomeView::build() [stateless]
//!             ├── Passkeys → PasskeysView::new() [stateful, cached]
//!             ├── Configuration → ConfigView::new() [stateful, cached]
//!             ├── Security → SecurityView::build() [stateless]
//!             └── About → AboutView::build() [stateless]
//! ```
//!
//! ### State Management
//!
//! - **DeviceConnectionState**: Holds device status, FIDO info, LED config, errors
//! - **LayoutState**: Active view, sidebar width/collapse state
//! - **ViewCache**: Cached entity views (PasskeysView, ConfigView) to preserve state
//!
//! State flows downward from `ApplicationRoot` to views via props. Views communicate
//! upward via callbacks (`on_select`, `on_refresh`) and events (`PasskeysEvent`).
//!
//! ---
//!
//! ## Code Style and Conventions
//!
//! ### Rust Edition and MSRV
//!
//! - **Edition**: 2024 (latest Rust edition)
//! - **MSRV**: 1.80+ (required for edition 2024 features)
//!
//! ### Naming and Documentation
//!
//! Follow the official [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
//! for naming conventions, documentation comments, and module-level documentation. The
//! Rust Book's [Style Guidelines](https://doc.rust-lang.org/book/appendix-06-style.html)
//! chapter is also a good reference.
//!
//! ### Error Handling
//!
//! - **`PFError`** (src/error.rs): Application-wide error enum with variants:
//!   - `NoDevice` — No smart card reader found
//!   - `Pcsc(pcsc::Error)` — PC/SC communication error
//!   - `Io(String)` — I/O or parsing error
//!   - `Device(String)` — Device-specific error
//!
//! - **`Result<T, PFError>`**: Used for device operations
//! - **`Result<T, String>`**: Used for FIDO2 operations (legacy)
//! - **`anyhow::Result`**: Used for application-level error propagation
//!
//! ### Logging
//!
//! Uses `log` facade with `log4rs` backend:
//! - **Debug builds**: `Trace` level to stdout + file
//! - **Release builds**: `Info` level to file only
//! - **Log location**: Platform-specific data directory (`ProjectDirs`)
//! - **Rotation**: 10MB size trigger, delete old logs
//!
//! ### Formatting
//!
//! - **Formatter**: `rustfmt` with project-specific config (`rustfmt.toml`)
//! - **Rule**: Always run `cargo fmt` after code changes
//! - **Indentation**: 4 spaces (Rust default)
//!
//! ### Comments
//!
//! - **Module-level**: `//!` doc comments with architecture details
//! - **Function-level**: `///` doc comments for public APIs
//! - **Inline**: Sparse, only for non-obvious logic
//! - **No**: Section dividers, "// Step 1:" narration, or obvious comments
//!
//! ### Dependency Management
//!
//! - **Minimal**: Only add dependencies when genuinely needed
//! - **Feature flags**: Enable only required features
//! - **Fork pinning**: `gpui-component` uses librekeys fork via git URL
//!
//! ---
//!
//! ## Error Handling
//!
//! ### Error Propagation Pattern
//!
//! ```text
//! Device layer:
//!   Rescue/FIDO functions return Result<T, PFError> or Result<T, String>
//!       │
//!       ▼
//! I/O layer (io.rs):
//!   Wraps errors with context, converts to PFError
//!       │
//!       ▼
//! UI layer:
//!   Catches errors, displays in dialog/notification
//! ```
//!
//! ### Error Display
//!
//! - **Device errors**: Shown in modal dialogs with user-friendly messages
//! - **Validation errors**: Shown inline in forms
//! - **Connection errors**: Shown in sidebar status indicator
//!
//! ---
//!
//! ## Firmware Compatibility
//!
//! ### Supported Firmware
//!
//! | Firmware | Language | MCU | AAGUID |
//! |----------|----------|-----|--------|
//! | pico-fido | C | RP2040, RP2350, ESP32-S3 | `89FB94B706C936739B7E30526D968145` |
//! | RS-Key | Rust | RP2350 | `2479C7BF6B3056839EC80E8171A918B7` |
//!
//! ### Firmware Detection
//!
//! PicoForge detects firmware type via:
//! 1. **AAGUID** from CTAP2 GetInfo response
//! 2. **Reader name** containing "RS-Key" or "RSK"
//! 3. **SDK version** from Rescue SELECT response
//!
//! ### Feature Availability
//!
//! | Feature | pico-fido | RS-Key |
//! |---------|-----------|--------|
//! | Rescue (PC/SC) | ✓ | ✓ |
//! | FIDO2 (CTAPHID) | ✓ | ✓ |
//! | LED Applet | ✗ | ✓ |
//! | Management Applet | ✗ | ✓ |
//! | Enterprise Attestation | ✓ | ✓ |
//! | Secure Boot | ✓ | ✓ (OTP fuses) |
//! | Legacy HW Config (≤7.2) | ✓ | ✗ |
//!
//! ---
//!
//! ## Build and Development
//!
//! ### Development Build
//!
//! ```bash
//! cargo run                    # Debug build with incremental compilation
//! cargo run --release          # Optimized release build
//! ```
//!
//! ### Code Quality
//!
//! ```bash
//! cargo fmt                    # Format code (MANDATORY after edits)
//! cargo clippy -- -D warnings  # Lint with strict warnings
//! cargo check                  # Fast type-check (no codegen)
//! ```
//!
//! ### Nix Development (NixOS)
//!
//! ```bash
//! nix develop                  # Enter dev shell with all dependencies
//! nix develop -c cargo run     # Run directly from nix shell
//! ```
//!
//! The `nix develop` environment provides:
//! - Rust toolchain with edition 2024 support
//! - `pcscd` daemon configuration
//! - All native library dependencies
//! - Development tools (rust-analyzer, clippy, rustfmt)
//!
//! ### Release Build
//!
//! ```bash
//! cargo build --release        # Binary at target/release/picoforge
//! ```
//!
//! Release profile:
//! - **LTO**: Enabled for smaller, faster binaries
//! - **Opt-level**: 3 (maximum optimization)
//! - **Strip**: Debug symbols removed
//! - **Codegen-units**: 1 (better optimization)
//!
//! ---
//!
//! ## Testing
//!
//! ### Unit Tests
//!
//! ```bash
//! cargo test                   # Run all tests
//! cargo test -- --nocapture    # Show println! output
//! ```
//!
//! Tests are located in:
//! - `src/device/fido/mod.rs` — FIDO2 parsing tests
//! - `src/device/rescue/mod.rs` — Rescue protocol tests
//! - `tests/` — Integration tests (if any)
//!
//! ### Manual Testing
//!
//! 1. Connect a Pico FIDO device
//! 2. Run `cargo run`
//! 3. Verify device detection in sidebar
//! 4. Test each view (Home, Passkeys, Configuration, Security, About)
//! 5. Test configuration changes (VID/PID, LED settings)
//! 6. Test credential operations (list, delete)
//!
//! ---
//!
//! ## Deployment and Packaging
//!
//! ### Supported Platforms
//!
//! | Platform | Format | Notes |
//! |----------|--------|-------|
//! | Linux | AppImage, .deb | Excludes `libpcsclite` (system dependency) |
//! | macOS | .dmg | For ARM (Apple Silicon) and Intel CPUs |
//! | Windows | .exe | Resource embedding via `tauri-winres` |
//!
//! ### Installation
//!
//! - **Linux**: Download AppImage, flatpak, rpm or .deb, install `pcscd` dependency
//! - **macOS**: Download the appropriate .dmg for your architecture (ARM or Intel), mount and drag to Applications
//! - **Windows**: Download .exe, no additional dependencies
//!
//! ---
//!
//! ## Contributing Guidelines
//!
//! Thank you for considering contributing to PicoForge! For full details, see
//! [CONTRIBUTING.md](.github/CONTRIBUTING.md).
//!
//! ### Workflow
//!
//! 1. Fork the repository
//! 2. Create a feature branch (`git checkout -b feature/my-feature`)
//! 3. Make changes following code style conventions
//! 4. Run `cargo fmt && cargo clippy -- -D warnings && cargo check`
//! 5. Commit with descriptive message
//! 6. Push and create a pull request against the `main` branch
//!
//! ### Pull Request Guidelines
//!
//! - Before submitting, ensure your code compiles, passes all tests, and CI checks run without errors
//! - Explicitly ask for a review from one of the maintainers
//! - If your PR goes unanswered for more than 2 weeks, feel free to tag `@lockedmutex` in the comments
//!
//! ### Commit Messages
//!
//! - Use imperative mood ("Add feature" not "Added feature")
//! - Keep subject line under 72 characters
//! - Reference issues when applicable
//!
//! ### Communication Channels
//!
//! - **Matrix**: [Join our Matrix room](https://matrix.to/#/%23librekeys:matrix.org)
//! - **Discord**: [Join our Discord server](https://discord.gg/6wYBpSHJY2)
//! - **Discussions**: [GitHub Discussions](https://github.com/librekeys/picoforge/discussions)
//! - **Issues**: [GitHub Issues](https://github.com/librekeys/picoforge/issues)
//!
//! ### Review Process
//!
//! - Our reviewers and maintainers contribute their free time to this project
//! - Please be patient, as it may take a few days for them to review, approve, or request changes
//! - For urgent matters, tag the main maintainer (`@lockedmutex`)
//!
//! ### Security
//!
//! - Never commit secrets, keys, or credentials
//! - Follow secure coding practices
//! - Review firmware protocol implementations carefully
//!
//! ---
//!
//! ## License
//!
//! PicoForge is licensed under the **GNU Affero General Public License v3.0**
//! (AGPL-3.0). See `LICENSE` for details.
//!
//! This license requires that:
//! - Source code must be available for modified binaries
//! - Network use constitutes distribution
//! - Derivative works must use the same license
//!
//! ---
//!
//! ## Acknowledgments
//!
//! - **Pol Henarejos**: pico-fido firmware and pico-keys-sdk
//! - **TheMaxMur**: RS-Key firmware
//! - **Zed Industries**: GPUI framework
//! - **Longbridge**: [gpui-component](https://github.com/longbridge/gpui-component) — Rust GUI components for building fantastic cross-platform desktop applications using GPUI
//! - **LibreKeys**: gpui-component fork with Linux bug fixes
//! - **FIDO Alliance**: CTAP2 specification
//!
//! ---
//!
//! ## References
//!
//! - [CTAP2 Specification](https://fidoalliance.org/specs/fido-v2.3-ps-20260226/fido-client-to-authenticator-protocol-v2.3-ps-20260226.html)
//! - [pico-fido Documentation](https://github.com/polhenarejos/pico-fido)
//! - [RS-Key Documentation](https://themaxmur.github.io/RS-Key/)
//! - [GPUI Documentation](https://docs.rs/gpui)
//! - [gpui-component](https://github.com/longbridge/gpui-component)
//! - [PC/SC Specification](https://pcsc1groupwg.readthedocs.io/)
//! - [ISO 7816-4](https://www.iso.org/standard/74873.html)
//!

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

use gpui::*;
use gpui_component::Root;
use gpui_component::{Theme, ThemeMode, ThemeSet};
use ui::rootview::ApplicationRoot;

pub mod error;
mod hal;
pub mod logging;
mod ui;

fn main() {
    logging::logger_init();
    let app = Application::new().with_assets(ui::assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        // Register sidebar toggle keybinding
        cx.bind_keys([gpui::KeyBinding::new(
            "ctrl-shift-d",
            ui::rootview::ToggleSidebar,
            None,
        )]);

        let theme_json = include_str!("../themes/picoforge-zinc.json");
        if let Ok(theme_set) = serde_json::from_str::<ThemeSet>(theme_json) {
            for config in theme_set.themes {
                if config.mode == ThemeMode::Dark {
                    let config = Rc::new(config);
                    Theme::global_mut(cx).apply_config(&config);
                    break;
                }
            }
        }

        cx.activate(true);

        let mut window_size = size(px(1344.0), px(756.0));

        // Basically, make sure that the window is max to max 85 percent size of the actual
        // monitor/display, so the window does not get too big on small monitors.
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;

            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                app_id: Some("in.suyogtandel.picoforge".into()),

                window_bounds: Some(WindowBounds::Windowed(window_bounds)),

                titlebar: Some(TitlebarOptions {
                    title: Some("PicoForge".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
                }),

                // Render our own window decorations(shadows and resize attack area) for linux/bsd.
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                window_decorations: Some(gpui::WindowDecorations::Client),

                window_min_size: Some(gpui::Size {
                    width: px(450.),
                    height: px(400.),
                }),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            cx.open_window(window_options, |window, cx| {
                let view = cx.new(ApplicationRoot::new);
                window.focus(&view.read(cx).focus_handle());
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();

        // Quit the application when the window is closed (specifically needed for macOS)
        #[cfg(target_os = "macos")]
        {
            cx.on_window_closed(|cx| cx.quit()).detach();
        }
    });
}
