#!/usr/bin/env python3
"""Builds the Voltkit manual (static HTML) from docs/manual/pages/*.md.

    python3 docs/manual/build.py OUTPUT_DIR [--version X.Y.Z]

Standard library only. Page order and titles come from pages/_order.txt
(one `file-stem | Title | Group` per line). Markdown subset: headings, paragraphs,
bullet and numbered lists, tables, fenced code, `> [!NOTE|TIP|WARNING] text`
callouts, **bold**, *italic*, `code`, [links](url), <kbd>, and
`![alt](shot:NAME)` for a light/dark pair of screenshots from docs/screenshots.
"""
import html, json, re, shutil, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SHOTS = HERE.parent / "screenshots"
SITE_TITLE = "Voltkit Manual"


def slug(text):
    s = re.sub(r"<[^>]+>", "", text).lower()
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s or "section"


def inline(text):
    codes = []

    def keep(m):
        codes.append(html.escape(m.group(1)))
        return f"\x00{len(codes) - 1}\x00"

    text = re.sub(r"`([^`]+)`", keep, text)
    text = text.replace("&", "&amp;")
    text = re.sub(r"<(?!/?kbd>)", "&lt;", text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", text)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", lambda m: f'<a href="{link(m.group(2))}">{m.group(1)}</a>', text)
    return re.sub(r"\x00(\d+)\x00", lambda m: f"<code>{codes[int(m.group(1))]}</code>", text)


def link(url):
    if url.startswith(("http://", "https://", "#", "mailto:")):
        return url
    if "#" in url:
        page, frag = url.split("#", 1)
        return f"{page}.html#{frag}" if not page.endswith(".html") else url
    return url if url.endswith(".html") else url + ".html"


def picture(alt, name):
    a = html.escape(alt, quote=True)
    return (f'<figure><picture><source media="(prefers-color-scheme: dark)" srcset="img/{name}-dark.png">'
            f'<img src="img/{name}-light.png" alt="{a}" loading="lazy"></picture>'
            f"<figcaption>{html.escape(alt)}</figcaption></figure>")


def render(md):
    """Returns (html, headings[(level, id, text)])."""
    out, heads = [], []
    lines = md.split("\n")
    i = 0
    used = set()

    def uid(t):
        base = slug(t)
        n, cand = 2, base
        while cand in used:
            cand = f"{base}-{n}"
            n += 1
        used.add(cand)
        return cand

    while i < len(lines):
        line = lines[i]
        if not line.strip():
            i += 1
        elif line.startswith("```"):
            lang = line[3:].strip()
            i += 1
            buf = []
            while i < len(lines) and not lines[i].startswith("```"):
                buf.append(lines[i])
                i += 1
            i += 1
            out.append(f'<pre><code class="lang-{lang}">{html.escape(chr(10).join(buf))}</code></pre>')
        elif m := re.match(r"(#{1,4}) (.+)", line):
            lvl, text = len(m.group(1)), m.group(2).strip()
            hid = uid(text)
            if lvl > 1:
                heads.append((lvl, hid, re.sub(r"<[^>]+>|`", "", text)))
            out.append(f'<h{lvl} id="{hid}">{inline(text)}'
                       + (f'<a class="anchor" href="#{hid}" aria-label="Link to this section">#</a>' if lvl > 1 else "")
                       + f"</h{lvl}>")
            i += 1
        elif m := re.match(r"!\[([^\]]*)\]\(shot:([\w-]+)\)", line):
            out.append(picture(m.group(1), m.group(2)))
            i += 1
        elif line.startswith(">"):
            buf = []
            while i < len(lines) and lines[i].startswith(">"):
                buf.append(lines[i][1:].strip())
                i += 1
            body = " ".join(buf)
            kind = "note"
            if m := re.match(r"\[!(NOTE|TIP|WARNING|IMPORTANT)\]\s*(.*)", body):
                kind, body = m.group(1).lower(), m.group(2)
            label = {"note": "Note", "tip": "Tip", "warning": "Warning", "important": "Important"}[kind]
            out.append(f'<div class="callout {kind}"><strong>{label}</strong><p>{inline(body)}</p></div>')
        elif line.startswith("|") and i + 1 < len(lines) and re.match(r"\|[\s:|-]+\|?$", lines[i + 1]):
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                rows.append([c.strip() for c in lines[i].strip().strip("|").split("|")])
                i += 1
            head, body = rows[0], rows[2:]
            t = "<div class=\"table\"><table><thead><tr>" + "".join(f"<th>{inline(c)}</th>" for c in head) + "</tr></thead><tbody>"
            for r in body:
                t += "<tr>" + "".join(f"<td>{inline(c)}</td>" for c in r) + "</tr>"
            out.append(t + "</tbody></table></div>")
        elif re.match(r"\s*[-*] ", line):
            items = []
            while i < len(lines) and (re.match(r"\s*[-*] ", lines[i]) or (lines[i].startswith("  ") and lines[i].strip() and items)):
                if re.match(r"\s*[-*] ", lines[i]):
                    items.append(re.sub(r"^\s*[-*] ", "", lines[i]))
                else:
                    items[-1] += " " + lines[i].strip()
                i += 1
            out.append("<ul>" + "".join(f"<li>{inline(x)}</li>" for x in items) + "</ul>")
        elif re.match(r"\d+\. ", line):
            items = []
            while i < len(lines) and (re.match(r"\d+\. ", lines[i]) or (lines[i].startswith("   ") and lines[i].strip() and items)):
                if re.match(r"\d+\. ", lines[i]):
                    items.append(re.sub(r"^\d+\. ", "", lines[i]))
                else:
                    items[-1] += " " + lines[i].strip()
                i += 1
            out.append("<ol>" + "".join(f"<li>{inline(x)}</li>" for x in items) + "</ol>")
        else:
            buf = []
            while i < len(lines) and lines[i].strip() and not re.match(r"(#{1,4} |```|>|\||\s*[-*] |\d+\. |!\[)", lines[i]):
                buf.append(lines[i].strip())
                i += 1
            if not buf:  # a stray line that matched nothing above
                buf.append(lines[i].strip())
                i += 1
            out.append(f"<p>{inline(' '.join(buf))}</p>")
    return "\n".join(out), heads


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    out = Path(args[0] if args else "site/manual")
    version = "dev"
    if "--version" in sys.argv:
        version = sys.argv[sys.argv.index("--version") + 1]
    order = []
    for raw in (HERE / "pages" / "_order.txt").read_text().splitlines():
        if raw.strip() and not raw.startswith("#"):
            stem, title, group = [p.strip() for p in raw.split("|")]
            order.append((stem, title, group))
    shutil.rmtree(out, ignore_errors=True)
    (out / "img").mkdir(parents=True)
    used_shots = set()
    pages = []
    for stem, title, group in order:
        md = (HERE / "pages" / f"{stem}.md").read_text()
        used_shots.update(re.findall(r"shot:([\w-]+)", md))
        body, heads = render(md)
        pages.append(dict(stem=stem, title=title, group=group, body=body, heads=heads, text=re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", body))))
    for name in sorted(used_shots):
        for theme in ("light", "dark"):
            src = SHOTS / f"{name}-{theme}.png"
            if not src.exists():
                sys.exit(f"missing screenshot: {src}")
            shutil.copy(src, out / "img" / src.name)
    shutil.copy(SHOTS.parent / "icon.png", out / "img" / "icon.png")
    shutil.copy(HERE / "style.css", out / "style.css")
    shutil.copy(HERE / "manual.js", out / "manual.js")
    (out / ".nojekyll").touch()

    groups = []
    for p in pages:
        if not groups or groups[-1][0] != p["group"]:
            groups.append((p["group"], []))
        groups[-1][1].append(p)

    def nav(current):
        h = ""
        for g, items in groups:
            h += f'<div class="group"><div class="group-title">{html.escape(g)}</div><ul>'
            for p in items:
                cur = ' aria-current="page"' if p["stem"] == current else ""
                h += f'<li><a href="{p["stem"]}.html"{cur}>{html.escape(p["title"])}</a></li>'
            h += "</ul></div>"
        return h

    index = [dict(url=f'{p["stem"]}.html', title=p["title"], group=p["group"], text=p["text"][:6000],
                  heads=[dict(id=i, t=t) for _, i, t in p["heads"]]) for p in pages]
    (out / "search.json").write_text(json.dumps(index, ensure_ascii=False))

    for n, p in enumerate(pages):
        toc = "".join(f'<li class="l{lvl}"><a href="#{i}">{html.escape(t)}</a></li>' for lvl, i, t in p["heads"] if lvl <= 3)
        prev = pages[n - 1] if n else None
        nxt = pages[n + 1] if n + 1 < len(pages) else None
        pn = '<nav class="pager" aria-label="Previous and next page">'
        pn += f'<a class="prev" href="{prev["stem"]}.html"><span>Previous</span>{html.escape(prev["title"])}</a>' if prev else "<span></span>"
        pn += f'<a class="next" href="{nxt["stem"]}.html"><span>Next</span>{html.escape(nxt["title"])}</a>' if nxt else "<span></span>"
        pn += "</nav>"
        page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(p["title"])} · {SITE_TITLE}</title>
<meta name="description" content="{html.escape(p["text"][:160])}">
<link rel="icon" href="img/icon.png"><link rel="stylesheet" href="style.css"></head>
<body>
<header class="top"><button class="menu" aria-label="Menu" aria-expanded="false">☰</button>
<a class="brand" href="index.html"><img src="img/icon.png" alt="" width="28" height="28"> {SITE_TITLE}</a>
<input id="q" type="search" placeholder="Search the manual…" aria-label="Search the manual" autocomplete="off">
<a class="ghost" href="https://github.com/arios4k/voltkit">GitHub</a></header>
<div id="results" class="results" hidden></div>
<div class="layout">
<aside class="side" aria-label="Manual pages">{nav(p["stem"])}</aside>
<main><article>{p["body"]}</article>{pn}
<footer>Voltkit {html.escape(version)} · <a href="https://github.com/arios4k/voltkit/issues">Report a problem with this manual</a> · MIT License</footer></main>
<aside class="toc" aria-label="On this page"><div class="group-title">On this page</div><ul>{toc}</ul></aside>
</div><script src="manual.js"></script></body></html>"""
        (out / f'{p["stem"]}.html').write_text(page)
    # A broken link or image fails the build.
    ids = {f'{p["stem"]}.html': set(re.findall(r'id="([^"]+)"', (out / f'{p["stem"]}.html').read_text())) for p in pages}
    bad = []
    for p in pages:
        text = (out / f'{p["stem"]}.html').read_text()
        for href in re.findall(r'href="([^"]+)"', text):
            if href.startswith(("http://", "https://", "mailto:")):
                continue
            target, _, frag = href.partition("#")
            target = target or f'{p["stem"]}.html'
            if target not in ids and not (out / target).exists():
                bad.append(f'{p["stem"]}: no page {href}')
            elif frag and target in ids and frag not in ids[target]:
                bad.append(f'{p["stem"]}: no section {href}')
        bad += [f'{p["stem"]}: missing {src}' for src in re.findall(r'src="([^"]+)"', text) if not (out / src).exists()]
    if bad:
        sys.exit("Broken links in the manual:\n  " + "\n  ".join(bad))
    print(f"Manual: {len(pages)} pages, {len(used_shots)} screenshots -> {out}")


if __name__ == "__main__":
    main()
