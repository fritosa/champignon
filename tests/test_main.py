import io
import os
import unittest
from contextlib import redirect_stdout
from unittest import mock

import main
from saves import slot_path


class ResetTest(unittest.TestCase):
    def setUp(self):
        self.files = [slot_path(1), slot_path(1) + ".tmp", slot_path(2), slot_path(7)]
        for p in self.files:
            with open(p, "w") as f:
                f.write("{}")
        self.addCleanup(lambda: [os.remove(p) for p in self.files if os.path.exists(p)])

    def reset(self, answer):
        side = answer if isinstance(answer, type) else None
        with mock.patch("builtins.input", side_effect=side, return_value=answer), redirect_stdout(io.StringIO()):
            return main.reset()

    def test_confirmed_deletes_every_slot(self):
        self.assertTrue(self.reset("o"))
        self.assertFalse(any(os.path.exists(p) for p in self.files))

    def test_refused_keeps_everything(self):
        for answer in ("", "n", EOFError, KeyboardInterrupt):
            self.assertFalse(self.reset(answer), answer)
            self.assertTrue(all(os.path.exists(p) for p in self.files), answer)

    def test_other_files_untouched(self):
        other = os.path.join(os.path.dirname(slot_path(1)), "autre.json")
        with open(other, "w") as f:
            f.write("{}")
        self.addCleanup(os.remove, other)
        self.reset("o")
        self.assertTrue(os.path.exists(other))

    def test_nothing_to_delete(self):
        self.reset("o")
        self.assertTrue(self.reset("o"))

    def test_failed_delete_does_not_start(self):
        with mock.patch("os.remove", side_effect=PermissionError(13, "Permission denied")):
            self.assertFalse(self.reset("o"))


class NoUsableSlotTest(unittest.TestCase):
    def test_refuses_to_start_instead_of_overwriting(self):
        with mock.patch("saves.purge_obsolete", return_value=[]), mock.patch("saves.save_slots", return_value=[]), \
                mock.patch("saves.free_slot", return_value=None), mock.patch("curses.wrapper") as wrapper, \
                mock.patch("sys.argv", ["champignon"]), mock.patch("signal.signal"), redirect_stdout(io.StringIO()) as out:
            main.main()
        wrapper.assert_not_called()
        self.assertIn("aucun emplacement libre", out.getvalue())


if __name__ == "__main__":
    unittest.main()
