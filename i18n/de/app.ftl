tab-catalogue = Katalog
tab-libraries = Bibliotheken
tab-exclusions = Ausschlüsse
tab-log = Protokoll

menu-title = Menü
addons-folder = AddOns-Ordner
addons-folder-none = Nicht ausgewählt
no-folder-title = AddOns-Ordner nicht ausgewählt
change-folder-title = Neuen AddOns-Ordner auswählen
no-folder-question = Das Spiel sucht Add-ons im AddOns-Ordner — meist unter { $example }. Starten Sie das Spiel mindestens einmal, damit es diese Ordner anlegt.
no-folder-warning = Wählen Sie genau den Ordner mit dem Namen AddOns — der Manager entfernt daraus alle ihm unbekannten Dateien.
no-folder-choose = Ordner auswählen
menu-language = Sprache
menu-choose-folder = Ändern
menu-select-folder = Auswählen
scan-folder = Scannen
menu-open-folder = Öffnen
menu-app-folders = App-Ordner
menu-configuration = Konfiguration
menu-cache = App-Cache
menu-custom = Eigene Addons
log-to-file = In Datei schreiben
log-open-folder = Ordner öffnen
log-clear = Protokolle löschen
log-clear-title = Protokolldateien löschen?
log-clear-all-disable = Alle und Schreiben beenden
log-clear-all = Alle Protokolldateien
log-clear-others = Frühere Durchläufe
sync-on-launch = Beim Start synchronisieren
sync = Synchronisieren

loading = Wird geladen…
loading-catalogue = Katalog wird geladen…
catalogue-failed = Der Katalog kann nicht geladen werden: { $error }
retry = Erneut versuchen

search-placeholder = Nach Name, Autor oder Ordner suchen…
addon-count =
    { $count ->
        [one] { $count } Addon
       *[other] { $count } Addons
    }
filter = Filter
filter-installed = Installiert
filter-dependencies = Abhängigkeiten
filter-favourites = Favoriten
sort-by = Sortieren nach
sort-name = Name
sort-author = Autor
sort-updated = Zuletzt aktualisiert
sort-downloads = Downloads insgesamt
sort-monthly-downloads = Downloads im Monat
sort-favourites = Favoriten
sort-status = Installationsstatus

addon-version = Version { $version }
add-favourite = Zu Favoriten hinzufügen
remove-favourite = Aus Favoriten entfernen
install = Installieren
install-directly = Automatisch installiert — zum direkten Installieren klicken
required-by = Benötigt von:
remove = Entfernen
open-page = Auf esoui.com öffnen
select-addon = Addon auswählen, um seine Beschreibung zu sehen
description-failed = Die Beschreibung kann nicht geladen werden: { $error }
description-render-failed = Die Beschreibung kann nicht angezeigt werden
tab-description = Beschreibung
tab-changelog = Änderungsprotokoll
screenshot-failed = Der Screenshot kann nicht geladen werden
previous = ZURÜCK
next = WEITER

libraries-none = Noch keine gespeicherte Auswahl. Wenn mehrere Addons denselben Ordner bereitstellen, erscheint Ihre Auswahl hier.
reset = Zurücksetzen
recommended = Empfohlen

exclusions-custom = Eigene Ausschlüsse — ein regulärer Ausdruck pro Zeile
exclusions-automatic = Automatisch (aus installierten Addons)
exclusions-automatic-none = Keine: Kein installiertes Addon benötigt sie.
clear = Leeren
save = Speichern

ok = OK
cancel = Abbrechen
continue = Weiter
open-link-question = Diesen Link im Browser öffnen?
open-link-confirm = Öffnen
providers-title = Addons auswählen
providers-note = Mehrere Addons stellen diese Ordner bereit. Wählen Sie für jeden Ordner aus, welches verwendet werden soll.
candidate-folders = Ordner: { $folders }
scan-prompt-title = Ordner nach Addons scannen?
scan-prompt-question = Möchten Sie diesen Ordner nach installierten Addons scannen und sie Ihrer Liste hinzufügen?
scan-prompt-warning = Warnung: Die Erkennung kann ungenau sein. Sichern Sie zuerst den Ordner — sein Inhalt kann bei der nächsten Synchronisierung überschrieben werden.
scan-keep-folder = Nein, Ordner nicht ändern
scan-skip = Nein, nicht scannen
scan-confirm = Ja, scannen
scan-complete = Scan abgeschlossen
scan-failed = Scan fehlgeschlagen
scan-added = Hinzugefügte Addons
scan-unrecognised = Nicht erkannte Ordner
scan-nothing-new = Im Ordner wurden keine neuen Addons gefunden.
catalogue-unavailable = Der Katalog ist nicht verfügbar.
catalogue-not-loaded = Der Katalog ist noch nicht geladen.

sync-complete = Synchronisierung abgeschlossen
sync-failed = Synchronisierung fehlgeschlagen
report-problems = Probleme
sync-installed-addons = Installierte Addons
sync-updated-addons = Aktualisierte Addons
sync-removed-addons = Entfernte Addons
sync-installed-dependencies = Installierte Abhängigkeiten
sync-updated-dependencies = Aktualisierte Abhängigkeiten
sync-removed-dependencies = Entfernte Abhängigkeiten
sync-downloaded-again = { $version } (erneut heruntergeladen)
sync-not-applied = Der AddOns-Ordner wurde nicht geändert, weil einige Downloads fehlgeschlagen sind.
sync-download-failed = Download fehlgeschlagen: { $name }: { $error }
sync-missing-dependency = Fehlende Abhängigkeit: { $directory }
sync-unknown-addon = Nicht im Katalog: { $name } (UID: { $uid })
remove-missing-title = Einige Addons sind nicht mehr im Katalog
remove-missing-question = Diese Addons wurden aus dem Katalog installiert, sind dort aber nicht mehr vorhanden (möglicherweise nur vorübergehend). Wenn Sie die Synchronisierung fortsetzen, werden ihre Dateien gelöscht und sie aus den installierten Addons und den Favoriten entfernt.
remove-missing-section = Nicht mehr im Katalog
remove-missing-uid = UID: { $uid }
sync-up-to-date = Alles ist aktuell.

log-empty = Protokoll ist leer…
migrate-title = Welche Kopie behalten?
migrate-question = Ihre Einstellungen und Add-ons wurden sowohl im alten Ordner als auch am neuen Profilspeicherort gefunden. Wählen Sie, welche Kopie behalten werden soll — die Auswahl gilt für alle Dateien mit Konflikt.
migrate-keep-old = Alten Ordner behalten
migrate-keep-profile = Profil behalten
migrate-open-folders = Ordner öffnen

menu-updates = Updates
menu-check-on-launch = Beim Start prüfen
menu-notify-prereleases = Vorabversionen einbeziehen
menu-check-now = Jetzt prüfen
update-title = Update
update-available = Eine neue Version ist verfügbar.
update-open-page = Download-Seite
update-dont-ask = Nicht mehr fragen
update-up-to-date = Sie haben die neueste Version.
update-check-failed = Updates konnten nicht geprüft werden.

menu-experimental = Experimentell
menu-discontinued = Veraltete Addons
menu-ttc-prices = TTC-Preise
menu-update-prices = Preise aktualisieren
region-eu = Europa
region-na = Amerika
ttc-updated = TTC-Preise aktualisiert
ttc-failed = TTC-Preisaktualisierung fehlgeschlagen: { $error }
