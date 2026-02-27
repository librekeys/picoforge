#!/bin/bash
set -e

PLATFORM=$1
VERSION=${version:-"0.0.0"}

if [[ "$PLATFORM" == "ubuntu-24.04-arm" ]]; then
    ARCH="arm64"
    FLATPAK_ARCH="aarch64"
else
    ARCH="x86-64"
    FLATPAK_ARCH="x86_64"
fi

echo "Building Flatpak for architecture: $FLATPAK_ARCH (Release: $VERSION)"

flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

flatpak install --user -y flathub \
    org.freedesktop.Platform//25.08 \
    org.freedesktop.Sdk//25.08 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08

flatpak-builder --user --arch="${FLATPAK_ARCH}" --force-clean --repo=repo build-dir .github/manifests/in.suyogtandel.picoforge.json

flatpak build-bundle repo "picoforge_${VERSION}_${ARCH}.flatpak" in.suyogtandel.picoforge
