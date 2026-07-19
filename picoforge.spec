%global debug_package %{nil}
Name:           picoforge
Version:        0.7.0
Release:        2%{?dist}
Summary:        An open source commissioning tool for Pico FIDO security keys. Developed with Rust and GPUI.
License:        AGPL-3.0
URL:            https://github.com/librekeys/picoforge
Source0:        %{name}-%{version}.tar.gz

# Dependencies needed to compile Rust
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  binutils
BuildRequires:  curl
BuildRequires:  unzip
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(freetype2)
BuildRequires:  pkgconfig(xcb)
BuildRequires:  pkgconfig(xcb-xkb)
BuildRequires:  pkgconfig(xcb-render)
BuildRequires:  pkgconfig(xcb-shape)
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(xkbcommon-x11)
# BuildRequires:  pkgconfig(vulkan)
# BuildRequires:  pkgconfig(wayland-client)

# HARDWARE / FIDO Specific
BuildRequires:  pkgconfig(libpcsclite)
BuildRequires:  pkgconfig(libudev)

%description
PicoForge is a modern desktop application for configuring and managing Pico FIDO security keys. Built with Rust and GPUI, it provides an intuitive interface for:

- Reading device information and firmware details
- Configuring USB VID/PID and product names
- Adjusting LED settings (GPIO, brightness, driver)
- Managing security features (secure boot, firmware locking) (WIP)
- Real-time system logging and diagnostics
- Support for multiple hardware variants and vendors

%prep
%autosetup

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version

%build
export PATH="$HOME/.cargo/bin:$PATH"

# Build the App
cargo build --release

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps

# 1. Install Binary
install -m 755 target/release/picoforge %{buildroot}%{_bindir}/%{name}

# 2. Install Desktop File
install -m 644 data/in.suyogtandel.picoforge.desktop %{buildroot}%{_datadir}/applications/%{name}.desktop

# 3. Install Icon
install -m 644 static/appIcons/in.suyogtandel.picoforge.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/in.suyogtandel.picoforge.svg

%files
%{_bindir}/%{name}
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/scalable/apps/in.suyogtandel.picoforge.svg

%changelog
* Sun Jul 19 2026 Suyog Tandel <git@suyogtandel.in> 0.7.0-2
- chore: sync spec to 0.7.0-2 [skip ci] (git@suyogtandel.in)
- feat: add wiki button in about screen and update application description
  (git@suyogtandel.in)
- fmt: Run cargo fmt (muravjev.mak@yandex.ru)
- feat(ui): auto-detect device hot-plug and refresh live
  (muravjev.mak@yandex.ru)
- fix: correctly read and write RS-Key device configuration
  (muravjev.mak@yandex.ru)
- fix(hal): open the FIDO HID non-exclusively on macOS (muravjev.mak@yandex.ru)
- chore: make UI more responsive and more wide (git@suyogtandel.in)
- picoforge: 0.5.0+1 -> 0.6.0 (github-actions[bot]@users.noreply.github.com)
- chore: automate spec sync via PRs and update release workflow
  (git@suyogtandel.in)
- fix(ci): security audit issues (git@suyogtandel.in)
- fix(ci): add write perms to code CI workflow for security audit step
  (git@suyogtandel.in)
- chore: rename local variables across UI module for readability
  (git@suyogtandel.in)
- refactor(hal): rename variables for clarity and extract
  format_firmware_version helper (git@suyogtandel.in)
- fix(ci): codeql build failure and securiy audit node 20 deprecation
  (git@suyogtandel.in)
- fix(ci): docs workflow deps step missing (git@suyogtandel.in)
- fix: codebase formatting (git@suyogtandel.in)
- fix: replace manual and_then/filter pattern with Option::filter
  (git@suyogtandel.in)
- ci: replace Nix with rust-toolchain, add audit-check and CodeQL jobs
  (git@suyogtandel.in)
- feat: RS-Key curve selection card, transport priority fix, RescueCurves re-
  export (git@suyogtandel.in)
- docs: add module/item docs across all 43 source files and enable
  #![deny(missing_docs)] (git@suyogtandel.in)
- refactor(hal): extract transport layer and split rescue/fido ops
  (git@suyogtandel.in)
- feat(hal): introduce RS-Key(0.3.x) support and FirmwareTrait architecture
  (git@suyogtandel.in)
- feat: refactor hal module architecture and add tests in fido module
  (git@suyogtandel.in)
- fix(ci/cd): test build for macos-x64 (git@suyogtandel.in)
- chore: Remove firmware type label from sidebar online status
  (git@suyogtandel.in)
- ci: add build matrix and cancel-in-progress concurrency (git@suyogtandel.in)
- chore: update contributing.md with new mirror sources for picoforge
  (git@suyogtandel.in)
- feat(ci/cd): new workflow to keep mirror repos synced (git@suyogtandel.in)
- refactor: establish DeviceRepo as sole HAL gateway; fix passkeys lifecycle
  regressions (git@suyogtandel.in)
- chore(docs): refactor and update the docs to point to correct dir locations
  and explain the new UI architecture. (git@suyogtandel.in)
- refactor: extract screens into view/view_model dirs, add AppModels DI, switch
  to Entity<DeviceRepo> (git@suyogtandel.in)
- chore: rename device dir to hal and create models module for repo storage
  (git@suyogtandel.in)
- fix: pr98 review, lock the storage before performing a sync to get new status
  after device reset (git@suyogtandel.in)
- fix(ui): sync cached FIDO state after mutations (kralonur1998@gmail.com)
- chore: remove verbose internal-dialogue comments and minor code cleanup
  (git@suyogtandel.in)
- feat: add project info in project documentation (git@suyogtandel.in)
- feat(device): add ML-DSA COSE algorithms and expand rescue curves Summary of
  changes: - src/device/fido/constants.rs — added ML-DSA-44/65/87 post-quantum
  COSE algorithm variants (-48 to -50); updated from_raw/Display; refreshed
  VendorCommand docs with version history and RS-Key caveat; removed stale
  CTAP2-vs-firmware discrepancy comment - src/device/rescue/constants.rs —
  added 8 new curve flags to RescueCurves (SECP256R1, SECP384R1, SECP521R1,
  BP256R1/384R1/512R1, ED25519, ED448, CURVE25519, CURVE448); corrected
  MANAGEMENT_AID doc to say it's available on both firmwares Non-breaking. All
  additions are purely additive: - CoseAlgorithm — only accessed via from_raw()
  (has _ => None catch-all) — existing match arms unaffected - RescueCurves —
  bitflags! type, existing code only touches SECP256K1 via contains/bits — new
  flags don't affect those - cargo check and cargo clippy both pass clean with
  no warnings (git@suyogtandel.in)
- fix: picoforge docs workflow failure due to missing libs (git@suyogtandel.in)
- feat: add documentation page for picoforge source code (git@suyogtandel.in)
- fix: clippy errors in new documentedation of code in device mod
  (git@suyogtandel.in)
- fix: ci workflow name (git@suyogtandel.in)
- feat: add basic rust ci workflow (git@suyogtandel.in)
- docs: add documentation to src/device/module (git@suyogtandel.in)
- chore: format code using cargo fmt (git@suyogtandel.in)
- fix: tests and update crates to latest versions (git@suyogtandel.in)
- fix: bug in fido curve config (git@suyogtandel.in)
- feat: add hardware-endpoints panel to control usb interfaces for rskey
  (git@suyogtandel.in)
- chore: format code using cargo fmt (git@suyogtandel.in)
- fix: config errors in config view (git@suyogtandel.in)
- feat: add support for RS-Keys firmware and add optito reset the device
  (git@suyogtandel.in)

* Mon Jun 22 2026 Suyog Tandel <git@suyogtandel.in> 0.6.0-1
- chore: sync spec to 0.6.0 [skip ci] (git@suyogtandel.in)
- chore: update deps and fix tito spec update script (git@suyogtandel.in)
- chore: Modify Readme.md with firmware support info (git@suyogtandel.in)
- chore: update screenshots and readme.md (git@suyogtandel.in)
- chore: format code (git@suyogtandel.in)
- fix: edge cases in conditional clauses for version check of firmware
  (git@suyogtandel.in)
- fix: update the device not connected message in config view
  (git@suyogtandel.in)
- docs: update picofido firmware compatibility notes (kralonur1998@gmail.com)
- docs: add kralonur to credits (kralonur1998@gmail.com)
- style: cargo clippy and fmt (kralonur1998@gmail.com)
- fix(ui): gate FIDO hardware config by firmware support
  (kralonur1998@gmail.com)
- refactor(fido): gate hardware config by firmware version
  (kralonur1998@gmail.com)
- refactor(fido): parse picofido 7.6 getinfo vendor commands
  (kralonur1998@gmail.com)
- refactor(fido): split raw HID and CBOR response handling
  (kralonur1998@gmail.com)
- Update according to review and fic Refresh button
  (sylvain.pelissier@gmail.com)
- Update src/device/fido/hid.rs (sylvain.pelissier@gmail.com)
- Add Enterprise attestation features: (sylvain.pelissier@gmail.com)
- chore(docs): update readme.md with flathub link (git@suyogtandel.in)
- picoforge: 0.4.1 -> 0.5.0+1 (github-actions[bot]@users.noreply.github.com)
- fix(pkg): appstream syntax error (git@suyogtandel.in)
- chore: update appstream for flathub (git@suyogtandel.in)

* Fri Mar 06 2026 Suyog Tandel <git@suyogtandel.in> 0.5.0-1
- chore: sync spec to 0.5.0 [skip ci] (git@suyogtandel.in)
- chore: bump app version to 0.5.0 (git@suyogtandel.in)
- fix(ui): migrate views to read device state from ApplicationRoot
  (git@suyogtandel.in)
- refactor: ApplicationRoot restructure (git@suyogtandel.in)
- refactor: rename GlobalDeviceState to DeviceConnectionState
  (git@suyogtandel.in)
- fix: clippy errors and remove logs view (git@suyogtandel.in)
- feat(ui): Add LibreKeys One Vendor Config to PicoForge (git@suyogtandel.in)
- fix(ui): bottom drawer issue when deleting a passkey (git@suyogtandel.in)
- fix(ui): Fido version displayed in fido information (git@suyogtandel.in)
- fix:(ui): set min pin length dialog : the enter key is triggering cancel
  instead of update (git@suyogtandel.in)
- fix(ui): error message colors in dialogs (git@suyogtandel.in)
- fix(ui): logs terminal overflowing to the right of the window
  (git@suyogtandel.in)
- fix clippy warings in fido code (fabrice.bellamy@distrilab.fr)
- remove dependency on ctap-hid-fido2 and use our own implementation of fido
  commands to avoid picoforge freeze with som bad firmwares
  (fabrice.bellamy@distrilab.fr)
- refactor: replace to_u64 match blocks with explicit enum discriminants
  (git@suyogtandel.in)
- support pico-fido version 7.4 (fabrice.bellamy@distrilab.fr)
- fix(ui): Application icon on macos and windows (#74)
  (38373466+Lab-8916100448256@users.noreply.github.com)
- fix(ui): window handling broken on windows due to interactive config on
  first(root) element (git@suyogtandel.in)
- chore(docs): update readme.md and PR template (git@suyogtandel.in)
- feat(pkg): add flatpak build manifest and workflow (#75) (git@suyogtandel.in)
- chore: Update funding.yml with donation links (git@suyogtandel.in)
- chore: delete old issue template (git@suyogtandel.in)
- chore: Update issue and feature templates (suyogtandel12@gmail.com)
- Update issue templates (suyogtandel12@gmail.com)
- chore: update github PR and ISSUE templates (git@suyogtandel.in)
- fix #72 : sidebar toggle button redesign (fabrice.bellamy@distrilab.fr)
- picoforge: 0.4.0 -> 0.4.1 (github-actions[bot]@users.noreply.github.com)

* Sun Feb 22 2026 Suyog Tandel <git@suyogtandel.in> 0.4.1-1
- chore: sync spec to 0.4.1 [skip ci] (git@suyogtandel.in)
- chore: bump app version to 0.4.1 (git@suyogtandel.in)
- fix(ui): Setup pin for a newly flashed pico-key(#68) (git@suyogtandel.in)
- build(package.nix): 0.3.1 -> 0.4.0
  (226018678+jetcookies@users.noreply.github.com)
- docs: update installation and building wiki with gpui version of the app
  (git@suyogtandel.in)
- chore: update readme screenshots (git@suyogtandel.in)

* Sun Feb 22 2026 Suyog Tandel <git@suyogtandel.in> 0.4.0-3
- chore: sync spec to 0.4.0 [skip ci] (git@suyogtandel.in)
- fix(ui): application view overflowing out of the window (git@suyogtandel.in)
- chore(ui): change corner radius of default theme (git@suyogtandel.in)
- change topbar/sidebar layout (fabrice.bellamy@distrilab.fr)
- chore(docs): Add contribution and issue templates, and contributing guide
  (git@suyogtandel.in)
- feat(ui): Add passkey info bottom sheet to the passkeyview
  (git@suyogtandel.in)
- chore(ui): Minor UI improvements in about view and config view
  (git@suyogtandel.in)
- quit the application when main window is closed on macOS
  (fabrice.bellamy@distrilab.fr)
- fix compilation error introduced by previous commit
  (fabrice.bellamy@distrilab.fr)
- fix sidebar toggle button position when sidebar is minimized
  (fabrice.bellamy@distrilab.fr)
- ui tweaks for macOS (fabrice.bellamy@distrilab.fr)
- feat(ui): Add tag component to match the svelte-shadcnui pills
  (git@suyogtandel.in)
- feat(ui): Restore original shadcn-ui based zinc theme (git@suyogtandel.in)
- fix(ui): pressing enter key, unlocks the passkeys storage
  (226018678+jetcookies@users.noreply.github.com)
- feat(ui): Show success/error states in dialogs (git@suyogtandel.in)
- feat(ui): extract dialogs into a custom component (git@suyogtandel.in)
- chore(ui): Minor code cleanup and refactor (git@suyogtandel.in)
- chore(ui): Improve logs view line spacing and set max terminal height
  (git@suyogtandel.in)
- fix: window resize cursor shown when picoforge maximised on linux
  (git@suyogtandel.in)
- chore(ci/cd): Modify release workflow to build gpui version of the
  application and fix appimages (#64) (git@suyogtandel.in)
- fix(ci/cd): release build workflow failing due to result no build command
  (git@suyogtandel.in)
- fix(ci/cd): release build workflow failing due to result output in wrong dir
  (git@suyogtandel.in)
- fix(ci/cd): release build workflow cleanup (git@suyogtandel.in)
- ci(.github/workflows/nix-update-package.yml): configure pr-title, pr-body &
  pr-labels (226018678+jetcookies@users.noreply.github.com)
- ci(.github/workflows/nix-update-package.yml): add a workflow to periodically
  update Nix package (226018678+jetcookies@users.noreply.github.com)
- ci(.github/workflows/nix-check-package.yml): add a workflow to check whether
  the Nix package can be successfully built
  (226018678+jetcookies@users.noreply.github.com)

* Wed Feb 18 2026 Suyog Tandel <git@suyogtandel.in> 0.4.0-2
- chore(ci/cd): update release workflow to use cargo packager and drop appimage
  build (git@suyogtandel.in)

* Wed Feb 18 2026 Suyog Tandel <git@suyogtandel.in> 0.4.0-1
- chore: cleanup spec file (git@suyogtandel.in)
- chore(packaging): update specfile with gpui deps for rpm build
  (git@suyogtandel.in)
- chore(packaging): add cargo packager config file (git@suyogtandel.in)
- fix(ui): fido mode config write raised in issue #62 (git@suyogtandel.in)
- fix #60 (12b@distrilab.fr)
- fix(ui): async functions blocking ui thread (git@suyogtandel.in)
- build(package.nix): revert to stable version '0.3.1'
  (226018678+jetcookies@users.noreply.github.com)
- Update Installation.md: add Mac installation instructions
  (phoeagon@gmail.com)
- chore(docs): update readme.md with new build info and credits.md with new
  deps (git@suyogtandel.in)
- fix: tip syntax in building.md and readme.md (git@suyogtandel.in)
- chore(ci): add cachix ci info (git@suyogtandel.in)
- docs: add a hint to the docs encouraging users to utilize the binary cache
  (226018678+jetcookies@users.noreply.github.com)
- build(flake.lock): nix flake update
  (226018678+jetcookies@users.noreply.github.com)
- ci: let nix build and populate cache
  (226018678+jetcookies@users.noreply.github.com)
- build(package.nix): 0.3.0 -> 0.3.1-unstable-2026-02-08
  (226018678+jetcookies@users.noreply.github.com)
- chore(ui): fix unused code and make io functions async (git@suyogtandel.in)
- fix(ui): inconsistencies in cards (git@suyogtandel.in)
- feat(ui): application logging output to logsview (git@suyogtandel.in)
- feat(ui): add font color customization to button component
  (git@suyogtandel.in)
- chore(ui): add borders to button component (git@suyogtandel.in)
- fix(ui): IO functions blocking ui thread in passkeys view
  (git@suyogtandel.in)
- feat(ui): Implement backend connection of passkeys ui (git@suyogtandel.in)
- chore(ui): convert buttons into a reusable component from entity
  (git@suyogtandel.in)
- feat(ui): implement passkeys handling and fetching with gpui
  (git@suyogtandel.in)
- chore: update shell config for gpui compilation and make configu update async
  (git@suyogtandel.in)
- build(package.nix): 0.3.0 -> 0.3.1
  (226018678+jetcookies@users.noreply.github.com)
- fix(ui): config view input field theme (git@suyogtandel.in)
- fix(ui): collapsed sidebare refresh status button (git@suyogtandel.in)
- chore(ui): refactor config view data/types (git@suyogtandel.in)
- chore: change logging levels for deps (git@suyogtandel.in)
- chore(ui): abstract away cards into a card component (git@suyogtandel.in)
- fix: led steady mode in config.rs (git@suyogtandel.in)
- feat: enable device configuration via the ui and report correct config
  (git@suyogtandel.in)
- feat(ui): implement config loading from the device module
  (git@suyogtandel.in)
- chore(ui): abstract away button into a component (git@suyogtandel.in)
- feat(ui): implement homeview functionality (git@suyogtandel.in)
- feat(ui): implement logs view skeleton in gpui (git@suyogtandel.in)
- feat: implement passkeys view skeleton (git@suyogtandel.in)
- feat(ui): add config page skeleton (git@suyogtandel.in)
- fix(ci/cd): safe deletion of spec file update script in release worflow
  (git@suyogtandel.in)
- feat: implement security view in gpui (git@suyogtandel.in)
- fix: refresh button theme (git@suyogtandel.in)
- chore: abstract pages into a pageview component (git@suyogtandel.in)
- feat: implement/migrate about page in gpui (git@suyogtandel.in)
- fix: ui children overflowing out of the window when window size is too small
  (git@suyogtandel.in)
- feat: add animation to sidebar collapsing (git@suyogtandel.in)
- feat: complete sidebar ui implementation using gpui (git@suyogtandel.in)
- chore: adjust sidebar icon size when collapsed and change min window width
  (git@suyogtandel.in)
- feat: add sidebar header and footer (git@suyogtandel.in)
- chore: change code formatting, use space for tabs and indents instead of tabs
  (git@suyogtandel.in)
- feat: add zinc colors from shadcnui (git@suyogtandel.in)
- feat: add original lucid icons and fix ui bugs with gpui window
  (git@suyogtandel.in)
- feat: init gpui frontend migration (git@suyogtandel.in)

* Thu Jan 29 2026 Suyog Tandel <git@suyogtandel.in> 0.3.1-1
- chore: sync spec to 0.3.1 [skip ci] (git@suyogtandel.in)
- chore(ci/cd): add workflow_dispatch to release workflow (git@suyogtandel.in)
- fix(ci/cd): build failure in release workflow due to git (git@suyogtandel.in)
- fix(ci/cd): tito build commit on github actions (git@suyogtandel.in)
- chore: bump app version to 0.3.1 (git@suyogtandel.in)
- chore(docs): Update Home.md and Installation.md with more info
  (git@suyogtandel.in)
- fix clippy warnings (fabrice.bellamy@distrilab.fr)
- Update the frontend save method to return the message received from the
  backend write_config command instead of a hardcoded string.
  (fabrice.bellamy@distrilab.fr)
- add debug logs in fido code (fabrice.bellamy@distrilab.fr)
- do not display the content of LED Configuration card when in fido fallback
  mode (fabrice.bellamy@distrilab.fr)
- change FullDeviceStatus.method into an enum (fabrice.bellamy@distrilab.fr)
- refactoring fido/mod.rs (fabrice.bellamy@distrilab.fr)
- refactoring fido/hid.rs (fabrice.bellamy@distrilab.fr)
- feat(docs): add building from source docs to wiki (git@suyogtandel.in)
- fix(nix): add udev to libraries in shell.nix (git@suyogtandel.in)
- chore(nix): add mold linker to shell.nix to improve linking speeds and also
  fix libcanberra errors (git@suyogtandel.in)
- fix: refresh device status when min pin len or pin is changed for passkey
  (git@suyogtandel.in)
- chore(ci/cd): update nightly build workflow (git@suyogtandel.in)
- chore(deps): add terser to minify the frontend code in final build
  (git@suyogtandel.in)
- chore(deps): update versions of all frontend dependencies to latest
  (git@suyogtandel.in)
- fix(ui): residential key card formatting in passkeysView (git@suyogtandel.in)
- Improve troubleshooting documentation for issue #38
  (38373466+Lab-8916100448256@users.noreply.github.com)
- squash commits that implement #37 (pico-openpgp support). See branch pico-
  openpgp for detailed commits. (12b@distrilab.fr)
- Implement #38 (#39) (38373466+Lab-8916100448256@users.noreply.github.com)
- docs(README.md): list the instructions separately for enabling and disabling
  flakes (226018678+jetcookies@users.noreply.github.com)
- docs(README.md): restore the instruction for nix-shell
  (226018678+jetcookies@users.noreply.github.com)
- better error message when trying to decrease min pin length
  (fabrice.bellamy@distrilab.fr)
- implement custom HidTransport to send set_min_pin_length command because
  ctap-hid-fido2 set_min_pin_length has a bug (fabrice.bellamy@distrilab.fr)
- fix minPinDialog submit button onclick handler (fabrice.bellamy@distrilab.fr)
- Enable the feature to chnage min pin length when a pin is defined
  (fabrice.bellamy@distrilab.fr)
- add pico-keys new USB VID/PIDs (fabrice.bellamy@distrilab.fr)
- docs(README.md): update the nix instructions to use flakes
  (226018678+jetcookies@users.noreply.github.com)
- build(flake.lock): nix flake update
  (226018678+jetcookies@users.noreply.github.com)
- build: add a basic flake.nix (226018678+jetcookies@users.noreply.github.com)
- build(package.nix): 0.2.1 -> 0.3.0
  (226018678+jetcookies@users.noreply.github.com)

* Thu Jan 22 2026 Suyog Tandel <git@suyogtandel.in> 0.3.0-1
- chore: sync spec to 0.3.0 [skip ci] (git@suyogtandel.in)
- chore: bump app version to 0.3.0 (git@suyogtandel.in)
- fix(docs): typo in troubleshooting.md (git@suyogtandel.in)
- Fix #20 (fabrice.bellamy@distrilab.fr)
- Add Nix-shell development environment section
  (38373466+Lab-8916100448256@users.noreply.github.com)
- Add troubleshooting section for pcsc issues with generic VID/PID
  (38373466+Lab-8916100448256@users.noreply.github.com)
- Update Installation.md with pcsc-lite installation instructions for Debian
  and NixOS (38373466+Lab-8916100448256@users.noreply.github.com)
- implement fido fallback for writeConfig (12b@distrilab.fr)
- build(package.nix): add wrapGAppsHook3 & copyDesktopItems to
  nativeBuildInputs (226018678+jetcookies@users.noreply.github.com)
- restore behavior when no device found as normal offline state instead of an
  error (fabrice.bellamy@distrilab.fr)
- move the connection method indication to the sidebar
  (fabrice.bellamy@distrilab.fr)
- format firmware version as major.minor and do not use AAGUID as serial number
  because it is too long and already displayed somwhere else
  (fabrice.bellamy@distrilab.fr)
- get device VID, PID and product name in fido::read_device_details()
  (fabrice.bellamy@distrilab.fr)
- display device connection method on frontend (fabrice.bellamy@distrilab.fr)
- Fallback to fido::read_device_details when rescue::read_device_details fails
  (fabrice.bellamy@distrilab.fr)
- fix(docs): incorrect github-wiki-action parameter (git@suyogtandel.in)
- chore(docs): Add doc files and wiki-sync-workflow in the repo for easy wiki
  edit (git@suyogtandel.in)
- fix(doc): readme.md formatting (git@suyogtandel.in)
- do not display the "no device found" case as an error
  (fabrice.bellamy@distrilab.fr)
- Display error instead of  when there was an error communicating with pcscd
  (fabrice.bellamy@distrilab.fr)
- docs(README.md): add instructions for building with nix
  (226018678+jetcookies@users.noreply.github.com)
- ci: update release workflow to handle tito version update
  (git@suyogtandel.in)
- Fix/copr builds (#13) (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-5
- fix: explicit gcc and make dependencies (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-4
- chore: bump spec file release version (git@suyogtandel.in)
- fix: rust path in spec file (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in>
- fix: rust path in spec file (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-3
- bump spec file release version (git@suyogtandel.in)
- fix: rust install command in spec file (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in>
- fix: rust install command in spec file (git@suyogtandel.in)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-2
- fix: use universal pkgconfig names and bump release to 2 (git@suyogtandel.in)
- chore:update rpm spec file with rust install (git@suyogtandel.in)
- fix: spec file build deps (git@suyogtandel.in)
- feat: Packaging picoforge for Fedora, CentOS/RHEL and OpenSuse (#11)
  (git@suyogtandel.in)
- build(package.nix): add nix packaging script
  (226018678+jetcookies@users.noreply.github.com)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in>
- chore:update rpm spec file with rust install (git@suyogtandel.in)
- fix: spec file build deps (git@suyogtandel.in)
- feat: Packaging picoforge for Fedora, CentOS/RHEL and OpenSuse (#11)
  (git@suyogtandel.in)
- build(package.nix): add nix packaging script
  (226018678+jetcookies@users.noreply.github.com)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in>
- fix: spec file build deps (git@suyogtandel.in)
- feat: Packaging picoforge for Fedora, CentOS/RHEL and OpenSuse (#11)
  (git@suyogtandel.in)
- build(package.nix): add nix packaging script
  (226018678+jetcookies@users.noreply.github.com)

* Sat Jan 17 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-1
- new package built with tito

* Fri Jan 16 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-1
- Initial release
