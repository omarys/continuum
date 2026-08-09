import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.OverlaySheet {
    id: sheet
    title: "Keyboard Shortcuts Cheat Sheet"

    Kirigami.FormLayout {
        wideMode: true

        Kirigami.Heading {
            text: "Reading & Navigation"
            level: 3
        }

        Controls.Label {
            Kirigami.FormData.label: "Toggle Reading Mode:"
            text: "M (Manhwa Vertical / Manga Horizontal)"
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
            Kirigami.FormData.label: "Full Page Scroll:"
            text: "Space / Shift + Space"
        }

        Kirigami.Heading {
            text: "Chapter Navigation"
            level: 3
        }

        Controls.Label {
            Kirigami.FormData.label: "Next / Previous Chapter:"
            text: "L / H or ] / ["
        }

        Kirigami.Heading {
            text: "Application"
            level: 3
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
