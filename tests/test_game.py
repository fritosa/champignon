import json
import os
import tempfile
import unittest
from unittest import mock

import data
import game
import saves
from game import Game, fmt, need

from .helpers import FixedRandom, exploded_game


class FormatTest(unittest.TestCase):
    def test_small_numbers(self):
        self.assertEqual(fmt(5), "5")
        self.assertEqual(fmt(2.5), "2.5")
        self.assertEqual(fmt(150), "150")

    def test_suffixes(self):
        self.assertEqual(fmt(1500), "1.50K")
        self.assertEqual(fmt(2e6), "2.00M")
        self.assertEqual(fmt(1e40), "1.00e+40")

    def test_need_grows(self):
        self.assertEqual(need(1), 25)
        self.assertLess(need(10), need(11))


class ClickTest(unittest.TestCase):
    def test_first_click_explodes(self):
        g = Game()
        g.click()
        self.assertTrue(g.exploded)
        self.assertEqual(g.level, 1)
        self.assertIn("boom", g.sfx)
        self.assertEqual(g.messages[-1][2], "epic")

    def test_manual_click_gains_and_sounds(self):
        g = exploded_game()
        g.click()
        self.assertEqual(g.stats["clicks"], 1)
        self.assertAlmostEqual(g.radiation, 1.08)
        self.assertEqual(list(g.sfx), ["click0"])

    def test_click_pitch_follows_combo(self):
        g = exploded_game()
        g.combo = g.max_combo() - 1
        g.click()
        self.assertEqual(g.sfx[-1], "click7")

    def test_auto_click_is_silent(self):
        g = exploded_game()
        g.click(manual=False)
        self.assertGreater(g.radiation, 0)
        self.assertEqual(g.stats["clicks"], 0)
        self.assertEqual(list(g.sfx), [])

    def test_overheat(self):
        g = exploded_game()
        for _ in range(g.overheat_threshold() + 1):
            g.click()
        self.assertTrue(g.overheated)
        self.assertIn("overheat", g.sfx)
        before = g.radiation
        g.sfx.clear()
        g.click()
        self.assertEqual(g.radiation, before)
        self.assertEqual(list(g.sfx), ["jam"])  # press refused but audible

    def test_crit_sound_only_when_manual(self):
        g = exploded_game(FixedRandom(0.0))
        g.talents["t_crit"] = 1
        g.click()
        self.assertIn("crit", g.sfx)
        self.assertAlmostEqual(g.radiation, 10 * 1.08)  # base 1 × combo × crit
        g.sfx.clear()
        g.click(manual=False)
        self.assertNotIn("crit", g.sfx)

    def test_onearm_forbids_space(self):
        g = exploded_game()
        g.challenge = "onearm"
        g.click()
        g.click()
        self.assertEqual(g.stats["clicks"], 0)
        self.assertIn("nope", g.sfx)
        self.assertEqual(len(g.messages), 1)  # repeated message → deduplicated


class LevelTest(unittest.TestCase):
    def test_level_up_gives_pt_and_sound(self):
        g = exploded_game()
        pt = g.pt
        g.gain(need(1))
        self.assertEqual(g.level, 2)
        self.assertEqual(g.pt, pt + 1)
        self.assertIn("level", g.sfx)

    def test_level_sound_only_when_pt_awarded(self):
        g = exploded_game()
        g.ptalents["p_cap"] = 1
        g.level, g.next_pt_level, g.mut_index = 200, 300, 10
        g.gain(need(200))
        self.assertEqual(g.level, 201)
        self.assertNotIn("level", g.sfx)
        g.next_pt_level = 202
        g.gain(need(201))
        self.assertIn("level", g.sfx)

    def test_level_capped(self):
        g = exploded_game()
        g.gain(1e30)
        self.assertEqual(g.level, g.cap)
        self.assertEqual(g.progress, 0)


class ShopTest(unittest.TestCase):
    def test_unit_needs_unlock(self):
        g = exploded_game()
        g.radiation = 1000
        self.assertFalse(g.buy_unit("soldier"))
        g.talents["u_soldier"] = 1
        self.assertTrue(g.buy_unit("soldier"))
        self.assertEqual(g.units["soldier"], 1)
        self.assertAlmostEqual(g.radiation, 1000 - 15)
        self.assertAlmostEqual(g.unit_cost("soldier"), 15 * data.UNIT_COST_GROWTH)

    def test_buy_unit_max(self):
        g = exploded_game()
        g.talents["u_soldier"] = 1
        g.radiation = g.unit_cost_n("soldier", 2)
        self.assertEqual(g.buy_unit_max("soldier"), 2)
        self.assertFalse(g.can_buy_unit("soldier"))

    def test_cost_n_and_affordable(self):
        g = exploded_game()
        g.talents["u_soldier"] = 1
        self.assertAlmostEqual(g.unit_cost_n("soldier", 2), 15 + 15 * data.UNIT_COST_GROWTH)
        g.radiation = g.unit_cost_n("soldier", 2) + 1
        self.assertEqual(g.affordable("soldier"), 2)
        g.talents.pop("u_soldier")
        self.assertEqual(g.affordable("soldier"), 0)

    def test_buy_n_all_or_nothing(self):
        g = exploded_game()
        g.talents["u_soldier"] = 1
        g.radiation = g.unit_cost_n("soldier", 10) - 1
        self.assertEqual(g.buy_unit_n("soldier", 10), 0)
        self.assertEqual(g.units["soldier"], 0)
        g.radiation += 1
        self.assertEqual(g.buy_unit_n("soldier", 10), 10)
        self.assertEqual(g.units["soldier"], 10)
        self.assertAlmostEqual(g.radiation, 0, places=6)

    def test_buy_n_exact_budget_rounding(self):
        import random
        rng = random.Random(0)
        for _ in range(500):
            g = exploded_game()
            g.talents["u_soldier"] = 1
            g.units["soldier"] = rng.randint(0, 300)
            k = rng.choice((10, 100))
            g.radiation = g.unit_cost_n("soldier", k)
            before = g.units["soldier"]
            self.assertEqual(g.buy_unit_n("soldier", k), k)
            self.assertEqual(g.units["soldier"], before + k)
            self.assertGreaterEqual(g.radiation, 0)

    def test_bulk_crosses_milestones(self):
        g = exploded_game()
        g.talents["u_soldier"] = 1
        g.units["soldier"] = 45
        g.radiation = 1e30
        g.buy_unit_n("soldier", 60)
        self.assertEqual(g.sfx.count("milestone"), 2)  # 50 and 100

    def test_affordable_bounded(self):
        g = exploded_game()
        g.talents["u_soldier"] = 1
        g.radiation = float("inf")
        self.assertEqual(g.affordable("soldier"), game.MAX_BULK)
        self.assertEqual(g.buy_unit_max("soldier"), game.MAX_BULK)

    def test_talent_cost_and_requirements(self):
        g = exploded_game()
        g.pt = 10
        self.assertFalse(g.buy_talent("t_crit"))  # requires t_click
        self.assertTrue(g.buy_talent("t_click"))
        self.assertEqual(g.pt, 9)
        self.assertEqual(g.talent_cost("t_click"), 2)
        self.assertTrue(g.buy_talent("t_crit"))

    def test_production(self):
        g = exploded_game()
        g.units["soldier"] = 10
        self.assertAlmostEqual(g.production(), 10 * 0.5)
        g.freeze_until = g.time + 5
        self.assertEqual(g.production(), 0)


class MilestoneTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()
        self.g.talents["u_soldier"] = 1
        self.first = data.UNIT_MILESTONES[0]

    def test_production_multiplied(self):
        g = self.g
        g.units["soldier"] = self.first - 1
        base = g.unit_prod("soldier")
        self.assertEqual(g.next_milestone("soldier"), self.first)
        g.units["soldier"] = self.first
        self.assertEqual(g.unit_milestones("soldier"), 1)
        self.assertAlmostEqual(g.unit_prod("soldier"), base * data.MILESTONE_MULT)

    def test_crossing_announced(self):
        g = self.g
        g.units["soldier"] = self.first - 1
        g.radiation = 1e12
        g.buy_unit("soldier")
        self.assertIn("milestone", g.sfx)
        self.assertEqual(g.messages[-1][2], "good")
        g.sfx.clear()
        g.buy_unit("soldier")
        self.assertNotIn("milestone", g.sfx)

    def test_all_reached(self):
        self.g.units["soldier"] = data.UNIT_MILESTONES[-1]
        self.assertIsNone(self.g.next_milestone("soldier"))
        self.assertEqual(self.g.unit_milestones("soldier"), len(data.UNIT_MILESTONES))


class PrestigeTest(unittest.TestCase):
    def test_prestige_resets_run(self):
        g = exploded_game()
        g.level = g.cap
        gain = g.prestige_gain()
        self.assertTrue(g.prestige())
        self.assertEqual(g.isotopes, gain)
        self.assertEqual(g.prestige_count, 1)
        self.assertEqual(g.level, 0)
        self.assertFalse(g.exploded)
        self.assertIn("prestige", g.sfx)

    def test_prestige_requires_max_level(self):
        g = exploded_game()
        self.assertFalse(g.prestige())

    def test_challenge_success(self):
        g = exploded_game()
        ch = data.CHALLENGES[0]
        g.challenge = ch["id"]
        g.level = ch["target"] - 1
        g.ptalents["p_cap"] = 4
        g.mut_index = 20
        g.level += 1
        g.on_level_up()
        self.assertIn(ch["id"], g.challenges_done)
        self.assertIn("fanfare", g.sfx)
        self.assertEqual(g.pending_story, (ch["id"], "end"))

    def test_challenge_start_requests_story(self):
        g = exploded_game()
        g.prestige_count = 1
        g.prestige(challenge="nanar")
        self.assertEqual(g.pending_story, ("nanar", "start"))


class SupernovaTest(unittest.TestCase):
    def ready(self):
        g = exploded_game()
        g.ptalents["p_cap"] = data.PTALENT_BY_ID["p_cap"]["max"]
        g.level = g.cap
        return g

    def test_locked_before_max_core(self):
        g = exploded_game()
        g.level = g.cap
        self.assertFalse(g.supernova_unlocked)
        self.assertFalse(g.can_supernova())
        self.assertFalse(g.supernova())

    def test_gain_counts_isotopes_earned(self):
        g = self.ready()
        g.stats["iso_earned"] = 50_000
        expected = int((( 50_000 + g.prestige_gain()) / data.SUPERNOVA_DIVISOR) ** 0.5)
        self.assertEqual(g.supernova_gain(), expected)

    def test_supernova_resets_and_keeps(self):
        g = self.ready()
        g.isotopes, g.prestige_count, g.stats["iso_earned"] = 999, 7, 100_000
        g.ptalents["p_prod"] = 5
        g.achievements.add("boom")
        g.skins.add("plasma")
        g.challenges_done.add("race")
        gain = g.supernova_gain()
        self.assertTrue(g.supernova())
        self.assertEqual((g.stars, g.supernova_count, g.isotopes), (gain, 1, 0))
        self.assertEqual(g.ptalents, {})
        self.assertEqual(g.cap, data.BASE_CAP)
        self.assertEqual(g.prestige_count, 7)
        self.assertEqual(g.stats["iso_earned"], 0)
        self.assertIn("boom", g.achievements)
        self.assertIn("race", g.challenges_done)
        self.assertFalse(g.exploded)
        self.assertIn("supernova", g.sfx)
        self.assertTrue(g.supernova_unlocked)

    def test_prestige_tracks_isotopes(self):
        g = exploded_game()
        g.level = g.cap
        gain = g.prestige_gain()
        g.prestige()
        self.assertEqual(g.stats["iso_earned"], gain)

    def test_star_talents(self):
        g = exploded_game()
        g.units["soldier"] = 10
        g.level = g.cap
        prod, iso = g.production(), g.prestige_gain()
        g.stars = 100
        self.assertTrue(g.buy_stalent("s_prod"))
        self.assertTrue(g.buy_stalent("s_iso"))
        self.assertAlmostEqual(g.production(), prod * 3)
        self.assertEqual(g.prestige_gain(), iso * 2)
        self.assertEqual(g.stars, 100 - 1 - 1)

    def test_dense_core_discounts_expansion(self):
        g = exploded_game()
        full = g.ptalent_cost("p_cap")
        g.stars = 10
        g.buy_stalent("s_core")
        self.assertEqual(g.ptalent_cost("p_cap"), round(full * 0.8))
        g.supernova_count = 1
        self.assertEqual(g.cap, data.BASE_CAP)  # leaves the starting max level alone

    def test_auto_talents(self):
        g = exploded_game()
        g.stalents["s_auto"] = 1
        g.pt = 3
        g.tick(1.0)
        self.assertEqual(g.tr("u_soldier"), 1)  # unlock bought first
        self.assertFalse(any(g.talent_available(t["id"]) and g.pt >= g.talent_cost(t["id"])
                             for t in data.TALENTS))  # then everything affordable

    def test_star_cost_and_max(self):
        g = exploded_game()
        g.stars = 1000
        while g.buy_stalent("s_auto"):
            pass
        self.assertEqual(g.sr("s_auto"), 1)
        self.assertEqual(g.stalent_cost("s_auto"), float("inf"))

    def test_saved(self):
        g = exploded_game()
        g.stars, g.supernova_count, g.stalents = 4, 2, {"s_prod": 3}
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "s.json")
            g.save(path)
            h = Game.load(path)
        self.assertEqual((h.stars, h.supernova_count, h.stalents), (4, 2, {"s_prod": 3}))


class RunReportTest(unittest.TestCase):
    def test_run_stats_tracked(self):
        g = exploded_game(FixedRandom(0.0))
        g.talents["t_crit"] = 1
        g.click()
        g.click(manual=False)
        rs = g.run_stats
        self.assertEqual((rs["clicks"], rs["crits"]), (1, 1))  # the automatic press doesn't count
        self.assertAlmostEqual(rs["best_click"], 10 * 1.08)
        self.assertAlmostEqual(rs["earned"], g.radiation)

    def test_prestige_report(self):
        g = exploded_game()
        g.mutations = ["spores", "mitosis"]
        g.run_stats["chests"] = ["Frénésie"]
        g.units["soldier"] = 12
        g.level, g.run_time = g.cap, 300
        gain = g.prestige_gain()
        g.prestige()
        r = g.last_run
        self.assertTrue(g.report_ready)
        self.assertEqual((r["kind"], r["gain"], r["level"], r["time"], r["units"], r["number"]),
                         ("prestige", gain, data.BASE_CAP, 300, 12, 1))
        self.assertEqual(r["synergies"], ["Prolifération"])
        self.assertEqual(r["chests"], ["Frénésie"])
        self.assertEqual(g.run_stats["chests"], [])  # reset for the next run

    def test_challenge_start_without_gain_has_no_report(self):
        g = exploded_game()
        g.prestige_count = 1
        g.prestige(challenge="race")
        self.assertFalse(g.report_ready)
        self.assertIsNone(g.last_run)

    def test_supernova_report(self):
        g = exploded_game()
        g.ptalents["p_cap"] = 4
        g.level = g.cap
        gain = g.supernova_gain()
        g.supernova()
        self.assertEqual((g.last_run["kind"], g.last_run["gain"]), ("supernova", gain))

    def write(self, d):
        path = os.path.join(self.dir.name, "s.json")
        with open(path, "w") as f:
            json.dump(d, f)
        return Game.load(path)

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)

    def test_corrupt_last_run_dropped(self):
        for bad in ({"kind": "prestige"}, {"kind": "x"}, "texte", None):
            d = exploded_game().to_dict()
            d["last_run"] = bad
            self.assertIsNone(self.write(d).last_run, bad)
        g = exploded_game()
        g.level = g.cap
        g.prestige()
        d = g.to_dict()
        d["last_run"]["mutations"] = None
        self.assertIsNone(self.write(d).last_run)

    def test_corrupt_run_stats_repaired(self):
        for bad in ([1, 2], None, {"best_click": "x", "chests": "abc", "clicks": 4}):
            d = exploded_game().to_dict()
            d["run_stats"] = bad
            g = self.write(d)
            g.click()
            g.pending_chest = ["frenzy"]
            import events
            events.choose_chest(g, 0)  # no longer crashes
            self.assertEqual(g.run_stats["chests"], ["Frénésie"])
        self.assertEqual(g.run_stats["clicks"], 5)  # the valid value (4) is kept

    def test_unknown_mutations_filtered(self):
        d = exploded_game().to_dict()
        d["mutations"], d["pending_mutation"] = ["spores", "zzz"], ["zzz"]
        g = self.write(d)
        self.assertEqual((g.mutations, g.pending_mutation), (["spores"], None))
        g.level = g.cap
        self.assertTrue(g.prestige())  # no more KeyError in the summary

    def test_report_and_synergies_saved(self):
        g = exploded_game()
        g.level = g.cap
        g.prestige()
        g.synergies_found = {"storm", "bogus"}
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "s.json")
            g.save(path)
            h = Game.load(path)
        self.assertEqual(h.last_run["kind"], "prestige")
        self.assertFalse(h.report_ready)  # the screen doesn't come back on reload
        self.assertEqual(h.synergies_found, {"storm"})


class AchievementTest(unittest.TestCase):
    def test_unlock_once(self):
        g = exploded_game()
        g.check_achievements()
        self.assertIn("boom", g.achievements)
        self.assertIn("achievement", g.sfx)
        self.assertEqual(g.messages[-1][2], "star")
        n = len(g.messages)
        g.check_achievements()
        self.assertEqual(len(g.messages), n)


class WatchedTimerTest(unittest.TestCase):
    def test_abandon_restores_normal_timer(self):
        g = exploded_game()
        g.challenge = "watched"
        g.reset_run()
        g.abandon_challenge()
        self.assertGreaterEqual(g.next_inspector - g.time, 150)

    def test_completion_restores_normal_timer(self):
        g = exploded_game()
        g.challenge, g.level, g.mut_index = "watched", 99, 20
        g.level = 100
        g.on_level_up()
        self.assertIsNone(g.challenge)
        self.assertGreaterEqual(g.next_inspector - g.time, 150)


class BlindMessagesTest(unittest.TestCase):
    def test_game_fmt_masks(self):
        g = exploded_game()
        self.assertEqual(g.fmt(1500), "1.50K")
        g.challenge = "blind"
        self.assertEqual(g.fmt(1500), "???")

    def test_drop_message_masked(self):
        import events
        g = exploded_game()
        g.challenge = "blind"
        g.plane = dict(letter="a", start=0, dur=10)
        g.rng = FixedRandom(0.5)
        events.catch(g, "a")
        g.units["soldier"] = 100
        self.assertIn("???", events.chest_text(g, "lump")[1])
        g.pending_chest = ["lump"]
        events.choose_chest(g, 0)
        self.assertIn("???", g.messages[-1][0])


class LogTest(unittest.TestCase):
    def test_kind_and_dedupe(self):
        g = Game()
        g.log("salut")
        g.time = 3
        g.log("salut", "bad")
        self.assertEqual(list(g.messages), [("salut", 3, "bad", None)])
        g.log("autre")
        self.assertEqual(len(g.messages), 2)

    def test_sfx_queue_bounded(self):
        g = exploded_game()
        for _ in range(100):
            g.click(manual=False)
            g.sfx.append("x")
        self.assertLessEqual(len(g.sfx), 16)


class SaveTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "s.json")

    def tearDown(self):
        self.dir.cleanup()

    def test_roundtrip(self):
        g = exploded_game()
        g.level, g.pt, g.audio = 12, 7, "sfx"
        g.units["soldier"] = 3
        g.achievements.add("boom")
        g.save(self.path)
        h = Game.load(self.path)
        self.assertEqual((h.level, h.pt, h.audio, h.units["soldier"]), (12, 7, "sfx", 3))
        self.assertEqual(h.achievements, {"boom"})
        self.assertFalse(os.path.exists(self.path + ".tmp"))

    def test_old_save_without_new_fields(self):
        d = exploded_game().to_dict()
        del d["audio"]
        with open(self.path, "w") as f:
            json.dump(d, f)
        self.assertEqual(Game.load(self.path).audio, "all")

    def test_muted_save_migrated(self):
        d = exploded_game().to_dict()
        del d["audio"]
        d["muted"] = True
        with open(self.path, "w") as f:
            json.dump(d, f)
        self.assertEqual(Game.load(self.path).audio, "off")

    def test_invalid_audio_reset(self):
        d = exploded_game().to_dict()
        d["audio"] = "fort"
        with open(self.path, "w") as f:
            json.dump(d, f)
        self.assertEqual(Game.load(self.path).audio, "all")

    def test_missing_or_corrupt(self):
        self.assertIsNone(Game.load(self.path))
        for content in ("{pas du json", "[1, 2]", "42"):
            with open(self.path, "w") as f:
                f.write(content)
            self.assertIsNone(Game.load(self.path), content)

    def test_null_wall_time(self):
        d = exploded_game().to_dict()
        d["wall_time"] = None
        with open(self.path, "w") as f:
            json.dump(d, f)
        self.assertTrue(Game.load(self.path).exploded)

    def test_default_path_follows_save_path(self):
        with mock.patch.object(saves, "SAVE_PATH", self.path):
            exploded_game().save()
            self.assertTrue(os.path.exists(self.path))
            self.assertTrue(Game.load().exploded)

    def test_old_save_seeds_isotopes_earned(self):
        g = exploded_game()
        g.isotopes = 100
        g.ptalents.update(p_cap=2, p_prod=1)
        d = g.to_dict()
        del d["stats"]["iso_earned"]
        with open(self.path, "w") as f:
            json.dump(d, f)
        self.assertEqual(Game.load(self.path).stats["iso_earned"], 100 + 20 + 150 + 2)

    def test_corrupt_pending_chest_sanitized(self):
        for bad, expected in ((["bogus", "pt"], ["pt"]), ("xyz", None), ([1, 2], None), (["bogus"], None)):
            d = exploded_game().to_dict()
            d["pending_chest"] = bad
            with open(self.path, "w") as f:
                json.dump(d, f)
            self.assertEqual(Game.load(self.path).pending_chest, expected, bad)

    def test_watched_save_reloads_with_short_timer(self):
        g = exploded_game()
        g.challenge = "watched"
        d = g.to_dict()
        del d["threats"]  # save from before threats were recorded
        with open(self.path, "w") as f:
            json.dump(d, f)
        h = Game.load(self.path)
        self.assertLessEqual(h.next_inspector - h.time, 60)

    def test_volume_saved_and_validated(self):
        g = exploded_game()
        self.assertEqual(g.volume, 100)
        self.assertFalse(g.change_volume(1))  # already at max
        self.assertTrue(g.change_volume(-1))
        g.save(self.path)
        self.assertEqual(Game.load(self.path).volume, 95)
        for bad, expected in ((-20, 100), (250, 100), ("fort", 100), (True, 100), (57, 55), (60.0, 60)):
            d = g.to_dict()
            d["volume"] = bad
            with open(self.path, "w") as f:
                json.dump(d, f)
            self.assertEqual(Game.load(self.path).volume, expected, bad)
        for _ in range(30):
            g.change_volume(-1)
        self.assertEqual(g.volume, 0)

    def test_threats_survive_reload(self):
        g = exploded_game()
        g.time += 500  # game time restarts at 0 on load: everything must be relative
        g.inspector = dict(start=g.time - 5, dur=20.0, hits=2)
        g.enemies = [dict(kind="commando", start=g.time - 3, dur=20.0, hits=4),
                     dict(kind="drone", start=g.time + 1.5, dur=15.0, hits=0)]  # not entered yet
        g.wave = 7
        g.next_inspector, g.next_enemy = g.time + 40, g.time + 70
        g.bribe_until, g.freeze_until = g.time + 100, g.time + 8
        g.save(self.path)
        h = Game.load(self.path)
        self.assertAlmostEqual(h.time - h.inspector["start"], 5)
        self.assertEqual((h.inspector["dur"], h.inspector["hits"]), (20.0, 2))
        self.assertEqual([(e["kind"], e["hits"]) for e in h.enemies], [("commando", 4), ("drone", 0)])
        self.assertAlmostEqual(h.time - h.enemies[0]["start"], 3)
        self.assertAlmostEqual(h.enemies[1]["start"] - h.time, 1.5)
        self.assertEqual(h.wave, 7)
        for attr, left in (("next_inspector", 40), ("next_enemy", 70), ("bribe_until", 100), ("freeze_until", 8)):
            self.assertAlmostEqual(getattr(h, attr) - h.time, left, msg=attr)
        self.assertTrue(h.frozen)

    def test_bad_threats_ignored(self):
        g = exploded_game()
        for bad in (None, [1], {"inspector": {"elapsed": 30, "dur": 20, "hits": 0}},
                    {"inspector": {"elapsed": 1, "dur": 20, "hits": 99}},
                    {"enemies": [{"kind": "zzz", "elapsed": 1, "dur": 10, "hits": 0}, "x", None]},
                    {"enemies": {"kind": "drone"}}, {"next_inspector": -5, "freeze": float("nan"), "bribe": 1e9},
                    {"enemies": [{"kind": "drone", "elapsed": 1, "dur": 10, "hits": True}]}):
            d = g.to_dict()
            d["threats"] = bad
            with open(self.path, "w") as f:
                json.dump(d, f)
            h = Game.load(self.path)
            self.assertIsNotNone(h, bad)
            self.assertIsNone(h.inspector, bad)
            self.assertEqual(h.enemies, [], bad)
            self.assertFalse(h.frozen, bad)
            self.assertGreater(h.next_inspector, h.time, bad)

    def test_offline_gain(self):
        g = exploded_game()
        g.ptalents["p_offline"] = 1
        g.units["soldier"] = 10
        g.apply_offline(3600)
        self.assertAlmostEqual(g.radiation, 5 * 3600 * 0.25)
        self.assertEqual(g.messages[-1][2], "good")


if __name__ == "__main__":
    unittest.main()
