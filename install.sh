#!/usr/bin/env bash
set -e

echo "=== Installing Continuum (KDE Plasma Native) ==="

# Build release binary
cargo build --release

# Install binary to user path
mkdir -p ~/.local/bin
cp target/release/continuum ~/.local/bin/continuum

# Install Desktop Entry
mkdir -p ~/.local/share/applications
cp dev.continuum.ManhwaReader.desktop ~/.local/share/applications/

# Install Icon
mkdir -p ~/.local/share/icons/hicolor/512x512/apps
cp dev.continuum.ManhwaReader.png ~/.local/share/icons/hicolor/512x512/apps/

# Refresh desktop database & KDE Plasma sycoca
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database ~/.local/share/applications
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 --noincremental || true
fi

echo "✅ Continuum successfully installed to KDE Plasma environment!"
