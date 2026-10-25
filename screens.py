"""Full-screen screens outside a game: introduction and challenge stories (typewriter text), game
selection menu at launch."""

import curses
import re
import textwrap
import time

from game import fmt
from saves import free_slot, slot_path

KEY_TAB = 9
ENTER_KEYS = (10, 13, curses.KEY_ENTER)
TITLE = "☢  CHAMPIGNON ATOMIQUE  ☢"


# --- Introduction -----------------------------------------------------------------

INTRO = [
    "ANNÉE 1962 ¾. QUELQUE PART ENTRE DEUX FRONTIÈRES. (STOCK-SHOT D'UN DÉSERT, PUIS D'UNE FORÊT, PUIS DU "
    "MÊME DÉSERT.)",
    "Le professeur Amédée Morille, mycologue, artificier amateur et ceinture noire de pétanque, fait LA "
    "découverte : les champignons atomiques sont VIVANTS ! Ils mangent de la radiation. « C'est "
    "impossible ! », s'écrie son assistante, face à la mauvaise caméra.",
    "Une nuit d'orage (tournée en plein jour, filtre bleu), le professeur disparaît en zone interdite. On ne "
    "retrouve que sa casquette et une bombe au fond de TON jardin, avec un post-it : « Arrose-la. »",
    "Te voici jardinier en chef. Le ministère envoie des soldats « pour aider », l'ONU des inspecteurs « pour "
    "vérifier ». Saboteurs, drones en fil de pêche et manifestants débarquent sans invitation. Le voisin porte "
    "plainte ; sa perruque change de couleur entre deux plans.",
    "Ta mission : le faire pousser jusqu'aux étoiles. Le professeur aurait été fier. Probablement : son "
    "doublage était inaudible.",
    "CET ÉTÉ... APPUIE SUR ESPACE.",
]
INTRO_SPEED = 80  # characters/s
STORY_GUARD = 1.0  # seconds of keys ignored by a challenge story (players are still hammering SPACE)

# Challenge stories, shown like the introduction at launch ("start") and on success ("end").
STORIES = {
    ("nanar", "start"): [
        "NANARLAND — UN FILM DE GÉRARD ZÉDÉ",
        "Gérard Zédé, producteur des « Tomates tueuses du Larzac » et de « Mon voisin le yéti 3 : le retour "
        "du yéti 2 », a vu ton champignon dans le journal. Il a eu une illumination. Et peut-être aussi une "
        "légère insolation.",
        "Son prochain chef-d'œuvre s'appellera « LE CHAMPIGNON QUI VOULAIT CONQUÉRIR LE MONDE (ET UN PEU LA "
        "BELGIQUE) ». Budget total : 312 francs, un sandwich au pâté et une caméra empruntée au beau-frère.",
        "Les explosions seront bruitées à la bouche par son neveu, qui est enrhumé : une fois sur trois, ça fait "
        "pschitt. La moitié de ton armée est réquisitionnée comme figurants et ne produit plus que des "
        "autographes.",
        "Ton rôle : faire monter le champignon jusqu'au niveau 100 avant la fin du tournage. Silence... "
        "moteur... ça tourne ! (La perche est dans le champ. Personne ne la retirera.)",
    ],
    ("nanar", "end"): [
        "NANARLAND — GÉNÉRIQUE DE FIN",
        "Le film sort un mardi pluvieux dans la salle des fêtes de Saint-Glinglin-sur-Marne. Quatorze "
        "spectateurs. À la scène la plus dramatique, les quatorze pleurent de rire. Le lendemain, ils "
        "reviennent avec leurs amis.",
        "Les critiques sont unanimes : « raté de A à Z », « le perchman mérite un César », « on y retourne "
        "demain ». Six mois plus tard, la séance de minuit affiche complet tous les vendredis, et la salle "
        "entière crie « PSCHITT ! » à chaque explosion. Ce n'est pas un navet : c'est un nanar, et on l'aime.",
        "Ton champignon est devenu CULTE : ses détonations critiques frappent désormais ×15. Gérard t'offre "
        "les décors en carton-pâte, à porter comme skin.",
        "Tout en bas du générique, une ligne intrigue les fans : « Lézard géant : A. Morille ». Le professeur "
        "ne répond toujours pas aux lettres.",
    ],
}


def _nbsp(text):
    """French-style non-breaking spaces: no « or » : ; ! ? left orphaned at a line edge."""
    return re.sub(r" ([»:;!?])", "\u00a0\\1", text.replace("« ", "«\u00a0"))


def intro(scr, sound=None, pages=INTRO, guard=0.0, then="commencer"):
    """Typewriter text (introduction, or a challenge story). One key shows everything, the next one
    closes. The first paragraph is bold. Keys during the first guard seconds are ignored.
    then: end of the prompt « Appuie sur une touche pour … »."""
    shown, start = 0.0, time.monotonic()
    opened = start
    scr.timeout(30)
    try:
        while True:
            h, w = scr.getmaxyx()
            bw = max(10, min(w - 4, 70))
            lines = []
            for i, par in enumerate(pages):
                lines += [(line, i == 0) for line in textwrap.wrap(_nbsp(par), bw)] + [("", False)]
            lines = lines[:max(1, h - 3)]  # overflow is neither typed nor waited for
            total = sum(len(line) for line, _ in lines)
            before, shown = int(shown), min(total, (time.monotonic() - start) * INTRO_SPEED)
            if sound and int(shown) // 6 != before // 6 and shown < total:
                sound.play("tick")
            scr.erase()
            y = max(0, (h - len(lines) - 4) // 2)
            x = max(0, (w - bw) // 2)
            _menu_put(scr, y, max(0, (w - len(TITLE)) // 2), TITLE, curses.A_BOLD)
            left = int(shown)
            for i, (line, bold) in enumerate(lines):
                if y + 2 + i >= h - 1:
                    break
                _menu_put(scr, y + 2 + i, x, line[:left], curses.A_BOLD if bold else 0)
                left = max(0, left - len(line))
            done = shown >= total
            msg = ("Appuie sur une touche pour " + then if done
                   else "Une touche pour tout afficher")
            _menu_put(scr, h - 1, max(0, (w - len(msg)) // 2), msg, curses.A_BOLD if done else curses.A_DIM)
            scr.refresh()
            key = scr.getch()
            if key in (-1, curses.KEY_RESIZE) or time.monotonic() - opened < guard:
                continue
            if done:
                return
            start = time.monotonic() - total / INTRO_SPEED
    finally:
        scr.timeout(-1)
        scr.nodelay(True)


# --- Start menu ------------------------------------------------------------------

def _number_or_zero(v):
    return v if isinstance(v, (int, float)) and not isinstance(v, bool) else 0


def _ago(wall):
    s = max(0, time.time() - wall)
    for unit, n in (("j", 86400), ("h", 3600), ("min", 60)):
        if s >= n:
            return f"il y a {int(s // n)} {unit}"
    return "à l'instant"


def _menu_put(scr, y, x, text, attr=0):
    """Lenient addnstr: clips at the edge and ignores errors (bottom-right corner, tiny window)."""
    h, w = scr.getmaxyx()
    if 0 <= y < h and 0 <= x < w - 1 and text:
        try:
            scr.addnstr(y, x, text, w - 1 - x, attr)
        except curses.error:
            pass


def save_menu(scr, slots, sound=None):
    """Game selection at launch. Returns the save path, or None to quit."""
    entries = []
    for n, path, d in slots:
        name = "Partie principale" if n == 1 else f"Partie {n}"
        level, isotopes, prestiges, wall = (_number_or_zero(d.get(k))
                                            for k in ("level", "isotopes", "prestige_count", "wall_time"))
        detail = (f"niveau {fmt(level)} · ⚛ {fmt(isotopes)} · prestige ×{int(prestiges)}"
                  + (f" · {_ago(wall)}" if wall else ""))
        entries.append(("Continuer : " + name, detail, path))
    free = free_slot()
    entries.append(("+ Nouvelle partie", "Recommence de zéro dans une sauvegarde séparée." if free else
                    "Plus d'emplacement libre : supprime un fichier de sauvegarde.",
                    slot_path(free) if free else None))
    sel = top = 0
    scr.nodelay(False)
    bold = curses.A_BOLD
    try:
        while True:
            h, w = scr.getmaxyx()
            scr.erase()
            # Header (4 lines), scrolling list (2 lines per game), help on the last line.
            visible = max(1, (h - 6) // 2)
            y = max(0, min(h // 2 - len(entries) - 3, h - 6 - 2 * min(len(entries), visible)))
            top = max(min(top, sel), sel - visible + 1)
            _menu_put(scr, y, max(0, (w - len(TITLE)) // 2), TITLE, bold)
            q = "Quelle partie lancer ?"
            _menu_put(scr, y + 2, max(0, (w - len(q)) // 2), q)
            x = max(0, w // 2 - 30)
            for row, i in enumerate(range(top, min(len(entries), top + visible))):
                label, detail, _ = entries[i]
                ly = y + 4 + 2 * row
                _menu_put(scr, ly, x, f" {'▶' if i == sel else ' '} {label} ",
                          curses.A_REVERSE | bold if i == sel else bold)
                _menu_put(scr, ly + 1, x + 4, detail, curses.A_DIM)
            if top:
                _menu_put(scr, y + 3, x, "↑")
            if top + visible < len(entries):
                _menu_put(scr, h - 2, x, "↓")
            help_ = "↑↓ choisir · Entrée valider · Q quitter"
            _menu_put(scr, h - 1, max(0, (w - len(help_)) // 2), help_, curses.A_DIM)
            scr.refresh()
            key = scr.getch()
            if key in (curses.KEY_UP, ord("k")):
                sel = (sel - 1) % len(entries)
            elif key in (curses.KEY_DOWN, ord("j"), KEY_TAB):
                sel = (sel + 1) % len(entries)
            elif key in ENTER_KEYS:
                if entries[sel][2]:
                    return entries[sel][2]
                if sound:
                    sound.play("nope")
                continue
            elif key in (ord("q"), ord("Q")):
                return None
            else:
                continue
            if sound:
                sound.play("tick")
    finally:
        scr.nodelay(True)
