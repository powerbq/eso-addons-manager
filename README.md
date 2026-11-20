# ESO Addons Manager

A desktop manager for installing, updating and removing add-ons for The Elder
Scrolls Online. Written in Rust with the [iced](https://iced.rs) GUI library,
with a multi-language interface and builds for Linux, macOS and Windows.

## Usage

This repository holds only the codebase of the manager. The release files here
are not a standalone application — they are used by
[eso-addons-updater](https://github.com/powerbq/eso-addons-updater). To use the app,
download the updater and run it; it fetches and launches this manager.

For end-user documentation and features, see the
[eso-addons-updater](https://github.com/powerbq/eso-addons-updater) repository.

## Build

Requirements:

- the stable Rust toolchain ([rustup](https://rustup.rs));
- a C++ compiler for the bundled HTML renderer: Visual Studio Build Tools on
  Windows, the Xcode command line tools on macOS, `g++` or `clang++` on Linux.

Run it during development:

```bash
cargo run
```

Run the tests:

```bash
cargo test --workspace
```

Build the release binary:

```bash
# Linux / macOS
./build.sh

# Windows
build.cmd
```

`build.sh` / `build.cmd` only **compile** with the `packaging` profile, producing
`target/packaging/app` (`app.exe` on Windows). Turning that binary into a
distributable is a separate step: the `packaging/` directory holds one packager per
format, each of which consumes the already-built binary and writes its output (plus a
`.sha256`) to `target/dist/<format>/`:

```bash
packaging/zip-linux/zip.sh        # portable zip (used by the updater)
packaging/appimage/appimage.sh    # .AppImage
packaging/deb/deb.sh              # .deb        (needs cargo-deb)
packaging/rpm/rpm.sh              # .rpm        (needs cargo-generate-rpm)
packaging/arch/arch.sh           # .pkg.tar.zst (needs makepkg, i.e. Arch)
packaging/dmg/dmg.sh              # .dmg        (macOS)
packaging/zip-macos/zip.sh        # macOS .app zip
packaging/zip-windows/zip.cmd     # Windows zip
packaging/inno/inno.cmd           # Windows installer (Inno Setup; runs ISCC on the .iss)
```

So a full local build is: run `build.sh`/`build.cmd` once, then any packager. Run a
packager without a built binary and it fails.

On Windows, `litehtml-sys` 0.2.5 does not build with MSVC as published, so
`patches/cargo-patched.ps1` copies it to `target/patched/`, applies
`patches/litehtml-sys-0.2.5-msvc.patch` and runs Cargo with that copy, restoring
`Cargo.lock` afterwards. Use it in place of `cargo` there, for example:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File patches\cargo-patched.ps1 run
```

The GitHub Actions workflow mirrors this split: per-OS **build** jobs compile the
binary, **packager** jobs (one per format) each depend on their build job and run the
matching script, and a final **release** job gathers every artifact, writes
`version.txt` and a combined `SHA256SUMS`, and publishes them when the `current` or
`testing` tag is pushed.

## Runtime files

The manager stores its data in the per-user, OS-standard locations, not next to
the executable:

- **settings** (`app.ini`) — installed addons, favourites, library choices and exclusions;
- **downloads** — the archive cache;
- **custom** — your own `.zip` archives, extracted into the AddOns folder on every sync;
- **logs** — optional on-disk logs (see below).

The exact folders follow each platform's conventions (`%LOCALAPPDATA%` on Windows,
`~/Library` on macOS, the XDG base directories on Linux). The ☰ menu's **App folders**
group opens each of them in your file manager. On first run, data found in the old
working-directory layout (`app.ini`, `addons/`, `custom/`) is migrated to these
locations automatically.

Diagnostics are shown on the Log tab and written to the console; set `RUST_LOG`
(for example `RUST_LOG=debug`) for more detail. On-disk file logging, and a shortcut
to the logs folder, are available from the ☰ menu.

## License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 powerbq.

You are free to fork, modify and redistribute this software, but the original
copyright notice and the MIT license text must be preserved in all copies and
derivative works.
