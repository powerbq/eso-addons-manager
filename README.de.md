# ESO Addons Manager

[English](README.md) · [Français](README.fr.md) · **Deutsch** · [Español](README.es.md) · [Русский](README.ru.md) · [Українська](README.uk.md)

Ein Desktop-Manager zum Installieren, Aktualisieren und Entfernen von Addons für
The Elder Scrolls Online — ein komfortablerer, eigenständiger Ersatz für Minion.
Die Abhängigkeiten eines Addons werden automatisch installiert (hin und wieder
bittet der Manager Sie, eine aus mehreren auszuwählen), und nicht mehr benötigte
werden entfernt, sodass Ihr AddOns-Ordner sauber bleibt: genau das, was zum
Ausführen Ihrer Addons nötig ist, und nichts Überflüssiges.

Läuft unter Windows, macOS und Linux. Die Oberfläche ist in sechs Sprachen verfügbar
— Englisch, Französisch, Deutsch, Spanisch, Russisch, Ukrainisch. Die Sprache wird
automatisch nach Ihrer Systemsprache gewählt und lässt sich in der App umschalten.

> Nutzen Sie bereits [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)?
> Nichts zu ändern — er aktualisiert und startet den Manager weiterhin wie bisher.
>
> Neue Nutzer laden den Manager jedoch besser aus den Dateien unten herunter. Beim
> Start aktualisiert [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)
> sich selbst und den Manager auf die neueste Version und startet ihn dann — und um
> sich selbst zu aktualisieren, überschreibt er seine eigene ausführbare Datei. Bei
> einer nicht signierten Anwendung können Antivirenprogramme dieses Verhalten als
> verdächtig einstufen und den Start blockieren. Der aus einer der Dateien unten
> installierte Manager tut nichts dergleichen: Er meldet lediglich eine neue Version
> und bietet an, die Download-Seite zu öffnen, während Sie sie selbst herunterladen
> und installieren. Das ist also der einfachste Weg, Ärger mit dem Antivirenprogramm
> zu vermeiden — es bleibt nur die Warnung, dass die Anwendung einen
> [unbekannten Herausgeber](#windows-smartscreen) hat.

## Download

Neueste Version: <https://github.com/powerbq/eso-addons-manager/releases/latest>

Wählen Sie einen **Installer** für Ihr System (empfohlen) oder ein
**eigenständiges** Zip:

| System | Installer | Eigenständig |
|--------|-----------|----------|
| Windows | `…-windows-amd64-setup.exe` | `eso-addons-manager-windows-amd64.zip` |
| macOS (Apple Silicon) | `…-macos-arm64.dmg` | `eso-addons-manager-macos-arm64.zip` |
| Linux | `.deb` / `.rpm` / `.pkg.tar.zst` / `.AppImage` | `eso-addons-manager-linux-amd64.zip` |

> Die Intel-Architektur unter macOS wird derzeit nicht unterstützt.

> Sie können jeden Download anhand der Datei `SHA256SUMS` in derselben Version
> überprüfen.

## Installation

- **Windows:** Führen Sie den `-setup.exe`-Installer aus (er bietet eine
  Installation pro Benutzer oder für alle Benutzer). Oder entpacken Sie das
  eigenständige Archiv an einen beliebigen Ort und führen Sie `app.exe` aus.
- **macOS:** Öffnen Sie das `.dmg` und ziehen Sie die App in den Ordner
  „Programme“. Oder entpacken Sie das eigenständige Archiv.
- **Linux:** Installieren Sie das Paket —
  `sudo dpkg -i eso-addons-manager_*.deb` / `sudo rpm -U eso-addons-manager-*.rpm` /
  `sudo pacman -U eso-addons-manager-*.pkg.tar.zst` — oder machen Sie das
  `.AppImage` ausführbar und starten Sie es, oder entpacken Sie das eigenständige
  Archiv (erteilen Sie der Binärdatei `app` Ausführungsrechte).

### Windows SmartScreen

Die App ist nicht mit einem Zertifikat signiert und wird wie besehen verteilt,
daher wird Windows SmartScreen höchstwahrscheinlich vor einem unbekannten
Herausgeber warnen. Das ist zu erwarten. Um sie trotzdem auszuführen, klicken Sie
auf **„Weitere Informationen“** → **„Trotzdem ausführen“**.

## Aktualisierung

Der Manager prüft beim Start auf eine neuere Version und bietet an, die
Download-Seite zu öffnen. Updates werden nie unbemerkt angewendet — Sie laden die
neue Version selbst herunter und installieren sie (den Installer erneut ausführen,
das Paket mit Ihrem Paketmanager aktualisieren oder die eigenständigen Dateien
ersetzen). Die Prüfung kann deaktiviert und bei Bedarf ausgeführt werden, über die
Gruppe **☰ Menü → Updates**.

## Vor dem Start

> **Wichtig.** Sichern Sie Ihren `AddOns`-Ordner vor der ersten Verwendung. Der
> Manager löscht aus dem ausgewählten Ordner alles, was er nicht verwaltet — wählen
> Sie den Ordner also sorgfältig. Um die Addons zu übernehmen, die Sie
> bereits haben, verwenden Sie **Vorhandene Addons scannen** (siehe unten).

## Verwendung

1. **Wählen Sie den AddOns-Ordner des Spiels** — **☰ Menü → AddOns-Ordner**. Ein
   Dialog erklärt, wo das Spiel seine Addons üblicherweise ablegt, und zeigt einen
   Beispielpfad für Ihr System.

2. **Vorhandene Addons scannen** — übernimmt Addons, die bereits im gewählten Ordner
   vorhanden sind, damit Sie sie nicht erneut hinzufügen müssen. Sie können vor der
   ersten Synchronisierung oder später scannen. Achtung: Wenn **Beim Start
   synchronisieren** aktiviert ist, löscht die automatische Synchronisierung beim
   nächsten Start extern hinzugefügte Addons, bevor Sie sie scannen — behalten Sie
   das im Hinterkopf.

3. **Katalog** — der Hauptbildschirm der App mit der Liste aller verfügbaren Addons.
   Oben: Suche nach Name, Autor oder Ordner, ein **Filter** (**Installiert**,
   **Abhängigkeiten**, **Favoriten**) und **Sortieren nach** (Name, Autor, Datum der
   letzten Aktualisierung, Downloadzahl usw.). Wählen Sie ein Addon, um seine Karte
   mit **Beschreibung**, **Änderungsprotokoll** und Screenshots zu öffnen. Von hier
   aus können Sie ein Addon installieren oder entfernen, es zu den **Favoriten**
   hinzufügen und seine Seite auf esoui.com öffnen.

4. **Synchronisieren** — aktualisiert alle installierten Addons und ihre
   Bibliotheken und entfernt alles Überflüssige. Aktivieren Sie **Beim Start
   synchronisieren** (unten rechts), um dies beim Start automatisch zu tun.

5. **Abhängigkeiten** — werden automatisch behandelt: Bibliotheken werden
   installiert, aktualisiert und entfernt, wenn kein Addon sie mehr benötigt. Sie
   pflegen nur die Liste der gewünschten Addons. Wenn mehrere Addons dieselbe
   Bibliothek bereitstellen, löst der Manager dies meist von selbst (er bevorzugt
   diejenige, auf die die Addon-Autoren verweisen). Nur wenn er es wirklich nicht
   entscheiden kann, **bittet er Sie zu wählen**. Ihre Wahl wird gemerkt, und jede
   solche Bibliothek (ob von Ihnen oder automatisch gewählt) wird im Reiter
   **Bibliotheken** angezeigt, wo Sie sie jederzeit ändern oder auf automatisch
   zurücksetzen können.

6. **Tamriel Trade Centre** — wenn TTC installiert ist, aktualisiert der Manager
   dessen Preistabellen automatisch während einer Synchronisierung, ohne den
   TTC-Client. Der Schalter und die Region (**Europa / Amerika**) befinden sich in
   der Gruppe **☰ Menü → TTC-Preise**.

7. **Veraltete Addons** — standardmäßig zeigt der Katalog nur aktuelle Addons. Wenn
   Sie ein altes, eingestelltes und daher ausgeblendetes Addon benötigen, aktivieren
   Sie **☰ Menü → Experimentell → Veraltete Addons**, und es erscheint im Katalog.
   Das ist experimentell und daher standardmäßig aus.

## Erweitert

- **Reiter Ausschlüsse.** Schützt Dateien vor dem Löschen — zum Beispiel die
  aktualisierbaren Preistabellen des Tamriel Trade Centre oder die heruntergeladene
  Datenbank aller Sammelpunkte von HarvestMap-Data. Automatische Ausschlüsse für
  **Tamriel Trade Centre** und **HarvestMap-Data** sind eingebaut und aktivieren
  sich, wenn diese Addons installiert sind; eigene Regeln können Sie links
  hinzufügen.

- **Reiter Patches.** Passt den *Inhalt* der Dateien eines Addons während einer
  Synchronisierung an — ein kleines Suchen-und-Ersetzen innerhalb einer Datei —,
  damit eine benötigte Korrektur nicht von den Originaldaten des Addons überschrieben
  wird. Eingebaute Patches werden automatisch angewendet, wenn das betreffende Addon
  installiert ist, und werden schreibgeschützt angezeigt, nach Addon beschriftet
  (zum Beispiel, um die Tamriel-Trade-Centre-Integration von **DovahMova** am Leben
  zu halten). Eigene Regeln können Sie links hinzufügen, ihre Anwendungsreihenfolge
  ändern und einzeln ein- oder ausschalten. Das ist die
  Selbsthilfe-Lösung, wenn ein eingebauter Patch nach einem Addon-Update nicht
  mehr passt oder wenn ein Addon, das Sie brauchen, noch keinen eingebauten Patch
  hat.

> Ausschluss- und Patch-Regeln werden mit regulären Ausdrücken definiert; schreiben
> und testen Sie sie auf <https://regex101.com/>.

> Wenn ein Addon Ausschlüsse oder Patches benötigt, um richtig zu funktionieren,
> geben Sie Feedback, dann kann die Unterstützung zu den eingebauten Listen
> hinzugefügt werden.

- **Ihre eigenen Addons (custom).** Um Addons zu installieren, die nicht im
  ESO-UI-Katalog stehen — Ihre eigenen oder solche, die Sie nur als `.zip` haben —,
  legen Sie das Archiv in den Ordner **custom** (**☰ Menü → App-Ordner → Eigene
  Addons**). Bei jeder Synchronisierung entpackt der Manager jedes `custom/*.zip` in
  Ihren AddOns-Ordner und behält es dort neben den Katalog-Addons, damit es nicht
  als verirrte Datei behandelt und entfernt wird. Ihre Abhängigkeiten werden wie bei
  jedem anderen Addon aufgelöst.

- **Wo Ihre Daten liegen.** Der Manager speichert seine Daten an den
  benutzerspezifischen, betriebssystemüblichen Orten, nicht neben der ausführbaren
  Datei; jeden Ordner können Sie direkt aus dem Menü öffnen:
  - **☰ Menü → App-Ordner → Konfiguration** — die `app.ini`-Einstellungen
    (installierte Addons, Favoriten, Bibliothekswahl, Ausschlüsse, Patches, Optionen)
    und ihre Sicherungen;
  - **☰ Menü → App-Ordner → App-Cache** — zwischengespeicherte Downloads:
    Addon-Archive, Katalog und TTC-Preisdaten;
  - **☰ Menü → App-Ordner → Eigene Addons** — Ihre eigenen `.zip`-Archive, bei jeder
    Synchronisierung in den AddOns-Ordner entpackt;
  - **☰ Menü → Protokoll → Ordner öffnen** — gespeicherte Protokolle.

  Die Ordner folgen den Konventionen der jeweiligen Plattform
  (`%LOCALAPPDATA%\EsoAddonsManager` unter Windows,
  `~/Library/…/io.github.powerbq.eso-addons-manager` unter macOS, die
  XDG-Basisverzeichnisse unter `eso-addons-manager` unter Linux). Beim ersten Start
  werden Daten aus dem alten Layout (im Arbeitsverzeichnis) automatisch an diese Orte
  migriert.

## Wenn etwas nicht stimmt

Wenn sich die App nicht wie erwartet verhält, nutzen Sie **☰ Menü → Support →
Problem melden**. In diesem Dialog können Sie wählen, was an die Meldung angehängt
wird, beschreiben, was passiert ist, und angeben, wie man Sie erreicht — und alles an
den Entwickler senden.

## Kompilieren

Das Erstellen aus dem Quellcode, das Packaging und der CI-Aufbau sind in
[BUILD.md](BUILD.md) (auf Englisch) dokumentiert.

## Lizenz

Veröffentlicht unter der [MIT-Lizenz](LICENSE).

Copyright (c) 2026 powerbq.

Es steht Ihnen frei, diese Software zu forken, zu verändern und
weiterzuverbreiten, aber der ursprüngliche Copyright-Hinweis und der Text der
MIT-Lizenz müssen in allen Kopien und abgeleiteten Werken erhalten bleiben.
