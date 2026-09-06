import QtQuick
import qs.Common
import qs.Services
import qs.Widgets
import qs.Modules.Plugins

PluginComponent {
    id: root

    readonly property var bridge: pluginService?.getGlobalVar("barLyrics", "bridge", null) ?? null
    readonly property var frame: bridge?.frame ?? ({
            visible: false
        })
    readonly property bool showCover: pluginData.show_cover !== false && !!frame.cover_path
    readonly property bool coverLeft: pluginData.cover_position === "left"
    readonly property int singleSize: pluginData.single_line_font_size ?? 15
    readonly property int mainSize: pluginData.multiline_main_font_size ?? 13
    readonly property int subtitleSize: pluginData.multiline_subtitle_font_size ?? 9
    readonly property real inactiveOpacity: (pluginData.inactive_opacity ?? 45) / 100
    readonly property bool shown: frame.visible === true && (frame.playing || pluginData.show_when_paused !== false)
    property bool syncingLayout: false
    property real scrollDelta: 0

    function scrollTrack(event) {
        const delta = event.angleDelta.y;
        if (!delta)
            return;
        event.accepted = true;
        if (scrollDelta * delta < 0)
            scrollDelta = 0;
        scrollDelta += delta;
        scrollReset.restart();
        if (Math.abs(scrollDelta) < 120)
            return;
        const player = MprisController.activePlayer;
        if (scrollDelta > 0 && player?.canGoPrevious)
            player.previous();
        else if (scrollDelta < 0 && player?.canGoNext)
            player.next();
        scrollDelta %= 120;
    }

    Timer {
        id: scrollReset
        interval: 250
        onTriggered: root.scrollDelta = 0
    }

    MouseArea {
        anchors.fill: parent
        z: 1
        acceptedButtons: Qt.NoButton
        onWheel: event => root.scrollTrack(event)
    }

    function syncBarLayout() {
        if (syncingLayout || section !== "center")
            return;
        for (let item = parent; item; item = item.parent) {
            if (item.centerWidgets === undefined || typeof item.updateLayout !== "function")
                continue;
            syncingLayout = true;
            try {
                item.updateLayout();
            } finally {
                syncingLayout = false;
            }
            return;
        }
    }

    onWidthChanged: syncBarLayout()
    onShownChanged: setVisibilityOverride(shown)
    Component.onCompleted: setVisibilityOverride(shown)

    pillClickAction: (x, y, width, section, screen) => PopoutService.toggleDankDash("media", x, y, width, section, screen)

    pillRightClickAction: () => {
        const player = MprisController.activePlayer;
        if (player?.canTogglePlaying)
            player.togglePlaying();
    }

    horizontalBarPill: Component {
        Item {
            id: content
            readonly property real coverSize: root.showCover ? Math.min(root.widgetThickness, 24) : 0
            readonly property real gap: root.showCover ? 4 : 0
            readonly property real factor: root.frame.transition?.outgoing_height_factor ?? 0
            readonly property real textWidth: root.frame.line ? incoming.implicitWidth * (1 - factor) + outgoing.implicitWidth * factor : fallback.implicitWidth
            implicitWidth: Math.ceil(Math.min(root.pluginData.max_width ?? 360, textWidth) + coverSize + gap)
            implicitHeight: root.widgetThickness

            Image {
                anchors.verticalCenter: parent.verticalCenter
                x: root.coverLeft ? 0 : parent.width - width
                width: content.coverSize
                height: width
                source: root.showCover ? "file://" + root.frame.cover_path : ""
                sourceSize.width: 48
                sourceSize.height: 48
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                visible: root.showCover
            }

            Item {
                id: viewport
                x: root.coverLeft ? content.coverSize + content.gap : 0
                width: parent.width - content.coverSize - content.gap
                height: parent.height
                clip: true

                component DisplayLine: LyricLine {
                    property real scrollOffset: 0
                    mainSize: line?.subtitle ? root.mainSize : root.singleSize
                    subtitleSize: root.subtitleSize
                    inactiveOpacity: root.inactiveOpacity
                    alignEnd: root.frame.alignment === "end"
                    x: alignEnd ? viewport.width - width : 0
                    y: (viewport.height - height) / 2 - 1 + scrollOffset * viewport.height
                    visible: !!line
                }

                DisplayLine {
                    id: incoming
                    line: root.frame.line ?? null
                    scrollOffset: content.factor
                    opacity: root.frame.transition?.incoming_opacity ?? 1
                }
                DisplayLine {
                    id: outgoing
                    line: root.frame.transition?.outgoing ?? null
                    scrollOffset: content.factor - 1
                    opacity: root.frame.transition?.outgoing_opacity ?? 0
                }
                StyledText {
                    id: fallback
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: -1
                    width: parent.width
                    text: root.frame.fallback_text ?? ""
                    textFormat: Text.PlainText
                    wrapMode: Text.NoWrap
                    maximumLineCount: 1
                    font.pixelSize: root.singleSize
                    font.weight: Font.Bold
                    color: Theme.surfaceText
                    elide: Text.ElideRight
                    visible: !root.frame.line
                }
            }
        }
    }
}
