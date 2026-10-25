#!/usr/bin/env python3
"""Champignon atomique — an incremental game in the terminal.

Launch: champignon (link ~/.local/bin/champignon → main.py), ./main.py or python3 main.py
(--reset to erase all games, --mute for no sound)

At startup, if games exist, a menu offers to continue one of them or to create a new one.
"""

import curses
import locale
import os
import signal
import sys
import time

# Escape reacts in 25 ms instead of one second; must be set before curses is initialized.
os.environ.setdefault("ESCDELAY", "25")

import saves  # noqa: E402
from game import Game  # noqa: E402
from sound import Sound  # noqa: E402
from screens import intro, save_menu  # noqa: E402
from ui import UI  # noqa: E402

FRAME_DURATION = 0.05
AUTOSAVE_INTERVAL = 30
MAX_TICK = 1.0  # after a sleep, the game doesn't catch up on all the elapsed time at once
YES = ("o", "oui", "y", "yes")
# xterm sequences: push the terminal title, then clear it and restore the old one.
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
    # With no readable game, start on the first free slot (main checked that one exists).
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
        ui.title = ""  # "" enables the title (None disables it)
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
            ui.show_story()  # blocking full screen: outside of draw
            if now - last_save > AUTOSAVE_INTERVAL:
                g.save(path)
                last_save = now
            time.sleep(max(0.0, FRAME_DURATION - (time.monotonic() - now)))
    finally:
        g.save(path)  # first: the terminal may already be closed (SIGHUP)
        run.path = path  # for the exit message, even after Ctrl+C
        if titled:
            _restore_title()


def _interrupt(signum, frame):
    raise KeyboardInterrupt


def reset():
    """Erases all games after confirmation. Returns False if the player backs out."""
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
    # Terminal closed or kill: same path as Ctrl+C, so the game is saved on exit.
    for sig in (signal.SIGHUP, signal.SIGTERM):
        signal.signal(sig, _interrupt)
    if "--reset" in sys.argv and not reset():
        return
    purged = saves.purge_obsolete()
    if not saves.save_slots() and saves.free_slot() is None:
        # Never overwrite an unreadable file or one from a newer version.
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
