"""Procedural ASCII generation of the atomic mushroom.

Each function returns a list of cells (y, x, character, color) relative to the panel,
the color being a name ("red", "orange", ...) that the UI turns into a curses attribute.
"""

import math
import unicodedata

PALETTES = {
    "classic": ["yellow", "orange", "red", "orange", "yellow", "white", "red"],
    "toxic": ["green", "green", "yellow", "cyan", "green"],
    "plasma": ["magenta", "cyan", "blue", "white", "magenta"],
    "gold": ["yellow", "orange", "yellow", "white", "yellow"],
    "ghost": ["white", "grey", "white", "grey", "cyan"],
    "neon": ["magenta", "cyan", "magenta", "blue", "cyan"],
    "ice": ["cyan", "white", "blue", "cyan", "white"],
    "lava": ["red", "orange", "red", "yellow", "red"],
    "camo": ["green", "grey", "yellow", "green", "grey"],
    "crystal": ["cyan", "magenta", "white", "blue", "cyan"],
    "obsidian": ["grey", "magenta", "grey", "blue", "white"],
    "carton": ["orange", "yellow", "grey", "orange", "white"],
}
RAINBOW = ["red", "orange", "yellow", "green", "cyan", "blue", "magenta"]
DENSE, MID, LIGHT = "@#%&", "*%+=", ".:'*"

BOMB = [
    r"  _/\_  ",
    r" |BOMB| ",
    r" |____| ",
]


def _noise(x, y, frame):
    """16-bit pseudo-random value, stable for a given cell and frame."""
    return ((x * 73856093) ^ (y * 19349663) ^ (frame * 83492791)) & 0xFFFF


def _pick(seq, n):
    return seq[n % len(seq)]


def size_fraction(level, cap):
    """Relative size (0..1): grows with the level, maxed at the run's max level."""
    if level <= 0:
        return 0.0
    return 0.15 + 0.85 * math.log(1 + level) / math.log(1 + cap)


def bomb(W, H):
    ground = H - 1
    top = ground - len(BOMB)
    left = W // 2 - len(BOMB[0]) // 2
    cells = []
    for i, row in enumerate(BOMB):
        color = "grey" if i == 0 else "white"
        for j, ch in enumerate(row):
            if ch != " ":
                cells.append((top + i, left + j, ch, color))
    return cells


def mushroom_height(H, f):
    """Total height of a size-f mushroom in a panel of height H."""
    return max(4, int(round((H - 2) * f)))


def _dimensions(W, H, f, twin):
    """(total height, cap width) of a size-f mushroom; twins share the width."""
    height = mushroom_height(H, f)
    cap_w = min(W - 2, max(5, int(height * 2.4)))
    if twin:
        cap_w = min(cap_w, (W - 4) // 2)
    return height, cap_w


def mushroom(W, H, f, t, skin="classic", twin=False):
    """Mushroom of size f (0..1) in a W×H panel, ground on the last line."""
    ground = H - 1
    frame = int(t * 6)
    height, cap_w = _dimensions(W, H, f, twin)
    cap_h = max(2, int(height * 0.42))
    stem_h = height - cap_h
    centers = [W // 2] if not twin else [W // 2 - cap_w // 2 - 1, W // 2 + cap_w // 2 + 1]
    cells = []
    for cx in centers:
        _one(cells, cx, ground, height, cap_h, stem_h, cap_w, frame, t, skin)
    return cells


def _color(skin, n, row, t):
    if skin == "rainbow":
        return RAINBOW[(row + int(t * 4)) % len(RAINBOW)]
    return _pick(PALETTES.get(skin, PALETTES["classic"]), n >> 3)


def _one(cells, cx, ground, height, cap_h, stem_h, cap_w, frame, t, skin):
    top = ground - height
    mario = skin == "mario"
    if mario:
        frame = 0
    _cap(cells, cx, top, cap_h, cap_w, frame, t, skin, mario)
    _stem(cells, cx, top + cap_h, stem_h, cap_h, cap_w, height, frame, t, skin, mario)
    _dust(cells, cx, ground, height, cap_w, frame)


def _cap(cells, cx, top, cap_h, cap_w, frame, t, skin, mario):
    """Cap: a dome, wider at the bottom, with a slightly tucked-in lower rim."""
    for i in range(cap_h):
        rel = (i + 1) / cap_h
        width = cap_w * (0.4 + 0.6 * math.sin(rel * math.pi / 2))
        is_rim = i == cap_h - 1 and cap_h > 2
        if is_rim:
            width *= 0.9
        half = max(1, int(width / 2))
        y = top + i
        for x in range(cx - half, cx + half + 1):
            n = _noise(x, y, frame)
            dist = abs(x - cx) / (half + 0.5)
            if mario:
                spot = (x + 2 * y) % 5 == 0 and dist < 0.85
                cells.append((y, x, "O" if spot else "#", "white" if spot else "red"))
                continue
            chars = DENSE if dist < 0.5 else MID if dist < 0.85 else LIGHT
            cells.append((y, x, _pick(chars, n), _color(skin, n, i, t)))


def _stem(cells, cx, top, stem_h, cap_h, cap_w, height, frame, t, skin, mario):
    """Stem: widens toward the bottom, with a condensation ring on large mushrooms."""
    stem_w = max(1, int(cap_w * 0.16)) | 1  # odd, to stay centered on cx
    ring_row = int(stem_h * 0.35) if height >= 10 else None
    for j in range(stem_h):
        y = top + j
        half = (stem_w + int(2 * j / max(1, stem_h))) // 2
        if j == ring_row:
            _ring(cells, cx, y, max(half + 2, int(cap_w * 0.3)), frame)
            continue
        for x in range(cx - half, cx + half + 1):
            n = _noise(x, y, frame)
            if mario:
                cells.append((y, x, "|" if x in (cx - half, cx + half) else " ", "white"))
            elif x == cx - half and half:
                cells.append((y, x, "(", "orange"))
            elif x == cx + half and half:
                cells.append((y, x, ")", "orange"))
            else:
                cells.append((y, x, _pick("!|:|", n), _color(skin, n, cap_h + j, t)))
    if mario and stem_h >= 2 and stem_w >= 3:
        y = top + stem_h // 2
        for k, ch in enumerate("•‿•"):
            cells.append((y, cx - 1 + k, ch, "white"))


def _ring(cells, cx, y, half, frame):
    for x in range(cx - half, cx + half + 1):
        ch = "(" if x == cx - half else ")" if x == cx + half else "~=-"[(x + frame) % 3]
        cells.append((y, x, ch, "white" if (x + frame) % 4 else "cyan"))


def _dust(cells, cx, ground, height, cap_w, frame):
    """Dust cloud at the base, frayed at the edges."""
    rows = [(ground - 1, 1.3)] + ([(ground - 2, 0.8)] if height >= 7 else [])
    for y, scale in rows:
        half = max(2, int(cap_w * scale / 2))
        for x in range(cx - half, cx + half + 1):
            n = _noise(x, y, frame)
            frayed = n % 5 == 0 and abs(x - cx) > half * 0.7
            if frayed:
                continue
            cells.append((y, x, _pick("~^°*~", n), "grey" if n % 3 else "orange"))


def mushroom_width(W, H, f, twin=False):
    """Approximate width taken on the ground (to place the units and the inspector)."""
    cap_w = _dimensions(W, H, f, twin)[1]
    if twin:
        return min(W - 2, 2 * cap_w + 2)
    return min(W - 2, int(cap_w * 1.3))


# --- Units and enemies ------------------------------------------------------------
# Two frames per sprite (animation), lines bottom-aligned, facing right.

UNIT_ART = {
    "soldier": ([" o ", "/|=", "/ \\"], [" o ", "/|=", " | "]),
    "mortar": (["  /", " / ", "[_]"], ["  *", " / ", "[_]"]),
    "artillery": (["    _", " __//", "(o)_)"], ["    °", " __//", "(o)_)"]),
    "tank": (["  __   ", " [__]==", "(oooo) "], ["  __   ", " [__]==", "(OoOo) "]),
    "bomber": (["  _\\ ", "=[__>"], ["  _\\ ", "-[__>"]),
    "sub": (["   _|_  ", "~<_____)"], ["   _|_  ", "<_____)~"]),
    "icbm": ([" /\\ ", " |!|", "_|_|_"], [" /\\ ", " |!|", "_|_|_"]),
    "reactor": ([" .-. ", "(@@@)", "|___|"], [" .-. ", "(@*@)", "|___|"]),
    "blackhole": ([" .-. ", "( @ )", " `-' "], [" .-. ", "( O )", " `-' "]),
}

ENEMY_ART = {
    "saboteur": (["  o $", " /|/ ", " / \\ "], ["  o $", " /|/ ", "  |  "]),
    "drone": (["-=-=-", " [o] "], ["=-=-=", " [o] "]),
    "hippies": (["[PAIX]  ", " \\o/ \\o/", "  |   | ", " / \\ / \\"],
                ["[PAIX]  ", " \\o/ \\o/", "  |   | ", "  |   | "]),
    "commando": ([" .o. ", "/|==-", "/ \\  "], [" .o. ", "/|==-", " |   "]),
}

_MIRRORED_CHARS = str.maketrans("()<>[]{}/\\`'", ")(><][}{\\/'`")


def sprite(frames, t, mirror=False):
    """Current frame of a sprite (2 frames/s), flipped horizontally if mirror."""
    rows = frames[int(t * 2) % len(frames)]
    w = max(map(len, rows))
    rows = [r.ljust(w) for r in rows]
    return [r[::-1].translate(_MIRRORED_CHARS) for r in rows] if mirror else rows


def sprite_width(frames):
    return max(len(r) for f in frames for r in f)


# --- Big-letter font (3 lines, half blocks) ------------------------------------------

_BIG_ROWS = 3
BIG_FONT = {
    "A": ["▄▀▄", "█▀█", "▀ ▀"], "B": ["█▀▄", "█▀▄", "▀▀ "], "C": ["▄▀▀", "█  ", " ▀▀"],
    "D": ["█▀▄", "█ █", "▀▀ "], "E": ["█▀▀", "█▀ ", "▀▀▀"], "F": ["█▀▀", "█▀ ", "▀  "],
    "G": ["█▀▀", "█ ▄", "▀▀▀"], "H": ["█ █", "█▀█", "▀ ▀"], "I": ["▀█▀", " █ ", "▀▀▀"],
    "J": ["  █", "  █", "▀▀ "], "K": ["█ █", "█▀▄", "▀ ▀"], "L": ["█  ", "█  ", "▀▀▀"],
    "M": ["█▄ ▄█", "█ ▀ █", "▀   ▀"], "N": ["█▄ █", "█ ▀█", "▀  ▀"], "O": ["█▀█", "█ █", "▀▀▀"],
    "P": ["█▀█", "█▀▀", "▀  "], "Q": ["█▀█", "█ █", "▀▀▄"], "R": ["█▀█", "█▀▄", "▀ ▀"],
    "S": ["█▀▀", "▀▀█", "▀▀▀"], "T": ["▀█▀", " █ ", " ▀ "], "U": ["█ █", "█ █", "▀▀▀"],
    "V": ["█ █", "█ █", " ▀ "], "W": ["█ █ █", "█ █ █", " ▀ ▀ "], "X": ["█ █", "▄▀▄", "▀ ▀"],
    "Y": ["█ █", " █ ", " ▀ "], "Z": ["▀▀█", "▄▀ ", "▀▀▀"],
    "0": ["█▀█", "█ █", "▀▀▀"], "1": ["▄█ ", " █ ", "▀▀▀"], "2": ["▀▀█", "█▀▀", "▀▀▀"],
    "3": ["▀▀█", " ▀█", "▀▀▀"], "4": ["█ █", "▀▀█", "  ▀"], "5": ["█▀▀", "▀▀█", "▀▀▀"],
    "6": ["█▀▀", "█▀█", "▀▀▀"], "7": ["▀▀█", "  █", "  ▀"], "8": ["█▀█", "█▀█", "▀▀▀"],
    "9": ["█▀█", "▀▀█", "▀▀▀"],
    "!": ["█", "▀", "▀"], "?": ["▀▀█", " █▀", " ▀ "], ".": [" ", " ", "▀"], ":": [" ", "▀", "▀"],
    "+": ["   ", "▄█▄", " ▀ "], "-": ["   ", "▀▀▀", "   "], "×": ["   ", "▀▄▀", "▀ ▀"],
    "'": ["▀", " ", " "], " ": [" ", " ", " "],
}


def _plain(text):
    """Uppercase without accents (É → E, Œ → OE) for the big font."""
    text = text.upper().replace("Œ", "OE").replace("Æ", "AE")
    return "".join(c for c in unicodedata.normalize("NFD", text) if not unicodedata.combining(c))


def big_text(text):
    """Three lines of big letters; unknown characters are skipped."""
    glyphs = [BIG_FONT[c] for c in _plain(text) if c in BIG_FONT]
    if not glyphs:
        return [""] * _BIG_ROWS
    return ["".join(g[row] + " " for g in glyphs).rstrip() for row in range(_BIG_ROWS)]


def big_width(text):
    return max(len(row) for row in big_text(text))
