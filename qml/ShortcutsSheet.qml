import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.OverlaySheet {
    id: sheet
    title: "Keyboard & Touch Shortcuts"

    ColumnLayout {
        Kirigami.FormLayout {
            wideMode: true
            Layout.preferredWidth: Kirigami.Units.gridUnit * 22

        Kirigami.Heading {
            text: "Reading & Scrolling"
            level: 3
        }

        Controls.Label {
            Kirigami.FormData.label: "Toggle Mode:"
            text: "M / D (Vertical Webtoon / Horizontal Manga)"
        }

        Controls.Label {
            Kirigami.FormData.label: "Scroll Down / Up:"
            text: "J / K or Down / Up Arrow"
        }

        Controls.Label {
            Kirigami.FormData.label: "Fast Scroll:"
            text: "Shift + J / Shift + K"
        }

        Controls.Label {
            Kirigami.FormData.label: "Page Scroll:"
            text: "Space (Down) / Shift + Space (Up)"
        }

        Controls.Label {
            Kirigami.FormData.label: "Chapter Top / Bottom:"
            text: "gg / Shift + G"
        }

        Controls.Label {
            Kirigami.FormData.label: "Zoom In / Out / Reset:"
            text: "+ / - / 0"
        }

        Kirigami.Heading {
            text: "Chapter Navigation"
            level: 3
        }

        Controls.Label {
            Kirigami.FormData.label: "Next / Prev Chapter:"
            text: "] / [ or L / H (in Webtoon mode)"
        }

        Kirigami.Heading {
            text: "Interface & Application"
            level: 3
        }

        Controls.Label {
            Kirigami.FormData.label: "Toggle Fullscreen:"
            text: "F11 / F or Double-tap screen"
        }

        Controls.Label {
            Kirigami.FormData.label: "Toggle Controls / Immersive:"
            text: "Tap comic page center"
        }

        Controls.Label {
            Kirigami.FormData.label: "Open Archive:"
            text: "Ctrl + O"
        }

        Controls.Label {
            Kirigami.FormData.label: "Shortcuts Help:"
            text: "?"
        }

        Controls.Label {
            Kirigami.FormData.label: "Quit Application:"
            text: "Q"
        }
    }
}
}
