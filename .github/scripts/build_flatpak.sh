#!/bin/bash
set -e

# ──────────────────────────────────────────────
# Utility Functions
# ──────────────────────────────────────────────

parse_platform() {
    local platform="$1"
    case "${platform}" in
        ubuntu-24.04-arm)
            echo "arm64 aarch64"
            ;;
        *)
            echo "x86-64 x86_64"
            ;;
    esac
}

# ──────────────────────────────────────────────
# Build Steps
# ──────────────────────────────────────────────

setup_runtime() {
    local flatpak_arch="$1"

    flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
    flatpak install --user -y flathub \
        org.freedesktop.Platform//25.08 \
        org.freedesktop.Sdk//25.08 \
        org.freedesktop.Sdk.Extension.rust-stable//25.08
}

build_flatpak() {
    local flatpak_arch="$1"
    flatpak-builder --user --arch="${flatpak_arch}" --force-clean --repo=repo build-dir .github/manifests/in.suyogtandel.picoforge.json
}

bundle_flatpak() {
    local version="$1"
    local arch="$2"
    flatpak build-bundle repo "picoforge_${version}_${arch}.flatpak" in.suyogtandel.picoforge
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────
main() {
    local platform="${1}"

    local ver
    ver="${version:-0.0.0}"

    local arch_flatpak_arch
    arch_flatpak_arch="$(parse_platform "${platform}")"
    read -r arch flatpak_arch <<< "${arch_flatpak_arch}"

    echo "Building Flatpak for architecture: ${flatpak_arch} (Release: ${ver})"

    setup_runtime "${flatpak_arch}"
    build_flatpak "${flatpak_arch}"
    bundle_flatpak "${ver}" "${arch}"
}

main "$@"
