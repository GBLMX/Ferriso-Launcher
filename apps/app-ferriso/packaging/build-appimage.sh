#!/usr/bin/env bash
# Build a type-2 AppImage for Ferriso Launcher.
#
# Requirements:
#   - a release binary from `cargo build --release -p ferriso-launcher`
#   - appimagetool on PATH or APPIMAGETOOL=/path/to/appimagetool
#
# Environment:
#   FERRISO_VERSION   version embedded in the AppImage name (default: 0.1.0)
#   FERRISO_BINARY    launcher binary (default: target/release/ferriso-launcher)
#   FERRISO_OUTPUT    output directory (default: target/release)
#   APPIMAGETOOL      appimagetool executable (default: appimagetool)
#   APPIMAGETOOL_FLAGS extra flags, e.g. --appimage-extract-and-run
#   FERRISO_APPIMAGE_STAGE_ONLY=1  assemble AppDir and skip appimagetool

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
VERSION="${FERRISO_VERSION:-0.1.0}"
BINARY="${FERRISO_BINARY:-$ROOT/target/release/ferriso-launcher}"
OUTPUT="${FERRISO_OUTPUT:-$ROOT/target/release}"
PACKAGING="$ROOT/apps/app-ferriso/packaging"
APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"
APPIMAGETOOL_FLAGS="${APPIMAGETOOL_FLAGS:-}"

if [[ ! -x "$BINARY" ]]; then
  echo "error: launcher binary not found or not executable: $BINARY" >&2
  exit 1
fi

case "$(uname -m)" in
  x86_64 | amd64) ARCH="x86_64" ;;
  aarch64 | arm64) ARCH="aarch64" ;;
  *) echo "error: unsupported AppImage architecture: $(uname -m)" >&2; exit 1 ;;
esac

APPDIR="$OUTPUT/Ferriso-Launcher.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/512x512/apps" \
  "$APPDIR/usr/share/metainfo"

install -Dm755 "$BINARY" "$APPDIR/usr/bin/ferriso-launcher"
install -Dm644 "$PACKAGING/com.ferriso.FerrisoLauncher.desktop" \
  "$APPDIR/usr/share/applications/com.ferriso.FerrisoLauncher.desktop"
install -Dm644 "$PACKAGING/com.ferriso.FerrisoLauncher.png" \
  "$APPDIR/usr/share/icons/hicolor/512x512/apps/com.ferriso.FerrisoLauncher.png"
install -Dm644 "$PACKAGING/com.ferriso.FerrisoLauncher.metainfo.xml" \
  "$APPDIR/usr/share/metainfo/com.ferriso.FerrisoLauncher.metainfo.xml"

# Type-2 AppImage metadata lives at the AppDir root so the runtime can find it.
install -Dm644 "$PACKAGING/com.ferriso.FerrisoLauncher.desktop" \
  "$APPDIR/com.ferriso.FerrisoLauncher.desktop"
install -Dm644 "$PACKAGING/com.ferriso.FerrisoLauncher.png" \
  "$APPDIR/com.ferriso.FerrisoLauncher.png"
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
set -e
exec "$APPDIR/usr/bin/ferriso-launcher" "$@"
APPRUN
chmod 0755 "$APPDIR/AppRun"

if [[ "${FERRISO_APPIMAGE_STAGE_ONLY:-0}" == "1" ]]; then
  echo "AppDir ready: $APPDIR"
  exit 0
fi

if ! command -v "$APPIMAGETOOL" >/dev/null 2>&1 && [[ ! -x "$APPIMAGETOOL" ]]; then
  echo "error: appimagetool not found; set APPIMAGETOOL=/path/to/appimagetool" >&2
  exit 1
fi

mkdir -p "$OUTPUT"
OUTFILE="$OUTPUT/Ferriso-Launcher-${VERSION}-${ARCH}.AppImage"
rm -f "$OUTFILE"
# shellcheck disable=SC2086
ARCH="$ARCH" "$APPIMAGETOOL" $APPIMAGETOOL_FLAGS "$APPDIR" "$OUTFILE"
echo "AppImage written: $OUTFILE"
