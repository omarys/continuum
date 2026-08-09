import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: readerRoot

    function scrollDown(amount) {
        if (engine.readingMode === 0) {
            verticalFlickable.contentY = Math.min(verticalFlickable.contentY + amount, verticalFlickable.contentHeight - verticalFlickable.height);
        } else {
            horizontalListView.incrementCurrentIndex();
        }
    }

    function scrollUp(amount) {
        if (engine.readingMode === 0) {
            verticalFlickable.contentY = Math.max(verticalFlickable.contentY - amount, 0);
        } else {
            horizontalListView.decrementCurrentIndex();
        }
    }

    function nextChapter() {
        engine.next_chapter();
    }

    function prevChapter() {
        // Will jump to previous chapter
    }

    // Empty State Placeholder
    Kirigami.PlaceholderMessage {
        id: placeholder
        anchors.centerIn: parent
        visible: !engine.hasComic
        width: Math.min(parent.width - 64, 480)
        icon.name: "dev.continuum.ManhwaReader"
        text: "No Comic Open"
        explanation: "Click anywhere or press Ctrl+O to select a .cbz Manhwa file"

        helpfulActions: [
            Kirigami.Action {
                text: "Open .cbz Archive"
                icon.name: "document-open"
                onTriggered: fileDialog.open()
            }
        ]
    }

    // Click anywhere when empty to open file dialog
    MouseArea {
        anchors.fill: parent
        visible: !engine.hasComic
        onClicked: fileDialog.open()
    }

    // Active Reader (Vertical Mode)
    Flickable {
        id: verticalFlickable
        anchors.fill: parent
        visible: engine.hasComic && engine.readingMode === 0
        contentWidth: parent.width
        contentHeight: verticalColumn.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        onContentYChanged: {
            // Trigger preloading around visible area
            var approxIndex = Math.floor((contentY / Math.max(contentHeight, 1)) * engine.get_total_page_count());
            engine.request_pages_around(approxIndex);
        }

        Column {
            id: verticalColumn
            width: Math.min(readerRoot.width, 900)
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            Repeater {
                model: engine.get_chapter_count()

                Column {
                    id: chapterGroup
                    width: parent.width
                    property int chapIdx: index

                    // Chapter Banner Header
                    Rectangle {
                        width: parent.width
                        height: 56
                        color: "#181b1d"

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 16
                            spacing: 12

                            Rectangle {
                                width: 4
                                height: 24
                                color: "#3daee9"
                                radius: 2
                            }

                            Kirigami.Heading {
                                text: engine.get_chapter_title(chapIdx)
                                level: 3
                                Layout.fillWidth: true
                                color: "#eff0f1"
                            }

                            Controls.Label {
                                text: engine.get_chapter_page_count(chapIdx) + " Pages"
                                color: "#bdc3c7"
                                font.pointSize: 10
                            }
                        }
                    }

                    // Pages List in Chapter
                    Repeater {
                        model: engine.get_chapter_page_count(chapIdx)

                        Item {
                            width: chapterGroup.width
                            height: Math.max(width * (1400 / 800), 200)

                            Image {
                                id: pageImg
                                anchors.fill: parent
                                fillMode: Image.PreserveAspectFit
                                source: "image://cbz/" + chapIdx + "/" + index
                                asynchronous: true
                                cache: false

                                // Loading Placeholder indicator
                                Rectangle {
                                    anchors.fill: parent
                                    color: "#1f2326"
                                    visible: pageImg.status !== Image.Ready

                                    Controls.BusyIndicator {
                                        anchors.centerIn: parent
                                        running: pageImg.status === Image.Loading
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Active Reader (Horizontal Mode)
    ListView {
        id: horizontalListView
        anchors.fill: parent
        visible: engine.hasComic && engine.readingMode === 1
        orientation: ListView.Horizontal
        snapMode: ListView.SnapOneItem
        highlightRangeMode: ListView.StrictlyEnforceRange
        spacing: 16
        clip: true

        model: engine.get_total_page_count()

        delegate: Item {
            width: horizontalListView.width
            height: horizontalListView.height

            Image {
                anchors.fill: parent
                fillMode: Image.PreserveAspectFit
                source: "image://cbz/0/" + index
                asynchronous: true
                cache: false
            }
        }
    }
}
