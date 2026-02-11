{
  pkgs ? import <nixpkgs> { },
}:

let
  libraries = with pkgs; [
    pcsclite
    hidapi
    mesa
    udev
    libxkbcommon
    vulkan-loader
    wayland
    libglvnd
    wayland-protocols
    libunwind
    libdrm
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libxcb
  ];

  packages = with pkgs; [
    curl
    wget
    pkg-config
    dbus
    openssl_3
    librsvg
    git

    # Development tools
    rustc
    clippy
    rustfmt
    rust-analyzer
    rustPlatform.rustLibSrc
    mold
    cargo

    # GPUI
    libxkbcommon

    # Hardware
    pcsclite
    hidapi
    udev
  ];
in
pkgs.mkShell {
  buildInputs = packages;

  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

  shellHook = ''
    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
    export XDG_DATA_DIRS=$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS
    export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

    # Try to uncomment the following lines if you encounter EGL_BAD_PARAMETER errors:
    # export LIBGL_ALWAYS_SOFTWARE=1
    # export WEBKIT_DISABLE_COMPOSITING_MODE=1

    echo "Nix development environment loaded!"
    echo "Available tools: rustc, cargo"
  '';
}
