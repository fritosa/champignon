"""Tests for the out-of-game screens (intro, challenge stories, save menu) on a fake screen."""

import curses
import os
import tempfile
import time
import unittest
from unittest import mock

import data
import saves
import screens
from game import Game

from .helpers import FakeScreen, FakeSound


class AgoTest(unittest.TestCase):
    def test_units(self):
        now = time.time()
        self.assertEqual(screens._ago(now - 30), "à l'instant")
        self.assertEqual(screens._ago(now - 125), "il y a 2 min")
        self.assertEqual(screens._ago(now - 7300), "il y a 2 h")
        self.assertEqual(screens._ago(now - 3 * 86400 - 5), "il y a 3 j")


class SaveMenuTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        patcher = mock.patch.object(saves, "SAVE_PATH", os.path.join(self.dir.name, "s.json"))
        patcher.start()
        self.addCleanup(patcher.stop)
        for n in (1, 2):
            Game().save(saves.slot_path(n))
        self.slots = saves.save_slots()

    def menu(self, keys, **kw):
        scr = FakeScreen(keys, **kw)
        return screens.save_menu(scr, self.slots, FakeSound()), scr

    def test_enter_continues_main(self):
        path, scr = self.menu([10])
        self.assertEqual(path, saves.slot_path(1))
        self.assertEqual(scr.delay, [False, True])  # blocking mode restored

    def test_navigate_to_second(self):
        path, _ = self.menu([curses.KEY_DOWN, ord("z"), 10])
        self.assertEqual(path, saves.slot_path(2))

    def test_new_game_uses_free_slot(self):
        path, scr = self.menu([curses.KEY_UP, 10])  # wraps up to "Nouvelle partie"
        self.assertEqual(path, saves.slot_path(3))
        self.assertTrue(any("Nouvelle partie" in t for t in scr.text))
        self.assertTrue(any("Continuer : Partie 2" in t for t in scr.text))

    def test_quit(self):
        path, scr = self.menu([ord("q")])
        self.assertIsNone(path)
        self.assertEqual(scr.delay[-1], True)

    def test_escape_does_not_quit(self):
        path, _ = self.menu([27, 10])
        self.assertEqual(path, saves.slot_path(1))

    def test_tiny_terminals(self):
        for size in [(5, 20), (24, 3), (8, 1), (1, 80), (2, 2)]:
            path, _ = self.menu([10], size=size)
            self.assertEqual(path, saves.slot_path(1), size)

    def test_scrolls_to_selection(self):
        for n in range(3, 16):
            Game().save(saves.slot_path(n))
        self.slots = saves.save_slots()
        path, scr = self.menu([curses.KEY_UP] + [10], size=(24, 80))
        self.assertEqual(path, saves.slot_path(16))
        self.assertTrue(any("Nouvelle partie" in t for t in scr.text))
        self.assertFalse(any("Partie principale" in t for t in scr.text))  # scrolled off screen

    def test_bad_field_types(self):
        self.slots = [(1, saves.slot_path(1), {"level": "5", "isotopes": None, "prestige_count": [],
                                              "wall_time": None})]
        path, scr = self.menu([10])
        self.assertEqual(path, saves.slot_path(1))
        self.assertTrue(any("niveau 0" in t for t in scr.text))

    def test_all_slots_taken(self):
        with mock.patch.object(screens, "free_slot", return_value=None):
            sound = FakeSound()
            scr = FakeScreen([curses.KEY_UP, 10, ord("q")])
            self.assertIsNone(screens.save_menu(scr, self.slots, sound))
        self.assertIn("nope", sound.played)
        self.assertTrue(any("Plus d'emplacement libre" in t for t in scr.text))


class IntroTest(unittest.TestCase):
    def test_first_key_reveals_second_closes(self):
        scr = FakeScreen([-1, ord("a"), ord("b")])
        scr.timeout = lambda ms: None
        screens.intro(scr, FakeSound())
        self.assertEqual(scr.keys, [])
        self.assertTrue(any("Morille" in t for t in scr.text))
        self.assertTrue(any("commencer" in t for t in scr.text))

    def test_small_screen(self):
        scr = FakeScreen([ord("a"), ord("b")], size=(8, 20))
        scr.timeout = lambda ms: None
        screens.intro(scr)

    def test_texts_fit_80x24(self):
        # The default terminal: the last paragraph (intro or story) must stay visible.
        for pages in [screens.INTRO] + list(screens.STORIES.values()):
            scr = FakeScreen([ord("a"), ord("b")], size=(24, 80))
            scr.timeout = lambda ms: None
            screens.intro(scr, pages=pages)
            last = screens._nbsp(pages[-1]).split()[-1]
            self.assertTrue(any(last in t for t in scr.text), pages[0])

    def test_story_ignores_keys_at_first(self):
        # The player is still hammering SPACE when the story opens: those presses don't close it.
        clock = iter(i * 0.3 for i in range(1000))
        scr = FakeScreen([ord(" ")] * 4)
        scr.timeout = lambda ms: None
        with mock.patch.object(screens.time, "monotonic", lambda: next(clock)), self.assertRaises(IndexError):
            screens.intro(scr, None, screens.STORIES[("nanar", "end")], guard=60)  # still open when the keys run out
        self.assertEqual(scr.keys, [])  # all 4 presses were read (and ignored), the IndexError comes from getch
        self.assertFalse(any("continuer" in t or "commencer" in t for t in scr.text))  # text not fully shown yet

    def test_stories_match_challenges(self):
        for cid, part in screens.STORIES:
            self.assertIn(cid, data.CHALLENGE_BY_ID)
            self.assertIn(part, ("start", "end"))
        self.assertIn(("nanar", "end"), screens.STORIES)


if __name__ == "__main__":
    unittest.main()
