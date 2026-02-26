#!/usr/bin/env zsh
set -e

PLATFORM=$1
VERSION=${version:-"0.0.0"}

if [[ "$PLATFORM" == "ubuntu-24.04-arm" ]]; then
    ARCH="arm64"
else
    ARCH="x86-64"
fi

# Setup flathub repository
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/

# Install flatpak runtime and SDKs
flatpak install --user -y flathub \
    org.freedesktop.Platform//25.08 \
    org.freedesktop.Sdk//25.08 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08

# Build the flatpak
flatpak-builder --user --force-clean --repo=repo build-dir .github/manifests/in.suyogtandel.picoforge.json

# Create the flatpak bundle
flatpak build-bundle repo "picoforge_${VERSION}_${ARCH}.flatpak" in.suyogtandel.picoforge
