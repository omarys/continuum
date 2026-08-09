# Continuum — KDE Plasma Native Manhwa Reader (plasma branch)

**Continuum** is a high-performance, minimal comic reader written in Rust, specifically optimized for vertical continuous scrolling of **Manhwa** and **Webtoons** stored in `.cbz` / `.zip` archives.

This branch (`plasma`) is refactored to be **KDE Plasma Native**, built with **Qt 6, QML & Kirigami (KF6)**, seamlessly integrating with the KDE Plasma desktop, Breeze styling, and Kirigami HIG.

---

## Key Features

- **KDE Plasma & Kirigami (KF6) Design**: Modern Linux KDE Plasma desktop integration with Breeze dark theme styling, Kirigami HeaderBar, action items, and native Qt file pickers.
- **Click Anywhere to Open**: When no comic is open, clicking anywhere on the screen opens the native KDE `.cbz` file picker.
- **Continuous & Seamless Multi-Archive Scrolling**: Automatically detects sister `.cbz` files in the same directory (e.g. `SoloLeveling_Ch01.cbz`, `SoloLeveling_Ch02.cbz`) and appends next chapters as you scroll.
- **Vertical & Horizontal Reading Modes**: Toggle instantly between Vertical continuous scrolling (Manhwa) and Horizontal page stepping (Manga) with key `M`.
- **High-Performance Memory Management**:
  - Uses Rust background threadpool for fast `zip` extraction and RGBA decoding.
  - Custom `QQuickImageProvider` serving dynamic textures directly to QML scene graph at 60FPS.
  - Maintains 256MB pre-buffer and 1024MB memory cap.

---

## System Requirements & Prerequisites (Arch Linux / Fedora / Ubuntu KDE)

Ensure Qt 6 and Kirigami development packages are installed:

```bash
# Arch Linux
sudo pacman -S qt6-base qt6-declarative kirigami kirigami-addons cmake extra-cmake-modules

# Fedora KDE
sudo dnf install qt6-qtbase-devel qt6-qtdeclarative-devel kf6-kirigami-devel cmake extra-cmake-modules

# Ubuntu KDE / Kubuntu
sudo apt install qt6-base-dev qml6-module-org-kde-kirigami kf6-kirigami-dev cmake extra-cmake-modules
```

---

## Build & Run

```bash
# Run application
cargo run

# Run with a specific CBZ archive
cargo run -- sample_comics/Solo_Leveling_Ch01.cbz
```

---

## Desktop Installation

To install **Continuum** to your KDE Plasma application launcher menu:

```bash
./install.sh
```
