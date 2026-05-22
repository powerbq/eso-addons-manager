import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import "Theme.js" as Theme

SplitView {
    id: root
    orientation: Qt.Horizontal

    Item {
        SplitView.preferredWidth: Math.round(root.width * 0.30)
        SplitView.minimumWidth: 220

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            TextField {
                id: searchField
                Layout.fillWidth: true
                Layout.margins: 8
                implicitHeight: 36
                font.pixelSize: 15
                placeholderText: "Search by name or author..."
                leftPadding: 12
                onTextChanged: filterList(text)
                color: Theme.inputText
                background: Rectangle {
                    color: Theme.inputBg
                    border.color: searchField.activeFocus ? Theme.accent : Theme.inputBorder
                    border.width: 1
                }
            }

            Rectangle {
                id: sortBar
                Layout.fillWidth: true
                height: Theme.barHeight
                color: Theme.bgSurface

                property int sortIndex: 3

                RowLayout {
                    anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                    spacing: 4

                    Text {
                        text: "Order by:"
                        font.pixelSize: Theme.fontSm
                        color: Theme.textMuted
                    }

                    Repeater {
                        model: ["Name", "Author", "Total DL", "Trending", "Favourites", "Updated"]
                        delegate: ChipButton {
                            Layout.fillWidth: true
                            text: modelData
                            active: sortBar.sortIndex === index
                            onClicked: {
                                sortBar.sortIndex = index
                                filterList(searchField.text)
                            }
                        }
                    }
                }
            }

            Rectangle {
                id: filterBar
                Layout.fillWidth: true
                height: Theme.barHeight
                color: Theme.bgSurface

                property bool filterInstalled: false
                property bool filterFavourites: false

                RowLayout {
                    anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                    spacing: 4

                    Text {
                        text: "Filter:"
                        font.pixelSize: Theme.fontSm
                        color: Theme.textMuted
                    }

                    ChipButton {
                        Layout.fillWidth: true
                        text: "Installed"
                        active: filterBar.filterInstalled
                        onClicked: {
                            filterBar.filterInstalled = !filterBar.filterInstalled
                            filterList(searchField.text)
                        }
                    }

                    ChipButton {
                        Layout.fillWidth: true
                        text: "My Favourites"
                        active: filterBar.filterFavourites
                        onClicked: {
                            filterBar.filterFavourites = !filterBar.filterFavourites
                            filterList(searchField.text)
                        }
                    }
                }
            }

            Divider {}

            ListView {
                id: listView
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: ListModel { id: addonModel }

                ScrollBar.vertical: ScrollBar {}

                BusyIndicator {
                    anchors.centerIn: parent
                    running: addonModel.count === 0 && allAddons.length === 0
                }

                delegate: Item {
                    width: listView.width
                    height: 76

                    Rectangle {
                        anchors.fill: parent
                        color: root.selectedUID === model.uid ? Theme.bgSelected : "transparent"
                    }

                    Text {
                        id: starBtn
                        anchors {
                            left: parent.left; leftMargin: 10
                            verticalCenter: parent.verticalCenter
                        }
                        text: root.favouriteUIDs[model.uid] ? "★" : "☆"
                        font.pixelSize: 24
                        color: root.favouriteUIDs[model.uid] ? Theme.starActive : Theme.starInactive

                        MouseArea {
                            anchors.fill: parent
                            onClicked: {
                                backend.toggleFavourite(model.uid, model.name)
                                root.refreshFavouriteUIDs()
                            }
                        }
                    }

                    Column {
                        anchors {
                            left: starBtn.right; leftMargin: 8
                            right: actionBtn.left; rightMargin: 8
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
                            font.pixelSize: Theme.fontSm
                            color: Theme.textSecondary
                            elide: Text.ElideRight
                            width: parent.width
                            text: {
                                var stats = root.fmtNum(model.downloads) + " downloads"
                                    + " (" + root.fmtNum(model.monthlyDownloads) + " this month)"
                                    + ", " + root.fmtNum(model.favorites) + " in favorites"
                                return model.author ? "By " + model.author + "  ·  " + stats : stats
                            }
                        }
                        Text {
                            font.pixelSize: Theme.fontSm
                            color: Theme.textSecondary
                            text: "Updated " + Qt.formatDate(new Date(model.date), "d MMM yyyy")
                        }
                    }

                    Item {
                        id: actionBtn
                        anchors {
                            right: parent.right; rightMargin: 10
                            verticalCenter: parent.verticalCenter
                        }
                        width: 100
                        height: Theme.buttonHeight

                        BusyIndicator {
                            anchors.centerIn: parent
                            width: 28; height: 28
                            running: root.pendingUID === model.uid
                            visible: running
                        }

                        Button {
                            anchors.fill: parent
                            visible: root.pendingUID !== model.uid
                            enabled: !root.syncing
                            text: root.installedUIDs[model.uid] ? "Remove" : "Install"
                            onClicked: {
                                root.pendingUID = model.uid
                                if (root.installedUIDs[model.uid])
                                    backend.removeAddon(model.uid)
                                else
                                    backend.installAddon(model.uid, model.name)
                            }
                        }
                    }

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.separator
                    }

                    MouseArea {
                        anchors.fill: parent
                        z: -1
                        onClicked: {
                            root.selectedUID = model.uid
                            detailText.text = ""
                            root.detailLoading = true
                            root.currentInfoURL = model.url
                            root.currentAddonName = model.name
                            backend.fetchAddonDetails(model.uid)
                        }
                    }
                }
            }
        }
    }

    Item {
        SplitView.fillWidth: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            Rectangle {
                id: viewToggle
                Layout.fillWidth: true
                height: Theme.barHeight
                color: Theme.bgSurface
                visible: root.currentInfoURL !== ""

                property int viewIndex: 0

                RowLayout {
                    anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                    spacing: 4

                    Repeater {
                        model: ["ESO UI Site", "Description"]
                        delegate: ChipButton {
                            Layout.fillWidth: true
                            text: modelData
                            active: viewToggle.viewIndex === index
                            onClicked: viewToggle.viewIndex = index
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Text {
                    anchors.centerIn: parent
                    text: "Select an addon to see details"
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontLg
                    visible: root.currentInfoURL === ""
                }

                Rectangle {
                    id: webBanner
                    anchors { top: parent.top; left: parent.left; right: parent.right }
                    height: visible ? 28 : 0
                    visible: root.currentInfoURL !== "" && viewToggle.viewIndex === 0
                    color: Theme.bgSurface

                    Text {
                        anchors.centerIn: parent
                        text: "What you see below is just a web view of esoui.com, provided for convenient browsing of addon descriptions and screenshots."
                        color: Theme.textMuted
                        font.pixelSize: 11
                        elide: Text.ElideRight
                        width: parent.width - 16
                        horizontalAlignment: Text.AlignHCenter
                    }
                }

                WebEngineView {
                    id: webView
                    anchors { top: webBanner.bottom; left: parent.left; right: parent.right; bottom: parent.bottom }
                    visible: root.currentInfoURL !== "" && viewToggle.viewIndex === 0
                    backgroundColor: Theme.bg
                    url: visible ? root.currentInfoURL : ""

                    onNavigationRequested: function(request) {
                        const blocked = [
                            WebEngineNavigationRequest.LinkClickedNavigation,
                            WebEngineNavigationRequest.FormSubmittedNavigation,
                            WebEngineNavigationRequest.BackForwardNavigation,
                        ]
                        if (blocked.includes(request.navigationType))
                            request.reject()
                    }
                }

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0
                    visible: root.currentInfoURL !== "" && viewToggle.viewIndex === 1

                    Text {
                        Layout.fillWidth: true
                        text: root.currentAddonName
                        font.pixelSize: 18
                        font.bold: true
                        color: Theme.textPrimary
                        padding: 14
                        bottomPadding: 8
                        elide: Text.ElideRight
                    }

                    Divider {}

                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        BusyIndicator {
                            anchors.centerIn: parent
                            running: detailLoading
                        }

                        ScrollView {
                            anchors.fill: parent
                            visible: !detailLoading && detailText.text !== ""
                            contentWidth: availableWidth

                            Text {
                                id: detailText
                                width: parent.width
                                textFormat: Text.RichText
                                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                                font.family: "sans-serif"
                                font.pixelSize: Theme.fontBase
                                color: Theme.textPrimary
                                padding: 14
                            }
                        }
                    }
                }
            }
        }
    }

    property var allAddons: []
    property var installedUIDs: ({})
    property var favouriteUIDs: ({})
    property string pendingUID: ""
    property string selectedUID: ""
    property bool syncing: false
    property bool detailLoading: false
    property string currentInfoURL: ""
    property string currentAddonName: ""

    function fmtNum(n) {
        if (n >= 1000000) return (n / 1000000).toFixed(1).replace(/\.0$/, '') + 'M'
        if (n >= 1000)    return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'K'
        return String(n)
    }

    function refreshInstalledUIDs() {
        const addons = backend.getInstalledAddons()
        const set = {}
        for (const a of addons) set[a.uid] = true
        installedUIDs = set
    }

    function refreshFavouriteUIDs() {
        const set = {}
        for (const uid of backend.getFavourites()) set[uid] = true
        favouriteUIDs = set
    }

    function filterList(query, preserveScroll) {
        const firstIdx = preserveScroll ? Math.max(0, listView.indexAt(1, listView.contentY + 1)) : -1
        addonModel.clear()
        const q = query.toLowerCase()
        const filtered = allAddons.filter(a => {
            if (q && !a.name.toLowerCase().includes(q) && !a.author.toLowerCase().includes(q))
                return false
            if (filterBar.filterInstalled || filterBar.filterFavourites) {
                if (filterBar.filterInstalled && installedUIDs[a.uid])   return true
                if (filterBar.filterFavourites && favouriteUIDs[a.uid])  return true
                return false
            }
            return true
        })
        const key = ['name', 'author', 'downloads', 'monthlyDownloads', 'favorites', 'date'][sortBar.sortIndex]
        filtered.sort((a, b) => typeof a[key] === 'string' ? a[key].localeCompare(b[key]) : b[key] - a[key])
        for (const addon of filtered) addonModel.append(addon)
        if (firstIdx > 0)
            Qt.callLater(() => listView.positionViewAtIndex(Math.min(firstIdx, addonModel.count - 1), ListView.Beginning))
    }

    Connections {
        target: backend
        function onAddonListReady(addons) {
            root.allAddons = addons
            root.filterList(searchField.text, true)
        }
        function onUpdateStarted()  { root.syncing = true }
        function onUpdateFinished() { root.syncing = false; root.pendingUID = "" }
        function onInstalledAddonsChanged() { root.refreshInstalledUIDs(); root.filterList(searchField.text) }
        function onAddonDetailsReady(text) { detailText.text = text; root.detailLoading = false }
    }

    Component.onCompleted: {
        root.refreshInstalledUIDs()
        root.refreshFavouriteUIDs()
    }
}
