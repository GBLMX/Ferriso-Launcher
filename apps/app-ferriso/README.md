# Ferriso Launcher

Ferriso Launcher is an unofficial, native fork of the Modrinth launcher. It
reuses the Modrinth backend and **Modrinth's services** for:

- Microsoft and Modrinth account authentication
- project search, metadata, versions, and install planning
- skin and cape state
- shared-instance collaboration

It is not affiliated with or endorsed by Modrinth. "Modrinth" and the Modrinth
marks belong to their respective owners. The fork is distributed under
GPL-3.0-only, matching the upstream launcher.

The native frontend is built with iced; launcher state and installs are owned
by `theseus` in `packages/app-lib`. The data directory keeps the upstream
`ModrinthApp` identifier so existing instances and accounts remain available.

## Build

```sh
cargo run -p ferriso-launcher
cargo test -p ferriso-launcher
cargo clippy -p ferriso-launcher --all-targets -- -D warnings
```

Window and layered configuration live in `$FERRISO_LAUNCHER_CONFIG` or the
platform config directory (`config.toml`).

## Accessibility

The iced frontend exposes an AccessKit tree: navigation, page status, refresh,
and the toggleable settings are available to assistive technology, and their
actions are routed back through the same semantic messages as pointer input.
On Linux this uses AT-SPI; the desktop's screen reader or
`org.gnome.desktop.interface toolkit-accessibility` enables it. AccessKit is
bridged through a small vendored `iced_winit` patch because iced 0.14 does not
expose an accessibility hook.

## Packaging

Release builds are produced by `.github/workflows/ferriso-build.yml` for tags
matching `ferriso-v*`:

- Linux: AppImage, `.deb`, and a portable tarball
- Windows: NSIS installer and standalone `.exe`
- Every artifact gets a SHA-256 checksum; the release includes
  `ferriso-updates.json` for the native updater.

Local Linux packaging after `cargo build --release -p ferriso-launcher`:

```sh
cargo deb -p ferriso-launcher --no-build --deb-version 0.1.0
tar -C target/release -czf ferriso-launcher-0.1.0-linux-x86_64.tar.gz ferriso-launcher
FERRISO_VERSION=0.1.0 apps/app-ferriso/packaging/build-appimage.sh
```

`build-appimage.sh` requires `appimagetool`; set `APPIMAGETOOL` when it is not
on `PATH`. The CI downloads the AppImage tool and runs it with
`--appimage-extract-and-run`, so FUSE is not required.

Optional signing secrets:

- `WINDOWS_CERTIFICATE` — base64-encoded PFX
- `WINDOWS_CERTIFICATE_PASSWORD`
- `GPG_PRIVATE_KEY` — armored private key for detached release signatures
