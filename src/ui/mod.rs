//! # PicoForge UI Layer
//!
//! This module is the entire frontend of PicoForge — a GPUI-based desktop application
//! that communicates with physical Pico FIDO / RS-Key hardware security keys.
//!
//! ## Architecture
//!
//! The UI follows a **reactive component tree** model with a strict layering
//! boundary: **Views and ViewModels never import `crate::hal` directly**. All
//! hardware communication passes through [`DeviceRepo`](models::device::DeviceRepo),
//! the sole gateway to the HAL.
//!
//! At the root sits [`ApplicationRoot`](app::ApplicationRoot), which holds:
//!
//! * **Shared reactive state** — [`AppModels`](app::AppModels) wrapping
//!   [`DeviceRepo`](models::device::DeviceRepo), an `Entity<DeviceRepo>` that any
//!   view-model can read or write.
//! * **Navigation** — [`active_destination`](app::ApplicationRoot::active_destination)
//!   determines which screen is displayed.
//! * **View-model registry** — [`ViewModelStore`](app::ViewModelStore) that
//!   lazily initializes each screen's view-model on first navigation.
//! * **Sidebar** — [`Entity<AppSidebar>`] that owns its own collapse state, width
//!   animation, and toggle hover state. The toggle button is rendered by
//!   [`ApplicationRoot`](app::ApplicationRoot) as the last child of `main-area`
//!   so it paints on top of the content column.
//!
//! Each screen (Home, Passkeys, Configuration, Security, About) is split into:
//! * `view_model.rs` — the reactive state machine (`Entity<T>`, `EventEmitter<T>`)
//! * `view.rs` — the `Render` impl that builds GPUI elements from view-model state
//!
//! Views never call device hardware directly. ViewModels call `DeviceRepo::*_blocking()`
//! static methods from background tasks and push results back via `repo.apply_fresh_state()`
//! or `repo.update_fido_info()`, which emit `DeviceEvent::Updated` to all subscribers.
//!
//! ## GPUI Concepts (zed-industries/gpui)
//!
//! GPUI — <https://github.com/zed-industries/zed/tree/main/crates/gpui> — is a
//! hybrid immediate/retained-mode GPU-accelerated UI framework for Rust.
//!
//! | Concept | Role |
//! |---|---|
//! | `Entity<T>` | Reference-counted smart pointer to reactive state. Created via `cx.new(\|cx\| T::new(...))`. |
//! | `Context<Self>` | (also `WindowContext`) — used for creating entities, subscribing to events, notifying. |
//! | `Window` | Window-level operations: animations, notifications, dialogs, bounds. |
//! | `Render` trait | `fn render(&mut self, window, cx) -> impl IntoElement`. The framework calls this on `cx.notify()`. |
//! | `cx.notify()` | Schedule a re-render. Every mutation to reactive state (inside `Entity::update`) must call this. |
//! | `cx.subscribe(&entity, callback)` | Listen for events emitted by another entity. **Unbounded** — must `cx.subscribe_in` and `detach()` or leak. |
//! | `EventEmitter<E>` | Trait an entity impls to declare it emits events of type `E`. |
//! | `FocusHandle` | Focus management — returned by `cx.focus_handle()`. Needed for keyboard event routing (`track_focus`, `key_context`). |
//! | `actions!` | Macro for defining custom actions (e.g. `ToggleSidebar`). Bound to elements via `on_action(...)`. |
//!
//! **Lifecycle:** When state changes, call `cx.notify()` inside an `Entity::update`
//! closure. GPUI re-invokes `Render` on the next frame, comparing the new element
//! tree against the previous one (reconciliation).
//!
//! ## gpui-component Concepts (longbridge/gpui-component)
//!
//! gpui-component — <https://github.com/longbridge/gpui-component> — provides
//! 60+ pre-built widgets on top of GPUI. PicoForge uses the **librekeys fork**
//! (`fix/client-window-linux` branch).
//!
//! * **Stateless widgets** — builder-pattern constructors like
//!   `Button::new("id").primary().label("text").on_click(...)`.
//!   These are lightweight; they produce `impl IntoElement` in `Render`.
//! * **Stateful components** — `InputState`, `SelectState`, `SliderState`, etc.,
//!   created via `cx.new(|cx| ...)` and stored in the view-model. They need
//!   `cx.subscribe(&state, callback)` to react to user interaction.
//! * **Theming** — `ActiveTheme` trait on `cx` (e.g. `cx.theme().background`).
//!   Themes come from JSON (e.g. `themes/picoforge-zinc.json`). `Root::render_dialog_layer`
//!   and `Root::render_sheet_layer` must be called in the root `Render`.
//! * **Layout helpers** — `h_flex`, `v_flex` for flexbox-style layouts,
//!   `scroll::ScrollableElement` for scroll containers, `TitleBar` for window chrome,
//!   `Icon` for SVG icons, `tooltip::Tooltip` for hover tooltips.
//! * **`Root`** — top-level wrapper created once per window:
//!   `cx.new(|cx| Root::new(content_view, window, cx))`.
//!
//! ## Tree Structure (file-by-file)
//!
//! ```text
//! src/ui/
//! ├── mod.rs             # This file — module declaration + architecture docs
//! ├── app.rs             # ApplicationRoot, AppModels, Destination, ViewModelStore
//! │                       # Root Render: sidebar entity, title bar, content routing
//! │                       # Triggers initial DeviceRepo::refresh(), subscribes to
//! │                       # DeviceEvent to invalidate passkeys on device change
//! ├── assets.rs          # AssetLoaderImpl via rust-embed (loads SVGs from static/)
//! ├── colors.rs          # Zinc palette constants (u32 RGB). WIP — HSLA migration planned.
//! │                       # Reference: https://ui.shadcn.com/colors
//! ├── models/
//! │   ├── mod.rs         # pub mod device
//! │   └── device.rs      # DeviceRepo — reactive state for device status, FIDO info,
//! │                       # LED config, management apps, loading/error flags.
//! │                       # Implements EventEmitter<DeviceEvent>
//! ├── components/
//! │   ├── mod.rs         # Module declarations for sub-components
//! │   ├── button.rs      # Custom button widgets
//! │   ├── card.rs        # Card container widgets
//! │   ├── dialog.rs      # Custom dialog widgets
//! │   ├── page_view.rs   # Page view layout container
//! │   ├── sidebar.rs     # AppSidebar — sidebar Entity with EventEmitter
//! │   │                   # Owns collapse, width animation, toggle hover state
//! │   │                   # Renders nav items; emits Nav / RefreshDevice events
//! │   └── tag.rs         # Tag/badge widgets
//! ├── screens/
//! │   ├── mod.rs         # pub mod home, config, passkeys, security, about
//! │   ├── home/
//! │   │   ├── mod.rs     # HomeView re-export
//! │   │   ├── view_model.rs  # HomeViewModel — device summary state
//! │   │   └── view.rs    # HomeView — device status cards, connection info
//! │   ├── config/
//! │   │   ├── mod.rs     # ConfigView re-export
//! │   │   ├── view_model.rs  # ConfigViewModel — PIN management, LED, transport config
//! │   │   └── view.rs    # ConfigView — configuration form UI
//! │   ├── passkeys/
//! │   │   ├── mod.rs     # PasskeysView re-export
//! │   │   ├── view_model.rs  # PasskeysViewModel — credential list, unlock state
//! │   │   └── view.rs    # PasskeysView — passkey table, credential operations
//! │   ├── security/
//! │   │   ├── mod.rs     # SecurityView re-export
//! │   │   ├── view_model.rs  # SecurityViewModel — reset, attestation, FIDO2 config
//! │   │   └── view.rs    # SecurityView — security settings UI
//! │   └── about/
//! │       ├── mod.rs     # AboutView re-export
//! │       ├── view_model.rs  # AboutViewModel — version, firmware details
//! │       └── view.rs    # AboutView — build info, licenses, firmware version
//! ```
//!
//! ## Navigation & Data Flow
//!
//! 1. [`AppSidebar`] emits [`SidebarEvent::Navigate`] when a nav item is clicked.
//!    [`ApplicationRoot`] receives it via `cx.subscribe` and sets `active_destination`.
//! 2. `ApplicationRoot::render` reads `active_destination` to decide which screen
//!    to display. Each screen's view-model is lazily created via `get_or_insert_with`
//!    on `ViewModelStore`. Passkeys survives navigation (only invalidated on device change).
//! 3. [`DeviceRepo::refresh()`] (called at startup and on sidebar refresh) performs the
//!    full HAL poll cycle — reads device details, FIDO info, LED/management config — and
//!    emits [`DeviceEvent::Updated`]. All subscribers re-read from `DeviceRepo`.
//! 4. For writes, ViewModels call `DeviceRepo::*_blocking()` static methods from background
//!    tasks, then push fresh state via `repo.apply_fresh_state()`, which emits the event.
//! 5. Screen view-models read `DeviceRepo` in their `Render` or event handlers
//!    via `models.device.read(cx)`.
//!
//! ## External References
//!
//! * GPUI framework: <https://github.com/zed-industries/zed/tree/main/crates/gpui>
//! * gpui-component library: <https://github.com/longbridge/gpui-component>
//! * gpui-component docs: <https://longbridge.github.io/gpui-component/>
//! * shadcn color palette (used in `colors.rs`): <https://ui.shadcn.com/colors>

pub mod app;
pub mod assets;
pub mod colors;
pub mod components;
pub mod models;
pub mod screens;
