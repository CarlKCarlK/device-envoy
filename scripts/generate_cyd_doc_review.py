#!/usr/bin/env python3
"""Generate a browser checklist for reviewing every rendered CYD doc page."""

from __future__ import annotations

import argparse
import html
import re
from pathlib import Path


SCOPES = (
    ("Core CYD", "target/doc/device_envoy_core/cyd"),
    ("Core Memory", "target/doc/device_envoy_core/memory"),
    ("Core WASM", "target/doc/device_envoy_core/wasm"),
    ("ESP CYD", "target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/cyd"),
    ("RP CYD", "target/thumbv8m.main-none-eabihf/doc/device_envoy_rp/cyd"),
)


def page_title(page: Path) -> str:
    source = page.read_text(encoding="utf-8")
    match = re.search(r"<title>(.*?)</title>", source, re.DOTALL)
    if match is None:
        raise SystemExit(f"missing HTML title: {page}")
    title = html.unescape(re.sub(r"<[^>]+>", "", match.group(1))).strip()
    return re.sub(r"\s+", " ", title)


def generate(repo: Path, output: Path) -> None:
    sections: list[str] = []
    total = 0
    for scope_name, relative_root in SCOPES:
        root = repo / relative_root
        if not root.is_dir():
            raise SystemExit(
                f"missing rendered documentation tree: {root}\n"
                "Build core, ESP, and RP documentation before generating the checklist."
            )
        pages = sorted(root.rglob("*.html"), key=lambda page: page.relative_to(root).as_posix())
        if not pages:
            raise SystemExit(f"rendered documentation tree contains no HTML pages: {root}")
        rows = []
        for page in pages:
            repo_relative = page.relative_to(repo).as_posix()
            output_relative = Path("..") / page.relative_to(repo)
            stable_id = repo_relative
            title = page_title(page)
            rows.append(
                f'''<article class="page" data-id="{html.escape(stable_id)}">
  <label class="reviewed"><input type="checkbox"> reviewed</label>
  <div class="page-main">
    <a href="{html.escape(output_relative.as_posix())}" target="_blank">{html.escape(title)}</a>
    <code>{html.escape(repo_relative)}</code>
    <textarea rows="3" placeholder="Comments, problems, or follow-up work…"></textarea>
  </div>
</article>'''
            )
        total += len(rows)
        sections.append(
            f'''<section>
  <h2>{html.escape(scope_name)} <span>{len(rows)} pages</span></h2>
  {''.join(rows)}
</section>'''
        )

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
  <div class="controls">
    <button id="export" type="button">Download comments as Markdown</button>
    <button id="clear" type="button">Clear saved review</button>
    <label><input id="unfinished" type="checkbox"> show unfinished only</label>
  </div>
</header>
<main>{''.join(sections)}</main>
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
  const blob = new Blob([lines.join("\n")], {{ type: "text/markdown" }});
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
    print(f"generated {output} with {total} pages")


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
