# Ferriso Launcher

Ferriso Launcher is an unofficial, native fork of the [Modrinth App](https://github.com/Modrinth/code). It keeps Modrinth's authentication, project metadata, install planning, skin, and shared-instance services, while replacing the web frontend with a Rust + [iced](https://iced.rs) desktop application.

> **Not affiliated with or endorsed by Modrinth or Mojang.** "Modrinth", "Minecraft", and their marks belong to their respective owners. Ferriso does not ship Minecraft assets or Modrinth branding.

## Branch layout

`main` tracks the upstream Modrinth monorepo. The launcher source lives on the active fork branch:

| Branch | Purpose |
| --- | --- |
| [`main`](https://github.com/GBLMX/Ferriso-Launcher/tree/main) | Upstream sync branch |
| [`win-work-20260923`](https://github.com/GBLMX/Ferriso-Launcher/tree/win-work-20260923) | Ferriso Launcher source, packaging, and release workflow |

## Features

- Native iced interface with responsive layout, themes, and keyboard shortcuts.
- Instance library with search, group filters, sorting, playtime, and running-state controls.
- Browse and install modpacks, mods, resource packs, and shaders through Modrinth.
- Microsoft and Modrinth account authentication, plus skin and cape management.
- Worlds, logs, file manager, crash analysis, shared instances, and server hosting.
- Background install jobs with progress and cancellation.
- Layered configuration through `config-rs`: system, user, and project files, an explicit config path, and environment overrides.
- AccessKit accessibility tree for navigation, status, and settings on Linux via AT-SPI.

## Download

Prebuilt Linux and Windows packages are attached to tagged [releases](https://github.com/GBLMX/Ferriso-Launcher/releases). A `ferriso-v*` tag triggers the build workflow and publishes:

- Linux x86_64: AppImage, `.deb`, and portable `.tar.gz`
- Windows x86_64: NSIS installer and standalone `.exe`
- SHA-256 checksums and a native update manifest

If no release is available yet, build from source using the active branch below.

## Build from source

Requirements:

- Rust `1.95.0` (the workspace pins this toolchain)
- JDK 17
- A C/C++ toolchain and platform GUI development libraries

```sh
git clone https://github.com/GBLMX/Ferriso-Launcher.git
cd Ferriso-Launcher
git switch win-work-20260923
cp packages/app-lib/.env.prod packages/app-lib/.env
cargo run -p ferriso-launcher
```

On Debian/Ubuntu, install the native dependencies with:

```sh
sudo apt-get install -y libxkbcommon-dev libwayland-dev libx11-dev \
  libxcursor-dev libxrandr-dev libxi-dev libfontconfig1-dev \
  libfreetype-dev libvulkan-dev libegl1-mesa-dev
```

On Windows, install the MSVC build tools and JDK 17, then run the same Cargo command from a Developer PowerShell.

## Test and lint

```sh
cargo test -p ferriso-launcher
cargo clippy -p ferriso-launcher --all-targets -- -D warnings
```

## Configuration

The native frontend reads `config.toml` from the system config directory and the platform user config directory. Later layers override earlier ones:

```text
/etc/ferriso-launcher/config.toml
<platform user config>/ferriso-launcher/config.toml
./ferriso-launcher.toml
$FERRISO_LAUNCHER_CONFIG
FERRISO_LAUNCHER__GUI__WIDTH=1920
```

Example:

```toml
[gui]
title = "Ferriso Launcher"
width = 1280
height = 820
application_id = "com.ferriso.FerrisoLauncher"
```

This configuration controls the native window only. Launcher settings such as memory, Java, downloads, and UI preferences remain in `theseus`'s settings store. The data directory keeps the upstream `ModrinthApp` identifier so existing instances and accounts remain available.

## Packaging

After a release build:

```sh
cargo build --release -p ferriso-launcher
cargo deb -p ferriso-launcher --no-build --deb-version 0.1.0
tar -C target/release -czf ferriso-launcher-0.1.0-linux-x86_64.tar.gz ferriso-launcher
FERRISO_VERSION=0.1.0 apps/app-ferriso/packaging/build-appimage.sh
```

`build-appimage.sh` needs `appimagetool` on `PATH` or via `APPIMAGETOOL`. The GitHub workflow downloads it and runs it with `--appimage-extract-and-run`, so FUSE is not required.

Optional release signing secrets:

- `WINDOWS_CERTIFICATE` — base64-encoded PFX
- `WINDOWS_CERTIFICATE_PASSWORD`
- `GPG_PRIVATE_KEY` — armored private key for detached asset signatures

## Project layout

| Path | Contents |
| --- | --- |
| `apps/app-ferriso` | Native iced frontend and launcher binary |
| `packages/app-lib` | `theseus` launcher core, installs, auth, and state |
| `packages/ferriso-access` | AccessKit/AT-SPI bridge |
| `apps/app` | Upstream Tauri application |
| `apps/frontend` | Upstream web frontend |

The launcher reuses `theseus` from `packages/app-lib`. The web desktop app remains in the workspace for upstream compatibility.

## License

Ferriso Launcher is distributed under **GPL-3.0-only**, matching the upstream launcher. Other packages keep their respective licenses; see [COPYING.md](COPYING.md) and each package's license file.

Upstream project: [Modrinth/code](https://github.com/Modrinth/code).
