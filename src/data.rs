//! Static definitions: units, run talents, prestige talents, challenges, skins.

use std::sync::LazyLock;

pub const BASE_CAP: u32 = 100;
pub const MAX_CAP: u32 = 1_000_000;

/// Unit bought with radiation. Each one is unlocked by a "u_<id>" talent
/// and boosted (×2 per rank) by the "m_<id>" talent.
#[derive(Debug)]
pub struct Unit {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub cost: f64,
    pub prod: f64,
    pub proj: &'static str,
    pub color: &'static str,
    pub unlock_name: &'static str,
    pub unlock_cost: u32,
    pub mult_name: &'static str,
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
const fn unit_def(
    id: &'static str,
    name: &'static str,
    icon: &'static str,
    cost: f64,
    prod: f64,
    proj: &'static str,
    color: &'static str,
    unlock_name: &'static str,
    unlock_cost: u32,
    mult_name: &'static str,
) -> Unit {
    Unit { id, name, icon, cost, prod, proj, color, unlock_name, unlock_cost, mult_name }
}

#[rustfmt::skip]
pub const UNITS: [Unit; 9] = [
    unit_def("soldier", "Soldat", "i", 15.0, 0.5, "o", "green", "Conscription", 1, "Grenades à fragmentation"),
    unit_def("mortar", "Mortier", "y", 150.0, 4.0, "*", "yellow", "Mortiers", 3, "Obus explosifs"),
    unit_def("artillery", "Artilleur", "F", 1_500.0, 30.0, "°", "orange", "Artillerie lourde", 5, "Obus perforants"),
    unit_def("tank", "Tank", "T", 15_000.0, 220.0, "=", "green", "Divisions blindées", 8, "Canons de 120"),
    unit_def("bomber", "Bombardier", ">", 160_000.0, 1_600.0, "v", "cyan", "Escadrilles", 12, "Bombes au napalm"),
    unit_def("sub", "Sous-marin nucléaire", "D", 1.8e6, 12_000.0, "^", "blue", "Flotte sous-marine", 16,
             "Torpilles atomiques"),
    unit_def("icbm", "Silo ICBM", "!", 2e7, 90_000.0, "!", "red", "Silos ICBM", 20, "Têtes multiples"),
    unit_def("reactor", "Réacteur à fusion", "@", 2.3e8, 700_000.0, "+", "magenta", "Réacteurs à fusion", 25,
             "Plasma confiné"),
    unit_def("blackhole", "Trou noir de poche", "O", 2.6e9, 5.5e6, "·", "white", "Singularité", 30,
             "Horizon des événements"),
];
pub const UNIT_COUNT: usize = UNITS.len();
pub const UNIT_COST_GROWTH: f64 = 1.15;
/// Milestones: each threshold reached multiplies the unit's production.
pub const UNIT_MILESTONES: [u64; 6] = [50, 100, 200, 300, 400, 500];
pub const MILESTONE_MULT: f64 = 2.0;

pub fn unit_index(id: &str) -> Option<usize> {
    UNITS.iter().position(|u| u.id == id)
}

pub fn find_unit(id: &str) -> Option<&'static Unit> {
    unit_index(id).map(|i| &UNITS[i])
}

/// Unit by id (panics on an unknown id, like the Python dict lookup).
pub fn unit(id: &str) -> &'static Unit {
    find_unit(id).unwrap_or_else(|| panic!("unknown unit {id}"))
}

/// Run talent. req: [(required talent id, minimum rank)]. Cost = cost.0 + cost.1 * current rank, in PT.
#[derive(Debug)]
pub struct Talent {
    pub id: &'static str,
    pub name: &'static str,
    pub branch: &'static str,
    pub depth: u32,
    pub max: u32,
    pub cost: (i64, i64),
    pub desc: &'static str,
    pub req: Vec<(&'static str, u32)>,
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
fn talent_def(
    id: &'static str,
    name: &'static str,
    branch: &'static str,
    depth: u32,
    max: u32,
    cost: (i64, i64),
    desc: &'static str,
    req: &[(&'static str, u32)],
) -> Talent {
    Talent { id, name, branch, depth, max, cost, desc, req: req.to_vec() }
}

/// Generated strings live as long as the program (the tables are built once).
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// For each unit, in order: the talent that unlocks it (after the previous one) and the one that boosts it.
fn unit_talents() -> Vec<Talent> {
    let mut talents = Vec::new();
    let mut previous: Option<&str> = None;
    for unit in &UNITS {
        let unlock_id = leak(format!("u_{}", unit.id));
        let unlock_req: Vec<(&'static str, u32)> = match previous {
            Some(p) => vec![(leak(format!("u_{p}")), 1)],
            None => vec![],
        };
        talents.push(talent_def(
            unlock_id,
            unit.unlock_name,
            "inf",
            0,
            1,
            (unit.unlock_cost as i64, 0),
            leak(format!("Débloque l'unité : {}.", unit.name)),
            &unlock_req,
        ));
        talents.push(talent_def(
            leak(format!("m_{}", unit.id)),
            unit.mult_name,
            "inf",
            1,
            3,
            ((unit.unlock_cost as i64 / 2).max(2), 1),
            leak(format!("Production des {}s ×2 par rang.", unit.name)),
            &[(unlock_id, 1)],
        ));
        previous = Some(unit.id);
    }
    talents
}

#[rustfmt::skip]
fn build_talents() -> Vec<Talent> {
    let mut talents = vec![
        talent_def("t_click", "Détonateur renforcé", "det", 0, 10, (1, 1),
                   "+2 de puissance de détonation de base par rang.", &[]),
        talent_def("t_crit", "Fission critique", "det", 1, 5, (2, 1),
                   "+5 % de chance de détonation critique par rang.", &[("t_click", 1)]),
        talent_def("t_combo", "Réaction en chaîne", "det", 1, 2, (2, 2), "Combo max +1 par rang (×3 → ×5).",
                   &[("t_click", 1)]),
        talent_def("t_cool", "Refroidissement", "det", 2, 3, (2, 1), "Seuil de surchauffe +3 appuis/s par rang.",
                   &[("t_combo", 1)]),
        talent_def("t_auto", "Auto-détonateur", "det", 1, 5, (3, 2),
                   "Appuie tout seul sur ESPACE : 1 fois/s par rang.", &[("t_click", 2)]),
        talent_def("t_shock", "Onde de choc", "det", 1, 5, (3, 2),
                   "Chaque appui ajoute 2 % de la production/s par rang.", &[("t_click", 3)]),
    ];
    talents.extend(unit_talents());
    talents.extend([
        talent_def("l_global", "Propagande", "log", 0, 10, (2, 1), "Production globale +10 % par rang.", &[]),
        talent_def("l_synergy", "Esprit de corps", "log", 1, 3, (4, 2),
                   "+0,1 % de production par unité possédée, par rang.", &[("l_global", 2)]),
        talent_def("l_guard", "Gardes", "log", 1, 1, (4, 0),
                   "Des gardes chassent automatiquement les inspecteurs de l'ONU.", &[("l_global", 1)]),
        talent_def("l_discount", "Marché noir", "log", 0, 5, (2, 1), "Coût des unités -4 % par rang.", &[]),
        talent_def("l_autobuy", "Quartier-maître", "log", 1, 3, (5, 3),
                   "Achète automatiquement les unités (3 types de plus par rang).", &[("l_discount", 1)]),
        talent_def("l_radar", "Radar", "log", 0, 3, (2, 1),
                   "Ravitaillements +30 % plus fréquents et avions plus lents.", &[]),
        talent_def("l_enrich", "Enrichissement", "log", 0, 3, (3, 1),
                   "La masse critique se charge 25 % plus vite par rang.", &[]),
    ]);
    talents
}

/// Run talents, paid in PT.
pub static TALENTS: LazyLock<Vec<Talent>> = LazyLock::new(build_talents);

/// Ids of the talents that unlock ("u_<id>") and boost ("m_<id>") each unit, in UNITS order.
pub static UNIT_TALENT_IDS: LazyLock<[(&'static str, &'static str); UNIT_COUNT]> =
    LazyLock::new(|| {
        std::array::from_fn(|i| {
            let id = UNITS[i].id;
            (talent(&format!("u_{id}")).id, talent(&format!("m_{id}")).id)
        })
    });

pub fn find_talent(id: &str) -> Option<&'static Talent> {
    TALENTS.iter().find(|t| t.id == id)
}

pub fn talent(id: &str) -> &'static Talent {
    find_talent(id).unwrap_or_else(|| panic!("unknown talent {id}"))
}

pub const BRANCHES: [(&str, &str); 3] = [
    ("det", "DÉTONATION"),
    ("inf", "INFANTERIE"),
    ("log", "LOGISTIQUE"),
];
/// Talents kept on reset with "Automates persistants".
pub const KEPT_TALENTS: [&str; 3] = ["t_auto", "l_autobuy", "l_discount"];

/// Permanent talent (prestige ⚛ or supernova ✦).
/// Cost = round(base * growth^rank) or costs[rank].
#[derive(Debug)]
pub struct Perk {
    pub id: &'static str,
    pub name: &'static str,
    pub max: u32,
    pub desc: &'static str,
    pub base: f64,
    pub growth: f64,
    pub costs: Option<&'static [i64]>,
}

const fn perk(
    id: &'static str,
    name: &'static str,
    max: u32,
    desc: &'static str,
    base: f64,
    growth: f64,
) -> Perk {
    Perk {
        id,
        name,
        max,
        desc,
        base,
        growth,
        costs: None,
    }
}

#[rustfmt::skip]
const fn perk_costs(id: &'static str, name: &'static str, max: u32, desc: &'static str, costs: &'static [i64]) -> Perk {
    Perk { id, name, max, desc, base: 1.0, growth: 1.5, costs: Some(costs) }
}

/// Prestige talents, paid in isotopes.
#[rustfmt::skip]
pub const PTALENTS: [Perk; 10] = [
    perk_costs("p_cap", "Expansion du cœur", 4, "Niveau max ×10 (100 → 1 000 000).", &[20, 150, 1_200, 9_000]),
    perk("p_prod", "Uranium appauvri", 25, "Production ×1,5 par rang (multiplicatif).", 2.0, 1.45),
    perk("p_click", "Doigt nucléaire", 15, "Puissance de détonation ×2 par rang.", 1.0, 1.5),
    perk("p_pt", "Mémoire génétique", 10, "Commence chaque run avec +5 PT par rang.", 3.0, 1.6),
    perk("p_vets", "Vétérans", 5, "Commence avec la Conscription et 10 soldats par rang.", 4.0, 2.0),
    perk_costs("p_keep", "Automates persistants", 1,
               "Auto-détonateur, Quartier-maître et Marché noir sont conservés au reset.", &[15]),
    perk("p_iso", "Centrifugeuses", 10, "+25 % d'isotopes gagnés par rang.", 5.0, 1.7),
    perk("p_drop", "Contrebandiers", 3, "Ravitaillements +50 % plus efficaces par rang.", 5.0, 2.0),
    perk("p_offline", "Sommeil radioactif", 4, "Progression hors-ligne : 25 % de la production par rang (2 h max).",
         4.0, 2.0),
    perk("p_dna", "ADN instable", 2, "+1 mutation proposée au choix par rang.", 20.0, 3.0),
];

pub fn find_ptalent(id: &str) -> Option<&'static Perk> {
    PTALENTS.iter().find(|p| p.id == id)
}

pub fn ptalent(id: &str) -> &'static Perk {
    find_ptalent(id).unwrap_or_else(|| panic!("unknown prestige talent {id}"))
}

/// Supernova talents, paid in stars (✦). Never reset.
#[rustfmt::skip]
pub const STALENTS: [Perk; 5] = [
    perk("s_prod", "Fusion stellaire", 10, "Production ×3 par rang.", 1.0, 2.0),
    perk("s_iso", "Nucléosynthèse", 10, "Isotopes gagnés ×2 par rang.", 1.0, 2.0),
    perk_costs("s_core", "Cœur dense", 4, "Expansion du cœur coûte 20 % de moins par rang.", &[2, 5, 10, 20]),
    perk("s_crit", "Pulsar", 3, "La masse critique se charge 2× plus vite par rang.", 3.0, 2.0),
    perk_costs("s_auto", "Chaîne de montage", 1, "Achète automatiquement les talents de run.", &[5]),
];

pub fn find_stalent(id: &str) -> Option<&'static Perk> {
    STALENTS.iter().find(|p| p.id == id)
}

pub fn stalent(id: &str) -> &'static Perk {
    find_stalent(id).unwrap_or_else(|| panic!("unknown supernova talent {id}"))
}

pub const SUPERNOVA_DIVISOR: f64 = 500.0;

#[derive(Debug)]
pub struct Challenge {
    pub id: &'static str,
    pub name: &'static str,
    pub target: u32,
    /// Time limit in seconds (None: untimed).
    pub time: Option<f64>,
    pub skin: &'static str,
    pub desc: &'static str,
    pub reward: &'static str,
}

#[rustfmt::skip]
const fn challenge_def(
    id: &'static str,
    name: &'static str,
    target: u32,
    time: Option<f64>,
    skin: &'static str,
    desc: &'static str,
    reward: &'static str,
) -> Challenge {
    Challenge { id, name, target, time, skin, desc, reward }
}

#[rustfmt::skip]
pub const CHALLENGES: [Challenge; 12] = [
    challenge_def("pacifist", "Pacifiste", 60, None, "toxic", "Aucune unité autorisée. Atteins le niveau 60.",
                  "Détonation ×3 permanente + skin Vert radioactif"),
    challenge_def("onearm", "Manchot", 100, None, "rainbow",
                  "Ni ESPACE ni auto-détonateur (3 soldats offerts). Atteins le niveau 100.",
                  "Production ×2 permanente + skin Arc-en-ciel"),
    challenge_def("race", "Course", 100, Some(600.0), "mario", "Atteins le niveau 100 en moins de 10 minutes.",
                  "Isotopes ×1,5 permanent + skin Champignon de Mario"),
    challenge_def("inflation", "Inflation", 100, None, "gold",
                  "Les unités coûtent 4 fois plus cher. Atteins le niveau 100.",
                  "Unités -10 % permanent + skin Or"),
    challenge_def("blind", "Brouillard de guerre", 100, None, "ghost",
                  "Niveau, radiation et production sont masqués. Atteins le niveau 100.",
                  "+1 mutation proposée permanente + skin Fantôme"),
    challenge_def("watched", "Sous surveillance", 100, None, "neon",
                  "Un inspecteur toutes les 30 à 60 s, gardes et brouillard inefficaces. Atteins le niveau 100.",
                  "Inspecteurs chassés ×3 de butin + skin Néon"),
    challenge_def("ascete", "Ascète", 100, None, "ice",
                  "Seuls les talents qui débloquent des unités sont autorisés. Atteins le niveau 100.",
                  "+10 PT au début de chaque run + skin Glace"),
    challenge_def("hothead", "Tête brûlée", 100, None, "lava",
                  "Surchauffe dès 6 appuis/s, pendant 6 s, même avec les Doigts d'amiante. Atteins le niveau 100.",
                  "Seuil de surchauffe +5 appuis/s permanent + skin Lave"),
    challenge_def("invasion", "Invasion", 100, None, "camo",
                  "Un ennemi toutes les 15 à 30 s, tous types dès le début. Atteins le niveau 100.",
                  "Ennemis 50 % plus lents permanent + skin Camouflage"),
    challenge_def("sterile", "Stérile", 100, None, "crystal",
                  "Aucune mutation ne t'est proposée. Atteins le niveau 100.",
                  "Masse critique 50 % plus rapide permanente + skin Cristal"),
    challenge_def("unlucky", "Chat noir", 100, None, "obsidian",
                  "Ni critiques, ni masse critique, ni ravitaillements. Atteins le niveau 100.",
                  "Chance de critique +10 % permanente + skin Obsidienne"),
    challenge_def("nanar", "Nanarland", 100, None, "carton",
                  "Effets spéciaux en carton : 1 détonation sur 3 fait pschitt, budget de série Z (production ÷2). \
                   Atteins le niveau 100.",
                  "Critiques ×15 au lieu de ×10 permanent + skin Carton-pâte"),
];

pub fn find_challenge(id: &str) -> Option<&'static Challenge> {
    CHALLENGES.iter().find(|c| c.id == id)
}

pub fn challenge(id: &str) -> &'static Challenge {
    find_challenge(id).unwrap_or_else(|| panic!("unknown challenge {id}"))
}

/// (id, name), in cycling order.
pub const SKINS: [(&str, &str); 14] = [
    ("classic", "Classique (feu)"),
    ("toxic", "Vert radioactif"),
    ("rainbow", "Arc-en-ciel"),
    ("mario", "Champignon de Mario"),
    ("plasma", "Plasma (5 prestiges)"),
    ("gold", "Or massif"),
    ("ghost", "Fantôme"),
    ("neon", "Néon"),
    ("ice", "Glace"),
    ("lava", "Lave"),
    ("camo", "Camouflage"),
    ("crystal", "Cristal"),
    ("obsidian", "Obsidienne"),
    ("carton", "Carton-pâte"),
];

/// Skin id as a static string (None if unknown).
pub fn find_skin(id: &str) -> Option<&'static str> {
    SKINS.iter().find(|s| s.0 == id).map(|s| s.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_talents_are_generated_in_order() {
        let u = talent("u_mortar");
        assert_eq!(u.req, vec![("u_soldier", 1)]);
        assert_eq!(u.cost, (3, 0));
        assert_eq!(u.desc, "Débloque l'unité : Mortier.");
        let m = talent("m_soldier");
        assert_eq!((m.cost, m.max, m.depth), ((2, 1), 3, 1));
        assert_eq!(talent("m_blackhole").cost, (15, 1));
        assert!(talent("u_soldier").req.is_empty());
        assert_eq!(TALENTS.len(), 6 + 2 * UNIT_COUNT + 7);
        assert_eq!(UNIT_TALENT_IDS[1], ("u_mortar", "m_mortar"));
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = TALENTS.iter().map(|t| t.id).collect();
        ids.extend(PTALENTS.iter().map(|p| p.id));
        ids.extend(STALENTS.iter().map(|p| p.id));
        ids.extend(CHALLENGES.iter().map(|c| c.id));
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn challenge_skins_exist() {
        for c in &CHALLENGES {
            assert!(find_skin(c.skin).is_some(), "{}", c.skin);
        }
    }
}
