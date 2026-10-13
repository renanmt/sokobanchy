import QtQuick
import Quickshell
import qs.Commons
import qs.Ui

BarWidget {
    id: root
    moduleName: "renan.sokobanchy"
    implicitWidth: button.implicitWidth
    implicitHeight: button.implicitHeight

    BarIconButton {
        id: button
        anchors.fill: parent
        bar: root.bar
        text: "\uf466"
        tooltipText: "Sokobanchy — Sokoban for Omarchy"
        onPressed: Quickshell.execDetached([
            "bash",
            decodeURIComponent(Qt.resolvedUrl("scripts/launch.sh").toString().replace(/^file:\/\//, ""))
        ])
    }
}
