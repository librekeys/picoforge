{
  lib,
  rustPlatform,
  buildNpmPackage,
  fetchFromGitHub,
  makeDesktopItem,

  pkg-config,
  cargo-tauri,
  wrapGAppsHook3,
  copyDesktopItems,

  glib,
  gtk3,
  openssl,
  pcsclite,
  udev,
  webkitgtk_4_1,
}:
rustPlatform.buildRustPackage (finalAttrs: {

  pname = "picoforge";
  version = "0.3.0";

  src = fetchFromGitHub {
    owner = "librekeys";
    repo = "picoforge";
    rev = "v${finalAttrs.version}";
    hash = "sha256-26UGHCzpN14UTsSQ7gIeRh/Qv0ze03BXTjvp4tdflo8=";
  };

  cargoRoot = "src-tauri";
  buildAndTestSubdir = finalAttrs.cargoRoot;

  cargoHash = "sha256-DB54egPebUniP/yjEZc+/AY9vOChJRBA+tqnbISmEgg=";

  postPatch = ''
    sed -i src-tauri/tauri.conf.json -e '/beforeBuildCommand/d'
  '';

  nativeBuildInputs = [
    pkg-config
    cargo-tauri.hook
    wrapGAppsHook3
    copyDesktopItems
  ];

  buildInputs = [
    glib
    gtk3
    openssl
    pcsclite
    udev
    webkitgtk_4_1
  ];

  frontendDist = buildNpmPackage {
    name = "${finalAttrs.pname}-${finalAttrs.version}-frontend-dist";
    inherit (finalAttrs) src;

    npmDepsHash = "sha256-7DLooiGLzk3JRsKAftOxSf7HAgHBXCJDaAFp2p/pryc=";

    installPhase = ''
      runHook preInstall

      mkdir -p $out
      cp -r build/* $out

      runHook postInstall
    '';
  };

  preBuild = ''
    cp -r ${finalAttrs.frontendDist} build
  '';

  postInstall = ''
    install -Dm644 ${finalAttrs.src}/static/in.suyogtandel.picoforge.svg $out/share/icons/hicolor/scalable/apps/picoforge.svg
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
    description = "An open source commissioning tool for Pico FIDO security keys";
    homepage = "https://github.com/librekeys/picoforge";
    license = lib.licenses.agpl3Only;
    mainProgram = "picoforge";
  };
})
