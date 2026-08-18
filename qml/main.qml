import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: engine.has_comic ? (engine.comic_title + " — Continuum") : "Continuum — KDE Plasma Manhwa Reader"
    width: Math.min(Screen.width * 0.65, 1100)
    height: Screen.height * 0.88
    minimumWidth: 480
    minimumHeight: 600
    visible: true

    globalDrawer: null

    function toggleFullscreen() {
        root.visibility = (root.visibility === Window.FullScreen) ? Window.Windowed : Window.FullScreen;
    }

    header: Controls.ToolBar {
        visible: !engine.has_comic || readerView.showControls
        height: visible ? implicitHeight : 0

        Behavior on height {
            NumberAnimation { duration: 150 }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12

            ColumnLayout {
                spacing: 2
                Layout.alignment: Qt.AlignVCenter
                Layout.maximumWidth: root.width * 0.38

                Controls.Label {
                    text: engine.has_comic ? engine.comic_title : "Continuum"
                    font.bold: true
                    font.pointSize: 11
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                Controls.Label {
                    text: engine.has_comic ? ("Chapter " + engine.current_chapter_idx + " of " + engine.total_chapters_count + " • " + (engine.reading_mode === 0 ? "Vertical Webtoon" : "Horizontal Manga")) : "High-Performance Manhwa Reader"
                    font.pointSize: 8.5
                    opacity: 0.7
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Kirigami.ActionToolBar {
                actions: [
                    Kirigami.Action {
                        text: "Open .cbz"
                        icon.name: "document-open"
                        shortcut: "Ctrl+O"
                        tooltip: "Open Manhwa CBZ Archive (Ctrl+O)"
                        onTriggered: fileDialog.open()
                    },
                    Kirigami.Action {
                        text: engine.reading_mode === 0 ? "Webtoon" : "Manga"
                        icon.name: engine.reading_mode === 0 ? "view-split-top-bottom" : "view-split-left-right"
                        shortcut: "M"
                        tooltip: "Toggle Reading Mode: Vertical / Horizontal (M)"
                        onTriggered: engine.toggle_reading_mode()
                    },
                    Kirigami.Action {
                        text: "Prev Ch."
                        icon.name: "go-previous"
                        shortcut: "["
                        enabled: engine.has_comic && engine.current_chapter_idx > 1
                        onTriggered: readerView.prevChapter()
                    },
                    Kirigami.Action {
                        text: "Next Ch."
                        icon.name: "go-next"
                        shortcut: "]"
                        enabled: engine.has_comic && engine.current_chapter_idx < engine.total_chapters_count
                        onTriggered: readerView.nextChapter()
                    },
                    Kirigami.Action {
                        text: root.visibility === Window.FullScreen ? "Exit Fullscreen" : "Fullscreen"
                        icon.name: root.visibility === Window.FullScreen ? "view-restore" : "view-fullscreen"
                        shortcut: "F11"
                        tooltip: "Toggle Fullscreen (F11 / F / Double-tap)"
                        onTriggered: root.toggleFullscreen()
                    },
                    Kirigami.Action {
                        text: "Help"
                        icon.name: "help-shortcut"
                        shortcut: "?"
                        tooltip: "Keyboard Shortcuts (?)"
                        onTriggered: shortcutsSheet.open()
                    }
                ]
            }
        }
    }

    // Native File Dialog
    FileDialog {
        id: fileDialog
        title: "Select Manhwa / Manga CBZ Archive"
        nameFilters: ["Comic Archives (*.cbz *.zip)", "All Files (*)"]
        onAccepted: {
            var rawUrl = fileDialog.selectedFile.toString();
            var selectedPath = decodeURIComponent(rawUrl);
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
        onOpenFileDialogRequested: fileDialog.open()
        onToggleFullscreenRequested: root.toggleFullscreen()
    }

    Connections {
        target: engine
        function onJumpToInitialPageRequested(pageIdx) {
            readerView.jumpToPage(pageIdx);
        }
    }

    Component.onCompleted: {
        if (engine.has_comic && engine.initial_page_idx > 0) {
            readerView.jumpToPage(engine.initial_page_idx);
        }
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
            } else if (event.key === Qt.Key_F || event.key === Qt.Key_F11) {
                root.toggleFullscreen();
                event.accepted = true;
            } else if (event.key === Qt.Key_Q) {
                Qt.quit();
                event.accepted = true;
            } else if (event.key === Qt.Key_Question) {
                shortcutsSheet.open();
                event.accepted = true;
            } else if (event.key === Qt.Key_J || event.key === Qt.Key_Down) {
                readerView.scrollDown(event.modifiers & Qt.ShiftModifier ? 350 : 100);
                event.accepted = true;
            } else if (event.key === Qt.Key_K || event.key === Qt.Key_Up) {
                readerView.scrollUp(event.modifiers & Qt.ShiftModifier ? 350 : 100);
                event.accepted = true;
            } else if (event.key === Qt.Key_H || event.key === Qt.Key_Left || event.key === Qt.Key_BracketLeft) {
                if (engine.reading_mode === 1) {
                    readerView.scrollUp(1);
                } else {
                    readerView.prevChapter();
                }
                event.accepted = true;
            } else if (event.key === Qt.Key_L || event.key === Qt.Key_Right || event.key === Qt.Key_BracketRight) {
                if (engine.reading_mode === 1) {
                    readerView.scrollDown(1);
                } else {
                    readerView.nextChapter();
                }
                event.accepted = true;
            } else if (event.key === Qt.Key_Space) {
                if (event.modifiers & Qt.ShiftModifier) {
                    readerView.scrollUp(readerView.height * 0.85);
                } else {
                    readerView.scrollDown(readerView.height * 0.85);
                }
                event.accepted = true;
            } else if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal) {
                readerView.zoomFactor = Math.min(readerView.zoomFactor + 0.1, 2.0);
                event.accepted = true;
            } else if (event.key === Qt.Key_Minus) {
                readerView.zoomFactor = Math.max(readerView.zoomFactor - 0.1, 0.5);
                event.accepted = true;
            } else if (event.key === Qt.Key_0) {
                readerView.zoomFactor = 1.0;
                event.accepted = true;
            }
        }
    }
}
