# ESO Addons Manager

**English** · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [Українська](README.uk.md)

A desktop manager for installing, updating and removing add-ons for The Elder
Scrolls Online — a more convenient, self-contained replacement for Minion. An
add-on's dependencies are installed automatically (now and then the manager will
ask you to pick one of several options), and unneeded ones are removed, so
your AddOns folder stays clean: exactly what is needed to run your add-ons and
nothing extra.

Runs on Windows, macOS and Linux. The interface is available in six languages —
English, French, German, Spanish, Russian, Ukrainian. The language is chosen
automatically from your system language and can be switched in-app.

> Already using [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)?
> Nothing to change — it still updates and launches the manager, just as before.
>
> New users, though, are better off downloading the manager from the files below.
> On launch, [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)
> updates itself and the manager to the latest version, then launches it — and to
> update itself, it overwrites its own executable. For an unsigned application,
> antivirus software can treat this as suspicious and block the launch. The manager
> installed from one of the files below does none of this: it only tells you about a
> new version and offers to open the download page, while you download and install
> it yourself. So this is the simplest way to avoid antivirus trouble — the only
> thing left is the warning that the application has an
> [unknown publisher](#windows-smartscreen).

## Download

Latest release: <https://github.com/powerbq/eso-addons-manager/releases/latest>

Pick an **installer** for your system (recommended), or a **standalone** zip:

| System | Installer | Standalone |
|--------|-----------|----------|
| Windows | `…-windows-amd64-setup.exe` | `eso-addons-manager-windows-amd64.zip` |
| macOS (Apple Silicon) | `…-macos-arm64.dmg` | `eso-addons-manager-macos-arm64.zip` |
| Linux | `.deb` / `.rpm` / `.pkg.tar.zst` / `.AppImage` | `eso-addons-manager-linux-amd64.zip` |

> Intel architecture on macOS is not supported at the moment.

> You can verify any download against the `SHA256SUMS` file in the same release.

## Installation

- **Windows:** run the `-setup.exe` installer (it offers per-user or all-users
  install). Or unzip the standalone archive anywhere and run `app.exe`.
- **macOS:** open the `.dmg` and drag the app to Applications. Or unzip the
  standalone archive.
- **Linux:** install the package —
  `sudo dpkg -i eso-addons-manager_*.deb` / `sudo rpm -U eso-addons-manager-*.rpm` /
  `sudo pacman -U eso-addons-manager-*.pkg.tar.zst` — or make the `.AppImage`
  executable and run it, or unzip the standalone archive (grant execute permission
  on the `app` binary).

### Windows SmartScreen

The app is not signed with a certificate and is distributed as is, so Windows
SmartScreen will most likely warn about an unknown publisher. This is expected. To
run it anyway, click **"More info"** → **"Run anyway"**.

## Updating

The manager checks for a newer version on launch and offers to open the download
page. Updates are never applied silently — you download and install the new version
yourself (re-run the installer, update the package with your package manager, or
replace the standalone files). The check can be turned off, and run on demand, from
the **☰ menu → Updates** group.

## Before you start

> **Important.** Back up your `AddOns` folder before first use. The manager deletes
> from the selected folder everything it does not manage, so choose the
> folder carefully. To adopt the add-ons you already have, use **Scan for existing
> add-ons** (see below).

## Usage

1. **Choose the game's AddOns folder** — **☰ menu → AddOns folder**. A dialog
   explains where the game usually keeps its add-ons and shows an example path for
   your system.

2. **Scan for existing add-ons** — adopts add-ons already present in the chosen
   folder, so you do not have to re-add them. You can scan before the first sync or
   later. Caution: if **Sync on launch** is enabled, the automatic sync will delete
   externally-added add-ons on the next launch before you scan them — keep this in
   mind.

3. **Catalogue** — the app's main screen, listing every available add-on. At the
   top: search by name, author or folder, a **Filter** (**Installed**,
   **Dependencies**, **Favourites**) and **Sort by** (name, author, last updated,
   total downloads and so on). Select an add-on to open its card with its
   **Description**, **Changelog** and screenshots. From here you can install or
   remove an add-on, add it to **Favourites** and open its page on esoui.com.

4. **Sync** — updates all installed add-ons and their libraries, and removes
   anything extra. Enable **Sync on launch** (bottom-right) to do this automatically
   on start.

5. **Dependencies** — handled automatically: libraries are installed, updated, and
   removed when no add-on needs them any more. You maintain only the list of
   add-ons you want. When several add-ons provide the same library, the manager
   usually resolves it on its own (it prefers the one the add-on authors link to).
   Only when it genuinely cannot tell does it **ask you to choose**. Your choice is
   remembered, and every such library (whether chosen by you or automatically) is
   shown on the **Libraries** tab, where you can change it or reset it back to
   automatic at any time.

6. **Tamriel Trade Centre** — if TTC is installed, the manager updates its price
   tables automatically during a sync, without the TTC client. The toggle and the
   region (**Europe / America**) are in the **☰ menu → TTC prices** group.

7. **Outdated add-ons** — by default the catalogue shows only current add-ons. If
   you need an old add-on that is discontinued and therefore hidden, enable **☰ menu
   → Experimental → Outdated add-ons** and it will appear in the catalogue. This is
   experimental, so it is off by default.

## Advanced

- **Exclusions tab.** Protects files from deletion — for example Tamriel Trade
  Centre's updatable price tables or HarvestMap-Data's downloaded database of every
  harvest-node location. Automatic exclusions for **Tamriel Trade Centre** and
  **HarvestMap-Data** are built in and activate when those add-ons are installed; you
  can add your own rules on the left.

- **Patches tab.** Adjusts the *contents* of an add-on's files during a sync — a
  small find-and-replace inside a file — so a needed fix is not overwritten by the
  add-on's original data. Built-in patches apply automatically when the
  relevant add-on is installed and are shown read-only, labelled by add-on (for
  example, keeping **DovahMova**'s Tamriel Trade Centre integration alive). You can
  add your own rules on the left, reorder how they are applied, and turn each on or
  off. This is the self-serve fix when a built-in patch stops
  matching after an add-on updates, or when an add-on you need has no built-in
  patch yet.

> Exclusion and patch rules are defined with regular expressions; write and test them
> at <https://regex101.com/>.

> If an add-on needs exclusions or patches to work correctly, leave feedback and
> support can be added to the built-in lists.

- **Your own add-ons (custom).** To install add-ons that are not in the ESO UI
  catalogue — your own, or ones you only have as a `.zip` — put the archive in the
  **custom** folder (**☰ menu → App folders → Custom add-ons**). On every sync the
  manager extracts each `custom/*.zip` into your AddOns folder and keeps it there
  alongside the catalogue add-ons, so it is not treated as a stray file and removed.
  Their dependencies are resolved like any other add-on's.

- **Where your data lives.** The manager stores its data in the per-user,
  OS-standard locations, not next to the executable; each folder can be opened
  straight from the menu:
  - **☰ menu → App folders → Configuration** — the `app.ini` settings (installed
    add-ons, favourites, library choices, exclusions, patches, options) and their
    backups;
  - **☰ menu → App folders → App cache** — cached downloads: add-on archives, the
    catalogue and TTC price data;
  - **☰ menu → App folders → Custom add-ons** — your own `.zip` archives, extracted
    into the AddOns folder on every sync;
  - **☰ menu → Log → Open folder** — saved logs.

  The folders follow each platform's conventions (`%LOCALAPPDATA%\EsoAddonsManager`
  on Windows, `~/Library/…/io.github.powerbq.eso-addons-manager` on macOS, the XDG
  base directories under `eso-addons-manager` on Linux). On first run, data from the
  old working-directory layout is migrated to these locations automatically.

## Something went wrong

If the app does not behave as expected, use **☰ menu → Support → Report a problem**.
In that dialog you can choose what to attach to the report, describe what happened
and say how to reach you — and send it all to the developer.

## Build

Building from source, packaging and the CI layout are documented in
[BUILD.md](BUILD.md).

## License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 powerbq.

You are free to fork, modify and redistribute this software, but the original
copyright notice and the MIT license text must be preserved in all copies and
derivative works.
