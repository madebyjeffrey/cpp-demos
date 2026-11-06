#!/usr/bin/env python3
"""Index this validation run's paired CPU Skia previews without external assets."""
from html import escape
import os
from pathlib import Path
import struct


def gallery(directory: Path) -> int:
    pairs = {}
    for path in sorted(directory.glob("*.png")):
        name, separator, scale = path.stem.rpartition("-")
        if not separator or scale not in ("1x", "2x"):
            raise ValueError(f"Unpaired preview name: {path.name}")
        with path.open("rb") as stream:
            header = stream.read(24)
        if header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
            raise ValueError(f"Invalid PNG header: {path.name}")
        pairs.setdefault(name, {})[scale] = (path.name, struct.unpack(">II", header[16:24]))
    if not pairs:
        raise ValueError("No Lab previews were generated")
    cards = []
    for name, pair in pairs.items():
        if set(pair) != {"1x", "2x"}:
            raise ValueError(f"Missing preview density: {name}")
        if pair["2x"][1] != tuple(value * 2 for value in pair["1x"][1]):
            raise ValueError(f"Incorrect 2x framebuffer dimensions: {name}")
        figures = []
        for scale in ("1x", "2x"):
            filename, (width, height) = pair[scale]
            figures.append(f'<figure><a href="{escape(filename, quote=True)}">'
                           f'<img loading="lazy" src="{escape(filename, quote=True)}" '
                           f'width="{width}" height="{height}" alt="{escape(name)} {scale}">'
                           f'</a><figcaption>{scale} · {width}×{height} pixels</figcaption></figure>')
        cards.append(f'<section><h2>{escape(name)}</h2><div>{"".join(figures)}</div></section>')
    page = '''<!doctype html><meta charset="utf-8"><title>SCARS raster matrix</title>
<style>body{font:15px system-ui;margin:24px;background:#eceff2;color:#17202b}
input{font:inherit;padding:10px;width:min(600px,90%)}section{margin:30px 0}h2{font-size:16px}
section>div{display:flex;align-items:start;gap:20px}figure{margin:0;width:min(48%,560px)}
img{display:block;width:100%;height:auto}figcaption{margin-top:8px}[hidden]{display:none}</style>
<h1>SCARS raster matrix</h1><p>CPU Skia previews at explicit pixel densities. These do not
certify native Metal, display backing scales, VoiceOver or IME placement.</p>
<label>Filter by sample, state, theme, width or text profile <input id="filter" type="search"></label>
'''
    page += f'<p>{len(pairs)} paired scenes · {len(pairs) * 2} PNGs. Click a preview for full resolution.</p>'
    page += "".join(cards)
    page += r'''<script>document.querySelector('#filter').addEventListener('input', event => {
const words = event.target.value.toLowerCase().split(/\s+/).filter(Boolean);
document.querySelectorAll('section').forEach(section => {
section.hidden = !words.every(word => section.querySelector('h2').textContent.includes(word));
}); });</script>'''
    (directory / "index.html").write_text(page, encoding="utf-8")
    return len(pairs)


if __name__ == "__main__":
    directory = Path(os.environ["SCARS_LAB_ARTIFACTS"])
    print(f"Indexed {gallery(directory)} paired raster scenes: {directory / 'index.html'}")
