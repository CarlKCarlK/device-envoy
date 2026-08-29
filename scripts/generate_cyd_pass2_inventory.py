#!/usr/bin/env python3
"""Generate a deterministic CYD Pass 2 inventory from rendered rustdoc.

Rustdoc declaration sections are the public-declaration authority here. Each
one must have rustdoc's source link, and duplicate exact paths are rejected.
Coverage never looks at page-wide code: only examples in an item's own docs,
or literal documentation links to a destination item's visible examples.
"""
from __future__ import annotations

import argparse
import hashlib
import html
import re
from collections import Counter
from pathlib import Path

from bs4 import BeautifulSoup

SCOPES = (
    ("core-cyd", "Core shared CYD", "device_envoy_core::cyd", "core", "cargo test -p device-envoy-core --doc --no-default-features"),
    ("memory", "Core Memory", "device_envoy_core::memory", "host", "cargo test -p device-envoy-core --doc --features host"),
    ("wasm", "Core WASM", "device_envoy_core::wasm", "wasm32-unknown-unknown/wasm", "cargo test -p device-envoy-core --doc --no-default-features --features wasm wasm"),
    ("esp-cyd", "ESP CYD", "device_envoy_esp::cyd", "esp32c6", "cargo test -p device-envoy-esp --target riscv32imac-unknown-none-elf --doc --no-default-features --features doc-images,esp32c6"),
    ("rp-cyd", "RP CYD", "device_envoy_rp::cyd", "pico2,arm,wifi", "cargo test -p device-envoy-rp --target thumbv8m.main-none-eabihf --doc --no-default-features --features pico2,arm,wifi"),
)
ITEM_PAGE = re.compile(r"^(?:struct|enum|trait|type|constant|fn)\.[^.]+\.html$")
SOUPS: dict[Path, BeautifulSoup] = {}


def read_soup(page: Path) -> BeautifulSoup:
    page = page.resolve()
    if page not in SOUPS:
        SOUPS[page] = BeautifulSoup(page.read_text(errors="replace"), "html.parser")
    return SOUPS[page]


def fail(message: str) -> None:
    raise SystemExit(f"inventory error: {message}")


def pages(root: Path) -> list[Path]:
    if not root.is_dir() or not (root / "index.html").is_file():
        fail(f"required rendered root or canonical index is missing: {root}")
    result = []
    for page in sorted(root.rglob("*.html")):
        relative = page.relative_to(root)
        if relative.parts[0] == "src" or (page.name != "index.html" and not ITEM_PAGE.match(page.name)):
            continue
        soup = read_soup(page)
        if soup.find("meta", attrs={"http-equiv": re.compile("refresh", re.I)}):
            continue
        if not soup.select_one("main"):
            fail(f"empty/non-rustdoc canonical page: {page}")
        result.append(page)
    if not result:
        fail(f"required rendered root has no canonical pages: {root}")
    return result


def path_for(page: Path, root: Path, crate: str) -> str:
    parts = list(page.relative_to(root).parts[:-1])
    if page.stem != "index":
        parts.append(page.stem.split(".", 1)[1])
    return "::".join([crate, *parts])


def page_kind(page: Path) -> str:
    return "public module" if page.name == "index.html" else page.name.split(".", 1)[0]


def item_doc(section):
    container = section.find_parent("details")
    if container:
        return container.select_one(".docblock")
    sibling = section.find_next_sibling("div", class_="docblock")
    return sibling


def purpose(node) -> str:
    if node is None:
        return "Public API item."
    copy = BeautifulSoup(str(node), "html.parser")
    for child in copy.select(".example-wrap, pre, .rust.item-decl, .code-header"):
        child.decompose()
    text = re.sub(r"\s+", " ", html.unescape(copy.get_text(" ", strip=True))).strip()
    return text.split(".", 1)[0].strip() or "Public API item."


def examples(node) -> list[str]:
    return [] if node is None else [block.get_text(" ", strip=True) for block in node.select(".example-wrap pre")]


def example_destination(block, page: Path, root: Path, fallback_anchor: str) -> str:
    docblock = block.find_parent(class_="docblock")
    heading = block.find_previous(("h2", "h3", "h4", "h5", "h6"), id=True)
    if heading is not None and heading.find_parent(class_="docblock") == docblock:
        return f"{page.relative_to(root)}#{heading['id']}"
    suffix = f"#{fallback_anchor}" if fallback_anchor else ""
    return f"{page.relative_to(root)}{suffix}"


def examples_at_anchor(soup: BeautifulSoup, anchor: str) -> list[str]:
    anchor_node = soup.find(id=anchor)
    if anchor_node is None:
        return []
    if anchor_node.name in {"h2", "h3", "h4", "h5", "h6"}:
        docblock = anchor_node.find_parent(class_="docblock")
        if docblock is None:
            return []
        blocks = []
        for block in docblock.select(".example-wrap pre"):
            if block.find_previous(("h2", "h3", "h4", "h5", "h6"), id=True) == anchor_node:
                blocks.append(block.get_text(" ", strip=True))
        return blocks
    container = anchor_node.find_parent("details") or anchor_node
    return examples(container)


def coverage(node, name: str, page: Path, root: Path, item_anchor: str) -> tuple[str, str]:
    if node is not None:
        for block in node.select(".example-wrap pre"):
            if re.search(rf"\b{re.escape(name)}\b", block.get_text(" ", strip=True)):
                return "own-example", example_destination(block, page, root, item_anchor)
    if node is None:
        return "uncovered", "—"
    for link in node.select("a[href]"):
        if "example" not in link.parent.get_text(" ", strip=True).lower():
            continue
        href = link["href"]
        destination, separator, anchor = href.partition("#")
        target = page.resolve() if not destination else (page.parent / destination).resolve()
        if not target.is_file() or root.resolve() not in target.parents:
            continue
        soup = read_soup(target)
        target_examples = (
            examples_at_anchor(soup, anchor)
            if separator
            else examples(soup.select_one("main > .docblock") or soup.select_one("main .docblock"))
        )
        if any(re.search(rf"\b{re.escape(name)}\b", block) for block in target_examples):
            suffix = f"#{anchor}" if separator else ""
            return "linked-example", f"{target.relative_to(root)}{suffix}"
    return "uncovered", "—"


def declarations(page: Path, soup: BeautifulSoup, root: Path, crate: str):
    base = path_for(page, root, crate)
    # The first documentation block is the item/module synopsis. Do not use
    # <main>'s navigation, declaration, or implementation chrome as purpose.
    yield base, page_kind(page), "", soup.select_one("main > .docblock") or soup.select_one("main .docblock")
    selectors = "section.method, section.associatedtype, section.associatedconstant, section.variant, [id^='structfield.'], [id*='.field.']"
    seen = set()
    for section in soup.select(selectors):
        anchor = section.get("id", "")
        if not anchor or anchor in seen or "trait-impl" in section.get("class", []):
            continue
        # Variants and public fields share the containing type declaration's
        # source link; methods retain their own source link.
        if not (section.select_one("a.src") or soup.select_one("main a.src")):
            fail(f"rendered declaration has no source link: {page}#{anchor}")
        seen.add(anchor)
        if anchor.startswith("structfield."):
            kind, suffix = "public field", anchor.removeprefix("structfield.")
        elif ".field." in anchor:
            kind, suffix = "public variant field", anchor.removeprefix("variant.").replace(".field.", "::")
        elif anchor.startswith("variant."):
            kind, suffix = "enum variant", anchor.removeprefix("variant.")
        elif anchor.startswith("associatedtype."):
            kind, suffix = "associated type", anchor.removeprefix("associatedtype.")
        elif anchor.startswith("associatedconstant."):
            kind, suffix = "associated constant", anchor.removeprefix("associatedconstant.")
        else:
            kind, suffix = "method", anchor.split(".", 1)[1]
        yield f"{base}::{suffix}", kind, anchor, item_doc(section)


def collect(scope_id, label, crate, configuration, command, root: Path):
    rows = []
    for page in pages(root):
        soup = read_soup(page)
        for exact_path, kind, anchor, node in declarations(page, soup, root, crate):
            name = exact_path.rsplit("::", 1)[-1]
            state, example = coverage(node, name, page, root, anchor)
            identity = f"{scope_id}|{kind}|{exact_path}"
            rows.append({"id": f"{scope_id}-{hashlib.sha256(identity.encode()).hexdigest()[:12]}", "scope": label, "path": exact_path, "kind": kind, "purpose": purpose(node), "destination": f"{page.relative_to(root)}{'#' + anchor if anchor else ''}", "coverage": state, "example": example, "configuration": configuration, "command": command})
    duplicate = [path for path, count in Counter(row["path"] for row in rows).items() if count > 1]
    if duplicate:
        fail(f"rendered declaration reconciliation found duplicate path: {duplicate[0]}")
    return rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--core-host", type=Path, required=True)
    parser.add_argument("--core-wasm", type=Path, required=True)
    parser.add_argument("--esp", type=Path, required=True)
    parser.add_argument("--rp", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    roots = {"core-cyd": args.core / "cyd", "memory": args.core_host / "memory", "wasm": args.core_wasm / "wasm", "esp-cyd": args.esp / "cyd", "rp-cyd": args.rp / "cyd"}
    rows = []
    for scope_id, label, crate, configuration, command in SCOPES:
        rows.extend(collect(scope_id, label, crate, configuration, command, roots[scope_id]))
    if len({row["id"] for row in rows}) != len(rows):
        fail("stable identifier collision")
    rows.sort(key=lambda row: (row["scope"], row["path"], row["kind"]))
    totals, covered = Counter(row["scope"] for row in rows), Counter(row["coverage"] for row in rows)
    lines = ["<!-- TODO&#48; Consider deleting this generated audit evidence once CYD Pass 2 is implemented and released. -->", "", "# CYD Pass 2 rendered inventory and matrix", "", "Generated from required rendered rustdoc roots. Every row is one compiler-rendered public declaration section with a source link; missing roots and duplicate paths fail generation. Coverage inspects only visible documentation example blocks.", "", f"Inventory records: **{len(rows)}**. Own examples: **{covered['own-example']}**. Linked examples: **{covered['linked-example']}**. Uncovered: **{covered['uncovered']}**.", "", "| Scope | Records |", "|---|---:|"]
    lines += [f"| {scope} | {count} |" for scope, count in sorted(totals.items())]
    lines += ["", "| ID | Exact rendered item | Kind | Purpose | Literal rendered destination | Coverage | Example destination | Compile configuration |", "|---|---|---|---|---|---|---|---|"]
    for row in rows:
        item_purpose = row["purpose"].replace("|", "\\|")
        lines.append(f"| `{row['id']}` | `{row['path']}` | {row['kind']} | {item_purpose} | `{row['destination']}` | {row['coverage']} | `{row['example']}` | `{row['configuration']}: {row['command']}` |")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
