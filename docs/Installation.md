Welcome to the official installation guide for PicoForge. This document will help you set up the application on your operating system.

## Windows

1.  #### Download the Installer

    Go to the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest) page and download the `picoforge_[version]_[arch]-setup.exe` file.

2.  #### Run the Installer

    Locate the downloaded file and run it to install the application.

3.  #### Verify Smart Card Service

    For the application to communicate with your device, the Windows Smart Card service must be running. You can ensure this by running the following command in PowerShell:

    ```powershell
    Get-Service SCardSvr | Start-Service
    ```

> [!IMPORTANT]
> Please run the application as **Administrator**. This is often required for the application to correctly detect and interact with the hardware device.

## macOS

> [!WARNING]
> The macOS version of this application is tested rarely, while it does work fine, it might have some bugs that I have not experienced, feel free to report it on issues.
> 
> If you are a developer or user who can assist with testing and debugging on macOS, contributions are highly appreciated. Please check the Issues tab or join our Discord server.

#### Setup Instructions:

- Download the application from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).
- No additional driver setup is usually required as the PC/SC framework is built into macOS.
  
> [!IMPORTANT]
> Note on Security: Picoforge images are currently adhoc-signed. You will need to manually authorize the application to run.

1. Download the App: Get the latest version from the Releases Page.

  + Apple Silicon (M1/M2/M3): Download the aarch64 (DMG) version for native performance.
  + Intel Macs: Download the x64 version.

2. Authorization (Apple Silicon): Since the app is ad-hoc signed, choose one of the following methods to get it running on Apple Silicon:

#### Option A: The "Rosetta" Path (Easiest)

If you prefer a GUI approach, you can run the Intel version via translation.

+ Download the x64 image (e.g., picoforge_0.3.1_x64.dmg).
+ Open the file. If prompted, agree to install Rosetta.
+ Go to System Settings > Privacy & Security.
+ Scroll down to the Security section and click "Open Anyway" for Picoforge.

#### Option B: The "Terminal" Path (Recommended for Performance)

Use this to run the native aarch64 version by stripping the "quarantine" flag.

+ Download and extract the aarch64 zip file to find picoforge.app.
+ Open your Terminal and navigate to the folder containing the app.
+ Run the following command to clear the extended attributes:
 
```
xattr -cr picoforge.app
```

 + Right-click (or Control-click) the app and select Open.

## Linux

We offer multiple ways to run the application on Linux. Please choose the method for your distribution.

### Global Prerequisite: Smart Card Daemon

Regardless of the installation method you choose below, your **host operating system** must have the Smart Card Daemon (`pcscd`) installed and running. This service allows the application to talk to the USB device.

#### Enable pcscd on your host:

```bash
# Command may vary by distro, commonly:
sudo systemctl enable --now pcscd
```
To have the pcscd service, you may need to install pcsc-lite if it is not installed by default on your Linux distribution.
- On Debian : `sudo apt install pcscd`
- On NixOS, add this line in your /etc/nixos/configuration.nix : `services.pcscd.enable = true;`


### Fedora

[COPR](https://copr.fedorainfracloud.org/coprs/lockedmutex/picoforge/) is the recommended way of installing the application on Fedora.

#### Fedora Rawhide, 44, 43, 42

```bash
sudo dnf copr enable lockedmutex/picoforge
sudo dnf install picoforge
```

If you prefer to install the `.rpm` manually, download the `picoforge-*.el8.*.rpm` (available for `x86_64` and `aarch64`) from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest). This RPM is built on EPEL 8 and is compatible with Fedora and all EPEL-based distributions.

```bash
sudo dnf install ./picoforge-*.el8.*.rpm
```

### RHEL-based Distributions

#### RHEL, CentOS, AlmaLinux, Rocky Linux (EPEL 10, 9, 8)

```bash
sudo dnf install dnf-plugins-core
sudo dnf copr enable lockedmutex/picoforge
sudo dnf install picoforge
```

> [!NOTE]
> For EPEL 8, use `yum` instead of `dnf` if `dnf` is not available.

**Troubleshooting**: Ensure the EPEL repository is enabled on your system.

### openSUSE

#### Tumbleweed

```bash
sudo zypper addrepo https://copr.fedorainfracloud.org/coprs/lockedmutex/picoforge/repo/opensuse-tumbleweed/lockedmutex-picoforge-opensuse-tumbleweed.repo
sudo zypper refresh
sudo zypper install picoforge
```

#### Leap 15.6

```bash
sudo zypper addrepo https://copr.fedorainfracloud.org/coprs/lockedmutex/picoforge/repo/opensuse-leap-15.6/lockedmutex-picoforge-opensuse-leap-15.6.repo
sudo zypper refresh
sudo zypper install picoforge
```

**Troubleshooting**: Verify the repository URL matches your distribution version.

For manual RPM install, download the `picoforge-*.suse.lp156.*.rpm` from [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).

```bash
sudo zypper install ./picoforge-*.suse.lp156.*.rpm
```

### openEuler

#### openEuler 24

PicoForge is available via COPR for openEuler 24.

```bash
sudo dnf copr enable lockedmutex/picoforge
sudo dnf install picoforge
```

### Debian / Ubuntu

For Debian and Ubuntu-based distributions, you can install the application using the `.deb` package.

1.  Download the `.deb` file from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).
2.  Install dependencies and the package:

<details>
<summary><strong>Debian / Ubuntu Dependencies</strong></summary>

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 \
libpcsclite1 pcscd pcsc-tools libhidapi-hidraw0 libsecret-1-0 \
gstreamer1.0-plugins-base libavif16 libwebp7 libenchant-2-2
```
</details>

```bash
# Example installation command (replace with actual filename)
sudo apt install ./picoforge_[version]_amd64.deb
```

### Other Distributions

For distributions that do not have a native package or repository (e.g., Arch Linux), you can use AppImage.

#### AppImage (Recommended)

AppImage is the recommended way to run PicoForge on unsupported distributions. The format is self-contained and runs on almost any Linux distribution.

1.  Download the `.AppImage` file from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).
2.  Mark the file as executable:
    ```bash
    chmod +x picoforge_*.AppImage
    ```
3.  Launch the file:
    ```bash
    ./picoforge_*.AppImage
    ```

> [!NOTE]
> If the AppImage does not start, check if you are missing FUSE (Filesystem in Userspace), which is required for AppImages on some newer distributions like Ubuntu 22.04+.

##### Managing AppImages with Gear Lever

For a better experience, you can use [Gear Lever](https://github.com/mijorus/gearlever) to manage AppImages. It integrates AppImages into your application menu, handles updates, and keeps them organized in a dedicated folder.

Install Gear Lever via Flatpak:

```bash
flatpak install flathub it.mijorus.gearlever
```

Once installed, simply open the downloaded `.AppImage` file with Gear Lever (or drag and drop it), and it will integrate PicoForge into your system — complete with a desktop entry and icon.

### Post-Installation

After installation, you can launch picoforge from your application menu or run:

```bash
picoforge
```
