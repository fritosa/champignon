//! Static definitions: units, run talents, prestige talents, challenges, skins.
//!
//! The effect values of talents and challenges are named constants next to their table; descriptions quote
//! them (fr_num, Desc) so that rebalancing touches one place.

use std::fmt;
use std::sync::LazyLock;

pub const BASE_CAP: u32 = 100;
pub const MAX_CAP: u32 = 1_000_000;

/// Number written the French way for player texts: decimal comma, at most 3 decimals without trailing zeros,
/// thousands separated by spaces (1,5 · 0,1 · 1 000 000).
pub fn fr_num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let (sign, digits) = int.split_at(usize::from(int.starts_with('-')));
    let mut grouped = String::from(sign);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(c);
    }
    if frac.is_empty() {
        grouped
    } else {
        format!("{grouped},{frac}")
    }
}

/// Most values a Desc can quote.
const DESC_VALUES: usize = 3;

/// Player text of a const table quoting tuning values: like `format!`, each `{}` shows the next value and
/// `{n}` the n-th one, written by fr_num. Displayed with `{}`, like a string.
#[derive(Clone, Copy, Debug)]
pub struct Desc {
    pub template: &'static str,
    pub values: [f64; DESC_VALUES],
    /// Values given (the others are unused padding).
    pub len: usize,
}

impl Desc {
    pub const fn new(template: &'static str, values: &[f64]) -> Desc {
        Desc::after(template, &[], values)
    }

    /// Values `first` then `rest` (a field of the table entry, then the values of its text).
    const fn after(template: &'static str, first: &[f64], rest: &[f64]) -> Desc {
        let mut values = [0.0; DESC_VALUES];
        let len = first.len() + rest.len();
        let mut i = 0;
        while i < len {
            values[i] = if i < first.len() {
                first[i]
            } else {
                rest[i - first.len()]
            };
            i += 1;
        }
        Desc {
            template,
            values,
            len,
        }
    }
}

impl fmt::Display for Desc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rest = self.template;
        let mut next = 0;
        while let Some(open) = rest.find('{') {
            let close = open + rest[open..].find('}').expect("unclosed { in a Desc");
            let index = match &rest[open + 1..close] {
                "" => {
                    next += 1;
                    next - 1
                }
                n => n.parse().expect("{n} expected in a Desc"),
            };
            let value = self.values[..self.len]
                .get(index)
                .unwrap_or_else(|| panic!("no value {index} in the Desc {:?}", self.template));
            write!(f, "{}{}", &rest[..open], fr_num(*value))?;
            rest = &rest[close + 1..];
        }
        f.write_str(rest)
    }
}

/// Unit bought with radiation. Each one is unlocked by a "u_<id>" talent
/// and boosted (×UNIT_TALENT_MULT per rank) by the "m_<id>" talent.
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
    desc: String,
    req: &[(&'static str, u32)],
) -> Talent {
    Talent { id, name, branch, depth, max, cost, desc: leak(desc), req: req.to_vec() }
}

/// Generated strings live as long as the program (the tables are built once).
pub(crate) fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

// "m_<id>" talents: production ×UNIT_TALENT_MULT per rank, UNIT_TALENT_RANKS ranks; first rank at half the
// unlock cost (at least UNIT_TALENT_MIN_COST), then +UNIT_TALENT_COST_STEP per rank.
pub const UNIT_TALENT_MULT: f64 = 2.0;
const UNIT_TALENT_RANKS: u32 = 3;
const UNIT_TALENT_MIN_COST: i64 = 2;
const UNIT_TALENT_COST_STEP: i64 = 1;

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
            format!("Débloque l'unité : {}.", unit.name),
            &unlock_req,
        ));
        talents.push(talent_def(
            leak(format!("m_{}", unit.id)),
            unit.mult_name,
            "inf",
            1,
            UNIT_TALENT_RANKS,
            (
                (unit.unlock_cost as i64 / 2).max(UNIT_TALENT_MIN_COST),
                UNIT_TALENT_COST_STEP,
            ),
            format!(
                "Production des {}s ×{} par rang.",
                unit.name,
                fr_num(UNIT_TALENT_MULT)
            ),
            &[(unlock_id, 1)],
        ));
        previous = Some(unit.id);
    }
    talents
}

// Run talent effects, per rank (shares are fractions: 0.1 = +10 %).
pub const BASE_CLICK: f64 = 1.0;
pub const T_CLICK_POWER: f64 = 2.0;
pub const T_CRIT_CHANCE: f64 = 0.05;
/// Max combo without talents or mutations, and "Réaction en chaîne" (t_combo): +T_COMBO_MAX per rank.
pub const BASE_MAX_COMBO: u32 = 3;
pub const T_COMBO_MAX: u32 = 1;
const T_COMBO_RANKS: u32 = 2;
/// Presses per second over which the detonator overheats, and "Refroidissement" (t_cool) per rank.
pub const BASE_OVERHEAT_THRESHOLD: u32 = 12;
pub const T_COOL_THRESHOLD: u32 = 3;
/// Auto-detonator presses per second.
pub const T_AUTO_RATE: f64 = 1.0;
/// Share of the production added to each press.
pub const T_SHOCK_SHARE: f64 = 0.02;
pub const L_GLOBAL_PROD: f64 = 0.1;
/// Production per unit owned.
pub const L_SYNERGY_PROD: f64 = 0.001;
pub const L_DISCOUNT: f64 = 0.04;
/// Unit types bought automatically (in UNITS order).
pub const L_AUTOBUY_TYPES: usize = 3;
/// Supply drops frequency, and planes duration.
pub const L_RADAR_FREQ: f64 = 0.3;
/// Critical mass charging speed.
pub const L_ENRICH_SPEED: f64 = 0.25;

/// Percentage of a share (0.1 → 10), for the texts.
pub const fn percent(share: f64) -> f64 {
    share * 100.0
}

#[rustfmt::skip]
fn build_talents() -> Vec<Talent> {
    let mut talents = vec![
        talent_def("t_click", "Détonateur renforcé", "det", 0, 10, (1, 1),
                   format!("+{} de puissance de détonation de base par rang.", fr_num(T_CLICK_POWER)), &[]),
        talent_def("t_crit", "Fission critique", "det", 1, 5, (2, 1),
                   format!("+{} % de chance de détonation critique par rang.", fr_num(percent(T_CRIT_CHANCE))),
                   &[("t_click", 1)]),
        talent_def("t_combo", "Réaction en chaîne", "det", 1, T_COMBO_RANKS, (2, 2),
                   format!("Combo max +{} par rang (×{} → ×{}).", T_COMBO_MAX, BASE_MAX_COMBO,
                           BASE_MAX_COMBO + T_COMBO_MAX * T_COMBO_RANKS),
                   &[("t_click", 1)]),
        talent_def("t_cool", "Refroidissement", "det", 2, 3, (2, 1),
                   format!("Seuil de surchauffe +{T_COOL_THRESHOLD} appuis/s par rang."), &[("t_combo", 1)]),
        talent_def("t_auto", "Auto-détonateur", "det", 1, 5, (3, 2),
                   format!("Appuie tout seul sur ESPACE : {} fois/s par rang.", fr_num(T_AUTO_RATE)),
                   &[("t_click", 2)]),
        talent_def("t_shock", "Onde de choc", "det", 1, 5, (3, 2),
                   format!("Chaque appui ajoute {} % de la production/s par rang.", fr_num(percent(T_SHOCK_SHARE))),
                   &[("t_click", 3)]),
    ];
    talents.extend(unit_talents());
    talents.extend([
        talent_def("l_global", "Propagande", "log", 0, 10, (2, 1),
                   format!("Production globale +{} % par rang.", fr_num(percent(L_GLOBAL_PROD))), &[]),
        talent_def("l_synergy", "Esprit de corps", "log", 1, 3, (4, 2),
                   format!("+{} % de production par unité possédée, par rang.", fr_num(percent(L_SYNERGY_PROD))),
                   &[("l_global", 2)]),
        talent_def("l_guard", "Gardes", "log", 1, 1, (4, 0),
                   "Des gardes chassent automatiquement les inspecteurs de l'ONU.".to_string(), &[("l_global", 1)]),
        talent_def("l_discount", "Marché noir", "log", 0, 5, (2, 1),
                   format!("Coût des unités -{} % par rang.", fr_num(percent(L_DISCOUNT))), &[]),
        talent_def("l_autobuy", "Quartier-maître", "log", 1, 3, (5, 3),
                   format!("Achète automatiquement les unités ({L_AUTOBUY_TYPES} types de plus par rang)."),
                   &[("l_discount", 1)]),
        talent_def("l_radar", "Radar", "log", 0, 3, (2, 1),
                   format!("Ravitaillements +{} % plus fréquents et avions plus lents.", fr_num(percent(L_RADAR_FREQ))),
                   &[]),
        talent_def("l_enrich", "Enrichissement", "log", 0, 3, (3, 1),
                   format!("La masse critique se charge {} % plus vite par rang.", fr_num(percent(L_ENRICH_SPEED))),
                   &[]),
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
    pub desc: Desc,
    pub base: f64,
    pub growth: f64,
    pub costs: Option<&'static [i64]>,
}

const fn perk(
    id: &'static str,
    name: &'static str,
    max: u32,
    desc: Desc,
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

/// Perk with a fixed cost per rank (base and growth are then unused).
#[rustfmt::skip]
const fn perk_costs(id: &'static str, name: &'static str, max: u32, desc: Desc, costs: &'static [i64]) -> Perk {
    Perk { id, name, max, desc, base: 0.0, growth: 0.0, costs: Some(costs) }
}

// Prestige talent effects, per rank.
/// "Expansion du cœur": max level ×P_CAP_MULT.
pub const P_CAP_MULT: u32 = 10;
pub const P_PROD_MULT: f64 = 1.5;
pub const P_CLICK_MULT: f64 = 2.0;
/// PT at the start of each run.
pub const P_PT_BONUS: i64 = 5;
/// Soldiers at the start of each run.
pub const P_VETS_SOLDIERS: u64 = 10;
/// Isotopes earned.
pub const P_ISO_BONUS: f64 = 0.25;
/// Supply drops power (buff durations, amounts).
pub const P_DROP_POWER: f64 = 0.5;
/// Share of the production earned while away, at most OFFLINE_MAX seconds.
pub const P_OFFLINE_SHARE: f64 = 0.25;
pub const OFFLINE_MAX: f64 = 2.0 * 3600.0;
/// Mutations offered.
pub const P_DNA_CHOICES: usize = 1;

/// Prestige talents, paid in isotopes.
#[rustfmt::skip]
pub const PTALENTS: [Perk; 10] = [
    perk_costs("p_cap", "Expansion du cœur", 4,
               Desc::new("Niveau max ×{} ({} → {}).", &[P_CAP_MULT as f64, BASE_CAP as f64, MAX_CAP as f64]),
               &[20, 150, 1_200, 9_000]),
    perk("p_prod", "Uranium appauvri", 25, Desc::new("Production ×{} par rang (multiplicatif).", &[P_PROD_MULT]),
         2.0, 2.0),
    perk("p_click", "Doigt nucléaire", 15, Desc::new("Puissance de détonation ×{} par rang.", &[P_CLICK_MULT]),
         1.0, 1.5),
    perk("p_pt", "Mémoire génétique", 10, Desc::new("Commence chaque run avec +{} PT par rang.", &[P_PT_BONUS as f64]),
         3.0, 1.6),
    perk("p_vets", "Vétérans", 5,
         Desc::new("Commence avec la Conscription et {} soldats par rang.", &[P_VETS_SOLDIERS as f64]), 4.0, 2.0),
    perk_costs("p_keep", "Automates persistants", 1,
               Desc::new("Auto-détonateur, Quartier-maître et Marché noir sont conservés au reset.", &[]), &[15]),
    perk("p_iso", "Centrifugeuses", 10, Desc::new("+{} % d'isotopes gagnés par rang.", &[percent(P_ISO_BONUS)]),
         5.0, 1.7),
    perk("p_drop", "Contrebandiers", 3,
         Desc::new("Ravitaillements +{} % plus efficaces par rang.", &[percent(P_DROP_POWER)]), 5.0, 2.0),
    perk("p_offline", "Sommeil radioactif", 4,
         Desc::new("Progression hors-ligne : {} % de la production par rang ({} h max).",
                   &[percent(P_OFFLINE_SHARE), OFFLINE_MAX / 3600.0]),
         4.0, 2.0),
    perk("p_dna", "ADN instable", 2,
         Desc::new("+{} mutation proposée au choix par rang.", &[P_DNA_CHOICES as f64]), 20.0, 3.0),
];

pub fn find_ptalent(id: &str) -> Option<&'static Perk> {
    PTALENTS.iter().find(|p| p.id == id)
}

pub fn ptalent(id: &str) -> &'static Perk {
    find_ptalent(id).unwrap_or_else(|| panic!("unknown prestige talent {id}"))
}

// Supernova talent effects, per rank.
pub const S_PROD_MULT: f64 = 2.0;
pub const S_ISO_MULT: f64 = 2.0;
/// Cost reduction of "Expansion du cœur".
pub const S_CORE_DISCOUNT: f64 = 0.2;
/// Critical mass charging speed multiplier.
pub const S_CRIT_MULT: f64 = 2.0;

/// Supernova talents, paid in stars (✦). Never reset.
#[rustfmt::skip]
pub const STALENTS: [Perk; 5] = [
    perk("s_prod", "Fusion stellaire", 10, Desc::new("Production ×{} par rang.", &[S_PROD_MULT]), 1.0, 2.0),
    perk("s_iso", "Nucléosynthèse", 10, Desc::new("Isotopes gagnés ×{} par rang.", &[S_ISO_MULT]), 1.0, 2.0),
    perk_costs("s_core", "Cœur dense", 4,
               Desc::new("Expansion du cœur coûte {} % de moins par rang.", &[percent(S_CORE_DISCOUNT)]),
               &[2, 5, 10, 20]),
    perk("s_crit", "Pulsar", 3, Desc::new("La masse critique se charge {}× plus vite par rang.", &[S_CRIT_MULT]),
         3.0, 2.0),
    perk_costs("s_auto", "Chaîne de montage", 1, Desc::new("Achète automatiquement les talents de run.", &[]), &[5]),
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
    /// `{0}` is the target level, `{1}`... the values given to challenge_def.
    pub desc: Desc,
    pub reward: Desc,
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
const fn challenge_def(
    id: &'static str,
    name: &'static str,
    target: u32,
    time: Option<f64>,
    skin: &'static str,
    desc: &'static str,
    desc_values: &[f64],
    reward: Desc,
) -> Challenge {
    let desc = Desc::after(desc, &[target as f64], desc_values);
    Challenge { id, name, target, time, skin, desc, reward }
}

// Challenge constraints.
pub const ONEARM_SOLDIERS: u64 = 3;
pub const RACE_TIME: f64 = 600.0;
pub const INFLATION_COST_MULT: f64 = 4.0;
/// Delay between two inspectors (min, max), in seconds.
pub const WATCHED_INSPECTOR_DELAY: (f64, f64) = (30.0, 60.0);
pub const HOTHEAD_THRESHOLD: u32 = 6;
pub const HOTHEAD_OVERHEAT: f64 = 6.0;
/// Delay between two enemy waves (min, max), in seconds.
pub const INVASION_ENEMY_DELAY: (f64, f64) = (15.0, 30.0);
/// One press in NANAR_FIZZLE_EVERY fizzles.
pub const NANAR_FIZZLE_EVERY: f64 = 3.0;
pub const NANAR_PROD_DIVISOR: f64 = 2.0;
// Challenge rewards.
pub const PACIFIST_CLICK_MULT: f64 = 3.0;
pub const ONEARM_PROD_MULT: f64 = 2.0;
pub const RACE_ISO_MULT: f64 = 1.5;
pub const INFLATION_COST_REWARD: f64 = 0.9;
/// Mutations offered.
pub const BLIND_CHOICES: usize = 1;
/// Loot of chased inspectors.
pub const WATCHED_LOOT_MULT: f64 = 3.0;
/// PT at the start of each run.
pub const ASCETE_PT: i64 = 10;
pub const HOTHEAD_THRESHOLD_BONUS: u32 = 5;
/// Enemies walk duration multiplier.
pub const INVASION_ENEMY_SLOWDOWN: f64 = 1.5;
/// Critical mass charging speed multiplier.
pub const STERILE_CRIT_SPEED: f64 = 1.5;
pub const UNLUCKY_CRIT_CHANCE: f64 = 0.1;
/// Critical press multiplier, without and with the Nanarland reward.
pub const BASE_CRIT_MULT: f64 = 10.0;
pub const NANAR_CRIT_MULT: f64 = 15.0;

#[rustfmt::skip]
pub const CHALLENGES: [Challenge; 12] = [
    challenge_def("pacifist", "Pacifiste", 60, None, "toxic", "Aucune unité autorisée. Atteins le niveau {0}.", &[],
                  Desc::new("Détonation ×{} permanente + skin Vert radioactif", &[PACIFIST_CLICK_MULT])),
    challenge_def("onearm", "Manchot", 100, None, "rainbow",
                  "Ni ESPACE ni auto-détonateur ({1} soldats offerts). Atteins le niveau {0}.",
                  &[ONEARM_SOLDIERS as f64],
                  Desc::new("Production ×{} permanente + skin Arc-en-ciel", &[ONEARM_PROD_MULT])),
    challenge_def("race", "Course", 100, Some(RACE_TIME), "mario", "Atteins le niveau {0} en moins de {1} minutes.",
                  &[RACE_TIME / 60.0],
                  Desc::new("Isotopes ×{} permanent + skin Champignon de Mario", &[RACE_ISO_MULT])),
    challenge_def("inflation", "Inflation", 100, None, "gold",
                  "Les unités coûtent {1} fois plus cher. Atteins le niveau {0}.", &[INFLATION_COST_MULT],
                  Desc::new("Unités -{} % permanent + skin Or", &[percent(1.0 - INFLATION_COST_REWARD)])),
    challenge_def("blind", "Brouillard de guerre", 100, None, "ghost",
                  "Niveau, radiation et production sont masqués. Atteins le niveau {0}.", &[],
                  Desc::new("+{} mutation proposée permanente + skin Fantôme", &[BLIND_CHOICES as f64])),
    challenge_def("watched", "Sous surveillance", 100, None, "neon",
                  "Un inspecteur toutes les {1} à {2} s, gardes et brouillard inefficaces. Atteins le niveau {0}.",
                  &[WATCHED_INSPECTOR_DELAY.0, WATCHED_INSPECTOR_DELAY.1],
                  Desc::new("Inspecteurs chassés ×{} de butin + skin Néon", &[WATCHED_LOOT_MULT])),
    challenge_def("ascete", "Ascète", 100, None, "ice",
                  "Seuls les talents qui débloquent des unités sont autorisés. Atteins le niveau {0}.", &[],
                  Desc::new("+{} PT au début de chaque run + skin Glace", &[ASCETE_PT as f64])),
    challenge_def("hothead", "Tête brûlée", 100, None, "lava",
                  "Surchauffe dès {1} appuis/s, pendant {2} s, même avec les Doigts d'amiante. Atteins le niveau {0}.",
                  &[HOTHEAD_THRESHOLD as f64, HOTHEAD_OVERHEAT],
                  Desc::new("Seuil de surchauffe +{} appuis/s permanent + skin Lave", &[HOTHEAD_THRESHOLD_BONUS as f64])),
    challenge_def("invasion", "Invasion", 100, None, "camo",
                  "Un ennemi toutes les {1} à {2} s, tous types dès le début. Atteins le niveau {0}.",
                  &[INVASION_ENEMY_DELAY.0, INVASION_ENEMY_DELAY.1],
                  Desc::new("Ennemis {} % plus lents permanent + skin Camouflage",
                            &[percent(INVASION_ENEMY_SLOWDOWN - 1.0)])),
    challenge_def("sterile", "Stérile", 100, None, "crystal",
                  "Aucune mutation ne t'est proposée. Atteins le niveau {0}.", &[],
                  Desc::new("Masse critique {} % plus rapide permanente + skin Cristal",
                            &[percent(STERILE_CRIT_SPEED - 1.0)])),
    challenge_def("unlucky", "Chat noir", 100, None, "obsidian",
                  "Ni critiques, ni masse critique, ni ravitaillements. Atteins le niveau {0}.", &[],
                  Desc::new("Chance de critique +{} % permanente + skin Obsidienne", &[percent(UNLUCKY_CRIT_CHANCE)])),
    challenge_def("nanar", "Nanarland", 100, None, "carton",
                  "Effets spéciaux en carton : 1 détonation sur {1} fait pschitt, budget de série Z (production ÷{2}). \
                   Atteins le niveau {0}.",
                  &[NANAR_FIZZLE_EVERY, NANAR_PROD_DIVISOR],
                  Desc::new("Critiques ×{} au lieu de ×{} permanent + skin Carton-pâte",
                            &[NANAR_CRIT_MULT, BASE_CRIT_MULT])),
];

pub fn find_challenge(id: &str) -> Option<&'static Challenge> {
    CHALLENGES.iter().find(|c| c.id == id)
}

pub fn challenge(id: &str) -> &'static Challenge {
    find_challenge(id).unwrap_or_else(|| panic!("unknown challenge {id}"))
}

/// Prestiges that unlock the "plasma" skin.
pub const PLASMA_PRESTIGES: u64 = 5;

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
        assert_eq!(m.desc, "Production des Soldats ×2 par rang.");
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

    #[test]
    fn french_numbers() {
        assert_eq!(fr_num(1.5), "1,5");
        assert_eq!(fr_num(percent(T_CRIT_CHANCE)), "5");
        assert_eq!(fr_num(percent(L_SYNERGY_PROD)), "0,1");
        assert_eq!(fr_num(100.0), "100");
        assert_eq!(fr_num(1_000_000.0), "1 000 000");
        assert_eq!(fr_num(-1234.25), "-1 234,25");
    }

    #[test]
    fn descriptions_quote_their_values() {
        assert_eq!(
            ptalent("p_prod").desc.to_string(),
            "Production ×1,5 par rang (multiplicatif)."
        );
        assert_eq!(
            ptalent("p_cap").desc.to_string(),
            "Niveau max ×10 (100 → 1 000 000)."
        );
        assert_eq!(
            ptalent("p_offline").desc.to_string(),
            "Progression hors-ligne : 25 % de la production par rang (2 h max)."
        );
        let race = challenge("race");
        assert_eq!(
            race.desc.to_string(),
            "Atteins le niveau 100 en moins de 10 minutes."
        );
        assert_eq!(
            challenge("pacifist").desc.to_string(),
            "Aucune unité autorisée. Atteins le niveau 60."
        );
        assert_eq!(
            challenge("nanar").reward.to_string(),
            "Critiques ×15 au lieu de ×10 permanent + skin Carton-pâte"
        );
        assert_eq!(talent("t_combo").desc, "Combo max +1 par rang (×3 → ×5).");
    }

    #[test]
    fn every_desc_has_its_values() {
        let descs = PTALENTS
            .iter()
            .chain(&STALENTS)
            .map(|p| p.desc)
            .chain(CHALLENGES.iter().flat_map(|c| [c.desc, c.reward]))
            .chain(crate::events::SYNERGIES.iter().map(|s| s.effect));
        for d in descs {
            let quoted = d.template.matches('{').count();
            assert!(quoted >= d.len, "{}", d.template);
            let _ = d.to_string(); // panics on a bad placeholder or a missing value
        }
    }
}
