"""Save files: game slots, format version, reading and atomic writing.

Only deals with JSON dicts: their content is the business of Game.to_dict / Game.load.
"""

import json
import os

SAVE_PATH = os.environ.get("CHAMPIGNON_SAVE", os.path.expanduser("~/.champignon_save.json"))
MAX_SLOTS = 99
SLOT_NUMBERS = range(1, MAX_SLOTS + 1)
# Format version. On a format change, increment SAVE_VERSION. If supporting older saves becomes
# too complicated, also raise MIN_SAVE_VERSION: older files are then deleted at startup
# (purge_obsolete) instead of being converted.
SAVE_VERSION = 1
MIN_SAVE_VERSION = 1


def slot_path(n):
    """File of game n (1 = main game, SAVE_PATH)."""
    if n == 1:
        return SAVE_PATH
    base, ext = os.path.splitext(SAVE_PATH)
    return f"{base}_{n}{ext}"


def version(d):
    """Version of a raw save (0 if missing or invalid)."""
    v = d.get("version")
    is_integer = isinstance(v, int) and not isinstance(v, bool)
    return v if is_integer else 0


def obsolete(d):
    """Save too old for this version of the game."""
    return version(d) < MIN_SAVE_VERSION


def supported(d):
    """Save readable by this version: neither too old nor written by a newer version
    (which must not be rewritten with a lower number, or a newer version would purge it)."""
    return MIN_SAVE_VERSION <= version(d) <= SAVE_VERSION


def read(path):
    """Raw content of a save (dict), None if missing or unreadable."""
    try:
        with open(path) as f:
            d = json.load(f)
    except (OSError, ValueError):
        return None
    return d if isinstance(d, dict) else None


def _tmp_path(path):
    return path + ".tmp"


def write(path, d):
    """Atomic write (.tmp then os.replace)."""
    tmp = _tmp_path(path)
    try:
        with open(tmp, "w") as f:
            json.dump(d, f)
        os.replace(tmp, path)
    except BaseException:
        delete([tmp])  # the real save is intact; don't leave a half-written file behind
        raise


def save_slots():
    """Readable games: list of (number, path, raw data), main game first."""
    slots = []
    for n in SLOT_NUMBERS:
        path = slot_path(n)
        d = read(path)
        if d is not None and supported(d):
            slots.append((n, path, d))
    return slots


def free_slot():
    """First number without a file (an unreadable file is never overwritten), None if all are taken."""
    return next((n for n in SLOT_NUMBERS if not os.path.exists(slot_path(n))), None)


def delete(paths):
    """Deletes the files; returns those that could not be deleted, with the reason."""
    failed = []
    for p in paths:
        try:
            os.remove(p)
        except FileNotFoundError:
            pass
        except OSError as e:
            failed.append(f"{p} ({e.strerror})")
    return failed


def purge_obsolete():
    """Deletes saves that are too old. Returns the numbers of the deleted games."""
    gone = []
    for n in SLOT_NUMBERS:
        path = slot_path(n)
        d = read(path)  # missing or damaged file (None): never deleted
        if d is not None and obsolete(d):
            delete((path, _tmp_path(path)))
            if not os.path.exists(path):
                gone.append(n)
    return gone


def all_files():
    """All existing game files, including .tmp and unreadable files."""
    candidates = (p for n in SLOT_NUMBERS for p in (slot_path(n), _tmp_path(slot_path(n))))
    return [p for p in candidates if os.path.exists(p)]
