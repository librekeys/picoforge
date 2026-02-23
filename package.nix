{
  lib,
  stdenv,
  rustPlatform,
  fetchFromGitHub,
  makeDesktopItem,

  pkg-config,
  copyDesktopItems,
  makeWrapper,

  hidapi,
  pcsclite,
  udev,
  alsa-lib,
  libxkbcommon,
  wayland,
  libGL,
  vulkan-loader,
  libx11,
  libxcb,

  withGLES ? false,
}:
assert withGLES -> stdenv.hostPlatform.isLinux;
rustPlatform.buildRustPackage (finalAttrs: {

  pname = "picoforge";
  version = "0.4.1";

  src = fetchFromGitHub {
    owner = "librekeys";
    repo = "picoforge";
    rev = "v${finalAttrs.version}";
    hash = "sha256-dggznCwFs1oYLbf8IYk2RZ+VvyuzfdDlhLPFEdee3mA=";
  };

  cargoHash = "sha256-SGKoj72WA8qrptI8j05mFczr30PHNnTSy6qyjy6T768=";

  nativeBuildInputs = [
    pkg-config
    copyDesktopItems
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    makeWrapper
  ];

  gpu-lib = if withGLES then libGL else vulkan-loader;

  buildInputs = [
    hidapi
    pcsclite
    udev
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    alsa-lib
    libxkbcommon
    wayland
    finalAttrs.gpu-lib
    libx11
    libxcb
  ];

  postInstall = ''
    install -Dm644 ${finalAttrs.src}/static/appIcons/in.suyogtandel.picoforge.svg $out/share/icons/hicolor/scalable/apps/picoforge.svg
  '';

  postFixup = lib.optionalString stdenv.hostPlatform.isLinux ''
    wrapProgram $out/bin/picoforge --prefix LD_LIBRARY_PATH : "${
      lib.makeLibraryPath [
        wayland
        libxkbcommon
        finalAttrs.gpu-lib
        libx11
        libxcb
      ]
    }"
  '';

  desktopItems = [
    (makeDesktopItem {
      name = "in.suyogtandel.picoforge";
      desktopName = "PicoForge";
      exec = "picoforge";
      terminal = false;
      icon = "picoforge";
      comment = finalAttrs.meta.description;
      categories = [ "Utility" ];
      dbusActivatable = true;
      keywords = [ "Config" ];
      startupNotify = true;
    })
  ];

  meta = {
    changelog = "https://github.com/librekeys/picoforge/releases/tag/v${finalAttrs.version}";
    description = "Open source commissioning tool for Pico FIDO security keys";
    homepage = "https://github.com/librekeys/picoforge";
    license = lib.licenses.agpl3Only;
    mainProgram = "picoforge";
    platforms = lib.platforms.linux;
  };
})
