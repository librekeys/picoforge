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

#### Fedora Rawhide, 43, 42

```bash
sudo dnf copr enable lockedmutex/picoforge
sudo dnf install picoforge
```

If you prefer to install the `.rpm` manually, you can download it from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).

<details>
<summary><strong>Fedora Dependencies (for manual .rpm install)</strong></summary>

```bash
sudo dnf install webkit2gtk4.1 libsoup3 gtk3 \
pcsc-lite pcsc-tools hidapi libsecret \
gstreamer1-plugins-base libavif libwebp enchant2

sudo systemctl enable --now pcscd
```
</details>

### RHEL-based Distributions

#### RHEL, CentOS, AlmaLinux, Rocky Linux (EPEL 10)

```bash
sudo dnf install dnf-plugins-core
sudo dnf copr enable lockedmutex/picoforge
sudo dnf install picoforge
```

**Troubleshooting**: Ensure EPEL repository is enabled.

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

For distributions that do not have a native package or repository (e.g., Arch Linux), you can use Distrobox or AppImage.

#### Distrobox (Recommended)

If you encounter dependency issues on your specific distribution, we highly recommend using **Distrobox**. This method runs the application inside a stable Fedora environment while integrating it seamlessly with your desktop.

**Requirements:**
- [Distrobox](https://distrobox.it/) installed on your system.
- A container engine like `podman` or `docker`.

**Step-by-Step Guide:**

1.  **Create the Container:**
    We will use a standard Fedora Toolbox image which is known to be compatible.
    ```bash
    distrobox create -i quay.io/fedora/fedora-toolbox:43 -n picoforge
    ```

2.  **Enter the Container:**
    ```bash
    distrobox enter picoforge
    ```

3.  **Install the Application:**
    ```bash
    sudo dnf copr enable lockedmutex/picoforge
    sudo dnf install picoforge
    ```

4.  **Export to Host System:**
    This step creates a shortcut in your system's application menu so you can launch it like any other app.
    ```bash
    distrobox-export --app picoforge
    ```

5.  **Launch:**
    You can now find and launch "picoforge" from your application launcher.

#### AppImage

> [!WARNING]
> AppImages in releases have a lot of issues right now and do not work correctly on a lot of distros, so it is better to avoid them if possible. Please use the Distrobox method if you can.

The AppImage format contains most dependencies and runs on almost any Linux distribution.

1.  Download the `.AppImage` file from the [Latest Releases](https://github.com/librekeys/picoforge/releases/latest).
2.  Mark the file as executable:
    ```bash
    chmod +x picoforge_*.AppImage
    ```
3.  Launch the file.

> [!NOTE]
> If the AppImage does not start, check if you are missing FUSE (Filesystem in Userspace), which is required for AppImages on some newer distributions like Ubuntu 22.04+.

### Post-Installation

After installation, you can launch picoforge from your application menu or run:

```bash
picoforge
```
