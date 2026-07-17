import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Theme.js" as Theme

ApplicationWindow {
    id: root
    visibility: Window.Maximized
    minimumWidth: 800
    minimumHeight: 500
    title: qsTr("ESO Addons Updater")
    color: Theme.bg

    palette.window:          Theme.bg
    palette.windowText:      Theme.textPrimary
    palette.base:            Theme.bgSurface
    palette.alternateBase:   "#2a2a2a"
    palette.text:            Theme.textPrimary
    palette.button:          "#2a2a2a"
    palette.buttonText:      Theme.textPrimary
    palette.highlight:       Theme.accent
    palette.highlightedText: Theme.accentText
    palette.placeholderText: Theme.textMuted
    palette.mid:             Theme.separator
    palette.dark:            "#333333"
    palette.light:           Theme.separator

    ColumnLayout {
        anchors.fill: parent
        spacing: 1

        Rectangle {
            Layout.fillWidth: true
            height: 40
            color: Theme.bg

            TabBar {
                id: tabBar
                anchors.fill: parent
                contentHeight: 40

                TabButton {
                    id: tabCatalogue
                    text: qsTr("Catalogue")
                    font.pixelSize: Theme.fontLg
                    implicitWidth: 140
                }
                TabButton {
                    id: tabInstalled
                    text: qsTr("Installed")
                    font.pixelSize: Theme.fontLg
                    implicitWidth: 140
                }
                TabButton {
                    id: tabLibraries
                    text: qsTr("Libraries")
                    font.pixelSize: Theme.fontLg
                    implicitWidth: 140
                }
                TabButton {
                    id: tabExclusions
                    text: qsTr("Exclusions")
                    font.pixelSize: Theme.fontLg
                    implicitWidth: 140
                }
                TabButton {
                    id: tabLog
                    text: qsTr("Log")
                    font.pixelSize: Theme.fontLg
                    implicitWidth: 140
                }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            AddonsTab {}
            InstalledTab {}
            LibrariesTab { id: librariesTab }
            ExclusionsTab {}
            LogTab {}
        }

        Rectangle {
            Layout.fillWidth: true
            height: 40
            color: Theme.bgSurface

            RowLayout {
                anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                spacing: 6

                ComboBox {
                    id: langCombo
                    implicitHeight: 28
                    Layout.preferredWidth: 100
                    Layout.rightMargin: 6
                    leftPadding: 4
                    font.pixelSize: Theme.fontMd
                    textRole: "name"
                    valueRole: "code"
                    model: backend ? backend.getLanguages() : []
                    function syncFromBackend() {
                        currentIndex = Math.max(0, indexOfValue(backend ? backend.getLanguage() : ""))
                    }
                    Component.onCompleted: syncFromBackend()
                    onModelChanged: syncFromBackend()
                    onActivated: backend.setLanguage(currentValue)
                }

                Text {
                    text: qsTr("AddOns folder:")
                    font.pixelSize: Theme.fontMd
                    color: Theme.textSecondary
                }

                TextField {
                    id: targetDirField
                    Layout.preferredWidth: Math.max(120, dirMetrics.width + leftPadding + rightPadding + 8)
                    implicitHeight: 28
                    font.pixelSize: Theme.fontMd
                    text: backend ? backend.getTargetDirectory() : ""
                    placeholderText: qsTr("Path to AddOns folder...")
                    readOnly: true

                    TextMetrics {
                        id: dirMetrics
                        font: targetDirField.font
                        text: targetDirField.text
                    }
                }

                Button {
                    text: "..."
                    implicitWidth: 28
                    implicitHeight: 28
                    font.pixelSize: Theme.fontLg
                    onClicked: backend.browseTargetDirectory()
                }

                Button {
                    text: qsTr("Scan for existing addons")
                    implicitHeight: 28
                    font.pixelSize: Theme.fontMd
                    enabled: !anyBusy
                    onClicked: scanDialog.openFor("", false)
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: qsTr("Launch TTC Client")
                    implicitHeight: Theme.buttonHeight
                    visible: ttcClientVisible
                    onClicked: backend.launchTtcClient()
                }

                Item { Layout.fillWidth: true }

                CheckBox {
                    text: qsTr("Sync on launch")
                    font.pixelSize: Theme.fontBase
                    checked: backend ? backend.getSyncOnLaunch() : false
                    onCheckedChanged: if (backend) backend.setSyncOnLaunch(checked)
                }

                BusyIndicator {
                    running: anyBusy
                    visible: anyBusy
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28
                }

                Button {
                    text: qsTr("Refresh List")
                    implicitHeight: Theme.buttonHeight
                    visible: !anyBusy
                    onClicked: { listLoading = true; backend.fetchAddonList() }
                }

                Button {
                    text: qsTr("Sync")
                    implicitHeight: Theme.buttonHeight
                    visible: !anyBusy
                    Layout.leftMargin: 4
                    onClicked: { goTo(tabLog); backend.runUpdate() }
                }
            }
        }
    }

    Dialog {
        id: scanDialog
        anchors.centerIn: parent
        modal: true
        width: 480
        padding: 20
        closePolicy: Popup.NoAutoClose

        property string previousDir: ""
        property bool allowRevert: false

        function openFor(prevDir, revert) {
            previousDir = prevDir
            allowRevert = revert
            open()
        }

        background: Rectangle {
            color: Theme.bgSurface
            border.color: Theme.separator
            radius: 4
        }

        contentItem: ColumnLayout {
            spacing: 12

            Text {
                text: qsTr("Scan folder for addons?")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontLg
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBase
                text: qsTr("Do you want to analyse this folder for installed addons and add them to your list?")
                    + "\n\n" + qsTr("Warning: detection may be inaccurate. Back up the folder first — its contents can be overwritten on the next sync.")
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 8
                spacing: 8

                Item { Layout.fillWidth: true }

                Button {
                    text: qsTr("No, don't change folder")
                    visible: scanDialog.allowRevert
                    onClicked: {
                        if (scanDialog.previousDir !== "")
                            backend.setTargetDirectory(scanDialog.previousDir)
                        scanDialog.close()
                    }
                }

                Button {
                    text: qsTr("No, don't scan")
                    onClicked: scanDialog.close()
                }

                Button {
                    text: qsTr("Yes, scan")
                    onClicked: {
                        scanDialog.close()
                        scanRunning = true
                        backend.scanTargetFolder()
                    }
                }
            }
        }
    }

    Dialog {
        id: scanResultDialog
        anchors.centerIn: parent
        modal: true
        width: 420
        padding: 20
        closePolicy: Popup.NoAutoClose

        property string message: ""

        function show(text) {
            message = text
            open()
        }

        background: Rectangle {
            color: Theme.bgSurface
            border.color: Theme.separator
            radius: 4
        }

        contentItem: ColumnLayout {
            spacing: 12

            Text {
                text: qsTr("Scan complete")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontLg
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBase
                text: scanResultDialog.message
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 8

                Item { Layout.fillWidth: true }

                Button {
                    text: qsTr("OK")
                    onClicked: scanResultDialog.close()
                }
            }
        }
    }

    property bool listLoading: false
    property bool syncRunning: false
    property bool scanRunning: false
    property bool anyBusy: syncRunning || listLoading || scanRunning

    Connections {
        target: backend
        function onUpdateStarted()           { syncRunning = true }
        function onUpdateFinished()          { syncRunning = false; ttcClientVisible = backend.hasTtcClient() }
        function onLibraryConflictsReady()   { listLoading = false }
        function onScanFinished(count) {
            scanRunning = false
            if (count < 0)
                scanResultDialog.show(qsTr("The folder scan failed. See the Log tab for details."))
            else if (count === 0)
                scanResultDialog.show(qsTr("No new addons were found in the folder."))
            else
                scanResultDialog.show(qsTr("%n addon(s) were found and added to your list.", "", count))
        }
        function onTargetDirectoryChanged(path) { targetDirField.text = path }
        function onTargetDirectoryPicked(oldPath, newPath) { scanDialog.openFor(oldPath, true) }
    }

    property bool ttcClientVisible: backend ? backend.hasTtcClient() : false

    function goTo(tab) {
        for (var i = 0; i < tabBar.count; i++)
            if (tabBar.itemAt(i) === tab) { tabBar.currentIndex = i; return }
    }

    Component.onCompleted: {
        goTo(tabInstalled)
        if (backend.getSyncOnLaunch()) {
            goTo(tabLog)
            backend.runUpdate()
        } else {
            listLoading = true
            backend.fetchAddonList()
        }
    }
}
