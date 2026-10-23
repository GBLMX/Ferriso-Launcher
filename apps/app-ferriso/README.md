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
