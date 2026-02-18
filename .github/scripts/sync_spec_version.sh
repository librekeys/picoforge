#!/bin/sh
set -e

dnf install -y tito git

git config --global --add safe.directory /workspace
git config --global user.email 'git@suyogtandel.in'
git config --global user.name 'Suyog Tandel'

git remote set-url origin https://x-access-token:$GITHUB_TOKEN@github.com/$GITHUB_REPOSITORY.git

git reset --hard HEAD
git clean -fd

git fetch origin main
git rebase origin/main

CARGO_FILE="Cargo.toml"
NEW_VERSION=$(grep -m 1 '^version' "$CARGO_FILE" | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')

CURRENT_SPEC_VERSION=$(grep -m 1 '^Version:' picoforge.spec | awk '{print $2}')
CURRENT_SPEC_RELEASE=$(grep -m 1 '^Release:' picoforge.spec | awk '{print $2}' | sed 's/%{?dist}//')

echo "Cargo Version: $NEW_VERSION"
echo "Spec Version:  $CURRENT_SPEC_VERSION"
echo "Spec Release:  $CURRENT_SPEC_RELEASE"

if [ "$NEW_VERSION" != "$CURRENT_SPEC_VERSION" ]; then
    echo "New version detected. Resetting Release to 1."
    sed -i "s/^Version:.*/Version:        $NEW_VERSION/" picoforge.spec
    sed -i "s/^Release:.*/Release:        1%{?dist}/" picoforge.spec
else
    echo "Version match. Incrementing Release number."
    NEW_RELEASE=$((CURRENT_SPEC_RELEASE + 1))
    sed -i "s/^Release:.*/Release:        $NEW_RELEASE%{?dist}/" picoforge.spec
fi

if git diff --quiet picoforge.spec; then
echo "No changes to spec file."
else
git add picoforge.spec
git commit -m "chore: sync spec to $NEW_VERSION [skip ci]"
fi

tito tag --offline --accept-auto-changelog --keep-version
git push --follow-tags origin HEAD:main HEAD:release