# Momo animation sheets

Source-of-truth art for the `momo` pet package. Each sheet is one state's
four animation frames laid out as a 2x2 grid (frame order: top-left,
top-right, bottom-left, bottom-right); `character-sheet.png` is the reference
pose the animation sheets were generated against, and is not used for frames.

## Provenance

The sheets are AI-generated (ChatGPT image generation, 2026-08-04) from a
text prompt describing the character and per-frame motion deltas; there is no
third-party source artwork. They were contributed by @gd-dg and are provided
under the repository's license terms once a project license is adopted.

The checked-in PNGs are palette-quantized (256 colours) copies of the raw
generations to keep the repository small; the raw outputs carried no
additional detail that survives pet-scale rendering.

## Regenerating frames

```bash
python3 scripts/split-momo-sheets.py   # sheets -> docs/UI-plan/assets/pet/momo/
python3 scripts/build-pet-packages.py  # sources -> src-tauri/resources/pets/
```

Replacing a sheet and re-running both scripts is the whole pipeline; frame
alignment and background keying are automatic.
