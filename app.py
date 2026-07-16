#!/usr/bin/python3

import configparser
import json
import os
import re
import shutil
import sys
import threading
import zipfile

import bbcode
from PyQt6.QtCore import pyqtSignal
from PyQt6.QtCore import pyqtSlot
from PyQt6.QtCore import QObject
from PyQt6.QtCore import QUrl
from PyQt6.QtQml import QQmlApplicationEngine
from PyQt6.QtWebEngineQuick import QtWebEngineQuick
from PyQt6.QtWidgets import QApplication

from func import download
from func import md5
from rsync import Logger as RsyncLogger
from rsync import sync


class AddOn:
    def __init__(self):
        self.name = None
        self.version = None


class SortedDict(dict):
    def items(self):
        return sorted(super().items(), key=key)


class Logger:
    write = print


def get_target_directory():
    return c['General']['TargetDirectory']


def set_target_directory(path):
    c['General']['TargetDirectory'] = path


def get_sync_on_launch():
    return c.getboolean('General', 'SyncOnLaunch', fallback=False)


def set_sync_on_launch(value):
    c['General']['SyncOnLaunch'] = 'true' if value else 'false'


def get_exclusions():
    return list(c['Exclusions'].values())


def set_exclusions(patterns):
    for opt in list(c['Exclusions']):
        c.remove_option('Exclusions', opt)
    for i, pattern in enumerate(patterns):
        c['Exclusions'][str(i)] = pattern


def key(item):
    return item[1] if item[0].isnumeric() and type(item[1]) is str else item[0]


def log(status, kind, uid, message):
    Logger.write('\t%s\t%s\t%s\t%s' % (status, kind, uid, message))


def dependencies(path):
    z = zipfile.ZipFile(path)

    for name in z.namelist():
        info = z.getinfo(name)

        if info.is_dir() or (not name.lower().endswith('.txt') and not name.lower().endswith('.addon')):
            continue

        with z.open(name) as f:
            lines = f.readlines()

        for line in lines:
            text = line.decode('utf-8-sig', errors='ignore')

            if text.startswith('## Title:'):
                satisfied.add('.'.join(os.path.basename(name).split('.')[:-1]))

            if text.startswith('## DependsOn:') or text.startswith('## PCDependsOn:'):
                for directory in re.sub(r'[=<>][^ ]+', '', text).strip().split()[2:]:
                    if directory in candidates:
                        if directory not in satisfied:
                            satisfied.add(directory)

                            uids = candidates[directory]
                            if len(uids) < 2:
                                process(uids[0])

                            else:
                                installed_candidates = [uid for uid in uids if uid in addons]
                                if len(installed_candidates) == 1:
                                    process(installed_candidates[0])
                                elif len(installed_candidates) >= 2:
                                    selected = c['SelectedLibraries'].get(directory)
                                    process(selected if selected in installed_candidates else installed_candidates[0])
                                else:
                                    if (
                                        directory not in c['SelectedLibraries']
                                        or c['SelectedLibraries'][directory] not in uids
                                    ):
                                        best = min(uids, key=lambda u: uid_dirs.get(u, 0))
                                        c['SelectedLibraries'][directory] = best
                                    process(c['SelectedLibraries'][directory])

                        continue

                    if directory in satisfied:
                        continue

                    unsatisfied.add(directory)


def process(uid):
    name = database[uid].name
    version = database[uid].version

    identifier = re.sub(r'\W', '', name) + '_' + uid
    path = 'addons/' + identifier + '.zip'

    invalid = (
        not os.path.exists(path)
        or not c.has_section(uid)
        or c[uid].get('UIVersion') != version
        or c[uid].get('UIMD5') != md5(path)
    )

    kind = 'lib' if uid not in addons else '-'

    if invalid:
        try:
            obj = json.loads(download(api_prefix + '/filedetails/' + uid + '.json'))[0]
            body = download(obj['UIDownload'])
            with open(path, 'wb') as f:
                f.write(body)
        except Exception as e:
            download_errors.add(uid)
            log('err', kind, uid, '%s (Download failed: %s)' % (name, e))
            return

        if not c.has_section(uid):
            c.add_section(uid)

        c[uid]['UIVersion'] = obj['UIVersion']
        c[uid]['UIMD5'] = obj['UIMD5']

    status = 'upd' if invalid else '-'
    log(status, kind, uid, name)

    sources.add(path)

    dependencies(path)


def fetch_filelist():
    return json.loads(download(api_prefix + '/filelist.json'))


def fetch_category_list():
    if not category_cache:
        try:
            data = json.loads(download(api_prefix + '/categorylist.json'))
        except Exception:
            return category_cache
        for obj in data:
            category_cache[obj['UICATID']] = {
                'title': obj.get('UICATTitle') or '',
                'icon': obj.get('UICATICON') or '',
            }
    return category_cache


def fetch_addon_details(uid):
    return json.loads(download(api_prefix + '/filedetails/' + uid + '.json'))[0]


def parse_addon_list(filelist):
    categories = fetch_category_list()
    result = []
    for obj in filelist:
        cat = categories.get(obj.get('UICATID'), {})
        entry = {
            'uid': obj['UID'],
            'name': obj['UIName'],
            'author': obj.get('UIAuthorName') or '',
            'version': obj.get('UIVersion') or '',
            'url': obj.get('UIFileInfoURL') or '',
            'downloads': int(obj.get('UIDownloadTotal') or 0),
            'monthlyDownloads': int(obj.get('UIDownloadMonthly') or 0),
            'favorites': int(obj.get('UIFavoriteTotal') or 0),
            'date': int(obj.get('UIDate') or 0),
            'category': cat.get('title', ''),
            'catIcon': cat.get('icon', ''),
        }
        addon_meta[entry['uid']] = {'category': entry['category'], 'catIcon': entry['catIcon']}
        result.append(entry)
    return result


def get_library_conflicts(filelist):
    names = {}
    local_candidates = {}
    for obj in filelist:
        uid = obj['UID']
        names[uid] = obj['UIName']
        for directory in obj.get('UIDir') or []:
            local_candidates.setdefault(directory, []).append(uid)

    conflicts = []
    for directory, current_uid in sorted(c['SelectedLibraries'].items()):
        uids = local_candidates.get(directory, [])
        if len(uids) < 2:
            continue
        if current_uid not in uids:
            current_uid = uids[0]
        conflicts.append(
            {
                'dir': directory,
                'addons': [{'uid': uid, 'name': names.get(uid, uid)} for uid in uids],
                'selected': current_uid,
            }
        )
    return conflicts


def is_library_dir(addon_dir):
    try:
        entries = os.listdir(addon_dir)
    except OSError:
        return False

    for name in entries:
        if not name.lower().endswith('.txt') and not name.lower().endswith('.addon'):
            continue

        path = os.path.join(addon_dir, name)
        if not os.path.isfile(path):
            continue

        try:
            with open(path, encoding='utf-8-sig', errors='ignore') as f:
                for line in f:
                    text = line.strip()
                    if not text.startswith('##') or ':' not in text:
                        continue

                    field, _, value = text[2:].partition(':')
                    if field.strip().lower() == 'islibrary' and value.strip().lower() == 'true':
                        return True
        except OSError:
            continue

    return False


def scan_target_directory(filelist=None):
    if filelist is None:
        filelist = fetch_filelist()

    target = get_target_directory()
    if not os.path.isdir(target):
        return 0

    dir_to_uids = {}
    names = {}
    uid_dir_count = {}
    for obj in filelist:
        uid = obj['UID']
        names[uid] = obj['UIName']
        dirs = obj.get('UIDir') or []
        uid_dir_count[uid] = len(dirs)
        for directory in dirs:
            dir_to_uids.setdefault(directory, []).append(uid)

    added = 0
    for entry in sorted(os.listdir(target)):
        full = os.path.join(target, entry)
        if not os.path.isdir(full):
            continue

        if is_library_dir(full):
            continue

        uids = dir_to_uids.get(entry)
        if not uids:
            continue

        if len(uids) == 1:
            uid = uids[0]
        else:
            selected = c['SelectedLibraries'].get(entry)
            uid = selected if selected in uids else min(uids, key=lambda u: uid_dir_count.get(u, 0))

        if uid in addons:
            continue

        addons[uid] = names.get(uid, '')
        added += 1

    if added:
        save()

    return added


def run(filelist=None):
    if filelist is None:
        filelist = fetch_filelist()
    for obj in filelist:
        uid = obj['UID']
        name = obj['UIName']
        version = obj['UIVersion']

        database[uid] = AddOn()
        database[uid].name = name
        database[uid].version = version

        dirs = obj.get('UIDir') or []
        uid_dirs[uid] = len(dirs)
        for directory in dirs:
            candidates.setdefault(directory, []).append(uid)

    for uid in addons.keys():
        if uid in database:
            if not c.has_section(uid):
                c.add_section(uid)

            addons[uid] = database[uid].name

            process(uid)

        else:
            name = addons[uid]
            if name:
                log('err', '-', uid, '%s (Not found in database)' % name)

            else:
                log('err', '-', uid, 'Not found in database')

    for path in sorted(os.listdir('custom')):
        if path.endswith('.zip'):
            path = 'custom/' + path
            if os.path.isfile(path):
                name = path.removeprefix('custom/').removesuffix('.zip')
                log('-', '-', '-', 'Custom (%s)' % name)

                sources.add(path)

                dependencies(path)

    errors = unsatisfied - satisfied
    for directory in errors:
        log('err', 'lib', '-', 'No candidates found for %s' % directory)

    effective_exclusions = list(get_exclusions())
    for uid, patterns in addon_exclusions.items():
        if uid in addons:
            effective_exclusions.extend(patterns)

    if download_errors:
        Logger.write('Skipping sync due to %d download error(s)' % len(download_errors))
    else:
        sync(sources, get_target_directory(), exclude_patterns=effective_exclusions)
        Logger.write('Sync complete')


def delete(path):
    if os.path.isdir(path):
        shutil.rmtree(path)
    else:
        os.remove(path)


def cleanup():
    for path in os.listdir('addons'):
        if 'addons/' + path not in sources:
            delete('addons/' + path)

    for path in os.listdir('custom'):
        if not path.endswith('.zip'):
            delete('custom/' + path)

    for section in c.sections():
        if section == 'General':
            for option in list(c[section].keys()):
                if option not in {'TargetDirectory', 'SyncOnLaunch'}:
                    c.remove_option(section, option)

        elif section == 'Exclusions':
            pass  # user-defined, keep as-is

        elif section == 'AddOns':
            for option in list(c[section].keys()):
                if not option.isnumeric() or option not in database:
                    c.remove_option(section, option)

        elif section == 'Favourites':
            for option in list(c[section].keys()):
                if not option.isnumeric() or option not in database:
                    c.remove_option(section, option)

        elif section == 'SelectedLibraries':
            for option in list(c[section].keys()):
                value = c[section][option]
                uids = candidates.get(option, [])
                installed_candidates = [uid for uid in uids if uid in addons]
                if value not in database or len(uids) < 2 or len(installed_candidates) == 1 or option not in satisfied:
                    c.remove_option(section, option)

        else:
            if not any(s.endswith('_' + section + '.zip') for s in sources):
                c.remove_section(section)

            else:
                for option in list(c[section].keys()):
                    if option not in {'UIVersion', 'UIMD5'}:
                        c.remove_option(section, option)


def save():
    with open('app.ini', 'w') as f:
        c.write(f)


def execute(log_callback=None, filelist=None):
    Logger.write = log_callback or print
    RsyncLogger.write = Logger.write

    os.makedirs(get_target_directory(), exist_ok=True)

    os.makedirs('addons', exist_ok=True)
    os.makedirs('custom', exist_ok=True)

    database.clear()
    candidates.clear()
    uid_dirs.clear()

    satisfied.clear()
    unsatisfied.clear()
    sources.clear()
    download_errors.clear()

    run(filelist)
    if not download_errors:
        cleanup()
    save()

    Logger.write = print
    RsyncLogger.write = print


api_prefix = 'https://api.mmoui.com/v3/game/ESO'

harvest_map_exclusions = [
    r'^HarvestMapData/$',
    r'^HarvestMapData/Modules/$',
]

for loc in ['AD', 'DC', 'EP', 'NF', 'DLC']:
    harvest_map_exclusions.append(r'^HarvestMapData/Modules/HarvestMap%s/$' % loc)
    harvest_map_exclusions.append(r'^HarvestMapData/Modules/HarvestMap%s/HarvestMap%s\.lua$' % (loc, loc))

ttc_exclusions = [
    r'^TamrielTradeCentre/$',
    r'^TamrielTradeCentre/Client/$',
    r'^TamrielTradeCentre/Client/TTC_Lock$',
    r'^TamrielTradeCentre/PriceTableEU\.lua$',
    r'^TamrielTradeCentre/PriceTableNA\.lua$',
]

for lang in ['DE', 'EN', 'ES', 'FR', 'JP', 'RU', 'ZH']:
    ttc_exclusions.append(r'^TamrielTradeCentre/ItemLookUpTable_%s\.lua$' % lang)

addon_exclusions = {
    '1245': ttc_exclusions,
    '3034': harvest_map_exclusions,
}

c = configparser.ConfigParser(dict_type=SortedDict)
c.optionxform = str
c.add_section('General')
c.add_section('AddOns')
c.add_section('Favourites')
c.add_section('SelectedLibraries')
set_target_directory('target/AddOns')
set_sync_on_launch(False)

if os.path.exists('app.ini'):
    c.read('app.ini')

if not c.has_section('Exclusions'):
    c.add_section('Exclusions')
    save()

addons = c['AddOns']
favourites = c['Favourites']

database = {}
candidates = {}
uid_dirs = {}
category_cache = {}
addon_meta = {}

satisfied = set()
unsatisfied = set()
sources = set()
download_errors = set()


class QmlBackend(QObject):
    addonListReady = pyqtSignal('QVariantList')
    addonDetailsReady = pyqtSignal(str)
    installedAddonsChanged = pyqtSignal()
    updateStarted = pyqtSignal()
    updateFinished = pyqtSignal()
    logCleared = pyqtSignal()
    logMessage = pyqtSignal(str)
    targetDirectoryChanged = pyqtSignal(str)
    targetDirectoryPicked = pyqtSignal(str, str)
    scanFinished = pyqtSignal(int)
    exclusionsChanged = pyqtSignal(str)
    conflictsLoading = pyqtSignal()
    libraryConflictsReady = pyqtSignal('QVariantList')

    def __init__(self, parent=None):
        super().__init__(parent)

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
            meta = addon_meta.get(uid, {})
            result.append(
                {
                    'uid': uid,
                    'name': name if name else uid,
                    'category': meta.get('category', ''),
                    'catIcon': meta.get('catIcon', ''),
                }
            )
        return result

    @pyqtSlot(str, str)
    def setSelectedLibrary(self, directory, uid):
        c['SelectedLibraries'][directory] = uid
        save()


if __name__ == '__main__':
    _file_path = os.path.abspath(sys.executable if getattr(sys, 'frozen', False) else __file__)
    _file_directory = os.path.dirname(_file_path)
    _bundle_directory = getattr(sys, '_MEIPASS', _file_directory)

    os.environ.setdefault('QT_QUICK_CONTROLS_STYLE', 'Fusion')

    QtWebEngineQuick.initialize()
    qt_app = QApplication(sys.argv)
    backend = QmlBackend()
    engine = QQmlApplicationEngine()
    engine.rootContext().setContextProperty('backend', backend)
    engine.load(QUrl.fromLocalFile(os.path.join(_bundle_directory, 'qml/main.qml')))
    if not engine.rootObjects():
        sys.exit(-1)
    sys.exit(qt_app.exec())
