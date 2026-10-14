# /// script
# requires-python = ">=3.10"
# dependencies = ["pillow==12.3.0"]
# ///
"""Turns a tmux ANSI capture into a documentation screenshot.

usage: uv run render.py --freeze BIN --font TTF [--cols A:B] [--rows A:B] IN.ansi OUT.webp

freeze draws the capture in a window frame; the result is scaled down to at most
MAX_WIDTH pixels and saved as lossless WebP. Before that, the capture is cropped to
the given screen cells and three of freeze's quirks are worked around:

- SGR 39 and 49 (default colours) leave the previous colour in place in freeze, so a
  highlighted row bleeds into the cells after it. They become the terminal's text and
  background colours instead.
- Bold (SGR 1) is dropped: freeze has no bold face, and resets the colour of text whose
  colour was set before the bold.
- freeze drops the last line, so the capture ends with an extra blank one.
"""

import argparse
import re
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

BACKGROUND = "#11111b"
TEXT_FG = "38;2;205;214;244"
CANVAS_BG = "48;2;17;17;27"
DEFAULTS = {"39": TEXT_FG, "49": CANVAS_BG}
MAX_WIDTH = 2000
SGR = re.compile(r"\x1b\[([0-9;:]*)m")


def fix_params(params: str) -> str:
    parts = params.split(";") if params else ["0"]
    out, i = [], 0
    while i < len(parts):
        part = parts[i]
        if part in ("38", "48") and i + 1 < len(parts):
            n = 5 if parts[i + 1] == "2" else 3
            out.extend(parts[i : i + n])
            i += n
            continue
        if part != "1":
            out.append(DEFAULTS.get(part, part))
        i += 1
    return ";".join(out)


def sgr(params: str) -> str:
    fixed = fix_params(params)
    # An SGR left with no parameters would mean reset.
    return f"\x1b[{fixed}m" if fixed else ""


def crop_line(line: str, start: int, end: int) -> str:
    out, col, pos = [], 0, 0
    while pos < len(line):
        match = SGR.match(line, pos)
        if match:
            out.append(sgr(match.group(1)))
            pos = match.end()
            continue
        if start <= col < end:
            out.append(line[pos])
        col += 1
        pos += 1
    return "".join(out) + "\x1b[0m"


def prepare(capture: str, cols: tuple[int, int], rows: tuple[int, int]) -> str:
    lines = capture.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    # Colour state carries over from rows cropped away above: replay their escapes.
    prefix = "".join(sgr(m.group(1)) for line in lines[: rows[0]] for m in SGR.finditer(line))
    body = [crop_line(line, *cols) for line in lines[rows[0] : rows[1]]]
    if body:
        body[0] = prefix + body[0]
    return "\n".join(body) + "\n\n"


def span(value: str) -> tuple[int, int]:
    start, end = value.split(":")
    return int(start), int(end)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--freeze", required=True)
    parser.add_argument("--font", required=True)
    parser.add_argument("--cols", type=span, default=(0, 10**6))
    parser.add_argument("--rows", type=span, default=(0, 10**6))
    parser.add_argument("capture", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as tmp:
        ansi = Path(tmp, "capture.ansi")
        png = Path(tmp, "shot.png")
        ansi.write_text(prepare(args.capture.read_text(encoding="utf-8"), args.cols, args.rows), encoding="utf-8")
        subprocess.run(
            [
                args.freeze, str(ansi), "--language", "ansi", "--window",
                "--padding", "16,20", "--margin", "0", "--background", BACKGROUND,
                "--font.size", "7", "--font.file", args.font, "--font.family", "JetBrains Mono NL",
                "--output", str(png),
            ],
            check=True,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
        )
        image = Image.open(png).convert("RGB")
        if image.width > MAX_WIDTH:
            height = round(image.height * MAX_WIDTH / image.width)
            image = image.resize((MAX_WIDTH, height), Image.LANCZOS)
        image.save(args.output, "WEBP", lossless=True, method=6)
    print(f"wrote {args.output} ({image.width}x{image.height})")


main()
