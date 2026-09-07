import QtQuick

QtObject {
    id: root
    property var address: null
    property var queue: []
    property bool busy: false
    property int serial: 0

    function call(method, params, callback) {
        queue.push({ method, params, callback });
        pump();
    }

    onAddressChanged: {
        if (address) {
            pump();
        } else {
            const pending = queue;
            queue = [];
            for (const request of pending) {
                if (request.callback) request.callback({ code: -32098, message: "IPC disconnected" });
            }
        }
    }

    function pump() {
        if (busy || !address || queue.length === 0) return;
        busy = true;
        const request = queue.shift();
        const id = ++serial;
        const xhr = new XMLHttpRequest();
        xhr.open("POST", address.url);
        xhr.setRequestHeader("Content-Type", "application/json");
        xhr.setRequestHeader("Authorization", "Bearer " + address.token);
        xhr.onreadystatechange = () => {
            if (xhr.readyState !== XMLHttpRequest.DONE) return;
            let error = { code: -32098, message: "IPC request failed" };
            try {
                const reply = JSON.parse(xhr.responseText);
                if (xhr.status === 200 && reply.jsonrpc === "2.0" && reply.id === id)
                    error = reply.error ?? null;
            } catch (_) {}
            busy = false;
            if (request.callback) request.callback(error);
            pump();
        };
        xhr.send(JSON.stringify({ jsonrpc: "2.0", id, method: request.method, params: request.params }));
    }
}
