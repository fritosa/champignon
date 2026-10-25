"""Curses interface: panel rendering, animations and key handling."""

import curses
import math
import random
import sys
import textwrap

import art
import data
import events
from game import AUDIO_MODES, VOLUME_STEP, fmt, need
from screens import ENTER_KEYS, KEY_TAB, STORIES, STORY_GUARD, intro

# Minimum delay between two plays of the same sound.
SFX_GAP = dict(level=0.6, hit=0.08, achievement=0.6, crit=0.15, buy=0.12, fizzle=0.5, jam=0.15)
DEFAULT_SFX_GAP = 0.05
MSG_TIME = 6
BIG_TIME = 3  # title in big letters (important messages: all of MSG_TIME)
AUDIO_LABELS = dict(all="musique + effets", sfx="effets seuls", off="muet")
MSG_COLORS = dict(info="white", good="green", bad="red", loot="cyan", star="yellow", epic="magenta")
KEY_HELP = [
    ("ESPACE", "Détonateur : gagne de la radiation (attention à la surchauffe)."),
    ("F", "Masse critique : lance la jauge, puis F au bon moment."),
    ("X", "Chasse l'inspecteur de l'ONU et les ennemis (à marteler)."),
    ("lettre", "Attrape le coffre de l'avion en tapant sa lettre."),
    ("1-5", "Change d'onglet (ou choisit un bonus de coffre / une mutation)."),
    ("↑↓ / j k", "Sélectionne une ligne."),
    ("Entrée", "Achète ou active la ligne sélectionnée."),
    ("M", "Quantité achetée par Entrée : ×1, ×10, ×100 ou max."),
    ("Tab ←→", "Onglet suivant / précédent."),
    ("0", "Son : " + " → ".join(AUDIO_LABELS[m] for m in AUDIO_MODES) + "."),
    ("+ / -", f"Volume général (par paliers de {VOLUME_STEP} %)."),
    ("Q", "Sauvegarde et quitte."),
]
# Screen shake duration per sound (the boom flash lasts 0.8 s).
SHAKE = dict(boom=1.3, perfect=0.6, prestige=0.6, supernova=1.5)
QTY = [1, 10, 100, "max"]
TABS = ["Unités", "Talents", "Prestige", "Succès", "Infos"]
MIN_W, MIN_H = 60, 20
MAX_FLOATS = 6  # floating numbers shown at once


def row(text, attr=0, action=None, desc="", sel=True, sprite=None):
    """sprite: (unit id, unlocked) to draw its art next to the description."""
    return dict(text=text, attr=attr, action=action, desc=desc, sel=sel, sprite=sprite)


class UI:
    def __init__(self, scr, game, sound=None):
        self.scr = scr
        self.sound = sound
        self.sfx_last = {}
        self.qty = 1
        self.shake_until = 0.0
        self.title = None  # None: terminal title disabled
        self.title_at = -9.0
        self.g = game
        self.tab = 0
        self.sel = [0] * len(TABS)
        self.scroll = [0] * len(TABS)
        self.particles = []
        self.floats = []
        self.spawn_acc = {}
        self.news = ""
        self.news_off = 0.0
        self.confirm = None
        self.t = 0.0
        self.h = self.w = 0
        self.tab_w = 40
        self.rng = random.Random()
        self.colors = {}
        self._init_colors()

    # --- Low level -----------------------------------------------------------------

    def _init_colors(self):
        if not curses.has_colors():
            return
        curses.start_color()
        try:
            curses.use_default_colors()
            bg = -1
        except curses.error:
            bg = curses.COLOR_BLACK
        many = curses.COLORS >= 256
        fgs = dict(red=curses.COLOR_RED, orange=208 if many else curses.COLOR_YELLOW, yellow=curses.COLOR_YELLOW,
                   green=curses.COLOR_GREEN, cyan=curses.COLOR_CYAN, blue=curses.COLOR_BLUE,
                   magenta=curses.COLOR_MAGENTA, white=curses.COLOR_WHITE, grey=244 if many else curses.COLOR_WHITE)
        for pair, (name, fg) in enumerate(fgs.items(), 1):
            curses.init_pair(pair, fg, bg)
            self.colors[name] = curses.color_pair(pair)
        if not many:
            self.colors["grey"] |= curses.A_DIM
        curses.init_pair(20, curses.COLOR_BLACK, curses.COLOR_YELLOW)
        curses.init_pair(21, curses.COLOR_WHITE, curses.COLOR_RED)
        self.colors["flash"] = curses.color_pair(20)
        self.colors["alert"] = curses.color_pair(21) | curses.A_BOLD
        for pair, (kind, banner_bg) in enumerate(MSG_COLORS.items(), 22):
            text_fg = curses.COLOR_WHITE if banner_bg in ("red", "magenta") else curses.COLOR_BLACK
            curses.init_pair(pair, text_fg, fgs[banner_bg])
            self.colors["msg_" + kind] = curses.color_pair(pair) | curses.A_BOLD

    def c(self, name):
        return self.colors.get(name, 0)

    def blink(self, period=0.5):
        return int(self.t / period) % 2 == 0

    def put(self, y, x, s, attr=0):
        if y < 0 or y >= self.h or x >= self.w or not s:
            return
        if x < 0:
            s, x = s[-x:], 0
        last_line = y == self.h - 1  # writing the bottom-right corner makes addstr fail
        s = s[:self.w - x - (1 if last_line else 0)]
        try:
            self.scr.addstr(y, x, s, attr)
        except curses.error:
            pass

    def box(self, y, x, h, w, title="", attr=0):
        if h < 2 or w < 2:
            return
        self.put(y, x, "┌" + "─" * (w - 2) + "┐", attr)
        for i in range(1, h - 1):
            self.put(y + i, x, "│", attr)
            self.put(y + i, x + w - 1, "│", attr)
        self.put(y + h - 1, x, "└" + "─" * (w - 2) + "┘", attr)
        if title:
            self.put(y, x + 2, f" {title} ", attr | curses.A_BOLD)

    def bar(self, frac, width, full="█", empty="░"):
        n = max(0, min(width, int(round(frac * width))))
        return full * n + empty * (width - n)

    # --- Rendering ---------------------------------------------------------------------

    def draw(self, dt):
        self.t += dt
        self.play_sfx()
        self.h, self.w = self.scr.getmaxyx()
        self.scr.erase()
        if self.w < MIN_W or self.h < MIN_H:
            self.put(0, 0, f"Terminal trop petit ({self.w}×{self.h}, min {MIN_W}×{MIN_H}).")
            self.put(1, 0, f"Niveau {self.g.level}  ☢ {fmt(self.g.radiation)}  — ESPACE marche quand même.")
            self.scr.refresh()
            return
        left_w = max(30, int(self.w * 0.45))
        right_w = self.w - left_w
        stats_h = 10
        news_y = self.h - 1
        self.draw_mushroom(0, 0, news_y, left_w, dt)
        self.draw_stats(0, left_w, stats_h, right_w)
        self.draw_tab(stats_h, left_w, news_y - stats_h, right_w)
        self.draw_news(news_y, dt)
        self.draw_qte(0, 0, news_y, left_w)
        self.draw_mutation()
        self.draw_chest()
        self.draw_report()
        self.scr.refresh()
        self.update_title()

    def show_story(self):
        """Pending challenge story (start or success): full screen, shown once."""
        if self.g.pending_story:
            story, self.g.pending_story = STORIES.get(self.g.pending_story), None
            if story:
                intro(self.scr, self.sound, story, guard=STORY_GUARD, then="continuer")

    def sfx(self, name):
        self.g.sfx.append(name)

    def music_track(self):
        g = self.g
        if g.inspector or g.enemies or g.qte or g.overheated or g.frozen:
            return "tension"
        if g.challenge or (g.exploded and g.level >= g.cap / 2):
            return "epic"
        return "calm"

    def play_sfx(self):
        g = self.g
        if self.sound:
            self.sound.muted = g.audio == "off"
            self.sound.music_on = g.audio == "all"
            self.sound.volume = g.volume / 100
            self.sound.set_music(self.music_track())
        played = set()
        while g.sfx:
            name = g.sfx.popleft()
            if name in SHAKE:
                self.shake_until = max(self.shake_until, self.t + SHAKE[name])
            key = "click" if name.startswith("click") else name  # press variants share one delay
            too_soon = self.t - self.sfx_last.get(key, -9) < SFX_GAP.get(key, DEFAULT_SFX_GAP)
            if not self.sound or key in played or too_soon:
                continue
            played.add(key)
            self.sfx_last[key] = self.t
            self.sound.play(name)

    def draw_mushroom(self, y0, x0, H, W, dt):
        g = self.g
        border = self.c("grey")
        if g.overheated and self.blink(0.2):
            border = self.c("red") | curses.A_BOLD
        elif g.frozen:
            border = self.c("blue") | curses.A_BOLD
        self.box(y0, x0, H, W, "Champignon atomique", border)
        iy, ix, IH, IW = y0 + 1, x0 + 1, H - 2, W - 2
        ground = IH - 1

        if g.time < g.flash_until:
            for y in range(IH):
                self.put(iy + y, ix, "".join(self.rng.choice("█▓▒░") for _ in range(IW)),
                         self.c("flash") if self.blink(0.1) else self.c("white") | curses.A_BOLD)
            self.draw_banners(iy, ix, IW)
            return

        if self.t < self.shake_until and IW > 4:
            shift = self.rng.choice((0, 1))
            ix, IW = ix + shift, IW - 1  # the lost width keeps the shift inside the frame
        self.put(iy + ground, ix, "▁" * IW, self.c("grey"))
        if not g.exploded:
            for y, x, ch, col in art.bomb(IW, IH):
                self.put(iy + y, ix + x, ch, self.c(col))
            if self.blink(0.6):
                msg = "Appuie sur ESPACE !"
                self.put(iy + IH // 2, ix + (IW - len(msg)) // 2, msg, self.c("yellow") | curses.A_BOLD)
            self.draw_banners(iy, ix, IW)
            return

        self.draw_sky(iy, ix, IW, ground)
        size = art.size_fraction(g.level, g.cap)
        twin = g.has_mut("twin")
        for y, x, ch, col in art.mushroom(IW, IH, size, self.t, g.skin, twin):
            if 0 <= y < IH and 0 <= x < IW:
                self.put(iy + y, ix + x, ch, self.c(col))

        foot_w = art.mushroom_width(IW, IH, size, twin)
        gap = max(3, foot_w // 6)
        self.draw_army(iy, ix, IW, ground, max(2, foot_w // 10))
        banners = self.banner_lines(IW)
        self.update_particles(iy, ix, IW, IH, ground, size, gap, dt, 1 + len(banners))
        self.draw_inspector(iy, ix, IW, ground, min(IW - 6, IW // 2 + gap))
        self.draw_enemy(iy, ix, IW, ground, gap)
        self.draw_plane(iy, ix, IW)
        self.draw_banners(iy, ix, IW, banners)

    def big_lines(self, title, kind, IW, flashing=False):
        """Title in big letters (3 lines of equal width), or in spaced letters if it does not fit."""
        attr = self.c(MSG_COLORS[kind]) | curses.A_BOLD
        if flashing and self.blink(0.15):
            attr = self.c("white") | curses.A_BOLD
        rows = art.big_text(title)
        width = max(len(r) for r in rows)
        if not width:
            return [(title[:IW - 2], attr)]
        if width > IW - 2:
            return [(" ".join(art._plain(title)), attr)]
        return [(r.ljust(width), attr) for r in rows]

    def draw_sky(self, iy, ix, IW, ground):
        """Backdrop that evolves with the level: clouds, then stars, then planets (fixed positions per size)."""
        lvl, top = self.g.level, ground - 3
        if top < 2 or IW < 10:
            return
        rng = random.Random(IW * 1000 + ground)
        if lvl < 100:
            for i in range(max(1, IW // 25)):
                y, x0 = rng.randint(1, top), rng.randint(0, IW)
                x = int(x0 + self.t * (0.5 + 0.3 * i)) % (IW + 6) - 6
                self.put(iy + y, ix + max(0, x), "(~~)"[max(0, -x):max(0, min(4, IW - x))], self.c("grey"))
            return
        for i in range(IW * top // 30):
            y, x = rng.randint(1, top), rng.randint(0, IW - 1)
            ch = "✦" if i % 7 == 0 else "·"
            attr = self.c("white") if (i + int(self.t * 2)) % 9 == 0 else self.c("grey")
            self.put(iy + y, ix + x, ch, attr)
        if lvl >= 1000:
            for ch, col in (("◯", "cyan"), ("♄", "yellow"), ("☾", "white")):
                self.put(iy + rng.randint(1, top), ix + rng.randint(0, IW - 1), ch, self.c(col) | curses.A_BOLD)

    def update_title(self):
        """Level in the terminal title (OSC sequence), at most once per second and only when it changes."""
        if self.title is None or self.t - self.title_at < 1:
            return
        text = f"☢ Niv. {self.num(self.g.level)} — Champignon atomique"
        if text != self.title:
            self.title, self.title_at = text, self.t
            try:
                sys.__stdout__.write(f"\x1b]0;{text}\x07")
                sys.__stdout__.flush()
            except (OSError, ValueError, AttributeError):
                self.title = None

    def draw_banners(self, iy, ix, IW, lines=None):
        for i, (text, attr) in enumerate(self.banner_lines(IW) if lines is None else lines):
            text = f" {text} "[:IW]
            self.put(iy + 1 + i, ix + max(0, (IW - len(text)) // 2), text, attr)

    def alert_attr(self, calm):
        return self.c("alert") if self.blink(0.3) else self.c(calm)

    def banner_lines(self, IW):
        """Active alerts then latest messages (most recent on top), stacked under the panel edge."""
        g = self.g
        lines = []
        if g.inspector:
            lines.append(("⚠ Un inspecteur ! Martèle X ! ⚠", self.alert_attr("msg_info")))
        if g.enemies:
            who = events.ENEMIES[g.enemies[0]["kind"]]["name"] if len(g.enemies) == 1 else f"{len(g.enemies)} ennemis"
            lines.append((f"⚠ {who} ! Martèle X ! ⚠", self.alert_attr("msg_bad")))
        if g.frozen:
            lines.append((f"❄ PRODUCTION GELÉE {g.freeze_until - g.time:.0f}s ❄", self.c("msg_loot")))
        recent = [m for m in g.messages if g.time - m[1] < MSG_TIME][-3:]
        for i, m in enumerate(reversed(recent)):
            msg, when, kind = m[:3]
            title = m[3] if len(m) > 3 else None
            age = g.time - when
            important = kind in ("epic", "star")
            if i == 0 and title and age < (MSG_TIME - 1 if important else BIG_TIME):
                lines += self.big_lines(title, kind, IW, flashing=important and age < 1)
            attr = self.c("msg_" + kind)
            if age < 0.6 and self.blink(0.1):
                attr ^= curses.A_REVERSE
            elif age > MSG_TIME - 1:
                attr = self.c(MSG_COLORS[kind]) | curses.A_BOLD
            parts = textwrap.wrap(msg, max(10, IW - 4))[:2]
            lines += [(part.center(max(map(len, parts))), attr) for part in parts]
        return lines

    def draw_sprite(self, iy, ix, IW, bottom, x, rows, attr, opaque=False):
        """Sprite whose last line is at `bottom`, clipped to the panel edges.
        Opaque: inner spaces erase the backdrop (readable in front of the dust)."""
        top = bottom - len(rows) + 1
        for i, line in enumerate(rows):
            y = top + i
            body = line.strip() if opaque else ""
            indent = len(line) - len(line.lstrip())
            for j, ch in enumerate(line):
                inside = opaque and body and indent <= j < indent + len(body)
                if (ch != " " or inside) and 0 <= x + j < IW and y >= 0:
                    self.put(iy + y, ix + x + j, ch, attr)

    def draw_army(self, iy, ix, IW, ground, gap):
        """One sprite per owned unit type, a 2nd from 10 units and a 3rd from 100 if there is room.
        Placed from the edges toward the center, leaving the stem clear; those on the right face left.
        Bombers fly separately."""
        g = self.g
        owned = [u for u in data.UNITS if g.units[u["id"]] and u["id"] != "bomber"]
        wanted = [(u, copy) for copy in range(3) for u in owned if g.units[u["id"]] >= 10 ** copy]
        LEFT, RIGHT = 0, 1
        bottoms = [ground - 1] + ([ground - 5] if ground >= 12 else [])
        slots = [(bottom, side) for bottom in bottoms for side in (LEFT, RIGHT)]
        cursor = {slot: (1 if slot[1] == LEFT else IW - 1) for slot in slots}
        next_slot = 0
        for u, copy in wanted:
            frames = art.UNIT_ART[u["id"]]
            w = art.sprite_width(frames)
            for tries in range(len(slots)):
                slot = slots[(next_slot + tries) % len(slots)]
                bottom, side = slot
                x = cursor[slot] if side == LEFT else cursor[slot] - w
                if (side == LEFT and x + w <= IW // 2 - gap) or (side == RIGHT and x >= IW // 2 + gap):
                    cursor[slot] = x + w + 1 if side == LEFT else x - 1
                    self.draw_sprite(iy, ix, IW, bottom, x, art.sprite(frames, self.t + copy * 0.3, side == RIGHT),
                                     self.c(u["color"]) | curses.A_BOLD, opaque=True)
                    next_slot = (next_slot + tries + 1) % len(slots)
                    break
        frames = art.UNIT_ART["bomber"]
        for i in range(min(2, g.units["bomber"])):
            x = int(self.t * 8 + i * 23) % (IW + 6) - 6
            self.draw_sprite(iy, ix, IW, 3 + 3 * i, x, art.sprite(frames, self.t), self.c("cyan") | curses.A_BOLD)

    def update_particles(self, iy, ix, IW, IH, ground, size, gap, dt, min_row=1):
        """Unit projectiles toward the mushroom, then floating numbers above its head."""
        g = self.g
        top = ground - art.mushroom_height(IH, size)
        if not g.frozen:
            for u in data.UNITS:
                n = g.units[u["id"]]
                if not n:
                    continue
                per_second = min(5, 0.4 + math.log2(1 + n) * 0.6)
                acc = self.spawn_acc.get(u["id"], 0) + dt * per_second
                while acc >= 1 and len(self.particles) < 60:
                    acc -= 1
                    if u["id"] == "bomber":
                        sx, sy, peak = self.rng.randint(0, IW - 1), 2, 0
                    else:
                        left = self.rng.random() < 0.5
                        sx = self.rng.randint(1, IW // 2 - gap) if left else self.rng.randint(IW // 2 + gap, IW - 2)
                        sy, peak = ground - 1, self.rng.uniform(2, 5)
                    tx = IW // 2 + self.rng.randint(-2, 2)
                    ty = self.rng.randint(min(top + 1, ground - 2), ground - 2)
                    self.particles.append(dict(sx=sx, sy=sy, tx=tx, ty=ty, peak=peak, age=0.0,
                                               dur=self.rng.uniform(0.7, 1.3), ch=u["proj"], col=u["color"]))
                self.spawn_acc[u["id"]] = min(acc, 1)
        alive = []
        for p in self.particles:
            p["age"] += dt
            flight = p["age"] / p["dur"]
            if flight >= 1.2:
                continue
            alive.append(p)
            if flight >= 1:
                self.put(iy + p["ty"], ix + p["tx"], "✸", self.c("yellow") | curses.A_BOLD)
                continue
            x = int(p["sx"] + (p["tx"] - p["sx"]) * flight)
            arc = 4 * p["peak"] * flight * (1 - flight)
            y = int(p["sy"] + (p["ty"] - p["sy"]) * flight - arc)
            if 0 <= y < IH and 0 <= x < IW:
                self.put(iy + y, ix + x, p["ch"], self.c(p["col"]) | curses.A_BOLD)
        self.particles = alive

        floats = []
        for fl in self.floats:
            fl["age"] += dt
            if fl["age"] < 0.9:
                floats.append(fl)
                y = int(top - 1 - fl["age"] * 4)
                x = IW // 2 + fl["dx"] - len(fl["text"]) // 2
                self.put(iy + max(min_row, y), ix + max(0, min(IW - len(fl["text"]), x)), fl["text"], fl["attr"])
        self.floats = floats

    def draw_plane(self, iy, ix, IW):
        p = self.g.plane
        if not p:
            return
        progress = (self.g.time - p["start"]) / p["dur"]
        x = int(IW - progress * (IW + 8))
        self.put(iy, ix + max(0, x), "✈", self.c("white") | curses.A_BOLD)
        label = f"[{p['letter'].upper()}]"
        attr = (self.c("yellow") if self.blink(0.25) else self.c("cyan")) | curses.A_BOLD
        if 0 <= x + 2 <= IW - len(label):
            self.put(iy, ix + x + 2, label, attr)

    def draw_inspector(self, iy, ix, IW, ground, target):
        ins = self.g.inspector
        if not ins:
            return
        progress = min(1.0, (self.g.time - ins["start"]) / ins["dur"])
        start = IW - 6
        x = int(start - (start - target) * progress)
        x = max(0, min(IW - 5, x))
        self.put(iy + ground - 2, ix + x + 1, " o/" if self.blink(0.3) else "\\o ", self.c("magenta") | curses.A_BOLD)
        self.put(iy + ground - 1, ix + x, "[ONU]", self.c("magenta") | curses.A_BOLD)
        hits = "x" * ins["hits"] + "·" * (events.INSPECTOR_HITS - ins["hits"])
        self.put(iy + ground - 3, ix + x, hits, self.c("red"))

    def draw_enemy(self, iy, ix, IW, ground, gap):
        """Enemies coming from the left (drones through the sky), remaining hits above each one.
        The front one stops at the foot of the mushroom, the others queue up behind it."""
        g = self.g
        for sky in (False, True):
            target = IW // 2 - gap
            for en in sorted((e for e in g.enemies if events.ENEMIES[e["kind"]]["sky"] == sky),
                             key=lambda e: -events.enemy_progress(g, e)):
                e = events.ENEMIES[en["kind"]]
                frames = art.ENEMY_ART[en["kind"]]
                w = art.sprite_width(frames)
                target = max(0, target - w)
                progress = max(0.0, min(1.0, events.enemy_progress(g, en)))
                if en["kind"] == "hippies":
                    progress = min(1.0, progress * 4)  # they arrive fast then stage their sit-in
                x = int(-w + (target + w) * progress)
                target = min(target, x) - 1
                rows = art.sprite(frames, self.t)
                bottom = 2 + len(rows) if sky else ground - 1
                self.draw_sprite(iy, ix, IW, bottom, x, rows, self.c("red") | curses.A_BOLD, opaque=True)
                if x + w > 0:
                    hits = "x" * en["hits"] + "·" * (e["hits"] - en["hits"])
                    self.put(iy + bottom - len(rows), ix + max(0, x), hits, self.c("yellow"))

    def num(self, x):
        """Formatted number, hidden during the Fog of War challenge."""
        return "???" if self.g.challenge == "blind" else fmt(x)

    def draw_stats(self, y0, x0, H, W):
        g = self.g
        self.box(y0, x0, H, W, "Statistiques", self.c("grey"))
        iy, ix, IW = y0 + 1, x0 + 2, W - 4
        lines = []

        head = f"NIVEAU {self.num(g.level)} / {fmt(g.cap)}"
        challenge = data.CHALLENGE_BY_ID.get(g.challenge)
        tag = ""
        if challenge:
            tag = f"Défi {challenge['name']}"
            if challenge["time"]:
                left = max(0, challenge["time"] - g.run_time)
                tag += f" {left // 60:.0f}:{left % 60:02.0f}"
        elif g.prestige_count:
            tag = f"Prestige ×{g.prestige_count}"
        lines.append([(head, self.c("white") | curses.A_BOLD), (" " * max(1, IW - len(head) - len(tag)), 0),
                      (tag, self.c("magenta"))])

        if g.level >= g.cap:
            lines.append([("MAX ! Prestige disponible → onglet 3", self.c("green") | curses.A_BOLD
                           if self.blink() else self.c("green"))])
        else:
            frac = g.progress / need(max(1, g.level)) if g.exploded else 0
            percent = " ???" if g.challenge == "blind" else f" {int(frac * 100):3d}%"
            lines.append([(self.bar(frac, IW - 6), self.c("orange")), (percent, 0)])

        prod = f"+{self.num(g.production())}/s"
        lines.append([(f"☢ {self.num(g.radiation)}   ", self.c("yellow") | curses.A_BOLD),
                      ("GELÉ" if g.frozen else prod, self.c("blue") if g.frozen else self.c("green"))])
        auto = f"   Auto {g.tr('t_auto')}/s" if g.tr("t_auto") else ""
        lines.append([(f"Détonation {self.num(g.click_value())}   Crit {int(g.crit_chance() * 100)}%{auto}", 0)])
        lines.append([(f"PT {g.pt}", self.c("cyan") | curses.A_BOLD),
                      (f"   ⚛ {fmt(g.isotopes)}", self.c("magenta") | curses.A_BOLD),
                      (f"   ★ {len(g.achievements)}/{len(events.ACHIEVEMENTS)}", self.c("yellow")),
                      (f"   ✦ {fmt(g.stars)}" if g.supernova_unlocked else "", self.c("magenta") | curses.A_BOLD)])

        if g.overheated:
            lines.append([(f"SURCHAUFFE ! {g.overheat_until - g.time:.1f}s",
                           self.c("alert") if self.blink(0.2) else self.c("red") | curses.A_BOLD)])
        else:
            steps = g.max_combo() - 1
            step_w = max(1, min(20, IW - 16) // steps)
            filled = int(g.combo * step_w)
            lines.append([(f"Combo ×{g.combo_mult():.1f} ", 0), ("■" * filled, self.c("red") | curses.A_BOLD),
                          ("□" * (steps * step_w - filled), self.c("grey"))])

        if g.qte:
            lines.append([("MASSE CRITIQUE EN COURS !", self.c("red") | curses.A_BOLD)])
        elif g.crit_gauge >= 1:
            lines.append([("Masse critique PRÊTE → appuie sur F", self.c("green") | curses.A_BOLD
                           if self.blink(0.4) else self.c("yellow"))])
        else:
            lines.append([("Masse critique ", 0), (self.bar(g.crit_gauge, max(5, IW - 20)), self.c("red")),
                          (f" {int(g.crit_gauge * 100)}%", 0)])

        if g.buffs:
            txt = "  ".join(f"{b['name']} {b['until'] - g.time:.0f}s" for b in g.buffs)
            lines.append([(txt, self.c("yellow") | curses.A_BOLD)])
        elif g.mutations:
            names = ", ".join(events.MUTATION_BY_ID[m][1] for m in g.mutations)
            lines.append([("Mutations : " + names, self.c("green"))])

        for i, segs in enumerate(lines[:H - 2]):
            x = ix
            for text, attr in segs:
                text = text[:max(0, ix + IW - x)]
                self.put(iy + i, x, text, attr)
                x += len(text)

    # --- Tabs ----------------------------------------------------------------------------

    def tab_has_news(self, i):
        g = self.g
        if i == 0:
            return any(g.can_buy_unit(u["id"]) for u in data.UNITS)
        if i == 1:
            return any(g.talent_available(t["id"]) and g.talent_allowed(t["id"]) and g.pt >= g.talent_cost(t["id"])
                       for t in data.TALENTS)
        if i == 2:
            return g.can_prestige() or g.can_supernova() or (g.prestige_unlocked and any(
                g.isotopes >= g.ptalent_cost(p["id"]) for p in data.PTALENTS)) or any(
                g.stars >= g.stalent_cost(s["id"]) for s in data.STALENTS)
        return False

    def rows(self):
        return [self.unit_rows, self.talent_rows, self.prestige_rows, self.achievement_rows,
                self.info_rows][self.tab]()

    def unit_rows(self):
        g = self.g
        rows = []
        if g.challenge == "pacifist":
            rows.append(row("Défi Pacifiste : unités interdites !", self.c("magenta"), sel=False))
        nw = max(8, self.tab_w - 28)
        teased = False
        for u in data.UNITS:
            uid = u["id"]
            if g.unit_unlocked(uid):
                n = g.units[uid]
                k = max(1, g.affordable(uid)) if self.qty == "max" else self.qty
                cost = g.unit_cost_n(uid, k)
                text = (f"{u['name'][:nw]:<{nw}} x{n:<4} {self.num(g.unit_prod(uid) * n):>7}/s "
                        f"{fmt(cost):>7} ☢")
                ok = g.can_buy_unit(uid) and g.radiation >= cost
                attr = self.c("white") | curses.A_BOLD if ok else self.c("grey")
                desc = f"{u['name']} : +{self.num(g.unit_prod(uid))} ☢/s chacun. Entrée : acheter ×{k} (M pour changer)."
                nxt = g.next_milestone(uid)
                if nxt:
                    desc += f" Palier à {nxt} : production ×2."
                if not ok and g.challenge not in ("pacifist", "blind"):
                    prod = g.production()
                    desc += (f" Encore {_dur((cost - g.radiation) / prod)} d'attente." if prod > 0 else
                             f" Il manque {fmt(cost - g.radiation)} ☢.")
                rows.append(row(text, attr, ("unit", uid), desc, sprite=(uid, True)))
            elif not teased:
                teased = True
                rows.append(row(f"??? — talent « {u['unlock_name']} »", self.c("grey"), None,
                                "Débloque de nouvelles unités dans l'onglet Talents (2), branche Infanterie.",
                                sprite=(uid, False)))
        return rows

    def talent_rows(self):
        g = self.g
        rows = [row(f"Points de talent : {g.pt} PT", self.c("cyan") | curses.A_BOLD, sel=False)]
        for bid, label in data.BRANCHES:
            rows.append(row(f"── {label} " + "─" * 20, self.c("cyan"), sel=False))
            for t in data.TALENTS:
                if t["branch"] != bid:
                    continue
                tid, r, avail = t["id"], g.tr(t["id"]), g.talent_available(t["id"])
                if bid == "inf" and not r and not avail:
                    continue
                cost = "MAX" if r >= t["max"] else f"{g.talent_cost(tid)} PT"
                name = ("  " * (t["depth"] - 1) + "└ " if t["depth"] else "") + t["name"]
                nw = max(10, self.tab_w - 16)
                text = f"{name[:nw]:<{nw}} {r}/{t['max']:<2} {cost:>6}"
                if r >= t["max"]:
                    attr = self.c("green")
                elif not g.talent_allowed(tid):
                    attr = self.c("grey")
                elif avail and g.pt >= g.talent_cost(tid):
                    attr = self.c("yellow") | curses.A_BOLD
                elif avail:
                    attr = self.c("white")
                else:
                    attr = self.c("grey")
                desc = t["desc"] + self.qty_note()
                if not g.talent_allowed(tid):
                    desc = "Défi Ascète : talent interdit. " + desc
                if not avail and r < t["max"]:
                    desc += " Requiert : " + ", ".join(
                        f"{data.TALENT_BY_ID[q]['name']} {n}" for q, n in t["req"]) + "."
                rows.append(row(text, attr, ("talent", tid), desc))
        return rows

    def prestige_rows(self):
        g = self.g
        if not g.prestige_unlocked:
            grey = self.c("grey")
            return [row("VERROUILLÉ", self.c("red") | curses.A_BOLD, sel=False),
                    row(f"Atteins le niveau {fmt(g.cap)} pour débloquer le prestige.", grey, sel=False),
                    row("Tu y gagneras des isotopes (⚛) à investir", grey, sel=False),
                    row("dans un arbre qui rend les runs suivantes", grey, sel=False),
                    row("plus efficaces.", grey, sel=False)]
        rows = []
        reset_desc = "Réinitialise niveau, ☢, unités, talents et mutations. Garde ⚛, prestige, succès et skins."
        if g.can_prestige():
            rows.append(row(f"☢ PRESTIGE : +{g.prestige_gain()} ⚛ ☢",
                            self.c("green") | curses.A_BOLD, ("prestige",), reset_desc))
        else:
            rows.append(row(f"Prestige : atteins le niveau {fmt(g.cap)}", self.c("grey"), None, reset_desc))
        if g.prestige_count:
            rows.append(row("── DÉFIS " + "─" * 20, self.c("cyan"), sel=False))
            for c in data.CHALLENGES:
                done = c["id"] in g.challenges_done
                cur = " (en cours)" if g.challenge == c["id"] else ""
                note = " Lance une nouvelle run" + (" (et fait le prestige)." if g.can_prestige() else
                                                    " sans gain d'isotopes.")
                rows.append(row(f"{'✔' if done else '·'} Défi {c['name']}{cur}",
                                self.c("green") if done else self.c("white"), ("challenge", c["id"]),
                                f"{c['desc']} Récompense : {c['reward']}.{note}"))
            if g.challenge:
                rows.append(row("✘ Abandonner le défi en cours", self.c("red"), ("abandon",),
                                "Retire la contrainte du défi sans réinitialiser la run."))
        rows.append(row(f"Skin : {data.SKINS[g.skin]}  ({len(g.skins)}/{len(data.SKINS)})", self.c("magenta"),
                        ("skin",), "Entrée : changer de skin. Les défis et 5 prestiges en débloquent d'autres."))
        rows.append(row(f"── ARBRE DE PRESTIGE ({fmt(g.isotopes)} ⚛) " + "─" * 10, self.c("cyan"), sel=False))
        rows += self.tree_rows(data.PTALENTS, "ptalent", g.pr, g.ptalent_cost, g.isotopes, "⚛",
                               lambda pid: f" Niveau max actuel : {fmt(g.cap)}." if pid == "p_cap" else "")
        if g.supernova_unlocked:
            rows += self.supernova_rows()
        return rows

    def supernova_rows(self):
        g = self.g
        rows = [row(f"── SUPERNOVA ({fmt(g.stars)} ✦) " + "─" * 10, self.c("magenta") | curses.A_BOLD, sel=False)]
        desc = ("Réinitialise aussi les isotopes et l'arbre de prestige. Garde ✦, succès, skins et défis. "
                f"Gain : racine des isotopes gagnés depuis la dernière supernova / {data.SUPERNOVA_DIVISOR}.")
        if g.can_supernova():
            rows.append(row(f"✦ SUPERNOVA : +{g.supernova_gain()} ✦ ✦", self.c("magenta") | curses.A_BOLD,
                            ("supernova",), desc))
        else:
            rows.append(row(f"Supernova : atteins le niveau {fmt(data.MAX_CAP)}", self.c("grey"), None, desc))
        return rows + self.tree_rows(data.STALENTS, "stalent", g.sr, g.stalent_cost, g.stars, "✦")

    def tree_rows(self, talents, kind, rank, cost, budget, unit, extra_desc=lambda tid: ""):
        """Tree paid in isotopes or stars: name, rank, next rank cost; yellow if affordable."""
        rows = []
        nw = max(10, self.tab_w - 18)
        for t in talents:
            tid, r = t["id"], rank(t["id"])
            maxed = r >= t["max"]
            c = None if maxed else cost(tid)
            price = "MAX" if maxed else f"{fmt(c)} {unit}"
            text = f"{t['name'][:nw]:<{nw}} {r}/{t['max']:<2} {price:>8}"
            if maxed:
                attr = self.c("green")
            elif budget >= c:
                attr = self.c("yellow") | curses.A_BOLD
            else:
                attr = self.c("white")
            rows.append(row(text, attr, (kind, tid), t["desc"] + extra_desc(tid) + self.qty_note()))
        return rows

    def achievement_rows(self):
        g = self.g
        n = len(g.achievements)
        rows = [row(f"{n}/{len(events.ACHIEVEMENTS)} succès · +{2 * n} % de production", self.c("yellow") |
                    curses.A_BOLD, sel=False)]
        for aid, name, desc, _check in events.ACHIEVEMENTS:
            done = aid in g.achievements
            rows.append(row(f"{'★' if done else '·'} {name}", self.c("yellow") if done else self.c("grey"),
                            None, desc))
        found = g.synergies_found
        rows.append(row(f"── SYNERGIES ({len(found)}/{len(events.SYNERGIES)}) " + "─" * 10, self.c("magenta"),
                        sel=False))
        for sid, name, (a, b), desc in events.SYNERGIES:
            pair = f"{events.MUTATION_BY_ID[a][1]} + {events.MUTATION_BY_ID[b][1]}"
            if sid in found:
                active = " (active)" if g.has_syn(sid) else ""
                rows.append(row(f"✦ {name}{active}", self.c("magenta") | curses.A_BOLD, None, f"{pair} : {desc}"))
            else:
                rows.append(row("· ???", self.c("grey"), None,
                                "Synergie inconnue : choisis la bonne paire de mutations pendant une partie."))
        return rows

    def qty_note(self):
        if self.qty == 1:
            return ""
        return " Entrée : tous les rangs possibles !" if self.qty == "max" else f" Entrée : jusqu'à {self.qty} rangs."

    def info_rows(self):
        g, st = self.g, self.g.stats
        head, key, grey = self.c("cyan") | curses.A_BOLD, self.c("yellow") | curses.A_BOLD, self.c("white")
        rows = [row("── TOUCHES " + "─" * 20, head, sel=False)]
        for k, what in KEY_HELP:
            rows.append(row(_clip(f"{k:<9} {what}", self.tab_w - 2), key, None, what))
        best = st["best_prestige_time"]
        rows.append(row("── STATISTIQUES " + "─" * 15, head, sel=False))
        for label, value in [
            ("Temps de jeu total", _dur(st["total_time"])),
            ("Partie en cours", _dur(g.run_time)),
            ("Meilleur prestige", _dur(best) if best is not None else "—"),
            ("Prestiges", fmt(g.prestige_count)),
            ("Supernovas", fmt(g.supernova_count)),
            ("Niveau max atteint", self.num(st["max_level"])),
            ("Appuis sur ESPACE", fmt(st["clicks"])),
            ("Surchauffes", fmt(st["overheats"])),
            ("Fusions parfaites", fmt(st["perfects"])),
            ("Colis attrapés", fmt(st["drops"])),
            ("Inspecteurs chassés", fmt(st["inspectors"])),
            ("Ennemis repoussés", fmt(st["enemies"])),
            ("Scellés subis", fmt(st["frozen"])),
            ("Mutations choisies", fmt(st["mutations"])),
            ("Défis réussis", f"{len(g.challenges_done)}/{len(data.CHALLENGES)}"),
            ("Skins débloqués", f"{len(g.skins)}/{len(data.SKINS)}"),
            ("Son", f"{AUDIO_LABELS[g.audio]} · {g.volume} %"),
            ("Synergies découvertes", f"{len(g.synergies_found)}/{len(events.SYNERGIES)}"),
        ]:
            vw = max(10, len(value))
            nw = max(6, self.tab_w - 4 - vw)
            rows.append(row(f"{label[:nw]:<{nw}} {value:>{vw}}", grey, None, f"{label} : {value}"))
        rows.append(row("Revoir l'introduction", self.c("magenta"), ("intro",),
                        "Entrée : relire l'histoire du professeur Morille et de son champignon."))
        for ch in data.CHALLENGES:
            if ch["id"] in g.challenges_done and any((ch["id"], part) in STORIES for part in ("start", "end")):
                rows.append(row(f"Revoir le défi {ch['name']}", self.c("magenta"), ("story", ch["id"]),
                                f"Entrée : relire l'histoire du défi {ch['name']}."))
        if g.last_run:
            rows.append(row("── DERNIÈRE PARTIE " + "─" * 12, head, sel=False))
            for label, value in self.report_lines(g.last_run):
                text = f"{label} : {value}"
                rows.append(row(_clip(text, self.tab_w - 2), grey, None, text))
        return rows

    def draw_tab(self, y0, x0, H, W):
        self.tab_w = W - 4
        self.box(y0, x0, H, W, "", self.c("grey"))
        self.draw_tab_labels(y0, x0, W)
        self.draw_tab_help(y0 + H - 1, x0, W)
        self.draw_tab_body(y0 + 1, x0 + 2, H - 2, W - 4)

    def draw_tab_labels(self, y, x0, W):
        """Tab names on the top edge, shortened until they fit; • if something there is affordable."""
        names = ["???" if i == 2 and not self.g.prestige_unlocked else n for i, n in enumerate(TABS)]
        if self.qty != 1:
            names[0] += f" ×{self.qty}"
        dots = ["•" if self.tab_has_news(i) else "" for i in range(len(names))]
        for fmt_label in (lambda i, n: f" {i + 1}:{n}{dots[i]} ", lambda i, n: f"{i + 1}:{n[:5]}{dots[i]}",
                          lambda i, n: f"{i + 1}:{n[:3]}{dots[i]}", lambda i, n: f"{i + 1}{dots[i]}"):
            labels = [fmt_label(i, n) for i, n in enumerate(names)]
            if sum(len(lb) + 1 for lb in labels) <= W - 3:
                break
        x = x0 + 2
        for i, label in enumerate(labels):
            attr = curses.A_REVERSE | curses.A_BOLD if i == self.tab else self.c("white")
            self.put(y, x, label, attr)
            x += len(label) + 1

    def draw_tab_help(self, y, x0, W):
        """Key help on the bottom edge, less and less detailed as width shrinks."""
        sound_label = dict(all="♪", sfx="on", off="off")[self.g.audio] if self.sound and self.sound.ok else "indispo"
        if sound_label in ("♪", "on") and self.g.volume < 100:
            sound_label += f" {self.g.volume}%"
        for help_ in (f" Entrée acheter · M ×{self.qty} · 5 aide · 0 son {sound_label} · Q ",
                      f" M ×{self.qty} · 5 aide · 0 son {sound_label} · Q ",
                      f" M ×{self.qty} · 5 aide · Q ", " 5 aide "):
            if len(help_) <= W - 2:
                break
        self.put(y, x0 + max(1, (W - len(help_)) // 2), help_[:W - 2], self.c("grey"))

    def draw_tab_body(self, iy, ix, IH, IW):
        """Scrolling list of the tab rows, then under a rule the description of the selected row
        (with the unit sprite on the right if it fits)."""
        rows = self.rows()
        selectable = [i for i, r in enumerate(rows) if r["sel"]]
        cur = None
        if selectable:
            self.sel[self.tab] = max(0, min(self.sel[self.tab], len(selectable) - 1))
            cur = selectable[self.sel[self.tab]]
        desc = rows[cur]["desc"] if cur is not None else ""
        desc_attr = 0
        confirming = self.confirm and self.confirm[1] > self.t
        if confirming:
            desc = "Appuie encore sur Entrée pour confirmer !"
            desc_attr = self.c("red") | curses.A_BOLD
        sprite = rows[cur]["sprite"] if cur is not None and not confirming else None
        lines, art_rows, art_attr = self.desc_layout(desc, sprite, IH, IW)
        list_h = max(1, IH - 1 - len(lines))
        sc = self.scroll_to(cur, selectable, list_h)
        for i, r in enumerate(rows[sc:sc + list_h]):
            idx = sc + i
            prefix = "▶ " if idx == cur else "  "
            attr = r["attr"] | (curses.A_REVERSE if idx == cur else 0)
            self.put(iy + i, ix, (prefix + r["text"])[:IW], attr)
        if sc > 0:
            self.put(iy, ix + IW - 1, "↑", self.c("grey"))
        if sc + list_h < len(rows):
            self.put(iy + list_h - 1, ix + IW - 1, "↓", self.c("grey"))

        self.put(iy + list_h, ix, "─" * IW, self.c("grey"))
        for i, line in enumerate(lines):
            self.put(iy + list_h + 1 + i, ix, line, desc_attr)
        if art_rows:
            w = len(art_rows[0])
            self.draw_sprite(iy + list_h + 1, ix + IW - w, w, len(art_rows) - 1, 0, art_rows, art_attr)

    def desc_layout(self, desc, sprite, IH, IW):
        """Description lines (at least 2, the list keeps 5), padded with blank lines up to the sprite height;
        the sprite is dropped if it does not fit. Returns (lines, sprite, attribute)."""
        art_rows, art_attr = [], 0
        if sprite and IW >= 36:
            uid, unlocked = sprite
            art_rows = art.sprite(art.UNIT_ART[uid], self.t)
            art_attr = self.c(data.UNIT_BY_ID[uid]["color"]) | curses.A_BOLD if unlocked else self.c("grey")
        text_w = IW - (len(art_rows[0]) + 2 if art_rows else 0)
        min_list_h = 5
        max_desc_h = IH - 1 - min_list_h
        lines = textwrap.wrap(desc, text_w)
        n_desc = max(2, min(max(len(lines), len(art_rows)), max_desc_h))
        if len(art_rows) > n_desc:
            art_rows = []
            text_w = IW
            lines = textwrap.wrap(desc, IW)
            n_desc = max(2, min(len(lines), max_desc_h))
        if len(lines) > n_desc:
            lines = lines[:n_desc - 1] + [lines[n_desc - 1][:text_w - 1] + "…"]
        return lines + [""] * (n_desc - len(lines)), art_rows, art_attr

    def scroll_to(self, cur, selectable, list_h):
        """Tab scroll that keeps the selected row visible (at the very top for the first one)."""
        if not selectable:
            return 0
        sc = self.scroll[self.tab]
        if cur < sc:
            sc = cur
        elif cur >= sc + list_h:
            sc = cur - list_h + 1
        if cur == selectable[0]:
            sc = 0
        self.scroll[self.tab] = sc
        return sc

    # --- Overlays and banner -----------------------------------------------------------

    def clear(self, y, x, h, w):
        for i in range(h):
            self.put(y + i, x, " " * w)

    def popup(self, bh, bw, title, attr):
        """Empty centered framed window; returns its top-left corner."""
        y, x = (self.h - bh) // 2, (self.w - bw) // 2
        self.clear(y, x, bh, bw)
        self.box(y, x, bh, bw, title, attr)
        return y, x

    def draw_qte(self, y0, x0, H, W):
        g = self.g
        bw = min(W - 4, 44)
        y = y0 + H // 3
        x = x0 + (W - bw) // 2
        if g.qte:
            q = g.qte
            self.clear(y, x, 6, bw)
            self.box(y, x, 6, bw, "MASSE CRITIQUE", self.c("red") | curses.A_BOLD)
            n = bw - 4
            for i in range(n):
                off_center = abs((i + 0.5) / n - q["center"])
                ch, col = (("█", "green") if off_center <= q["green"] else ("▓", "yellow") if off_center <= q["yellow"]
                           else ("░", "red"))
                self.put(y + 3, x + 2 + i, ch, self.c(col))
            cx = x + 2 + min(n - 1, int(events.qte_pos(q) * n))
            self.put(y + 2, cx, "▼", self.c("white") | curses.A_BOLD)
            self.put(y + 1, x + 2, "Appuie sur F au bon moment !"[:n], self.c("white") | curses.A_BOLD)
        elif g.qte_result and g.time < g.qte_result[2]:
            text, col, _ = g.qte_result
            self.box(y, x, 3, bw, "", self.c(col) | curses.A_BOLD)
            self.put(y + 1, x + 1, " " * (bw - 2))
            self.put(y + 1, x + max(1, (bw - len(text)) // 2), text[:bw - 2], self.c(col) | curses.A_BOLD)

    def report_lines(self, r):
        """(label, value) rows of a finished game summary."""
        gain = f"+{fmt(r['gain'])} {'✦' if r['kind'] == 'supernova' else '⚛'}"
        lines = [("Durée", _dur(r["time"])), ("Niveau atteint", f"{fmt(r['level'])} / {fmt(r['cap'])}"),
                 ("Gain", gain), ("Radiation produite", f"{fmt(r['earned'])} ☢"),
                 ("Appuis sur ESPACE", f"{fmt(r['clicks'])} (critiques : {fmt(r['crits'])})"),
                 ("Meilleure détonation", f"{fmt(r['best_click'])} ☢"), ("Fusions parfaites", fmt(r["perfects"])),
                 ("Inspecteurs chassés", fmt(r["inspectors"])), ("Unités", fmt(r["units"]))]
        lines.append(("Mutations", ", ".join(r["mutations"]) or "aucune"))
        lines.append(("Synergies", ", ".join(r["synergies"]) or "aucune"))
        chests = r["chests"]
        lines.append(("Coffres", ", ".join(chests[-6:]) + (f" (+{len(chests) - 6})" if len(chests) > 6 else "")
                      if chests else "aucun"))
        return lines

    def draw_report(self):
        g, r = self.g, self.g.last_run
        if not g.report_ready or not r:
            return
        lines = self.report_lines(r)
        bw = min(self.w - 4, 72)
        lw = max(len(label) for label, _ in lines) + 2
        rows = []
        width = max(10, bw - 6 - lw)
        for label, value in lines:
            wrapped = textwrap.wrap(value, width) or [""]
            if len(wrapped) > 2:
                wrapped = [wrapped[0], wrapped[1][:width - 1] + "…"]
            rows.append((label, wrapped[0]))
            rows += [("", more) for more in wrapped[1:]]
        bh = min(self.h - 2, len(rows) + 4)
        if len(rows) > bh - 4 > 0:
            rows = rows[:bh - 5] + [("…", "")]
        y, x = max(0, (self.h - bh) // 2), max(0, (self.w - bw) // 2)
        self.clear(y, x, bh, bw)
        title = (f"FIN DE PARTIE — SUPERNOVA n°{g.supernova_count}" if r["kind"] == "supernova"
                 else f"FIN DE PARTIE — PRESTIGE n°{r['number']}")
        col = "magenta" if r["kind"] == "supernova" else "green"
        self.box(y, x, bh, bw, title, self.c(col) | curses.A_BOLD)
        for i, (label, value) in enumerate(rows[:bh - 4]):
            self.put(y + 1 + i, x + 2, label, self.c("cyan"))
            self.put(y + 1 + i, x + 2 + lw, value[:bw - 4 - lw], self.c("white") | curses.A_BOLD)
        msg = "Appuie sur une touche pour recommencer"
        self.put(y + bh - 2, x + max(1, (bw - len(msg)) // 2), msg[:bw - 2],
                 self.c("yellow") | curses.A_BOLD if self.blink(0.6) else self.c("yellow"))

    def draw_chest(self):
        opts = self.g.pending_chest
        if not opts:
            return
        bw = min(self.w - 4, 70)
        y, x = self.popup(4 + 2 * len(opts), bw, "COFFRE DE RAVITAILLEMENT", self.c("cyan") | curses.A_BOLD)
        self.put(y + 1, x + 2, f"Choisis ton bonus avec 1-{len(opts)} :", self.c("white") | curses.A_BOLD)
        for i, kind in enumerate(opts):
            name, desc = events.chest_text(self.g, kind)
            ok = events._chest_allowed(self.g, kind)
            if not ok:
                desc = "Plus utile pour l'instant."
            self.put(y + 2 + 2 * i, x + 2, f"{i + 1}. {name}"[:bw - 4],
                     self.c("yellow") | curses.A_BOLD if ok else self.c("grey"))
            self.put(y + 3 + 2 * i, x + 5, desc[:bw - 7], self.c("white") if ok else self.c("grey"))

    def draw_mutation(self):
        opts = self.g.pending_mutation
        if not opts or self.g.pending_chest:
            return
        bw = min(self.w - 4, 70)
        y, x = self.popup(4 + 2 * len(opts), bw, "MUTATION !", self.c("green") | curses.A_BOLD)
        self.put(y + 1, x + 2, f"Ton champignon mute. Choisis avec 1-{len(opts)} :", self.c("white") | curses.A_BOLD)
        for i, mid in enumerate(opts):
            _, name, desc = events.MUTATION_BY_ID[mid]
            label = f"{i + 1}. {name}"
            self.put(y + 2 + 2 * i, x + 2, label[:bw - 4], self.c("yellow") | curses.A_BOLD)
            syn = events.synergy_completed_by(self.g, mid)
            if syn:
                hint = f" ★ {syn[1]}" if syn[0] in self.g.synergies_found else " ★ synergie inconnue !"
                self.put(y + 2 + 2 * i, x + 2 + len(label), hint[:max(0, bw - 4 - len(label))],
                         self.c("magenta") | curses.A_BOLD)
            self.put(y + 3 + 2 * i, x + 5, desc[:bw - 7], self.c("white"))

    def draw_news(self, y, dt):
        label = " FLASH INFO "
        self.put(y, 0, label, self.c("alert"))
        ww = self.w - len(label) - 2
        self.news_off += dt * 12
        if not self.news or self.news_off > len(self.news) + ww:
            self.news = events.pick_news(self.g)
            self.news_off = 0.0
        text = " " * ww + self.news + " " * ww
        start = int(self.news_off)
        self.put(y, len(label) + 1, text[start:start + ww], self.c("white"))

    # --- Keys ----------------------------------------------------------------------------

    def handle(self, key):
        g = self.g
        too_small = getattr(self, "w", 0) and (self.w < MIN_W or self.h < MIN_H)
        report_shown = g.report_ready and g.last_run and not too_small
        if report_shown and key != curses.KEY_RESIZE:
            g.report_ready = False  # the key closes the end screen, with no other effect
            self.sfx("tick")
            return None
        if key == ord(" "):
            before, lvl, fizzles = g.radiation, g.level, g.sfx.count("fizzle")
            g.click()
            gained = g.radiation - before
            fizzled = g.sfx.count("fizzle") > fizzles
            if gained > 0:
                crit = g.last_crit == g.time
                self.add_float(("CRIT ! +" if crit else "+") + self.num(gained),
                               (self.c("red") if crit else self.c("yellow")) | curses.A_BOLD, self.rng.randint(-4, 4))
            elif fizzled:
                self.add_float("pschitt…", self.c("grey") | curses.A_BOLD, self.rng.randint(-4, 4))
            if g.level > lvl and lvl:
                self.add_float(f"NIVEAU {self.num(g.level)} !", self.c("green") | curses.A_BOLD)
        elif key in (ord("q"), ord("Q")):
            return "quit"
        elif key in (ord("f"), ord("F")):
            events.crit_press(g)
        elif key in (ord("x"), ord("X")):
            events.chase(g)
        elif g.pending_chest and ord("1") <= key <= ord("9"):
            if not events.choose_chest(g, key - ord("1")):
                self.sfx("nope")
        elif g.pending_mutation and ord("1") <= key < ord("1") + len(g.pending_mutation):
            events.choose_mutation(g, key - ord("1"))
        elif key == ord("0"):
            g.audio = AUDIO_MODES[(AUDIO_MODES.index(g.audio) + 1) % len(AUDIO_MODES)]
            muted = g.audio == "off"
            g.log("Son : " + AUDIO_LABELS[g.audio] + (" (0 pour le remettre)" if muted else ""),
                  "bad" if muted else "info")
        elif key in (ord("+"), ord("="), ord("-")):
            if g.change_volume(-1 if key == ord("-") else 1):
                self.sfx("tick")  # preview at the new volume
            # A single volume banner: otherwise 20 presses flush the useful messages from the history (8 messages).
            if g.messages and g.messages[-1][0].startswith("Volume : "):
                g.messages.pop()
            g.log(f"Volume : {g.volume} %", "info")
        elif ord("1") <= key < ord("1") + len(TABS):
            self.tab = key - ord("1")
            self.sfx("tick")
        elif key in (KEY_TAB, curses.KEY_RIGHT):
            self.tab = (self.tab + 1) % len(TABS)
            self.sfx("tick")
        elif key in (curses.KEY_BTAB, curses.KEY_LEFT):
            self.tab = (self.tab - 1) % len(TABS)
            self.sfx("tick")
        elif key in (curses.KEY_UP, ord("k")):
            self.sel[self.tab] = max(0, self.sel[self.tab] - 1)
            self.sfx("tick")
        elif key in (curses.KEY_DOWN, ord("j")):
            self.sel[self.tab] += 1
            self.sfx("tick")
        elif key == curses.KEY_PPAGE:
            self.sel[self.tab] = max(0, self.sel[self.tab] - 5)
        elif key == curses.KEY_NPAGE:
            self.sel[self.tab] += 5
        elif key in ENTER_KEYS:
            self.activate()
        elif key in (ord("m"), ord("M")):
            self.qty = QTY[(QTY.index(self.qty) + 1) % len(QTY)]
            self.sfx("tick")
        elif 0 < key < 256 and chr(key).lower() in events.DROP_LETTERS:
            if events.catch(g, chr(key).lower()):
                self.add_float("COFFRE !", self.c("cyan") | curses.A_BOLD)
        elif key == curses.KEY_RESIZE:
            self.scr.clear()

    def add_float(self, text, attr, dx=0):
        """Text rising above the mushroom (see update_particles)."""
        self.floats.append(dict(text=text, age=0.0, dx=dx, attr=attr))
        del self.floats[:-MAX_FLOATS]

    def selected_action(self):
        selectable = [r for r in self.rows() if r["sel"]]
        if not selectable:
            return None
        return selectable[max(0, min(self.sel[self.tab], len(selectable) - 1))]["action"]

    def activate(self):
        g = self.g
        action = self.selected_action()
        if not action:
            return
        kind = action[0]
        if kind == "unit":
            ok = g.buy_unit_max(action[1]) if self.qty == "max" else g.buy_unit_n(action[1], self.qty)
            self.sfx("buy" if ok else "nope")
        elif kind in ("talent", "ptalent", "stalent"):
            buy = dict(talent=g.buy_talent, ptalent=g.buy_ptalent, stalent=g.buy_stalent)[kind]
            n, limit = 0, math.inf if self.qty == "max" else self.qty
            while n < limit and buy(action[1]):
                n += 1
            self.sfx("talent" if n else "nope")
        elif kind == "abandon":
            g.abandon_challenge()
        elif kind == "intro":
            intro(self.scr, self.sound)
        elif kind == "story":
            for part in ("start", "end"):
                if (action[1], part) in STORIES:
                    intro(self.scr, self.sound, STORIES[(action[1], part)], then="continuer")
        elif kind == "skin":
            owned = [s for s in data.SKINS if s in g.skins]
            g.skin = owned[(owned.index(g.skin) + 1) % len(owned)] if g.skin in owned else owned[0]
        elif kind in ("prestige", "challenge", "supernova"):
            if self.confirm and self.confirm[0] == action and self.confirm[1] > self.t:
                self.confirm = None
                ok = g.supernova() if kind == "supernova" else g.prestige(
                    challenge=action[1] if kind == "challenge" else None)
                if ok:
                    self.particles.clear()
                    self.spawn_acc.clear()
            else:
                self.confirm = (action, self.t + 3)


def _clip(text, width):
    """Text cut to width characters, including the « … »."""
    return text if len(text) <= width else text[:width - 1] + "…"


def _dur(sec):
    if sec == math.inf:
        return "∞"
    sec = int(sec)
    if sec >= 3600:
        return f"{sec // 3600} h {sec % 3600 // 60:02d} min"
    if sec >= 60:
        return f"{sec // 60} min {sec % 60:02d} s"
    return f"{sec} s"
