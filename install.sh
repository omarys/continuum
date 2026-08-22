#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Append native-target flags, preserving any user-supplied RUSTFLAGS.
export RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native"

echo "=== Installing Continuum (KDE Plasma Native) ==="

# Build release binary
cargo build --release

# Install binary to user paths (install uses unlink to prevent text file busy errors)
install -Dm755 target/release/continuum ~/.local/bin/continuum

if [ -d "$HOME/.cargo/bin" ]; then
    install -Dm755 target/release/continuum "$HOME/.cargo/bin/continuum"
fi

# Install Desktop Entry
mkdir -p ~/.local/share/applications
cp dev.continuum.ManhwaReader.desktop ~/.local/share/applications/

# Install Icon
mkdir -p ~/.local/share/icons/hicolor/512x512/apps
cp dev.continuum.ManhwaReader.png ~/.local/share/icons/hicolor/512x512/apps/

# Refresh desktop database, icon cache & KDE Plasma sycoca
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database ~/.local/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor >/dev/null 2>&1 || true
fi

if command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 --noincremental || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then
    kbuildsycoca5 --noincremental || true
fi

echo "✅ Continuum successfully installed to KDE Plasma environment!"
