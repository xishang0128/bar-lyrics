import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import qs.Modules.Plugins

PluginComponent {
    id: root

    property var frame: ({
            "visible": false
        })
    property bool ready: false
    property bool restarting: false
    readonly property var arguments: [decodeURIComponent(Qt.resolvedUrl("./bin/bar-lyrics").toString().replace(/^file:\/\//, "")), "--output", "json", "--source-endpoint", pluginData.base_url ?? "http://127.0.0.1:14558", "--offset-ms", String(pluginData.offset_ms ?? 0), "--max-chars", String(pluginData.max_chars ?? 32), "--align", pluginData.alignment ?? "end", "--subtitle", pluginData.subtitle ?? "auto", "--cover-dir", (Quickshell.env("XDG_CACHE_HOME") || Quickshell.env("HOME") + "/.cache") + "/bar-lyrics/covers"]

    Component.onCompleted: {
        pluginService.setGlobalVar(pluginId, "bridge", root);
        ready = true;
        stream.running = true;
    }
    Component.onDestruction: {
        ready = false;
        stream.running = false;
        if (pluginService)
            pluginService.setGlobalVar(pluginId, "bridge", null);
    }
    onArgumentsChanged: {
        if (ready)
            restartDelay.restart();
    }

    function control(action) {
        if (!controller.running) {
            controller.command = [root.arguments[0], "--source", "splayer", "--source-endpoint", pluginData.base_url ?? "http://127.0.0.1:14558", "--control", action];
            controller.running = true;
        }
    }

    Process {
        id: controller
        onExited: (code, status) => {
            if (code === 0) return;
            const player = MprisController.activePlayer;
            const action = command[command.length - 1];
            if (action === "toggle" && player?.canTogglePlaying)
                player.togglePlaying();
            else if (action === "previous" && player?.canGoPrevious)
                player.previous();
            else if (action === "next" && player?.canGoNext)
                player.next();
        }
        stderr: SplitParser {
            onRead: data => console.warn("Bar Lyrics:", data)
        }
    }

    Timer {
        id: restartDelay

        interval: 150
        onTriggered: {
            if (stream.running) {
                root.restarting = true;
                stream.running = false;
            } else {
                stream.running = true;
            }
        }
    }

    Process {
        id: stream

        command: root.arguments
        onExited: (code, status) => {
            root.frame = {
                "visible": false
            };
            if (root.ready && root.restarting) {
                root.restarting = false;
                stream.running = true;
            } else if (code !== 0) {
                console.warn("Bar Lyrics: backend exited:", code);
            }
        }

        stdout: SplitParser {
            onRead: data => {
                try {
                    root.frame = JSON.parse(data);
                } catch (error) {
                    console.warn("Bar Lyrics: invalid frame:", error);
                }
            }
        }

        stderr: SplitParser {
            onRead: data => console.warn("Bar Lyrics:", data)
        }
    }
}
