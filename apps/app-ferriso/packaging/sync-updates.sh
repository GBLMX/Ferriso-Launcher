#!/usr/bin/env bash
# Publish the latest Ferriso GitHub release to the static update mirror.
#
# Run from cron on the mirror host (or by hand after a release):
#   FERRISO_UPDATE_ROOT=/var/www/ferriso-updates \
#   FERRISO_UPDATE_BASE_URL=https://updates.bionickernel.com \
#   apps/app-ferriso/packaging/sync-updates.sh
#
# The manifest is replaced only after every asset has been downloaded and
# rewritten, so clients never see a half-published release. The script is
# idempotent: when the mirror already serves the latest version it exits.

set -euo pipefail

REPO="${FERRISO_REPO:-GBLMX/Ferriso-Launcher}"
BASE_URL="${FERRISO_UPDATE_BASE_URL:-https://updates.bionickernel.com}"
ROOT="${FERRISO_UPDATE_ROOT:-/var/www/ferriso-updates}"
LOCK="${FERRISO_UPDATE_LOCK:-/run/ferriso-update-sync.lock}"

exec 9>"$LOCK"
flock -n 9 || exit 0

release="$(
    curl -fsSL --retry 3 --max-time 30 \
        "https://api.github.com/repos/$REPO/releases/latest"
)"
tag="$(jq -r '.tag_name // empty' <<<"$release")"
case "$tag" in
    ferriso-v*) ;;
    *) exit 0 ;;
esac

version="${tag#ferriso-v}"
current="$(
    jq -r '.version // empty' "$ROOT/ferriso-updates.json" 2>/dev/null || true
)"
[ "$current" = "$version" ] && exit 0

tmp="$(mktemp -d "$ROOT/.sync.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

asset_urls="$(
    jq -r '
        .assets[]
        | select(
            .name == "ferriso-updates.json"
            or .name == "SHA256SUMS"
            or (.name | endswith(".AppImage"))
            or (.name | endswith(".deb"))
            or (.name | endswith(".tar.gz"))
            or (.name | endswith(".exe"))
        )
        | [.name, .browser_download_url]
        | @tsv
    ' <<<"$release"
)"

while IFS=$'\t' read -r name url; do
    [ -n "$name" ] || continue
    mkdir -p "$tmp/assets"
    curl -fsSL --retry 3 --max-time 180 -o "$tmp/assets/$name" "$url"
done <<<"$asset_urls"

jq --arg base "$BASE_URL/$tag" '
    def local_url: ($base + "/" + (split("/") | last));
    .platforms |= with_entries(
        .value.url |= local_url
        | .value.install_urls |= map(local_url)
    )
' "$tmp/assets/ferriso-updates.json" >"$tmp/ferriso-updates.json"
rm -f "$tmp/assets/ferriso-updates.json"

jq -e --arg version "$version" '.version == $version' \
    "$tmp/ferriso-updates.json" >/dev/null

install -d -m 755 "$ROOT/$tag"
mv "$tmp/assets/"* "$ROOT/$tag/"
chmod 644 "$ROOT/$tag/"*
chmod 644 "$tmp/ferriso-updates.json"
mv "$tmp/ferriso-updates.json" "$ROOT/ferriso-updates.json"

logger -t ferriso-updates "published $version from $REPO"
