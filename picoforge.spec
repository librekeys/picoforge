%global debug_package %{nil}
Name:           picoforge
Version:        0.4.0
Release:        1%{?dist}
Summary:        An open source commissioning tool for Pico FIDO security keys. Developed with Rust and GPUI.
License:        AGPL-3.0
URL:            https://github.com/librekeys/picoforge
Source0:        %{name}-%{version}.tar.gz

# Dependencies needed to compile Rust
BuildRequires:  gcc
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
# Verify if these are needed for your specific GPUI usage or if they are pulled in transitively
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
