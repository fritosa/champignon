"""Random events, critical mass, mutations, achievements and news (pure logic)."""

import math

import data

# Letters usable to catch a supply drop (no control keys).
DROP_LETTERS = "abcdeghilnoprstuvwyz"

# (id, name, effect)
MUTATIONS = [
    ("spores", "Spores radioactives", "+1 % de production par soldat."),
    ("twin", "Champignon jumeau", "Détonation ×2 (et un deuxième chapeau !)."),
    ("acid", "Pluie acide", "Unités -30 % de coût, mais détonation ÷2."),
    ("unstable", "Cœur instable", "La masse critique se charge 2× plus vite."),
    ("fallout", "Retombées massives", "Production ×1,5."),
    ("magnet", "Aimant à colis", "Ravitaillements 2× plus fréquents."),
    ("berserk", "Rage atomique", "Combo max +2."),
    ("fog", "Brouillard radioactif", "Les inspecteurs ne trouvent plus le champignon."),
    ("mitosis", "Mitose", "+1 PT supplémentaire par niveau."),
    ("asbestos", "Doigts d'amiante", "Plus jamais de surchauffe."),
    ("glow", "Lueur verte", "+15 % de chance de détonation critique."),
    ("gamble", "Roulette russe", "Production ×3, mais les inspecteurs gèlent 2× plus longtemps."),
]
MUTATION_BY_ID = {m[0]: m for m in MUTATIONS}

# (id, name, (mutation, mutation), effect): two specific mutations unlock a hidden bonus.
# Each mutation belongs to only one pair.
SYNERGIES = [
    ("proliferation", "Prolifération", ("spores", "mitosis"), "Spores : +2 % de production par soldat au lieu de 1 %."),
    ("chain", "Fission en cascade", ("unstable", "glow"), "Chaque détonation critique charge 3 % de la masse critique."),
    ("storm", "Orage acide", ("acid", "fallout"), "La pluie acide ne divise plus la détonation."),
    ("berserker", "Berserker", ("berserk", "asbestos"), "Le combo retombe 3 fois moins vite."),
    ("smuggling", "Réseau de contrebande", ("magnet", "fog"), "Les coffres proposent 4 choix au lieu de 3."),
    ("twins", "Jumeaux maudits", ("twin", "gamble"), "Production ×1,5 supplémentaire."),
]
SYNERGY_BY_ID = {s[0]: s for s in SYNERGIES}


def synergy_completed_by(g, mid):
    """Synergy that mutation mid would complete with the mutations already owned (or None)."""
    for syn in SYNERGIES:
        pair = syn[2]
        if mid in pair:
            partner = pair[1] if mid == pair[0] else pair[0]
            if partner in g.mutations:
                return syn
    return None


_FIRST_MUTATION_LEVELS = (25, 50, 75, 100)


def mutation_milestone(i):
    """Levels that trigger a mutation: 25, 50, 75, 100, then 200, 400, 800..."""
    if i < len(_FIRST_MUTATION_LEVELS):
        return _FIRST_MUTATION_LEVELS[i]
    return 100 * 2 ** (i - 3)


def _unit_count(g):
    return sum(g.units.values())


# (id, name, displayed condition, test on the game)
ACHIEVEMENTS = [
    ("boom", "Premier boom", "Déclenche l'explosion.", lambda g: g.exploded),
    ("lvl10", "Ça chauffe", "Niveau 10.", lambda g: g.stats["max_level"] >= 10),
    ("lvl25", "Petit nuage", "Niveau 25.", lambda g: g.stats["max_level"] >= 25),
    ("lvl50", "Ça se voit de loin", "Niveau 50.", lambda g: g.stats["max_level"] >= 50),
    ("lvl100", "Masse maximale", "Niveau 100.", lambda g: g.stats["max_level"] >= 100),
    ("lvl1k", "Stratosphère", "Niveau 1 000.", lambda g: g.stats["max_level"] >= 1_000),
    ("lvl10k", "Orbite basse", "Niveau 10 000.", lambda g: g.stats["max_level"] >= 10_000),
    ("lvl100k", "Visible depuis Mars", "Niveau 100 000.", lambda g: g.stats["max_level"] >= 100_000),
    ("lvl1m", "Le champignon ultime", "Niveau 1 000 000.", lambda g: g.stats["max_level"] >= 1_000_000),
    ("clicks100", "Index musclé", "100 appuis sur ESPACE.", lambda g: g.stats["clicks"] >= 100),
    ("clicks1k", "Tendinite", "1 000 appuis sur ESPACE.", lambda g: g.stats["clicks"] >= 1_000),
    ("clicks10k", "Barre d'espace usée", "10 000 appuis sur ESPACE.", lambda g: g.stats["clicks"] >= 10_000),
    ("soldiers50", "Section", "50 soldats.", lambda g: g.units["soldier"] >= 50),
    ("soldiers100", "Compagnie", "100 soldats.", lambda g: g.units["soldier"] >= 100),
    ("tanks50", "Division blindée", "50 tanks.", lambda g: g.units["tank"] >= 50),
    ("army", "Armée complète", "Au moins une unité de chaque type.", lambda g: all(g.units.values())),
    ("units500", "Mobilisation générale", "500 unités au total.", lambda g: _unit_count(g) >= 500),
    ("prod1k", "Mille par seconde", "1 000 ☢/s.", lambda g: g.production() >= 1e3),
    ("prod1m", "Mégatonne", "1 M ☢/s.", lambda g: g.production() >= 1e6),
    ("prod1b", "Gigatonne", "1 B ☢/s.", lambda g: g.production() >= 1e9),
    ("rich", "Réserve stratégique", "1 T ☢ en stock.", lambda g: g.radiation >= 1e12),
    ("combo", "Réaction en chaîne", "Atteins le combo maximum.", lambda g: g.stats["max_combo"]),
    ("overheat", "Doigts brûlés", "Provoque une surchauffe.", lambda g: g.stats["overheats"] >= 1),
    ("drop1", "Colis reçu", "Attrape un ravitaillement.", lambda g: g.stats["drops"] >= 1),
    ("drop25", "Contrebandier", "Attrape 25 ravitaillements.", lambda g: g.stats["drops"] >= 25),
    ("perfect", "Timing parfait", "Masse critique en zone verte.", lambda g: g.stats["perfects"] >= 1),
    ("perfect10", "Maître de la fission", "10 masses critiques parfaites.", lambda g: g.stats["perfects"] >= 10),
    ("inspector", "Circulez !", "Chasse un inspecteur de l'ONU.", lambda g: g.stats["inspectors"] >= 1),
    ("inspector25", "Persona non grata", "Chasse 25 inspecteurs.", lambda g: g.stats["inspectors"] >= 25),
    ("enemy1", "Légitime défense", "Repousse un ennemi (saboteur, drone...).", lambda g: g.stats["enemies"] >= 1),
    ("enemy25", "Forteresse", "Repousse 25 ennemis.", lambda g: g.stats["enemies"] >= 25),
    ("frozen", "Visite surprise", "Laisse un inspecteur geler ta production.", lambda g: g.stats["frozen"] >= 1),
    ("mutant", "Mutant", "Choisis 10 mutations.", lambda g: g.stats["mutations"] >= 10),
    ("prestige1", "Renaissance", "Premier prestige.", lambda g: g.prestige_count >= 1),
    ("prestige10", "Cycle éternel", "10 prestiges.", lambda g: g.prestige_count >= 10),
    ("supernova", "Supernova", "Déclenche une supernova.", lambda g: g.supernova_count >= 1),
    ("synergy1", "Alchimiste", "Découvre une synergie de mutations.", lambda g: len(g.synergies_found) >= 1),
    ("synergy_all", "Généticien fou", "Découvre toutes les synergies.",
     lambda g: len(g.synergies_found) >= len(SYNERGIES)),
    ("speed", "Éclair", "Prestige en moins de 15 minutes.",
     lambda g: g.stats["best_prestige_time"] is not None and g.stats["best_prestige_time"] < 900),
    ("challenges", "Vétéran des défis", "Réussis 3 défis.", lambda g: len(g.challenges_done) >= 3),
    ("challenges_all", "Maître des défis", "Réussis tous les défis.",
     lambda g: len(g.challenges_done) >= len(data.CHALLENGES)),
]
ACHIEVEMENT_BY_ID = {a[0]: a for a in ACHIEVEMENTS}

# (minimum level, headline). The banner picks at random among the unlocked ones.
NEWS = [
    (0, "Un militaire distrait a laissé une bombe dans le jardin. Rien d'inquiétant."),
    (0, "Les autorités recommandent de ne PAS appuyer sur la barre d'espace."),
    (1, "Le village voisin signale une légère hausse des températures."),
    (1, "Un agriculteur affirme que ses tomates brillent dans le noir."),
    (1, "Le maire inaugure un rond-point. Personne ne remarque le cratère juste à côté."),
    (2, "Un chat du voisinage a désormais trois queues. Il refuse de commenter."),
    (3, "Le ministère dément toute explosion et conseille de fermer les volets, par pure coquetterie."),
    (5, "Un réalisateur de série Z repère les lieux : « Pas besoin d'effets spéciaux. »"),
    (5, "Les soldats demandent des grenades plus grosses."),
    (10, "Les pigeons du quartier évitent désormais la zone."),
    (10, "La météo annonce un nuage en forme de champignon. « C'est normal », assure le ministère."),
    (12, "Tournage annulé pour « L'Attaque des tomates fluo » : les tomates ont attaqué pour de vrai."),
    (15, "Le compteur Geiger de la mairie est parti en vacances. Sans prévenir."),
    (20, "Un influenceur fait un selfie devant le champignon. Il ne s'est plus jamais coiffé."),
    (25, "Un acteur en costume de lézard géant postule comme mascotte du site. Refusé : pas assez crédible."),
    (30, "Les ventes de crème solaire indice 5000 explosent."),
    (35, "Les vaches du coin donnent du lait pétillant. Les fromagers sont perplexes."),
    (40, "Un champignonniste local porte plainte pour concurrence déloyale."),
    (45, "Sortie au cinéma de « Champignon 2 : le retour du retour ». Le public sort irradié d'émotion."),
    (50, "Le champignon est classé monument historique. Personne n'ose l'inspecter."),
    (60, "Les avions de ligne font désormais un détour de 300 km."),
    (70, "Un savant fou réclame des droits d'auteur sur le champignon. Il porte une blouse, c'est donc sérieux."),
    (75, "Une secte vénère le Grand Champignon. Ses membres ont tous le teint fluo."),
    (90, "Les scientifiques débattent : est-ce un champignon ou un nuage ? Les deux, apparemment."),
    (95, "Le doublage de la bande-annonce est confié à une voix grave qui dit « Cet été » toutes les trois secondes."),
    (100, "Le champignon atteint sa taille maximale. Il semble... frustré."),
    (200, "Les maquettes en carton du film catastrophe sont plus petites que le vrai champignon. Le tournage est reporté."),
    (300, "La NASA confirme : le champignon est visible depuis Mars."),
    (500, "Les Martiens envoient une soucoupe en fil de pêche pour voir de plus près."),
    (1_000, "Le champignon a son propre fuseau horaire."),
    (2_000, "Un festival de films de série Z organise sa cérémonie à l'ombre du champignon. Il fait très chaud."),
    (5_000, "Les extraterrestres demandent poliment de baisser le volume."),
    (10_000, "Le champignon reçoit un prix d'interprétation. Il n'a pas de discours, il n'a pas de bouche."),
    (20_000, "La Lune signale un léger réchauffement de sa face cachée."),
    (100_000, "Le champignon dépasse l'orbite des satellites. Les GPS indiquent tous « champignon »."),
    (200_000, "Les étoiles filantes font un détour."),
    (500_000, "Le Soleil se sent menacé."),
    (999_999, "Les astronomes renomment la Voie lactée « Le Grand Champignon »."),
]
NEWS_PRESTIGE = [
    "Déjà-vu : un militaire distrait laisse à nouveau une bombe dans le jardin.",
    "Les historiens notent que le champignon semble repousser de plus en plus vite.",
    "Suite, remake, reboot : le champignon est de retour, et il n'a rien appris.",
    "Le militaire distrait jure qu'on ne l'y reprendra plus. On l'y reprendra.",
]
NEWS_NANAR = [
    "Le champignon est en carton-pâte, mais le réalisateur jure que ça ne se voit pas.",
    "Une perche de micro apparaît dans le champ du champignon. Personne ne la retire.",
    "Faux raccord : le champignon a changé de couleur entre deux plans.",
    "Le budget effets spéciaux est épuisé. Les explosions seront désormais bruitées à la bouche.",
]
NEWS_INSPECTOR = "ALERTE : un inspecteur de l'ONU approche du site !"
NEWS_ENEMY = dict(
    saboteur="Un individu cagoulé avec un sac marqué « BUTIN » a été aperçu près du site.",
    drone="Un drone sans immatriculation survole la zone. Il a l'air de prendre des notes.",
    hippies="Des manifestants réclament la paix dans le monde et un champignon plus petit.",
    commando="Un commando en tenue de camouflage avance. Il est très mal camouflé devant un champignon orange.",
)


def pick_news(g):
    if g.inspector:
        return NEWS_INSPECTOR
    if g.enemies:
        return NEWS_ENEMY[g.enemies[0]["kind"]]
    pool = [t for lvl, t in NEWS if lvl <= g.level]
    if g.prestige_count:
        pool += NEWS_PRESTIGE
    if g.challenge == "nanar":
        pool += NEWS_NANAR * 3  # the challenge sets the mood
    return g.rng.choice(pool)


# --- Supply drops --------------------------------------------------------------

def _radar_bonus(g):
    return 1 + 0.3 * g.tr("l_radar")


def _drop_interval(g):
    frequency = _radar_bonus(g) * (2 if g.has_mut("magnet") else 1)
    return g.rng.uniform(60, 150) / frequency


CHEST_REWARDS = ("frenzy", "clickfrenzy", "lump", "pt", "gauge", "army", "shield")


def catch(g, letter):
    """Catching the parcel opens a chest: a choice of 3 bonuses (choose_chest)."""
    plane = g.plane
    if not plane or letter != plane["letter"] or g.pending_chest:
        return False
    g.plane = None
    g.next_plane = g.time + _drop_interval(g)
    g.stats["drops"] += 1
    g.sfx.append("drop")
    pool = [k for k in CHEST_REWARDS if _chest_allowed(g, k)]
    choices = 4 if g.has_syn("smuggling") else 3
    g.pending_chest = g.rng.sample(pool, min(choices, len(pool)))
    return True


def _chest_allowed(g, kind):
    if kind == "gauge":
        return not g.qte and g.crit_gauge < 1
    if kind == "army":
        return g.challenge != "pacifist" and any(g.units.values())
    if kind == "shield":
        return g.challenge != "watched" and g.level >= 10 and not g.has_mut("fog")
    return kind in CHEST_REWARDS


def _drop_power(g):
    return 1 + 0.5 * g.pr("p_drop")


def chest_text(g, kind):
    """(name, description) of a chest bonus, numbers included."""
    power = _drop_power(g)
    if kind == "frenzy":
        return "Frénésie", f"Production ×7 pendant {30 * power:.0f} s."
    if kind == "clickfrenzy":
        return "Détonateur fou", f"Détonation ×20 pendant {15 * power:.0f} s."
    if kind == "lump":
        return "Caisse d'uranium", f"+{g.fmt(_uranium_lump(g))} ☢ tout de suite."
    if kind == "pt":
        return "Manuel tactique", f"+{g.fmt(_chest_points(g))} PT."
    if kind == "gauge":
        return "Plutonium", "Masse critique chargée à 100 % (appuie sur F)."
    if kind == "army":
        return "Renforts", f"+{10 * power:.0f} % de chaque type d'unité possédé (au moins 1)."
    return "Pots-de-vin", f"Aucun inspecteur pendant {5 * power:.0f} min (l'actuel repart)."


def _uranium_lump(g):
    return max(g.production() * 120, g.click_value(False) * 100) * _drop_power(g)


def _chest_points(g):
    return max(1, int(g.level // 25 * _drop_power(g)))


def choose_chest(g, idx):
    """Applies the chosen bonus; False if the choice is invalid or no longer has an effect (the gauge filled up)."""
    if not g.pending_chest or idx >= len(g.pending_chest) or not _chest_allowed(g, g.pending_chest[idx]):
        return False
    kind = g.pending_chest[idx]
    g.pending_chest = None
    power = _drop_power(g)
    name = chest_text(g, kind)[0]
    message = name
    if kind == "frenzy":
        g.add_buff("prod", 7, 30 * power, "Frénésie ×7")
    elif kind == "clickfrenzy":
        g.add_buff("click", 20, 15 * power, "Détonation ×20")
    elif kind == "lump":
        amount = _uranium_lump(g)
        g.gain(amount)
        message += f" (+{g.fmt(amount)} ☢)"
    elif kind == "pt":
        pts = _chest_points(g)
        g.pt += pts
        message += f" (+{g.fmt(pts)} PT)"
    elif kind == "gauge":
        g.crit_gauge = 1.0
    elif kind == "army":
        for uid, n in g.units.items():
            if n:
                g.add_units(uid, max(1, int(n * 0.1 * power)))
    elif kind == "shield":
        if g.inspector:
            _inspector_gone(g, chased=False)
        g.bribe_until = g.time + 300 * power
        g.next_inspector = max(g.next_inspector, g.bribe_until)
    g.sfx.append("chest")
    g.run_stats["chests"].append(name)
    g.log(f"Coffre : {message} !", "loot", title="COFFRE")
    return True


# --- UN inspectors ----------------------------------------------------------

INSPECTOR_HITS = 5
_INSPECTOR_LEVEL = 10
MAX_HITS_MULT = 2


def threat_pressure(g):
    """Threats grow with the mushroom: 1 up to level 100, +0.5 per ×10 beyond (3 at level 1 000 000)."""
    return 1 + 0.5 * math.log10(max(1, g.level / data.BASE_CAP))


def _scaled(g, base, max_mult=math.inf):
    """base × threat pressure (at most ×max_mult), rounded half up: banker's rounding would give 3×1.5 → 4
    but 5×1.5 → 8."""
    return math.floor(base * min(max_mult, threat_pressure(g)) + 0.5)


def inspector_hits(g):
    return _scaled(g, INSPECTOR_HITS, MAX_HITS_MULT)


def enemy_hits(g, kind):
    return _scaled(g, ENEMIES[kind]["hits"], MAX_HITS_MULT)


def inspector_freeze(g):
    """Seconds of frozen production if the inspector reaches the mushroom."""
    return _scaled(g, 20) * (2 if g.has_mut("gamble") else 1)


def stolen_percent(g):
    """Share of the radiation the saboteur steals."""
    return _scaled(g, 15)


def sabotaged_percent(g):
    """Share of each unit type the commando destroys."""
    return _scaled(g, 10)


def protest_divisor(g):
    """Production divisor while protesters are present (to the tenth, as announced)."""
    return _scaled(g, 20) / 10


def _inspector_can_come(g):
    """The fog hides the mushroom from inspectors, except during the Sous surveillance challenge."""
    return g.level >= _INSPECTOR_LEVEL and (g.challenge == "watched" or not g.has_mut("fog"))


def chase(g):
    """X hits the inspector if there is one, otherwise the other enemy present."""
    inspector = g.inspector
    if inspector:
        inspector["hits"] += 1
        g.sfx.append("hit")
        if inspector["hits"] >= inspector_hits(g):
            _inspector_gone(g, chased=True)
    elif g.enemies:
        closest = max(g.enemies, key=lambda e: enemy_progress(g, e))
        closest["hits"] += 1
        g.sfx.append("hit")
        if closest["hits"] >= enemy_hits(g, closest["kind"]):
            _enemy_gone(g, closest, chased=True)


def _inspector_gone(g, chased):
    g.inspector = None
    g.schedule_inspector()
    if chased:
        g.stats["inspectors"] += 1
        g.run_stats["inspectors"] += 1
        g.sfx.append("chased")
        loot_mult = 3 if "watched" in g.challenges_done else 1
        bonus = max(g.production() * 30, g.click_value(False) * 20) * loot_mult
        g.gain(bonus)
        g.log(f"L'inspecteur s'enfuit en courant ! (+{g.fmt(bonus)} ☢ de matériel abandonné)", "good", title="OUSTE !")


# --- Other enemies ------------------------------------------------------------
# g.enemies: only one at a time normally, waves of 1 to 3 during the Invasion challenge (arriving
# WAVE_GAP s apart). X chases them too: the inspector first, then the most advanced enemy.
# Effect on reaching the mushroom: _enemy_arrives. Protesters, however, act for as long as they are present.

# level: level it appears at, hits: X presses to chase it (base, scaled by enemy_hits),
# dur: seconds to reach the mushroom, sky: flies (drawn in the sky), desc: announced effect ({}: its current
# strength, see enemy_desc).
ENEMIES = {
    "saboteur": dict(name="Saboteur", level=15, hits=3, dur=12.0, sky=False,
                     desc="vole {} % de la radiation s'il atteint le champignon"),
    "drone": dict(name="Drone espion", level=25, hits=4, dur=15.0, sky=True,
                  desc="vide la masse critique et appelle un inspecteur"),
    "hippies": dict(name="Manifestants", level=40, hits=8, dur=25.0, sky=False,
                    desc="production divisée par {} tant qu'ils sont là"),
    "commando": dict(name="Commando", level=60, hits=10, dur=20.0, sky=False,
                     desc="détruit {} % de chaque type d'unité s'il arrive"),
}


WAVE_GAP = 1.5
WAVE_MAX = 3


def wave_size(wave):
    """Wave 1: 1 enemy, 2-3: 2 enemies, then 3."""
    return min(WAVE_MAX, 1 + wave // 2)


_EFFECT_STRENGTH = dict(saboteur=stolen_percent, commando=sabotaged_percent, hippies=protest_divisor)


def enemy_desc(g, kind):
    """Announced effect, with its current strength (it grows with the threat pressure)."""
    strength = _EFFECT_STRENGTH.get(kind)
    value = strength(g) if strength else None
    if isinstance(value, float):
        value = f"{value:g}".replace(".", ",")
    return ENEMIES[kind]["desc"].format(value)


def enemy_progress(g, e):
    return (g.time - e["start"]) / e["dur"]


def enemy_duration(g, kind):
    return ENEMIES[kind]["dur"] * (1.5 if "invasion" in g.challenges_done else 1)


def _new_enemy(g, kind, start):
    return dict(kind=kind, start=start, dur=enemy_duration(g, kind), hits=0)


def spawn_enemy(g):
    invasion = g.challenge == "invasion"
    pool = [k for k, e in ENEMIES.items() if g.level >= e["level"] or invasion]
    if not pool:
        g.schedule_enemy()
        return
    g.sfx.append("enemy")
    if not invasion:
        kind = g.rng.choice(pool)
        g.enemies = [_new_enemy(g, kind, g.time)]
        spec = ENEMIES[kind]
        g.log(f"{spec['name']} en approche : {enemy_desc(g, kind)} ! Martèle X.", "bad", title="ALERTE")
        return
    g.wave += 1
    kinds = [g.rng.choice(pool) for _ in range(wave_size(g.wave))]
    g.enemies = [_new_enemy(g, k, g.time + i * WAVE_GAP) for i, k in enumerate(kinds)]
    names = ", ".join(ENEMIES[k]["name"] for k in kinds)
    g.log(f"Vague {g.wave} : {names} ! Martèle X.", "bad", title=f"VAGUE {g.wave}")


def _enemy_gone(g, e, chased):
    kind = e["kind"]
    g.enemies.remove(e)
    if not g.enemies:
        g.schedule_enemy()
    if chased:
        g.stats["enemies"] += 1
        g.sfx.append("chased")
        bonus = max(g.production() * 20, g.click_value(False) * 15)
        g.gain(bonus)
        g.log(f"{ENEMIES[kind]['name']} repoussé ! (+{g.fmt(bonus)} ☢ de butin)", "good", title="REPOUSSÉ")


def _enemy_arrives(g, e):
    kind = e["kind"]
    if kind == "saboteur":
        stolen = g.radiation * stolen_percent(g) / 100
        g.radiation -= stolen
        g.log(f"Le saboteur s'enfuit avec {g.fmt(stolen)} ☢ !", "bad", title="VOLÉ !")
    elif kind == "drone":
        g.crit_gauge = 0.0
        # The drone overrides neither bribes nor the fog.
        bribed = g.time < g.bribe_until
        calls_inspector = not g.inspector and not bribed and _inspector_can_come(g)
        if calls_inspector:
            g.next_inspector = min(g.next_inspector, g.time + 5)
            outcome = " : un inspecteur arrive bientôt !"
        else:
            outcome = " et vidé ta masse critique !"
        g.log("Le drone a photographié le site" + outcome, "bad", title="REPÉRÉ")
    elif kind == "commando":
        percent = sabotaged_percent(g)
        for uid, n in g.units.items():
            g.units[uid] = n - n * percent // 100
        g.log(f"Le commando a saboté {percent} % de ton armée !", "bad", title="SABOTAGE")
    else:
        g.log("Les manifestants rentrent chez eux.", "info", title="FIN DU SIT-IN")
    g.sfx.append("freeze" if kind != "hippies" else "tick")
    _enemy_gone(g, e, chased=False)


# --- Critical mass -----------------------------------------------------------

_QTE_TIMEOUT = 6
_PERFECT_MULT = 300
_PARTIAL_MULT = 60
_CRIT_CHARGE_TIME = 90


def crit_press(g):
    if g.qte:
        _qte_resolve(g)
    elif g.crit_gauge >= 1 and g.exploded:
        center = g.rng.uniform(0.2, 0.8)
        g.sfx.append("charge")
        g.qte = dict(t=0.0, center=center, green=0.06, yellow=0.17, speed=0.9 + 0.1 * g.rng.random())


def qte_pos(q):
    """Cursor position (0..1), going back and forth."""
    x = (q["t"] * q["speed"]) % 2
    return x if x <= 1 else 2 - x


def _qte_resolve(g, timeout=False):
    q = g.qte
    g.qte = None
    g.crit_gauge = 0
    distance = abs(qte_pos(q) - q["center"])
    base = max(g.production(), g.click_value(False) * 2)
    shown_until = g.time + 2
    if not timeout and distance <= q["green"]:
        reward = base * _PERFECT_MULT
        g.stats["perfects"] += 1
        g.run_stats["perfects"] += 1
        g.sfx.append("perfect")
        g.gain(reward)
        g.flash_until = g.time + 0.5
        g.qte_result = ("FUSION PARFAITE ! +" + g.fmt(reward) + " ☢", "green", shown_until)
    elif not timeout and distance <= q["yellow"]:
        reward = base * _PARTIAL_MULT
        g.gain(reward)
        g.sfx.append("partial")
        g.qte_result = ("Fusion partielle : +" + g.fmt(reward) + " ☢", "yellow", shown_until)
    else:
        g.sfx.append("fizzle")
        g.qte_result = ("Pétard mouillé...", "red", shown_until)


# --- Mutations -----------------------------------------------------------------

def offer_mutation(g):
    if g.challenge == "sterile":
        g.mut_queue = 0
        return
    pool = [m[0] for m in MUTATIONS if m[0] not in g.mutations]
    if not pool:
        g.mut_queue = 0
        return
    g.sfx.append("mutation")
    choices = 3 + g.pr("p_dna") + (1 if "blind" in g.challenges_done else 0)
    g.pending_mutation = g.rng.sample(pool, min(len(pool), choices))


def choose_mutation(g, idx):
    if not g.pending_mutation or idx >= len(g.pending_mutation):
        return
    mid = g.pending_mutation[idx]
    syn = synergy_completed_by(g, mid)
    g.mutations.append(mid)
    g.stats["mutations"] += 1
    g.pending_mutation = None
    g.log(f"Mutation : {MUTATION_BY_ID[mid][1]} !", "good", title="MUTATION")
    if syn:
        syn_id, syn_name, _, syn_effect = syn
        discovered = syn_id not in g.synergies_found
        g.synergies_found.add(syn_id)
        g.sfx.append("synergy")
        g.log(f"Synergie{' découverte' if discovered else ''} : {syn_name} ! {syn_effect}", "epic", title="SYNERGIE")
    if g.mut_queue > 0:
        g.mut_queue -= 1
        offer_mutation(g)


# --- Tick ------------------------------------------------------------------------

def tick(g, dt):
    _tick_crit_mass(g, dt)
    _tick_plane(g)
    _tick_inspector(g)
    _tick_enemies(g, dt)


def _tick_crit_mass(g, dt):
    if g.qte:
        g.qte["t"] += dt
        if g.qte["t"] > _QTE_TIMEOUT:
            _qte_resolve(g, timeout=True)
        return
    rate = (1 + 0.25 * g.tr("l_enrich")) * (2 if g.has_mut("unstable") else 1) * 2 ** g.sr("s_crit") / _CRIT_CHARGE_TIME
    if "sterile" in g.challenges_done:
        rate *= 1.5
    if g.challenge != "unlucky":
        g.crit_gauge = min(1.0, g.crit_gauge + rate * dt)


def _tick_plane(g):
    if g.plane:
        if g.time > g.plane["start"] + g.plane["dur"]:
            g.plane = None
            g.next_plane = g.time + _drop_interval(g)
    elif g.time >= g.next_plane and g.level >= 3 and not g.pending_chest and g.challenge != "unlucky":
        g.sfx.append("plane")
        g.plane = dict(letter=g.rng.choice(DROP_LETTERS), start=g.time, dur=10 * _radar_bonus(g))


def _tick_inspector(g):
    if g.inspector:
        progress = (g.time - g.inspector["start"]) / g.inspector["dur"]
        if g.tr("l_guard") and g.challenge != "watched" and progress > 0.3:
            g.log("Tes gardes escortent l'inspecteur vers la sortie.", "good", title="GARDES")
            _inspector_gone(g, chased=True)
        elif progress >= 1:
            freeze = inspector_freeze(g)
            g.freeze_until = g.time + freeze
            g.stats["frozen"] += 1
            g.sfx.append("freeze")
            g.log(f"L'inspecteur a mis les scellés : production gelée {freeze} s !", "bad", title="GELÉ !")
            _inspector_gone(g, chased=False)
    elif g.time >= g.next_inspector and _inspector_can_come(g):
        g.sfx.append("alarm")
        g.inspector = dict(start=g.time, dur=20.0, hits=0)


def _tick_enemies(g, dt):
    for e in list(g.enemies):
        if e["kind"] == "drone" and g.time >= e["start"]:
            g.crit_gauge = max(0.0, g.crit_gauge - 0.03 * dt)
        if g.time - e["start"] >= e["dur"]:
            _enemy_arrives(g, e)
    if not g.enemies and g.time >= g.next_enemy:
        spawn_enemy(g)
