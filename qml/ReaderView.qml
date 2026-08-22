import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: readerRoot

    signal openFileDialogRequested()
    signal toggleFullscreenRequested()

    property bool showControls: true
    property real zoomFactor: 1.0

    function toggleControls() {
        showControls = !showControls;
    }

    function scrollDown(amount) {
        if (engine.reading_mode === 0) {
            verticalListView.contentY = Math.min(
                verticalListView.contentY + amount,
                Math.max(verticalListView.contentHeight - verticalListView.height, 0)
            );
        } else {
            horizontalListView.incrementCurrentIndex();
        }
    }

    function scrollUp(amount) {
        if (engine.reading_mode === 0) {
            verticalListView.contentY = Math.max(verticalListView.contentY - amount, 0);
        } else {
            horizontalListView.decrementCurrentIndex();
        }
    }

    function nextChapter() {
        var currentSeriesIdx = engine.current_chapter_idx - 1;
        var nextSeriesIdx = currentSeriesIdx + 1;
        if (nextSeriesIdx < engine.total_chapters_count) {
            var firstIdx = engine.get_series_chapter_first_global_idx(nextSeriesIdx);
            if (firstIdx >= 0) {
                jumpToPage(firstIdx);
            } else {
                engine.jump_to_chapter(nextSeriesIdx);
            }
        }
    }

    function prevChapter() {
        var currentSeriesIdx = engine.current_chapter_idx - 1;
        var prevSeriesIdx = currentSeriesIdx - 1;
        if (prevSeriesIdx >= 0) {
            var firstIdx = engine.get_series_chapter_first_global_idx(prevSeriesIdx);
            if (firstIdx >= 0) {
                jumpToPage(firstIdx);
            } else {
                engine.jump_to_chapter(prevSeriesIdx);
            }
        }
    }

    function jumpToChapterPage(pageNumber) {
        var globalIdx = engine.get_current_chapter_page_global_idx(pageNumber);
        if (globalIdx >= 0) {
            jumpToPage(globalIdx);
        }
    }

    function jumpToPage(pageIndex) {
        if (engine.reading_mode === 0) {
            verticalListView.positionViewAtIndex(pageIndex, ListView.Beginning);
        } else {
            horizontalListView.currentIndex = pageIndex;
        }
        engine.update_current_page(pageIndex);
        engine.request_pages_around(pageIndex);
    }

    function jumpToCurrentChapterBottom() {
        var lastIdx = engine.get_current_chapter_last_global_idx();
        if (lastIdx >= 0) {
            jumpToPage(lastIdx);
        }
    }

    function jumpToCurrentChapterTop() {
        var firstIdx = engine.get_current_chapter_first_global_idx();
        if (firstIdx >= 0) {
            jumpToPage(firstIdx);
        }
    }

    // Empty State Placeholder (when no comic is open)
    Kirigami.PlaceholderMessage {
        id: placeholder
        anchors.centerIn: parent
        visible: !engine.has_comic
        width: Math.min(parent.width - 48, 520)
        icon.source: engine.app_icon
        text: "No Comic Open"
        explanation: "Tap anywhere or press Ctrl+O to select a .cbz Manhwa or Manga archive"

        helpfulAction: Kirigami.Action {
            text: "Open .cbz Archive"
            icon.name: "document-open"
            onTriggered: readerRoot.openFileDialogRequested()
        }
    }

    // Background click for empty state
    MouseArea {
        anchors.fill: parent
        visible: !engine.has_comic
        z: -1
        onClicked: readerRoot.openFileDialogRequested()
    }

    // =========================================================================
    // VERTICAL MODE (Continuous Manhwa / Webtoon Infinite Scroll)
    // =========================================================================
    ListView {
        id: verticalListView
        anchors.fill: parent
        visible: engine.has_comic && engine.reading_mode === 0
        orientation: ListView.Vertical
        spacing: 0
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        cacheBuffer: 3000
        reuseItems: true

        model: engine.total_pages_count

        // Track scrolling to preload and update current reading position
        onContentYChanged: {
            if (count > 0 && contentHeight > 0) {
                var visibleIdx = indexAt(width / 2, contentY + Math.min(height * 0.3, 200));
                if (visibleIdx >= 0) {
                    engine.update_current_page(visibleIdx);
                    engine.request_pages_around(visibleIdx);
                }
            }
        }

        delegate: Item {
            id: pageDelegate
            width: verticalListView.width
            readonly property int pageIndex: index
            readonly property bool isChapterStart: engine.is_first_page_of_chapter(pageIndex)
            readonly property real naturalRatio: (pageImg.implicitWidth > 0) ? (pageImg.implicitHeight / pageImg.implicitWidth) : 1.75
            readonly property real contentWidth: pageDelegate.width * readerRoot.zoomFactor
            readonly property real bannerHeight: isChapterStart ? 64 : 0
            readonly property real calculatedImgHeight: pageImg.status === Image.Ready ? (contentWidth * naturalRatio) : Math.max(contentWidth * 1.5, 400)

            height: bannerHeight + calculatedImgHeight

            Column {
                anchors.horizontalCenter: parent.horizontalCenter
                width: pageDelegate.contentWidth
                spacing: 0

                // Chapter Separator Banner (Only at start of a chapter)
                Rectangle {
                    visible: pageDelegate.isChapterStart
                    width: parent.width
                    height: pageDelegate.bannerHeight
                    color: "#181c20"
                    radius: 4

                    Rectangle {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: 5
                        color: "#3daee9"
                        radius: 2
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 20
                        anchors.rightMargin: 20
                        spacing: 12

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Kirigami.Heading {
                                text: engine.get_page_chapter_title(pageDelegate.pageIndex)
                                level: 3
                                color: "#eff0f1"
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Controls.Label {
                                text: "Chapter " + engine.get_page_chapter_number(pageDelegate.pageIndex) + " • " + engine.get_page_chapter_page_count(pageDelegate.pageIndex) + " Pages"
                                color: "#8b949e"
                                font.pointSize: 9
                            }
                        }

                        Kirigami.Icon {
                            source: "bookmarks"
                            implicitWidth: 24
                            implicitHeight: 24
                            color: "#3daee9"
                        }
                    }
                }

                // Page Image Container
                Item {
                    width: parent.width
                    height: pageDelegate.calculatedImgHeight

                    Image {
                        id: pageImg
                        anchors.fill: parent
                        fillMode: Image.PreserveAspectFit
                        source: engine.get_page_source(pageDelegate.pageIndex)
                        asynchronous: true
                        cache: false

                        // Loading skeleton indicator
                        Rectangle {
                            anchors.fill: parent
                            color: "#1f2326"
                            visible: pageImg.status !== Image.Ready

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 10

                                Controls.BusyIndicator {
                                    Layout.alignment: Qt.AlignHCenter
                                    running: pageImg.status === Image.Loading
                                }

                                Controls.Label {
                                    text: "Page " + engine.get_page_chapter_page_idx(pageDelegate.pageIndex)
                                    color: "#6c757d"
                                    font.pointSize: 10
                                    Layout.alignment: Qt.AlignHCenter
                                }
                            }
                        }
                    }

                    // Tap to toggle controls, double-tap to toggle fullscreen
                    TapHandler {
                        onSingleTapped: readerRoot.toggleControls()
                        onDoubleTapped: readerRoot.toggleFullscreenRequested()
                    }
                }
            }
        }
    }

    // =========================================================================
    // HORIZONTAL MODE (Single Page Manga Mode)
    // =========================================================================
    ListView {
        id: horizontalListView
        anchors.fill: parent
        visible: engine.has_comic && engine.reading_mode === 1
        orientation: ListView.Horizontal
        snapMode: ListView.SnapOneItem
        highlightRangeMode: ListView.StrictlyEnforceRange
        spacing: 0
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        cacheBuffer: horizontalListView.width * 2

        model: engine.total_pages_count

        onCurrentIndexChanged: {
            if (currentIndex >= 0) {
                engine.update_current_page(currentIndex);
                engine.request_pages_around(currentIndex);
            }
        }

        delegate: Item {
            id: horizontalDelegate
            width: horizontalListView.width
            height: horizontalListView.height

            Image {
                id: hImg
                anchors.fill: parent
                fillMode: Image.PreserveAspectFit
                source: engine.get_page_source(index)
                asynchronous: true
                cache: false

                // Loading placeholder
                Rectangle {
                    anchors.fill: parent
                    color: "#181c20"
                    visible: hImg.status !== Image.Ready

                    Controls.BusyIndicator {
                        anchors.centerIn: parent
                        running: hImg.status === Image.Loading
                    }
                }
            }

            // Left touch zone: Previous page (single tap) / Toggle fullscreen (double tap)
            Item {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * 0.3

                TapHandler {
                    onSingleTapped: horizontalListView.decrementCurrentIndex()
                    onDoubleTapped: readerRoot.toggleFullscreenRequested()
                }
            }

            // Center touch zone: Toggle controls (single tap) / Toggle fullscreen (double tap)
            Item {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * 0.4

                TapHandler {
                    onSingleTapped: readerRoot.toggleControls()
                    onDoubleTapped: readerRoot.toggleFullscreenRequested()
                }
            }

            // Right touch zone: Next page (single tap) / Toggle fullscreen (double tap)
            Item {
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * 0.3

                TapHandler {
                    onSingleTapped: horizontalListView.incrementCurrentIndex()
                    onDoubleTapped: readerRoot.toggleFullscreenRequested()
                }
            }
        }
    }

    // =========================================================================
    // FLOATING BOTTOM PROGRESS HUD & SCRUBBER
    // =========================================================================
    Rectangle {
        id: bottomHud
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        width: Math.min(parent.width - 36, 680)
        height: 52
        radius: 26
        color: "#232629"
        border.color: "#31363b"
        border.width: 1
        visible: engine.has_comic && readerRoot.showControls
        opacity: readerRoot.showControls ? 0.95 : 0.0

        Behavior on opacity {
            NumberAnimation { duration: 200 }
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            spacing: 12

            Controls.ToolButton {
                icon.name: "go-previous"
                Controls.ToolTip.text: "Previous Chapter ([)"
                Controls.ToolTip.visible: hovered
                enabled: engine.current_chapter_idx > 1
                onClicked: readerRoot.prevChapter()
            }

            Controls.Label {
                text: "Ch. " + engine.current_chapter_idx + " (" + engine.current_page_number + "/" + Math.max(engine.current_chapter_page_count, 1) + ")"
                font.bold: true
                font.pointSize: 9
                color: "#eff0f1"
            }

            Controls.Slider {
                id: pageSlider
                Layout.fillWidth: true
                from: 1
                to: Math.max(engine.current_chapter_page_count, 1)
                stepSize: 1
                value: engine.current_page_number
                live: false
                onMoved: {
                    readerRoot.jumpToChapterPage(Math.round(value));
                }
            }

            Controls.Label {
                text: (engine.reading_mode === 0 ? "Webtoon" : "Manga")
                font.pointSize: 9
                color: "#3daee9"
            }

            Controls.ToolButton {
                icon.name: "go-next"
                Controls.ToolTip.text: "Next Chapter (])"
                Controls.ToolTip.visible: hovered
                enabled: engine.current_chapter_idx < engine.total_chapters_count
                onClicked: readerRoot.nextChapter()
            }
        }
    }
}
