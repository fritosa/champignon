"""Fichiers de sauvegarde : emplacements des parties, version du format, lecture et écriture atomique.

Ne connaît que des dicts JSON : leur contenu est l'affaire de Game.to_dict / Game.load.
"""

import json
import os

SAVE_PATH = os.environ.get("CHAMPIGNON_SAVE", os.path.expanduser("~/.champignon_save.json"))
MAX_SLOTS = 99
SLOT_NUMBERS = range(1, MAX_SLOTS + 1)
# Version du format. Changement de format : incrémenter SAVE_VERSION. Si supporter les anciennes
# sauvegardes devient trop compliqué, monter aussi MIN_SAVE_VERSION : les fichiers plus anciens sont
# alors supprimés au lancement (purge_obsolete) au lieu d'être convertis.
SAVE_VERSION = 1
MIN_SAVE_VERSION = 1


def slot_path(n):
    """Fichier de la partie n (1 = partie principale, SAVE_PATH)."""
    if n == 1:
        return SAVE_PATH
    base, ext = os.path.splitext(SAVE_PATH)
    return f"{base}_{n}{ext}"


def version(d):
    """Version d'une sauvegarde brute (0 si absente ou invalide)."""
    v = d.get("version")
    is_integer = isinstance(v, int) and not isinstance(v, bool)
    return v if is_integer else 0


def obsolete(d):
    """Sauvegarde trop ancienne pour cette version du jeu."""
    return version(d) < MIN_SAVE_VERSION


def supported(d):
    """Sauvegarde lisible par cette version : ni trop ancienne, ni écrite par une version plus récente
    (qu'on ne doit pas réécrire avec un numéro plus bas, sinon une version récente la purgerait)."""
    return MIN_SAVE_VERSION <= version(d) <= SAVE_VERSION


def read(path):
    """Contenu brut d'une sauvegarde (dict), None si absente ou illisible."""
    try:
        with open(path) as f:
            d = json.load(f)
    except (OSError, ValueError):
        return None
    return d if isinstance(d, dict) else None


def _tmp_path(path):
    return path + ".tmp"


def write(path, d):
    """Écriture atomique (.tmp puis os.replace)."""
    tmp = _tmp_path(path)
    try:
        with open(tmp, "w") as f:
            json.dump(d, f)
        os.replace(tmp, path)
    except BaseException:
        delete([tmp])  # la vraie sauvegarde est intacte, on ne laisse pas de fichier à moitié écrit
        raise


def save_slots():
    """Parties lisibles : liste de (numéro, chemin, données brutes), principale en premier."""
    slots = []
    for n in SLOT_NUMBERS:
        path = slot_path(n)
        d = read(path)
        if d is not None and supported(d):
            slots.append((n, path, d))
    return slots


def free_slot():
    """Premier numéro sans fichier (un fichier illisible n'est jamais écrasé), None si tout est pris."""
    return next((n for n in SLOT_NUMBERS if not os.path.exists(slot_path(n))), None)


def delete(paths):
    """Supprime les fichiers ; renvoie ceux qui n'ont pas pu l'être, avec la raison."""
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
    """Supprime les sauvegardes trop anciennes. Renvoie les numéros des parties supprimées."""
    gone = []
    for n in SLOT_NUMBERS:
        path = slot_path(n)
        d = read(path)  # fichier absent ou abîmé (None) : jamais supprimé
        if d is not None and obsolete(d):
            delete((path, _tmp_path(path)))
            if not os.path.exists(path):
                gone.append(n)
    return gone


def all_files():
    """Tous les fichiers de parties existants, y compris les .tmp et les fichiers illisibles."""
    candidates = (p for n in SLOT_NUMBERS for p in (slot_path(n), _tmp_path(slot_path(n))))
    return [p for p in candidates if os.path.exists(p)]
