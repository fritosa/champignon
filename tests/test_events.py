import unittest

import events

from .helpers import FixedRandom, exploded_game


class DropTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()
        self.g.plane = dict(letter="a", start=self.g.time, dur=10)

    def test_wrong_letter(self):
        self.assertFalse(events.catch(self.g, "b"))
        self.assertIsNotNone(self.g.plane)

    def test_catch_opens_chest(self):
        self.assertTrue(events.catch(self.g, "a"))
        self.assertIsNone(self.g.plane)
        self.assertEqual(self.g.stats["drops"], 1)
        self.assertIn("drop", self.g.sfx)
        self.assertEqual(len(self.g.pending_chest), 3)
        self.assertEqual(len(set(self.g.pending_chest)), 3)


class ChestTest(unittest.TestCase):
    def open_with(self, kind, g=None):
        g = g or exploded_game()
        g.pending_chest = [kind]
        events.choose_chest(g, 0)
        self.assertIsNone(g.pending_chest)
        self.assertIn("chest", g.sfx)
        self.assertEqual(g.messages[-1][2:], ("loot", "COFFRE"))
        return g

    def test_frenzies(self):
        self.assertEqual(self.open_with("frenzy").buff_mult("prod"), 7)
        self.assertEqual(self.open_with("clickfrenzy").buff_mult("click"), 20)

    def test_lump_and_pt(self):
        g = exploded_game()
        g.units["soldier"] = 10
        expected = max(g.production() * 120, g.click_value(False) * 100)
        self.open_with("lump", g)
        self.assertAlmostEqual(g.radiation, expected)
        g = exploded_game()
        g.level, g.pt = 50, 0
        self.open_with("pt", g)
        self.assertEqual(g.pt, 2)

    def test_gauge(self):
        self.assertEqual(self.open_with("gauge").crit_gauge, 1.0)

    def test_army(self):
        g = exploded_game()
        g.units.update(soldier=45, mortar=3)
        self.open_with("army", g)
        self.assertEqual((g.units["soldier"], g.units["mortar"]), (49, 4))
        g = exploded_game()
        g.units["soldier"] = 48
        self.open_with("army", g)
        self.assertIn("milestone", g.sfx)  # 48 → 52 crosses the 50 milestone

    def test_shield(self):
        g = exploded_game()
        g.level = 10
        g.inspector = dict(start=g.time, dur=20, hits=0)
        self.open_with("shield", g)
        self.assertIsNone(g.inspector)
        self.assertGreaterEqual(g.next_inspector - g.time, 300)
        self.assertEqual(g.stats["inspectors"], 0)

    def test_pool_filters(self):
        g = exploded_game()
        g.crit_gauge, g.challenge = 1.0, "watched"
        allowed = [k for k in events.CHEST_REWARDS if events._chest_allowed(g, k)]
        self.assertNotIn("gauge", allowed)
        self.assertNotIn("army", allowed)  # no units
        self.assertNotIn("shield", allowed)

    def test_texts_and_drop_power(self):
        g = exploded_game()
        for kind in events.CHEST_REWARDS:
            name, desc = events.chest_text(g, kind)
            self.assertTrue(name and desc)
        g.ptalents["p_drop"] = 2
        self.assertIn("60 s", events.chest_text(g, "frenzy")[1])

    def test_choice_rechecked_when_opened(self):
        g = exploded_game()
        g.pending_chest = ["gauge", "frenzy", "pt"]
        g.crit_gauge = 1.0  # filled up on its own meanwhile
        self.assertFalse(events.choose_chest(g, 0))
        self.assertEqual(g.pending_chest, ["gauge", "frenzy", "pt"])
        self.assertTrue(events.choose_chest(g, 1))

    def test_unknown_kind_does_nothing(self):
        g = exploded_game()
        g.pending_chest = ["bogus"]
        before = g.next_inspector
        self.assertFalse(events.choose_chest(g, 0))
        self.assertEqual(g.next_inspector, before)  # no Bribes applied by default
        self.assertNotIn("chest", g.sfx)

    def test_no_second_plane_while_chest_pending(self):
        g = exploded_game()
        g.level, g.next_plane, g.next_inspector = 3, g.time, 1e9
        g.pending_chest = ["pt", "lump", "frenzy"]
        events.tick(g, 0.1)
        self.assertIsNone(g.plane)
        g.plane = dict(letter="a", start=g.time, dur=10)
        self.assertFalse(events.catch(g, "a"))
        self.assertEqual(g.pending_chest, ["pt", "lump", "frenzy"])

    def test_army_never_in_pacifist(self):
        g = exploded_game()
        g.challenge, g.units["soldier"] = "pacifist", 10
        self.assertFalse(events._chest_allowed(g, "army"))

    def test_shield_useless_with_fog(self):
        g = exploded_game()
        g.level = 10
        g.mutations.append("fog")
        self.assertFalse(events._chest_allowed(g, "shield"))

    def test_blind_hides_pt(self):
        g = exploded_game()
        g.level, g.challenge = 100, "blind"
        self.assertIn("???", events.chest_text(g, "pt")[1])

    def test_bad_index_ignored(self):
        g = exploded_game()
        g.pending_chest = ["frenzy"]
        events.choose_chest(g, 3)
        self.assertEqual(g.pending_chest, ["frenzy"])

    def test_pending_chest_saved(self):
        import os
        import tempfile
        from game import Game
        g = exploded_game()
        g.pending_chest = ["pt", "gauge", "lump"]
        with tempfile.TemporaryDirectory() as d:
            g.save(os.path.join(d, "s.json"))
            self.assertEqual(Game.load(os.path.join(d, "s.json")).pending_chest, ["pt", "gauge", "lump"])

    def test_plane_spawns_and_leaves(self):
        g = exploded_game()
        g.level, g.next_plane, g.next_inspector = 3, g.time, 1e9
        events.tick(g, 0.1)
        self.assertIsNotNone(g.plane)
        self.assertIn("plane", g.sfx)
        self.assertIn(g.plane["letter"], events.DROP_LETTERS)
        g.time += g.plane["dur"] + 1
        events.tick(g, 0.1)
        self.assertIsNone(g.plane)

    def test_drop_letters_avoid_controls(self):
        for key in "fjkmqx0":
            self.assertNotIn(key, events.DROP_LETTERS)


class InspectorTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()
        self.g.next_plane = 1e9

    def test_spawn(self):
        g = self.g
        g.level, g.next_inspector = 10, g.time
        events.tick(g, 0.1)
        self.assertIsNotNone(g.inspector)
        self.assertIn("alarm", g.sfx)

    def test_fog_prevents_spawn(self):
        g = self.g
        g.level, g.next_inspector = 10, g.time
        g.mutations.append("fog")
        events.tick(g, 0.1)
        self.assertIsNone(g.inspector)

    def test_chase(self):
        g = self.g
        g.inspector = dict(start=g.time, dur=20.0, hits=0)
        for _ in range(events.INSPECTOR_HITS):
            events.chase(g)
        self.assertIsNone(g.inspector)
        self.assertEqual(g.stats["inspectors"], 1)
        self.assertIn("hit", g.sfx)
        self.assertIn("chased", g.sfx)

    def test_freeze(self):
        g = self.g
        g.inspector = dict(start=g.time - 25, dur=20.0, hits=0)
        events.tick(g, 0.1)
        self.assertTrue(g.frozen)
        self.assertIn("freeze", g.sfx)
        self.assertEqual(g.messages[-1][2], "bad")

    def test_guards(self):
        g = self.g
        g.talents["l_guard"] = 1
        g.inspector = dict(start=g.time - 10, dur=20.0, hits=0)
        events.tick(g, 0.1)
        self.assertIsNone(g.inspector)
        self.assertFalse(g.frozen)


class NewChallengesTest(unittest.TestCase):
    def test_inflation(self):
        g = exploded_game()
        base = g.unit_cost("soldier")
        g.challenge = "inflation"
        self.assertAlmostEqual(g.unit_cost("soldier"), base * 4)
        g.challenge = None
        g.challenges_done.add("inflation")
        self.assertAlmostEqual(g.unit_cost("soldier"), base * 0.9)

    def test_blind_reward_extra_mutation(self):
        g = exploded_game()
        g.challenges_done.add("blind")
        events.offer_mutation(g)
        self.assertEqual(len(g.pending_mutation), 4)

    def test_watched_frequent_inspectors_ignore_fog_and_guards(self):
        g = exploded_game()
        g.challenge, g.level, g.next_plane = "watched", 10, 1e9
        g.mutations.append("fog")
        g.talents["l_guard"] = 1
        g.schedule_inspector()
        self.assertTrue(30 <= g.next_inspector - g.time <= 60)
        g.next_inspector = g.time
        events.tick(g, 0.1)
        self.assertIsNotNone(g.inspector)
        g.inspector["start"] = g.time - 10  # past the 30 % where the guards would step in
        events.tick(g, 0.1)
        self.assertIsNotNone(g.inspector)

    def test_watched_start_of_run(self):
        g = exploded_game()
        g.challenge = "watched"
        g.reset_run()
        self.assertLessEqual(g.next_inspector - g.time, 60)

    def test_watched_reward(self):
        g = exploded_game()
        g.inspector = dict(start=g.time, dur=20.0, hits=events.INSPECTOR_HITS - 1)
        events.chase(g)
        normal = g.radiation
        h = exploded_game()
        h.challenges_done.add("watched")
        h.inspector = dict(start=h.time, dur=20.0, hits=events.INSPECTOR_HITS - 1)
        events.chase(h)
        self.assertAlmostEqual(h.radiation, normal * 3)

    def test_every_challenge_skin_has_palette(self):
        import art
        import data
        for c in data.CHALLENGES:
            self.assertIn(c["skin"], data.SKINS)
            self.assertTrue(c["skin"] in art.PALETTES or c["skin"] in ("rainbow", "mario"), c["skin"])

    def test_master_achievement(self):
        import data
        g = exploded_game()
        g.challenges_done = {c["id"] for c in data.CHALLENGES}
        g.check_achievements()
        self.assertIn("challenges_all", g.achievements)


class CriticalMassTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()
        self.g.crit_gauge = 1

    def start(self, offset):
        events.crit_press(self.g)
        q = self.g.qte
        q["t"] = (q["center"] + offset) / q["speed"]
        events.crit_press(self.g)

    def test_start(self):
        events.crit_press(self.g)
        self.assertIsNotNone(self.g.qte)
        self.assertIn("charge", self.g.sfx)

    def test_not_ready(self):
        self.g.crit_gauge = 0.5
        events.crit_press(self.g)
        self.assertIsNone(self.g.qte)

    def test_perfect(self):
        self.start(0)
        self.assertEqual(self.g.stats["perfects"], 1)
        self.assertIn("perfect", self.g.sfx)
        self.assertEqual(self.g.crit_gauge, 0)

    def test_partial(self):
        self.start(0.1)
        self.assertEqual(self.g.qte_result[1], "yellow")
        self.assertIn("partial", self.g.sfx)

    def test_timeout(self):
        events.crit_press(self.g)
        events.tick(self.g, 7)
        self.assertIsNone(self.g.qte)
        self.assertEqual(self.g.qte_result[1], "red")
        self.assertIn("fizzle", self.g.sfx)

    def test_cursor_bounces(self):
        q = dict(t=1.5, speed=1.0)
        self.assertAlmostEqual(events.qte_pos(q), 0.5)


class MutationTest(unittest.TestCase):
    def test_milestones(self):
        self.assertEqual([events.mutation_milestone(i) for i in range(6)], [25, 50, 75, 100, 200, 400])

    def test_offer_and_choose(self):
        g = exploded_game()
        events.offer_mutation(g)
        self.assertEqual(len(g.pending_mutation), 3)
        self.assertIn("mutation", g.sfx)
        mid = g.pending_mutation[1]
        events.choose_mutation(g, 1)
        self.assertEqual(g.mutations, [mid])
        self.assertIsNone(g.pending_mutation)
        self.assertEqual(g.messages[-1][2], "good")

    def test_queue(self):
        g = exploded_game()
        g.mut_queue = 1
        events.offer_mutation(g)
        events.choose_mutation(g, 0)
        self.assertIsNotNone(g.pending_mutation)
        self.assertEqual(g.mut_queue, 0)
        self.assertNotIn(g.mutations[0], g.pending_mutation)

    def test_level_triggers_mutation(self):
        g = exploded_game()
        g.level = 24
        g.gain(need_for(g))
        self.assertEqual(g.level, 25)
        self.assertIsNotNone(g.pending_mutation)


def need_for(g):
    from game import need
    return need(g.level)


class SynergyTest(unittest.TestCase):
    def pick(self, g, mid):
        g.pending_mutation = [mid]
        events.choose_mutation(g, 0)

    def test_each_mutation_in_exactly_one_pair(self):
        used = [m for s in events.SYNERGIES for m in s[2]]
        self.assertEqual(sorted(used), sorted(m[0] for m in events.MUTATIONS))

    def test_discovery(self):
        g = exploded_game()
        self.pick(g, "spores")
        self.assertFalse(g.has_syn("proliferation"))
        self.assertIsNotNone(events.synergy_completed_by(g, "mitosis"))
        self.pick(g, "mitosis")
        self.assertTrue(g.has_syn("proliferation"))
        self.assertIn("proliferation", g.synergies_found)
        self.assertIn("synergy", g.sfx)
        self.assertEqual(g.messages[-1][2:], ("epic", "SYNERGIE"))
        self.assertIn("découverte", g.messages[-1][0])
        g.check_achievements()
        self.assertIn("synergy1", g.achievements)

    def test_rediscovery_not_new(self):
        g = exploded_game()
        g.synergies_found.add("storm")
        self.pick(g, "acid")
        self.pick(g, "fallout")
        self.assertNotIn("découverte", g.messages[-1][0])

    def test_effects(self):
        g = exploded_game()
        g.units["soldier"] = 100
        g.mutations = ["spores"]
        base = g.unit_prod("soldier")
        g.mutations.append("mitosis")
        self.assertAlmostEqual(g.unit_prod("soldier"), base * 3 / 2)  # ×(1+2) instead of ×(1+1)
        g = exploded_game()
        g.mutations = ["acid"]
        halved = g.click_value(False)
        g.mutations.append("fallout")
        self.assertAlmostEqual(g.click_value(False), halved * 2)
        g = exploded_game()
        g.mutations = ["twin"]
        g.units["soldier"] = 10
        p = g.production()
        g.mutations.append("gamble")
        self.assertAlmostEqual(g.production(), p * 3 * 1.5)

    def test_chain_charges_gauge(self):
        g = exploded_game(FixedRandom(0.0))
        g.mutations = ["unstable", "glow"]
        g.click()
        self.assertAlmostEqual(g.crit_gauge, 0.03)

    def test_berserker_slow_decay(self):
        g = exploded_game()
        g.mutations = ["berserk", "asbestos"]
        g.combo, g.last_click = 2.0, -10
        g.tick(1.0)
        self.assertAlmostEqual(g.combo, 1.6)

    def test_smuggling_four_choices(self):
        g = exploded_game()
        g.mutations = ["magnet", "fog"]
        g.units["soldier"] = 1
        g.plane = dict(letter="a", start=g.time, dur=10)
        events.catch(g, "a")
        self.assertEqual(len(g.pending_chest), 4)


class NewsTest(unittest.TestCase):
    def test_inspector_news(self):
        g = exploded_game()
        g.inspector = dict(start=0, dur=20, hits=0)
        self.assertEqual(events.pick_news(g), events.NEWS_INSPECTOR)

    def test_news_unlocked_by_level(self):
        g = exploded_game()
        allowed = {t for lvl, t in events.NEWS if lvl <= g.level}
        for _ in range(20):
            self.assertIn(events.pick_news(g), allowed)


if __name__ == "__main__":
    unittest.main()


class EnemyTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()
        self.g.next_inspector = self.g.next_plane = 1e9

    def spawn(self, kind, level=100):
        g = self.g
        g.level = level
        g.enemies = [dict(kind=kind, start=g.time, dur=events.enemy_duration(g, kind), hits=0)]
        return g

    def test_spawn_respects_level(self):
        g = self.g
        g.level, g.next_enemy = 5, g.time
        g.tick(0.1)
        self.assertEqual(g.enemies, [])  # too early: no enemy available, reschedule
        self.assertGreater(g.next_enemy, g.time)
        g.level, g.next_enemy = 15, g.time
        g.tick(0.1)
        self.assertEqual([e["kind"] for e in g.enemies], ["saboteur"])

    def test_chase_inspector_first_then_enemy(self):
        g = self.spawn("saboteur")
        g.inspector = dict(start=g.time, dur=20.0, hits=0)
        for _ in range(events.INSPECTOR_HITS):
            events.chase(g)
        self.assertIsNone(g.inspector)
        self.assertEqual(g.enemies[0]["hits"], 0)
        for _ in range(events.ENEMIES["saboteur"]["hits"]):
            events.chase(g)
        self.assertEqual(g.enemies, [])
        self.assertEqual(g.stats["enemies"], 1)
        self.assertGreater(g.radiation, 0)  # loot

    def test_saboteur_steals(self):
        g = self.spawn("saboteur")
        g.radiation = 1000
        g.tick(events.ENEMIES["saboteur"]["dur"] + 0.1)
        self.assertEqual(g.enemies, [])
        self.assertLess(g.radiation, 1000)
        self.assertGreaterEqual(g.radiation, 850)

    def test_hippies_halve_production(self):
        g = self.spawn("hippies")
        g.talents["u_soldier"], g.units["soldier"] = 1, 10
        with_them = g.production()
        g.enemies = []
        self.assertAlmostEqual(with_them, g.production() / 2)

    def test_commando_destroys_units(self):
        g = self.spawn("commando")
        g.units["soldier"], g.units["tank"] = 50, 5
        g.tick(events.ENEMIES["commando"]["dur"] + 0.1)
        self.assertEqual((g.units["soldier"], g.units["tank"]), (45, 5))

    def test_drone_calls_inspector(self):
        g = self.spawn("drone")
        g.crit_gauge = 0.5
        g.tick(events.ENEMIES["drone"]["dur"] + 0.1)
        self.assertEqual(g.crit_gauge, 0)
        self.assertLessEqual(g.next_inspector, g.time + 5)

    def test_drone_respects_bribe(self):
        g = self.spawn("drone")
        g.pending_chest = ["shield"]
        events.choose_chest(g, 0)
        before = g.next_inspector
        g.enemies = [dict(kind="drone", start=g.time, dur=1.0, hits=0)]
        g.tick(1.1)
        self.assertEqual(g.next_inspector, before)

    def test_enemy_delays(self):
        g = self.g
        g.schedule_enemy(first=True)
        self.assertTrue(60 <= g.next_enemy - g.time <= 120)
        g.schedule_enemy()
        self.assertTrue(90 <= g.next_enemy - g.time <= 180)

    def test_invasion(self):
        g = self.g
        g.challenge = "invasion"
        g.schedule_enemy()
        self.assertTrue(15 <= g.next_enemy - g.time <= 30)
        g.level, g.next_enemy = 1, g.time
        g.tick(0.1)
        self.assertTrue(g.enemies)  # every type from the start
        g.challenges_done.add("invasion")
        self.assertEqual(events.enemy_duration(g, "saboteur"), events.ENEMIES["saboteur"]["dur"] * 1.5)

    def test_invasion_waves(self):
        g = self.g
        g.challenge, g.level = "invasion", 50
        self.assertEqual([events.wave_size(w) for w in range(1, 7)], [1, 2, 2, 3, 3, 3])
        sizes = []
        for _ in range(5):
            g.enemies, g.next_enemy = [], g.time
            g.tick(0.01)
            sizes.append(len(g.enemies))
        self.assertEqual(sizes, [1, 2, 2, 3, 3])
        self.assertEqual(g.wave, 5)
        starts = [e["start"] - g.enemies[0]["start"] for e in g.enemies]
        self.assertEqual(starts, [0, events.WAVE_GAP, 2 * events.WAVE_GAP])  # staggered entries

    def test_chase_hits_most_advanced_and_next_wave_waits(self):
        g = self.g
        g.level = 50
        late = dict(kind="saboteur", start=g.time, dur=12.0, hits=0)
        early = dict(kind="saboteur", start=g.time - 6, dur=12.0, hits=0)
        g.enemies = [late, early]
        for _ in range(3):
            events.chase(g)
        self.assertEqual(g.enemies, [late])  # the most advanced one fell first
        g.next_enemy = g.time  # deadline passed, but an enemy is still there: no new enemy
        g.tick(0.01)
        self.assertEqual(len(g.enemies), 1)
        for _ in range(3):
            events.chase(g)
        self.assertEqual(g.enemies, [])
        self.assertGreater(g.next_enemy, g.time)  # the next one is scheduled once the field is clear

    def test_two_hippies_do_not_stack(self):
        g = self.spawn("hippies")
        g.talents["u_soldier"], g.units["soldier"] = 1, 10
        one = g.production()
        g.enemies.append(dict(g.enemies[0]))
        self.assertAlmostEqual(g.production(), one)

    def test_every_enemy_has_art_and_news(self):
        import art
        for kind in events.ENEMIES:
            self.assertIn(kind, art.ENEMY_ART)
            self.assertIn(kind, events.NEWS_ENEMY)


class ExtraChallengesTest(unittest.TestCase):
    def setUp(self):
        self.g = exploded_game()

    def test_ascete_only_unit_unlocks(self):
        g = self.g
        g.challenge, g.pt = "ascete", 50
        self.assertFalse(g.buy_talent("t_click"))
        self.assertTrue(g.buy_talent("u_soldier"))
        g.auto_talents()  # doesn't loop forever on forbidden talents
        self.assertEqual(g.tr("t_click"), 0)
        g.challenges_done.add("ascete")
        g.challenge = None
        g.reset_run()
        self.assertEqual(g.pt, 10)

    def test_hothead(self):
        g = self.g
        g.challenge = "hothead"
        g.mutations.append("asbestos")
        for _ in range(7):
            g.click()
        self.assertTrue(g.overheated)
        self.assertAlmostEqual(g.overheat_until - g.time, 6)
        g.challenge = None
        g.challenges_done.add("hothead")
        self.assertEqual(g.overheat_threshold(), 17)

    def test_sterile(self):
        g = self.g
        g.challenge = "sterile"
        events.offer_mutation(g)
        self.assertIsNone(g.pending_mutation)
        g.challenge, g.crit_gauge = None, 0
        g.challenges_done.add("sterile")
        g.next_plane = g.next_inspector = g.next_enemy = 1e9
        g.tick(1)
        self.assertAlmostEqual(g.crit_gauge, 1.5 / 90)

    def test_unlucky(self):
        g = self.g
        g.challenge, g.level, g.next_plane = "unlucky", 10, g.time
        g.talents["t_crit"] = 5
        self.assertEqual(g.crit_chance(), 0)
        g.tick(1)
        self.assertEqual(g.crit_gauge, 0)
        self.assertIsNone(g.plane)
        g.challenge = None
        g.challenges_done.add("unlucky")
        self.assertAlmostEqual(g.crit_chance(), 0.35)

    def test_nanar_fizzles_and_halves_production(self):
        g = exploded_game(FixedRandom(0.2))  # 0.2 < 1/3: fizzle
        g.units["soldier"] = 10
        full = g.production()
        g.challenge = "nanar"
        self.assertAlmostEqual(g.production(), full / 2)
        before = g.radiation + g.progress
        g.click()
        self.assertEqual(g.radiation + g.progress, before)
        self.assertIn("fizzle", g.sfx)
        g.sfx.clear()
        g.click(manual=False)  # the auto-detonator misfires too, but silently
        self.assertNotIn("fizzle", g.sfx)
        g.rng = FixedRandom(0.5)
        g.click()
        self.assertGreater(g.radiation + g.progress, before)

    def test_nanar_reward_and_news(self):
        g = self.g
        self.assertEqual(g.crit_mult(), 10)
        g.challenges_done.add("nanar")
        self.assertEqual(g.crit_mult(), 15)
        g.challenge = "nanar"
        self.assertTrue(set(events.NEWS_NANAR) & {events.pick_news(g) for _ in range(50)})
