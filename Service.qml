import QtQuick
import Quickshell
import Quickshell.Io

// Register a normal application. This service has no windows or bar widget.
Item {
    id: root
    property string helperCode: ""
    readonly property string pluginDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, ""))
    readonly property string owner: Date.now().toString() + "-" + Math.random().toString(36).slice(2)

    FileView {
        id: helper
        path: root.pluginDir + "scripts/desktop-entry.py"
        blockLoading: true
    }

    Process {
        id: registration
        command: ["python3", "-c", root.helperCode, "register", root.pluginDir, root.owner]
        stderr: SplitParser { onRead: data => console.warn("Sokobanchy: " + data) }
    }

    Component.onCompleted: {
        // Cache the helper so cleanup still works after plugin remove deletes the checkout.
        helperCode = helper.text()
        registration.running = true
    }
    Component.onDestruction: {
        if (helperCode) {
            Quickshell.execDetached(["python3", "-c", helperCode, "remove", pluginDir,
                owner, String(registration.processId || 0)])
        }
    }
}
