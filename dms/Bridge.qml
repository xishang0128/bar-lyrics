import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import qs.Modules.Plugins

PluginComponent {
    id: root

    property var frame: ({ visible: false })
    property bool ready: false
    readonly property var configuration: ({
        source: "splayer",
        source_endpoint: pluginData.base_url ?? "http://127.0.0.1:14558",
        offset_ms: pluginData.offset_ms ?? 0,
        max_chars: pluginData.max_chars ?? 32,
        alignment: pluginData.alignment ?? "end",
        subtitle: pluginData.subtitle ?? "auto",
        cover_dir: (Quickshell.env("XDG_CACHE_HOME") || Quickshell.env("HOME") + "/.cache") + "/bar-lyrics/covers"
    })

    Component.onCompleted: {
        pluginService.setGlobalVar(pluginId, "bridge", root);
        ready = true;
        stream.running = true;
    }
    Component.onDestruction: {
        ready = false;
        stream.running = false;
        if (pluginService) pluginService.setGlobalVar(pluginId, "bridge", null);
    }
    onConfigurationChanged: {
        if (rpc.address) configure();
    }

    function configure() {
        rpc.call("config.update", configuration, error => {
            if (error) console.warn("Bar Lyrics:", error.message);
        });
    }

    function fallback(action) {
        if (!ready) return;
        const player = MprisController.activePlayer;
        if (action === "toggle" && player?.canTogglePlaying)
            player.togglePlaying();
        else if (action === "previous" && player?.canGoPrevious)
            player.previous();
        else if (action === "next" && player?.canGoNext)
            player.next();
    }

    function control(action) {
        if (!rpc.address) { fallback(action); return; }
        rpc.call("player.control", { action }, error => {
            if (error) {
                console.warn("Bar Lyrics:", error.message);
                if (error.code === -32001 || error.code === -32098) fallback(action);
            }
        });
    }

    Ipc { id: rpc }

    Process {
        id: stream
        command: [decodeURIComponent(Qt.resolvedUrl("./bin/bar-lyrics").toString().replace(/^file:\/\//, ""))]
        onExited: (code, status) => {
            root.frame = { visible: false };
            rpc.address = null;
            if (code !== 0) console.warn("Bar Lyrics: backend exited:", code);
        }
        stdout: SplitParser {
            onRead: data => {
                try {
                    const message = JSON.parse(data);
                    if (message.jsonrpc !== "2.0") return;
                    if (message.method === "ready") {
                        rpc.address = message.params;
                        root.configure();
                    } else if (message.method === "frame") {
                        root.frame = message.params;
                    }
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
