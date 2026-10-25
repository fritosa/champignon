//! Random events, critical mass, mutations, achievements and news (pure logic).

use crate::data;
use crate::game::{BuffKind, Enemy, Game, Inspector, MsgKind, Plane, Qte, QteResult};
use crate::rng::RandomExt;

/// Letters usable to catch a supply drop (no control keys).
pub const DROP_LETTERS: &str = "abcdeghilnoprstuvwyz";

pub struct Mutation {
    pub id: &'static str,
    pub name: &'static str,
    pub effect: &'static str,
}

const fn mutation_def(id: &'static str, name: &'static str, effect: &'static str) -> Mutation {
    Mutation { id, name, effect }
}

#[rustfmt::skip]
pub const MUTATIONS: [Mutation; 12] = [
    mutation_def("spores", "Spores radioactives", "+1 % de production par soldat."),
    mutation_def("twin", "Champignon jumeau", "Détonation ×2 (et un deuxième chapeau !)."),
    mutation_def("acid", "Pluie acide", "Unités -30 % de coût, mais détonation ÷2."),
    mutation_def("unstable", "Cœur instable", "La masse critique se charge 2× plus vite."),
    mutation_def("fallout", "Retombées massives", "Production ×1,5."),
    mutation_def("magnet", "Aimant à colis", "Ravitaillements 2× plus fréquents."),
    mutation_def("berserk", "Rage atomique", "Combo max +2."),
    mutation_def("fog", "Brouillard radioactif", "Les inspecteurs ne trouvent plus le champignon."),
    mutation_def("mitosis", "Mitose", "+1 PT supplémentaire par niveau."),
    mutation_def("asbestos", "Doigts d'amiante", "Plus jamais de surchauffe."),
    mutation_def("glow", "Lueur verte", "+15 % de chance de détonation critique."),
    mutation_def("gamble", "Roulette russe", "Production ×3, mais les inspecteurs gèlent 2× plus longtemps."),
];

pub fn find_mutation(id: &str) -> Option<&'static Mutation> {
    MUTATIONS.iter().find(|m| m.id == id)
}

pub fn mutation(id: &str) -> &'static Mutation {
    find_mutation(id).unwrap_or_else(|| panic!("unknown mutation {id}"))
}

/// Two specific mutations unlock a hidden bonus. Each mutation belongs to only one pair.
pub struct Synergy {
    pub id: &'static str,
    pub name: &'static str,
    pub pair: (&'static str, &'static str),
    pub effect: &'static str,
}

const fn synergy_def(
    id: &'static str,
    name: &'static str,
    pair: (&'static str, &'static str),
    effect: &'static str,
) -> Synergy {
    Synergy {
        id,
        name,
        pair,
        effect,
    }
}

#[rustfmt::skip]
pub const SYNERGIES: [Synergy; 6] = [
    synergy_def("proliferation", "Prolifération", ("spores", "mitosis"),
                "Spores : +2 % de production par soldat au lieu de 1 %."),
    synergy_def("chain", "Fission en cascade", ("unstable", "glow"),
                "Chaque détonation critique charge 3 % de la masse critique."),
    synergy_def("storm", "Orage acide", ("acid", "fallout"), "La pluie acide ne divise plus la détonation."),
    synergy_def("berserker", "Berserker", ("berserk", "asbestos"), "Le combo retombe 3 fois moins vite."),
    synergy_def("smuggling", "Réseau de contrebande", ("magnet", "fog"),
                "Les coffres proposent 4 choix au lieu de 3."),
    synergy_def("twins", "Jumeaux maudits", ("twin", "gamble"), "Production ×1,5 supplémentaire."),
];

pub fn find_synergy(id: &str) -> Option<&'static Synergy> {
    SYNERGIES.iter().find(|s| s.id == id)
}

pub fn synergy(id: &str) -> &'static Synergy {
    find_synergy(id).unwrap_or_else(|| panic!("unknown synergy {id}"))
}

/// Synergy that mutation mid would complete with the mutations already owned (or None).
pub fn synergy_completed_by(g: &Game, mid: &str) -> Option<&'static Synergy> {
    SYNERGIES.iter().find(|syn| {
        let (a, b) = syn.pair;
        let partner = if mid == a {
            b
        } else if mid == b {
            a
        } else {
            return false;
        };
        g.mutations.contains(&partner)
    })
}

const FIRST_MUTATION_LEVELS: [u64; 4] = [25, 50, 75, 100];

/// Levels that trigger a mutation: 25, 50, 75, 100, then 200, 400, 800...
pub fn mutation_milestone(i: u32) -> u64 {
    match FIRST_MUTATION_LEVELS.get(i as usize) {
        Some(&level) => level,
        None => 100u64.saturating_mul(1u64.checked_shl(i - 3).unwrap_or(u64::MAX)),
    }
}

pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    /// Displayed condition.
    pub desc: &'static str,
    pub check: fn(&Game) -> bool,
}

const fn achievement(
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    check: fn(&Game) -> bool,
) -> Achievement {
    Achievement {
        id,
        name,
        desc,
        check,
    }
}

#[rustfmt::skip]
pub const ACHIEVEMENTS: [Achievement; 41] = [
    achievement("boom", "Premier boom", "Déclenche l'explosion.", |g| g.exploded),
    achievement("lvl10", "Ça chauffe", "Niveau 10.", |g| g.stats.max_level >= 10),
    achievement("lvl25", "Petit nuage", "Niveau 25.", |g| g.stats.max_level >= 25),
    achievement("lvl50", "Ça se voit de loin", "Niveau 50.", |g| g.stats.max_level >= 50),
    achievement("lvl100", "Masse maximale", "Niveau 100.", |g| g.stats.max_level >= 100),
    achievement("lvl1k", "Stratosphère", "Niveau 1 000.", |g| g.stats.max_level >= 1_000),
    achievement("lvl10k", "Orbite basse", "Niveau 10 000.", |g| g.stats.max_level >= 10_000),
    achievement("lvl100k", "Visible depuis Mars", "Niveau 100 000.", |g| g.stats.max_level >= 100_000),
    achievement("lvl1m", "Le champignon ultime", "Niveau 1 000 000.", |g| g.stats.max_level >= 1_000_000),
    achievement("clicks100", "Index musclé", "100 appuis sur ESPACE.", |g| g.stats.clicks >= 100),
    achievement("clicks1k", "Tendinite", "1 000 appuis sur ESPACE.", |g| g.stats.clicks >= 1_000),
    achievement("clicks10k", "Barre d'espace usée", "10 000 appuis sur ESPACE.", |g| g.stats.clicks >= 10_000),
    achievement("soldiers50", "Section", "50 soldats.", |g| g.units["soldier"] >= 50),
    achievement("soldiers100", "Compagnie", "100 soldats.", |g| g.units["soldier"] >= 100),
    achievement("tanks50", "Division blindée", "50 tanks.", |g| g.units["tank"] >= 50),
    achievement("army", "Armée complète", "Au moins une unité de chaque type.", |g| g.units.all()),
    achievement("units500", "Mobilisation générale", "500 unités au total.", |g| g.units.total() >= 500),
    achievement("prod1k", "Mille par seconde", "1 000 ☢/s.", |g| g.production() >= 1e3),
    achievement("prod1m", "Mégatonne", "1 M ☢/s.", |g| g.production() >= 1e6),
    achievement("prod1b", "Gigatonne", "1 B ☢/s.", |g| g.production() >= 1e9),
    achievement("rich", "Réserve stratégique", "1 T ☢ en stock.", |g| g.radiation >= 1e12),
    achievement("combo", "Réaction en chaîne", "Atteins le combo maximum.", |g| g.stats.max_combo),
    achievement("overheat", "Doigts brûlés", "Provoque une surchauffe.", |g| g.stats.overheats >= 1),
    achievement("drop1", "Colis reçu", "Attrape un ravitaillement.", |g| g.stats.drops >= 1),
    achievement("drop25", "Contrebandier", "Attrape 25 ravitaillements.", |g| g.stats.drops >= 25),
    achievement("perfect", "Timing parfait", "Masse critique en zone verte.", |g| g.stats.perfects >= 1),
    achievement("perfect10", "Maître de la fission", "10 masses critiques parfaites.", |g| g.stats.perfects >= 10),
    achievement("inspector", "Circulez !", "Chasse un inspecteur de l'ONU.", |g| g.stats.inspectors >= 1),
    achievement("inspector25", "Persona non grata", "Chasse 25 inspecteurs.", |g| g.stats.inspectors >= 25),
    achievement("enemy1", "Légitime défense", "Repousse un ennemi (saboteur, drone...).", |g| g.stats.enemies >= 1),
    achievement("enemy25", "Forteresse", "Repousse 25 ennemis.", |g| g.stats.enemies >= 25),
    achievement("frozen", "Visite surprise", "Laisse un inspecteur geler ta production.", |g| g.stats.frozen >= 1),
    achievement("mutant", "Mutant", "Choisis 10 mutations.", |g| g.stats.mutations >= 10),
    achievement("prestige1", "Renaissance", "Premier prestige.", |g| g.prestige_count >= 1),
    achievement("prestige10", "Cycle éternel", "10 prestiges.", |g| g.prestige_count >= 10),
    achievement("supernova", "Supernova", "Déclenche une supernova.", |g| g.supernova_count >= 1),
    achievement("synergy1", "Alchimiste", "Découvre une synergie de mutations.", |g| !g.synergies_found.is_empty()),
    achievement("synergy_all", "Généticien fou", "Découvre toutes les synergies.",
                |g| g.synergies_found.len() >= SYNERGIES.len()),
    achievement("speed", "Éclair", "Prestige en moins de 15 minutes.",
                |g| g.stats.best_prestige_time.is_some_and(|t| t < 900.0)),
    achievement("challenges", "Vétéran des défis", "Réussis 3 défis.", |g| g.challenges_done.len() >= 3),
    achievement("challenges_all", "Maître des défis", "Réussis tous les défis.",
                |g| g.challenges_done.len() >= data::CHALLENGES.len()),
];

pub fn find_achievement(id: &str) -> Option<&'static Achievement> {
    ACHIEVEMENTS.iter().find(|a| a.id == id)
}

/// (minimum level, headline). The banner picks at random among the unlocked ones.
#[rustfmt::skip]
pub const NEWS: [(u32, &str); 38] = [
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
];
pub const NEWS_PRESTIGE: [&str; 4] = [
    "Déjà-vu : un militaire distrait laisse à nouveau une bombe dans le jardin.",
    "Les historiens notent que le champignon semble repousser de plus en plus vite.",
    "Suite, remake, reboot : le champignon est de retour, et il n'a rien appris.",
    "Le militaire distrait jure qu'on ne l'y reprendra plus. On l'y reprendra.",
];
pub const NEWS_NANAR: [&str; 4] = [
    "Le champignon est en carton-pâte, mais le réalisateur jure que ça ne se voit pas.",
    "Une perche de micro apparaît dans le champ du champignon. Personne ne la retire.",
    "Faux raccord : le champignon a changé de couleur entre deux plans.",
    "Le budget effets spéciaux est épuisé. Les explosions seront désormais bruitées à la bouche.",
];
pub const NEWS_INSPECTOR: &str = "ALERTE : un inspecteur de l'ONU approche du site !";

pub fn pick_news(g: &mut Game) -> &'static str {
    if g.inspector.is_some() {
        return NEWS_INSPECTOR;
    }
    if let Some(e) = g.enemies.first() {
        return e.kind.spec().news;
    }
    let mut pool: Vec<&'static str> = NEWS
        .iter()
        .filter(|(lvl, _)| *lvl <= g.level)
        .map(|(_, t)| *t)
        .collect();
    if g.prestige_count > 0 {
        pool.extend(NEWS_PRESTIGE);
    }
    if g.challenge == Some("nanar") {
        for _ in 0..3 {
            pool.extend(NEWS_NANAR); // the challenge sets the mood
        }
    }
    g.rng.choice(&pool)
}

// --- Supply drops --------------------------------------------------------------

fn radar_bonus(g: &Game) -> f64 {
    1.0 + 0.3 * g.tr("l_radar") as f64
}

fn drop_interval(g: &mut Game) -> f64 {
    let frequency = radar_bonus(g) * if g.has_mut("magnet") { 2.0 } else { 1.0 };
    g.rng.uniform(60.0, 150.0) / frequency
}

pub const CHEST_REWARDS: [&str; 7] = [
    "frenzy",
    "clickfrenzy",
    "lump",
    "pt",
    "gauge",
    "army",
    "shield",
];

/// Catching the parcel opens a chest: a choice of 3 bonuses (choose_chest).
pub fn catch(g: &mut Game, letter: char) -> bool {
    match g.plane {
        Some(plane) if plane.letter == letter && g.pending_chest.is_none() => {}
        _ => return false,
    }
    g.plane = None;
    g.next_plane = g.time + drop_interval(g);
    g.stats.drops += 1;
    g.sfx.push_back("drop");
    let pool: Vec<&'static str> = CHEST_REWARDS
        .into_iter()
        .filter(|k| chest_allowed(g, k))
        .collect();
    let choices = if g.has_syn("smuggling") { 4 } else { 3 };
    g.pending_chest = Some(g.rng.sample(&pool, choices.min(pool.len())));
    true
}

/// Whether a chest bonus can be offered (or still be taken).
pub fn chest_allowed(g: &Game, kind: &str) -> bool {
    match kind {
        "gauge" => g.qte.is_none() && g.crit_gauge < 1.0,
        "army" => g.challenge != Some("pacifist") && g.units.any(),
        "shield" => g.challenge != Some("watched") && g.level >= 10 && !g.has_mut("fog"),
        _ => CHEST_REWARDS.contains(&kind),
    }
}

fn drop_power(g: &Game) -> f64 {
    1.0 + 0.5 * g.pr("p_drop") as f64
}

/// (name, description) of a chest bonus, numbers included.
pub fn chest_text(g: &Game, kind: &str) -> (&'static str, String) {
    let power = drop_power(g);
    match kind {
        "frenzy" => (
            "Frénésie",
            format!("Production ×7 pendant {:.0} s.", 30.0 * power),
        ),
        "clickfrenzy" => (
            "Détonateur fou",
            format!("Détonation ×20 pendant {:.0} s.", 15.0 * power),
        ),
        "lump" => (
            "Caisse d'uranium",
            format!("+{} ☢ tout de suite.", g.fmt(uranium_lump(g))),
        ),
        "pt" => (
            "Manuel tactique",
            format!("+{} PT.", g.fmt(chest_points(g) as f64)),
        ),
        "gauge" => (
            "Plutonium",
            "Masse critique chargée à 100 % (appuie sur F).".to_string(),
        ),
        "army" => (
            "Renforts",
            format!(
                "+{:.0} % de chaque type d'unité possédé (au moins 1).",
                10.0 * power
            ),
        ),
        _ => (
            "Pots-de-vin",
            format!(
                "Aucun inspecteur pendant {:.0} min (l'actuel repart).",
                5.0 * power
            ),
        ),
    }
}

fn uranium_lump(g: &Game) -> f64 {
    (g.production() * 120.0).max(g.click_value(false) * 100.0) * drop_power(g)
}

fn chest_points(g: &Game) -> i64 {
    (((g.level / 25) as f64 * drop_power(g)) as i64).max(1)
}

/// Applies the chosen bonus; false if the choice is invalid or no longer has an effect (the gauge filled up).
pub fn choose_chest(g: &mut Game, idx: usize) -> bool {
    let kind = match &g.pending_chest {
        Some(chest) if idx < chest.len() && chest_allowed(g, chest[idx]) => chest[idx],
        _ => return false,
    };
    g.pending_chest = None;
    let power = drop_power(g);
    let name = chest_text(g, kind).0;
    let mut message = name.to_string();
    match kind {
        "frenzy" => g.add_buff(BuffKind::Prod, 7.0, 30.0 * power, "Frénésie ×7"),
        "clickfrenzy" => g.add_buff(BuffKind::Click, 20.0, 15.0 * power, "Détonation ×20"),
        "lump" => {
            let amount = uranium_lump(g);
            g.gain(amount);
            message += &format!(" (+{} ☢)", g.fmt(amount));
        }
        "pt" => {
            let pts = chest_points(g);
            g.pt += pts;
            message += &format!(" (+{} PT)", g.fmt(pts as f64));
        }
        "gauge" => g.crit_gauge = 1.0,
        "army" => {
            let owned: Vec<(&'static str, u64)> = g.units.iter().filter(|&(_, n)| n > 0).collect();
            for (uid, n) in owned {
                g.add_units(uid, ((n as f64 * 0.1 * power) as u64).max(1));
            }
        }
        _ => {
            // shield
            if g.inspector.is_some() {
                inspector_gone(g, false);
            }
            g.bribe_until = g.time + 300.0 * power;
            g.next_inspector = g.next_inspector.max(g.bribe_until);
        }
    }
    g.sfx.push_back("chest");
    g.run_stats.chests.push(name.to_string());
    g.log(
        format!("Coffre : {message} !"),
        MsgKind::Loot,
        Some("COFFRE"),
    );
    true
}

// --- UN inspectors ----------------------------------------------------------

pub const INSPECTOR_HITS: u32 = 5;
const INSPECTOR_LEVEL: u32 = 10;
pub const MAX_HITS_MULT: f64 = 2.0;

/// Threats grow with the mushroom: 1 up to level 100, +0.5 per ×10 beyond (3 at level 1 000 000).
pub fn threat_pressure(g: &Game) -> f64 {
    1.0 + 0.5 * (g.level as f64 / data::BASE_CAP as f64).max(1.0).log10()
}

/// base × threat pressure (at most ×max_mult), rounded half up: banker's rounding would give 3×1.5 → 4
/// but 5×1.5 → 8.
fn scaled(g: &Game, base: u32, max_mult: f64) -> u32 {
    (base as f64 * max_mult.min(threat_pressure(g)) + 0.5).floor() as u32
}

pub fn inspector_hits(g: &Game) -> u32 {
    scaled(g, INSPECTOR_HITS, MAX_HITS_MULT)
}

pub fn enemy_hits(g: &Game, kind: EnemyKind) -> u32 {
    scaled(g, kind.spec().hits, MAX_HITS_MULT)
}

/// Seconds of frozen production if the inspector reaches the mushroom.
pub fn inspector_freeze(g: &Game) -> u32 {
    scaled(g, 20, f64::INFINITY) * if g.has_mut("gamble") { 2 } else { 1 }
}

/// Share of the radiation the saboteur steals.
pub fn stolen_percent(g: &Game) -> u32 {
    scaled(g, 15, f64::INFINITY)
}

/// Share of each unit type the commando destroys.
pub fn sabotaged_percent(g: &Game) -> u32 {
    scaled(g, 10, f64::INFINITY)
}

/// Production divisor while protesters are present (to the tenth, as announced).
pub fn protest_divisor(g: &Game) -> f64 {
    scaled(g, 20, f64::INFINITY) as f64 / 10.0
}

/// The fog hides the mushroom from inspectors, except during the Sous surveillance challenge.
fn inspector_can_come(g: &Game) -> bool {
    g.level >= INSPECTOR_LEVEL && (g.challenge == Some("watched") || !g.has_mut("fog"))
}

/// X hits the inspector if there is one, otherwise the other enemy present.
pub fn chase(g: &mut Game) {
    if let Some(inspector) = g.inspector.as_mut() {
        inspector.hits += 1;
        let hits = inspector.hits;
        g.sfx.push_back("hit");
        if hits >= inspector_hits(g) {
            inspector_gone(g, true);
        }
    } else if !g.enemies.is_empty() {
        // The most advanced one (the first of them on a tie).
        let mut closest = 0;
        for (i, e) in g.enemies.iter().enumerate() {
            if enemy_progress(g, e) > enemy_progress(g, &g.enemies[closest]) {
                closest = i;
            }
        }
        g.enemies[closest].hits += 1;
        g.sfx.push_back("hit");
        let e = g.enemies[closest];
        if e.hits >= enemy_hits(g, e.kind) {
            enemy_gone(g, closest, true);
        }
    }
}

fn inspector_gone(g: &mut Game, chased: bool) {
    g.inspector = None;
    g.schedule_inspector(false);
    if chased {
        g.stats.inspectors += 1;
        g.run_stats.inspectors += 1;
        g.sfx.push_back("chased");
        let loot_mult = if g.challenges_done.contains("watched") {
            3.0
        } else {
            1.0
        };
        let bonus = (g.production() * 30.0).max(g.click_value(false) * 20.0) * loot_mult;
        g.gain(bonus);
        g.log(
            format!(
                "L'inspecteur s'enfuit en courant ! (+{} ☢ de matériel abandonné)",
                g.fmt(bonus)
            ),
            MsgKind::Good,
            Some("OUSTE !"),
        );
    }
}

// --- Other enemies ------------------------------------------------------------
// g.enemies: only one at a time normally, waves of 1 to 3 during the Invasion challenge (arriving
// WAVE_GAP s apart). X chases them too: the inspector first, then the most advanced enemy.
// Effect on reaching the mushroom: enemy_arrives. Protesters, however, act for as long as they are present.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EnemyKind {
    Saboteur,
    Drone,
    Hippies,
    Commando,
}

/// level: level it appears at, hits: X presses to chase it (base, scaled by enemy_hits),
/// dur: seconds to reach the mushroom, sky: flies (drawn in the sky), desc: announced effect ({}: its current
/// strength, see enemy_desc), news: headline while it is there.
pub struct EnemySpec {
    pub kind: EnemyKind,
    pub id: &'static str,
    pub name: &'static str,
    pub level: u32,
    pub hits: u32,
    pub dur: f64,
    pub sky: bool,
    pub desc: &'static str,
    pub news: &'static str,
}

#[rustfmt::skip]
pub const ENEMIES: [EnemySpec; 4] = [
    EnemySpec { kind: EnemyKind::Saboteur, id: "saboteur", name: "Saboteur", level: 15, hits: 3, dur: 12.0, sky: false,
                desc: "vole {} % de la radiation s'il atteint le champignon",
                news: "Un individu cagoulé avec un sac marqué « BUTIN » a été aperçu près du site." },
    EnemySpec { kind: EnemyKind::Drone, id: "drone", name: "Drone espion", level: 25, hits: 4, dur: 15.0, sky: true,
                desc: "vide la masse critique et appelle un inspecteur",
                news: "Un drone sans immatriculation survole la zone. Il a l'air de prendre des notes." },
    EnemySpec { kind: EnemyKind::Hippies, id: "hippies", name: "Manifestants", level: 40, hits: 8, dur: 25.0,
                sky: false, desc: "production divisée par {} tant qu'ils sont là",
                news: "Des manifestants réclament la paix dans le monde et un champignon plus petit." },
    EnemySpec { kind: EnemyKind::Commando, id: "commando", name: "Commando", level: 60, hits: 10, dur: 20.0,
                sky: false, desc: "détruit {} % de chaque type d'unité s'il arrive",
                news: "Un commando en tenue de camouflage avance. Il est très mal camouflé devant un champignon \
                       orange." },
];

impl EnemyKind {
    pub const ALL: [EnemyKind; 4] = [
        EnemyKind::Saboteur,
        EnemyKind::Drone,
        EnemyKind::Hippies,
        EnemyKind::Commando,
    ];

    pub fn spec(self) -> &'static EnemySpec {
        &ENEMIES[self as usize]
    }

    pub fn id(self) -> &'static str {
        self.spec().id
    }

    pub fn from_id(id: &str) -> Option<EnemyKind> {
        ENEMIES.iter().find(|e| e.id == id).map(|e| e.kind)
    }
}

pub const WAVE_GAP: f64 = 1.5;
pub const WAVE_MAX: usize = 3;

/// Wave 1: 1 enemy, 2-3: 2 enemies, then 3.
pub fn wave_size(wave: u32) -> usize {
    WAVE_MAX.min(1 + wave as usize / 2)
}

/// Current strength of an enemy's effect, as announced (None: no number).
fn effect_strength(g: &Game, kind: EnemyKind) -> Option<String> {
    match kind {
        EnemyKind::Saboteur => Some(stolen_percent(g).to_string()),
        EnemyKind::Commando => Some(sabotaged_percent(g).to_string()),
        EnemyKind::Hippies => Some(protest_divisor(g).to_string().replace('.', ",")),
        EnemyKind::Drone => None,
    }
}

/// Announced effect, with its current strength (it grows with the threat pressure).
pub fn enemy_desc(g: &Game, kind: EnemyKind) -> String {
    let desc = kind.spec().desc;
    match effect_strength(g, kind) {
        Some(value) => desc.replace("{}", &value),
        None => desc.to_string(),
    }
}

pub fn enemy_progress(g: &Game, e: &Enemy) -> f64 {
    (g.time - e.start) / e.dur
}

pub fn enemy_duration(g: &Game, kind: EnemyKind) -> f64 {
    kind.spec().dur
        * if g.challenges_done.contains("invasion") {
            1.5
        } else {
            1.0
        }
}

fn new_enemy(g: &Game, kind: EnemyKind, start: f64) -> Enemy {
    Enemy {
        kind,
        start,
        dur: enemy_duration(g, kind),
        hits: 0,
    }
}

pub fn spawn_enemy(g: &mut Game) {
    let invasion = g.challenge == Some("invasion");
    let pool: Vec<EnemyKind> = ENEMIES
        .iter()
        .filter(|e| g.level >= e.level || invasion)
        .map(|e| e.kind)
        .collect();
    if pool.is_empty() {
        g.schedule_enemy(false);
        return;
    }
    g.sfx.push_back("enemy");
    if !invasion {
        let kind = *g.rng.choice(&pool);
        g.enemies = vec![new_enemy(g, kind, g.time)];
        let message = format!(
            "{} en approche : {} ! Martèle X.",
            kind.spec().name,
            enemy_desc(g, kind)
        );
        g.log(message, MsgKind::Bad, Some("ALERTE"));
        return;
    }
    g.wave += 1;
    let kinds: Vec<EnemyKind> = (0..wave_size(g.wave))
        .map(|_| *g.rng.choice(&pool))
        .collect();
    g.enemies = kinds
        .iter()
        .enumerate()
        .map(|(i, &k)| new_enemy(g, k, g.time + i as f64 * WAVE_GAP))
        .collect();
    let names: Vec<&str> = kinds.iter().map(|k| k.spec().name).collect();
    let title = format!("VAGUE {}", g.wave);
    g.log(
        format!("Vague {} : {} ! Martèle X.", g.wave, names.join(", ")),
        MsgKind::Bad,
        Some(&title),
    );
}

fn enemy_gone(g: &mut Game, idx: usize, chased: bool) {
    let kind = g.enemies.remove(idx).kind;
    if g.enemies.is_empty() {
        g.schedule_enemy(false);
    }
    if chased {
        g.stats.enemies += 1;
        g.sfx.push_back("chased");
        let bonus = (g.production() * 20.0).max(g.click_value(false) * 15.0);
        g.gain(bonus);
        g.log(
            format!(
                "{} repoussé ! (+{} ☢ de butin)",
                kind.spec().name,
                g.fmt(bonus)
            ),
            MsgKind::Good,
            Some("REPOUSSÉ"),
        );
    }
}

fn enemy_arrives(g: &mut Game, idx: usize) {
    let kind = g.enemies[idx].kind;
    match kind {
        EnemyKind::Saboteur => {
            let stolen = g.radiation * stolen_percent(g) as f64 / 100.0;
            g.radiation -= stolen;
            g.log(
                format!("Le saboteur s'enfuit avec {} ☢ !", g.fmt(stolen)),
                MsgKind::Bad,
                Some("VOLÉ !"),
            );
        }
        EnemyKind::Drone => {
            g.crit_gauge = 0.0;
            // The drone overrides neither bribes nor the fog.
            let bribed = g.time < g.bribe_until;
            let calls_inspector = g.inspector.is_none() && !bribed && inspector_can_come(g);
            let outcome = if calls_inspector {
                g.next_inspector = g.next_inspector.min(g.time + 5.0);
                " : un inspecteur arrive bientôt !"
            } else {
                " et vidé ta masse critique !"
            };
            g.log(
                format!("Le drone a photographié le site{outcome}"),
                MsgKind::Bad,
                Some("REPÉRÉ"),
            );
        }
        EnemyKind::Commando => {
            let percent = sabotaged_percent(g) as u64;
            for n in g.units.0.iter_mut() {
                *n -= *n * percent / 100;
            }
            g.log(
                format!("Le commando a saboté {percent} % de ton armée !"),
                MsgKind::Bad,
                Some("SABOTAGE"),
            );
        }
        EnemyKind::Hippies => {
            g.log(
                "Les manifestants rentrent chez eux.",
                MsgKind::Info,
                Some("FIN DU SIT-IN"),
            );
        }
    }
    if kind == EnemyKind::Hippies {
        g.sfx.push_back("tick");
    } else {
        g.sfx.push_back("freeze");
    }
    enemy_gone(g, idx, false);
}

// --- Critical mass -----------------------------------------------------------

const QTE_TIMEOUT: f64 = 6.0;
const PERFECT_MULT: f64 = 300.0;
const PARTIAL_MULT: f64 = 60.0;
const CRIT_CHARGE_TIME: f64 = 90.0;

pub fn crit_press(g: &mut Game) {
    if g.qte.is_some() {
        qte_resolve(g, false);
    } else if g.crit_gauge >= 1.0 && g.exploded {
        let center = g.rng.uniform(0.2, 0.8);
        g.sfx.push_back("charge");
        let speed = 0.9 + 0.1 * g.rng.random();
        g.qte = Some(Qte {
            t: 0.0,
            center,
            green: 0.06,
            yellow: 0.17,
            speed,
        });
    }
}

/// Cursor position (0..1), going back and forth.
pub fn qte_pos(q: &Qte) -> f64 {
    let x = (q.t * q.speed).rem_euclid(2.0);
    if x <= 1.0 { x } else { 2.0 - x }
}

fn qte_resolve(g: &mut Game, timeout: bool) {
    let Some(q) = g.qte.take() else { return };
    g.crit_gauge = 0.0;
    let distance = (qte_pos(&q) - q.center).abs();
    let base = g.production().max(g.click_value(false) * 2.0);
    let shown_until = g.time + 2.0;
    let (text, color) = if !timeout && distance <= q.green {
        let reward = base * PERFECT_MULT;
        g.stats.perfects += 1;
        g.run_stats.perfects += 1;
        g.sfx.push_back("perfect");
        g.gain(reward);
        g.flash_until = g.time + 0.5;
        (format!("FUSION PARFAITE ! +{} ☢", g.fmt(reward)), "green")
    } else if !timeout && distance <= q.yellow {
        let reward = base * PARTIAL_MULT;
        g.gain(reward);
        g.sfx.push_back("partial");
        (format!("Fusion partielle : +{} ☢", g.fmt(reward)), "yellow")
    } else {
        g.sfx.push_back("fizzle");
        ("Pétard mouillé...".to_string(), "red")
    };
    g.qte_result = Some(QteResult {
        text,
        color,
        until: shown_until,
    });
}

// --- Mutations -----------------------------------------------------------------

pub fn offer_mutation(g: &mut Game) {
    if g.challenge == Some("sterile") {
        g.mut_queue = 0;
        return;
    }
    let pool: Vec<&'static str> = MUTATIONS
        .iter()
        .map(|m| m.id)
        .filter(|m| !g.mutations.contains(m))
        .collect();
    if pool.is_empty() {
        g.mut_queue = 0;
        return;
    }
    g.sfx.push_back("mutation");
    let choices = 3 + g.pr("p_dna") as usize + usize::from(g.challenges_done.contains("blind"));
    g.pending_mutation = Some(g.rng.sample(&pool, pool.len().min(choices)));
}

pub fn choose_mutation(g: &mut Game, idx: usize) {
    let mid = match &g.pending_mutation {
        Some(offered) if idx < offered.len() => offered[idx],
        _ => return,
    };
    let syn = synergy_completed_by(g, mid);
    g.mutations.push(mid);
    g.stats.mutations += 1;
    g.pending_mutation = None;
    g.log(
        format!("Mutation : {} !", mutation(mid).name),
        MsgKind::Good,
        Some("MUTATION"),
    );
    if let Some(syn) = syn {
        let discovered = g.synergies_found.insert(syn.id);
        g.sfx.push_back("synergy");
        let found = if discovered { " découverte" } else { "" };
        g.log(
            format!("Synergie{found} : {} ! {}", syn.name, syn.effect),
            MsgKind::Epic,
            Some("SYNERGIE"),
        );
    }
    if g.mut_queue > 0 {
        g.mut_queue -= 1;
        offer_mutation(g);
    }
}

// --- Tick ------------------------------------------------------------------------

pub fn tick(g: &mut Game, dt: f64) {
    tick_crit_mass(g, dt);
    tick_plane(g);
    tick_inspector(g);
    tick_enemies(g, dt);
}

fn tick_crit_mass(g: &mut Game, dt: f64) {
    if let Some(q) = g.qte.as_mut() {
        q.t += dt;
        if q.t > QTE_TIMEOUT {
            qte_resolve(g, true);
        }
        return;
    }
    let mut rate = (1.0 + 0.25 * g.tr("l_enrich") as f64)
        * if g.has_mut("unstable") { 2.0 } else { 1.0 }
        * 2f64.powi(g.sr("s_crit") as i32)
        / CRIT_CHARGE_TIME;
    if g.challenges_done.contains("sterile") {
        rate *= 1.5;
    }
    if g.challenge != Some("unlucky") {
        g.crit_gauge = (g.crit_gauge + rate * dt).min(1.0);
    }
}

fn tick_plane(g: &mut Game) {
    if let Some(plane) = g.plane {
        if g.time > plane.start + plane.dur {
            g.plane = None;
            g.next_plane = g.time + drop_interval(g);
        }
    } else if g.time >= g.next_plane
        && g.level >= 3
        && g.pending_chest.is_none()
        && g.challenge != Some("unlucky")
    {
        g.sfx.push_back("plane");
        let letters: Vec<char> = DROP_LETTERS.chars().collect();
        let letter = *g.rng.choice(&letters);
        g.plane = Some(Plane {
            letter,
            start: g.time,
            dur: 10.0 * radar_bonus(g),
        });
    }
}

fn tick_inspector(g: &mut Game) {
    if let Some(inspector) = g.inspector {
        let progress = (g.time - inspector.start) / inspector.dur;
        if g.tr("l_guard") > 0 && g.challenge != Some("watched") && progress > 0.3 {
            g.log(
                "Tes gardes escortent l'inspecteur vers la sortie.",
                MsgKind::Good,
                Some("GARDES"),
            );
            inspector_gone(g, true);
        } else if progress >= 1.0 {
            let freeze = inspector_freeze(g);
            g.freeze_until = g.time + freeze as f64;
            g.stats.frozen += 1;
            g.sfx.push_back("freeze");
            g.log(
                format!("L'inspecteur a mis les scellés : production gelée {freeze} s !"),
                MsgKind::Bad,
                Some("GELÉ !"),
            );
            inspector_gone(g, false);
        }
    } else if g.time >= g.next_inspector && inspector_can_come(g) {
        g.sfx.push_back("alarm");
        g.inspector = Some(Inspector {
            start: g.time,
            dur: 20.0,
            hits: 0,
        });
    }
}

fn tick_enemies(g: &mut Game, dt: f64) {
    for e in g.enemies.clone() {
        if e.kind == EnemyKind::Drone && g.time >= e.start {
            g.crit_gauge = (g.crit_gauge - 0.03 * dt).max(0.0);
        }
        if g.time - e.start >= e.dur
            && let Some(idx) = g.enemies.iter().position(|x| *x == e)
        {
            enemy_arrives(g, idx);
        }
    }
    if g.enemies.is_empty() && g.time >= g.next_enemy {
        spawn_enemy(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::need;
    use crate::game::tests::{approx, exploded_game, exploded_game_with, fixed, has};

    fn plane_at(g: &Game, letter: char) -> Option<Plane> {
        Some(Plane {
            letter,
            start: g.time,
            dur: 10.0,
        })
    }

    fn inspector(start: f64, hits: u32) -> Option<Inspector> {
        Some(Inspector {
            start,
            dur: 20.0,
            hits,
        })
    }

    fn enemy(kind: EnemyKind, start: f64, dur: f64) -> Enemy {
        Enemy {
            kind,
            start,
            dur,
            hits: 0,
        }
    }

    fn last_message(g: &Game) -> &crate::game::Message {
        g.messages.back().unwrap()
    }

    // --- Drops ---

    fn drop_game() -> Game {
        let mut g = exploded_game();
        g.plane = plane_at(&g, 'a');
        g
    }

    #[test]
    fn wrong_letter() {
        let mut g = drop_game();
        assert!(!catch(&mut g, 'b'));
        assert!(g.plane.is_some());
    }

    #[test]
    fn catch_opens_chest() {
        let mut g = drop_game();
        assert!(catch(&mut g, 'a'));
        assert!(g.plane.is_none());
        assert_eq!(g.stats.drops, 1);
        assert!(has(&g, "drop"));
        let mut chest = g.pending_chest.clone().unwrap();
        assert_eq!(chest.len(), 3);
        chest.sort();
        chest.dedup();
        assert_eq!(chest.len(), 3);
    }

    // --- Chests ---

    fn open_with(kind: &'static str, mut g: Game) -> Game {
        g.pending_chest = Some(vec![kind]);
        choose_chest(&mut g, 0);
        assert!(g.pending_chest.is_none());
        assert!(has(&g, "chest"));
        let m = last_message(&g);
        assert_eq!(
            (m.kind, m.title.as_deref()),
            (MsgKind::Loot, Some("COFFRE"))
        );
        g
    }

    #[test]
    fn frenzies() {
        assert_eq!(
            open_with("frenzy", exploded_game()).buff_mult(BuffKind::Prod),
            7.0
        );
        assert_eq!(
            open_with("clickfrenzy", exploded_game()).buff_mult(BuffKind::Click),
            20.0
        );
    }

    #[test]
    fn lump_and_pt() {
        let mut g = exploded_game();
        g.units["soldier"] = 10;
        let expected = (g.production() * 120.0).max(g.click_value(false) * 100.0);
        let g = open_with("lump", g);
        assert!(approx(g.radiation, expected));
        let mut g = exploded_game();
        (g.level, g.pt) = (50, 0);
        let g = open_with("pt", g);
        assert_eq!(g.pt, 2);
    }

    #[test]
    fn gauge() {
        assert_eq!(open_with("gauge", exploded_game()).crit_gauge, 1.0);
    }

    #[test]
    fn army() {
        let mut g = exploded_game();
        (g.units["soldier"], g.units["mortar"]) = (45, 3);
        let g = open_with("army", g);
        assert_eq!((g.units["soldier"], g.units["mortar"]), (49, 4));
        let mut g = exploded_game();
        g.units["soldier"] = 48;
        let g = open_with("army", g);
        assert!(has(&g, "milestone")); // 48 → 52 crosses the 50 milestone
    }

    #[test]
    fn shield() {
        let mut g = exploded_game();
        g.level = 10;
        g.inspector = inspector(g.time, 0);
        let g = open_with("shield", g);
        assert!(g.inspector.is_none());
        assert!(g.next_inspector - g.time >= 300.0);
        assert_eq!(g.stats.inspectors, 0);
    }

    #[test]
    fn pool_filters() {
        let mut g = exploded_game();
        (g.crit_gauge, g.challenge) = (1.0, Some("watched"));
        let allowed: Vec<&str> = CHEST_REWARDS
            .into_iter()
            .filter(|k| chest_allowed(&g, k))
            .collect();
        assert!(!allowed.contains(&"gauge"));
        assert!(!allowed.contains(&"army")); // no units
        assert!(!allowed.contains(&"shield"));
    }

    #[test]
    fn texts_and_drop_power() {
        let mut g = exploded_game();
        for kind in CHEST_REWARDS {
            let (name, desc) = chest_text(&g, kind);
            assert!(!name.is_empty() && !desc.is_empty());
        }
        g.ptalents.insert("p_drop", 2);
        assert!(chest_text(&g, "frenzy").1.contains("60 s"));
        assert!(chest_text(&g, "shield").1.contains("10 min"));
    }

    #[test]
    fn choice_rechecked_when_opened() {
        let mut g = exploded_game();
        g.pending_chest = Some(vec!["gauge", "frenzy", "pt"]);
        g.crit_gauge = 1.0; // filled up on its own meanwhile
        assert!(!choose_chest(&mut g, 0));
        assert_eq!(g.pending_chest, Some(vec!["gauge", "frenzy", "pt"]));
        assert!(choose_chest(&mut g, 1));
    }

    #[test]
    fn unknown_kind_does_nothing() {
        let mut g = exploded_game();
        g.pending_chest = Some(vec!["bogus"]);
        let before = g.next_inspector;
        assert!(!choose_chest(&mut g, 0));
        assert_eq!(g.next_inspector, before); // no Bribes applied by default
        assert!(!has(&g, "chest"));
    }

    #[test]
    fn no_second_plane_while_chest_pending() {
        let mut g = exploded_game();
        (g.level, g.next_plane, g.next_inspector) = (3, g.time, 1e9);
        g.pending_chest = Some(vec!["pt", "lump", "frenzy"]);
        tick(&mut g, 0.1);
        assert!(g.plane.is_none());
        g.plane = plane_at(&g, 'a');
        assert!(!catch(&mut g, 'a'));
        assert_eq!(g.pending_chest, Some(vec!["pt", "lump", "frenzy"]));
    }

    #[test]
    fn army_never_in_pacifist() {
        let mut g = exploded_game();
        (g.challenge, g.units["soldier"]) = (Some("pacifist"), 10);
        assert!(!chest_allowed(&g, "army"));
    }

    #[test]
    fn shield_useless_with_fog() {
        let mut g = exploded_game();
        g.level = 10;
        g.mutations.push("fog");
        assert!(!chest_allowed(&g, "shield"));
    }

    #[test]
    fn blind_hides_pt() {
        let mut g = exploded_game();
        (g.level, g.challenge) = (100, Some("blind"));
        assert!(chest_text(&g, "pt").1.contains("???"));
    }

    #[test]
    fn bad_index_ignored() {
        let mut g = exploded_game();
        g.pending_chest = Some(vec!["frenzy"]);
        choose_chest(&mut g, 3);
        assert_eq!(g.pending_chest, Some(vec!["frenzy"]));
    }

    #[test]
    fn pending_chest_saved() {
        let tmp = crate::saves::TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        g.pending_chest = Some(vec!["pt", "gauge", "lump"]);
        g.save(&path).unwrap();
        assert_eq!(
            Game::load(&path).unwrap().pending_chest,
            Some(vec!["pt", "gauge", "lump"])
        );
    }

    #[test]
    fn plane_spawns_and_leaves() {
        let mut g = exploded_game();
        (g.level, g.next_plane, g.next_inspector) = (3, g.time, 1e9);
        tick(&mut g, 0.1);
        let plane = g.plane.unwrap();
        assert!(has(&g, "plane"));
        assert!(DROP_LETTERS.contains(plane.letter));
        g.time += plane.dur + 1.0;
        tick(&mut g, 0.1);
        assert!(g.plane.is_none());
    }

    #[test]
    fn drop_letters_avoid_controls() {
        for key in "fjkmqx0".chars() {
            assert!(!DROP_LETTERS.contains(key), "{key}");
        }
    }

    // --- Inspector ---

    fn inspector_game() -> Game {
        let mut g = exploded_game();
        g.next_plane = 1e9;
        g
    }

    #[test]
    fn inspector_spawn() {
        let mut g = inspector_game();
        (g.level, g.next_inspector) = (10, g.time);
        tick(&mut g, 0.1);
        assert!(g.inspector.is_some());
        assert!(has(&g, "alarm"));
    }

    #[test]
    fn fog_prevents_spawn() {
        let mut g = inspector_game();
        (g.level, g.next_inspector) = (10, g.time);
        g.mutations.push("fog");
        tick(&mut g, 0.1);
        assert!(g.inspector.is_none());
    }

    #[test]
    fn inspector_chase() {
        let mut g = inspector_game();
        g.inspector = inspector(g.time, 0);
        for _ in 0..INSPECTOR_HITS {
            chase(&mut g);
        }
        assert!(g.inspector.is_none());
        assert_eq!(g.stats.inspectors, 1);
        assert!(has(&g, "hit"));
        assert!(has(&g, "chased"));
    }

    #[test]
    fn inspector_freeze_on_arrival() {
        let mut g = inspector_game();
        g.inspector = inspector(g.time - 25.0, 0);
        tick(&mut g, 0.1);
        assert!(g.frozen());
        assert!(has(&g, "freeze"));
        assert_eq!(last_message(&g).kind, MsgKind::Bad);
    }

    #[test]
    fn guards() {
        let mut g = inspector_game();
        g.talents.insert("l_guard", 1);
        g.inspector = inspector(g.time - 10.0, 0);
        tick(&mut g, 0.1);
        assert!(g.inspector.is_none());
        assert!(!g.frozen());
    }

    // --- Challenges ---

    #[test]
    fn inflation() {
        let mut g = exploded_game();
        let base = g.unit_cost("soldier", None);
        g.challenge = Some("inflation");
        assert!(approx(g.unit_cost("soldier", None), base * 4.0));
        g.challenge = None;
        g.challenges_done.insert("inflation");
        assert!(approx(g.unit_cost("soldier", None), base * 0.9));
    }

    #[test]
    fn blind_reward_extra_mutation() {
        let mut g = exploded_game();
        g.challenges_done.insert("blind");
        offer_mutation(&mut g);
        assert_eq!(g.pending_mutation.unwrap().len(), 4);
    }

    #[test]
    fn watched_frequent_inspectors_ignore_fog_and_guards() {
        let mut g = exploded_game();
        (g.challenge, g.level, g.next_plane) = (Some("watched"), 10, 1e9);
        g.mutations.push("fog");
        g.talents.insert("l_guard", 1);
        g.schedule_inspector(false);
        assert!((30.0..=60.0).contains(&(g.next_inspector - g.time)));
        g.next_inspector = g.time;
        tick(&mut g, 0.1);
        assert!(g.inspector.is_some());
        g.inspector.as_mut().unwrap().start = g.time - 10.0; // past the 30 % where the guards would step in
        tick(&mut g, 0.1);
        assert!(g.inspector.is_some());
    }

    #[test]
    fn watched_start_of_run() {
        let mut g = exploded_game();
        g.challenge = Some("watched");
        g.reset_run();
        assert!(g.next_inspector - g.time <= 60.0);
    }

    #[test]
    fn watched_reward() {
        let mut g = exploded_game();
        g.inspector = inspector(g.time, INSPECTOR_HITS - 1);
        chase(&mut g);
        let normal = g.radiation;
        let mut h = exploded_game();
        h.challenges_done.insert("watched");
        h.inspector = inspector(h.time, INSPECTOR_HITS - 1);
        chase(&mut h);
        assert!(approx(h.radiation, normal * 3.0));
    }

    #[test]
    fn every_challenge_skin_exists() {
        // The palettes are checked by the art module.
        for c in &data::CHALLENGES {
            assert!(data::find_skin(c.skin).is_some(), "{}", c.skin);
        }
    }

    #[test]
    fn master_achievement() {
        let mut g = exploded_game();
        g.challenges_done = data::CHALLENGES.iter().map(|c| c.id).collect();
        g.check_achievements();
        assert!(g.achievements.contains("challenges_all"));
    }

    // --- Critical mass ---

    fn crit_game() -> Game {
        let mut g = exploded_game();
        g.crit_gauge = 1.0;
        g
    }

    fn start_qte(g: &mut Game, offset: f64) {
        crit_press(g);
        let q = g.qte.as_mut().unwrap();
        q.t = (q.center + offset) / q.speed;
        crit_press(g);
    }

    #[test]
    fn crit_start() {
        let mut g = crit_game();
        crit_press(&mut g);
        assert!(g.qte.is_some());
        assert!(has(&g, "charge"));
    }

    #[test]
    fn crit_not_ready() {
        let mut g = crit_game();
        g.crit_gauge = 0.5;
        crit_press(&mut g);
        assert!(g.qte.is_none());
    }

    #[test]
    fn crit_perfect() {
        let mut g = crit_game();
        start_qte(&mut g, 0.0);
        assert_eq!(g.stats.perfects, 1);
        assert!(has(&g, "perfect"));
        assert_eq!(g.crit_gauge, 0.0);
        assert_eq!(g.qte_result.unwrap().color, "green");
    }

    #[test]
    fn crit_partial() {
        let mut g = crit_game();
        start_qte(&mut g, 0.1);
        assert_eq!(g.qte_result.as_ref().unwrap().color, "yellow");
        assert!(has(&g, "partial"));
    }

    #[test]
    fn crit_timeout() {
        let mut g = crit_game();
        crit_press(&mut g);
        tick(&mut g, 7.0);
        assert!(g.qte.is_none());
        assert_eq!(g.qte_result.as_ref().unwrap().color, "red");
        assert!(has(&g, "fizzle"));
    }

    #[test]
    fn cursor_bounces() {
        let q = Qte {
            t: 1.5,
            center: 0.5,
            green: 0.06,
            yellow: 0.17,
            speed: 1.0,
        };
        assert!(approx(qte_pos(&q), 0.5));
        let q = Qte { t: 0.25, ..q };
        assert!(approx(qte_pos(&q), 0.25));
    }

    // --- Mutations ---

    #[test]
    fn milestones() {
        let levels: Vec<u64> = (0..6).map(mutation_milestone).collect();
        assert_eq!(levels, [25, 50, 75, 100, 200, 400]);
        assert_eq!(mutation_milestone(200), u64::MAX);
    }

    #[test]
    fn offer_and_choose() {
        let mut g = exploded_game();
        offer_mutation(&mut g);
        assert_eq!(g.pending_mutation.as_ref().unwrap().len(), 3);
        assert!(has(&g, "mutation"));
        let mid = g.pending_mutation.as_ref().unwrap()[1];
        choose_mutation(&mut g, 1);
        assert_eq!(g.mutations, [mid]);
        assert!(g.pending_mutation.is_none());
        assert_eq!(last_message(&g).kind, MsgKind::Good);
    }

    #[test]
    fn mutation_queue() {
        let mut g = exploded_game();
        g.mut_queue = 1;
        offer_mutation(&mut g);
        choose_mutation(&mut g, 0);
        assert!(g.pending_mutation.is_some());
        assert_eq!(g.mut_queue, 0);
        assert!(!g.pending_mutation.unwrap().contains(&g.mutations[0]));
    }

    #[test]
    fn level_triggers_mutation() {
        let mut g = exploded_game();
        g.level = 24;
        g.gain(need(g.level));
        assert_eq!(g.level, 25);
        assert!(g.pending_mutation.is_some());
    }

    // --- Synergies ---

    fn pick(g: &mut Game, mid: &'static str) {
        g.pending_mutation = Some(vec![mid]);
        choose_mutation(g, 0);
    }

    #[test]
    fn each_mutation_in_exactly_one_pair() {
        let mut used: Vec<&str> = SYNERGIES
            .iter()
            .flat_map(|s| [s.pair.0, s.pair.1])
            .collect();
        let mut all: Vec<&str> = MUTATIONS.iter().map(|m| m.id).collect();
        used.sort();
        all.sort();
        assert_eq!(used, all);
    }

    #[test]
    fn synergy_discovery() {
        let mut g = exploded_game();
        pick(&mut g, "spores");
        assert!(!g.has_syn("proliferation"));
        assert!(synergy_completed_by(&g, "mitosis").is_some());
        pick(&mut g, "mitosis");
        assert!(g.has_syn("proliferation"));
        assert!(g.synergies_found.contains("proliferation"));
        assert!(has(&g, "synergy"));
        let m = last_message(&g);
        assert_eq!(
            (m.kind, m.title.as_deref()),
            (MsgKind::Epic, Some("SYNERGIE"))
        );
        assert!(m.text.contains("découverte"));
        g.check_achievements();
        assert!(g.achievements.contains("synergy1"));
    }

    #[test]
    fn rediscovery_not_new() {
        let mut g = exploded_game();
        g.synergies_found.insert("storm");
        pick(&mut g, "acid");
        pick(&mut g, "fallout");
        assert!(!last_message(&g).text.contains("découverte"));
    }

    #[test]
    fn synergy_effects() {
        let mut g = exploded_game();
        g.units["soldier"] = 100;
        g.mutations = vec!["spores"];
        let base = g.unit_prod("soldier");
        g.mutations.push("mitosis");
        assert!(approx(g.unit_prod("soldier"), base * 3.0 / 2.0)); // ×(1+2) instead of ×(1+1)
        let mut g = exploded_game();
        g.mutations = vec!["acid"];
        let halved = g.click_value(false);
        g.mutations.push("fallout");
        assert!(approx(g.click_value(false), halved * 2.0));
        let mut g = exploded_game();
        g.mutations = vec!["twin"];
        g.units["soldier"] = 10;
        let p = g.production();
        g.mutations.push("gamble");
        assert!(approx(g.production(), p * 3.0 * 1.5));
    }

    #[test]
    fn chain_charges_gauge() {
        let mut g = exploded_game_with(fixed(0.0));
        g.mutations = vec!["unstable", "glow"];
        g.click(true);
        assert!(approx(g.crit_gauge, 0.03));
    }

    #[test]
    fn berserker_slow_decay() {
        let mut g = exploded_game();
        g.mutations = vec!["berserk", "asbestos"];
        (g.combo, g.last_click) = (2.0, -10.0);
        g.tick(1.0);
        assert!(approx(g.combo, 1.6));
    }

    #[test]
    fn smuggling_four_choices() {
        let mut g = exploded_game();
        g.mutations = vec!["magnet", "fog"];
        g.units["soldier"] = 1;
        g.plane = plane_at(&g, 'a');
        catch(&mut g, 'a');
        assert_eq!(g.pending_chest.unwrap().len(), 4);
    }

    // --- News ---

    #[test]
    fn inspector_news() {
        let mut g = exploded_game();
        g.inspector = inspector(0.0, 0);
        assert_eq!(pick_news(&mut g), NEWS_INSPECTOR);
    }

    #[test]
    fn news_unlocked_by_level() {
        let mut g = exploded_game();
        let allowed: Vec<&str> = NEWS
            .iter()
            .filter(|(lvl, _)| *lvl <= g.level)
            .map(|(_, t)| *t)
            .collect();
        for _ in 0..20 {
            assert!(allowed.contains(&pick_news(&mut g)));
        }
    }

    #[test]
    fn enemy_news() {
        let mut g = exploded_game();
        g.enemies = vec![enemy(EnemyKind::Drone, 0.0, 15.0)];
        assert_eq!(pick_news(&mut g), EnemyKind::Drone.spec().news);
    }

    // --- Enemies ---

    fn enemy_game() -> Game {
        let mut g = exploded_game();
        (g.next_inspector, g.next_plane) = (1e9, 1e9);
        g
    }

    fn spawn(kind: EnemyKind, level: u32) -> Game {
        let mut g = enemy_game();
        g.level = level;
        g.enemies = vec![enemy(kind, g.time, enemy_duration(&g, kind))];
        g
    }

    #[test]
    fn spawn_respects_level() {
        let mut g = enemy_game();
        (g.level, g.next_enemy) = (5, g.time);
        g.tick(0.1);
        assert!(g.enemies.is_empty()); // too early: no enemy available, reschedule
        assert!(g.next_enemy > g.time);
        (g.level, g.next_enemy) = (15, g.time);
        g.tick(0.1);
        assert_eq!(
            g.enemies.iter().map(|e| e.kind).collect::<Vec<_>>(),
            [EnemyKind::Saboteur]
        );
        assert_eq!(last_message(&g).title.as_deref(), Some("ALERTE"));
    }

    #[test]
    fn chase_inspector_first_then_enemy() {
        let mut g = spawn(EnemyKind::Saboteur, 100);
        g.inspector = inspector(g.time, 0);
        for _ in 0..INSPECTOR_HITS {
            chase(&mut g);
        }
        assert!(g.inspector.is_none());
        assert_eq!(g.enemies[0].hits, 0);
        for _ in 0..EnemyKind::Saboteur.spec().hits {
            chase(&mut g);
        }
        assert!(g.enemies.is_empty());
        assert_eq!(g.stats.enemies, 1);
        assert!(g.radiation > 0.0); // loot
    }

    #[test]
    fn saboteur_steals() {
        let mut g = spawn(EnemyKind::Saboteur, 100);
        g.radiation = 1000.0;
        g.tick(EnemyKind::Saboteur.spec().dur + 0.1);
        assert!(g.enemies.is_empty());
        assert!(g.radiation < 1000.0);
        assert!(g.radiation >= 850.0);
    }

    #[test]
    fn hippies_halve_production() {
        let mut g = spawn(EnemyKind::Hippies, 100);
        (g.units["soldier"], g.talents) = (10, [("u_soldier", 1)].into());
        let with_them = g.production();
        g.enemies.clear();
        assert!(approx(with_them, g.production() / 2.0));
    }

    #[test]
    fn hippies_leave() {
        let mut g = spawn(EnemyKind::Hippies, 100);
        g.tick(EnemyKind::Hippies.spec().dur + 0.1);
        assert!(g.enemies.is_empty());
        assert!(has(&g, "tick"));
        assert_eq!(g.stats.enemies, 0);
    }

    #[test]
    fn commando_destroys_units() {
        let mut g = spawn(EnemyKind::Commando, 100);
        (g.units["soldier"], g.units["tank"]) = (50, 5);
        g.tick(EnemyKind::Commando.spec().dur + 0.1);
        assert_eq!((g.units["soldier"], g.units["tank"]), (45, 5));
    }

    #[test]
    fn drone_calls_inspector() {
        let mut g = spawn(EnemyKind::Drone, 100);
        g.crit_gauge = 0.5;
        g.tick(EnemyKind::Drone.spec().dur + 0.1);
        assert_eq!(g.crit_gauge, 0.0);
        assert!(g.next_inspector <= g.time + 5.0);
    }

    #[test]
    fn drone_respects_bribe() {
        let mut g = spawn(EnemyKind::Drone, 100);
        g.pending_chest = Some(vec!["shield"]);
        choose_chest(&mut g, 0);
        let before = g.next_inspector;
        g.enemies = vec![enemy(EnemyKind::Drone, g.time, 1.0)];
        g.tick(1.1);
        assert_eq!(g.next_inspector, before);
    }

    #[test]
    fn enemy_delays() {
        let mut g = enemy_game();
        g.schedule_enemy(true);
        assert!((60.0..=120.0).contains(&(g.next_enemy - g.time)));
        g.schedule_enemy(false);
        assert!((90.0..=180.0).contains(&(g.next_enemy - g.time)));
    }

    #[test]
    fn threats_grow_beyond_level_100() {
        let mut g = enemy_game();
        let mut pressures = Vec::new();
        for level in [50, 100, 1_000, 10_000, 1_000_000] {
            g.level = level;
            pressures.push((threat_pressure(&g) * 1e6).round() / 1e6);
        }
        assert_eq!(pressures, [1.0, 1.0, 1.5, 2.0, 3.0]);
        g.schedule_enemy(false);
        assert!((30.0..=60.0).contains(&(g.next_enemy - g.time)));
        g.schedule_inspector(false);
        assert!((50.0..=100.0).contains(&(g.next_inspector - g.time)));
        // Hits grow too, but no more than ×2 so the counter stays readable.
        assert_eq!(enemy_hits(&g, EnemyKind::Commando), 20);
        assert_eq!(inspector_hits(&g), 10);
        g.level = 1_000;
        assert_eq!(enemy_hits(&g, EnemyKind::Commando), 15);
    }

    #[test]
    fn effects_grow_beyond_level_100() {
        let mut g = enemy_game();
        let mut effects = Vec::new();
        for level in [100, 1_000, 10_000, 1_000_000] {
            g.level = level;
            effects.push((
                stolen_percent(&g),
                sabotaged_percent(&g),
                protest_divisor(&g),
                inspector_freeze(&g),
            ));
        }
        assert_eq!(
            effects,
            [
                (15, 10, 2.0, 20),
                (23, 15, 3.0, 30),
                (30, 20, 4.0, 40),
                (45, 30, 6.0, 60)
            ]
        );
        g.level = 10_000;
        assert!(enemy_desc(&g, EnemyKind::Saboteur).contains("vole 30 %"));
        g.level = 3_162; // pressure ≈ 1.75
        assert!(enemy_desc(&g, EnemyKind::Hippies).contains("divisée par 3,5 "));
        g.level = 100;
        assert!(enemy_desc(&g, EnemyKind::Hippies).contains("divisée par 2 "));
        assert_eq!(
            enemy_desc(&g, EnemyKind::Drone),
            EnemyKind::Drone.spec().desc
        );
    }

    #[test]
    fn late_saboteur_and_commando_hit_harder() {
        let mut g = spawn(EnemyKind::Saboteur, 10_000);
        g.radiation = 1000.0;
        g.tick(EnemyKind::Saboteur.spec().dur + 0.1);
        assert!(g.radiation >= 700.0);
        assert!(g.radiation < 701.0);
        let mut g = spawn(EnemyKind::Commando, 10_000);
        g.units["soldier"] = 100;
        g.tick(EnemyKind::Commando.spec().dur + 0.1);
        assert_eq!(g.units["soldier"], 80);
    }

    #[test]
    fn late_protesters_and_inspector() {
        let mut g = spawn(EnemyKind::Hippies, 10_000);
        g.units["soldier"] = 10;
        let with_them = g.production();
        g.enemies.clear();
        assert!(approx(with_them, g.production() / 4.0));
        g.inspector = inspector(g.time - 20.0, 0);
        tick(&mut g, 0.1);
        assert!(approx(g.freeze_until - g.time, 40.0));
    }

    #[test]
    fn chase_needs_more_hits_late() {
        let mut g = spawn(EnemyKind::Saboteur, 10_000);
        for _ in 0..EnemyKind::Saboteur.spec().hits {
            chase(&mut g);
        }
        assert_eq!(g.enemies.len(), 1);
        for _ in 0..EnemyKind::Saboteur.spec().hits {
            chase(&mut g);
        }
        assert!(g.enemies.is_empty());
    }

    #[test]
    fn invasion() {
        let mut g = enemy_game();
        g.challenge = Some("invasion");
        g.schedule_enemy(false);
        assert!((15.0..=30.0).contains(&(g.next_enemy - g.time)));
        (g.level, g.next_enemy) = (1, g.time);
        g.tick(0.1);
        assert!(!g.enemies.is_empty()); // every type from the start
        g.challenges_done.insert("invasion");
        assert_eq!(
            enemy_duration(&g, EnemyKind::Saboteur),
            EnemyKind::Saboteur.spec().dur * 1.5
        );
    }

    #[test]
    fn invasion_waves() {
        let mut g = enemy_game();
        (g.challenge, g.level) = (Some("invasion"), 50);
        assert_eq!(
            (1..7).map(wave_size).collect::<Vec<_>>(),
            [1, 2, 2, 3, 3, 3]
        );
        let mut sizes = Vec::new();
        for _ in 0..5 {
            (g.enemies, g.next_enemy) = (Vec::new(), g.time);
            g.tick(0.01);
            sizes.push(g.enemies.len());
        }
        assert_eq!(sizes, [1, 2, 2, 3, 3]);
        assert_eq!(g.wave, 5);
        assert_eq!(last_message(&g).title.as_deref(), Some("VAGUE 5"));
        let starts: Vec<f64> = g
            .enemies
            .iter()
            .map(|e| e.start - g.enemies[0].start)
            .collect();
        assert_eq!(starts, [0.0, WAVE_GAP, 2.0 * WAVE_GAP]); // staggered entries
    }

    #[test]
    fn chase_hits_most_advanced_and_next_wave_waits() {
        let mut g = enemy_game();
        g.level = 50;
        let late = enemy(EnemyKind::Saboteur, g.time, 12.0);
        let early = enemy(EnemyKind::Saboteur, g.time - 6.0, 12.0);
        g.enemies = vec![late, early];
        for _ in 0..3 {
            chase(&mut g);
        }
        assert_eq!(g.enemies, [late]); // the most advanced one fell first
        g.next_enemy = g.time; // deadline passed, but an enemy is still there: no new enemy
        g.tick(0.01);
        assert_eq!(g.enemies.len(), 1);
        for _ in 0..3 {
            chase(&mut g);
        }
        assert!(g.enemies.is_empty());
        assert!(g.next_enemy > g.time); // the next one is scheduled once the field is clear
    }

    #[test]
    fn two_hippies_do_not_stack() {
        let mut g = spawn(EnemyKind::Hippies, 100);
        (g.units["soldier"], g.talents) = (10, [("u_soldier", 1)].into());
        let one = g.production();
        g.enemies.push(g.enemies[0]);
        assert!(approx(g.production(), one));
    }

    #[test]
    fn every_enemy_has_spec_and_news() {
        // The sprites are checked by the art module.
        for kind in EnemyKind::ALL {
            assert_eq!(kind.spec().kind, kind);
            assert_eq!(EnemyKind::from_id(kind.id()), Some(kind));
            assert!(!kind.spec().news.is_empty());
        }
    }

    // --- Other challenges ---

    #[test]
    fn ascete_only_unit_unlocks() {
        let mut g = exploded_game();
        (g.challenge, g.pt) = (Some("ascete"), 50);
        assert!(!g.buy_talent("t_click"));
        assert!(g.buy_talent("u_soldier"));
        g.auto_talents(); // doesn't loop forever on forbidden talents
        assert_eq!(g.tr("t_click"), 0);
        g.challenges_done.insert("ascete");
        g.challenge = None;
        g.reset_run();
        assert_eq!(g.pt, 10);
    }

    #[test]
    fn hothead() {
        let mut g = exploded_game();
        g.challenge = Some("hothead");
        g.mutations.push("asbestos");
        for _ in 0..7 {
            g.click(true);
        }
        assert!(g.overheated());
        assert!(approx(g.overheat_until - g.time, 6.0));
        g.challenge = None;
        g.challenges_done.insert("hothead");
        assert_eq!(g.overheat_threshold(), 17);
    }

    #[test]
    fn sterile() {
        let mut g = exploded_game();
        g.challenge = Some("sterile");
        offer_mutation(&mut g);
        assert!(g.pending_mutation.is_none());
        (g.challenge, g.crit_gauge) = (None, 0.0);
        g.challenges_done.insert("sterile");
        (g.next_plane, g.next_inspector, g.next_enemy) = (1e9, 1e9, 1e9);
        g.tick(1.0);
        assert!(approx(g.crit_gauge, 1.5 / 90.0));
    }

    #[test]
    fn unlucky() {
        let mut g = exploded_game();
        (g.challenge, g.level, g.next_plane) = (Some("unlucky"), 10, g.time);
        g.talents.insert("t_crit", 5);
        assert_eq!(g.crit_chance(), 0.0);
        g.tick(1.0);
        assert_eq!(g.crit_gauge, 0.0);
        assert!(g.plane.is_none());
        g.challenge = None;
        g.challenges_done.insert("unlucky");
        assert!(approx(g.crit_chance(), 0.35));
    }

    #[test]
    fn nanar_fizzles_and_halves_production() {
        let mut g = exploded_game_with(fixed(0.2)); // 0.2 < 1/3: fizzle
        g.units["soldier"] = 10;
        let full = g.production();
        g.challenge = Some("nanar");
        assert!(approx(g.production(), full / 2.0));
        let before = g.radiation + g.progress;
        g.click(true);
        assert_eq!(g.radiation + g.progress, before);
        assert!(has(&g, "fizzle"));
        g.sfx.clear();
        g.click(false); // the auto-detonator misfires too, but silently
        assert!(!has(&g, "fizzle"));
        g.rng = fixed(0.5);
        g.click(true);
        assert!(g.radiation + g.progress > before);
    }

    #[test]
    fn nanar_reward_and_news() {
        let mut g = exploded_game();
        assert_eq!(g.crit_mult(), 10.0);
        g.challenges_done.insert("nanar");
        assert_eq!(g.crit_mult(), 15.0);
        g.challenge = Some("nanar");
        let picked: Vec<&str> = (0..50).map(|_| pick_news(&mut g)).collect();
        assert!(picked.iter().any(|n| NEWS_NANAR.contains(n)));
    }

    // --- Sounds ---

    #[test]
    fn sounds_are_pushed_literally() {
        // The sound tests scan the sources for literal pushes: every emission must use that form.
        for file in [include_str!("events.rs"), include_str!("game.rs")] {
            let source = file.split("#[cfg(test)]").next().unwrap();
            let pushes = source.matches("sfx.push_back(").count();
            let literal = source.matches("sfx.push_back(\"").count();
            let clicks = source.matches("sfx.push_back(CLICK_SOUNDS[").count();
            assert_eq!(pushes, literal + clicks);
        }
    }
}
