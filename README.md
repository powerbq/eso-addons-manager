# ESO Addons Manager

A desktop manager for installing, updating and removing add-ons for The Elder
Scrolls Online. Built with Python and Qt (PySide6 / QML), with a multi-language
interface and cross-platform builds for Linux, macOS and Windows.

## Usage

This repository holds only the codebase of the manager. The release files here
are not a standalone application — they are used by
[eso-addons-updater](https://github.com/powerbq/eso-addons-updater). To use the app,
download the updater and run it; it fetches and launches this manager.

For end-user documentation and features, see the
[eso-addons-updater](https://github.com/powerbq/eso-addons-updater) repository.

## Build

Install the dependencies and build a standalone bundle with PyInstaller.

```bash
pip install -r requirements.txt

# Linux / macOS
./build.sh

# Windows
build.cmd
```

The build script compiles the Qt translations and writes the bundle to
`build/dist/`.

## License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 powerbq.

You are free to fork, modify and redistribute this software, but the original
copyright notice and the MIT license text must be preserved in all copies and
derivative works.
