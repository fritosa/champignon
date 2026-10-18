"""Tests unitaires : python3 -m unittest (depuis la racine du projet)."""

import atexit
import os
import shutil
import tempfile

# Ne jamais toucher à la vraie sauvegarde : saves.SAVE_PATH est lu à l'import.
_dir = tempfile.mkdtemp(prefix="champignon-tests-")
atexit.register(shutil.rmtree, _dir, True)
os.environ["CHAMPIGNON_SAVE"] = os.path.join(_dir, "save.json")
