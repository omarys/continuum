import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: engine.hasComic ? engine.comicTitle + " — Continuum" : "Continuum — KDE Plasma Manhwa Reader"
    width: Math.min(Screen.width * 0.58, 1100)
    height: Screen.height * 0.85
    minimumWidth: 480
    minimumHeight: 600
    visible: true

    // Header Bar Actions
    globalDrawer: null

    header: Kirigami.HeaderBar {
        title: engine.hasComic ? engine.comicTitle : "Continuum"
        subtitle: engine.hasComic ? ("Chapter " + engine.currentChapterIdx + " of " + engine.totalChaptersCount + " • " + (engine.readingMode === 0 ? "Vertical Scroll Mode" : "Horizontal Manga Mode")) : "Continuous Manhwa Reader"

        actions: [
            Kirigami.Action {
                text: "Open .cbz"
                icon.name: "document-open"
                shortcut: "Ctrl+O"
                tooltip: "Open Manhwa CBZ Archive (Ctrl+O)"
                onTriggered: fileDialog.open()
            },
            Kirigami.Action {
                text: engine.readingMode === 0 ? "Vertical (Manhwa)" : "Horizontal (Manga)"
                icon.name: engine.readingMode === 0 ? "view-split-top-bottom" : "view-split-left-right"
                shortcut: "M"
                tooltip: "Toggle Reading Mode: Vertical / Horizontal (M)"
                onTriggered: engine.toggle_reading_mode()
            },
            Kirigami.Action {
                text: "Previous Chapter"
                icon.name: "go-previous"
                shortcut: "["
                enabled: engine.hasComic
                onTriggered: readerView.prevChapter()
            },
            Kirigami.Action {
                text: "Next Chapter"
                icon.name: "go-next"
                shortcut: "]"
                enabled: engine.hasComic
                onTriggered: readerView.nextChapter()
            },
            Kirigami.Action {
                text: "Shortcuts"
                icon.name: "help-shortcut"
                shortcut: "?"
                tooltip: "Keyboard Shortcuts (?)"
                onTriggered: shortcutsSheet.open()
            }
        ]
    }

    // Native File Dialog
    FileDialog {
        id: fileDialog
        title: "Select Manhwa CBZ File"
        nameFilters: ["Comic Archives (*.cbz *.zip)", "All Files (*)"]
        onAccepted: {
            var selectedPath = fileDialog.selectedFile.toString();
            // Remove file:// prefix if present
            if (selectedPath.indexOf("file://") === 0) {
                selectedPath = selectedPath.substring(7);
            }
            engine.open_file(selectedPath);
        }
    }

    // Main Reader Component
    ReaderView {
        id: readerView
        anchors.fill: parent
    }

    // Keyboard Shortcuts Sheet
    ShortcutsSheet {
        id: shortcutsSheet
    }

    // Global Key Controller
    Item {
        focus: true
        anchors.fill: parent

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_M) {
                engine.toggle_reading_mode();
                event.accepted = true;
            } else if (event.key === Qt.Key_Q) {
                Qt.quit();
                event.accepted = true;
            } else if (event.key === Qt.Key_Question) {
                shortcutsSheet.open();
                event.accepted = true;
            } else if (event.key === Qt.Key_J) {
                readerView.scrollDown(event.modifiers & Qt.ShiftModifier ? 300 : 80);
                event.accepted = true;
            } else if (event.key === Qt.Key_K) {
                readerView.scrollUp(event.modifiers & Qt.ShiftModifier ? 300 : 80);
                event.accepted = true;
            } else if (event.key === Qt.Key_H || event.key === Qt.Key_Left || event.key === Qt.Key_BracketLeft) {
                readerView.prevChapter();
                event.accepted = true;
            } else if (event.key === Qt.Key_L || event.key === Qt.Key_Right || event.key === Qt.Key_BracketRight) {
                readerView.nextChapter();
                event.accepted = true;
            } else if (event.key === Qt.Key_Space) {
                if (event.modifiers & Qt.ShiftModifier) {
                    readerView.scrollUp(readerView.height * 0.85);
                } else {
                    readerView.scrollDown(readerView.height * 0.85);
                }
                event.accepted = true;
            }
        }
    }
}
