#!/usr/bin/env python3
"""Bundle CYD-related Core, ESP, and RP rustdoc as agent-readable Markdown.

The corpus includes portable CYD and button APIs, the memory and WASM CYD/button
implementations, the browser CYD shell and simulator, and ESP/RP CYD/button
implementations. It omits unrelated device APIs, rustdoc navigation, scripts,
embedded image data, synthetic auto traits, and blanket implementations.
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path

from bs4 import BeautifulSoup, NavigableString, Tag


@dataclass(frozen=True)
class Site:
    crate: str
    directory: Path


BLOCK_TAGS = {
    "article",
    "details",
    "div",
    "dl",
    "main",
    "section",
    "summary",
}
SKIP_IDS = {
    "blanket-implementations",
    "blanket-implementations-list",
    "synthetic-implementations",
    "synthetic-implementations-list",
}


def clean_inline(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def inline_markdown(node: Tag | NavigableString) -> str:
    if isinstance(node, NavigableString):
        return str(node)

    if node.name in {"button", "script", "style", "svg"}:
        return ""
    if node.name == "br":
        return "\n"

    text = "".join(inline_markdown(child) for child in node.children)
    text = clean_inline(text)
    if not text:
        return ""

    if node.name == "code":
        fence = "``" if "`" in text else "`"
        return f"{fence}{text}{fence}"
    if node.name in {"strong", "b"}:
        return f"**{text}**"
    if node.name in {"em", "i"}:
        return f"*{text}*"
    if node.name == "del":
        return f"~~{text}~~"
    return text


def list_markdown(tag: Tag, depth: int = 0) -> list[str]:
    lines: list[str] = []
    ordered = tag.name == "ol"
    item_index = 1
    for item in tag.find_all("li", recursive=False):
        inline_children = [
            child
            for child in item.children
            if not isinstance(child, Tag) or child.name not in {"ul", "ol"}
        ]
        text = clean_inline("".join(inline_markdown(child) for child in inline_children))
        marker = f"{item_index}." if ordered else "-"
        if text:
            lines.append(f"{'    ' * depth}{marker} {text}")
        for nested in item.find_all(["ul", "ol"], recursive=False):
            lines.extend(list_markdown(nested, depth + 1))
        item_index += 1
    return lines


def table_markdown(tag: Tag) -> list[str]:
    rows = []
    for row in tag.find_all("tr"):
        cells = [clean_inline(inline_markdown(cell)) for cell in row.find_all(["th", "td"])]
        if cells:
            rows.append(cells)
    if not rows:
        return []

    width = max(len(row) for row in rows)
    rows = [row + [""] * (width - len(row)) for row in rows]
    lines = ["| " + " | ".join(rows[0]) + " |"]
    lines.append("| " + " | ".join(["---"] * width) + " |")
    lines.extend("| " + " | ".join(row) + " |" for row in rows[1:])
    return lines


def render_blocks(parent: Tag, heading_shift: int = 2) -> list[str]:
    lines: list[str] = []
    for child in parent.children:
        if isinstance(child, NavigableString):
            text = clean_inline(str(child))
            if text:
                lines.extend([text, ""])
            continue
        if not isinstance(child, Tag):
            continue
        if child.get("id") in SKIP_IDS:
            continue
        if child.name in {"nav", "noscript", "script", "style"}:
            continue
        if "hideme" in child.get("class", []):
            continue

        if child.name in {"h1", "h2", "h3", "h4", "h5", "h6"}:
            level = min(6, int(child.name[1]) + heading_shift)
            text = clean_inline(inline_markdown(child))
            if text:
                lines.extend([f"{'#' * level} {text}", ""])
        elif child.name == "p":
            text = clean_inline(inline_markdown(child))
            if text:
                lines.extend([text, ""])
        elif child.name == "pre":
            text = child.get_text("", strip=False).strip("\n")
            if text:
                language = "rust" if "rust" in child.get("class", []) else "text"
                lines.extend([f"```{language}", text, "```", ""])
        elif child.name == "blockquote":
            text = clean_inline(inline_markdown(child))
            if text:
                lines.extend([f"> {text}", ""])
        elif child.name in {"ul", "ol"}:
            lines.extend(list_markdown(child))
            lines.append("")
        elif child.name == "table":
            lines.extend(table_markdown(child))
            lines.append("")
        elif child.name == "dl":
            for term in child.find_all("dt", recursive=False):
                text = clean_inline(inline_markdown(term))
                if text:
                    lines.extend([f"**{text}**", ""])
                definition = term.find_next_sibling("dd")
                if definition:
                    lines.extend(render_blocks(definition, heading_shift))
        elif child.name in BLOCK_TAGS or child.find(
            ["p", "pre", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "table"],
            recursive=False,
        ):
            lines.extend(render_blocks(child, heading_shift))
        else:
            text = clean_inline(inline_markdown(child))
            if text:
                lines.extend([text, ""])
    return lines


def discover_pages(site: Site) -> list[Path]:
    return [
        page
        for page in sorted(site.directory.rglob("*.html"))
        if include_page(site.crate, page.relative_to(site.directory))
    ]


def include_page(crate: str, relative_path: Path) -> bool:
    path = relative_path.as_posix()
    if crate in {"device_envoy_esp", "device_envoy_rp"}:
        return path.startswith(("button/", "cyd/"))

    if path.startswith(("button/", "cyd/")):
        return True
    if path == "memory/index.html":
        return True
    if path.startswith("memory/struct.Cyd") or path == "memory/struct.ButtonMemory.html":
        return True
    if path in {
        "memory/enum.Error.html",
        "memory/fn.assert_framebuffer_matches_expected_png.html",
        "wasm/fn.next_animation_frame.html",
    }:
        return True
    if path.startswith(("wasm/struct.Cyd", "wasm/struct.Button")):
        return True
    if path.startswith(("wasm/animation_frame/", "wasm/cyd_web/")):
        return True
    return path.startswith("wasm/simulator/struct.Cyd")


def page_markdown(site: Site, page: Path) -> list[str]:
    soup = BeautifulSoup(page.read_text(encoding="utf-8", errors="replace"), "html.parser")
    main = soup.find("main")
    if main is None:
        return []

    for selector in (
        ".anchor",
        ".rustdoc-breadcrumbs",
        ".rustdoc-toolbar",
        ".sidebar-resizer",
        ".src",
        ".sub-heading",
    ):
        for element in main.select(selector):
            element.decompose()
    for element_id in SKIP_IDS:
        element = main.find(id=element_id)
        if element:
            element.decompose()

    relative_path = page.relative_to(site.directory).as_posix()
    lines = [
        f"<!-- page: {site.crate}/{relative_path} -->",
        f"## Page `{site.crate}/{relative_path}`",
        "",
    ]
    lines.extend(render_blocks(main))
    while lines and not lines[-1]:
        lines.pop()
    lines.extend(["", "---", ""])
    return lines


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace_root = Path(__file__).resolve().parent.parent
    sites = [
        Site("device_envoy_core", workspace_root / "target/doc/device_envoy_core"),
        Site(
            "device_envoy_esp",
            workspace_root
            / "target/riscv32imac-unknown-none-elf/doc/device_envoy_esp",
        ),
        Site(
            "device_envoy_rp",
            workspace_root
            / "target/thumbv8m.main-none-eabihf/doc/device_envoy_rp",
        ),
    ]

    for site in sites:
        if not (site.directory / "index.html").is_file():
            raise SystemExit(f"missing rustdoc site; run `just docs` first: {site.directory}")

    rendered_by_site: list[tuple[Site, list[list[str]]]] = []
    for site in sites:
        rendered_pages = []
        for page in discover_pages(site):
            rendered = page_markdown(site, page)
            if rendered:
                rendered_pages.append(rendered)
        rendered_by_site.append((site, rendered_pages))

    lines = [
        "# Device Envoy CYD rustdoc corpus",
        "",
        "Agent-readable export of the authoritative Core, ESP, and RP CYD documentation,",
        "including portable buttons and the memory/WASM companion implementations.",
        "Unrelated device APIs and generated rustdoc boilerplate are omitted.",
        "",
        "## Corpus manifest",
        "",
    ]
    for site, rendered_pages in rendered_by_site:
        lines.append(f"- `{site.crate}`: {len(rendered_pages)} pages")
    lines.append("")

    rendered_pages = 0
    for site, pages in rendered_by_site:
        lines.extend([f"# Crate `{site.crate}`", ""])
        for rendered in pages:
            lines.extend(rendered)
            rendered_pages += 1

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")
    print(f"Wrote agent rustdoc corpus: {output}")
    print(f"Rendered pages: {rendered_pages}")
    print(f"Bytes: {output.stat().st_size}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
