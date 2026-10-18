"""Définitions statiques : unités, talents de run, talents de prestige, défis, skins."""

BASE_CAP = 100
MAX_CAP = 1_000_000

# Unités achetées en radiation. Chacune est débloquée par un talent "u_<id>"
# et boostée (×2 par rang) par le talent "m_<id>".
UNITS = [
    dict(id="soldier", name="Soldat", icon="i", cost=15, prod=0.5, proj="o", color="green",
         unlock_name="Conscription", unlock_cost=1, mult_name="Grenades à fragmentation"),
    dict(id="mortar", name="Mortier", icon="y", cost=150, prod=4, proj="*", color="yellow",
         unlock_name="Mortiers", unlock_cost=3, mult_name="Obus explosifs"),
    dict(id="artillery", name="Artilleur", icon="F", cost=1_500, prod=30, proj="°", color="orange",
         unlock_name="Artillerie lourde", unlock_cost=5, mult_name="Obus perforants"),
    dict(id="tank", name="Tank", icon="T", cost=15_000, prod=220, proj="=", color="green",
         unlock_name="Divisions blindées", unlock_cost=8, mult_name="Canons de 120"),
    dict(id="bomber", name="Bombardier", icon=">", cost=160_000, prod=1_600, proj="v", color="cyan",
         unlock_name="Escadrilles", unlock_cost=12, mult_name="Bombes au napalm"),
    dict(id="sub", name="Sous-marin nucléaire", icon="D", cost=1.8e6, prod=12_000, proj="^", color="blue",
         unlock_name="Flotte sous-marine", unlock_cost=16, mult_name="Torpilles atomiques"),
    dict(id="icbm", name="Silo ICBM", icon="!", cost=2e7, prod=90_000, proj="!", color="red",
         unlock_name="Silos ICBM", unlock_cost=20, mult_name="Têtes multiples"),
    dict(id="reactor", name="Réacteur à fusion", icon="@", cost=2.3e8, prod=700_000, proj="+", color="magenta",
         unlock_name="Réacteurs à fusion", unlock_cost=25, mult_name="Plasma confiné"),
    dict(id="blackhole", name="Trou noir de poche", icon="O", cost=2.6e9, prod=5.5e6, proj="·", color="white",
         unlock_name="Singularité", unlock_cost=30, mult_name="Horizon des événements"),
]
UNIT_BY_ID = {u["id"]: u for u in UNITS}
UNIT_COST_GROWTH = 1.15
# Paliers : chaque seuil atteint multiplie la production de l'unité.
UNIT_MILESTONES = (50, 100, 200, 300, 400, 500)
MILESTONE_MULT = 2


def _talent(id, name, branch, depth, max, cost, desc, req=()):
    """Talent de run. req : [(id du talent requis, rang minimum)]."""
    return dict(id=id, name=name, branch=branch, depth=depth, max=max, cost=cost, desc=desc, req=list(req))


def _unit_talents():
    """Pour chaque unité, dans l'ordre : le talent qui la débloque (après la précédente) et celui qui la booste."""
    talents = []
    previous = None
    for unit in UNITS:
        uid, unlock_cost = unit["id"], unit["unlock_cost"]
        unlock_req = [(f"u_{previous}", 1)] if previous else []
        talents.append(_talent(f"u_{uid}", unit["unlock_name"], "inf", 0, 1, (unlock_cost, 0),
                               f"Débloque l'unité : {unit['name']}.", unlock_req))
        talents.append(_talent(f"m_{uid}", unit["mult_name"], "inf", 1, 3, (max(2, unlock_cost // 2), 1),
                               f"Production des {unit['name']}s ×2 par rang.", [(f"u_{uid}", 1)]))
        previous = uid
    return talents


# Talents de run : coût = cost[0] + cost[1] * rang actuel, payé en PT.
TALENTS = [
    _talent("t_click", "Détonateur renforcé", "det", 0, 10, (1, 1), "+2 de puissance de détonation de base par rang."),
    _talent("t_crit", "Fission critique", "det", 1, 5, (2, 1), "+5 % de chance de détonation critique par rang.",
            [("t_click", 1)]),
    _talent("t_combo", "Réaction en chaîne", "det", 1, 2, (2, 2), "Combo max +1 par rang (×3 → ×5).",
            [("t_click", 1)]),
    _talent("t_cool", "Refroidissement", "det", 2, 3, (2, 1), "Seuil de surchauffe +3 appuis/s par rang.",
            [("t_combo", 1)]),
    _talent("t_auto", "Auto-détonateur", "det", 1, 5, (3, 2), "Appuie tout seul sur ESPACE : 1 fois/s par rang.",
            [("t_click", 2)]),
    _talent("t_shock", "Onde de choc", "det", 1, 5, (3, 2), "Chaque appui ajoute 2 % de la production/s par rang.",
            [("t_click", 3)]),
]

TALENTS += _unit_talents()

TALENTS += [
    _talent("l_global", "Propagande", "log", 0, 10, (2, 1), "Production globale +10 % par rang."),
    _talent("l_synergy", "Esprit de corps", "log", 1, 3, (4, 2), "+0,1 % de production par unité possédée, par rang.",
            [("l_global", 2)]),
    _talent("l_guard", "Gardes", "log", 1, 1, (4, 0), "Des gardes chassent automatiquement les inspecteurs de l'ONU.",
            [("l_global", 1)]),
    _talent("l_discount", "Marché noir", "log", 0, 5, (2, 1), "Coût des unités -4 % par rang."),
    _talent("l_autobuy", "Quartier-maître", "log", 1, 3, (5, 3),
            "Achète automatiquement les unités (3 types de plus par rang).", [("l_discount", 1)]),
    _talent("l_radar", "Radar", "log", 0, 3, (2, 1), "Ravitaillements +30 % plus fréquents et avions plus lents."),
    _talent("l_enrich", "Enrichissement", "log", 0, 3, (3, 1), "La masse critique se charge 25 % plus vite par rang."),
]
TALENT_BY_ID = {t["id"]: t for t in TALENTS}
BRANCHES = [("det", "DÉTONATION"), ("inf", "INFANTERIE"), ("log", "LOGISTIQUE")]
# Talents conservés au reset avec "Automates persistants".
KEPT_TALENTS = ("t_auto", "l_autobuy", "l_discount")


def _perk(id, name, max, desc, base=1, growth=1.5, costs=None):
    """Talent permanent (prestige ⚛ ou supernova ✦)."""
    return dict(id=id, name=name, max=max, desc=desc, base=base, growth=growth, costs=costs)


# Talents de prestige, payés en isotopes. Coût = round(base * growth^rang) ou costs[rang].
PTALENTS = [
    _perk("p_cap", "Expansion du cœur", 4, "Niveau max ×10 (100 → 1 000 000).", costs=[20, 150, 1_200, 9_000]),
    _perk("p_prod", "Uranium appauvri", 25, "Production ×1,5 par rang (multiplicatif).", base=2, growth=1.45),
    _perk("p_click", "Doigt nucléaire", 15, "Puissance de détonation ×2 par rang.", base=1, growth=1.5),
    _perk("p_pt", "Mémoire génétique", 10, "Commence chaque run avec +5 PT par rang.", base=3, growth=1.6),
    _perk("p_vets", "Vétérans", 5, "Commence avec la Conscription et 10 soldats par rang.", base=4, growth=2),
    _perk("p_keep", "Automates persistants", 1,
          "Auto-détonateur, Quartier-maître et Marché noir sont conservés au reset.", costs=[15]),
    _perk("p_iso", "Centrifugeuses", 10, "+25 % d'isotopes gagnés par rang.", base=5, growth=1.7),
    _perk("p_drop", "Contrebandiers", 3, "Ravitaillements +50 % plus efficaces par rang.", base=5, growth=2),
    _perk("p_offline", "Sommeil radioactif", 4, "Progression hors-ligne : 25 % de la production par rang (2 h max).",
          base=4, growth=2),
    _perk("p_dna", "ADN instable", 2, "+1 mutation proposée au choix par rang.", base=20, growth=3),
]
PTALENT_BY_ID = {p["id"]: p for p in PTALENTS}

# Talents de supernova, payés en étoiles (✦). Remis à zéro jamais.
STALENTS = [
    _perk("s_prod", "Fusion stellaire", 10, "Production ×3 par rang.", base=1, growth=2),
    _perk("s_iso", "Nucléosynthèse", 10, "Isotopes gagnés ×2 par rang.", base=1, growth=2),
    _perk("s_core", "Cœur dense", 4, "Expansion du cœur coûte 20 % de moins par rang.", costs=[2, 5, 10, 20]),
    _perk("s_crit", "Pulsar", 3, "La masse critique se charge 2× plus vite par rang.", base=3, growth=2),
    _perk("s_auto", "Chaîne de montage", 1, "Achète automatiquement les talents de run.", costs=[5]),
]
STALENT_BY_ID = {p["id"]: p for p in STALENTS}
SUPERNOVA_DIVISOR = 500

CHALLENGES = [
    dict(id="pacifist", name="Pacifiste", target=60, time=None, skin="toxic",
         desc="Aucune unité autorisée. Atteins le niveau 60.",
         reward="Détonation ×3 permanente + skin Vert radioactif"),
    dict(id="onearm", name="Manchot", target=100, time=None, skin="rainbow",
         desc="Ni ESPACE ni auto-détonateur (3 soldats offerts). Atteins le niveau 100.",
         reward="Production ×2 permanente + skin Arc-en-ciel"),
    dict(id="race", name="Course", target=100, time=600, skin="mario",
         desc="Atteins le niveau 100 en moins de 10 minutes.",
         reward="Isotopes ×1,5 permanent + skin Champignon de Mario"),
    dict(id="inflation", name="Inflation", target=100, time=None, skin="gold",
         desc="Les unités coûtent 4 fois plus cher. Atteins le niveau 100.",
         reward="Unités -10 % permanent + skin Or"),
    dict(id="blind", name="Brouillard de guerre", target=100, time=None, skin="ghost",
         desc="Niveau, radiation et production sont masqués. Atteins le niveau 100.",
         reward="+1 mutation proposée permanente + skin Fantôme"),
    dict(id="watched", name="Sous surveillance", target=100, time=None, skin="neon",
         desc="Un inspecteur toutes les 30 à 60 s, gardes et brouillard inefficaces. Atteins le niveau 100.",
         reward="Inspecteurs chassés ×3 de butin + skin Néon"),
    dict(id="ascete", name="Ascète", target=100, time=None, skin="ice",
         desc="Seuls les talents qui débloquent des unités sont autorisés. Atteins le niveau 100.",
         reward="+10 PT au début de chaque run + skin Glace"),
    dict(id="hothead", name="Tête brûlée", target=100, time=None, skin="lava",
         desc="Surchauffe dès 6 appuis/s, pendant 6 s, même avec les Doigts d'amiante. Atteins le niveau 100.",
         reward="Seuil de surchauffe +5 appuis/s permanent + skin Lave"),
    dict(id="invasion", name="Invasion", target=100, time=None, skin="camo",
         desc="Un ennemi toutes les 15 à 30 s, tous types dès le début. Atteins le niveau 100.",
         reward="Ennemis 50 % plus lents permanent + skin Camouflage"),
    dict(id="sterile", name="Stérile", target=100, time=None, skin="crystal",
         desc="Aucune mutation ne t'est proposée. Atteins le niveau 100.",
         reward="Masse critique 50 % plus rapide permanente + skin Cristal"),
    dict(id="unlucky", name="Chat noir", target=100, time=None, skin="obsidian",
         desc="Ni critiques, ni masse critique, ni ravitaillements. Atteins le niveau 100.",
         reward="Chance de critique +10 % permanente + skin Obsidienne"),
    dict(id="nanar", name="Nanarland", target=100, time=None, skin="carton",
         desc="Effets spéciaux en carton : 1 détonation sur 3 fait pschitt, budget de série Z (production ÷2). "
              "Atteins le niveau 100.",
         reward="Critiques ×15 au lieu de ×10 permanent + skin Carton-pâte"),
]
CHALLENGE_BY_ID = {c["id"]: c for c in CHALLENGES}

SKINS = {
    "classic": "Classique (feu)",
    "toxic": "Vert radioactif",
    "rainbow": "Arc-en-ciel",
    "mario": "Champignon de Mario",
    "plasma": "Plasma (5 prestiges)",
    "gold": "Or massif",
    "ghost": "Fantôme",
    "neon": "Néon",
    "ice": "Glace",
    "lava": "Lave",
    "camo": "Camouflage",
    "crystal": "Cristal",
    "obsidian": "Obsidienne",
    "carton": "Carton-pâte",
}
