# Build

Developer and packaging documentation for ESO Addons Manager. English only; the
user guide lives in the README (and its translations). Back to the
[README](README.md).

Written in Rust with the [iced](https://iced.rs) GUI library; builds for Windows,
macOS and Linux.

## Requirements

- the stable Rust toolchain ([rustup](https://rustup.rs));
- a C++ compiler for the bundled HTML renderer: Visual Studio Build Tools on
  Windows, the Xcode command line tools on macOS, `g++` or `clang++` on Linux.

## Compiling

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
format, each of which consumes the already-built binary and writes its output to
`target/dist/<format>/`:

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

## Release (GitHub Actions)

The GitHub Actions workflow mirrors this split: per-OS **build** jobs compile the
binary, **packager** jobs (one per format) each depend on their build job and run the
matching script, and a final **release** job gathers every artifact, generates a combined `SHA256SUMS`, and
publishes them as a GitHub release when a per-version `vX.Y.Z` tag is pushed. That tag
is created automatically by a separate workflow from the `Cargo.toml` version on each
push to `main`; a `-rc.N` suffix marks the release a pre-release.

## Diagnostics

Diagnostics are shown on the Log tab and written to the console; set `RUST_LOG`
(for example `RUST_LOG=debug`) for more detail. On-disk file logging and a shortcut
to the logs folder are available from the ☰ menu.
