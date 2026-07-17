import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Theme.js" as Theme

Item {
    property string pendingUID: ""
    property bool syncing: false

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            height: Theme.toolbarHeight
            color: Theme.bgSurface

            Text {
                anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                text: qsTr("%n addon(s) installed", "", installedModel.count)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMd
            }
        }

        Divider {}

        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: ListModel { id: installedModel }

            ScrollBar.vertical: ScrollBar {}

            Text {
                anchors.centerIn: parent
                text: qsTr("No addons installed")
                color: Theme.textMuted
                font.pixelSize: Theme.fontLg
                visible: installedModel.count === 0
            }

            delegate: Item {
                width: listView.width
                height: 52

                Image {
                    id: catIcon
                    anchors {
                        left: parent.left; leftMargin: 12
                        verticalCenter: parent.verticalCenter
                    }
                    width: 40
                    height: 40
                    fillMode: Image.PreserveAspectFit
                    asynchronous: true
                    cache: true
                    source: model.catIcon || ""
                    visible: model.catIcon

                    ToolTip.visible: catIconArea.containsMouse && model.category
                    ToolTip.text: model.category || ""

                    MouseArea {
                        id: catIconArea
                        anchors.fill: parent
                        hoverEnabled: true
                    }
                }

                Column {
                    anchors {
                        left: catIcon.visible ? catIcon.right : parent.left
                        leftMargin: catIcon.visible ? 10 : 14
                        right: deleteArea.left; rightMargin: 8
                        verticalCenter: parent.verticalCenter
                    }
                    spacing: 2

                    Text {
                        text: model.name
                        font.pixelSize: Theme.fontBase
                        font.bold: true
                        elide: Text.ElideRight
                        width: parent.width
                        color: Theme.textPrimary
                    }
                    Text {
                        text: qsTr("UID: %1").arg(model.uid)
                        font.pixelSize: Theme.fontSm
                        color: Theme.textSecondary
                    }
                }

                Item {
                    id: deleteArea
                    anchors {
                        right: parent.right; rightMargin: 10
                        verticalCenter: parent.verticalCenter
                    }
                    width: 80
                    height: Theme.buttonHeight

                    BusyIndicator {
                        anchors.centerIn: parent
                        width: 28; height: 28
                        running: pendingUID === model.uid
                        visible: running
                    }

                    Button {
                        anchors.fill: parent
                        visible: pendingUID !== model.uid
                        enabled: !syncing
                        text: qsTr("Remove")
                        onClicked: {
                            pendingUID = model.uid
                            backend.removeAddon(model.uid)
                        }
                    }
                }

                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 1
                    color: Theme.separator
                }
            }
        }
    }

    Connections {
        target: backend
        function onInstalledAddonsChanged() { reload() }
        function onAddonListReady(addons)   { reload() }
        function onUpdateStarted()          { syncing = true }
        function onUpdateFinished()         { pendingUID = ""; syncing = false }
    }

    function reload() {
        installedModel.clear()
        const addons = backend.getInstalledAddons()
        addons.sort((a, b) => a.name.localeCompare(b.name))
        for (const addon of addons) installedModel.append(addon)
    }

    Component.onCompleted: reload()
}
