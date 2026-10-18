#!/usr/bin/env python3
"""Champignon atomique — jeu incrémental en terminal.

Lancement : champignon (lien ~/.local/bin/champignon → main.py), ./main.py ou python3 main.py
(--reset pour effacer toutes les parties, --mute sans son)

Au démarrage, si des parties existent, un menu propose de continuer l'une d'elles ou d'en créer une nouvelle.
"""

import curses
import locale
import os
import signal
import sys
import time

# Échap réagit en 25 ms au lieu d'une seconde ; à définir avant l'initialisation de curses.
os.environ.setdefault("ESCDELAY", "25")

import saves  # noqa: E402
from game import Game  # noqa: E402
from sound import Sound  # noqa: E402
from ui import UI, intro, save_menu  # noqa: E402

FRAME_DURATION = 0.05
AUTOSAVE_INTERVAL = 30
MAX_TICK = 1.0  # après une mise en veille, le jeu ne rattrape pas tout le temps écoulé d'un coup
YES = ("o", "oui", "y", "yes")
# Séquences xterm : empiler le titre du terminal, puis le vider et restaurer l'ancien.
PUSH_TITLE = "\x1b[22;0t"
RESTORE_TITLE = "\x1b]0;\x07\x1b[23;0t"


def _log_purged(g, purged):
    names = ", ".join("partie principale" if n == 1 else f"partie {n}" for n in purged)
    g.log(("Sauvegardes trop anciennes supprimées : " if len(purged) > 1 else
           "Sauvegarde trop ancienne supprimée : ") + names, "bad")


def _can_set_title():
    return os.environ.get("TERM", "dumb") not in ("dumb", "linux") and sys.__stdout__.isatty()


def _restore_title():
    try:
        sys.__stdout__.write(RESTORE_TITLE)
        sys.__stdout__.flush()
    except OSError:
        pass


def run(scr, purged):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    scr.nodelay(True)
    scr.keypad(True)
    sound = Sound(enabled="--mute" not in sys.argv)
    slots = saves.save_slots()
    # Sans partie lisible, on démarre sur le premier emplacement libre (main a vérifié qu'il en existe un).
    path = save_menu(scr, slots, sound) if slots else saves.slot_path(saves.free_slot())
    if path is None:
        return None
    g = Game.load(path)
    if g is None:
        g = Game()
        intro(scr, sound)
    if purged:
        _log_purged(g, purged)
    ui = UI(scr, g, sound)
    titled = _can_set_title()
    if titled:
        ui.title = ""  # "" active le titre (None le désactive)
        sys.__stdout__.write(PUSH_TITLE)
    last = last_save = time.monotonic()
    try:
        while True:
            now = time.monotonic()
            dt, last = min(now - last, MAX_TICK), now
            while (key := scr.getch()) != -1:
                if ui.handle(key) == "quit":
                    return path
            g.tick(dt)
            ui.draw(dt)
            ui.show_story()  # écran plein bloquant : hors de draw
            if now - last_save > AUTOSAVE_INTERVAL:
                g.save(path)
                last_save = now
            time.sleep(max(0.0, FRAME_DURATION - (time.monotonic() - now)))
    finally:
        g.save(path)  # d'abord : le terminal est peut-être déjà fermé (SIGHUP)
        run.path = path  # pour le message de fin, même après Ctrl+C
        if titled:
            _restore_title()


def _interrupt(signum, frame):
    raise KeyboardInterrupt


def reset():
    """Efface toutes les parties après confirmation. Renvoie False si le joueur renonce."""
    files = saves.all_files()
    if not files:
        print("Aucune partie à effacer.")
        return True
    print("Parties à effacer :\n  " + "\n  ".join(files))
    try:
        if input("Effacer définitivement ? [o/N] ").strip().lower() not in YES:
            print("Rien n'a été effacé.")
            return False
    except (EOFError, KeyboardInterrupt):
        print("\nRien n'a été effacé.")
        return False
    failed = saves.delete(files)
    if failed:
        print("Impossible d'effacer :\n  " + "\n  ".join(failed))
        return False
    print("Parties effacées.")
    return True


def main():
    locale.setlocale(locale.LC_ALL, "")
    # Terminal fermé ou kill : même chemin que Ctrl+C, donc la partie est sauvegardée en sortant.
    for sig in (signal.SIGHUP, signal.SIGTERM):
        signal.signal(sig, _interrupt)
    if "--reset" in sys.argv and not reset():
        return
    purged = saves.purge_obsolete()
    if not saves.save_slots() and saves.free_slot() is None:
        # Jamais écraser un fichier illisible ou d'une version plus récente.
        print(f"Aucune partie utilisable et aucun emplacement libre ({saves.MAX_SLOTS} fichiers) : "
              "supprime un fichier de sauvegarde ou lance champignon --reset.")
        return
    run.path = None
    try:
        curses.wrapper(run, purged)
    except KeyboardInterrupt:
        pass
    if run.path:
        print(f"Partie sauvegardée dans {run.path}")


if __name__ == "__main__":
    main()
