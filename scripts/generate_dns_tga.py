#!/usr/bin/env python3
"""Render DNS tester SVG backgrounds to RGB TGA and record dynamic field slots.

The source SVGs intentionally include preview values for design review. This
script removes only the values that the application owns, leaving fixed words,
panel artwork, and control labels in the bitmap. It writes:

* `crates/device-envoy-core/docs/assets/dns_landscape.tga` (320x240)
* `crates/device-envoy-core/docs/assets/dns_portrait.tga` (240x320)
* `crates/device-envoy-core/docs/assets/dns_tga_layout.json`

Requirements: `ffmpeg` with SVG input support and Pillow (`python3 -m pip
install Pillow` when it is not already available).
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as element_tree
from pathlib import Path

from PIL import Image


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
ASSET_DIRECTORY = REPOSITORY_ROOT / "crates/device-envoy-core/docs/assets"
SVG_NAMESPACE = "{http://www.w3.org/2000/svg}"

# Values shown only in the SVG mockups. Fixed labels such as TARGET, QUERIES,
# SUCCESS, FAILED, and LAST LOOKUP deliberately remain in the TGA.
PREVIEW_TEXT = {"READY", "example.com", "22 ms", "OK", "1", "0"}

LAYOUTS = {
    "landscape": {
        "size": [320, 240],
        "fields": {
            "status": {"clear_rect": [246, 15, 58, 18], "baseline": [263, 28]},
            "target": {"clear_rect": [20, 76, 210, 22], "baseline": [22, 92]},
            "latency": {"clear_rect": [88, 92, 144, 36], "baseline": [160, 121], "anchor": "middle"},
            "lookup_status": {"clear_rect": [250, 82, 44, 24], "baseline": [272, 102], "anchor": "middle"},
            "queries": {"clear_rect": [38, 160, 28, 20], "baseline": [52, 176], "anchor": "middle"},
            "successes": {"clear_rect": [146, 160, 28, 20], "baseline": [160, 176], "anchor": "middle"},
            "failures": {"clear_rect": [254, 160, 28, 20], "baseline": [268, 176], "anchor": "middle"},
        },
    },
    "portrait": {
        "size": [240, 320],
        "fields": {
            "status": {"clear_rect": [168, 15, 58, 18], "baseline": [185, 28]},
            "target": {"clear_rect": [20, 72, 200, 22], "baseline": [22, 88]},
            "latency": {"clear_rect": [48, 112, 144, 36], "baseline": [120, 142], "anchor": "middle"},
            "queries": {"clear_rect": [184, 184, 28, 20], "baseline": [210, 200], "anchor": "end"},
            "successes": {"clear_rect": [184, 206, 28, 20], "baseline": [210, 222], "anchor": "end"},
            "failures": {"clear_rect": [184, 224, 28, 20], "baseline": [210, 240], "anchor": "end"},
        },
    },
}


def local_name(element: element_tree.Element) -> str:
    """Return an XML element's namespace-free name."""
    return element.tag.removeprefix(SVG_NAMESPACE)


def strip_preview_values(svg_path: Path, output_path: Path) -> None:
    """Copy an SVG without preview-only values and its top status indicator."""
    root = element_tree.parse(svg_path).getroot()

    for parent in root.iter():
        for child in list(parent):
            if local_name(child) == "text" and (child.text or "").strip() in PREVIEW_TEXT:
                parent.remove(child)

    # The only root-level success circle is the preview READY indicator. Control
    # icons also contain circles, but are nested in groups and must stay.
    for child in list(root):
        if local_name(child) == "circle" and child.get("class") == "success":
            root.remove(child)

    element_tree.register_namespace("", "http://www.w3.org/2000/svg")
    element_tree.ElementTree(root).write(output_path, encoding="utf-8", xml_declaration=True)


def render_tga(svg_path: Path, tga_path: Path, expected_size: tuple[int, int]) -> None:
    """Rasterize one SVG with ffmpeg and encode an uncompressed 24-bit TGA."""
    with tempfile.TemporaryDirectory() as temporary_directory:
        sanitized_svg = Path(temporary_directory) / "background.svg"
        png_path = Path(temporary_directory) / "background.png"
        strip_preview_values(svg_path, sanitized_svg)
        subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", sanitized_svg, "-frames:v", "1", png_path],
            check=True,
        )
        with Image.open(png_path) as image:
            if image.size != expected_size:
                raise ValueError(f"{svg_path} rendered as {image.size}, expected {expected_size}")
            image.convert("RGB").save(tga_path, compression="raw")


def main() -> int:
    """Generate both static backgrounds and their dynamic-value layout map."""
    if shutil.which("ffmpeg") is None:
        print("ffmpeg is required to rasterize the SVG assets", file=sys.stderr)
        return 1

    for orientation, layout in LAYOUTS.items():
        svg_path = ASSET_DIRECTORY / f"dns_{orientation}.svg"
        tga_path = ASSET_DIRECTORY / f"dns_{orientation}.tga"
        render_tga(svg_path, tga_path, tuple(layout["size"]))
        print(f"wrote {tga_path.relative_to(REPOSITORY_ROOT)}")

    layout_path = ASSET_DIRECTORY / "dns_tga_layout.json"
    layout_path.write_text(json.dumps(LAYOUTS, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {layout_path.relative_to(REPOSITORY_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
