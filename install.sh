#!/usr/bin/env bash
set -e

echo "Building and installing Continuum..."
cargo install --path .

echo "Installing desktop entry and icons..."
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/512x512/apps
mkdir -p ~/.local/share/pixmaps

cp dev.continuum.ManhwaReader.png ~/.local/share/icons/hicolor/512x512/apps/dev.continuum.ManhwaReader.png
cp dev.continuum.ManhwaReader.png ~/.local/share/pixmaps/dev.continuum.ManhwaReader.png
cp dev.continuum.ManhwaReader.desktop ~/.local/share/applications/dev.continuum.ManhwaReader.desktop

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database ~/.local/share/applications
fi

echo "Done! Continuum is now installed and available in your application launcher menu."
