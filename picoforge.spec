%global debug_package %{nil}
Name:           picoforge
Version:        0.4.0
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
