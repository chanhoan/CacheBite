from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw


PROJECT_ROOT = Path(__file__).resolve().parents[1]
BUILD_SCRIPT = PROJECT_ROOT / "scripts" / "build-pet-packages.py"
SOURCE_ROOT = PROJECT_ROOT / "docs" / "UI-plan" / "assets" / "pet"
RUNTIME_ROOT = PROJECT_ROOT / "src-tauri" / "resources" / "pets"


def load_build_script():
    spec = importlib.util.spec_from_file_location("build_pet_packages", BUILD_SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("failed to load build script")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PetBackgroundCleanupTest(unittest.TestCase):
    def save_image(self, image: Image.Image) -> Path:
        tempdir = tempfile.TemporaryDirectory()
        self.addCleanup(tempdir.cleanup)
        path = Path(tempdir.name) / "source.png"
        image.save(path)
        return path

    def test_keyed_frame_clears_only_edge_connected_background(self) -> None:
        build = load_build_script()

        source = Image.new("RGBA", (512, 512), (245, 245, 245, 255))
        draw = ImageDraw.Draw(source)
        draw.rectangle((128, 128, 383, 383), fill=(96, 96, 96, 255))
        draw.rectangle((224, 224, 287, 287), fill=(250, 250, 250, 255))

        result = build.keyed_frame(self.save_image(source))

        self.assertEqual(result.mode, "RGBA")
        self.assertEqual(result.size, (512, 512))
        self.assertEqual(result.getpixel((0, 0))[3], 0)
        self.assertEqual(result.getpixel((255, 255))[3], 255)
        self.assertEqual(result.getpixel((255, 255))[:3], (250, 250, 250))

    def test_keyed_frame_preserves_existing_transparency(self) -> None:
        build = load_build_script()

        source = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        draw = ImageDraw.Draw(source)
        draw.rectangle((192, 192, 319, 319), fill=(255, 255, 255, 255))

        result = build.keyed_frame(self.save_image(source))

        self.assertEqual(result.mode, "RGBA")
        self.assertEqual(result.size, (512, 512))
        self.assertEqual(result.getpixel((0, 0))[3], 0)
        self.assertEqual(result.getpixel((255, 255))[3], 255)
        self.assertEqual(result.getpixel((255, 255))[:3], (255, 255, 255))

    def test_keyed_frame_applies_fringe_alpha_curve(self) -> None:
        build = load_build_script()

        source = Image.new("RGBA", (512, 512), (245, 245, 245, 255))
        source.putpixel((2, 2), (230, 230, 230, 255))

        result = build.keyed_frame(self.save_image(source))

        self.assertEqual(result.getpixel((0, 0))[3], 0)
        self.assertEqual(result.getpixel((2, 2))[3], 240)

    def test_keyed_frame_falls_back_without_numpy_or_scipy(self) -> None:
        build = load_build_script()
        build.np = None
        build.ndi = None

        source = Image.new("RGBA", (512, 512), (245, 245, 245, 255))
        draw = ImageDraw.Draw(source)
        draw.rectangle((128, 128, 383, 383), fill=(96, 96, 96, 255))
        draw.rectangle((224, 224, 287, 287), fill=(250, 250, 250, 255))

        result = build.keyed_frame(self.save_image(source))

        self.assertEqual(result.getpixel((0, 0))[3], 0)
        self.assertEqual(result.getpixel((255, 255))[3], 255)

    def test_cleaned_source_frames_have_transparent_corners(self) -> None:
        sources = [
            source
            for source in sorted(SOURCE_ROOT.rglob("*.png"))
            if source.relative_to(SOURCE_ROOT).parts[:2] != ("cat", "critical")
        ]

        self.assertEqual(len(sources), 28)
        for source in sources:
            with self.subTest(source=source.relative_to(PROJECT_ROOT)):
                with Image.open(source) as image:
                    self.assertEqual(image.size, (512, 512))
                    alpha = image.convert("RGBA").getchannel("A")
                    self.assertEqual(
                        [
                            alpha.getpixel((0, 0)),
                            alpha.getpixel((511, 0)),
                            alpha.getpixel((0, 511)),
                            alpha.getpixel((511, 511)),
                        ],
                        [0, 0, 0, 0],
                    )

    def test_generated_runtime_frames_are_transparent_rgba(self) -> None:
        frames = sorted(RUNTIME_ROOT.glob("*/frames/*.png"))

        self.assertEqual(len(frames), 32)
        for frame in frames:
            with self.subTest(frame=frame.relative_to(PROJECT_ROOT)):
                with Image.open(frame) as image:
                    self.assertEqual(image.size, (512, 512))
                    alpha = image.convert("RGBA").getchannel("A")
                    self.assertEqual(alpha.getextrema()[0], 0)
                    self.assertEqual(
                        [
                            alpha.getpixel((0, 0)),
                            alpha.getpixel((511, 0)),
                            alpha.getpixel((0, 511)),
                            alpha.getpixel((511, 511)),
                        ],
                        [0, 0, 0, 0],
                    )


if __name__ == "__main__":
    unittest.main()
