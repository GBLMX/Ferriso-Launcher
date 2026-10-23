#!/usr/bin/env python3
"""Publish stable, beta, and alpha update feeds from release directories.

`--local` scans `FERRISO_UPDATE_ROOT/ferriso-v*` for staged releases and writes
the three channel manifests without network access. This is the mode used
after the release workflow uploads artifacts:

    FERRISO_UPDATE_ROOT=/var/www/ferriso-updates \
    apps/app-ferriso/packaging/sync-updates.py --local

Without `--local`, the script reads GitHub releases and downloads their assets
for manual mirror recovery:

    FERRISO_UPDATE_ROOT=/var/www/ferriso-updates \
    FERRISO_UPDATE_BASE_URL=https://updates.bionickernel.com \
    apps/app-ferriso/packaging/sync-updates.py

Each channel selects the newest release it is allowed to run: stable for the
stable feed, stable/beta for beta, and stable/beta/alpha for alpha. Assets and
manifests are checked before any channel file is replaced, so clients never see
a feed pointing at missing files.
"""

from __future__ import annotations

import fcntl
import json
import os
import re
import shutil
import sys
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path

REPO = os.environ.get("FERRISO_REPO", "GBLMX/Ferriso-Launcher")
BASE_URL = os.environ.get(
    "FERRISO_UPDATE_BASE_URL", "https://updates.bionickernel.com"
).rstrip("/")
ROOT = Path(
    os.environ.get("FERRISO_UPDATE_ROOT", "/var/www/ferriso-updates")
)
LOCK = Path(
    os.environ.get(
        "FERRISO_UPDATE_LOCK", "/run/ferriso-update-sync.lock"
    )
)
DRY_RUN = os.environ.get("FERRISO_UPDATE_DRY_RUN") == "1"
API = f"https://api.github.com/repos/{REPO}/releases?per_page=100"
USER_AGENT = "FerrisoUpdateMirror/1.0"
SEMVER = re.compile(
    r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$"
)
CHANNELS = ("stable", "beta", "alpha")
MANIFESTS = {
    "stable": ROOT / "ferriso-updates.json",
    "beta": ROOT / "ferriso-updates-beta.json",
    "alpha": ROOT / "ferriso-updates-alpha.json",
}


@dataclass(frozen=True)
class Release:
    tag: str
    version: str
    channel: str
    assets: dict[str, str]


def fetch_json(url: str) -> object:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": USER_AGENT,
        },
    )
    last_error: Exception | None = None
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.load(response)
        except (OSError, ValueError) as error:
            last_error = error
            if attempt < 2:
                time.sleep(2 * (attempt + 1))
    assert last_error is not None
    raise last_error


def download(url: str, destination: Path) -> None:
    if destination.exists() and destination.stat().st_size > 0:
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_name(destination.name + ".part")
    request = urllib.request.Request(
        url, headers={"User-Agent": USER_AGENT}
    )
    last_error: OSError | None = None
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=600) as source:
                with partial.open("wb") as target:
                    shutil.copyfileobj(source, target)
            os.replace(partial, destination)
            destination.chmod(0o644)
            return
        except OSError as error:
            last_error = error
            partial.unlink(missing_ok=True)
            if attempt < 2:
                time.sleep(2 * (attempt + 1))
    assert last_error is not None
    raise last_error


def version_key(version: str) -> tuple[object, ...]:
    match = SEMVER.fullmatch(version)
    if match is None:
        raise ValueError(f"invalid release version: {version}")
    main = tuple(int(part) for part in match.group(1, 2, 3))
    prerelease = match.group(4)
    if prerelease is None:
        return (main, 1, ())
    identifiers = tuple(
        (0, int(part)) if part.isdigit() else (1, part)
        for part in prerelease.split(".")
    )
    return (main, 0, identifiers)


def channel_for_version(version: str) -> str:
    match = SEMVER.fullmatch(version)
    if match is None:
        raise ValueError(f"invalid release version: {version}")
    prerelease = match.group(4)
    if prerelease is None:
        return "stable"
    if prerelease.startswith("alpha"):
        return "alpha"
    # `rc` and any future pre-release kind are beta-visible.
    return "beta"


def parse_release(raw: dict[str, object]) -> Release | None:
    if raw.get("draft"):
        return None
    tag = str(raw.get("tag_name", ""))
    if not tag.startswith("ferriso-v"):
        return None
    version = tag.removeprefix("ferriso-v")
    try:
        channel = channel_for_version(version)
    except ValueError:
        return None
    assets = {
        str(asset["name"]): str(asset["browser_download_url"])
        for asset in raw.get("assets", [])
    }
    return Release(tag, version, channel, assets)


def latest_for(releases: list[Release], channel: str) -> Release | None:
    allowed = {
        "stable": {"stable"},
        "beta": {"stable", "beta"},
        "alpha": {"stable", "beta", "alpha"},
    }[channel]
    candidates = [
        release for release in releases if release.channel in allowed
    ]
    return max(
        candidates, key=lambda release: version_key(release.version),
        default=None,
    )


def local_name(url: str) -> str:
    return urllib.parse.unquote(
        urllib.parse.urlparse(url).path.rsplit("/", 1)[-1]
    )


def rewrite_manifest(manifest: dict[str, object], release: Release) -> dict[str, object]:
    result = json.loads(json.dumps(manifest))
    platforms = result.get("platforms", {})
    if not isinstance(platforms, dict):
        raise ValueError(f"{release.tag}: manifest has no platforms")
    for platform in platforms.values():
        if not isinstance(platform, dict):
            continue
        url = platform.get("url")
        if isinstance(url, str) and url:
            platform["url"] = f"{BASE_URL}/{release.tag}/{local_name(url)}"
        install_urls = platform.get("install_urls")
        if isinstance(install_urls, list):
            platform["install_urls"] = [
                f"{BASE_URL}/{release.tag}/{local_name(url)}"
                if isinstance(url, str) and url
                else url
                for url in install_urls
            ]
    return result


def manifest_asset_names(manifest: dict[str, object]) -> set[str]:
    names: set[str] = set()
    platforms = manifest.get("platforms", {})
    if not isinstance(platforms, dict):
        return names
    for platform in platforms.values():
        if not isinstance(platform, dict):
            continue
        for key in ("url", "install_urls"):
            value = platform.get(key)
            urls = value if isinstance(value, list) else [value]
            for url in urls:
                if isinstance(url, str) and url:
                    names.add(local_name(url))
    return names


def manifest_version(path: Path) -> str | None:
    if not path.exists():
        return None
    try:
        value = json.loads(path.read_text()).get("version")
    except (OSError, ValueError):
        return None
    return value if isinstance(value, str) else None


def load_local_releases() -> list[Release]:
    releases: list[Release] = []
    if not ROOT.is_dir():
        raise RuntimeError(f"mirror root is missing: {ROOT}")
    for directory in ROOT.iterdir():
        if not directory.is_dir() or not directory.name.startswith(
            "ferriso-v"
        ):
            continue
        version = directory.name.removeprefix("ferriso-v")
        manifest_path = directory / "ferriso-updates.json"
        if not manifest_path.is_file():
            continue
        try:
            channel = channel_for_version(version)
            manifest = json.loads(manifest_path.read_text())
        except (OSError, ValueError):
            continue
        if not isinstance(manifest, dict):
            continue
        if manifest.get("version") != version:
            continue
        releases.append(
            Release(directory.name, version, channel, {})
        )
    return releases


def write_manifest(path: Path, manifest: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(path.name + ".part")
    partial.write_text(json.dumps(manifest, indent=2) + "\n")
    partial.chmod(0o644)
    os.replace(partial, path)


def publish_local() -> dict[str, Release | None]:
    releases = load_local_releases()
    targets = {
        channel: latest_for(releases, channel) for channel in CHANNELS
    }
    pending: dict[str, Release | None] = {}
    for channel, target in targets.items():
        path = MANIFESTS[channel]
        if target is None:
            if channel != "stable" and path.exists():
                pending[channel] = None
        elif manifest_version(path) != target.version:
            pending[channel] = target

    for channel, target in pending.items():
        path = MANIFESTS[channel]
        if target is None:
            path.unlink(missing_ok=True)
            continue
        raw = json.loads(
            (ROOT / target.tag / "ferriso-updates.json").read_text()
        )
        if not isinstance(raw, dict) or raw.get("version") != target.version:
            raise RuntimeError(f"{target.tag}: invalid local manifest")
        manifest = rewrite_manifest(raw, target)
        for name in manifest_asset_names(manifest):
            if not (ROOT / target.tag / name).is_file():
                raise RuntimeError(
                    f"{target.tag}: manifest references missing asset {name}"
                )
        write_manifest(path, manifest)
    return pending


def publish(releases: list[Release]) -> dict[str, Release | None]:
    targets = {
        channel: latest_for(releases, channel) for channel in CHANNELS
    }
    pending: dict[str, Release | None] = {}
    for channel, target in targets.items():
        path = MANIFESTS[channel]
        if target is None:
            if channel != "stable" and path.exists():
                pending[channel] = None
        elif manifest_version(path) != target.version:
            pending[channel] = target

    if not pending:
        return pending

    manifests: dict[str, dict[str, object]] = {}
    selected = {
        target.tag: target for target in pending.values() if target
    }
    for release in selected.values():
        if "ferriso-updates.json" not in release.assets:
            raise RuntimeError(
                f"{release.tag} has no ferriso-updates.json asset"
            )
        manifest_url = release.assets["ferriso-updates.json"]
        raw = fetch_json(manifest_url)
        if not isinstance(raw, dict):
            raise ValueError(f"{release.tag}: manifest is not an object")
        if raw.get("version") != release.version:
            raise ValueError(
                f"{release.tag}: manifest version {raw.get('version')} "
                f"does not match {release.version}"
            )
        manifests[release.tag] = rewrite_manifest(raw, release)
        for name in manifest_asset_names(manifests[release.tag]):
            asset_url = release.assets.get(name)
            if asset_url is None:
                raise RuntimeError(
                    f"{release.tag}: manifest references missing asset {name}"
                )
            download(asset_url, ROOT / release.tag / name)
        checksum_url = release.assets.get("SHA256SUMS")
        if checksum_url:
            download(checksum_url, ROOT / release.tag / "SHA256SUMS")

    for channel, target in pending.items():
        path = MANIFESTS[channel]
        if target is None:
            path.unlink(missing_ok=True)
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        partial = path.with_name(path.name + ".part")
        partial.write_text(
            json.dumps(manifests[target.tag], indent=2) + "\n"
        )
        partial.chmod(0o644)
        os.replace(partial, path)
    return pending


def main() -> int:
    if "--local" in sys.argv[1:]:
        try:
            changed = publish_local()
        except (OSError, ValueError, KeyError, RuntimeError) as error:
            print(f"local publish failed: {error}", file=sys.stderr)
            return 1
        for channel, target in changed.items():
            if target is not None:
                print(f"{channel}: {target.tag}")
        return 0

    try:
        releases = [
            release
            for raw in fetch_json(API)
            if isinstance(raw, dict)
            for release in [parse_release(raw)]
            if release is not None
        ]
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"release list failed: {error}", file=sys.stderr)
        return 1

    targets = {channel: latest_for(releases, channel) for channel in CHANNELS}
    if DRY_RUN:
        for channel, target in targets.items():
            print(
                f"{channel}: {target.tag if target else 'none'}"
            )
        return 0

    ROOT.mkdir(parents=True, exist_ok=True)
    lock_handle = os.open(LOCK, os.O_CREAT | os.O_RDWR, 0o644)
    try:
        try:
            fcntl.flock(lock_handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return 0
        try:
            changed = publish(releases)
        except (OSError, ValueError, KeyError, RuntimeError) as error:
            print(f"publish failed: {error}", file=sys.stderr)
            return 1
    finally:
        os.close(lock_handle)

    for channel, target in changed.items():
        if target is not None:
            print(f"{channel}: {target.tag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
