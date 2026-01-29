%global debug_package %{nil}
Name:           picoforge
Version:        0.3.1
Release:        1%{?dist}
Summary:        An open source commissioning tool for Pico FIDO security keys. Developed with Rust, Tauri, and Svelte.
License:        AGPL-3.0
URL:            https://github.com/librekeys/picoforge
Source0:        %{name}-%{version}.tar.gz

# Dependencies needed to compile Tauri/Rust
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  binutils
BuildRequires:  curl
BuildRequires:  unzip

# Standard Tauri v2 Requirements
BuildRequires:  pkgconfig(gtk+-3.0)
BuildRequires:  pkgconfig(webkit2gtk-4.1)
BuildRequires:  pkgconfig(javascriptcoregtk-4.1)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(glib-2.0)
BuildRequires:  pkgconfig(libsoup-3.0)
BuildRequires:  pkgconfig(appindicator3-0.1)

# HARDWARE / FIDO Specific
BuildRequires:  pkgconfig(libpcsclite)
BuildRequires:  pkgconfig(libudev)

%description
PicoForge is a modern desktop application for configuring and managing Pico FIDO security keys. Built with Rust, Tauri, and Svelte, it provides an intuitive interface for:

- Reading device information and firmware details
- Configuring USB VID/PID and product names
- Adjusting LED settings (GPIO, brightness, driver)
- Managing security features (secure boot, firmware locking) (WIP)
- Real-time system logging and diagnostics
- Support for multiple hardware variants and vendors

%prep
%autosetup

curl -fsSL https://deno.land/x/install/install.sh | sh
export DENO_INSTALL="$HOME/.deno"
export PATH="$DENO_INSTALL/bin:$PATH"
deno --version

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version

%build
export DENO_INSTALL="$HOME/.deno"
export PATH="$DENO_INSTALL/bin:$PATH"
export PATH="$HOME/.cargo/bin:$PATH"

# Build the App
# This will download Rust crates and Deno modules over the internet
deno install
deno task tauri build --no-bundle

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps

# 1. Install Binary
install -m 755 src-tauri/target/release/picoforge %{buildroot}%{_bindir}/%{name}

# 2. Install Desktop File
install -m 644 data/in.suyogtandel.picoforge.desktop %{buildroot}%{_datadir}/applications/%{name}.desktop

# 3. Install Icon (Assumes you have this icon in your source)
install -m 644 src-tauri/icons/in.suyogtandel.picoforge.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/in.suyogtandel.picoforge.svg

%files
%{_bindir}/%{name}
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/scalable/apps/in.suyogtandel.picoforge.svg

%changelog
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
