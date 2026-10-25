import os
import tempfile
import unittest
from unittest import mock

import saves
from game import Game

from .helpers import exploded_game


class SlotTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.main = os.path.join(self.dir.name, "s.json")
        patcher = mock.patch.object(saves, "SAVE_PATH", self.main)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.addCleanup(self.dir.cleanup)

    def write(self, n, content='{"version": 1}'):
        with open(saves.slot_path(n), "w") as f:
            f.write(content)

    def test_paths(self):
        self.assertEqual(saves.slot_path(1), self.main)
        self.assertEqual(saves.slot_path(2), os.path.join(self.dir.name, "s_2.json"))

    def test_empty(self):
        self.assertEqual(saves.save_slots(), [])
        self.assertEqual(saves.free_slot(), 1)

    def test_listing_and_free_slot(self):
        self.write(1, '{"level": 4, "version": 1}')
        self.write(3)
        self.write(4, "corrompu")
        slots = saves.save_slots()
        self.assertEqual([n for n, _, _ in slots], [1, 3])
        self.assertEqual(slots[0][2], {"level": 4, "version": 1})
        self.assertEqual(saves.free_slot(), 2)

    def test_main_reused_when_deleted(self):
        self.write(2)
        self.assertEqual(saves.free_slot(), 1)

    def test_unreadable_file_never_reused(self):
        self.write(1, "corrompu")
        self.write(2, "[]")
        self.assertEqual(saves.save_slots(), [])
        self.assertEqual(saves.free_slot(), 3)

    def test_full(self):
        for n in range(1, saves.MAX_SLOTS + 1):
            self.write(n)
        self.assertEqual(len(saves.save_slots()), saves.MAX_SLOTS)
        self.assertIsNone(saves.free_slot())

    def test_current_version_written(self):
        exploded_game().save(saves.slot_path(1))
        self.assertEqual(saves.save_slots()[0][2]["version"], saves.SAVE_VERSION)
        self.assertEqual(saves.purge_obsolete(), [])

    def test_obsolete_purged(self):
        self.write(1, '{"version": 1}')
        self.write(2, '{"version": 1, "level": 3}')
        self.write(3, '{"level": 3}')  # no version: predates versioning
        self.write(4, '{"version": "2"}')
        self.write(5, "corrompu")
        with open(saves.slot_path(2) + ".tmp", "w") as f:
            f.write("{}")
        with mock.patch.object(saves, "MIN_SAVE_VERSION", 2), mock.patch.object(saves, "SAVE_VERSION", 2):
            self.assertIsNone(Game.load(saves.slot_path(2)))
            self.assertEqual(saves.save_slots(), [])
            self.assertEqual(saves.purge_obsolete(), [1, 2, 3, 4])
        for n in (1, 2, 3, 4):
            self.assertFalse(os.path.exists(saves.slot_path(n)), n)
        self.assertFalse(os.path.exists(saves.slot_path(2) + ".tmp"))
        self.assertTrue(os.path.exists(saves.slot_path(5)))  # unreadable: never deleted
        self.assertEqual(saves.free_slot(), 1)

    def test_newer_version_kept(self):
        self.write(1, '{"version": 99}')
        self.assertEqual(saves.purge_obsolete(), [])
        self.assertTrue(os.path.exists(saves.slot_path(1)))
        # Not offered, loaded or reused: it is never rewritten with a lower version.
        self.assertEqual(saves.save_slots(), [])
        self.assertIsNone(Game.load(saves.slot_path(1)))
        self.assertEqual(saves.free_slot(), 2)

    def test_atomic_write(self):
        saves.write(self.main, {"level": 2})
        self.assertEqual(saves.read(self.main), {"level": 2})
        self.assertFalse(os.path.exists(self.main + ".tmp"))

    def test_failed_write_leaves_no_tmp(self):
        saves.write(self.main, {"level": 2})
        with self.assertRaises(TypeError):
            saves.write(self.main, {"level": object()})
        self.assertFalse(os.path.exists(self.main + ".tmp"))
        self.assertEqual(saves.read(self.main), {"level": 2})

    def test_all_files_and_delete(self):
        self.write(1)
        self.write(3, "corrompu")
        with open(saves.slot_path(2) + ".tmp", "w") as f:
            f.write("{}")
        files = saves.all_files()
        self.assertEqual(files, [saves.slot_path(1), saves.slot_path(2) + ".tmp", saves.slot_path(3)])
        self.assertEqual(saves.delete(files + [saves.slot_path(9)]), [])  # missing file: ignored
        self.assertEqual(saves.all_files(), [])


if __name__ == "__main__":
    unittest.main()
