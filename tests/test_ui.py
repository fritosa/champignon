"""Tests de l'UI sans terminal : on construit UI sans __init__ (qui initialise curses)."""

import curses
import os
import random
import tempfile
import time
import unittest
from unittest import mock

import data
import events
import game
import saves
import ui
from game import Game

from .helpers import FixedRandom, exploded_game


class FakeSound:
    def __init__(self, ok=True):
        self.ok = ok
        self.muted = False
        self.music_on = True
        self.music = None
        self.played = []

    def set_music(self, name):
        self.music = name

    def play(self, name, vol=1.0):
        self.played.append(name)


def bare_ui(g=None, sound=None):
    u = ui.UI.__new__(ui.UI)
    u.g = g or exploded_game()
    u.sound = sound
    u.sfx_last = {}
    u.colors = {}
    u.t = 0.0
    u.tab, u.tab_w, u.qty, u.confirm = 0, 40, 1, None
    u.shake_until, u.title, u.title_at, u.rng = 0.0, None, -9.0, random.Random(0)
    u.sel, u.scroll = [0] * len(ui.TABS), [0] * len(ui.TABS)
    return u


class PlaySfxTest(unittest.TestCase):
    def test_dedupe_per_frame(self):
        u = bare_ui(sound=FakeSound())
        u.g.sfx.extend(["click0", "click3", "level", "level", "tick"])
        u.play_sfx()
        self.assertEqual(u.sound.played, ["click0", "level", "tick"])
        self.assertFalse(u.g.sfx)

    def test_throttle(self):
        u = bare_ui(sound=FakeSound())
        u.g.sfx.append("level")
        u.play_sfx()
        u.t = 0.1
        u.g.sfx.append("level")
        u.play_sfx()
        self.assertEqual(u.sound.played, ["level"])
        u.t = ui.SFX_GAP["level"]
        u.g.sfx.append("level")
        u.play_sfx()
        self.assertEqual(u.sound.played, ["level", "level"])

    def test_without_sound(self):
        u = bare_ui()
        u.g.sfx.append("boom")
        u.play_sfx()
        self.assertFalse(u.g.sfx)

    def test_audio_modes_synced(self):
        u = bare_ui(sound=FakeSound())
        for mode, muted, music in (("all", False, True), ("sfx", False, False), ("off", True, False)):
            u.g.audio = mode
            u.play_sfx()
            self.assertEqual((u.sound.muted, u.sound.music_on), (muted, music), mode)

    def test_audio_key_cycles(self):
        u = bare_ui()
        seen = []
        for _ in range(4):
            u.handle(ord("0"))
            seen.append(u.g.audio)
        self.assertEqual(seen, ["sfx", "off", "all", "sfx"])

    def test_music_track(self):
        u = bare_ui(sound=FakeSound())
        u.play_sfx()
        self.assertEqual(u.sound.music, "calm")
        u.g.level = u.g.cap
        u.play_sfx()
        self.assertEqual(u.sound.music, "epic")
        u.g.inspector = dict(start=0, dur=20, hits=0)
        u.play_sfx()
        self.assertEqual(u.sound.music, "tension")


class BannerTest(unittest.TestCase):
    def test_order_and_expiry(self):
        g = exploded_game()
        g.time = 100
        g.messages.extend([("vieux", 90, "info"), ("a", 97, "good"), ("b", 98, "bad"),
                           ("c", 99, "loot"), ("d", 99.5, "star")])
        texts = [t.strip() for t, _ in bare_ui(g).banner_lines(60)]
        self.assertEqual(texts, ["d", "c", "b"])

    def test_alerts_first(self):
        g = exploded_game()
        g.inspector = dict(start=g.time, dur=20, hits=0)
        g.freeze_until = g.time + 10
        g.log("coucou")
        lines = [t for t, _ in bare_ui(g).banner_lines(60)]
        self.assertIn("inspecteur", lines[0])
        self.assertIn("GELÉE", lines[1])
        self.assertEqual(lines[2], "coucou")

    def test_long_message_wrapped_in_block(self):
        g = exploded_game()
        g.log("un message vraiment beaucoup trop long pour tenir sur une seule ligne du panneau")
        lines = [t for t, _ in bare_ui(g).banner_lines(40)]
        self.assertEqual(len(lines), 2)
        self.assertEqual(len(lines[0]), len(lines[1]))
        self.assertTrue(all(len(t) <= 36 for t in lines))

    def test_fade_drops_background(self):
        g = exploded_game()
        g.log("fin", "bad")
        u = bare_ui(g)
        u.colors = {"msg_bad": 1 << 20, "red": 1 << 21}
        g.time += ui.MSG_TIME - 0.5
        self.assertEqual(u.banner_lines(60)[0][1], (1 << 21) | curses.A_BOLD)


class QuantityTest(unittest.TestCase):
    def setUp(self):
        self.u = bare_ui()
        self.g = self.u.g
        self.g.talents["u_soldier"] = 1

    def test_m_cycles(self):
        seen = []
        for _ in range(5):
            self.u.handle(ord("m"))
            seen.append(self.u.qty)
        self.assertEqual(seen, [10, 100, "max", 1, 10])

    def test_buy_ten(self):
        self.u.qty = 10
        self.g.radiation = self.g.unit_cost_n("soldier", 10)
        self.u.activate()
        self.assertEqual(self.g.units["soldier"], 10)
        self.assertEqual(self.g.sfx[-1], "buy")

    def test_buy_ten_refused(self):
        self.u.qty = 10
        self.g.radiation = self.g.unit_cost_n("soldier", 9)
        self.u.activate()
        self.assertEqual(self.g.units["soldier"], 0)
        self.assertEqual(self.g.sfx[-1], "nope")

    def test_buy_max(self):
        self.u.qty = "max"
        self.g.radiation = self.g.unit_cost_n("soldier", 7) + 1
        self.u.activate()
        self.assertEqual(self.g.units["soldier"], 7)

    def test_row_shows_cost_and_wait(self):
        self.u.qty = 10
        self.g.units["mortar"] = 0
        self.g.units["soldier"] = 2
        r = self.u.unit_rows()[0]
        self.assertIn(game.fmt(self.g.unit_cost_n("soldier", 10)), r["text"])
        self.assertIn("×10", r["desc"])
        self.assertIn("d'attente", r["desc"])

    def test_talent_ranks(self):
        self.u.tab, self.u.qty = 1, 100
        self.g.pt = 1000
        self.u.sel[1] = 0  # premier talent sélectionnable : Détonateur renforcé (max 10)
        self.u.activate()
        self.assertEqual(self.g.tr("t_click"), 10)


class SupernovaUITest(unittest.TestCase):
    def setUp(self):
        self.u = bare_ui()
        self.g = self.u.g
        self.g.prestige_count = 1
        self.g.ptalents["p_cap"] = 4
        self.g.level = self.g.cap
        self.u.tab = 2

    def select(self, kind):
        rows = [r for r in self.u.prestige_rows() if r["sel"]]
        self.u.sel[2] = next(i for i, r in enumerate(rows) if r["action"] and r["action"][0] == kind)

    def test_section_shown(self):
        texts = [r["text"] for r in self.u.prestige_rows()]
        self.assertTrue(any("SUPERNOVA :" in t for t in texts))
        self.assertTrue(any("Fusion stellaire" in t for t in texts))

    def test_hidden_before_unlock(self):
        self.g.ptalents["p_cap"] = 3
        self.assertFalse(any("SUPERNOVA" in r["text"] for r in self.u.prestige_rows()))

    def test_double_confirm(self):
        self.select("supernova")
        self.u.activate()
        self.assertEqual(self.g.supernova_count, 0)
        self.u.particles, self.u.spawn_acc = [], {}
        self.select("supernova")
        self.u.activate()
        self.assertEqual(self.g.supernova_count, 1)

    def test_buy_star_talent(self):
        self.g.stars = 5
        self.select("stalent")
        self.u.activate()
        self.assertEqual(self.g.sr("s_prod"), 1)
        self.assertEqual(self.g.sfx[-1], "talent")


class EffectsTest(unittest.TestCase):
    def test_shake_set_even_without_sound(self):
        u = bare_ui()
        u.g.sfx.append("perfect")
        u.play_sfx()
        self.assertAlmostEqual(u.shake_until, ui.SHAKE["perfect"])

    def test_shake_not_shortened(self):
        u = bare_ui()
        u.g.sfx.extend(["supernova", "perfect"])
        u.play_sfx()
        self.assertAlmostEqual(u.shake_until, ui.SHAKE["supernova"])

    def test_title_disabled_by_default(self):
        u = bare_ui()
        with mock.patch.object(ui.sys, "__stdout__") as out:
            u.update_title()
        out.write.assert_not_called()

    def test_title_only_on_change(self):
        u = bare_ui()
        u.title, u.t = "", 5.0
        with mock.patch.object(ui.sys, "__stdout__") as out:
            u.update_title()
            u.t = 7.0
            u.update_title()  # même niveau : rien
            u.g.level, u.t = 42, 7.5
            u.update_title()  # moins d'une seconde : rien
            u.t = 8.5
            u.update_title()
        writes = [c.args[0] for c in out.write.call_args_list]
        self.assertEqual(len(writes), 2)
        self.assertIn("Niv. 42", writes[1])
        self.assertTrue(writes[1].startswith("\x1b]0;") and writes[1].endswith("\x07"))

    def test_title_masked_when_blind(self):
        u = bare_ui()
        u.title, u.t = "", 5.0
        u.g.challenge = "blind"
        with mock.patch.object(ui.sys, "__stdout__") as out:
            u.update_title()
        self.assertIn("???", out.write.call_args.args[0])


class BlindTest(unittest.TestCase):
    def test_numbers_masked(self):
        u = bare_ui()
        self.assertEqual(u.num(1500), "1.50K")
        u.g.challenge = "blind"
        self.assertEqual(u.num(1500), "???")


class PutScreen:
    """Écran minimal pour UI.put/box : enregistre chaque écriture et refuse ce qui sort de l'écran."""

    def __init__(self, h, w):
        self.h, self.w, self.writes = h, w, []

    def addstr(self, y, x, s, attr=0):
        assert 0 <= y < self.h and 0 <= x and x + len(s) <= self.w, (y, x, s)
        self.writes.append((y, x, s))


class TabBarTest(unittest.TestCase):
    def draw(self, W, qty=1, audio="all"):
        u = bare_ui(sound=FakeSound())
        u.scr = PutScreen(24, W)
        u.h, u.w, u.qty = 24, W, qty
        u.g.audio = audio
        u.g.talents["u_soldier"] = 1
        u.g.radiation = 1e9  # des « • » partout sur les onglets
        u.draw_tab(0, 0, 14, W)
        return u.scr.writes

    def test_all_tabs_visible_and_inside_box(self):
        for W in (26, 30, 36, 44, 60, 80):
            writes = self.draw(W, qty="max")
            top = [(x, s) for y, x, s in writes if y == 0 and s.strip("─┌┐ ")]
            labels = "".join(s for _, s in top)
            for i in range(1, 6):
                self.assertIn(str(i), labels, W)
            self.assertTrue(all(x + len(s) <= W - 1 for x, s in top), W)  # pas sur le bord droit

    def test_help_line_complete_variant(self):
        for W in (30, 44, 60, 80):
            bottom = [s for y, _, s in self.draw(W) if y == 13 and "aide" in s]
            self.assertTrue(bottom, W)
            self.assertTrue(bottom[0].strip().endswith(("Q", "aide")), (W, bottom[0]))


class QtyNoteTest(unittest.TestCase):
    def test_talent_desc_mentions_quantity(self):
        u = bare_ui()
        u.tab_w = 40
        self.assertNotIn("Entrée", u.talent_rows()[2]["desc"])
        u.qty = "max"
        self.assertIn("tous les rangs", u.talent_rows()[2]["desc"])
        u.qty = 10
        self.assertIn("10 rangs", u.talent_rows()[2]["desc"])


class BlindLeakTest(unittest.TestCase):
    def test_unit_rows_masked(self):
        u = bare_ui()
        g = u.g
        g.challenge = "blind"
        g.talents["u_soldier"] = 1
        g.units["soldier"] = 10
        r = u.unit_rows()[0]
        self.assertIn("???/s", r["text"])
        self.assertNotIn("attente", r["desc"])
        self.assertNotIn("manque", r["desc"])
        self.assertTrue(any("Niveau max" in x["text"] and "???" in x["text"] for x in u.info_rows()))

    def test_infinite_cost_and_wait(self):
        self.assertEqual(ui._dur(float("inf")), "∞")
        self.assertEqual(game.fmt(float("inf")), "∞")
        u = bare_ui()
        u.g.talents["u_soldier"] = 1
        u.g.units["soldier"], u.g.units["mortar"] = 6000, 0
        u.g.talents["u_mortar"] = 1
        u.g.units["mortar"] = 1
        r = u.unit_rows()[0]
        self.assertIn("∞", r["text"])


class ChestKeysTest(unittest.TestCase):
    def test_chest_keys_win_over_mutation_and_tabs(self):
        u = bare_ui()
        u.g.pending_mutation = ["spores", "twin", "acid"]
        u.g.pending_chest = ["gauge", "frenzy", "pt"]
        u.handle(ord("1"))
        self.assertIsNone(u.g.pending_chest)
        self.assertEqual(u.g.crit_gauge, 1.0)
        self.assertEqual(u.g.mutations, [])  # la mutation attend son tour
        self.assertEqual(u.tab, 0)


class ChestOverlayKeysTest(unittest.TestCase):
    def test_hidden_mutation_not_picked(self):
        u = bare_ui()
        u.g.pending_mutation = ["spores", "twin", "acid", "fog", "glow"]
        u.g.pending_chest = ["gauge", "frenzy", "pt"]
        u.handle(ord("4"))
        self.assertEqual(u.g.mutations, [])
        self.assertEqual(u.g.pending_chest, ["gauge", "frenzy", "pt"])
        self.assertEqual(u.g.sfx[-1], "nope")
        self.assertEqual(u.tab, 0)

    def test_unusable_choice_refused(self):
        u = bare_ui()
        u.g.pending_chest = ["gauge", "frenzy", "pt"]
        u.g.crit_gauge = 1.0
        u.handle(ord("1"))
        self.assertIsNotNone(u.g.pending_chest)
        self.assertEqual(u.g.sfx[-1], "nope")

    def test_unknown_title_glyphs(self):
        g = exploded_game()
        g.log("x", "good", "☢☢")
        self.assertEqual([t for t, _ in bare_ui(g).banner_lines(60)][0], "☢☢")


class ReportUITest(unittest.TestCase):
    def finished(self):
        u = bare_ui()
        u.g.mutations = ["acid", "fallout"]
        u.g.level = u.g.cap
        u.g.prestige()
        return u

    def test_any_key_closes_and_is_swallowed(self):
        u = self.finished()
        u.handle(ord(" "))
        self.assertFalse(u.g.report_ready)
        self.assertFalse(u.g.exploded)  # ESPACE n'a pas fait exploser la bombe
        u.handle(ord(" "))
        self.assertTrue(u.g.exploded)

    def test_resize_keeps_report(self):
        u = self.finished()
        u.scr = mock.Mock()
        u.handle(curses.KEY_RESIZE)
        self.assertTrue(u.g.report_ready)
        u.scr.clear.assert_called_once()

    def test_report_lines(self):
        u = self.finished()
        lines = dict(u.report_lines(u.g.last_run))
        self.assertIn("⚛", lines["Gain"])
        self.assertEqual(lines["Synergies"], "Orage acide")
        self.assertEqual(lines["Coffres"], "aucun")
        u.g.last_run["chests"] = ["A"] * 9
        self.assertIn("(+3)", dict(u.report_lines(u.g.last_run))["Coffres"])

    def test_draw_report_fits(self):
        for W, H in ((60, 20), (80, 24), (140, 40)):
            u = self.finished()
            u.scr = PutScreen(H, W)
            u.h, u.w = H, W
            u.draw_report()
            self.assertTrue(any("FIN DE PARTIE" in s for _, _, s in u.scr.writes), W)

    def test_long_values_get_ellipsis(self):
        u = self.finished()
        u.g.last_run["mutations"] = [m[1] for m in events.MUTATIONS]
        u.scr = PutScreen(20, 60)
        u.h, u.w = 20, 60
        u.draw_report()
        self.assertTrue(any(s.endswith("…") or s == "…" for _, _, s in u.scr.writes))

    def test_key_not_swallowed_when_report_invisible(self):
        u = self.finished()
        u.w, u.h = 40, 10  # « Terminal trop petit » : l'écran de fin n'est pas dessiné
        u.handle(ord(" "))
        self.assertTrue(u.g.exploded)

    def test_infos_tab_shows_last_run(self):
        u = self.finished()
        texts = [r["text"] for r in u.info_rows()]
        self.assertTrue(any("DERNIÈRE PARTIE" in t for t in texts))
        self.assertTrue(all(len(t) <= u.tab_w for t in texts if "──" not in t))


class SynergyUITest(unittest.TestCase):
    def test_collection_in_achievements(self):
        u = bare_ui()
        u.g.synergies_found.add("storm")
        u.g.mutations = ["acid", "fallout"]
        texts = [r["text"] for r in u.achievement_rows()]
        self.assertTrue(any("SYNERGIES (1/6)" in t for t in texts))
        self.assertTrue(any("Orage acide (active)" in t for t in texts))
        self.assertEqual(sum("???" in t for t in texts), 5)

    def test_mutation_hint(self):
        u = bare_ui()
        u.g.mutations = ["spores"]
        u.g.pending_mutation = ["mitosis", "glow", "fog"]
        u.scr = PutScreen(24, 80)
        u.h, u.w = 24, 80
        u.draw_mutation()
        self.assertTrue(any("synergie inconnue" in s for _, _, s in u.scr.writes))
        u.g.synergies_found.add("proliferation")
        u.scr = PutScreen(24, 80)
        u.draw_mutation()
        self.assertTrue(any("★ Prolifération" in s for _, _, s in u.scr.writes))


class BigTitleTest(unittest.TestCase):
    def test_font(self):
        import art
        rows = art.big_text("Gelé !")
        self.assertEqual(len(rows), 3)
        self.assertEqual(rows, art.big_text("GELE!".replace("!", " !")))
        self.assertEqual(art.big_text("☢"), ["", "", ""])
        self.assertEqual(art.big_text("cœur"), art.big_text("COEUR"))
        for ch, glyph in art.BIG_FONT.items():
            self.assertEqual(len({len(r) for r in glyph}), 1, ch)

    def test_newest_message_gets_big_title(self):
        g = exploded_game()
        g.log("Coffre : Frénésie !", "loot", "COFFRE")
        lines = [t for t, _ in bare_ui(g).banner_lines(60)]
        self.assertEqual(len(lines), 4)  # 3 lignes de titre + le bandeau
        self.assertTrue(all("█" in t or "▀" in t for t in lines[:3]))
        self.assertEqual(len({len(t) for t in lines[:3]}), 1)  # même largeur : centrage aligné
        self.assertEqual(lines[3], "Coffre : Frénésie !")

    def test_big_title_expires_unless_important(self):
        g = exploded_game()
        g.log("Coffre", "loot", "COFFRE")
        g.time += ui.BIG_TIME + 0.1
        self.assertEqual(len(bare_ui(g).banner_lines(60)), 1)
        g.log("Succès", "star", "SUCCÈS !")
        g.time += ui.BIG_TIME + 0.1
        lines = [t for t, _ in bare_ui(g).banner_lines(60)]
        self.assertEqual(len(lines), 3 + 1)  # titre toujours là ; « Coffre » (6,2 s) a expiré
        self.assertEqual(lines[3], "Succès")

    def test_only_newest_is_big(self):
        g = exploded_game()
        g.log("a", "good", "UN")
        g.log("b", "good", "DEUX")
        lines = [t for t, _ in bare_ui(g).banner_lines(60)]
        self.assertEqual(len(lines), 3 + 2)

    def test_too_wide_falls_back_to_spaced(self):
        g = exploded_game()
        g.log("x", "epic", "SUPERNOVA")
        lines = [t for t, _ in bare_ui(g).banner_lines(20)]
        self.assertEqual(lines[0], "S U P E R N O V A")


class InfoTabTest(unittest.TestCase):
    def setUp(self):
        self.u = bare_ui()
        self.u.tab, self.u.tab_w = 0, 40

    def test_key_5_opens_infos(self):
        self.u.handle(ord("5"))
        self.assertEqual(ui.TABS[self.u.tab], "Infos")

    def test_mutation_keys_take_priority(self):
        self.u.g.pending_mutation = ["spores", "twin", "acid", "fog", "glow"]
        self.u.handle(ord("5"))
        self.assertEqual(self.u.tab, 0)
        self.assertIn("glow", self.u.g.mutations)

    def test_content(self):
        g = self.u.g
        g.stats["clicks"], g.stats["total_time"], g.stats["best_prestige_time"] = 1234, 3725, None
        texts = [r["text"] for r in self.u.info_rows() if r["sel"]]
        self.assertTrue(any("ESPACE" in t for t in texts))
        self.assertTrue(any("Appuis sur ESPACE" in t and "1.23K" in t for t in texts))
        self.assertTrue(any("1 h 02 min" in t for t in texts))
        self.assertTrue(any("Meilleur prestige" in t and "—" in t for t in texts))
        self.assertTrue(all(len(t) <= 40 for t in texts))

    def test_rows_scrollable(self):
        self.assertTrue(any(r["sel"] for r in self.u.info_rows()))

    def test_dur(self):
        self.assertEqual(ui._dur(42), "42 s")
        self.assertEqual(ui._dur(125), "2 min 05 s")
        self.assertEqual(ui._dur(7260), "2 h 01 min")


class AgoTest(unittest.TestCase):
    def test_units(self):
        now = time.time()
        self.assertEqual(ui._ago(now - 30), "à l'instant")
        self.assertEqual(ui._ago(now - 125), "il y a 2 min")
        self.assertEqual(ui._ago(now - 7300), "il y a 2 h")
        self.assertEqual(ui._ago(now - 3 * 86400 - 5), "il y a 3 j")


class FakeScreen:
    def __init__(self, keys, size=(30, 100)):
        self.keys = list(keys)
        self.size = size
        self.text = []
        self.delay = []

    def getmaxyx(self):
        return self.size

    def erase(self):
        self.text = []

    def addnstr(self, y, x, s, n, attr=0):
        # Mêmes refus que curses : hors écran, n <= 0, ou écriture jusqu'au coin bas-droit.
        h, w = self.size
        s = s[:n]
        if n <= 0 or not (0 <= y < h and 0 <= x < w) or x + len(s) > w or (y == h - 1 and x + len(s) >= w):
            raise curses.error("addnstr() returned ERR")
        self.text.append(s)

    def refresh(self):
        pass

    def nodelay(self, flag):
        self.delay.append(flag)

    def getch(self):
        return self.keys.pop(0)


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
        return ui.save_menu(scr, self.slots, FakeSound()), scr

    def test_enter_continues_main(self):
        path, scr = self.menu([10])
        self.assertEqual(path, saves.slot_path(1))
        self.assertEqual(scr.delay, [False, True])  # mode bloquant restauré

    def test_navigate_to_second(self):
        path, _ = self.menu([curses.KEY_DOWN, ord("z"), 10])
        self.assertEqual(path, saves.slot_path(2))

    def test_new_game_uses_free_slot(self):
        path, scr = self.menu([curses.KEY_UP, 10])  # remonte en boucle sur « Nouvelle partie »
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
        self.assertFalse(any("Partie principale" in t for t in scr.text))  # défilé hors écran

    def test_bad_field_types(self):
        self.slots = [(1, saves.slot_path(1), {"level": "5", "isotopes": None, "prestige_count": [],
                                              "wall_time": None})]
        path, scr = self.menu([10])
        self.assertEqual(path, saves.slot_path(1))
        self.assertTrue(any("niveau 0" in t for t in scr.text))

    def test_all_slots_taken(self):
        with mock.patch.object(ui, "free_slot", return_value=None):
            sound = FakeSound()
            scr = FakeScreen([curses.KEY_UP, 10, ord("q")])
            self.assertIsNone(ui.save_menu(scr, self.slots, sound))
        self.assertIn("nope", sound.played)
        self.assertTrue(any("Plus d'emplacement libre" in t for t in scr.text))


if __name__ == "__main__":
    unittest.main()


class NanarUITest(unittest.TestCase):
    def test_fizzle_float(self):
        u = bare_ui(exploded_game(FixedRandom(0.0)))
        u.floats = []
        u.g.challenge = "nanar"
        u.handle(ord(" "))
        self.assertEqual(u.floats[-1]["text"], "pschitt…")

    def test_story_shown_once_then_cleared(self):
        u = bare_ui()
        u.scr = FakeScreen([ord("a"), ord("b")], size=(30, 100))
        u.scr.timeout = lambda ms: None
        u.g.pending_story = ("nanar", "start")
        with mock.patch.object(ui, "STORY_GUARD", 0.0):
            u.show_story()
        self.assertIsNone(u.g.pending_story)
        self.assertEqual(u.scr.keys, [])  # l'écran d'histoire a consommé ses deux touches
        self.assertTrue(any("Zédé" in t for t in u.scr.text))

    def test_story_ignores_keys_at_first(self):
        # Le joueur martèle encore ESPACE quand l'histoire s'ouvre : ces appuis ne la ferment pas.
        clock = iter(i * 0.3 for i in range(1000))
        scr = FakeScreen([ord(" ")] * 4)
        scr.timeout = lambda ms: None
        with mock.patch.object(ui.time, "monotonic", lambda: next(clock)), self.assertRaises(IndexError):
            ui.intro(scr, None, ui.STORIES[("nanar", "end")], guard=60)  # toujours ouverte quand les touches s'épuisent
        self.assertEqual(scr.keys, [])  # les 4 appuis ont été lus (et ignorés), l'IndexError vient de getch
        self.assertFalse(any("continuer" in t or "commencer" in t for t in scr.text))  # texte pas encore affiché en entier

    def test_challenge_without_story_shows_nothing(self):
        u = bare_ui()
        u.scr = FakeScreen([ord("a")], size=(30, 100))
        u.g.pending_story = ("race", "end")
        u.show_story()
        self.assertIsNone(u.g.pending_story)
        self.assertEqual(u.scr.keys, [ord("a")])

    def test_stories_match_challenges(self):
        for cid, part in ui.STORIES:
            self.assertIn(cid, data.CHALLENGE_BY_ID)
            self.assertIn(part, ("start", "end"))
        self.assertIn(("nanar", "end"), ui.STORIES)

    def test_replay_story_with_only_one_part(self):
        u = bare_ui()
        u.scr = FakeScreen([], size=(30, 100))
        u.g.challenges_done.add("race")
        with mock.patch.dict(ui.STORIES, {("race", "end"): ["FIN"]}), mock.patch.object(ui, "intro") as shown:
            action = next(r["action"] for r in u.info_rows() if r["text"].startswith("Revoir le défi"))
            with mock.patch.object(u, "selected_action", return_value=action):
                u.activate()  # pas de KeyError sur la partie "start" absente
        self.assertEqual(shown.call_count, 1)

    def test_replay_row_after_success(self):
        u = bare_ui()
        label = "Revoir le défi Nanarland"
        self.assertFalse(any(r.get("text", "").startswith(label) for r in u.info_rows()))
        u.g.challenges_done.add("nanar")
        self.assertTrue(any(r.get("text", "").startswith(label) for r in u.info_rows()))


class VolumeKeysTest(unittest.TestCase):
    def test_plus_minus(self):
        u = bare_ui(sound=FakeSound())
        u.handle(ord("-"))
        u.handle(ord("-"))
        self.assertEqual(u.g.volume, 90)
        u.handle(ord("+"))
        self.assertEqual(u.g.volume, 95)
        u.play_sfx()
        self.assertAlmostEqual(u.sound.volume, 0.95)
        self.assertIn("tick", u.sound.played)
        self.assertEqual(u.g.messages[-1][0], "Volume : 95 %")
        self.assertEqual(sum(m[0].startswith("Volume : ") for m in u.g.messages), 1)


class AudioModeKeyTest(unittest.TestCase):
    def test_zero_cycles_and_announces(self):
        u = bare_ui()
        seen = []
        for _ in range(3):
            u.handle(ord("0"))
            seen.append((u.g.audio, u.g.messages[-1][0]))
        self.assertEqual([a for a, _ in seen], ["sfx", "off", "all"])
        self.assertEqual(seen[1][1], "Son : muet (0 pour le remettre)")


class FieldDrawTest(unittest.TestCase):
    """Le panneau du champignon avec toute l'armée et chaque ennemi reste dans l'écran."""

    def test_army_and_enemies_inside_screen(self):
        for W in (30, 45, 80):
            for kind in events.ENEMIES:
                u = bare_ui()
                u.particles, u.floats, u.spawn_acc = [], [], {}
                u.h, u.w = 24, W
                u.scr = PutScreen(24, W)
                g = u.g
                g.level = 50
                for uid in g.units:
                    g.units[uid] = 150
                g.enemies = [dict(kind=kind, start=g.time - 3, dur=10.0, hits=1),
                             dict(kind=kind, start=g.time - 1, dur=10.0, hits=0)]
                for t in (0.0, 0.7):
                    u.t = t
                    u.draw_mushroom(0, 0, 23, W, 0.05)
                self.assertTrue(u.scr.writes)

    def test_enemy_banner(self):
        u = bare_ui()
        u.g.enemies = [dict(kind="drone", start=0, dur=15, hits=0)]
        self.assertIn("Drone espion", u.banner_lines(40)[0][0])
        self.assertEqual(u.music_track(), "tension")
        u.g.enemies.append(dict(kind="saboteur", start=0, dur=12, hits=0))
        self.assertIn("2 ennemis", u.banner_lines(40)[0][0])


class LongDescTest(unittest.TestCase):
    def test_challenge_reward_visible(self):
        u = bare_ui()
        u.g.prestige_count = 1
        u.tab = 2
        u.sel[2] = 1  # premier défi
        u.scr = PutScreen(30, 60)
        u.h, u.w = 30, 60
        u.draw_tab(10, 0, 19, 60)
        text = "".join(w[2] for w in u.scr.writes)
        self.assertIn("Récompense", text)

    def test_ascete_no_talent_dot(self):
        u = bare_ui()
        u.g.challenge = "ascete"
        u.g.talents["u_soldier"] = 1
        u.g.talents["u_mortar"] = 1
        u.g.talents["u_artillery"] = 1
        u.g.pt = 2  # assez pour t_click (1 PT) mais pas pour le déblocage suivant (8 PT)
        self.assertFalse(u.tab_has_news(1))


class UnitSpriteTest(unittest.TestCase):
    def draw(self, W, H=19, sel=0):
        u = bare_ui()
        u.g.talents["u_soldier"] = 1
        u.sel[0] = sel
        u.scr = PutScreen(30, W)
        u.h, u.w = 30, W
        u.draw_tab(10, 0, H, W)
        return u.scr.writes

    @staticmethod
    def art_line(writes, y, x0=60):
        """Caractères isolés écrits sur la ligne y à droite de x0 (le sprite est écrit caractère par caractère)."""
        return "".join(ch for wy, wx, ch in sorted(writes) if wy == y and wx >= x0 and len(ch) == 1 and ch != "│")

    def test_selected_unit_sprite_beside_desc(self):
        writes = self.draw(80)
        self.assertEqual([self.art_line(writes, y) for y in (25, 26, 27)], ["o", "/|=", "/\\"])
        self.assertIn((26, 75, "/"), writes)  # collé au bord droit de la zone de description
        desc = [w for w in writes if w[0] == 25 and w[1] == 2][0][2]
        self.assertLessEqual(2 + len(desc), 75 - 2)  # le texte ne passe pas sous le dessin

    def test_locked_unit_silhouette(self):
        writes = self.draw(80, sel=1)  # ligne « ??? » : le mortier à débloquer
        self.assertEqual(self.art_line(writes, 27), "[_]")

    def test_no_sprite_when_narrow_or_short(self):
        for W, H in ((36, 19), (80, 8)):
            writes = self.draw(W, H)
            self.assertFalse(any(w[2] == "=" for w in writes), (W, H))


class IntroTest(unittest.TestCase):
    def test_first_key_reveals_second_closes(self):
        scr = FakeScreen([-1, ord("a"), ord("b")])
        scr.timeout = lambda ms: None
        ui.intro(scr, FakeSound())
        self.assertEqual(scr.keys, [])
        self.assertTrue(any("Morille" in t for t in scr.text))
        self.assertTrue(any("commencer" in t for t in scr.text))

    def test_small_screen(self):
        scr = FakeScreen([ord("a"), ord("b")], size=(8, 20))
        scr.timeout = lambda ms: None
        ui.intro(scr)

    def test_texts_fit_80x24(self):
        # Le terminal par défaut : le dernier paragraphe (introduction ou histoire) doit rester visible.
        for pages in [ui.INTRO] + list(ui.STORIES.values()):
            scr = FakeScreen([ord("a"), ord("b")], size=(24, 80))
            scr.timeout = lambda ms: None
            ui.intro(scr, pages=pages)
            last = ui._nbsp(pages[-1]).split()[-1]
            self.assertTrue(any(last in t for t in scr.text), pages[0])
