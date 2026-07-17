import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Theme.js" as Theme

Item {
    property string savedText: backend ? backend.getExclusionsText() : ""

    ColumnLayout {
        anchors { left: parent.left; top: parent.top; bottom: parent.bottom; right: columnDivider.left }
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            height: Theme.toolbarHeight
            color: Theme.bgSurface

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }

                Text {
                    text: qsTr("Custom exclusions — one regex per line")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMd
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: qsTr("Save")
                    implicitHeight: Theme.buttonHeight
                    enabled: textArea.text !== savedText
                    opacity: enabled ? 1.0 : Theme.disabledOpacity
                    onClicked: {
                        backend.setExclusionsText(textArea.text)
                        savedText = textArea.text
                    }
                }

                Button {
                    text: qsTr("Clear")
                    implicitHeight: Theme.buttonHeight
                    onClicked: textArea.text = ""
                }
            }
        }

        Divider {}

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth

            TextArea {
                id: textArea
                text: savedText
                font.family: "monospace"
                font.pixelSize: Theme.fontBase
                color: Theme.inputText
                background: Rectangle {
                    color: Theme.inputBg
                    border.color: textArea.activeFocus ? Theme.accent : Theme.inputBorder
                    border.width: 1
                }
            }
        }
    }

    Rectangle {
        id: columnDivider
        width: 1
        anchors { top: parent.top; bottom: parent.bottom; horizontalCenter: parent.horizontalCenter }
        color: Theme.separator
    }

    ColumnLayout {
        anchors { left: columnDivider.right; top: parent.top; bottom: parent.bottom; right: parent.right }
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            height: Theme.toolbarHeight
            color: Theme.bgSurface

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }

                Text {
                    text: qsTr("Automatic (from installed addons)")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMd
                }
            }
        }

        Divider {}

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth

            TextArea {
                id: addonExclusionsArea
                text: backend ? backend.getAddonExclusionsText() : ""
                readOnly: true
                font.family: "monospace"
                font.pixelSize: Theme.fontBase
                color: Theme.textPrimary
                background: Rectangle {
                    color: Theme.bg
                }
            }
        }
    }

    Connections {
        target: backend
        function onExclusionsChanged(text) {
            textArea.text = text
            savedText = text
        }
        function onInstalledAddonsChanged() {
            addonExclusionsArea.text = backend.getAddonExclusionsText()
        }
    }
}
