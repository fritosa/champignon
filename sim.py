"""Simulation sans affichage pour vérifier l'équilibrage.

Usage : python3 champignon/sim.py [nombre_de_runs] [appuis_par_seconde]
"""

import random
import sys

import data
import events
from game import Game, fmt

DT = 0.1
MAX_RUN_SECONDS = 3600
LEVEL_MARKS = (10, 25, 50, 100, 1_000, 10_000, 100_000, 1_000_000)
PRESTIGE_ORDER = ["p_cap", "p_keep", "p_prod", "p_click", "p_pt", "p_vets", "p_iso", "p_drop", "p_offline", "p_dna"]


def buy_units(g):
    for _ in range(100):
        affordable = [u["id"] for u in data.UNITS if g.can_buy_unit(u["id"])]
        if not affordable:
            return
        g.buy_unit(max(affordable, key=lambda u: g.unit_prod(u) / g.unit_cost(u)))


def play_qte(g, rng):
    """Lance la masse critique et appuie à `miss` du centre : 40 % dans le vert, 40 % dans le jaune, 20 % raté."""
    events.crit_press(g)
    qte = g.qte
    r = rng.random()
    miss = 0 if r < 0.4 else (0.1 if r < 0.8 else 0.4)
    pos = qte["center"] - miss if qte["center"] >= miss else qte["center"] + miss
    qte["t"] = pos / qte["speed"]
    events.crit_press(g)


def bot(g, rng, cps):
    if not g.exploded:
        g.click()
    g._sim_presses = getattr(g, "_sim_presses", 0) + DT * cps
    while g._sim_presses >= 1:
        g._sim_presses -= 1
        g.click()
    g.auto_talents()
    buy_units(g)
    if g.pending_mutation:
        events.choose_mutation(g, 0)
    if g.pending_chest:
        events.choose_chest(g, 0)
    if g.crit_gauge >= 1 and not g.qte:
        play_qte(g, rng)
    if g.plane and rng.random() < 0.1:
        events.catch(g, g.plane["letter"])
    if g.inspector or g.enemies:
        events.chase(g)


def run_once(g, rng, cps):
    marks = {}
    while g.run_time < MAX_RUN_SECONDS and g.level < g.cap:
        bot(g, rng, cps)
        g.tick(DT)
        for m in LEVEL_MARKS:
            if g.level >= m and m not in marks:
                marks[m] = g.run_time
    return marks


def spend_isotopes(g):
    """Achète les talents de prestige, l'expansion du cœur d'abord. Renvoie les identifiants achetés."""
    bought = []
    if g.buy_ptalent("p_cap"):
        bought.append("p_cap")
    # On garde de quoi viser la prochaine expansion du cœur.
    reserve = g.ptalent_cost("p_cap") * 0.6 if g.pr("p_cap") < 4 else 0
    for pid in PRESTIGE_ORDER[1:] * 40:
        if g.isotopes - g.ptalent_cost(pid) >= reserve and g.buy_ptalent(pid):
            bought.append(pid)
    return bought


def main():
    runs = int(sys.argv[1]) if len(sys.argv) > 1 else 8
    cps = float(sys.argv[2]) if len(sys.argv) > 2 else 6
    rng = random.Random(1)
    g = Game(rng=random.Random(2))
    for r in range(runs):
        marks = run_once(g, rng, cps)
        txt = "  ".join(f"L{fmt(m)}:{t / 60:.1f}m" for m, t in marks.items())
        print(f"run {r + 1:2d} cap {fmt(g.cap):>5}  niveau {fmt(g.level):>5}  {txt}")
        print(f"        prod {fmt(g.production())}/s  détonation {fmt(g.click_value())}  PT restants {g.pt}  "
              f"unités {sum(g.units.values())}  mutations {len(g.mutations)}  succès {len(g.achievements)}")
        if not g.can_prestige():
            print("  -> bloqué, fin de la simulation")
            break
        if g.can_supernova():
            stars = g.supernova_gain()
            g.supernova()
            while any(g.buy_stalent(s["id"]) for s in data.STALENTS):
                pass
            print(f"        SUPERNOVA n°{g.supernova_count} +{stars} ✦ -> talents {g.stalents}  reste {g.stars} ✦")
            continue
        gain = g.prestige_gain()
        g.prestige()
        bought = spend_isotopes(g)
        print(f"        prestige +{gain} ⚛ -> achats {', '.join(sorted(set(bought)))}  reste {g.isotopes}")


if __name__ == "__main__":
    main()
