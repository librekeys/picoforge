#!/bin/bash

# Script to update the Homebrew formula with a new commit hash
# Usage:
#
# ./update_formula.sh 9c5ec68673a55eb13dc93cbab723f6683f211ea7

#
set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <commit-hash>"
  exit 1
fi

COMMIT_HASH=$1
FORMULA="Formula/picoforge.rb"

echo "Updating $FORMULA to commit $COMMIT_HASH..."

# 1. Download the tarball for the new commit hash and calculate the new sha256
URL="https://github.com/librekeys/picoforge/archive/${COMMIT_HASH}.tar.gz"
echo "Fetching $URL to calculate sha256..."
NEW_SHA256=$(curl -sL "$URL" | shasum -a 256 | awk '{print $1}')

if [ -z "$NEW_SHA256" ]; then
  echo "Failed to calculate SHA256 checksum."
  exit 1
fi

echo "New SHA256: $NEW_SHA256"

# 2. Update the url and sha256 lines in the formula
# We look for lines matching 'url "..."' and 'sha256 "..."' and replace them
sed -i.bak -E "s|url \".*\"|url \"$URL\"|" "$FORMULA"
sed -i.bak -E "s|sha256 \".*\"|sha256 \"$NEW_SHA256\"|" "$FORMULA"

rm -f "${FORMULA}.bak"

echo "Successfully updated $FORMULA."
