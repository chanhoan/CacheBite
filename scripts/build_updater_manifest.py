#!/usr/bin/env python3
"""Build a `tauri-plugin-updater` channel manifest from a GitHub release.

The manifest is a static JSON file uploaded to a fixed-tag `updater` release, so
clients read it from a constant CDN URL with no API rate limit and no CacheBite
server in the path.

Fail-closed rules, in order of importance:

* A present artifact with no matching `.sig` is a hard error. Emitting an entry
  the client cannot verify is worse than emitting nothing.
* A newer manifest is never replaced by an older release (`--previous`), so a
  human publishing an old draft after a new one cannot offer a downgrade.
* A missing *platform* is a warning, not an error — but see `PLATFORM_GAP_NOTE`.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

MAX_NOTES_CHARS = 400

# `Updater::get_urls` searches `{os}-{arch}-{installer}` then `{os}-{arch}` and
# stops. There is no `darwin-universal` fallback, which is why one universal
# archive has to be listed under both macOS architectures.
ASSET_RULES: list[tuple[str, tuple[str, ...]]] = [
    ("-setup.exe", ("windows-x86_64-nsis", "windows-x86_64")),
    (".msi", ("windows-x86_64-msi",)),
    ("universal.app.tar.gz", ("darwin-aarch64", "darwin-x86_64")),
    (".appimage", ("linux-x86_64-appimage", "linux-x86_64")),
]

EXPECTED_KEYS = (
    "windows-x86_64",
    "windows-x86_64-nsis",
    "windows-x86_64-msi",
    "darwin-aarch64",
    "darwin-x86_64",
    "linux-x86_64",
    "linux-x86_64-appimage",
)

PLATFORM_GAP_NOTE = (
    "A missing key makes EVERY check on that platform return TargetsNotFound - "
    "including checks where the client already matches this version, which the "
    "user then sees as 'No update is published for this platform yet' rather "
    "than 'Up to date'. Re-run the release job for that platform."
)


class ManifestError(RuntimeError):
    """A condition that must stop the workflow rather than publish a bad file."""


def truncate_notes(notes: str) -> str:
    """Cap the release body so a changelog cannot resize the panel."""
    trimmed = notes.strip()
    if len(trimmed) <= MAX_NOTES_CHARS:
        return trimmed
    return trimmed[:MAX_NOTES_CHARS] + "…"


def platform_keys(asset_name: str) -> tuple[str, ...]:
    """Which manifest keys an asset satisfies, if any."""
    lowered = asset_name.lower()
    # `.sig` files are inputs, never entries.
    if lowered.endswith(".sig"):
        return ()
    for suffix, keys in ASSET_RULES:
        if lowered.endswith(suffix):
            return keys
    return ()


def read_signature(signatures: Path, asset_name: str) -> str:
    """The verbatim contents of `<asset>.sig`, which is what the client checks."""
    candidate = signatures / f"{asset_name}.sig"
    if not candidate.is_file():
        raise ManifestError(
            f"{asset_name} was published without {candidate.name}; refusing to "
            "emit an entry the client cannot verify"
        )
    signature = candidate.read_text(encoding="utf-8").strip()
    if not signature:
        raise ManifestError(f"{candidate.name} is empty")
    return signature


def parse_version(version: str) -> tuple[tuple[int, ...], bool, str]:
    """Enough of semver to order two of our own tags.

    Returns `(numeric core, is_release, pre-release)`. A release outranks a
    pre-release with the same core, matching semver's own rule.
    """
    match = re.fullmatch(
        r"(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.\-]+))?(?:\+.*)?", version
    )
    if not match:
        raise ManifestError(f"{version!r} is not a version this script can order")
    core = tuple(int(part) for part in match.group(1, 2, 3))
    pre = match.group(4) or ""
    return core, pre == "", pre


def _pre_key(pre: str) -> list[tuple[int, object]]:
    key: list[tuple[int, object]] = []
    for part in pre.split(".") if pre else []:
        # Numeric identifiers always rank below alphanumeric ones (semver 11.4.3).
        key.append((0, int(part)) if part.isdigit() else (1, part))
    return key


def is_newer(candidate: str, existing: str) -> bool:
    """Whether `candidate` should replace `existing` in a channel manifest."""
    candidate_core, candidate_release, candidate_pre = parse_version(candidate)
    existing_core, existing_release, existing_pre = parse_version(existing)
    if candidate_core != existing_core:
        return candidate_core > existing_core
    if candidate_release != existing_release:
        return candidate_release
    # Dotted pre-release identifiers compare numerically where both are numeric,
    # which is what orders beta.9 below beta.10.
    return _pre_key(candidate_pre) > _pre_key(existing_pre)


def build_platforms(assets: list[dict], signatures: Path) -> dict[str, dict[str, str]]:
    platforms: dict[str, dict[str, str]] = {}
    for asset in assets:
        name = asset["name"]
        keys = platform_keys(name)
        if not keys:
            continue
        entry = {
            "signature": read_signature(signatures, name),
            "url": asset["browser_download_url"],
        }
        for key in keys:
            platforms[key] = entry
    return platforms


def build_manifest(
    version: str,
    notes: str,
    pub_date: str,
    assets: list[dict],
    signatures: Path,
) -> dict:
    platforms = build_platforms(assets, signatures)
    if not platforms:
        raise ManifestError("no installable artifact was found in the release")
    return {
        "version": version,
        "notes": truncate_notes(notes),
        "pub_date": pub_date,
        "platforms": platforms,
    }


def report_gaps(platforms: dict[str, dict[str, str]]) -> None:
    missing = [key for key in EXPECTED_KEYS if key not in platforms]
    if not missing:
        return
    # Loud on purpose: a one-off upload miss must be visible in the run summary
    # rather than surfacing months later as a bug report.
    print(
        f"::warning::updater manifest is missing {', '.join(missing)}",
        file=sys.stderr,
    )
    print(f"::warning::{PLATFORM_GAP_NOTE}", file=sys.stderr)


def load_previous_version(previous: Path | None) -> str | None:
    if previous is None or not previous.is_file():
        return None
    try:
        return json.loads(previous.read_text(encoding="utf-8"))["version"]
    except (json.JSONDecodeError, KeyError, TypeError):
        # An unreadable previous manifest must not block a good release; the
        # regression guard simply has nothing to compare against.
        print(
            "::warning::the previous manifest was unreadable; skipping the regression guard",
            file=sys.stderr,
        )
        return None


def run(args: argparse.Namespace) -> int:
    assets = json.loads(Path(args.assets).read_text(encoding="utf-8"))
    notes = Path(args.notes_file).read_text(encoding="utf-8") if args.notes_file else ""
    out = Path(args.out)
    previous_version = load_previous_version(
        Path(args.previous) if args.previous else None
    )

    if previous_version is not None and not is_newer(args.version, previous_version):
        print(
            f"{out.name} already advertises {previous_version}, which is not older "
            f"than {args.version}; leaving it untouched"
        )
        return 0

    manifest = build_manifest(
        args.version, notes, args.pub_date, assets, Path(args.signatures)
    )
    report_gaps(manifest["platforms"])
    out.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(
        f"wrote {out} for {args.version} with {len(manifest['platforms'])} platform keys"
    )
    return 0


# --- self test ------------------------------------------------------------


def _self_test() -> int:
    import tempfile

    failures: list[str] = []

    def check(label: str, condition: bool) -> None:
        if not condition:
            failures.append(label)

    check(
        "nsis maps to both windows keys",
        platform_keys("CacheBite_0.1.0_x64-setup.exe")
        == ("windows-x86_64-nsis", "windows-x86_64"),
    )
    check(
        "msi maps to its own key",
        platform_keys("CacheBite_0.1.0_x64_en-US.msi") == ("windows-x86_64-msi",),
    )
    check(
        "a universal archive covers both mac arches",
        platform_keys("CacheBite_0.1.0_universal.app.tar.gz")
        == ("darwin-aarch64", "darwin-x86_64"),
    )
    check(
        "appimage maps to both linux keys",
        platform_keys("CacheBite_0.1.0_amd64.AppImage")
        == ("linux-x86_64-appimage", "linux-x86_64"),
    )
    check(
        "signatures are never entries",
        platform_keys("CacheBite_0.1.0_x64-setup.exe.sig") == (),
    )
    check("unrelated assets are ignored", platform_keys("SHA256SUMS.txt") == ())

    check("a release outranks its pre-release", is_newer("0.1.0", "0.1.0-beta.4"))
    check(
        "a pre-release never outranks its release", not is_newer("0.1.0-beta.4", "0.1.0")
    )
    check("beta.10 outranks beta.9", is_newer("0.1.0-beta.10", "0.1.0-beta.9"))
    check("the same version is not newer", not is_newer("0.1.0", "0.1.0"))
    check("an older patch is not newer", not is_newer("0.1.0", "0.1.1"))
    check("a newer minor is newer", is_newer("0.2.0", "0.1.9"))

    with tempfile.TemporaryDirectory() as raw:
        signatures = Path(raw)
        (signatures / "CacheBite_0.1.0_universal.app.tar.gz.sig").write_text(
            "SIG-MAC", encoding="utf-8"
        )
        assets = [
            {
                "name": "CacheBite_0.1.0_universal.app.tar.gz",
                "browser_download_url": "https://example.invalid/mac.tar.gz",
            }
        ]

        manifest = build_manifest(
            "0.1.0", "  notes  ", "2026-08-04T12:00:00Z", assets, signatures
        )
        check(
            "both mac keys share one entry",
            manifest["platforms"]["darwin-aarch64"]
            == manifest["platforms"]["darwin-x86_64"],
        )
        check(
            "the signature is verbatim",
            manifest["platforms"]["darwin-x86_64"]["signature"] == "SIG-MAC",
        )
        check("notes are trimmed", manifest["notes"] == "notes")

        long_notes = build_manifest(
            "0.1.0", "x" * 5000, "2026-08-04T12:00:00Z", assets, signatures
        )
        check("notes are capped", len(long_notes["notes"]) == MAX_NOTES_CHARS + 1)

        unsigned = [
            {
                "name": "CacheBite_0.1.0_x64-setup.exe",
                "browser_download_url": "https://example.invalid/setup.exe",
            }
        ]
        try:
            build_manifest("0.1.0", "", "2026-08-04T12:00:00Z", unsigned, signatures)
            failures.append("a missing signature must be fatal")
        except ManifestError:
            pass

        try:
            build_manifest("0.1.0", "", "2026-08-04T12:00:00Z", [], signatures)
            failures.append("an empty release must be fatal")
        except ManifestError:
            pass

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print("build_updater_manifest self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test", action="store_true", help="run the built-in assertions and exit"
    )
    parser.add_argument("--version", help="release version without the leading v")
    parser.add_argument("--notes-file", help="file holding the release body")
    parser.add_argument("--pub-date", help="RFC 3339 publication timestamp")
    parser.add_argument("--assets", help="JSON array of {name, browser_download_url}")
    parser.add_argument("--signatures", help="directory of downloaded .sig files")
    parser.add_argument("--out", help="manifest path to write")
    parser.add_argument(
        "--previous", help="existing manifest guarding against a downgrade"
    )
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    required = ("version", "pub_date", "assets", "signatures", "out")
    missing = [name for name in required if not getattr(args, name)]
    if missing:
        parser.error(
            "missing required arguments: "
            + ", ".join(f"--{name.replace('_', '-')}" for name in missing)
        )

    try:
        return run(args)
    except ManifestError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
