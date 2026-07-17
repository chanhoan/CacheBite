#!/usr/bin/env python3
"""Build bundled CacheBite pet packages from the UI design source images."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

from PIL import Image, ImageChops


PROJECT_ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOT = PROJECT_ROOT / "docs" / "UI-plan" / "assets" / "pet"
OUTPUT_ROOT = PROJECT_ROOT / "src-tauri" / "resources" / "pets"
PET_SOURCES = {"cat": "cat", "corgi": "corgi"}
STATE_SOURCES = {
    "idle": "idle",
    "idle_warn": "warn",
    "idle_critical": "critical",
    "idle_exhausted": "exhausted",
}
FRAME_COUNT = 4
FRAME_DURATION_MS = 240
OUTPUT_SIZE = (512, 512)
WHITE_THRESHOLD = 247


def keyed_frame(source: Path) -> Image.Image:
    """Resize a source frame and make near-white pixels transparent."""
    with Image.open(source) as image:
        rgba = image.convert("RGBA").resize(OUTPUT_SIZE, Image.Resampling.LANCZOS)
    red, green, blue, alpha = rgba.split()
    threshold = lambda value: 255 if value >= WHITE_THRESHOLD else 0
    near_white = ImageChops.multiply(
        ImageChops.multiply(red.point(threshold), green.point(threshold)),
        blue.point(threshold),
    )
    return Image.merge("RGBA", (red, green, blue, ImageChops.subtract(alpha, near_white)))


def animation(package_id: str, state: str, source_state: str) -> dict[str, object]:
    return {
        "type": "frames",
        "frames": [
            f"frames/{package_id}_{source_state}_{index:02}.png"
            for index in range(1, FRAME_COUNT + 1)
        ],
        "frameDurationMs": FRAME_DURATION_MS,
    }


def build_package(package_id: str, source_pet: str) -> None:
    package_root = OUTPUT_ROOT / package_id
    if package_root.exists():
        shutil.rmtree(package_root)
    frames_root = package_root / "frames"
    frames_root.mkdir(parents=True)

    for state, source_state in STATE_SOURCES.items():
        sources = sorted((SOURCE_ROOT / source_pet / source_state).glob("*.png"))
        if len(sources) != FRAME_COUNT:
            raise ValueError(
                f"expected {FRAME_COUNT} PNG frames for {source_pet}/{source_state}, "
                f"found {len(sources)}"
            )
        for index, source in enumerate(sources, start=1):
            destination = frames_root / f"{package_id}_{source_state}_{index:02}.png"
            keyed_frame(source).save(destination, format="PNG", optimize=True)

    manifest = {
        "id": package_id,
        "displayName": package_id.title(),
        "defaultSize": {"width": 128, "height": 128},
        "animations": {
            state: animation(package_id, state, source_state)
            for state, source_state in STATE_SOURCES.items()
        },
        "states": {state: state for state in STATE_SOURCES},
    }
    (package_root / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


def main() -> None:
    for package_id, source_pet in PET_SOURCES.items():
        build_package(package_id, source_pet)


if __name__ == "__main__":
    main()
