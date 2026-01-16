Name:           picoforge
Version:        0.2.1
Release:        1%{?dist}
Summary:        An open source commissioning tool for Pico FIDO security keys. Developed with Rust, Tauri, and Svelte.
License:        AGPL-3.0
URL:            https://github.com/librekeys/picoforge
Source0:        %{name}-%{version}.tar.gz

# Dependencies needed to compile Tauri/Rust
BuildRequires:  curl
BuildRequires:  unzip

# Standard Tauri v2 Requirements
BuildRequires:  gtk3-devel
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  openssl-devel
BuildRequires:  glib2-devel
BuildRequires:  libsoup3-devel
BuildRequires:  libappindicator-gtk3-devel

# HARDWARE / FIDO Specific
BuildRequires:  pcsc-lite-devel
BuildRequires:  systemd-devel

# Build Tools
BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  curl
BuildRequires:  unzip

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

%build
export DENO_INSTALL="$HOME/.deno"
export PATH="$DENO_INSTALL/bin:$PATH"

# Build the App
# This will download Rust crates and Deno modules over the internet
deno task tauri build --target release

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps

# 1. Install Binary
install -m 755 src-tauri/target/release/picoforge %{buildroot}%{_bindir}/%{name}

# 2. Install Desktop File
install -m 644 data/in.suyogtandel.picoforge.desktop %{buildroot}%{_datadir}/applications/%{name}.desktop

# 3. Install Icon (Assumes you have this icon in your source)
install -m 644 src-tauri/icons/in.suyogtandel.picoforge.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{name}.png

%files
%{_bindir}/%{name}
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/scalable/apps/%{name}.png

%changelog
* Fri Jan 16 2026 Suyog Tandel <git@suyogtandel.in> 0.2.1-1
- Initial release