"""Game state and pure logic (no dependency on curses)."""

import math
import random
import time
from collections import deque

import data
import events
import saves

SUFFIXES = ["", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc"]
OFFLINE_MAX = 2 * 3600
OFFLINE_MIN = 60
OFFLINE_SHARE = 0.25
MAX_BULK = 1000  # beyond that, 1.15**n overflows
AUDIO_MODES = ("all", "sfx", "off")  # music + effects, effects only, muted
NANAR_FIZZLE = 1 / 3
VOLUME_STEP = 5  # in %
COMBO_STEP = 0.08
COMBO_GRACE = 0.4
CLICK_PITCHES = 8
PT_BONUS_EVERY = 10
PT_LEVEL_GROWTH = 1.04  # beyond data.BASE_CAP, one PT every +4% of level


def fmt(n):
    if n == math.inf:
        return "∞"
    if n < 100:
        return f"{n:.1f}".rstrip("0").rstrip(".") if n != int(n) else str(int(n))
    if n < 1000:
        return str(int(n))
    thousands = int(math.log10(n) // 3)
    if thousands < len(SUFFIXES):
        return f"{n / 10 ** (3 * thousands):.2f}{SUFFIXES[thousands]}"
    return f"{n:.2e}"


REPORT_NUMBERS = ("gain", "time", "level", "cap", "earned", "clicks", "best_click", "crits", "perfects",
                  "inspectors", "units", "number")
REPORT_LISTS = ("mutations", "synergies", "chests")


def _is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _is_str_list(v):
    return isinstance(v, list) and all(isinstance(x, str) for x in v)


def _valid_report(r):
    """Game summary usable by the UI (otherwise it is dropped on load)."""
    return (isinstance(r, dict) and r.get("kind") in ("prestige", "supernova")
            and all(_is_num(r.get(k)) for k in REPORT_NUMBERS) and all(_is_str_list(r.get(k)) for k in REPORT_LISTS))


def _clean_run_stats(rs):
    """Stats of the current run: defaults for anything missing or of the wrong type."""
    clean = dict(clicks=0, best_click=0.0, crits=0, earned=0.0, chests=[], inspectors=0, perfects=0)
    if isinstance(rs, dict):
        for k, default in clean.items():
            v = rs.get(k)
            if (k == "chests" and _is_str_list(v)) or (k != "chests" and _is_num(v)):
                clean[k] = v
    return clean


THREAT_MAX = 3600  # maximum duration read back from a save (seconds)


def _is_delay(v):
    return _is_num(v) and 0 <= v <= THREAT_MAX


def _load_walker(d, max_hits):
    """Saved inspector or enemy {elapsed, dur, hits} → game dict without `start` (None if invalid).
    elapsed may be slightly negative: an enemy of a wave that hasn't entered yet."""
    if not isinstance(d, dict):
        return None
    elapsed, dur, hits = d.get("elapsed"), d.get("dur"), d.get("hits")
    if not (_is_num(elapsed) and -10 <= elapsed < dur and _is_delay(dur) and dur > 0
            and isinstance(hits, int) and not isinstance(hits, bool) and 0 <= hits < max_hits):
        return None
    return dict(elapsed=elapsed, dur=dur, hits=hits)


def _rank_cost(talent, rank):
    """Cost of the next rank of a prestige or supernova talent (fixed list or geometric growth)."""
    return talent["costs"][rank] if talent["costs"] else round(talent["base"] * talent["growth"] ** rank)


def need(level):
    """Radiation needed to go from level `level` to the next one."""
    return 25 * level ** 2.4


class Game:
    def fmt(self, n):
        """Number formatted for messages, hidden during the Brouillard de guerre challenge."""
        return "???" if self.challenge == "blind" else fmt(n)

    def __init__(self, rng=None):
        self.rng = rng or random.Random()
        self.time = 0.0
        # Permanent (survives prestige)
        self.isotopes = 0
        self.ptalents = {}
        self.prestige_count = 0
        self.stars = 0
        self.stalents = {}
        self.supernova_count = 0
        self.achievements = set()
        self.synergies_found = set()
        self.last_run = None  # summary of the last finished run (end screen, Infos tab)
        self.report_ready = False  # the UI shows the end screen then resets it to False
        self.pending_story = None  # (challenge id, "start" or "end"): the UI shows the story then resets it to None
        self.challenges_done = set()
        self.skins = {"classic"}
        self.skin = "classic"
        self.audio = "all"
        self.volume = 100
        self.challenge = None
        self.stats = dict(clicks=0, drops=0, perfects=0, inspectors=0, frozen=0, overheats=0, mutations=0,
                          max_combo=False, best_prestige_time=None, max_level=0, total_time=0.0,
                          iso_earned=0, enemies=0)
        self.messages = deque(maxlen=8)
        self.qte_result = None
        self.flash_until = 0.0
        self.sfx = deque(maxlen=16)  # sounds to play, drained by the UI
        self.reset_run()

    # --- Run ---------------------------------------------------------------------

    def _kept_talents(self):
        """Talents kept by "p_keep" (none on the first call, from __init__: ptalents is still empty)."""
        if not self.pr("p_keep"):
            return {}
        return {t: self.talents.get(t, 0) for t in data.KEPT_TALENTS}

    def reset_run(self):
        kept = self._kept_talents()
        self.exploded = False
        self.level = 0
        self.progress = 0.0
        self.next_pt_level = 0
        self.radiation = 0.0
        self.pt = 5 * self.pr("p_pt") + (10 if "ascete" in self.challenges_done else 0)
        self.units = {u["id"]: 0 for u in data.UNITS}
        self.talents = {t: r for t, r in kept.items() if r and self.talent_allowed(t)}
        if self.pr("p_vets"):
            self.talents["u_soldier"] = 1
            self.units["soldier"] = 10 * self.pr("p_vets")
        if self.challenge == "onearm":
            self.talents["u_soldier"] = 1
            self.units["soldier"] = max(3, self.units["soldier"])
        self.mutations = []
        self.pending_mutation = None
        self.pending_chest = None
        self.mut_index = 0
        self.mut_queue = 0
        self.run_time = 0.0
        self.run_stats = _clean_run_stats(None)
        self.combo = 0.0
        self.last_click = -10.0
        self.click_times = deque()
        self.overheat_until = 0.0
        self.last_crit = -10.0
        self.buffs = []
        self.freeze_until = 0.0
        self.bribe_until = 0.0  # bribes: no inspector before this time
        self.crit_gauge = 0.0
        self.qte = None
        self.plane = None
        self.inspector = None
        self.enemies = []
        self.wave = 0  # number of the last wave (Invasion challenge)
        self.schedule_enemy(first=True)
        self.next_plane = self.time + self.rng.uniform(40, 90)
        self.schedule_inspector(first=True)
        self.auto_acc = 0.0
        self.autobuy_acc = 0.0
        self.ach_acc = 0.0

    def schedule_inspector(self, first=False):
        if self.challenge == "watched":
            delay = self.rng.uniform(30, 60)
        else:
            delay = (self.rng.uniform(120, 240) if first else self.rng.uniform(150, 300)) / events.threat_pressure(self)
        self.next_inspector = self.time + delay

    def schedule_enemy(self, first=False):
        if self.challenge == "invasion":
            delay = self.rng.uniform(15, 30)
        else:
            delay = (self.rng.uniform(60, 120) if first else self.rng.uniform(90, 180)) / events.threat_pressure(self)
        self.next_enemy = self.time + delay

    def log(self, msg, kind="info", title=None):
        """kind: info, good, bad, loot, star or epic (banner color in the UI);
        title: short title shown in big letters above the banner."""
        if self.messages and self.messages[-1][0] == msg:
            self.messages.pop()
        self.messages.append((msg, self.time, kind, title))

    def tr(self, tid):
        return self.talents.get(tid, 0)

    def pr(self, pid):
        return self.ptalents.get(pid, 0)

    def sr(self, sid):
        return self.stalents.get(sid, 0)

    def has_mut(self, mid):
        return mid in self.mutations

    def has_syn(self, sid):
        return all(m in self.mutations for m in events.SYNERGY_BY_ID[sid][2])

    def synergies(self):
        return [s for s in events.SYNERGIES if self.has_syn(s[0])]

    @property
    def cap(self):
        return min(data.MAX_CAP, data.BASE_CAP * 10 ** self.pr("p_cap"))

    @property
    def frozen(self):
        return self.time < self.freeze_until

    @property
    def overheated(self):
        return self.time < self.overheat_until

    # --- Multipliers -------------------------------------------------------------

    def buff_mult(self, kind):
        m = 1.0
        for b in self.buffs:
            if b["kind"] == kind:
                m *= b["mult"]
        return m

    def add_buff(self, kind, mult, duration, name):
        self.buffs.append(dict(kind=kind, mult=mult, until=self.time + duration, name=name))

    def max_combo(self):
        return 3 + self.tr("t_combo") + (2 if self.has_mut("berserk") else 0)

    def combo_mult(self):
        return 1 + self.combo

    def overheat_threshold(self):
        if self.challenge == "hothead":
            return 6
        return 12 + 3 * self.tr("t_cool") + (5 if "hothead" in self.challenges_done else 0)

    def crit_mult(self):
        return 15 if "nanar" in self.challenges_done else 10

    def crit_chance(self):
        if self.challenge == "unlucky":
            return 0.0
        return (0.05 * self.tr("t_crit") + (0.15 if self.has_mut("glow") else 0)
                + (0.1 if "unlucky" in self.challenges_done else 0))

    def global_mult(self):
        m = (1 + 0.1 * self.tr("l_global")) * 1.5 ** self.pr("p_prod") * 3 ** self.sr("s_prod")
        m *= 1 + 0.02 * len(self.achievements)
        m *= 1 + 0.001 * self.tr("l_synergy") * sum(self.units.values())
        if self.has_mut("fallout"):
            m *= 1.5
        if self.has_mut("gamble"):
            m *= 3
        if self.has_syn("twins"):
            m *= 1.5
        if "onearm" in self.challenges_done:
            m *= 2
        return m * self.buff_mult("prod")

    def unit_milestones(self, uid):
        return sum(1 for m in data.UNIT_MILESTONES if self.units[uid] >= m)

    def next_milestone(self, uid):
        return next((m for m in data.UNIT_MILESTONES if self.units[uid] < m), None)

    def unit_prod(self, uid):
        """Production of one unit of this type (multipliers included, freeze excluded)."""
        p = data.UNIT_BY_ID[uid]["prod"] * 2 ** self.tr(f"m_{uid}") * data.MILESTONE_MULT ** self.unit_milestones(uid)
        if self.has_mut("spores"):
            p *= 1 + (0.02 if self.has_syn("proliferation") else 0.01) * self.units["soldier"]
        return p * self.global_mult()

    def production(self):
        if self.frozen or not self.exploded:
            return 0.0
        p = sum(n * self.unit_prod(uid) for uid, n in self.units.items() if n)
        if self.challenge == "nanar":
            p *= 0.5
        protesters = any(e["kind"] == "hippies" for e in self.enemies)
        return p / events.protest_divisor(self) if protesters else p

    def click_value(self, manual=True):
        m = 2 ** self.pr("p_click") * self.buff_mult("click")
        if self.has_mut("twin"):
            m *= 2
        if self.has_mut("acid") and not self.has_syn("storm"):
            m *= 0.5
        if "pacifist" in self.challenges_done:
            m *= 3
        v = (1 + 2 * self.tr("t_click")) * m
        if manual:
            v *= self.combo_mult()
        return v + self.production() * 0.02 * self.tr("t_shock")

    # --- Actions -----------------------------------------------------------------

    def explode(self):
        self.exploded = True
        self.level = 1
        self.progress = 0.0
        self.flash_until = self.time + 0.8
        self.sfx.append("boom")
        self.log("BOOM ! Le champignon est né.", "epic", title="BOOM !")

    def click(self, manual=True):
        if not self.exploded:
            self.explode()
            return
        if manual:
            if self.challenge == "onearm":
                self.sfx.append("nope")
                self.log("Défi Manchot : ESPACE est interdit !", "bad", title="NON !")
                return
            if self.overheated:
                self.sfx.append("jam")  # press rejected, but audible: otherwise spamming seems to cut the sound
                return
            if self._press_overheats():
                self._overheat()
                return
            combo_top = self.max_combo() - 1
            self.combo = min(combo_top, self.combo + COMBO_STEP)
            if self.combo >= combo_top:
                self.stats["max_combo"] = True
            self.last_click = self.time
            self.stats["clicks"] += 1
            pitch = min(CLICK_PITCHES - 1, int(self.combo / max(1, combo_top) * CLICK_PITCHES))
            self.sfx.append(f"click{pitch}")
        if self.challenge == "nanar" and self.rng.random() < NANAR_FIZZLE:
            if manual:
                self.run_stats["clicks"] += 1
                self.sfx.append("fizzle")
            return
        v = self.click_value(manual)
        if self.rng.random() < self.crit_chance():
            v *= self.crit_mult()
            self.last_crit = self.time
            if manual:
                self.run_stats["crits"] += 1
            if self.has_syn("chain") and not self.qte:
                self.crit_gauge = min(1.0, self.crit_gauge + 0.03)
            if manual:
                self.sfx.append("crit")
        if manual:
            self.run_stats["clicks"] += 1
            self.run_stats["best_click"] = max(self.run_stats["best_click"], v)
        self.gain(v)

    def _press_overheats(self):
        """Records the press; true if it exceeds the press threshold over the last second."""
        self.click_times.append(self.time)
        while self.click_times and self.click_times[0] < self.time - 1:
            self.click_times.popleft()
        immune = self.has_mut("asbestos") and self.challenge != "hothead"
        return len(self.click_times) > self.overheat_threshold() and not immune

    def _overheat(self):
        self.overheat_until = self.time + (6 if self.challenge == "hothead" else 3)
        self.combo = 0
        self.click_times.clear()
        self.stats["overheats"] += 1
        self.sfx.append("overheat")
        self.log("SURCHAUFFE ! Doucement sur le détonateur...", "bad", title="CHAUD !")

    def gain(self, amount):
        self.radiation += amount
        self.run_stats["earned"] += amount
        if self.level >= self.cap or not self.exploded:
            return
        self.progress += amount
        while self.progress >= need(self.level) and self.level < self.cap:
            self.progress -= need(self.level)
            self.level += 1
            self.on_level_up()
        if self.level >= self.cap:
            self.progress = 0.0

    def on_level_up(self):
        lvl = self.level
        below_base_cap = lvl <= data.BASE_CAP
        if below_base_cap or lvl >= self.next_pt_level:
            self.sfx.append("level")
            round_level = below_base_cap and lvl % PT_BONUS_EVERY == 0
            self.pt += 1 + (5 if round_level else 0) + (1 if self.has_mut("mitosis") else 0)
            self.next_pt_level = math.ceil(lvl * PT_LEVEL_GROWTH)
        self.stats["max_level"] = max(self.stats["max_level"], lvl)
        while lvl >= events.mutation_milestone(self.mut_index):
            self.mut_index += 1
            if self.pending_mutation:
                self.mut_queue += 1
            else:
                events.offer_mutation(self)
        if lvl == self.cap:
            self.log("NIVEAU MAX ! Le prestige est disponible (onglet 3).", "epic", title="MAX !")
        ch = data.CHALLENGE_BY_ID.get(self.challenge)
        if ch and lvl >= ch["target"]:
            self._complete_challenge(ch)

    def _complete_challenge(self, ch):
        self.challenges_done.add(ch["id"])
        self.skins.add(ch["skin"])
        self.challenge = None
        self.pending_story = (ch["id"], "end")
        if ch["id"] == "watched":
            self.schedule_inspector()
        self.sfx.append("fanfare")
        self.log(f"Défi {ch['name']} réussi ! {ch['reward']}", "star", title="DÉFI OK")

    def unit_unlocked(self, uid):
        return self.tr(f"u_{uid}") > 0

    def unit_cost(self, uid, count=None):
        u = data.UNIT_BY_ID[uid]
        n = self.units[uid] if count is None else count
        try:
            c = u["cost"] * data.UNIT_COST_GROWTH ** n * (1 - 0.04 * self.tr("l_discount"))
        except OverflowError:
            return math.inf
        if self.challenge == "inflation":
            c *= 4
        if "inflation" in self.challenges_done:
            c *= 0.9
        return c * (0.7 if self.has_mut("acid") else 1)

    def can_buy_unit(self, uid):
        return (self.challenge != "pacifist" and self.unit_unlocked(uid)
                and self.radiation >= self.unit_cost(uid))

    def buy_unit(self, uid):
        return self.buy_unit_n(uid, 1) == 1

    def add_units(self, uid, k):
        """Adds k units and announces each milestone crossed."""
        before = self.units[uid]
        self.units[uid] += k
        for m in data.UNIT_MILESTONES:
            if before < m <= self.units[uid]:
                self.sfx.append("milestone")
                self.log(f"Palier : {m} {data.UNIT_BY_ID[uid]['name']}s, production ×2 !", "good", title="PALIER")

    def unit_cost_n(self, uid, k):
        """Total cost of the next k units."""
        n = self.units[uid]
        return sum(self.unit_cost(uid, n + i) for i in range(k))

    def affordable(self, uid):
        """Number of units affordable at once with the current radiation (capped at MAX_BULK)."""
        if not self.can_buy_unit(uid):
            return 0
        k, total, n = 0, 0.0, self.units[uid]
        while k < MAX_BULK and total + self.unit_cost(uid, n + k) <= self.radiation:
            total += self.unit_cost(uid, n + k)
            k += 1
        return k

    def buy_unit_n(self, uid, k):
        """Buys exactly k units (all or nothing); returns the number bought."""
        total = self.unit_cost_n(uid, k)
        if k <= 0 or not self.can_buy_unit(uid) or self.radiation < total:
            return 0
        self.radiation = max(0.0, self.radiation - total)  # in one go: no rounding drift along the way
        self.add_units(uid, k)
        return k

    def buy_unit_max(self, uid):
        return self.buy_unit_n(uid, self.affordable(uid))

    def talent_cost(self, tid):
        t = data.TALENT_BY_ID[tid]
        return t["cost"][0] + t["cost"][1] * self.tr(tid)

    def talent_allowed(self, tid):
        """Ascète challenge: unit unlocks only."""
        return self.challenge != "ascete" or tid.startswith("u_")

    def talent_available(self, tid):
        t = data.TALENT_BY_ID[tid]
        return self.tr(tid) < t["max"] and all(self.tr(r) >= n for r, n in t["req"])

    def buy_talent(self, tid):
        if not self.talent_available(tid) or not self.talent_allowed(tid) or self.pt < self.talent_cost(tid):
            return False
        self.pt -= self.talent_cost(tid)
        self.talents[tid] = self.tr(tid) + 1
        return True

    def ptalent_cost(self, pid):
        p = data.PTALENT_BY_ID[pid]
        r = self.pr(pid)
        if r >= p["max"]:
            return math.inf
        if pid == "p_cap":
            return round(p["costs"][r] * (1 - 0.2 * self.sr("s_core")))
        return _rank_cost(p, r)

    def buy_ptalent(self, pid):
        p = data.PTALENT_BY_ID[pid]
        if self.pr(pid) >= p["max"] or self.isotopes < self.ptalent_cost(pid):
            return False
        self.isotopes -= self.ptalent_cost(pid)
        self.ptalents[pid] = self.pr(pid) + 1
        if pid == "p_cap":
            self.log(f"Le cœur s'étend : niveau max {fmt(self.cap)} !", "epic", title="CŒUR +")
        return True

    # --- Prestige ----------------------------------------------------------------

    @property
    def prestige_unlocked(self):
        return self.prestige_count > 0 or self.level >= self.cap

    def can_prestige(self):
        return self.level >= self.cap

    def prestige_gain(self):
        g = 10 * (self.level / 100) ** 0.6 * (1 + 0.25 * self.pr("p_iso"))
        if "race" in self.challenges_done:
            g *= 1.5
        return int(g * 2 ** self.sr("s_iso"))

    def make_report(self, kind, gain):
        """Summary of the run that is ending (prestige or supernova)."""
        rs = self.run_stats
        self.last_run = dict(kind=kind, gain=gain, time=self.run_time, level=self.level, cap=self.cap,
                             earned=rs["earned"], clicks=rs["clicks"], best_click=rs["best_click"], crits=rs["crits"],
                             perfects=rs["perfects"], inspectors=rs["inspectors"], chests=list(rs["chests"]),
                             units=sum(self.units.values()),
                             mutations=[events.MUTATION_BY_ID[m][1] for m in self.mutations],
                             synergies=[s[1] for s in self.synergies()], number=self.prestige_count)
        self.report_ready = True

    def prestige(self, challenge=None):
        """Normal prestige (at max level) or start of a challenge (always possible after a 1st prestige)."""
        if self.can_prestige():
            gain = self.prestige_gain()
            self.isotopes += gain
            self.stats["iso_earned"] += gain
            self.prestige_count += 1
            self.make_report("prestige", gain)
            best = self.stats["best_prestige_time"]
            if best is None or self.run_time < best:
                self.stats["best_prestige_time"] = self.run_time
            if self.prestige_count >= 5:
                self.skins.add("plasma")
            self.sfx.append("prestige")
            self.log(f"PRESTIGE ! +{gain} ⚛. Tout recommence...", "epic", title="PRESTIGE")
        elif challenge is None or self.prestige_count == 0:
            return False
        self.challenge = challenge
        if challenge:
            self.log(f"Défi {data.CHALLENGE_BY_ID[challenge]['name']} lancé !", "info", "DÉFI")
            self.pending_story = (challenge, "start")
        self.reset_run()
        return True

    # --- Supernova (2nd prestige tier) -----------------------------------------

    @property
    def supernova_unlocked(self):
        return self.supernova_count > 0 or self.pr("p_cap") >= data.PTALENT_BY_ID["p_cap"]["max"]

    def can_supernova(self):
        return self.pr("p_cap") >= data.PTALENT_BY_ID["p_cap"]["max"] and self.level >= self.cap

    def supernova_gain(self):
        """Stars: square root of the isotopes earned since the last supernova (current prestige included)."""
        earned = self.stats["iso_earned"] + (self.prestige_gain() if self.can_prestige() else 0)
        return max(1, int(math.sqrt(earned / data.SUPERNOVA_DIVISOR)))

    def supernova(self):
        """Resets isotopes and the prestige tree; keeps achievements, skins, challenges and the prestige count."""
        if not self.can_supernova():
            return False
        gain = self.supernova_gain()
        self.make_report("supernova", gain)
        self.stars += gain
        self.supernova_count += 1
        self.isotopes = 0
        self.ptalents = {}
        self.stats["iso_earned"] = 0
        self.challenge = None
        self.sfx.append("supernova")
        self.log(f"SUPERNOVA ! +{gain} ✦. L'univers recommence...", "epic", title="SUPERNOVA")
        self.reset_run()
        return True

    def isotopes_invested(self):
        """Isotopes owned + spent in the prestige tree (estimate of what was earned)."""
        total = self.isotopes
        for p in data.PTALENTS:
            for r in range(self.pr(p["id"])):
                total += _rank_cost(p, r)
        return total

    def stalent_cost(self, sid):
        p = data.STALENT_BY_ID[sid]
        r = self.sr(sid)
        if r >= p["max"]:
            return math.inf
        return _rank_cost(p, r)

    def buy_stalent(self, sid):
        if self.stars < self.stalent_cost(sid):
            return False
        self.stars -= self.stalent_cost(sid)
        self.stalents[sid] = self.sr(sid) + 1
        return True

    def auto_talents(self):
        """Spends PT: unit unlocks first, otherwise the cheapest talent."""
        while True:
            avail = [t["id"] for t in data.TALENTS if self.talent_available(t["id"]) and self.talent_allowed(t["id"])
                     and self.pt >= self.talent_cost(t["id"])]
            if not avail:
                return
            unlocks = [t for t in avail if t.startswith("u_")]
            self.buy_talent(unlocks[0] if unlocks else min(avail, key=self.talent_cost))

    def change_volume(self, step):
        """+1 / -1 step; returns True if the volume changed."""
        old = self.volume
        self.volume = max(0, min(100, old + step * VOLUME_STEP))
        return self.volume != old

    def abandon_challenge(self):
        if self.challenge:
            self.log("Défi abandonné.", "info", "ABANDON")
            was_watched = self.challenge == "watched"
            self.challenge = None
            if was_watched:
                self.schedule_inspector()

    # --- Loop --------------------------------------------------------------------

    def tick(self, dt):
        self.time += dt
        self.stats["total_time"] += dt
        if not self.exploded:
            return
        self.run_time += dt

        if self.time - self.last_click > COMBO_GRACE:
            decay_rate = 0.4 if self.has_syn("berserker") else 1.2
            self.combo = max(0.0, self.combo - decay_rate * dt)

        self.gain(self.production() * dt)
        self._auto_detonate(dt)

        self.autobuy_acc += dt
        if self.autobuy_acc >= 1:
            self.autobuy_acc = 0
            self.autobuy()
            if self.sr("s_auto"):
                self.auto_talents()

        self.buffs = [b for b in self.buffs if b["until"] > self.time]
        self._fail_slow_challenge()

        events.tick(self, dt)

        self.ach_acc += dt
        if self.ach_acc >= 1:
            self.ach_acc = 0
            self.check_achievements()

    def _auto_detonate(self, dt):
        if self.tr("t_auto") and self.challenge != "onearm":
            self.auto_acc += dt * self.tr("t_auto")
            while self.auto_acc >= 1:
                self.auto_acc -= 1
                self.click(manual=False)

    def _fail_slow_challenge(self):
        ch = data.CHALLENGE_BY_ID.get(self.challenge)
        if ch and ch["time"] and self.run_time > ch["time"]:
            self.challenge = None
            self.log(f"Défi {ch['name']} échoué : trop lent !", "bad", title="ÉCHEC")

    def autobuy(self):
        rank = self.tr("l_autobuy")
        if not rank:
            return
        allowed = [u["id"] for u in data.UNITS[:3 * rank]]
        for _ in range(50):
            choices = [u for u in allowed if self.can_buy_unit(u)]
            if not choices:
                break
            self.buy_unit(min(choices, key=self.unit_cost))

    def check_achievements(self):
        for aid, name, _desc, check in events.ACHIEVEMENTS:
            if aid not in self.achievements and check(self):
                self.achievements.add(aid)
                self.sfx.append("achievement")
                self.log(f"★ Succès : {name} (+2 % de production)", "star", title="SUCCÈS")

    # --- Saving ------------------------------------------------------------------

    SAVED = ("isotopes", "ptalents", "prestige_count", "skin", "challenge", "stats", "exploded", "level",
             "progress", "radiation", "pt", "units", "talents", "mutations", "pending_mutation", "pending_chest", "mut_index",
             "mut_queue", "run_time", "crit_gauge", "next_pt_level", "audio", "volume", "stars", "stalents",
             "supernova_count", "run_stats", "last_run")

    def threats_dict(self):
        """Current threats and deadlines, in relative time (g.time restarts from 0 on load)."""
        def walker(w):
            return dict(elapsed=self.time - w["start"], dur=w["dur"], hits=w["hits"])

        def left(t):
            return max(0.0, t - self.time)
        return dict(inspector=walker(self.inspector) if self.inspector else None,
                    enemies=[dict(walker(e), kind=e["kind"]) for e in self.enemies], wave=self.wave,
                    next_inspector=left(self.next_inspector), next_enemy=left(self.next_enemy),
                    bribe=left(self.bribe_until), freeze=left(self.freeze_until))

    def load_threats(self, t):
        """Restores threats_dict; each invalid value is ignored (the one from reset_run is kept)."""
        if not isinstance(t, dict):
            return
        for key, attr in (("next_inspector", "next_inspector"), ("next_enemy", "next_enemy"),
                          ("bribe", "bribe_until"), ("freeze", "freeze_until")):
            if _is_delay(t.get(key)):
                setattr(self, attr, self.time + t[key])
        inspector = _load_walker(t.get("inspector"), events.inspector_hits(self))
        if inspector:
            self.inspector = dict(start=self.time - inspector.pop("elapsed"), **inspector)
        enemies = t.get("enemies")
        for e in enemies if isinstance(enemies, list) else []:
            kind = e.get("kind") if isinstance(e, dict) else None
            w = _load_walker(e, events.enemy_hits(self, kind)) if kind in events.ENEMIES else None
            if w and len(self.enemies) < events.WAVE_MAX:
                self.enemies.append(dict(kind=kind, start=self.time - w.pop("elapsed"), **w))
        wave = t.get("wave")
        if isinstance(wave, int) and not isinstance(wave, bool) and 0 <= wave <= 10_000:
            self.wave = wave

    def to_dict(self):
        d = {k: getattr(self, k) for k in self.SAVED}
        d["threats"] = self.threats_dict()
        d.update(achievements=sorted(self.achievements), challenges_done=sorted(self.challenges_done),
                 synergies_found=sorted(self.synergies_found),
                 skins=sorted(self.skins), wall_time=time.time(), version=saves.SAVE_VERSION)
        return d

    def save(self, path=None):
        saves.write(path or saves.SAVE_PATH, self.to_dict())

    @classmethod
    def load(cls, path=None):
        d = saves.read(path or saves.SAVE_PATH)
        if d is None or not saves.supported(d):
            return None
        g = cls()
        for k in cls.SAVED:
            if k in d:
                v = d[k]
                if isinstance(getattr(g, k), dict):
                    if isinstance(v, dict):
                        getattr(g, k).update(v)
                else:
                    setattr(g, k, v)
        if "audio" not in d and d.get("muted"):
            g.audio = "off"  # saves from before the music
        if g.audio not in AUDIO_MODES:
            g.audio = "all"
        if not (_is_num(g.volume) and 0 <= g.volume <= 100):
            g.volume = 100
        g.volume = int(g.volume) // VOLUME_STEP * VOLUME_STEP
        g.achievements = set(d.get("achievements", []))
        g.challenges_done = set(d.get("challenges_done", []))
        g.synergies_found = set(d.get("synergies_found", [])) & set(events.SYNERGY_BY_ID)
        if not _valid_report(g.last_run):
            g.last_run = None
        g.run_stats = _clean_run_stats(g.run_stats)
        g.mutations = [m for m in g.mutations if m in events.MUTATION_BY_ID] if isinstance(g.mutations, list) else []
        offered = g.pending_mutation if isinstance(g.pending_mutation, list) else []
        g.pending_mutation = [m for m in offered if m in events.MUTATION_BY_ID] or None
        g.skins = set(d.get("skins", ["classic"]))
        chest = g.pending_chest
        if not isinstance(chest, list) or not all(isinstance(k, str) for k in chest):
            chest = []
        g.pending_chest = [k for k in chest if k in events.CHEST_REWARDS] or None
        if g.challenge == "watched" and not g.inspector:
            g.schedule_inspector(first=True)  # reset_run ran before the challenge was loaded
        if g.challenge == "invasion":
            g.schedule_enemy()
        g.load_threats(d.get("threats"))  # after the default deadlines: the save wins
        if "iso_earned" not in d.get("stats", {}):
            g.stats["iso_earned"] = g.isotopes_invested()  # saves from before the supernova
        wall = d.get("wall_time")
        g.apply_offline(time.time() - wall if isinstance(wall, (int, float)) else 0)
        return g

    def apply_offline(self, elapsed):
        rank = self.pr("p_offline")
        if not rank or elapsed < OFFLINE_MIN or not self.exploded:
            return
        secs = min(elapsed, OFFLINE_MAX)
        amount = self.production() * secs * OFFLINE_SHARE * rank
        if amount > 0:
            self.gain(amount)
            self.log(f"Pendant ton absence ({int(secs // 60)} min) : +{self.fmt(amount)} ☢", "good", title="RETOUR")
