"""Unit tests: python3 -m unittest (from the project root)."""

import atexit
import os
import shutil
import tempfile

# Never touch the real save: saves.SAVE_PATH is read at import time.
_dir = tempfile.mkdtemp(prefix="champignon-tests-")
atexit.register(shutil.rmtree, _dir, True)
os.environ["CHAMPIGNON_SAVE"] = os.path.join(_dir, "save.json")
