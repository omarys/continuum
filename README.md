# Continuum — Modern GTK4 / Libadwaita Manhwa Reader

**Continuum** is a high-performance, minimal comic reader written in Rust, specifically optimized for vertical continuous scrolling of **Manhwa** and **Webtoons** stored in `.cbz` / `.zip` archives.

> ⚡ **100% Vibe-Coded with Gemini 3.6 Flash**
> This entire codebase was **100% vibe-coded** using **Gemini 3.6 Flash** via Google Antigravity—from native GTK4/Adwaita layout design and `zip` archive extraction to multi-threaded memory caching and zero-layout-shift scrolling.

---

## Key Features

- **Libadwaita / GTK4 Design**: Sleek, modern Linux desktop integration with dark mode aesthetics, clean status pages, header bars, and native file pickers.
- **Click Anywhere to Open**: When no comic is open, clicking anywhere on the window opens the native `.cbz` file picker.
- **Continuous & Seamless Multi-Archive Scrolling**: Automatically detects sister `.cbz` files in the same directory (e.g. `SoloLeveling_Ch01.cbz`, `SoloLeveling_Ch02.cbz`) and seamlessly appends next chapters as you scroll down.
- **Upward Scroll & Anticipatory Preloading**: Scroll back up to previous chapters effortlessly with zero delay.
- **Zero Layout Shifts**: Pre-calculates exact page heights upon opening archives to prevent scroll jumping or position shifts.
- **Lazy Load & Memory Management**:
  - Uses `zip` crate to lazy load page contents asynchronously on background thread pools.
  - Maintains a **minimum of 256MB** loaded pre-buffer in memory for stutter-free instant scrolling.
  - Enforces a **maximum 1024MB (1GB)** memory cap, keeping memory light without interrupting the reader.

---

## Desktop Launcher & System Installation

To install **Continuum** to your Linux desktop application menu with system icons and `.cbz` file manager associations:

```bash
./install.sh
```

This installs:
1. Binary: `~/.cargo/bin/continuum`
2. Desktop Entry: `~/.local/share/applications/dev.continuum.ManhwaReader.desktop`
3. Application Icon: `~/.local/share/icons/hicolor/512x512/apps/dev.continuum.ManhwaReader.png`

Once installed, **Continuum** appears in your GNOME / KDE / XFCE app launcher menu and allows opening `.cbz` files directly from your file manager.

---

## Development Setup

Dependencies are managed with `mise` and pre-commit hooks are configured via `pre-commit-config.yml`.

### Prerequisites (Ubuntu / Debian / Fedora / Arch)

Ensure system GTK4 & Libadwaita development headers are installed:
```bash
# Ubuntu / Debian
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev pkg-config

# Fedora
sudo dnf install gtk4-devel libadwaita-devel pkg-config

# Arch Linux
sudo pacman -S gtk4 libadwaita pkg-config
```

### Build & Run from CLI

```bash
# Run application
cargo run

# Run with a sample archive
cargo run -- sample_comics/Solo_Leveling_Ch01.cbz
```
