#!/usr/bin/python3

import os
import re
import sys
import threading

import bbcode
from PyQt6.QtCore import pyqtSignal
from PyQt6.QtCore import pyqtSlot
from PyQt6.QtCore import QObject
from PyQt6.QtCore import QUrl

from app import addon_exclusions
from app import c
from app import check_for_app_update
from app import execute
from app import favourites
from app import fetch_addon_details
from app import fetch_filelist
from app import get_exclusions
from app import get_library_conflicts
from app import get_sync_on_launch
from app import get_target_directory
from app import parse_addon_list
from app import save
from app import scan_target_directory
from app import set_exclusions
from app import set_sync_on_launch
from app import set_target_directory


class Backend(QObject):
    addonListReady = pyqtSignal('QVariantList')
    addonDetailsReady = pyqtSignal(str)
    installedAddonsChanged = pyqtSignal()
    updateStarted = pyqtSignal()
    updateFinished = pyqtSignal()
    logCleared = pyqtSignal()
    logMessage = pyqtSignal(str)
    appUpdateStatus = pyqtSignal(str)
    targetDirectoryChanged = pyqtSignal(str)
    targetDirectoryPicked = pyqtSignal(str, str)
    scanFinished = pyqtSignal(int)
    exclusionsChanged = pyqtSignal(str)
    conflictsLoading = pyqtSignal()
    libraryConflictsReady = pyqtSignal('QVariantList')

    def __init__(self, parent=None):
        super().__init__(parent)

    @pyqtSlot()
    def checkForUpdate(self):
        threading.Thread(
            target=check_for_app_update,
            args=(self.appUpdateStatus.emit,),
            daemon=True,
        ).start()

    def _refreshAddonList(self):
        self.logMessage.emit('Refreshing addon list...')
        filelist = fetch_filelist()
        self.addonListReady.emit(parse_addon_list(filelist))
        self.logMessage.emit('Refresh complete')
        return filelist

    def _publishConflicts(self, filelist):
        self.libraryConflictsReady.emit(get_library_conflicts(filelist))

    @pyqtSlot()
    def fetchAddonList(self):
        self.logCleared.emit()
        self.conflictsLoading.emit()

        def _run():
            try:
                filelist = self._refreshAddonList()
                self._publishConflicts(filelist)
            except Exception as e:
                self.logMessage.emit('Error fetching addon list: %s' % e)

        threading.Thread(target=_run, daemon=True).start()

    @pyqtSlot(str)
    def fetchAddonDetails(self, uid):
        def _run():
            try:
                obj = fetch_addon_details(uid)

                parser = bbcode.Parser(escape_html=False, drop_unrecognized=True)

                def _bb(text):
                    text = re.sub(r'\[(\w+)="([^"]+)"\]', r'[\1=\2]', text)
                    return parser.format(text)

                parts = []
                desc = (obj.get('UIDescription') or '').strip()
                if desc:
                    parts.append('<h3>Description</h3>' + _bb(desc))
                log = (obj.get('UIChangeLog') or '').strip()
                if log:
                    parts.append('<h3>Changelog</h3>' + _bb(log))

                self.addonDetailsReady.emit('<br>'.join(parts))
            except Exception as e:
                self.addonDetailsReady.emit('Failed to load details: %s' % e)

        threading.Thread(target=_run, daemon=True).start()

    @pyqtSlot(result=str)
    def getTargetDirectory(self):
        return get_target_directory()

    @pyqtSlot(str)
    def setTargetDirectory(self, path):
        if path.startswith('file://'):
            path = QUrl(path).toLocalFile()
        set_target_directory(path)
        save()
        self.targetDirectoryChanged.emit(path)

    @pyqtSlot()
    def browseTargetDirectory(self):
        from PyQt6.QtWidgets import QFileDialog

        old = get_target_directory()
        path = QFileDialog.getExistingDirectory(
            None,
            'Select AddOns folder',
            old,
        )
        if path and path != old:
            set_target_directory(path)
            save()
            self.targetDirectoryChanged.emit(path)
            self.targetDirectoryPicked.emit(old, path)

    @pyqtSlot()
    def scanTargetFolder(self):
        self.logCleared.emit()
        self.conflictsLoading.emit()

        def _run():
            try:
                filelist = self._refreshAddonList()
                self.logMessage.emit('Scanning folder...')
                added = scan_target_directory(filelist)
                self._publishConflicts(filelist)
                self.installedAddonsChanged.emit()
                self.logMessage.emit('Folder scan complete: %d addon(s) added.' % added)
                self.scanFinished.emit(added)
            except Exception as e:
                self.logMessage.emit('Folder scan failed: %s' % e)
                self.scanFinished.emit(-1)

        threading.Thread(target=_run, daemon=True).start()

    @pyqtSlot(result=bool)
    def hasTtcClient(self):
        path = os.path.join(get_target_directory(), 'TamrielTradeCentre', 'Client', 'Client.exe')
        return os.path.isfile(path)

    @pyqtSlot()
    def launchTtcClient(self):
        path = os.path.join(get_target_directory(), 'TamrielTradeCentre', 'Client', 'Client.exe')
        if sys.platform == 'win32':
            os.startfile(path)
        else:
            import subprocess

            subprocess.Popen([path], start_new_session=True)

    @pyqtSlot(result=str)
    def getExclusionsText(self):
        return '\n'.join(get_exclusions())

    @pyqtSlot(str)
    def setExclusionsText(self, text):
        patterns = [line for line in text.splitlines() if line.strip()]
        set_exclusions(patterns)
        save()

    @pyqtSlot(result=str)
    def getAddonExclusionsText(self):
        result = []
        for uid, patterns in addon_exclusions.items():
            if uid in c['AddOns']:
                result.extend(patterns)
        return '\n'.join(result)

    @pyqtSlot(result=bool)
    def getSyncOnLaunch(self):
        return get_sync_on_launch()

    @pyqtSlot(bool)
    def setSyncOnLaunch(self, value):
        set_sync_on_launch(value)
        save()

    @pyqtSlot()
    def runUpdate(self):
        self.logCleared.emit()
        self.updateStarted.emit()
        self.conflictsLoading.emit()

        def _run():
            try:
                filelist = self._refreshAddonList()
                self.logMessage.emit('Starting sync...')
                execute(log_callback=self.logMessage.emit, filelist=filelist)
                self._publishConflicts(filelist)
            except Exception as e:
                self.logMessage.emit('Update failed: %s' % e)
            finally:
                self.installedAddonsChanged.emit()
                self.updateFinished.emit()

        threading.Thread(target=_run, daemon=True).start()

    @pyqtSlot(str, str)
    def toggleFavourite(self, uid, name):
        if uid in favourites:
            c.remove_option('Favourites', uid)
        else:
            favourites[uid] = name
        save()

    @pyqtSlot(result='QVariantList')
    def getFavourites(self):
        return list(favourites.keys())

    @pyqtSlot(str, str)
    def installAddon(self, uid, name):
        c['AddOns'][uid] = name
        save()
        self.runUpdate()

    @pyqtSlot(str)
    def removeAddon(self, uid):
        c.remove_option('AddOns', uid)
        save()
        self.runUpdate()

    @pyqtSlot(result='QVariantList')
    def getInstalledAddons(self):
        result = []
        for uid, name in c['AddOns'].items():
            result.append({'uid': uid, 'name': name if name else uid})
        return result

    @pyqtSlot(str, str)
    def setSelectedLibrary(self, directory, uid):
        c['SelectedLibraries'][directory] = uid
        save()
