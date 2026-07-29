#!/bin/bash
set -eo pipefail

# ──────────────────────────────────────────────
# Utility Functions
# ──────────────────────────────────────────────

get_version() {
    local version
    version="$(grep -m 1 '^version' Cargo.toml | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')"
    echo "${version:-0.0.0}"
}

get_commits_since_last_tag() {
    local prev_tag
    prev_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"
    if [ -z "${prev_tag}" ]; then
        return 0
    fi
    git log "${prev_tag}"..HEAD --format="- %s (%ae)" --no-decorate
}

format_date() {
    date "+%a %b %d %Y"
}

# ──────────────────────────────────────────────
# Changelog Generation
# ──────────────────────────────────────────────

update_spec_version() {
    local version="$1"

    echo "Updating spec version to ${version}"
    sed -i "s/^Version:.*/Version:        ${version}/" picoforge.spec
    sed -i "s/^Release:.*/Release:        1%{?dist}/" picoforge.spec
}

prepend_spec_changelog() {
    local version="$1"
    local date="$2"
    local author="$3"
    local commits="$4"

    local entry
    entry="* ${date} ${author} ${version}-1
${commits}"

    awk -v e="${entry}" '
        /^%changelog/ { print; print e; next }
        { print }
    ' picoforge.spec > picoforge.spec.tmp && mv picoforge.spec.tmp picoforge.spec

    echo "Prepended entry to %changelog in spec"
}

prepend_changes_file() {
    local version="$1"
    local date="$2"
    local author="$3"
    local commits="$4"

    local entry
    entry="-------------------------------------------------------------------
${date} ${author}

- picoforge ${version}
$(echo "${commits}" | sed 's/^/  /')"

    if [ -f picoforge.changes ]; then
        echo "${entry}" > picoforge.changes.tmp
        cat picoforge.changes >> picoforge.changes.tmp
        mv picoforge.changes.tmp picoforge.changes
    else
        echo "${entry}" > picoforge.changes
    fi

    echo "Prepended entry to picoforge.changes"
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────

main() {
    local version date author commits

    version="$(get_version)"
    date="$(format_date)"
    author="Suyog Tandel <git@suyogtandel.in>"
    commits="$(get_commits_since_last_tag)"

    echo "=== Syncing changelog for version ${version} ==="

    update_spec_version "${version}"
    prepend_spec_changelog "${version}" "${date}" "${author}" "${commits}"
    prepend_changes_file "${version}" "${date}" "${author}" "${commits}"

    echo "=== Done: spec and changes synced to version ${version} ==="
}

main "$@"
