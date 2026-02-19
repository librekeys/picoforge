#!/bin/bash
set -e

# Detected architecture
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

# Download linuxdeploy
echo "Downloading linuxdeploy..."
wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage"
chmod +x linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage

# Set up environment variables for linuxdeploy
export APPIMAGE_EXTRACT_AND_RUN=1

# Define paths
BINARY_PATH="target/release/picoforge"
DESKTOP_FILE="data/in.suyogtandel.picoforge.desktop"
ICON_FILE="static/appIcons/in.suyogtandel.picoforge.svg"

# Verify files exist
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

# Run linuxdeploy
# --appdir AppDir: specifies the AppDir location
# --executable: path to the main executable
# --desktop-file: path to the desktop entry
# --icon-file: path to the icon
# --output appimage: generate an AppImage file
# --exclude-library: excludes specific libraries (libpcsclite as requested)

echo "Running linuxdeploy..."
./linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage \
    --appdir AppDir \
    --executable "$BINARY_PATH" \
    --desktop-file "$DESKTOP_FILE" \
    --icon-file "$ICON_FILE" \
    --exclude-library libpcsclite.so.1 \
    --output appimage

# Move generated AppImage to target/release
mkdir -p target/release
mv *.AppImage target/release/

echo "AppImage build complete. Artifacts moved to target/release/"
ls -l target/release/*.AppImage
