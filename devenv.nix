{
  pkgs,
  lib,
  ...
}:
{
  languages = {
    rust = {
      enable = true;
      channel = "stable";
    };
  };

  packages = lib.optionals pkgs.stdenv.isLinux [
    # PicoForge
    pkgs.hidapi
    pkgs.pcsclite
    pkgs.udev
    # GPUI
    pkgs.curl
    pkgs.pkg-config
    pkgs.fontconfig
    pkgs.freetype
    pkgs.libxkbcommon
    pkgs.libx11
    pkgs.libxcb
    pkgs.libxcursor
    pkgs.libxi
    pkgs.libxrandr
    pkgs.libGL
    pkgs.vulkan-loader
    pkgs.wayland
  ];

  env = lib.optionalAttrs pkgs.stdenv.isLinux {
    LD_LIBRARY_PATH = "${lib.makeLibraryPath [
      pkgs.libGL
      pkgs.libxkbcommon
      pkgs.wayland
      pkgs.vulkan-loader
    ]}";
  };
}
