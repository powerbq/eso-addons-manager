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

Build a release archive:

```bash
# Linux / macOS
./build.sh

# Windows
build.cmd
```

On Windows, `litehtml-sys` 0.2.5 does not build with MSVC as published, so
`patches/cargo-patched.ps1` copies it to `target/patched/`, applies
`patches/litehtml-sys-0.2.5-msvc.patch` and runs Cargo with that copy, restoring
`Cargo.lock` afterwards. Use it in place of `cargo` there, for example:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File patches\cargo-patched.ps1 run
```

The build scripts compile with the `packaging` profile and write the archive and
its SHA-256 checksum to `target/dist/`, for example
`eso-addons-manager-linux-amd64.zip` and `eso-addons-manager-linux-amd64.zip.sha256`.
The same scripts are used by the GitHub Actions workflow, which publishes the
archives when the `current` or `testing` tag is pushed.

## Runtime files

The manager keeps everything in its current working directory:

- `app.ini` — settings, installed addons, favourites, library choices and exclusions;
- `addons/` — the download cache;
- `custom/` — your own `.zip` archives, extracted into the AddOns folder on every sync.

Diagnostics are shown on the Log tab and written to the console; set `RUST_LOG`
(for example `RUST_LOG=debug`) for more detail.

## License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 powerbq.

You are free to fork, modify and redistribute this software, but the original
copyright notice and the MIT license text must be preserved in all copies and
derivative works.
