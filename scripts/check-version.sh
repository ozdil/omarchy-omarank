#!/bin/bash
# Verifies that every place carrying the release version agrees with Cargo.toml,
# the single source of truth. Exits non-zero and names the mismatching files otherwise.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

# First `version = "..."` in the [package] table
cargo_version="$(sed -n '/^\[package\]/,/^\[/{s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p}' Cargo.toml | head -n1)"
lock_version="$(sed -n '/^name = "omarank-engine"$/{n;s/^version = "\([^"]*\)"$/\1/p}' Cargo.lock | head -n1)"
manifest_version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' manifest.json | head -n1)"
pkgbuild_version="$(sed -n 's/^pkgver=\(.*\)$/\1/p' PKGBUILD | head -n1)"
panel_version="$(sed -n 's/^[[:space:]]*property string pluginVersion:[[:space:]]*"\([^"]*\)".*/\1/p' Panel.qml | head -n1)"

if [[ -z "${cargo_version}" ]]; then
    echo "check-version: could not read the package version from Cargo.toml" >&2
    exit 1
fi

status=0
check() {
    local label="$1" value="$2"
    if [[ "${value}" != "${cargo_version}" ]]; then
        echo "check-version: ${label} has version '${value:-<missing>}', expected '${cargo_version}' (Cargo.toml)" >&2
        status=1
    fi
}

check "Cargo.lock (omarank-engine)" "${lock_version}"
check "manifest.json \"version\"" "${manifest_version}"
check "PKGBUILD pkgver" "${pkgbuild_version}"
check "Panel.qml pluginVersion fallback" "${panel_version}"

if [[ "${status}" -ne 0 ]]; then
    echo "check-version: version mismatch; bump all of the above together." >&2
    exit "${status}"
fi

echo "check-version: all versions are ${cargo_version}"
