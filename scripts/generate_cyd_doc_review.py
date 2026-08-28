#!/usr/bin/env python3
"""Generate a browser checklist for reviewing every rendered CYD doc page."""

from __future__ import annotations

import argparse
import html
import re
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote, urlsplit

from bs4 import BeautifulSoup, Tag


SCOPES = (
    ("ESP CYD", "target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/cyd"),
    ("RP CYD", "target/thumbv8m.main-none-eabihf/doc/device_envoy_rp/cyd"),
    ("Core CYD", "target/doc/device_envoy_core/cyd"),
    ("Core WASM", "target/doc/device_envoy_core/wasm"),
    ("Core Memory", "target/doc/device_envoy_core/memory"),
)

# These paths remain technically public so sibling platform crates can use
# them, but they are deliberately hidden from application-facing Rustdoc.
HIDDEN_REVIEW_PREFIXES = (
    "target/doc/device_envoy_core/cyd/backend/",
)


@dataclass(frozen=True)
class Page:
    scope: str
    root: Path
    path: Path
    repo_relative: str


def page_title(page: Path) -> str:
    source = page.read_text(encoding="utf-8")
    match = re.search(r"<title>(.*?)</title>", source, re.DOTALL)
    if match is None:
        raise SystemExit(f"missing HTML title: {page}")
    title = html.unescape(re.sub(r"<[^>]+>", "", match.group(1))).strip()
    return re.sub(r"\s+", " ", title)


def load_pages(repo: Path) -> tuple[dict[str, Page], dict[Path, str], dict[str, Path]]:
    pages: dict[str, Page] = {}
    by_path: dict[Path, str] = {}
    crate_roots: dict[str, Path] = {}
    for scope_name, relative_root in SCOPES:
        root = repo / relative_root
        if not root.is_dir():
            raise SystemExit(
                f"missing rendered documentation tree: {root}\n"
                "Build core, ESP, and RP documentation before generating the checklist."
            )
        scope_pages = sorted(
            path
            for path in root.rglob("*.html")
            if not any(
                path.relative_to(repo).as_posix().startswith(prefix)
                for prefix in HIDDEN_REVIEW_PREFIXES
            )
        )
        if not scope_pages:
            raise SystemExit(f"rendered documentation tree contains no HTML pages: {root}")
        crate_roots[root.parent.name] = root.parent.resolve()
        for path in scope_pages:
            repo_relative = path.relative_to(repo).as_posix()
            if repo_relative in pages:
                raise SystemExit(f"rendered page belongs to multiple scopes: {repo_relative}")
            page = Page(scope_name, root.resolve(), path.resolve(), repo_relative)
            pages[repo_relative] = page
            by_path[page.path] = repo_relative
    return pages, by_path, crate_roots


def skips_traversal(link: Tag) -> bool:
    if "src" in link.get("class", []):
        return True
    for ancestor in link.parents:
        if not isinstance(ancestor, Tag):
            continue
        if ancestor.name in {"nav", "rustdoc-topbar", "rustdoc-toolbar"}:
            return True
        identity = " ".join(
            [str(ancestor.get("id", "")), *[str(value) for value in ancestor.get("class", [])]]
        ).lower()
        if any(
            marker in identity
            for marker in (
                "sidebar",
                "rustdoc-breadcrumbs",
                "implementors",
                "trait-implementations",
                "synthetic-implementations",
                "blanket-implementations",
                "impl-items",
            )
        ):
            return True
    return False


def resolve_link(
    page: Page,
    href: str,
    by_path: dict[Path, str],
    crate_roots: dict[str, Path],
) -> str | None:
    parsed = urlsplit(href)
    if parsed.scheme in {"http", "https"}:
        if parsed.netloc != "docs.rs":
            return None
        parts = [unquote(part) for part in parsed.path.split("/") if part]
        crate_index = next(
            (index for index, part in enumerate(parts) if part in crate_roots), None
        )
        if crate_index is None:
            return None
        candidate = crate_roots[parts[crate_index]].joinpath(*parts[crate_index + 1 :])
    elif parsed.scheme:
        return None
    else:
        if not parsed.path:
            return None
        candidate = page.path.parent / unquote(parsed.path)
    candidate = candidate.resolve()
    if candidate.is_dir():
        candidate = candidate / "index.html"
    return by_path.get(candidate)


def linked_pages(
    page: Page,
    by_path: dict[Path, str],
    crate_roots: dict[str, Path],
) -> list[str]:
    soup = BeautifulSoup(page.path.read_text(encoding="utf-8"), "html.parser")
    linked: list[str] = []
    seen: set[str] = set()
    for link in soup.select("a[href]"):
        if skips_traversal(link):
            continue
        target = resolve_link(page, link["href"], by_path, crate_roots)
        if target is None or target == page.repo_relative or target in seen:
            continue
        seen.add(target)
        linked.append(target)
    return linked


def depth_first_order(
    start: str,
    pages: dict[str, Page],
    by_path: dict[Path, str],
    crate_roots: dict[str, Path],
) -> tuple[list[str], list[str]]:
    reached: list[str] = []
    visited: set[str] = set()

    def visit(page_id: str) -> None:
        if page_id in visited:
            return
        visited.add(page_id)
        reached.append(page_id)
        for linked_page_id in linked_pages(pages[page_id], by_path, crate_roots):
            visit(linked_page_id)

    visit(start)
    orphaned = sorted(set(pages) - visited)
    if len(reached) != len(set(reached)):
        raise SystemExit("depth-first traversal emitted a duplicate page")
    if set(reached) | set(orphaned) != set(pages):
        raise SystemExit("depth-first traversal did not reconcile with the scoped pages")
    return reached, orphaned


def existing_stable_ids(output: Path) -> set[str]:
    if not output.is_file():
        return set()
    soup = BeautifulSoup(output.read_text(encoding="utf-8"), "html.parser")
    return {article["data-id"] for article in soup.select("article.page[data-id]")}


def page_rows(repo: Path, pages: dict[str, Page], page_ids: list[str]) -> str:
    rows = []
    for page_id in page_ids:
        page = pages[page_id]
        output_relative = Path("..") / page.path.relative_to(repo)
        rows.append(
            f'''<article class="page" data-id="{html.escape(page.repo_relative)}">
  <label class="reviewed"><input type="checkbox"> reviewed</label>
  <div class="page-main">
    <a href="{html.escape(output_relative.as_posix())}" target="_blank">{html.escape(page_title(page.path))}</a>
    <span class="scope">{html.escape(page.scope)}</span>
    <code>{html.escape(page.repo_relative)}</code>
    <textarea rows="3" placeholder="Comments, problems, or follow-up work…"></textarea>
  </div>
</article>'''
        )
    return "".join(rows)


def generate(repo: Path, output: Path) -> None:
    pages, by_path, crate_roots = load_pages(repo)
    start = "target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/cyd/index.html"
    if start not in pages:
        raise SystemExit(f"required first page is missing: {start}")
    reached, orphaned = depth_first_order(start, pages, by_path, crate_roots)
    if reached[0] != start:
        raise SystemExit("ESP CYD overview is not first in the traversal")

    old_ids = existing_stable_ids(output)
    removed_ids = old_ids - set(pages)
    added_ids = set(pages) - old_ids

    total = len(pages)
    orphan_content = (
        page_rows(repo, pages, orphaned)
        if orphaned
        else '<p class="empty">Every scoped page was reached by the traversal.</p>'
    )
    sections = f'''<section data-kind="reached">
  <h2>1. Depth-first traversal <span>{len(reached)} reached pages</span></h2>
  {page_rows(repo, pages, reached)}
</section>
<section data-kind="orphaned">
  <h2>2. Orphaned pages <span>{len(orphaned)} pages</span></h2>
  {orphan_content}
</section>'''

    document = f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CYD rendered documentation review</title>
<style>
:root {{ color-scheme: light dark; font-family: system-ui, sans-serif; }}
body {{ margin: 0 auto; max-width: 76rem; padding: 1.5rem; }}
header {{ position: sticky; top: 0; z-index: 1; padding: 1rem 0; background: Canvas; border-bottom: 1px solid GrayText; }}
h1 {{ margin: 0 0 .4rem; }}
h2 {{ margin-top: 2rem; }}
h2 span {{ color: GrayText; font-size: .75em; font-weight: normal; }}
.controls {{ display: flex; flex-wrap: wrap; gap: .7rem; align-items: center; }}
button {{ padding: .45rem .75rem; }}
.page {{ display: grid; grid-template-columns: 7rem 1fr; gap: .8rem; padding: .8rem 0; border-top: 1px solid color-mix(in srgb, CanvasText 18%, transparent); }}
.page-main {{ display: grid; gap: .35rem; }}
.page a {{ font-weight: 650; }}
.page .scope {{ color: GrayText; font-size: .85rem; }}
.page code {{ overflow-wrap: anywhere; color: GrayText; }}
.page textarea {{ box-sizing: border-box; width: 100%; resize: vertical; font: inherit; }}
.page.done {{ opacity: .68; }}
.page.done a {{ text-decoration: line-through; }}
.reviewed {{ white-space: nowrap; }}
@media (max-width: 40rem) {{ .page {{ grid-template-columns: 1fr; }} }}
</style>
</head>
<body>
<header>
  <h1>CYD rendered documentation review</h1>
  <p id="progress">0 / {total} pages reviewed</p>
  <p>Review order follows meaningful public documentation links depth first, starting at the ESP CYD overview. Links are followed in rendered order and stay within the configured Device Envoy scopes. Any scoped page not reached this way appears last under <strong>Orphaned pages</strong>.</p>
  <div class="controls">
    <button id="export" type="button">Download comments as Markdown</button>
    <button id="clear" type="button">Clear saved review</button>
    <label><input id="unfinished" type="checkbox"> show unfinished only</label>
  </div>
</header>
<main>{sections}</main>
<script>
const storageKey = "device-envoy-cyd-doc-review-v1";
const pages = [...document.querySelectorAll(".page")];
let saved = {{}};
try {{ saved = JSON.parse(localStorage.getItem(storageKey) || "{{}}"); }} catch (_) {{}}

function update() {{
  const state = {{}};
  let reviewed = 0;
  for (const page of pages) {{
    const checkbox = page.querySelector('input[type="checkbox"]');
    const notes = page.querySelector("textarea");
    page.classList.toggle("done", checkbox.checked);
    page.hidden = document.querySelector("#unfinished").checked && checkbox.checked;
    if (checkbox.checked) reviewed += 1;
    if (checkbox.checked || notes.value) state[page.dataset.id] = {{ reviewed: checkbox.checked, notes: notes.value }};
  }}
  document.querySelector("#progress").textContent = `${{reviewed}} / ${{pages.length}} pages reviewed`;
  localStorage.setItem(storageKey, JSON.stringify(state));
}}

for (const page of pages) {{
  const state = saved[page.dataset.id] || {{}};
  page.querySelector('input[type="checkbox"]').checked = Boolean(state.reviewed);
  page.querySelector("textarea").value = state.notes || "";
  page.addEventListener("input", update);
  page.addEventListener("change", update);
}}
document.querySelector("#unfinished").addEventListener("change", update);
document.querySelector("#clear").addEventListener("click", () => {{
  if (!confirm("Clear every checkbox and comment in this review?")) return;
  localStorage.removeItem(storageKey);
  for (const page of pages) {{
    page.querySelector('input[type="checkbox"]').checked = false;
    page.querySelector("textarea").value = "";
  }}
  update();
}});
document.querySelector("#export").addEventListener("click", () => {{
  const lines = ["# CYD rendered documentation review comments", ""];
  for (const section of document.querySelectorAll("main section")) {{
    const commented = [...section.querySelectorAll(".page")].filter(page => page.querySelector("textarea").value.trim());
    if (!commented.length) continue;
    lines.push(`## ${{section.querySelector("h2").childNodes[0].textContent.trim()}}`, "");
    for (const page of commented) {{
      const link = page.querySelector("a");
      const notes = page.querySelector("textarea").value.trim();
      lines.push(`### ${{link.textContent}}`, "", `Rendered page: \`${{page.dataset.id}}\``, "", notes, "");
    }}
  }}
  if (lines.length === 2) lines.push("No comments recorded.", "");
  const blob = new Blob([lines.join("\\n")], {{ type: "text/markdown" }});
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "CYD_DOC_REVIEW_COMMENTS.md";
  link.click();
  URL.revokeObjectURL(link.href);
}});
update();
</script>
</body>
</html>
'''
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(document, encoding="utf-8")
    print(
        f"generated {output} with {total} pages: "
        f"{len(reached)} reached, {len(orphaned)} orphaned; "
        f"{len(added_ids)} added, {len(removed_ids)} removed"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()
    output = args.output.resolve() if args.output else repo / "specs/CYD_DOC_REVIEW.html"
    generate(repo, output)


if __name__ == "__main__":
    main()
