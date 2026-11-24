# ESO Addons Manager

A desktop manager for installing, updating and removing add-ons for The Elder
Scrolls Online — a more convenient, self-contained replacement for Minion. It
installs, updates and removes add-ons together with their dependencies and keeps
your AddOns folder clean: exactly what is needed to run your add-ons and nothing
extra. Add-ons are shown with the same icons as on ESO UI.

Written in Rust with the [iced](https://iced.rs) GUI library. Builds for Windows,
macOS and Linux. The interface is available in six languages — English, French,
German, Spanish, Russian, Ukrainian — chosen automatically from your system
language and switchable in-app.

> Existing [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)
> users: that updater is now legacy. It still works and keeps launching this
> manager, but new users should install the manager directly from the downloads
> below.

## Download

Latest release: <https://github.com/powerbq/eso-addons-manager/releases/latest>

Pick an **installer** for your system (recommended), or a **standalone** zip:

| System | Installer | Standalone |
|--------|-----------|----------|
| Windows | `…-windows-amd64-setup.exe` | `eso-addons-manager-windows-amd64.zip` |
| macOS (Apple Silicon) | `…-macos-arm64.dmg` | `eso-addons-manager-macos-arm64.zip` |
| Linux | `.deb` / `.rpm` / `.pkg.tar.zst` / `.AppImage` | `eso-addons-manager-linux-amd64.zip` |

> Intel architecture on macOS is not supported at the moment. You can verify any
> download against the `SHA256SUMS` file in the same release.

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
SmartScreen (and the installer) will most likely warn about an unknown publisher.
This is expected. To run it anyway, click **"More info"** → **"Run anyway"**.

## Updating

The manager checks for a newer version on launch and offers to open the download
page. Updates are never applied silently — you download and install the new version
yourself (re-run the installer, update the package with your package manager, or
replace the standalone files). The check can be turned off, and run on demand, from
the **☰ menu → Updates** group.

## Before you start

> **Important.** Back up your `AddOns` folder before first use. The manager deletes
> from the selected folder everything it did not install itself, so choose the
> folder carefully. To adopt the add-ons you already have, use **Scan for existing
> add-ons** (see below).

## Usage

1. **Choose the game's AddOns folder** — **☰ menu → AddOns folder**. A dialog
   explains it and shows an example path for your system, for example:

   ```
   Documents\Elder Scrolls Online\live\AddOns
   ```

2. **Scan for existing add-ons** — adopt add-ons already present in the chosen
   folder, so you do not have to re-add them. You can scan before the first sync or
   later. Caution: if **Sync on launch** is enabled, the automatic sync will delete
   externally-added add-ons on the next launch before you scan them — keep this in
   mind.

3. **Sync** — updates all installed add-ons and their libraries. Enable **Sync on
   launch** (bottom-right) to do this automatically on start.

4. **Dependencies** — handled automatically: libraries are installed, updated, and
   removed when no add-on needs them any more. You maintain only the list of
   add-ons you want. When several add-ons provide the same library, the manager
   usually resolves it on its own (it prefers the one the add-on authors link to).
   Only when it genuinely cannot tell does it **ask you to choose** — a short dialog
   lists the candidates, and any that an add-on author links to are marked
   **Recommended** (there may be none, or several). Your choice is remembered, and
   every such library (whether chosen by you or automatically) is shown on the
   **Libraries** tab, where you can change it or reset it back to automatic at any
   time.

5. **Tamriel Trade Centre** — if TTC is installed, the manager updates its price
   tables automatically during a sync, without the TTC client. The toggle and the
   region (**Europe / America**) are in the **☰ menu → TTC prices** group.

## Advanced

- **Exclusions tab.** Protects files from deletion — for example the Tamriel Trade
  Centre client files or the HarvestMap-Data map download. Automatic exclusions for
  **Tamriel Trade Centre** and **HarvestMap-Data** are built in and activate when
  those add-ons are installed; you can add your own regexes on the left. How to
  write and test rules: <https://regex101.com/>. If an add-on needs exclusions,
  leave feedback and support can be added to the automatic list.

- **Your own add-ons (custom).** To install add-ons that are not in the ESO UI
  catalogue — your own, or ones you only have as a `.zip` — put the archive in the
  **custom** folder (**☰ menu → App folders → Custom add-ons**). On every sync the
  manager extracts each `custom/*.zip` into your AddOns folder and keeps it there
  alongside the catalogue add-ons, so it is not treated as a stray file and removed.
  Their dependencies are resolved like any other add-on's.

- **Where your data lives.** The manager stores its data in the per-user,
  OS-standard locations, not next to the executable:
  - **settings** (`app.ini`) — installed add-ons, favourites, library choices,
    exclusions and options;
  - **app cache** — cached downloads (add-on archives, catalogue and TTC price data);
  - **custom** — your own `.zip` archives, extracted into the AddOns folder on every
    sync;
  - **logs** — optional on-disk logs.

  The folders follow each platform's conventions (`%LOCALAPPDATA%\EsoAddonsManager`
  on Windows, `~/Library/…/io.github.powerbq.eso-addons-manager` on macOS, the XDG
  base directories under `eso-addons-manager` on Linux). **☰ menu → App folders**
  opens the configuration, custom and cache folders in your file manager; the logs
  folder opens from **☰ menu → Log → Open folder**. On first run, data from the old
  working-directory layout is migrated to these locations automatically.

## Build

Requirements:

- the stable Rust toolchain ([rustup](https://rustup.rs));
- a C++ compiler for the bundled HTML renderer: Visual Studio Build Tools on
  Windows, the Xcode command line tools on macOS, `g++` or `clang++` on Linux.

Run it during development, and the tests:

```bash
cargo run
cargo test --workspace
```

Build the release binary:

```bash
./build.sh    # Linux / macOS
build.cmd     # Windows
```

`build.sh` / `build.cmd` only **compile** with the `packaging` profile, producing
`target/packaging/app` (`app.exe` on Windows). Turning that binary into a
distributable is a separate step: the `packaging/` directory holds one packager per
format, each of which consumes the already-built binary and writes its output (plus a
`.sha256`) to `target/dist/<format>/`:

```bash
packaging/zip-linux/zip.sh        # standalone zip
packaging/appimage/appimage.sh    # .AppImage
packaging/deb/deb.sh              # .deb         (needs cargo-deb)
packaging/rpm/rpm.sh              # .rpm         (needs cargo-generate-rpm)
packaging/arch/arch.sh            # .pkg.tar.zst (needs makepkg, i.e. Arch)
packaging/dmg/dmg.sh              # .dmg         (macOS)
packaging/zip-macos/zip.sh        # macOS .app zip
packaging/zip-windows/zip.cmd     # Windows zip
packaging/inno/inno.cmd           # Windows installer (Inno Setup; runs ISCC on the .iss)
```

So a full local build is: run `build.sh`/`build.cmd` once, then any packager. Run a
packager without a built binary and it fails. App icons are generated from
`assets/eso-addons-manager.svg` by `./icons.sh` into the committed `icons/`.

On Windows, `litehtml-sys` 0.2.5 does not build with MSVC as published, so
`patches/cargo-patched.ps1` copies it to `target/patched/`, applies
`patches/litehtml-sys-0.2.5-msvc.patch` and runs Cargo with that copy, restoring
`Cargo.lock` afterwards. Use it in place of `cargo` there, for example:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File patches\cargo-patched.ps1 run
```

The GitHub Actions workflow mirrors this split: per-OS **build** jobs compile the
binary, **packager** jobs (one per format) each depend on their build job and run the
matching script, and a final **release** job gathers every artifact, generates a combined `SHA256SUMS`, and
publishes them as a GitHub release when a per-version `vX.Y.Z` tag is pushed. That tag
is created automatically by a separate workflow from the `Cargo.toml` version on each
push to `main`; a `-rc.N` suffix marks the release a pre-release.

Diagnostics are shown on the Log tab and written to the console; set `RUST_LOG`
(for example `RUST_LOG=debug`) for more detail. On-disk file logging and a shortcut
to the logs folder are available from the ☰ menu.

## License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 powerbq.

You are free to fork, modify and redistribute this software, but the original
copyright notice and the MIT license text must be preserved in all copies and
derivative works.
