#!/usr/bin/env python3
"""Generates apercu.svg: a color capture of the game screen for the README (python3 apercu.py).

The real UI rendering runs on a fake screen that records each character and its color; curses is not
initialized, its color functions are replaced by stand-ins that remember the pairs.
"""

import curses
import random
from html import escape
from pathlib import Path

import events
import ui
from game import Game, need

W, H = 100, 30
FONT_SIZE = 14
CELL_W, CELL_H = 8.4, 17
MARGIN = 10
BASELINE_OFFSET = 4  # from the cell bottom to the text baseline
SETTLE_FRAMES = 40
BG, FG = "#1d1f21", "#c5c8c6"
PALETTE = {curses.COLOR_BLACK: "#1d1f21", curses.COLOR_RED: "#e5534b", curses.COLOR_GREEN: "#8ec07c",
           curses.COLOR_YELLOW: "#f0c674", curses.COLOR_BLUE: "#81a2be", curses.COLOR_MAGENTA: "#c397d8",
           curses.COLOR_CYAN: "#70c0b1", curses.COLOR_WHITE: "#eaeaea", 208: "#f28c28", 244: "#808080"}
PAIRS = {}
# Border strokes drawn as SVG lines (top, bottom, left, right): as glyphs, the font does not fill the full
# line height and vertical borders turn into dotted lines.
BOX = {"│": "ud", "─": "lr", "┌": "dr", "┐": "dl", "└": "ur", "┘": "ul", "├": "udr", "┤": "udl", "┬": "lrd",
       "┴": "lru", "┼": "udlr"}
# Same flaw for the mushroom's full blocks: drawn as rectangles (top, bottom as a fraction of a line).
BLOCK = {"█": (0, 1), "▀": (0, 0.5), "▄": (0.5, 1), "▁": (0.875, 1)}


class Sound:
    """Fake "available" sound, so the help bar shows the sound mode."""
    ok, muted, music_on, volume = True, False, True, 1.0

    def __getattr__(self, name):
        return lambda *a, **k: None


class Screen:
    """Fake screen: grid of (character, attribute)."""

    def __init__(self):
        self.erase()

    def getmaxyx(self):
        return H, W

    def erase(self):
        self.cells = [[(" ", 0)] * W for _ in range(H)]

    def addstr(self, y, x, s, attr=0):
        for i, ch in enumerate(s):
            if 0 <= x + i < W:
                self.cells[y][x + i] = (ch, attr)

    def __getattr__(self, name):
        return lambda *a, **k: None


def fake_curses():
    curses.has_colors = lambda: True
    curses.start_color = curses.use_default_colors = lambda: None
    curses.COLORS = 256
    curses.init_pair = lambda i, fg, bg: PAIRS.__setitem__(i, (fg, bg))
    curses.color_pair = lambda i: i << 8


def scene():
    g = Game(rng=random.Random(4))
    g.click()
    g.exploded, g.challenge, g.level, g.isotopes, g.pt = True, "nanar", 72, 41, 4
    g.radiation = 42e6
    g.progress = 0.6 * need(g.level)
    counts = dict(soldier=140, mortar=95, artillery=60, tank=32, bomber=12, sub=8, icbm=3, reactor=1)
    for uid in counts:
        g.talents["u_" + uid] = 1
    g.talents["u_blackhole"] = 1
    g.units.update(counts)
    g.mutations = ["spores", "fallout"]
    g.combo, g.crit_gauge = 1.4, 0.62
    g.time = 500.0
    dur = events.enemy_duration(g, "commando")
    g.enemies = [dict(kind="commando", start=g.time - dur * 0.65, dur=dur, hits=0)]
    g.plane = dict(letter="R", start=g.time - 2, dur=10)
    g.messages.clear()
    g.sfx.clear()
    return g


def color(c):
    return PALETTE.get(c, FG)


def same_attr_runs(row):
    """Splits a line into (start, end, attribute) segments of the same attribute."""
    x = 0
    while x < W:
        attr = row[x][1]
        end = x
        while end < W and row[end][1] == attr:
            end += 1
        yield x, end, attr
        x = end


def colors(attr):
    """(text, background or None) colors of a curses attribute."""
    pair = (attr >> 8) & 0xFF  # inverse of color_pair in fake_curses
    fg, bg = PAIRS.get(pair, (-1, -1))
    fg, bg = color(fg) if fg != -1 else FG, color(bg) if bg != -1 else None
    if attr & curses.A_REVERSE:
        fg, bg = bg or BG, fg
    return fg, bg


def box_path(c, px, py, i):
    """SVG paths for border stroke c, i-th cell of the segment starting at (px, py)."""
    x0, cx, cy = px + i * CELL_W, px + (i + 0.5) * CELL_W, py + CELL_H / 2
    ends = dict(u=(cx, py), d=(cx, py + CELL_H), l=(x0, cy), r=(x0 + CELL_W, cy))
    # The first two arms form a single polyline through the center: solid corner, no notch.
    (ax, ay), (bx, by) = ends[BOX[c][0]], ends[BOX[c][1]]
    d = [f"M{ax:.1f} {ay:.1f}L{cx:.1f} {cy:.1f}L{bx:.1f} {by:.1f}"]
    d += [f"M{cx:.1f} {cy:.1f}L{ends[k][0]:.1f} {ends[k][1]:.1f}" for k in BOX[c][2:]]
    return d


def svg(cells):
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W * CELL_W + 2 * MARGIN:.0f}" '
           f'height="{H * CELL_H + 2 * MARGIN}" '
           f'font-family="DejaVu Sans Mono, Menlo, Consolas, monospace" font-size="{FONT_SIZE}">',
           f'<rect width="100%" height="100%" rx="8" fill="{BG}"/>']
    for y, row in enumerate(cells):
        for x, end, attr in same_attr_runs(row):
            text = "".join(c for c, _ in row[x:end])
            fg, bg = colors(attr)
            px, py = MARGIN + x * CELL_W, MARGIN + y * CELL_H
            width = (end - x) * CELL_W
            if bg:
                out.append(f'<rect x="{px:.1f}" y="{py}" width="{width:.1f}" height="{CELL_H}" fill="{bg}"/>')
            dim = ' opacity="0.6"' * bool(attr & curses.A_DIM)
            d = []
            for i, c in enumerate(text):
                if c in BOX:
                    d += box_path(c, px, py, i)
                elif c in BLOCK:
                    top, bottom = BLOCK[c]
                    x0 = px + i * CELL_W
                    out.append(f'<rect x="{x0:.1f}" y="{py + top * CELL_H:.1f}" width="{CELL_W:.1f}" '
                               f'height="{(bottom - top) * CELL_H:.1f}" fill="{fg}"{dim}/>')
            if d:  # a single path per segment: lighter file, and no darker joins in dim mode
                out.append(f'<path d="{"".join(d)}" stroke="{fg}" stroke-width="1.2" fill="none"{dim}/>')
            text = "".join(" " if c in BOX or c in BLOCK else c for c in text)
            if text.strip():
                style = ' font-weight="bold"' * bool(attr & curses.A_BOLD) + dim
                out.append(f'<text x="{px:.1f}" y="{py + CELL_H - BASELINE_OFFSET}" fill="{fg}"{style} '
                           f'xml:space="preserve" textLength="{width:.1f}" '
                           f'lengthAdjust="spacingAndGlyphs">{escape(text)}</text>')
    out.append("</svg>")
    return "\n".join(out)


def main():
    fake_curses()
    scr = Screen()
    u = ui.UI(scr, scene(), Sound())
    u.rng = random.Random(1)
    for _ in range(SETTLE_FRAMES):
        u.draw(0.05)
    # Nanarland challenge: a press that fizzles, and a botched-shoot title set at the start of the banner.
    u.floats.append(dict(text="pschitt…", age=0.0, dx=-3, attr=u.c("grey") | curses.A_BOLD))
    u.news, u.news_off = events.NEWS_NANAR[1], u.w - len(" FLASH INFO ") - 2
    u.draw(0.0)
    Path(__file__).with_name("apercu.svg").write_text(svg(scr.cells), encoding="utf-8")
    print("apercu.svg écrit")


if __name__ == "__main__":
    main()
