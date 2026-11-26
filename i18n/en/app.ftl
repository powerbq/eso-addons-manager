tab-catalogue = Catalogue
tab-libraries = Libraries
tab-exclusions = Exclusions
tab-log = Log

menu-title = Menu
addons-folder = AddOns folder
addons-folder-none = Not selected
no-folder-title = AddOns folder not selected
change-folder-title = Select a new AddOns folder
no-folder-question = The game looks for add-ons in the AddOns folder, usually at { $example }. Launch the game at least once so it creates these folders.
no-folder-warning = Pick the folder named exactly AddOns — the manager removes from it any files it does not recognise.
no-folder-choose = Choose folder
menu-language = Language
menu-choose-folder = Change
menu-select-folder = Choose
scan-folder = Scan
menu-open-folder = Open
menu-app-folders = App folders
menu-configuration = Configuration
menu-cache = App cache
menu-custom = Custom add-ons
log-to-file = Write to file
log-open-folder = Open folder
log-clear = Delete logs
log-clear-title = Delete log files?
log-clear-all-disable = All and stop writing
log-clear-all = All log files
log-clear-others = Previous runs
sync-on-launch = Sync on launch
sync = Sync

loading = Loading…
loading-catalogue = Loading catalogue…
catalogue-failed = Cannot load the catalogue: { $error }
retry = Retry

search-placeholder = Search by name, author or folder…
addon-count =
    { $count ->
        [one] { $count } addon
       *[other] { $count } addons
    }
filter = Filter
filter-installed = Installed
filter-dependencies = Dependencies
filter-favourites = Favourites
sort-by = Sort by
sort-name = Name
sort-author = Author
sort-updated = Last updated
sort-downloads = Total downloads
sort-monthly-downloads = Monthly downloads
sort-favourites = Favourites
sort-status = Install status

addon-version = version { $version }
add-favourite = Add to favourites
remove-favourite = Remove from favourites
install = Install
install-directly = Installed automatically — click to install directly
required-by = Required by:
remove = Remove
open-page = Open on esoui.com
select-addon = Select an addon to see its description
description-failed = Cannot load the description: { $error }
description-render-failed = Cannot display the description
tab-description = Description
tab-changelog = Changelog
screenshot-failed = Cannot load the screenshot
previous = PREV
next = NEXT

libraries-none = No saved choices yet. When several addons provide the same folder, your choice appears here.
reset = Reset
recommended = Recommended

exclusions-custom = Custom exclusions — one regex per line
exclusions-automatic = Automatic (from installed addons)
exclusions-automatic-none = None: no installed addon needs them.
clear = Clear
save = Save

ok = OK
cancel = Cancel
continue = Continue
open-link-question = Open this link in your browser?
open-link-confirm = Open
providers-title = Choose addons
providers-note = Several addons provide these folders. Choose which one to use for each folder.
candidate-folders = Folders: { $folders }
scan-prompt-title = Scan folder for addons?
scan-prompt-question = Do you want to analyse this folder for installed addons and add them to your list?
scan-prompt-warning = Warning: detection may be inaccurate. Back up the folder first — its contents can be overwritten on the next sync.
scan-keep-folder = No, don't change folder
scan-skip = No, don't scan
scan-confirm = Yes, scan
scan-complete = Scan complete
scan-failed = Scan failed
scan-added = Added addons
scan-unrecognised = Unrecognised folders
scan-nothing-new = No new addons were found in the folder.
catalogue-unavailable = The catalogue is not available.
catalogue-not-loaded = The catalogue is not loaded yet.

sync-complete = Sync complete
sync-failed = Sync failed
report-problems = Problems
sync-installed-addons = Installed addons
sync-updated-addons = Updated addons
sync-removed-addons = Removed addons
sync-installed-dependencies = Installed dependencies
sync-updated-dependencies = Updated dependencies
sync-removed-dependencies = Removed dependencies
sync-downloaded-again = { $version } (downloaded again)
sync-not-applied = The AddOns folder was not changed because some downloads failed.
sync-download-failed = Download failed: { $name }: { $error }
sync-missing-dependency = Missing dependency: { $directory }
sync-unknown-addon = Not in the catalogue: { $name } (UID: { $uid })
remove-missing-title = Some addons are no longer in the catalogue
remove-missing-question = These addons were installed from the catalogue but are no longer in it (possibly only temporarily). Continuing the sync will delete their files and remove them from your installed list and favourites.
remove-missing-section = No longer in the catalogue
remove-missing-uid = UID: { $uid }
sync-up-to-date = Everything is up to date.

log-empty = Log is empty…
migrate-title = Keep which copy?
migrate-question = Your settings and add-ons exist both in the old folder and in the new profile location. Choose which copy to keep — it applies to all conflicting files.
migrate-keep-old = Keep the old folder
migrate-keep-profile = Keep the profile
migrate-open-folders = Open the folders

menu-updates = Updates
menu-check-on-launch = Check on launch
menu-notify-prereleases = Include pre-releases
menu-check-now = Check now
update-title = Update
update-available = A new version is available.
update-open-page = Download page
update-dont-ask = Don't ask again
update-up-to-date = You have the latest version.
update-check-failed = Could not check for updates.

menu-experimental = Experimental
menu-discontinued = Outdated add-ons
menu-ttc-prices = TTC prices
menu-update-prices = Update prices
region-eu = Europe
region-na = America
ttc-updated = TTC prices updated
ttc-failed = TTC price update failed: { $error }
