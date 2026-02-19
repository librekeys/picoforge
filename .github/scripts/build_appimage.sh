#!/bin/bash
set -e

ARCH=$(uname -m)
if [ "$ARCH" == "x86_64" ]; then
    LINUXDEPLOY_ARCH="x86_64"
elif [ "$ARCH" == "aarch64" ]; then
    LINUXDEPLOY_ARCH="aarch64"
else
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

echo "Building AppImage for architecture: $ARCH"

echo "Downloading linuxdeploy..."
wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage"
chmod +x linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage

export APPIMAGE_EXTRACT_AND_RUN=1

BINARY_PATH="target/release/picoforge"
DESKTOP_FILE="data/in.suyogtandel.picoforge.desktop"
ICON_FILE="static/appIcons/in.suyogtandel.picoforge.svg"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

if [ ! -f "$DESKTOP_FILE" ]; then
    echo "Error: Desktop file not found at $DESKTOP_FILE"
    exit 1
fi

if [ ! -f "$ICON_FILE" ]; then
    echo "Error: Icon file not found at $ICON_FILE"
    exit 1
fi

echo "Running linuxdeploy..."
./linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage \
    --appdir AppDir \
    --executable "$BINARY_PATH" \
    --desktop-file "$DESKTOP_FILE" \
    --icon-file "$ICON_FILE" \
    --exclude-library libpcsclite.so.1 \
    --output appimage

rm "linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage"

mkdir -p target/release
mv *.AppImage target/release/

echo "AppImage build complete. Artifacts moved to target/release/"
ls -l target/release/*.AppImage
