#!/usr/bin/env python3
"""Split the momo 2x2 animation sheets into aligned 512x512 source frames.

The sheets under docs/UI-plan/uploads/momo-sheets/ are AI-generated: each one
carries the four frames of a single state, so all frames of a state share one
generation and stay on-model. This script turns them into the per-frame
source art under docs/UI-plan/assets/pet/momo/ that build-pet-packages.py
consumes.

Frames are assembled from connected components assigned by centroid, so a
neighbouring quadrant's overhanging hair or sock never leaks into a frame.
Scale and anchoring come from the body (largest component) alone — satellites
like Zzz, sweat drops, and trembling lines ride along at their original
offsets without skewing the alignment, which is what keeps the loop from
jittering.

Requires numpy + scipy (same optional stack as build-pet-packages.py, but
hard-required here). The generated frames are committed, so this only needs
to run again when the sheet art changes.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage as ndi

PROJECT_ROOT = Path(__file__).resolve().parents[1]
SHEETS = PROJECT_ROOT / "docs" / "UI-plan" / "uploads" / "momo-sheets"
OUT = PROJECT_ROOT / "docs" / "UI-plan" / "assets" / "pet" / "momo"
STATES = ["idle", "warn", "critical", "exhausted"]
CANVAS = 512
TARGET_BODY_HEIGHT = 460  # standing states
TARGET_BODY_WIDTH = 470  # the lying state scales by body width instead
BOTTOM_MARGIN = 16
MIN_COMPONENT_AREA = 40  # ignore antialiasing crumbs


# An enclosed region only counts as background when its mean colour sits this
# close to the measured outer-background mean (max per-channel difference).
# Measured on the actual sheets: enclosed background pockets differ by <= ~3,
# while the character's warm whites (hoodie highlights) differ by >= ~9.
ENCLOSED_COLOR_TOLERANCE = 5
# ...and when it is at least this many pixels. The pure-white star highlights
# in the eyes match the background colour exactly but are far smaller (~300px
# at sheet scale), so the area floor is what protects them.
ENCLOSED_MIN_AREA = 600


def key_background(rgba: np.ndarray) -> np.ndarray:
    """Clear the near-white background, keeping shadows.

    Two passes: everything connected to the sheet edge, then enclosed pockets
    (between hair strands, between the legs) that match the measured outer
    background colour — those are unreachable by the edge flood fill.
    """
    rgb = rgba[:, :, :3].astype(np.int16)
    mn = rgb.min(axis=2)
    chroma = rgb.max(axis=2) - mn
    candidate = (mn >= 225) & (chroma <= 14)
    labels, count = ndi.label(candidate)
    edge_labels = np.unique(
        np.concatenate([labels[0, :], labels[-1, :], labels[:, 0], labels[:, -1]])
    )
    background = candidate & np.isin(labels, edge_labels)

    bg_mean = rgb[background].mean(axis=0)
    enclosed = candidate & ~background
    enclosed_labels, enclosed_count = ndi.label(enclosed)
    for component in range(1, enclosed_count + 1):
        mask = enclosed_labels == component
        if mask.sum() < ENCLOSED_MIN_AREA:
            continue
        difference = np.abs(rgb[mask].mean(axis=0) - bg_mean).max()
        if difference <= ENCLOSED_COLOR_TOLERANCE:
            background |= mask

    fringe_zone = ndi.binary_dilation(background, iterations=2) & ~background
    fringe = fringe_zone & (mn >= 210) & (chroma <= 18)
    alpha = np.full(mn.shape, 255, dtype=np.uint8)
    alpha[background] = 0
    if fringe.any():
        alpha[fringe] = np.clip((240 - mn[fringe]) * 8, 0, 255).astype(np.uint8)
    out = rgba.copy()
    out[:, :, 3] = alpha
    return out


def component_bbox(mask: np.ndarray) -> tuple[int, int, int, int]:
    ys, xs = np.nonzero(mask)
    return xs.min(), ys.min(), xs.max() + 1, ys.max() + 1


def process_state(state: str) -> None:
    sheet_path = SHEETS / f"{state}-sheet.png"
    sheet = np.asarray(Image.open(sheet_path).convert("RGBA"), dtype=np.uint8)
    keyed = key_background(sheet)
    height, width = keyed.shape[:2]
    half_x, half_y = width / 2, height / 2

    opaque = keyed[:, :, 3] > 24
    # Bridge hairline antialiasing gaps so one character stays one component;
    # satellites (Zzz, sweat, trembling lines) stay separate on purpose.
    bridged = ndi.binary_closing(opaque, structure=np.ones((3, 3)), iterations=2)
    labels, count = ndi.label(bridged)

    # Quadrant index doubles as frame order: 0 TL, 1 TR, 2 BL, 3 BR.
    quadrant_components: dict[int, list[int]] = {0: [], 1: [], 2: [], 3: []}
    centroids = ndi.center_of_mass(bridged, labels, range(1, count + 1))
    areas = ndi.sum_labels(bridged, labels, range(1, count + 1))
    for component, ((cy, cx), area) in enumerate(zip(centroids, areas), start=1):
        if area < MIN_COMPONENT_AREA:
            continue
        quadrant = (0 if cx < half_x else 1) + (0 if cy < half_y else 2)
        quadrant_components[quadrant].append(component)

    out_dir = OUT / state
    out_dir.mkdir(parents=True, exist_ok=True)
    for quadrant in range(4):
        components = quadrant_components[quadrant]
        if not components:
            raise ValueError(f"{state}: quadrant {quadrant} has no components")
        body = max(components, key=lambda c: areas[c - 1])
        bx0, by0, bx1, by1 = component_bbox(labels == body)
        frame_mask = np.isin(labels, components)
        frame = keyed.copy()
        frame[:, :, 3] = np.where(frame_mask, keyed[:, :, 3], 0)

        scale = (
            TARGET_BODY_WIDTH / (bx1 - bx0)
            if state == "exhausted"
            else TARGET_BODY_HEIGHT / (by1 - by0)
        )
        fx0, fy0, fx1, fy1 = component_bbox(frame_mask)
        crop = Image.fromarray(frame, "RGBA").crop((fx0, fy0, fx1, fy1))
        resized = crop.resize(
            (max(1, round(crop.width * scale)), max(1, round(crop.height * scale))),
            Image.Resampling.LANCZOS,
        )
        body_cx = ((bx0 + bx1) / 2 - fx0) * scale
        body_bottom = (by1 - fy0) * scale
        x = round(CANVAS / 2 - body_cx)
        y = round(CANVAS - BOTTOM_MARGIN - body_bottom)
        canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
        canvas.paste(resized, (x, y), resized)
        canvas.save(out_dir / f"momo_{state}_{quadrant + 1:02}.png", optimize=True)
    print(f"{state}: 4 frames")


def main() -> None:
    for state in STATES:
        process_state(state)


if __name__ == "__main__":
    main()
