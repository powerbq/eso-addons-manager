import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Theme.js" as Theme

Item {
    property var conflicts: []
    property bool loading: false

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            height: Theme.toolbarHeight
            color: Theme.bgSurface

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                spacing: 8

                Text {
                    text: loading ? "Loading..." : (conflicts.length === 0
                        ? "No libraries provided by multiple candidates"
                        : conflicts.length + (conflicts.length === 1 ? " library" : " libraries") + " provided by multiple candidates")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMd
                }

                Item { Layout.fillWidth: true }

                BusyIndicator {
                    running: loading
                    visible: running
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                }
            }
        }

        Divider {}

        ScrollView {
            id: scrollView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth

            Column {
                width: scrollView.availableWidth

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    topPadding: 40
                    text: "No libraries provided by multiple candidates"
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontLg
                    visible: !loading && conflicts.length === 0
                }

                Repeater {
                    model: conflicts

                    delegate: Column {
                        id: conflictGroup
                        width: scrollView.availableWidth

                        property var conflict: modelData
                        property string selectedUid: modelData.selected

                        Rectangle {
                            width: parent.width
                            height: 36
                            color: Theme.bgSurface

                            Text {
                                anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                                text: conflictGroup.conflict.dir
                                font.pixelSize: Theme.fontBase
                                font.bold: true
                                color: Theme.textPrimary
                            }
                        }

                        Repeater {
                            model: conflictGroup.conflict.addons

                            delegate: RadioButton {
                                width: conflictGroup.width
                                leftPadding: 28
                                checked: modelData.uid === conflictGroup.selectedUid
                                text: modelData.name + "  ·  UID " + modelData.uid
                                font.pixelSize: Theme.fontBase

                                contentItem: Text {
                                    leftPadding: parent.indicator.width + parent.spacing
                                    text: parent.text
                                    font: parent.font
                                    color: parent.checked ? Theme.textPrimary : Theme.textSecondary
                                    verticalAlignment: Text.AlignVCenter
                                }

                                onToggled: {
                                    if (checked) {
                                        conflictGroup.selectedUid = modelData.uid
                                        backend.setSelectedLibrary(conflictGroup.conflict.dir, modelData.uid)
                                    }
                                }
                            }
                        }

                        Rectangle {
                            width: parent.width
                            height: 1
                            color: Theme.separator
                        }
                    }
                }
            }
        }
    }

    Connections {
        target: backend
        function onConflictsLoading()           { loading = true }
        function onLibraryConflictsReady(data)  { conflicts = data; loading = false }
    }
}
