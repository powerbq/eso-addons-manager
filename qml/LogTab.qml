import QtQuick
import QtQuick.Controls
import "Theme.js" as Theme

Item {
    ScrollView {
        anchors.fill: parent

        TextArea {
            id: logArea
            readOnly: true
            wrapMode: TextArea.Wrap
            font.family: "monospace"
            font.pixelSize: Theme.fontBase
            color: Theme.textLog
            background: Rectangle { color: Theme.bg }
            padding: 12
            placeholderText: qsTr("Log is empty...")
        }
    }

    Connections {
        target: backend
        function onLogCleared()    { logArea.clear() }
        function onLogMessage(msg) { logArea.append(msg) }
    }
}
