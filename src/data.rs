//! Static definitions: units, run talents, prestige talents, challenges, skins.
//!
//! The effect values of talents and challenges are named constants next to their table; descriptions quote
//! them (i18n::num, Desc) so that rebalancing touches one place.

use std::sync::LazyLock;

use crate::i18n::{num, tr};

pub const BASE_CAP: u32 = 100;
pub const MAX_CAP: u32 = 1_000_000;

/// Most values a Desc can quote.
const DESC_VALUES: usize = 3;

/// Values quoted by a player text of a const table: the text itself is a template of the catalog
/// (`i18n::tr().data`), where each `{}` shows the next value and `{n}` the n-th one, written by i18n::num.
#[derive(Clone, Copy, Debug)]
pub struct Desc {
    pub values: [f64; DESC_VALUES],
    /// Values given (the others are unused padding).
    pub len: usize,
}

impl Desc {
    pub const fn new(values: &[f64]) -> Desc {
        Desc::after(&[], values)
    }

    /// Values `first` then `rest` (a field of the table entry, then the values of its text).
    const fn after(first: &[f64], rest: &[f64]) -> Desc {
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
        Desc { values, len }
    }

    /// The template with its values.
    pub fn render(&self, template: &str) -> String {
        let mut out = String::new();
        let mut rest = template;
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
                .unwrap_or_else(|| panic!("no value {index} in the Desc {template:?}"));
            out.push_str(&rest[..open]);
            out.push_str(&num(*value));
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// Unit bought with radiation. Each one is unlocked by a "u_<id>" talent
/// and boosted (×UNIT_TALENT_MULT per rank) by the "m_<id>" talent.
#[derive(Debug)]
pub struct Unit {
    pub id: &'static str,
    pub icon: &'static str,
    pub cost: f64,
    pub prod: f64,
    pub proj: &'static str,
    pub color: &'static str,
    pub unlock_cost: u32,
    /// Challenge to complete (rank I) before its unlock talent can be bought, and the production
    /// multiplier per rank of that challenge.
    pub challenge: Option<(&'static str, f64)>,
}

impl Unit {
    pub fn name(&self) -> &'static str {
        tr().data.unit(self.id).name
    }

    pub fn plural(&self) -> &'static str {
        tr().data.unit(self.id).plural
    }

    /// Name of the talent that unlocks it.
    pub fn unlock_name(&self) -> &'static str {
        tr().data.unit(self.id).unlock_name
    }

    /// Name of the talent that boosts it.
    pub fn mult_name(&self) -> &'static str {
        tr().data.unit(self.id).mult_name
    }

    /// Same unit, unlockable only once this challenge is done, its production ×`mult` per rank of it.
    const fn after_challenge(self, challenge: &'static str, mult: f64) -> Unit {
        Unit {
            challenge: Some((challenge, mult)),
            ..self
        }
    }
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
const fn unit_def(
    id: &'static str,
    icon: &'static str,
    cost: f64,
    prod: f64,
    proj: &'static str,
    color: &'static str,
    unlock_cost: u32,
) -> Unit {
    Unit { id, icon, cost, prod, proj, color, unlock_cost, challenge: None }
}

#[rustfmt::skip]
pub const UNITS: [Unit; 15] = [
    unit_def("soldier", "i", 15.0, 0.5, "o", "green", 1),
    unit_def("mortar", "y", 150.0, 4.0, "*", "yellow", 3),
    unit_def("artillery", "F", 1_500.0, 30.0, "°", "orange", 5),
    unit_def("tank", "T", 15_000.0, 220.0, "=", "green", 8),
    unit_def("bomber", ">", 160_000.0, 1_600.0, "v", "cyan", 12),
    unit_def("sub", "D", 1.8e6, 12_000.0, "^", "blue", 16),
    unit_def("icbm", "!", 2e7, 90_000.0, "!", "red", 20),
    unit_def("reactor", "@", 2.3e8, 700_000.0, "+", "magenta", 25),
    unit_def("blackhole", "O", 2.6e9, 5.5e6, "·", "white", 30),
    unit_def("moussaka", "#", 3e10, 1.4e7, "~", "orange", 35).after_challenge("moussaka", MOUSSAKA_PROD_MULT),
    unit_def("savior", "&", 3.5e11, 1.1e8, "*", "yellow", 40).after_challenge("savior", NANAR_UNIT_MULT),
    unit_def("diamond", "$", 4e12, 8e8, "'", "white", 45).after_challenge("whitefire", NANAR_UNIT_MULT),
    unit_def("croc", "C", 4.6e13, 6e9, "w", "green", 50).after_challenge("crocodile", NANAR_UNIT_MULT),
    unit_def("mummy", "M", 5.3e14, 4.7e10, "#", "red", 55).after_challenge("devil", NANAR_UNIT_MULT),
    unit_def("ninja", "N", 6.1e15, 3.5e11, "x", "magenta", 60).after_challenge("ninja", NANAR_UNIT_MULT),
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
    pub branch: &'static str,
    pub depth: u32,
    pub max: u32,
    pub cost: (i64, i64),
    pub text: TalentText,
    pub req: Vec<(&'static str, u32)>,
}

/// Where the name and description of a talent come from.
#[derive(Debug)]
pub enum TalentText {
    /// Its entry in the catalog (`i18n::tr().data.talents`), with the values its description quotes.
    Table(Desc),
    /// "u_<id>": unlocks this unit.
    Unlock(&'static Unit),
    /// "m_<id>": boosts this unit.
    Mult(&'static Unit),
}

impl Talent {
    pub fn name(&self) -> &'static str {
        match self.text {
            TalentText::Table(_) => tr().data.talent(self.id).name,
            TalentText::Unlock(unit) => unit.unlock_name(),
            TalentText::Mult(unit) => unit.mult_name(),
        }
    }

    pub fn desc(&self) -> String {
        let texts = &tr().data;
        match self.text {
            TalentText::Table(desc) => desc.render(texts.talent(self.id).desc),
            TalentText::Unlock(unit) => match unit.challenge {
                Some((c, _)) => (texts.unit_unlock_after)(unit.name(), challenge(c).name()),
                None => (texts.unit_unlock)(unit.name()),
            },
            TalentText::Mult(unit) => (texts.unit_mult)(unit.plural(), &num(UNIT_TALENT_MULT)),
        }
    }
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
fn talent_def(
    id: &'static str,
    branch: &'static str,
    depth: u32,
    max: u32,
    cost: (i64, i64),
    text: TalentText,
    req: &[(&'static str, u32)],
) -> Talent {
    Talent { id, branch, depth, max, cost, text, req: req.to_vec() }
}

/// Run talent described by its catalog entry, quoting `values`.
#[allow(clippy::too_many_arguments)]
fn table_talent(
    id: &'static str,
    branch: &'static str,
    depth: u32,
    max: u32,
    cost: (i64, i64),
    values: &[f64],
    req: &[(&'static str, u32)],
) -> Talent {
    talent_def(
        id,
        branch,
        depth,
        max,
        cost,
        TalentText::Table(Desc::new(values)),
        req,
    )
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
            "inf",
            0,
            1,
            (unit.unlock_cost as i64, 0),
            TalentText::Unlock(unit),
            &unlock_req,
        ));
        talents.push(talent_def(
            leak(format!("m_{}", unit.id)),
            "inf",
            1,
            UNIT_TALENT_RANKS,
            (
                (unit.unlock_cost as i64 / 2).max(UNIT_TALENT_MIN_COST),
                UNIT_TALENT_COST_STEP,
            ),
            TalentText::Mult(unit),
            &[(unlock_id, 1)],
        ));
        if unit.challenge.is_none() {
            previous = Some(unit.id);
        }
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
        table_talent("t_click", "det", 0, 10, (1, 1), &[T_CLICK_POWER], &[]),
        table_talent("t_crit", "det", 1, 5, (2, 1), &[percent(T_CRIT_CHANCE)], &[("t_click", 1)]),
        table_talent("t_combo", "det", 1, T_COMBO_RANKS, (2, 2),
                     &[T_COMBO_MAX as f64, BASE_MAX_COMBO as f64, (BASE_MAX_COMBO + T_COMBO_MAX * T_COMBO_RANKS) as f64],
                     &[("t_click", 1)]),
        table_talent("t_cool", "det", 2, 3, (2, 1), &[T_COOL_THRESHOLD as f64], &[("t_combo", 1)]),
        table_talent("t_auto", "det", 1, 5, (3, 2), &[T_AUTO_RATE], &[("t_click", 2)]),
        table_talent("t_shock", "det", 1, 5, (3, 2), &[percent(T_SHOCK_SHARE)], &[("t_click", 3)]),
    ];
    talents.extend(unit_talents());
    talents.extend([
        table_talent("l_global", "log", 0, 10, (2, 1), &[percent(L_GLOBAL_PROD)], &[]),
        table_talent("l_synergy", "log", 1, 3, (4, 2), &[percent(L_SYNERGY_PROD)], &[("l_global", 2)]),
        table_talent("l_guard", "log", 1, 1, (4, 0), &[], &[("l_global", 1)]),
        table_talent("l_discount", "log", 0, 5, (2, 1), &[percent(L_DISCOUNT)], &[]),
        table_talent("l_autobuy", "log", 1, 3, (5, 3), &[L_AUTOBUY_TYPES as f64], &[("l_discount", 1)]),
        table_talent("l_radar", "log", 0, 3, (2, 1), &[percent(L_RADAR_FREQ)], &[]),
        table_talent("l_enrich", "log", 0, 3, (3, 1), &[percent(L_ENRICH_SPEED)], &[]),
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

/// Talent branches (names: i18n).
pub const BRANCHES: [&str; 3] = ["det", "inf", "log"];
/// Talents kept on reset with "Automates persistants".
pub const KEPT_TALENTS: [&str; 3] = ["t_auto", "l_autobuy", "l_discount"];

/// Permanent talent (prestige ⚛ or supernova ✦).
/// Cost = round(base * growth^rank) or costs[rank].
#[derive(Debug)]
pub struct Perk {
    pub id: &'static str,
    pub max: u32,
    /// Values of its description.
    pub desc: Desc,
    pub base: f64,
    pub growth: f64,
    pub costs: Option<&'static [i64]>,
}

const fn perk(id: &'static str, max: u32, desc: Desc, base: f64, growth: f64) -> Perk {
    Perk {
        id,
        max,
        desc,
        base,
        growth,
        costs: None,
    }
}

/// Perk with a fixed cost per rank (base and growth are then unused).
#[rustfmt::skip]
const fn perk_costs(id: &'static str, max: u32, desc: Desc, costs: &'static [i64]) -> Perk {
    Perk { id, max, desc, base: 0.0, growth: 0.0, costs: Some(costs) }
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
    perk_costs("p_cap", 4,
               Desc::new(&[P_CAP_MULT as f64, BASE_CAP as f64, MAX_CAP as f64]),
               &[20, 100, 400, 1_600]),
    perk("p_prod", 25, Desc::new(&[P_PROD_MULT]),
         2.0, 2.0),
    perk("p_click", 15, Desc::new(&[P_CLICK_MULT]),
         1.0, 1.5),
    perk("p_pt", 10, Desc::new(&[P_PT_BONUS as f64]),
         3.0, 1.6),
    perk("p_vets", 5,
         Desc::new(&[P_VETS_SOLDIERS as f64]), 4.0, 2.0),
    perk_costs("p_keep", 1,
               Desc::new(&[]), &[15]),
    perk("p_iso", 10, Desc::new(&[percent(P_ISO_BONUS)]),
         5.0, 1.7),
    perk("p_drop", 3,
         Desc::new(&[percent(P_DROP_POWER)]), 5.0, 2.0),
    perk("p_offline", 4,
         Desc::new(&[percent(P_OFFLINE_SHARE), OFFLINE_MAX / 3600.0]),
         4.0, 2.0),
    perk("p_dna", 2,
         Desc::new(&[P_DNA_CHOICES as f64]), 20.0, 3.0),
];

impl Perk {
    pub fn name(&self) -> &'static str {
        tr().data.perk(self.id).name
    }

    pub fn desc(&self) -> String {
        self.desc.render(tr().data.perk(self.id).desc)
    }
}

pub fn find_ptalent(id: &str) -> Option<&'static Perk> {
    PTALENTS.iter().find(|p| p.id == id)
}

pub fn ptalent(id: &str) -> &'static Perk {
    find_ptalent(id).unwrap_or_else(|| panic!("unknown prestige talent {id}"))
}

// Supernova talent effects, per rank.
pub const S_PROD_MULT: f64 = 1.5;
pub const S_ISO_MULT: f64 = 1.5;
/// Cost reduction of "Expansion du cœur".
pub const S_CORE_DISCOUNT: f64 = 0.2;
/// Critical mass charging speed multiplier.
pub const S_CRIT_MULT: f64 = 2.0;
/// "Expansion du cœur" ranks kept through a supernova.
pub const S_KEEP_CORE: u32 = 1;

/// Supernova talents, paid in stars (✦). Never reset.
#[rustfmt::skip]
pub const STALENTS: [Perk; 6] = [
    perk("s_prod", 10, Desc::new(&[S_PROD_MULT]), 1.0, 2.0),
    perk("s_iso", 10, Desc::new(&[S_ISO_MULT]), 1.0, 2.0),
    perk_costs("s_core", 4,
               Desc::new(&[percent(S_CORE_DISCOUNT)]),
               &[2, 5, 10, 20]),
    perk("s_crit", 3, Desc::new(&[S_CRIT_MULT]),
         3.0, 2.0),
    perk_costs("s_auto", 1, Desc::new(&[]), &[5]),
    perk_costs("s_keep", 2,
               Desc::new(&[S_KEEP_CORE as f64]),
               &[8, 10]),
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
    pub target: u32,
    /// Time limit of rank I in seconds (None: untimed), ×(rank number) at the next ranks; shown as `{1}` (in
    /// minutes) in the description.
    pub time: Option<f64>,
    pub skin: &'static str,
    /// Values of the description: `{0}` is the target level, `{1}`... the values given to challenge_def.
    pub desc: Desc,
    /// Values of the effect of each rank.
    pub reward: Desc,
    /// Target level multiplier from one rank to the next.
    pub rank_mult: u32,
    /// Offline gains allowed during the challenge (false when its constraint would keep acting while away).
    pub offline: bool,
}

/// Ranks of each challenge; the target level is ×CHALLENGE_TARGET_MULT at each rank (unless the challenge sets
/// its own `rank_mult`).
pub const CHALLENGE_RANKS: u32 = 3;
pub const CHALLENGE_TARGET_MULT: u32 = 10;
/// Challenges to complete (rank I) before double challenges open.
pub const DOUBLE_UNLOCK: usize = 10;
/// Pairs that cannot be played together: without presses (Manchot), these challenges produce nothing.
pub const DOUBLE_FORBIDDEN: [(&str, &str); 2] = [("onearm", "strike"), ("onearm", "savior")];

/// Challenges never played in a double: without units, Pacifiste cannot reach the higher target of another
/// challenge.
pub const DOUBLE_SOLO: [&str; 1] = ["pacifist"];

/// The two challenges cannot be played together (in either order).
pub fn double_forbidden(a: &str, b: &str) -> bool {
    DOUBLE_SOLO.iter().any(|&c| c == a || c == b)
        || DOUBLE_FORBIDDEN
            .iter()
            .any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
}
/// Rank numbers shown to the player.
pub const RANK_NAMES: [&str; CHALLENGE_RANKS as usize] = ["I", "II", "III"];

impl Challenge {
    pub fn name(&self) -> &'static str {
        tr().data.challenge(self.id).name
    }

    /// Effect of each rank.
    pub fn reward_text(&self) -> String {
        self.reward.render(tr().data.challenge(self.id).reward)
    }

    /// Target level of the rank after `done` ranks.
    pub fn target_at(&self, done: u32) -> u32 {
        self.target
            .saturating_mul(self.rank_mult.saturating_pow(done))
    }

    /// Same challenge, its target ×`mult` per rank instead of CHALLENGE_TARGET_MULT.
    const fn rank_mult(self, mult: u32) -> Challenge {
        Challenge {
            rank_mult: mult,
            ..self
        }
    }

    /// Same challenge, without offline gains: its constraint (decay, growth, bites) runs on the run time,
    /// which does not advance while away.
    const fn no_offline(self) -> Challenge {
        Challenge {
            offline: false,
            ..self
        }
    }

    /// Time limit of the rank after `done` ranks: the base limit ×(rank number).
    pub fn time_at(&self, done: u32) -> Option<f64> {
        self.time.map(|t| t * f64::from(done + 1))
    }

    /// Description of the rank after `done` ranks: its target level (`{0}`) and, for a timed challenge, its
    /// time limit in minutes (`{1}`).
    pub fn desc_at(&self, done: u32) -> String {
        let mut desc = self.desc;
        desc.values[0] = self.target_at(done) as f64;
        if let Some(t) = self.time_at(done) {
            desc.values[1] = t / 60.0;
        }
        desc.render(tr().data.challenge(self.id).desc)
    }
}

#[rustfmt::skip]
#[allow(clippy::too_many_arguments)]
const fn challenge_def(
    id: &'static str,
    target: u32,
    time: Option<f64>,
    skin: &'static str,
    desc_values: &[f64],
    reward: Desc,
) -> Challenge {
    let desc = Desc::after(&[target as f64], desc_values);
    Challenge { id, target, time, skin, desc, reward, rank_mult: CHALLENGE_TARGET_MULT, offline: true }
}

// Challenge constraints.
/// Target multipliers per rank of the hardest challenges (the others use CHALLENGE_TARGET_MULT).
pub const PACIFIST_RANK_MULT: u32 = 3;
pub const UNLUCKY_RANK_MULT: u32 = 4;
pub const MONO_RANK_MULT: u32 = 4;
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
/// Half-life of the stored radiation and of the level progress, in seconds.
pub const HALFLIFE_SECONDS: f64 = 10.0;
/// Share of each unit type lost every second (rounded down).
pub const HALFLIFE_UNIT_DECAY: f64 = 0.05;
/// Seconds of production after each press.
pub const STRIKE_WINDOW: f64 = 1.0;
/// The giant moussaka grows by 1 every MOUSSAKA_GROWTH seconds; production is divided by its size.
pub const MOUSSAKA_GROWTH: f64 = 60.0;
/// Seconds of army production released by each press (L'homme qui sauve le monde).
pub const SAVIOR_PUNCH: f64 = 0.15;
/// White Fire: chance per press of a brawl, and share of the radiation it costs.
pub const WHITEFIRE_BRAWL_CHANCE: f64 = 0.05;
pub const WHITEFIRE_BRAWL_LOSS: f64 = 0.2;
/// Crocodile Fury: delay between two bites, and share of a unit type eaten.
pub const CROCODILE_PERIOD: f64 = 30.0;
pub const CROCODILE_BITE: f64 = 0.5;
/// Devil Story: minimum delay between two units bought.
pub const DEVIL_BUY_GAP: f64 = 2.0;
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
/// Critical press multiplier, and the Nanarland reward per rank.
pub const BASE_CRIT_MULT: f64 = 10.0;
pub const NANAR_CRIT_BONUS: f64 = 5.0;
/// Unit milestone multiplier bonus per Monoculture rank (MILESTONE_MULT without).
pub const MONO_MILESTONE_BONUS: f64 = 0.25;
/// Max combo bonus.
pub const HALFLIFE_COMBO: u32 = 2;
/// "Onde de choc" multiplier.
pub const STRIKE_SHOCK_MULT: f64 = 2.0;
/// Production of the tamed moussakas.
pub const MOUSSAKA_PROD_MULT: f64 = 2.0;
/// Production of the units unlocked by the nanar challenges (savior to ninja), per rank.
pub const NANAR_UNIT_MULT: f64 = 2.0;

#[rustfmt::skip]
pub const CHALLENGES: [Challenge; 21] = [
    challenge_def("pacifist", 45, None, "toxic", &[],
                  Desc::new(&[PACIFIST_CLICK_MULT]))
        .rank_mult(PACIFIST_RANK_MULT),
    challenge_def("onearm", 100, None, "rainbow", &[ONEARM_SOLDIERS as f64],
                  Desc::new(&[ONEARM_PROD_MULT])),
    challenge_def("race", 100, Some(RACE_TIME), "mario", &[RACE_TIME / 60.0],
                  Desc::new(&[RACE_ISO_MULT]))
        .no_offline(),
    challenge_def("inflation", 100, None, "gold", &[INFLATION_COST_MULT],
                  Desc::new(&[INFLATION_COST_REWARD])),
    challenge_def("blind", 100, None, "ghost", &[],
                  Desc::new(&[BLIND_CHOICES as f64])),
    challenge_def("watched", 100, None, "neon", &[WATCHED_INSPECTOR_DELAY.0, WATCHED_INSPECTOR_DELAY.1],
                  Desc::new(&[WATCHED_LOOT_MULT])),
    challenge_def("ascete", 100, None, "ice", &[],
                  Desc::new(&[ASCETE_PT as f64])),
    challenge_def("hothead", 100, None, "lava", &[HOTHEAD_THRESHOLD as f64, HOTHEAD_OVERHEAT],
                  Desc::new(&[HOTHEAD_THRESHOLD_BONUS as f64])),
    challenge_def("invasion", 100, None, "camo", &[INVASION_ENEMY_DELAY.0, INVASION_ENEMY_DELAY.1],
                  Desc::new(&[INVASION_ENEMY_SLOWDOWN])),
    challenge_def("sterile", 100, None, "crystal", &[],
                  Desc::new(&[STERILE_CRIT_SPEED])),
    challenge_def("unlucky", 100, None, "obsidian", &[],
                  Desc::new(&[percent(UNLUCKY_CRIT_CHANCE)]))
        .rank_mult(UNLUCKY_RANK_MULT),
    challenge_def("nanar", 100, None, "carton", &[NANAR_FIZZLE_EVERY, NANAR_PROD_DIVISOR],
                  Desc::new(&[NANAR_CRIT_BONUS, BASE_CRIT_MULT])),
    challenge_def("mono", 100, None, "moss", &[],
                  Desc::new(&[MONO_MILESTONE_BONUS, MILESTONE_MULT]))
        .rank_mult(MONO_RANK_MULT),
    challenge_def("halflife", 100, None, "uranium", &[HALFLIFE_SECONDS, percent(HALFLIFE_UNIT_DECAY)],
                  Desc::new(&[HALFLIFE_COMBO as f64]))
        .no_offline(),
    challenge_def("strike", 100, None, "sunset", &[STRIKE_WINDOW],
                  Desc::new(&[STRIKE_SHOCK_MULT])),
    challenge_def("moussaka", 100, None, "moussaka", &[MOUSSAKA_GROWTH],
                  Desc::new(&[MOUSSAKA_PROD_MULT]))
        .no_offline(),
    challenge_def("savior", 100, None, "savior", &[SAVIOR_PUNCH],
                  Desc::new(&[NANAR_UNIT_MULT])),
    challenge_def("whitefire", 100, None, "whitefire", &[percent(WHITEFIRE_BRAWL_CHANCE), percent(WHITEFIRE_BRAWL_LOSS)],
                  Desc::new(&[NANAR_UNIT_MULT])),
    challenge_def("crocodile", 100, None, "ricefield", &[CROCODILE_PERIOD, percent(CROCODILE_BITE)],
                  Desc::new(&[NANAR_UNIT_MULT]))
        .no_offline(),
    challenge_def("devil", 100, None, "goremandie", &[DEVIL_BUY_GAP],
                  Desc::new(&[NANAR_UNIT_MULT])),
    challenge_def("ninja", 100, None, "carambar", &[],
                  Desc::new(&[NANAR_UNIT_MULT])),
];

pub fn find_challenge(id: &str) -> Option<&'static Challenge> {
    CHALLENGES.iter().find(|c| c.id == id)
}

/// The unit unlocked by challenge `id` (moussaka, nanars), if any.
pub fn challenge_unit(id: &str) -> Option<&'static Unit> {
    UNITS
        .iter()
        .find(|u| u.challenge.is_some_and(|(c, _)| c == id))
}

pub fn challenge(id: &str) -> &'static Challenge {
    find_challenge(id).unwrap_or_else(|| panic!("unknown challenge {id}"))
}

/// Prestiges that unlock the "plasma" skin.
pub const PLASMA_PRESTIGES: u64 = 5;

/// Skin ids, in cycling order (names: i18n).
#[rustfmt::skip]
pub const SKINS: [&str; 23] = [
    "classic", "toxic", "rainbow", "mario", "plasma", "gold", "ghost", "neon", "ice", "lava", "camo", "crystal",
    "obsidian", "carton", "moss", "uranium", "sunset", "moussaka", "savior", "whitefire", "ricefield", "goremandie",
    "carambar",
];

/// Name of a skin.
pub fn skin_name(id: &str) -> &'static str {
    tr().data.skin(id)
}

/// Skin id as a static string (None if unknown).
pub fn find_skin(id: &str) -> Option<&'static str> {
    SKINS.iter().find(|&&s| s == id).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_talents_are_generated_in_order() {
        let u = talent("u_mortar");
        assert_eq!(u.req, vec![("u_soldier", 1)]);
        assert_eq!(u.cost, (3, 0));
        assert_eq!(u.desc(), "Débloque l'unité : Mortier.");
        let m = talent("m_soldier");
        assert_eq!((m.cost, m.max, m.depth), ((2, 1), 3, 1));
        assert_eq!(m.desc(), "Production des Soldats ×2 par rang.");
        assert_eq!(
            talent("m_sub").desc(),
            "Production des Sous-marins nucléaires ×2 par rang."
        );
        assert_eq!(talent("m_blackhole").cost, (15, 1));
        assert!(talent("u_soldier").req.is_empty());
        // Challenge units only need the last regular unit, not each other.
        assert_eq!(talent("u_ninja").req, vec![("u_blackhole", 1)]);
        assert_eq!(talent("u_moussaka").req, vec![("u_blackhole", 1)]);
        assert_eq!(TALENTS.len(), 6 + 2 * UNIT_COUNT + 7);
        assert_eq!(
            talent("u_moussaka").desc(),
            "Débloque l'unité : Moussaka apprivoisée. Après le défi L'attaque de la moussaka géante."
        );
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
        assert_eq!(num(1.5), "1,5");
        assert_eq!(num(percent(T_CRIT_CHANCE)), "5");
        assert_eq!(num(percent(L_SYNERGY_PROD)), "0,1");
    }

    #[test]
    fn descriptions_quote_their_values() {
        assert_eq!(
            ptalent("p_prod").desc(),
            "Production ×1,5 par rang (multiplicatif)."
        );
        assert_eq!(ptalent("p_cap").desc(), "Niveau max ×10 (100 → 1 000 000).");
        assert_eq!(
            ptalent("p_offline").desc(),
            "Progression hors-ligne : 25 % de la production par rang (2 h max)."
        );
        let race = challenge("race");
        assert_eq!(
            race.desc_at(0),
            "Atteins le niveau 100 en moins de 10 minutes."
        );
        assert_eq!(
            challenge("pacifist").desc_at(0),
            "Aucune unité autorisée. Atteins le niveau 45."
        );
        assert_eq!(
            challenge("nanar").reward_text(),
            "Critiques +5 par rang (×10 sans) + skin Carton-pâte"
        );
        assert_eq!(talent("t_combo").desc(), "Combo max +1 par rang (×3 → ×5).");
    }
}
