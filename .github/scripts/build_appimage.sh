#!/bin/bash
set -eo pipefail

# ──────────────────────────────────────────────
# Utility Functions
# ──────────────────────────────────────────────

detect_src_dir() {
    if [ -f "/workspace/Cargo.toml" ]; then
        echo "/workspace"
    elif [ -f "/workspace/picoforge/Cargo.toml" ]; then
        echo "/workspace/picoforge"
    else
        echo "Error: Cargo.toml not found in /workspace or /workspace/picoforge"
        exit 1
    fi
}

get_version() {
    local version
    version="$(grep -m 1 '^version' Cargo.toml | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')"
    echo "${version:-0.0.0}"
}

get_arch_name() {
    local arch
    arch="$(uname -m)"
    case "${arch}" in
        x86_64)  echo "x86-64" ;;
        aarch64) echo "aarch64" ;;
        *)       echo "${arch}" ;;
    esac
}

compute_output_name() {
    local version="$1"
    local variant="$2"
    local arch_name="$3"

    case "${variant}" in
        glibc-2.28-x86_64|glibc-2.28-aarch64)
            echo "picoforge_${version}_glibc-2.28_${arch_name}.AppImage"
            ;;
        musl-x86_64|musl-aarch64)
            echo "picoforge_${version}_musl_${arch_name}.AppImage"
            ;;
        *)
            echo "picoforge_${version}_${variant}_${arch_name}.AppImage"
            ;;
    esac
}

# ──────────────────────────────────────────────
# Build Steps
# ──────────────────────────────────────────────

setup_update_info() {
    local zsync_glob="$1"

    if [ -z "${GITHUB_REPOSITORY_OWNER}" ] || [ -z "${GITHUB_REPOSITORY}" ]; then
        return 0
    fi

    local repo_name="${GITHUB_REPOSITORY#*/}"
    export UPDATE_INFORMATION="gh-releases-zsync|${GITHUB_REPOSITORY_OWNER}|${repo_name}|latest|${zsync_glob}"
    echo "Set UPDATE_INFORMATION: ${UPDATE_INFORMATION}"
}

build_release() {
    echo "Building release binary with Cargo..."
    cargo build --release
}

verify_assets() {
    local binary="$1"
    local desktop="$2"
    local icon="$3"

    if [ ! -f "${binary}" ]; then
        echo "Error: Binary not found at ${binary}"
        exit 1
    fi
    if [ ! -f "${desktop}" ]; then
        echo "Error: Desktop file not found at ${desktop}"
        exit 1
    fi
    if [ ! -f "${icon}" ]; then
        echo "Error: Icon file not found at ${icon}"
        exit 1
    fi
}

package_appimage() {
    local output_name="$1"
    local binary="$2"
    local desktop="$3"
    local icon="$4"

    echo "Packaging AppImage using linuxdeploy..."
    export APPIMAGE_EXTRACT_AND_RUN=1

    if [ -n "${UPDATE_INFORMATION}" ]; then
        echo "Using UPDATE_INFORMATION: ${UPDATE_INFORMATION}"
    else
        echo "UPDATE_INFORMATION not set — AppImage will not support auto-update"
    fi

    export LDAI_OUTPUT="${output_name}"

    rm -rf AppDir
    linuxdeploy \
        --appdir AppDir \
        --executable "${binary}" \
        --desktop-file "${desktop}" \
        --icon-file "${icon}" \
        --exclude-library libpcsclite.so.1 \
        --output appimage
}

move_artifacts() {
    local output_name="$1"

    local generated_appimage
    generated_appimage="$(ls -t *.AppImage 2>/dev/null | head -n 1)"
    if [ -z "${generated_appimage}" ]; then
        echo "Error: AppImage generation failed"
        exit 1
    fi

    mkdir -p /workspace/target/release
    mv "${generated_appimage}" "/workspace/target/release/${output_name}"

    local generated_zsync
    generated_zsync="$(ls -t *.zsync 2>/dev/null | head -n 1)"
    if [ -n "${generated_zsync}" ]; then
        mv "${generated_zsync}" "/workspace/target/release/${output_name}.zsync"
        sed -i "s|^URL:.*|URL: ${output_name}|" "/workspace/target/release/${output_name}.zsync"
        sed -i "s|^Filename:.*|Filename: ${output_name}|" "/workspace/target/release/${output_name}.zsync"
        echo "Moved and patched zsync to ${output_name}.zsync"
    fi
}

cleanup() {
    rm -rf AppDir
}

print_summary() {
    local output_name="$1"
    echo "=== Build Complete! Output: /workspace/target/release/${output_name} ==="
    ls -lh "/workspace/target/release/${output_name}"
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────
main() {
    local variant="${1:-glibc-x86_64}"
    echo "=== Building PicoForge AppImage for variant: ${variant} ==="

    local src_dir
    src_dir="$(detect_src_dir)"
    cd "${src_dir}"

    local version
    version="$(get_version)"
    echo "PicoForge Version: ${version}"

    local arch_name
    arch_name="$(get_arch_name)"

    local output_name
    output_name="$(compute_output_name "${version}" "${variant}" "${arch_name}")"

    local zsync_glob="${output_name/${version}/*}.zsync"
    setup_update_info "${zsync_glob}"
    build_release

    local binary="target/release/picoforge"
    local desktop="data/in.suyogtandel.picoforge.desktop"
    local icon="static/appIcons/in.suyogtandel.picoforge.svg"

    verify_assets "${binary}" "${desktop}" "${icon}"
    package_appimage "${output_name}" "${binary}" "${desktop}" "${icon}"
    move_artifacts "${output_name}"
    cleanup
    print_summary "${output_name}"
}

main "$@"
