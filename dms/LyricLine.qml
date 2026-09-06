import QtQuick
import qs.Common
import qs.Widgets

Item {
    id: root

    property var line: null
    property int mainSize: 15
    property int subtitleSize: 9
    property real inactiveOpacity: 0.45
    property bool alignEnd: true
    readonly property string subtitle: line?.subtitle ?? ""
    readonly property var segments: line?.segments ?? []
    readonly property string mainText: segments.map(segment => segment.text).join("")

    implicitWidth: Math.max(mainMetrics.advanceWidth, secondary.implicitWidth)
    implicitHeight: mainSize + 3 + (subtitle ? subtitleSize : 0)
    width: implicitWidth
    height: implicitHeight

    TextMetrics {
        id: mainMetrics
        text: root.mainText
        font: base.font
    }

    Item {
        id: words
        width: mainMetrics.advanceWidth
        x: root.alignEnd ? root.width - width : 0
        height: root.mainSize + 3

        StyledText {
            id: base
            text: root.mainText
            textFormat: Text.PlainText
            wrapMode: Text.NoWrap
            maximumLineCount: 1
            font.pixelSize: root.mainSize
            font.weight: Font.Bold
            color: Theme.surfaceText
            opacity: root.inactiveOpacity
        }

        Repeater {
            model: root.segments.length
            delegate: Item {
                id: glyph
                required property int index
                readonly property var segment: root.segments[index] ?? { text: "", progress: 0 }
                readonly property string prefix: root.segments.slice(0, index).map(segment => segment.text).join("")
                readonly property real progress: Math.max(0, Math.min(1, segment.progress ?? (segment.active ? 1 : 0)))
                x: before.advanceWidth
                width: Math.max(0, after.advanceWidth - before.advanceWidth) * progress
                height: words.height
                clip: true

                TextMetrics {
                    id: before
                    text: glyph.prefix
                    font: base.font
                }
                TextMetrics {
                    id: after
                    text: glyph.prefix + glyph.segment.text
                    font: base.font
                }
                StyledText {
                    x: -glyph.x
                    text: base.text
                    textFormat: Text.PlainText
                    wrapMode: Text.NoWrap
                    maximumLineCount: 1
                    font: base.font
                    color: Theme.surfaceText
                }
            }
        }
    }

    StyledText {
        id: secondary
        x: root.alignEnd ? root.width - implicitWidth : 0
        y: root.mainSize + 2
        text: root.subtitle
        textFormat: Text.PlainText
        wrapMode: Text.NoWrap
        maximumLineCount: 1
        font.pixelSize: root.subtitleSize
        color: Theme.surfaceVariantText
        opacity: root.inactiveOpacity
        visible: text !== ""
    }
}
