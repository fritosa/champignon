//! Game state and pure logic (no dependency on the terminal).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::data::{self, UNIT_COUNT, UNITS};
use crate::events::{self, EnemyKind};
use crate::rng::{Rng, StdRandom};
use crate::saves;

pub const SUFFIXES: [&str; 12] = [
    "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc",
];
pub const OFFLINE_MAX: f64 = 2.0 * 3600.0;
pub const OFFLINE_MIN: f64 = 60.0;
pub const OFFLINE_SHARE: f64 = 0.25;
pub const MAX_BULK: u64 = 1000; // beyond that, 1.15^n overflows
pub const AUDIO_MODES: [&str; 3] = ["all", "sfx", "off"]; // music + effects, effects only, muted
pub const NANAR_FIZZLE: f64 = 1.0 / 3.0;
pub const VOLUME_STEP: u32 = 5; // in %
pub const COMBO_STEP: f64 = 0.08;
pub const COMBO_GRACE: f64 = 0.4;
pub const CLICK_PITCHES: usize = 8;
/// Press sounds, from the lowest to the highest combo.
pub const CLICK_SOUNDS: [&str; CLICK_PITCHES] = [
    "click0", "click1", "click2", "click3", "click4", "click5", "click6", "click7",
];
pub const PT_BONUS_EVERY: u32 = 10;
pub const PT_LEVEL_GROWTH: f64 = 1.04; // beyond data::BASE_CAP, one PT every +4% of level
pub const SFX_MAX: usize = 16;
pub const MESSAGES_MAX: usize = 8;
pub const THREAT_MAX: f64 = 3600.0; // maximum duration read back from a save (seconds)

/// Powers of 1000 as correctly rounded floats (like Python's int → float conversion).
const THOUSANDS: [f64; 12] = [
    1e0, 1e3, 1e6, 1e9, 1e12, 1e15, 1e18, 1e21, 1e24, 1e27, 1e30, 1e33,
];

/// Short number: 2.5, 150, 1.50K, 2.00M... then 1.00e+40.
pub fn fmt(n: f64) -> String {
    if n == f64::INFINITY {
        return "∞".to_string();
    }
    if n < 100.0 {
        if n == n.trunc() {
            return format!("{}", n as i64);
        }
        let s = format!("{n:.1}");
        return s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if n < 1000.0 {
        return format!("{}", n as i64);
    }
    let thousands = (n.log10() / 3.0).floor() as usize;
    if thousands < SUFFIXES.len() {
        return format!("{:.2}{}", n / THOUSANDS[thousands], SUFFIXES[thousands]);
    }
    format!("{n:.2e}").replace('e', "e+")
}

/// Radiation needed to go from level `level` to the next one.
pub fn need(level: u32) -> f64 {
    25.0 * (level as f64).powf(2.4)
}

/// Cost of the next rank of a prestige or supernova talent (fixed list or geometric growth).
fn rank_cost(talent: &data::Perk, rank: u32) -> i64 {
    match talent.costs {
        Some(costs) => costs[rank as usize],
        None => (talent.base * talent.growth.powi(rank as i32)).round_ties_even() as i64,
    }
}

// --- Records ---------------------------------------------------------------------

/// Sounds to play, drained by the UI (the oldest are dropped beyond SFX_MAX).
#[derive(Clone, Debug, Default)]
pub struct Sfx(VecDeque<&'static str>);

impl Sfx {
    pub fn push_back(&mut self, name: &'static str) {
        if self.0.len() >= SFX_MAX {
            self.0.pop_front();
        }
        self.0.push_back(name);
    }
}

impl Deref for Sfx {
    type Target = VecDeque<&'static str>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Sfx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Banner color in the UI (color `msg_<name>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MsgKind {
    Info,
    Good,
    Bad,
    Loot,
    Star,
    Epic,
}

impl MsgKind {
    pub const ALL: [MsgKind; 6] = [
        MsgKind::Info,
        MsgKind::Good,
        MsgKind::Bad,
        MsgKind::Loot,
        MsgKind::Star,
        MsgKind::Epic,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MsgKind::Info => "info",
            MsgKind::Good => "good",
            MsgKind::Bad => "bad",
            MsgKind::Loot => "loot",
            MsgKind::Star => "star",
            MsgKind::Epic => "epic",
        }
    }

    pub fn from_name(name: &str) -> Option<MsgKind> {
        MsgKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub text: String,
    pub time: f64,
    pub kind: MsgKind,
    /// Short title shown in big letters above the banner.
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffKind {
    Prod,
    Click,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Buff {
    pub kind: BuffKind,
    pub mult: f64,
    pub until: f64,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enemy {
    pub kind: EnemyKind,
    /// May be in the future: a wave member that hasn't entered yet.
    pub start: f64,
    pub dur: f64,
    pub hits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inspector {
    pub start: f64,
    pub dur: f64,
    pub hits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub letter: char,
    pub start: f64,
    pub dur: f64,
}

/// Critical mass in progress: the cursor goes back and forth, F at the right time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Qte {
    pub t: f64,
    pub center: f64,
    pub green: f64,
    pub yellow: f64,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QteResult {
    pub text: String,
    pub color: &'static str,
    pub until: f64,
}

/// Unit counts, in data::UNITS order, indexed by unit id (`g.units["soldier"]`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Units(pub [u64; UNIT_COUNT]);

impl Units {
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, u64)> + '_ {
        UNITS.iter().zip(self.0).map(|(u, n)| (u.id, n))
    }

    pub fn total(&self) -> u64 {
        self.0.iter().sum()
    }

    /// At least one unit of each type.
    pub fn all(&self) -> bool {
        self.0.iter().all(|&n| n > 0)
    }

    pub fn any(&self) -> bool {
        self.0.iter().any(|&n| n > 0)
    }
}

fn unit_slot(id: &str) -> usize {
    data::unit_index(id).unwrap_or_else(|| panic!("unknown unit {id}"))
}

impl Index<&str> for Units {
    type Output = u64;
    fn index(&self, id: &str) -> &u64 {
        &self.0[unit_slot(id)]
    }
}

impl IndexMut<&str> for Units {
    fn index_mut(&mut self, id: &str) -> &mut u64 {
        &mut self.0[unit_slot(id)]
    }
}

/// Whole-game statistics (survive prestige).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    #[serde(deserialize_with = "or_default")]
    pub clicks: u64,
    #[serde(deserialize_with = "or_default")]
    pub drops: u64,
    #[serde(deserialize_with = "or_default")]
    pub perfects: u64,
    #[serde(deserialize_with = "or_default")]
    pub inspectors: u64,
    #[serde(deserialize_with = "or_default")]
    pub frozen: u64,
    #[serde(deserialize_with = "or_default")]
    pub overheats: u64,
    #[serde(deserialize_with = "or_default")]
    pub mutations: u64,
    #[serde(deserialize_with = "or_default")]
    pub max_combo: bool,
    #[serde(deserialize_with = "or_default")]
    pub best_prestige_time: Option<f64>,
    #[serde(deserialize_with = "or_default")]
    pub max_level: u32,
    #[serde(deserialize_with = "or_default")]
    pub total_time: f64,
    #[serde(deserialize_with = "or_default")]
    pub iso_earned: i64,
    #[serde(deserialize_with = "or_default")]
    pub enemies: u64,
}

/// Stats of the current run (end-of-run summary).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunStats {
    #[serde(deserialize_with = "or_default")]
    pub clicks: u64,
    #[serde(deserialize_with = "or_default")]
    pub best_click: f64,
    #[serde(deserialize_with = "or_default")]
    pub crits: u64,
    #[serde(deserialize_with = "or_default")]
    pub earned: f64,
    #[serde(deserialize_with = "or_default")]
    pub chests: Vec<String>,
    #[serde(deserialize_with = "or_default")]
    pub inspectors: u64,
    #[serde(deserialize_with = "or_default")]
    pub perfects: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportKind {
    Prestige,
    Supernova,
}

/// Summary of the last finished run (end screen, Infos tab). Saved; dropped on load if anything is invalid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunReport {
    pub kind: ReportKind,
    pub gain: i64,
    pub time: f64,
    pub level: u32,
    pub cap: u32,
    pub earned: f64,
    pub clicks: u64,
    pub best_click: f64,
    pub crits: u64,
    pub perfects: u64,
    pub inspectors: u64,
    pub chests: Vec<String>,
    pub units: u64,
    pub mutations: Vec<String>,
    pub synergies: Vec<String>,
    pub number: u64,
}

// --- Save format -------------------------------------------------------------------

/// Field read leniently: missing or of the wrong type → None (the game keeps its default value).
fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned>(d: D) -> Result<Option<T>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).ok())
}

/// Field read leniently, its type's default value if invalid.
fn or_default<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(
    d: D,
) -> Result<T, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).unwrap_or_default())
}

/// Map whose invalid entries are dropped one by one (None if it is not a map).
fn lenient_map<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    d: D,
) -> Result<Option<BTreeMap<String, T>>, D::Error> {
    let Value::Object(map) = Value::deserialize(d)? else {
        return Ok(None);
    };
    Ok(Some(
        map.into_iter()
            .filter_map(|(k, v)| Some((k, serde_json::from_value(v).ok()?)))
            .collect(),
    ))
}

/// List of strings whose other elements are dropped (None if it is not a list).
fn lenient_strings<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<String>>, D::Error> {
    let Value::Array(items) = Value::deserialize(d)? else {
        return Ok(None);
    };
    Ok(Some(
        items
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    ))
}

/// Saved inspector or enemy, in relative time. elapsed may be slightly negative: an enemy of a wave
/// that hasn't entered yet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkerData {
    pub elapsed: f64,
    pub dur: f64,
    pub hits: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemyData {
    pub kind: String,
    #[serde(flatten)]
    pub walker: WalkerData,
}

/// Current threats and deadlines, in relative time (g.time restarts from 0 on load).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThreatsData {
    #[serde(deserialize_with = "lenient")]
    pub inspector: Option<WalkerData>,
    /// Raw entries: each one is validated separately.
    #[serde(deserialize_with = "lenient")]
    pub enemies: Option<Vec<Value>>,
    #[serde(deserialize_with = "lenient")]
    pub wave: Option<u32>,
    #[serde(deserialize_with = "lenient")]
    pub next_inspector: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub next_enemy: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub bribe: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub freeze: Option<f64>,
}

/// Content of a save file. Every field is optional on reading: a missing or invalid value is ignored on its
/// own (the fresh game's value is kept), then Game::load validates the values.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    #[serde(deserialize_with = "lenient")]
    pub version: Option<i64>,
    #[serde(deserialize_with = "lenient")]
    pub isotopes: Option<i64>,
    #[serde(deserialize_with = "lenient_map")]
    pub ptalents: Option<BTreeMap<String, u32>>,
    #[serde(deserialize_with = "lenient")]
    pub prestige_count: Option<u64>,
    #[serde(deserialize_with = "lenient")]
    pub skin: Option<String>,
    #[serde(deserialize_with = "lenient")]
    pub challenge: Option<Option<String>>,
    #[serde(deserialize_with = "lenient")]
    pub stats: Option<Stats>,
    #[serde(deserialize_with = "lenient")]
    pub exploded: Option<bool>,
    #[serde(deserialize_with = "lenient")]
    pub level: Option<u32>,
    #[serde(deserialize_with = "lenient")]
    pub progress: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub radiation: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub pt: Option<i64>,
    #[serde(deserialize_with = "lenient_map")]
    pub units: Option<BTreeMap<String, u64>>,
    #[serde(deserialize_with = "lenient_map")]
    pub talents: Option<BTreeMap<String, u32>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub mutations: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub pending_mutation: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub pending_chest: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient")]
    pub mut_index: Option<u32>,
    #[serde(deserialize_with = "lenient")]
    pub mut_queue: Option<u32>,
    #[serde(deserialize_with = "lenient")]
    pub run_time: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub crit_gauge: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub next_pt_level: Option<u32>,
    #[serde(deserialize_with = "lenient")]
    pub audio: Option<String>,
    #[serde(deserialize_with = "lenient")]
    pub volume: Option<f64>,
    #[serde(deserialize_with = "lenient")]
    pub stars: Option<i64>,
    #[serde(deserialize_with = "lenient_map")]
    pub stalents: Option<BTreeMap<String, u32>>,
    #[serde(deserialize_with = "lenient")]
    pub supernova_count: Option<u64>,
    #[serde(deserialize_with = "lenient")]
    pub run_stats: Option<RunStats>,
    #[serde(deserialize_with = "lenient")]
    pub last_run: Option<RunReport>,
    #[serde(deserialize_with = "lenient")]
    pub threats: Option<ThreatsData>,
    #[serde(deserialize_with = "lenient_strings")]
    pub achievements: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub challenges_done: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub synergies_found: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient_strings")]
    pub skins: Option<Vec<String>>,
    #[serde(deserialize_with = "lenient")]
    pub wall_time: Option<f64>,
}

fn is_delay(v: Option<f64>) -> Option<f64> {
    v.filter(|v| (0.0..=THREAT_MAX).contains(v))
}

/// Saved inspector or enemy → (elapsed, dur, hits) if valid.
fn load_walker(w: &WalkerData, max_hits: u32) -> Option<(f64, f64, u32)> {
    let WalkerData { elapsed, dur, hits } = *w;
    let valid =
        (-10.0..dur).contains(&elapsed) && dur > 0.0 && dur <= THREAT_MAX && hits < max_hits;
    valid.then_some((elapsed, dur, hits))
}

/// Wall-clock time in seconds since the epoch.
pub fn wall_time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// --- Game --------------------------------------------------------------------------

pub struct Game {
    /// Injectable randomness (tests pass a seeded or fixed one).
    pub rng: Box<dyn Rng>,
    pub time: f64,
    // Permanent (survives prestige)
    pub isotopes: i64,
    pub ptalents: BTreeMap<&'static str, u32>,
    pub prestige_count: u64,
    pub stars: i64,
    pub stalents: BTreeMap<&'static str, u32>,
    pub supernova_count: u64,
    pub achievements: BTreeSet<&'static str>,
    pub synergies_found: BTreeSet<&'static str>,
    /// Summary of the last finished run (end screen, Infos tab).
    pub last_run: Option<RunReport>,
    /// The UI shows the end screen then resets it to false.
    pub report_ready: bool,
    /// (challenge id, "start" or "end"): the UI shows the story then resets it to None.
    pub pending_story: Option<(&'static str, &'static str)>,
    pub challenges_done: BTreeSet<&'static str>,
    pub skins: BTreeSet<&'static str>,
    pub skin: &'static str,
    pub audio: &'static str,
    pub volume: u32,
    pub challenge: Option<&'static str>,
    pub stats: Stats,
    pub messages: VecDeque<Message>,
    pub qte_result: Option<QteResult>,
    pub flash_until: f64,
    /// Sounds to play, drained by the UI.
    pub sfx: Sfx,
    // Current run (reset_run)
    pub exploded: bool,
    pub level: u32,
    pub progress: f64,
    pub next_pt_level: u32,
    pub radiation: f64,
    pub pt: i64,
    pub units: Units,
    pub talents: BTreeMap<&'static str, u32>,
    pub mutations: Vec<&'static str>,
    pub pending_mutation: Option<Vec<&'static str>>,
    pub pending_chest: Option<Vec<&'static str>>,
    pub mut_index: u32,
    pub mut_queue: u32,
    pub run_time: f64,
    pub run_stats: RunStats,
    pub combo: f64,
    pub last_click: f64,
    pub click_times: VecDeque<f64>,
    pub overheat_until: f64,
    pub last_crit: f64,
    pub buffs: Vec<Buff>,
    pub freeze_until: f64,
    /// Bribes: no inspector before this time.
    pub bribe_until: f64,
    pub crit_gauge: f64,
    pub qte: Option<Qte>,
    pub plane: Option<Plane>,
    pub inspector: Option<Inspector>,
    pub enemies: Vec<Enemy>,
    /// Number of the last wave (Invasion challenge).
    pub wave: u32,
    pub next_enemy: f64,
    pub next_plane: f64,
    pub next_inspector: f64,
    pub auto_acc: f64,
    pub autobuy_acc: f64,
    pub ach_acc: f64,
}

impl Default for Game {
    fn default() -> Self {
        Game::new(Box::new(StdRandom::new()))
    }
}

impl Game {
    pub fn new(rng: Box<dyn Rng>) -> Self {
        let mut g = Game {
            rng,
            time: 0.0,
            isotopes: 0,
            ptalents: BTreeMap::new(),
            prestige_count: 0,
            stars: 0,
            stalents: BTreeMap::new(),
            supernova_count: 0,
            achievements: BTreeSet::new(),
            synergies_found: BTreeSet::new(),
            last_run: None,
            report_ready: false,
            pending_story: None,
            challenges_done: BTreeSet::new(),
            skins: BTreeSet::from(["classic"]),
            skin: "classic",
            audio: "all",
            volume: 100,
            challenge: None,
            stats: Stats::default(),
            messages: VecDeque::new(),
            qte_result: None,
            flash_until: 0.0,
            sfx: Sfx::default(),
            // Run values: set by reset_run.
            exploded: false,
            level: 0,
            progress: 0.0,
            next_pt_level: 0,
            radiation: 0.0,
            pt: 0,
            units: Units::default(),
            talents: BTreeMap::new(),
            mutations: Vec::new(),
            pending_mutation: None,
            pending_chest: None,
            mut_index: 0,
            mut_queue: 0,
            run_time: 0.0,
            run_stats: RunStats::default(),
            combo: 0.0,
            last_click: -10.0,
            click_times: VecDeque::new(),
            overheat_until: 0.0,
            last_crit: -10.0,
            buffs: Vec::new(),
            freeze_until: 0.0,
            bribe_until: 0.0,
            crit_gauge: 0.0,
            qte: None,
            plane: None,
            inspector: None,
            enemies: Vec::new(),
            wave: 0,
            next_enemy: 0.0,
            next_plane: 0.0,
            next_inspector: 0.0,
            auto_acc: 0.0,
            autobuy_acc: 0.0,
            ach_acc: 0.0,
        };
        g.reset_run();
        g
    }

    /// Number formatted for messages, hidden during the Brouillard de guerre challenge.
    pub fn fmt(&self, n: f64) -> String {
        if self.challenge == Some("blind") {
            "???".to_string()
        } else {
            fmt(n)
        }
    }

    // --- Run ---------------------------------------------------------------------

    /// Talents kept by "p_keep" (none without it).
    fn kept_talents(&self) -> Vec<(&'static str, u32)> {
        if self.pr("p_keep") == 0 {
            return Vec::new();
        }
        data::KEPT_TALENTS
            .iter()
            .map(|&t| (t, self.tr(t)))
            .collect()
    }

    pub fn reset_run(&mut self) {
        let kept = self.kept_talents();
        self.exploded = false;
        self.level = 0;
        self.progress = 0.0;
        self.next_pt_level = 0;
        self.radiation = 0.0;
        self.pt = 5 * self.pr("p_pt") as i64
            + if self.challenges_done.contains("ascete") {
                10
            } else {
                0
            };
        self.units = Units::default();
        self.talents = kept
            .into_iter()
            .filter(|&(t, r)| r > 0 && self.talent_allowed(t))
            .collect();
        if self.pr("p_vets") > 0 {
            self.talents.insert("u_soldier", 1);
            self.units["soldier"] = 10 * self.pr("p_vets") as u64;
        }
        if self.challenge == Some("onearm") {
            self.talents.insert("u_soldier", 1);
            self.units["soldier"] = self.units["soldier"].max(3);
        }
        self.mutations = Vec::new();
        self.pending_mutation = None;
        self.pending_chest = None;
        self.mut_index = 0;
        self.mut_queue = 0;
        self.run_time = 0.0;
        self.run_stats = RunStats::default();
        self.combo = 0.0;
        self.last_click = -10.0;
        self.click_times = VecDeque::new();
        self.overheat_until = 0.0;
        self.last_crit = -10.0;
        self.buffs = Vec::new();
        self.freeze_until = 0.0;
        self.bribe_until = 0.0;
        self.crit_gauge = 0.0;
        self.qte = None;
        self.plane = None;
        self.inspector = None;
        self.enemies = Vec::new();
        self.wave = 0;
        self.schedule_enemy(true);
        self.next_plane = self.time + self.rng.uniform(40.0, 90.0);
        self.schedule_inspector(true);
        self.auto_acc = 0.0;
        self.autobuy_acc = 0.0;
        self.ach_acc = 0.0;
    }

    pub fn schedule_inspector(&mut self, first: bool) {
        let delay = if self.challenge == Some("watched") {
            self.rng.uniform(30.0, 60.0)
        } else {
            let base = if first {
                self.rng.uniform(120.0, 240.0)
            } else {
                self.rng.uniform(150.0, 300.0)
            };
            base / events::threat_pressure(self)
        };
        self.next_inspector = self.time + delay;
    }

    pub fn schedule_enemy(&mut self, first: bool) {
        let delay = if self.challenge == Some("invasion") {
            self.rng.uniform(15.0, 30.0)
        } else {
            let base = if first {
                self.rng.uniform(60.0, 120.0)
            } else {
                self.rng.uniform(90.0, 180.0)
            };
            base / events::threat_pressure(self)
        };
        self.next_enemy = self.time + delay;
    }

    /// kind: banner color in the UI; title: short title shown in big letters above the banner.
    /// A message repeating the last one replaces it.
    pub fn log(&mut self, msg: impl Into<String>, kind: MsgKind, title: Option<&str>) {
        let text = msg.into();
        if self.messages.back().is_some_and(|m| m.text == text) {
            self.messages.pop_back();
        }
        if self.messages.len() >= MESSAGES_MAX {
            self.messages.pop_front();
        }
        self.messages.push_back(Message {
            text,
            time: self.time,
            kind,
            title: title.map(str::to_string),
        });
    }

    pub fn tr(&self, tid: &str) -> u32 {
        self.talents.get(tid).copied().unwrap_or(0)
    }

    pub fn pr(&self, pid: &str) -> u32 {
        self.ptalents.get(pid).copied().unwrap_or(0)
    }

    pub fn sr(&self, sid: &str) -> u32 {
        self.stalents.get(sid).copied().unwrap_or(0)
    }

    pub fn has_mut(&self, mid: &str) -> bool {
        self.mutations.contains(&mid)
    }

    pub fn has_syn(&self, sid: &str) -> bool {
        let (a, b) = events::synergy(sid).pair;
        self.has_mut(a) && self.has_mut(b)
    }

    pub fn synergies(&self) -> Vec<&'static events::Synergy> {
        events::SYNERGIES
            .iter()
            .filter(|s| self.has_syn(s.id))
            .collect()
    }

    /// Max level: 100 × 10^rank of "Expansion du cœur".
    pub fn cap(&self) -> u32 {
        data::MAX_CAP.min(data::BASE_CAP.saturating_mul(10u32.saturating_pow(self.pr("p_cap"))))
    }

    pub fn frozen(&self) -> bool {
        self.time < self.freeze_until
    }

    pub fn overheated(&self) -> bool {
        self.time < self.overheat_until
    }

    // --- Multipliers -------------------------------------------------------------

    pub fn buff_mult(&self, kind: BuffKind) -> f64 {
        self.buffs
            .iter()
            .filter(|b| b.kind == kind)
            .map(|b| b.mult)
            .product()
    }

    pub fn add_buff(&mut self, kind: BuffKind, mult: f64, duration: f64, name: &'static str) {
        self.buffs.push(Buff {
            kind,
            mult,
            until: self.time + duration,
            name,
        });
    }

    pub fn max_combo(&self) -> u32 {
        3 + self.tr("t_combo") + if self.has_mut("berserk") { 2 } else { 0 }
    }

    pub fn combo_mult(&self) -> f64 {
        1.0 + self.combo
    }

    pub fn overheat_threshold(&self) -> u32 {
        if self.challenge == Some("hothead") {
            return 6;
        }
        12 + 3 * self.tr("t_cool")
            + if self.challenges_done.contains("hothead") {
                5
            } else {
                0
            }
    }

    pub fn crit_mult(&self) -> f64 {
        if self.challenges_done.contains("nanar") {
            15.0
        } else {
            10.0
        }
    }

    pub fn crit_chance(&self) -> f64 {
        if self.challenge == Some("unlucky") {
            return 0.0;
        }
        0.05 * self.tr("t_crit") as f64
            + if self.has_mut("glow") { 0.15 } else { 0.0 }
            + if self.challenges_done.contains("unlucky") {
                0.1
            } else {
                0.0
            }
    }

    pub fn global_mult(&self) -> f64 {
        let mut m = (1.0 + 0.1 * self.tr("l_global") as f64)
            * 1.5f64.powi(self.pr("p_prod") as i32)
            * 2f64.powi(self.sr("s_prod") as i32);
        m *= 1.0 + 0.02 * self.achievements.len() as f64;
        m *= 1.0 + 0.001 * self.tr("l_synergy") as f64 * self.units.total() as f64;
        if self.has_mut("fallout") {
            m *= 1.5;
        }
        if self.has_mut("gamble") {
            m *= 3.0;
        }
        if self.has_syn("twins") {
            m *= 1.5;
        }
        if self.challenges_done.contains("onearm") {
            m *= 2.0;
        }
        m * self.buff_mult(BuffKind::Prod)
    }

    pub fn unit_milestones(&self, uid: &str) -> u32 {
        data::UNIT_MILESTONES
            .iter()
            .filter(|&&m| self.units[uid] >= m)
            .count() as u32
    }

    pub fn next_milestone(&self, uid: &str) -> Option<u64> {
        data::UNIT_MILESTONES
            .iter()
            .copied()
            .find(|&m| self.units[uid] < m)
    }

    /// Production of one unit of this type (multipliers included, freeze excluded).
    pub fn unit_prod(&self, uid: &str) -> f64 {
        let mut p = data::unit(uid).prod
            * 2f64.powi(self.tr(data::UNIT_TALENT_IDS[unit_slot(uid)].1) as i32)
            * data::MILESTONE_MULT.powi(self.unit_milestones(uid) as i32);
        if self.has_mut("spores") {
            let per_soldier = if self.has_syn("proliferation") {
                0.02
            } else {
                0.01
            };
            p *= 1.0 + per_soldier * self.units["soldier"] as f64;
        }
        p * self.global_mult()
    }

    pub fn production(&self) -> f64 {
        if self.frozen() || !self.exploded {
            return 0.0;
        }
        let mut p: f64 = self
            .units
            .iter()
            .filter(|&(_, n)| n > 0)
            .map(|(uid, n)| n as f64 * self.unit_prod(uid))
            .sum();
        if self.challenge == Some("nanar") {
            p *= 0.5;
        }
        let protesters = self.enemies.iter().any(|e| e.kind == EnemyKind::Hippies);
        if protesters {
            p / events::protest_divisor(self)
        } else {
            p
        }
    }

    pub fn click_value(&self, manual: bool) -> f64 {
        let mut m = 2f64.powi(self.pr("p_click") as i32) * self.buff_mult(BuffKind::Click);
        if self.has_mut("twin") {
            m *= 2.0;
        }
        if self.has_mut("acid") && !self.has_syn("storm") {
            m *= 0.5;
        }
        if self.challenges_done.contains("pacifist") {
            m *= 3.0;
        }
        let mut v = (1.0 + 2.0 * self.tr("t_click") as f64) * m;
        if manual {
            v *= self.combo_mult();
        }
        v + self.production() * 0.02 * self.tr("t_shock") as f64
    }

    // --- Actions -----------------------------------------------------------------

    pub fn explode(&mut self) {
        self.exploded = true;
        self.level = 1;
        self.progress = 0.0;
        self.flash_until = self.time + 0.8;
        self.sfx.push_back("boom");
        self.log(
            "BOOM ! Le champignon est né.",
            MsgKind::Epic,
            Some("BOOM !"),
        );
    }

    /// A press on SPACE (manual) or by the auto-detonator.
    pub fn click(&mut self, manual: bool) {
        if !self.exploded {
            self.explode();
            return;
        }
        if manual {
            if self.challenge == Some("onearm") {
                self.sfx.push_back("nope");
                self.log(
                    "Défi Manchot : ESPACE est interdit !",
                    MsgKind::Bad,
                    Some("NON !"),
                );
                return;
            }
            if self.overheated() {
                self.sfx.push_back("jam"); // press rejected, but audible: otherwise spamming seems to cut the sound
                return;
            }
            if self.press_overheats() {
                self.overheat();
                return;
            }
            let combo_top = (self.max_combo() - 1) as f64;
            self.combo = combo_top.min(self.combo + COMBO_STEP);
            if self.combo >= combo_top {
                self.stats.max_combo = true;
            }
            self.last_click = self.time;
            self.stats.clicks += 1;
            let pitch = (CLICK_PITCHES - 1)
                .min((self.combo / combo_top.max(1.0) * CLICK_PITCHES as f64) as usize);
            self.sfx.push_back(CLICK_SOUNDS[pitch]);
        }
        if self.challenge == Some("nanar") && self.rng.random() < NANAR_FIZZLE {
            if manual {
                self.run_stats.clicks += 1;
                self.sfx.push_back("fizzle");
            }
            return;
        }
        let mut v = self.click_value(manual);
        if self.rng.random() < self.crit_chance() {
            v *= self.crit_mult();
            self.last_crit = self.time;
            if manual {
                self.run_stats.crits += 1;
            }
            if self.has_syn("chain") && self.qte.is_none() {
                self.crit_gauge = (self.crit_gauge + 0.03).min(1.0);
            }
            if manual {
                self.sfx.push_back("crit");
            }
        }
        if manual {
            self.run_stats.clicks += 1;
            self.run_stats.best_click = self.run_stats.best_click.max(v);
        }
        self.gain(v);
    }

    /// Records the press; true if it exceeds the press threshold over the last second.
    fn press_overheats(&mut self) -> bool {
        self.click_times.push_back(self.time);
        while self
            .click_times
            .front()
            .is_some_and(|&t| t < self.time - 1.0)
        {
            self.click_times.pop_front();
        }
        let immune = self.has_mut("asbestos") && self.challenge != Some("hothead");
        self.click_times.len() > self.overheat_threshold() as usize && !immune
    }

    fn overheat(&mut self) {
        self.overheat_until = self.time
            + if self.challenge == Some("hothead") {
                6.0
            } else {
                3.0
            };
        self.combo = 0.0;
        self.click_times.clear();
        self.stats.overheats += 1;
        self.sfx.push_back("overheat");
        self.log(
            "SURCHAUFFE ! Doucement sur le détonateur...",
            MsgKind::Bad,
            Some("CHAUD !"),
        );
    }

    pub fn gain(&mut self, amount: f64) {
        self.radiation += amount;
        self.run_stats.earned += amount;
        if self.level >= self.cap() || !self.exploded {
            return;
        }
        self.progress += amount;
        while self.progress >= need(self.level) && self.level < self.cap() {
            self.progress -= need(self.level);
            self.level += 1;
            self.on_level_up();
        }
        if self.level >= self.cap() {
            self.progress = 0.0;
        }
    }

    pub fn on_level_up(&mut self) {
        let lvl = self.level;
        let below_base_cap = lvl <= data::BASE_CAP;
        if below_base_cap || lvl >= self.next_pt_level {
            self.sfx.push_back("level");
            let round_level = below_base_cap && lvl.is_multiple_of(PT_BONUS_EVERY);
            self.pt +=
                1 + if round_level { 5 } else { 0 } + if self.has_mut("mitosis") { 1 } else { 0 };
            self.next_pt_level = (lvl as f64 * PT_LEVEL_GROWTH).ceil() as u32;
        }
        self.stats.max_level = self.stats.max_level.max(lvl);
        while lvl as u64 >= events::mutation_milestone(self.mut_index) {
            self.mut_index += 1;
            if self.pending_mutation.is_some() {
                self.mut_queue += 1;
            } else {
                events::offer_mutation(self);
            }
        }
        if lvl == self.cap() {
            self.log(
                "NIVEAU MAX ! Le prestige est disponible (onglet 3).",
                MsgKind::Epic,
                Some("MAX !"),
            );
        }
        if let Some(ch) = self.challenge.map(data::challenge)
            && lvl >= ch.target
        {
            self.complete_challenge(ch);
        }
    }

    fn complete_challenge(&mut self, ch: &'static data::Challenge) {
        self.challenges_done.insert(ch.id);
        self.skins.insert(ch.skin);
        self.challenge = None;
        self.pending_story = Some((ch.id, "end"));
        if ch.id == "watched" {
            self.schedule_inspector(false);
        }
        self.sfx.push_back("fanfare");
        self.log(
            format!("Défi {} réussi ! {}", ch.name, ch.reward),
            MsgKind::Star,
            Some("DÉFI OK"),
        );
    }

    pub fn unit_unlocked(&self, uid: &str) -> bool {
        self.tr(data::UNIT_TALENT_IDS[unit_slot(uid)].0) > 0
    }

    /// Cost of the next unit (or of the unit number `count`).
    pub fn unit_cost(&self, uid: &str, count: Option<u64>) -> f64 {
        let u = data::unit(uid);
        let n = count.unwrap_or(self.units[uid]);
        let mut c = u.cost
            * data::UNIT_COST_GROWTH.powf(n as f64)
            * (1.0 - 0.04 * self.tr("l_discount") as f64);
        if self.challenge == Some("inflation") {
            c *= 4.0;
        }
        if self.challenges_done.contains("inflation") {
            c *= 0.9;
        }
        c * if self.has_mut("acid") { 0.7 } else { 1.0 }
    }

    pub fn can_buy_unit(&self, uid: &str) -> bool {
        self.challenge != Some("pacifist")
            && self.unit_unlocked(uid)
            && self.radiation >= self.unit_cost(uid, None)
    }

    pub fn buy_unit(&mut self, uid: &str) -> bool {
        self.buy_unit_n(uid, 1) == 1
    }

    /// Adds k units and announces each milestone crossed.
    pub fn add_units(&mut self, uid: &str, k: u64) {
        let before = self.units[uid];
        self.units[uid] += k;
        for m in data::UNIT_MILESTONES {
            if before < m && m <= self.units[uid] {
                self.sfx.push_back("milestone");
                let text = format!("Palier : {m} {}s, production ×2 !", data::unit(uid).name);
                self.log(text, MsgKind::Good, Some("PALIER"));
            }
        }
    }

    /// Total cost of the next k units.
    pub fn unit_cost_n(&self, uid: &str, k: u64) -> f64 {
        let n = self.units[uid];
        (0..k).map(|i| self.unit_cost(uid, Some(n + i))).sum()
    }

    /// Number of units affordable at once with the current radiation (capped at MAX_BULK).
    pub fn affordable(&self, uid: &str) -> u64 {
        if !self.can_buy_unit(uid) {
            return 0;
        }
        let (mut k, mut total, n) = (0, 0.0, self.units[uid]);
        while k < MAX_BULK && total + self.unit_cost(uid, Some(n + k)) <= self.radiation {
            total += self.unit_cost(uid, Some(n + k));
            k += 1;
        }
        k
    }

    /// Buys exactly k units (all or nothing); returns the number bought.
    pub fn buy_unit_n(&mut self, uid: &str, k: u64) -> u64 {
        let total = self.unit_cost_n(uid, k);
        if k == 0 || !self.can_buy_unit(uid) || self.radiation < total {
            return 0;
        }
        self.radiation = (self.radiation - total).max(0.0); // in one go: no rounding drift along the way
        self.add_units(uid, k);
        k
    }

    pub fn buy_unit_max(&mut self, uid: &str) -> u64 {
        self.buy_unit_n(uid, self.affordable(uid))
    }

    pub fn talent_cost(&self, tid: &str) -> i64 {
        let t = data::talent(tid);
        t.cost.0 + t.cost.1 * self.tr(tid) as i64
    }

    /// Ascète challenge: unit unlocks only.
    pub fn talent_allowed(&self, tid: &str) -> bool {
        self.challenge != Some("ascete") || tid.starts_with("u_")
    }

    pub fn talent_available(&self, tid: &str) -> bool {
        let t = data::talent(tid);
        self.tr(tid) < t.max && t.req.iter().all(|&(r, n)| self.tr(r) >= n)
    }

    pub fn buy_talent(&mut self, tid: &str) -> bool {
        if !self.talent_available(tid)
            || !self.talent_allowed(tid)
            || self.pt < self.talent_cost(tid)
        {
            return false;
        }
        self.pt -= self.talent_cost(tid);
        let id = data::talent(tid).id;
        self.talents.insert(id, self.tr(id) + 1);
        true
    }

    /// Cost of the next rank (infinite once maxed).
    pub fn ptalent_cost(&self, pid: &str) -> f64 {
        let p = data::ptalent(pid);
        let r = self.pr(pid);
        if r >= p.max {
            return f64::INFINITY;
        }
        if pid == "p_cap" {
            let cost = p.costs.expect("p_cap has fixed costs")[r as usize] as f64;
            return (cost * (1.0 - 0.2 * self.sr("s_core") as f64)).round_ties_even();
        }
        rank_cost(p, r) as f64
    }

    pub fn buy_ptalent(&mut self, pid: &str) -> bool {
        let p = data::ptalent(pid);
        if self.pr(pid) >= p.max || (self.isotopes as f64) < self.ptalent_cost(pid) {
            return false;
        }
        self.isotopes -= self.ptalent_cost(pid) as i64;
        self.ptalents.insert(p.id, self.pr(pid) + 1);
        if pid == "p_cap" {
            let text = format!("Le cœur s'étend : niveau max {} !", fmt(self.cap() as f64));
            self.log(text, MsgKind::Epic, Some("CŒUR +"));
        }
        true
    }

    // --- Prestige ----------------------------------------------------------------

    pub fn prestige_unlocked(&self) -> bool {
        self.prestige_count > 0 || self.level >= self.cap()
    }

    pub fn can_prestige(&self) -> bool {
        self.level >= self.cap()
    }

    pub fn prestige_gain(&self) -> i64 {
        let mut g =
            10.0 * (self.level as f64 / 100.0).powf(0.6) * (1.0 + 0.25 * self.pr("p_iso") as f64);
        if self.challenges_done.contains("race") {
            g *= 1.5;
        }
        (g * 2f64.powi(self.sr("s_iso") as i32)) as i64
    }

    /// Summary of the run that is ending (prestige or supernova).
    pub fn make_report(&mut self, kind: ReportKind, gain: i64) {
        let rs = &self.run_stats;
        self.last_run = Some(RunReport {
            kind,
            gain,
            time: self.run_time,
            level: self.level,
            cap: self.cap(),
            earned: rs.earned,
            clicks: rs.clicks,
            best_click: rs.best_click,
            crits: rs.crits,
            perfects: rs.perfects,
            inspectors: rs.inspectors,
            chests: rs.chests.clone(),
            units: self.units.total(),
            mutations: self
                .mutations
                .iter()
                .map(|m| events::mutation(m).name.to_string())
                .collect(),
            synergies: self
                .synergies()
                .iter()
                .map(|s| s.name.to_string())
                .collect(),
            number: self.prestige_count,
        });
        self.report_ready = true;
    }

    /// Normal prestige (at max level) or start of a challenge (always possible after a 1st prestige).
    pub fn prestige(&mut self, challenge: Option<&str>) -> bool {
        let challenge = challenge.map(|c| data::challenge(c).id);
        if self.can_prestige() {
            let gain = self.prestige_gain();
            self.isotopes += gain;
            self.stats.iso_earned += gain;
            self.prestige_count += 1;
            self.make_report(ReportKind::Prestige, gain);
            if self
                .stats
                .best_prestige_time
                .is_none_or(|best| self.run_time < best)
            {
                self.stats.best_prestige_time = Some(self.run_time);
            }
            if self.prestige_count >= 5 {
                self.skins.insert("plasma");
            }
            self.sfx.push_back("prestige");
            self.log(
                format!("PRESTIGE ! +{gain} ⚛. Tout recommence..."),
                MsgKind::Epic,
                Some("PRESTIGE"),
            );
        } else if challenge.is_none() || self.prestige_count == 0 {
            return false;
        }
        self.challenge = challenge;
        if let Some(c) = challenge {
            self.log(
                format!("Défi {} lancé !", data::challenge(c).name),
                MsgKind::Info,
                Some("DÉFI"),
            );
            self.pending_story = Some((c, "start"));
        }
        self.reset_run();
        true
    }

    // --- Supernova (2nd prestige tier) -----------------------------------------

    fn core_maxed(&self) -> bool {
        self.pr("p_cap") >= data::ptalent("p_cap").max
    }

    pub fn supernova_unlocked(&self) -> bool {
        self.supernova_count > 0 || self.core_maxed()
    }

    pub fn can_supernova(&self) -> bool {
        self.core_maxed() && self.level >= self.cap()
    }

    /// Stars: square root of the isotopes earned since the last supernova (current prestige included).
    pub fn supernova_gain(&self) -> i64 {
        let earned = self.stats.iso_earned
            + if self.can_prestige() {
                self.prestige_gain()
            } else {
                0
            };
        ((earned as f64 / data::SUPERNOVA_DIVISOR).sqrt() as i64).max(1)
    }

    /// Resets isotopes and the prestige tree; keeps achievements, skins, challenges and the prestige count.
    pub fn supernova(&mut self) -> bool {
        if !self.can_supernova() {
            return false;
        }
        let gain = self.supernova_gain();
        self.make_report(ReportKind::Supernova, gain);
        self.stars += gain;
        self.supernova_count += 1;
        self.isotopes = 0;
        self.ptalents = BTreeMap::new();
        self.stats.iso_earned = 0;
        self.challenge = None;
        self.sfx.push_back("supernova");
        self.log(
            format!("SUPERNOVA ! +{gain} ✦. L'univers recommence..."),
            MsgKind::Epic,
            Some("SUPERNOVA"),
        );
        self.reset_run();
        true
    }

    /// Isotopes owned + spent in the prestige tree (estimate of what was earned).
    pub fn isotopes_invested(&self) -> i64 {
        let spent: i64 = data::PTALENTS
            .iter()
            .flat_map(|p| (0..self.pr(p.id)).map(move |r| rank_cost(p, r)))
            .sum();
        self.isotopes + spent
    }

    /// Cost of the next rank (infinite once maxed).
    pub fn stalent_cost(&self, sid: &str) -> f64 {
        let p = data::stalent(sid);
        let r = self.sr(sid);
        if r >= p.max {
            return f64::INFINITY;
        }
        rank_cost(p, r) as f64
    }

    pub fn buy_stalent(&mut self, sid: &str) -> bool {
        if (self.stars as f64) < self.stalent_cost(sid) {
            return false;
        }
        self.stars -= self.stalent_cost(sid) as i64;
        self.stalents
            .insert(data::stalent(sid).id, self.sr(sid) + 1);
        true
    }

    /// Spends PT: unit unlocks first, otherwise the cheapest talent.
    pub fn auto_talents(&mut self) {
        loop {
            let avail: Vec<&'static str> = data::TALENTS
                .iter()
                .map(|t| t.id)
                .filter(|&t| {
                    self.talent_available(t)
                        && self.talent_allowed(t)
                        && self.pt >= self.talent_cost(t)
                })
                .collect();
            let pick = avail
                .iter()
                .find(|t| t.starts_with("u_"))
                .or_else(|| avail.iter().min_by_key(|t| self.talent_cost(t)));
            match pick {
                Some(&t) => {
                    self.buy_talent(t);
                }
                None => return,
            }
        }
    }

    /// +1 / -1 step; returns true if the volume changed.
    pub fn change_volume(&mut self, step: i32) -> bool {
        let old = self.volume;
        self.volume = (old as i32 + step * VOLUME_STEP as i32).clamp(0, 100) as u32;
        self.volume != old
    }

    pub fn abandon_challenge(&mut self) {
        if let Some(challenge) = self.challenge {
            self.log("Défi abandonné.", MsgKind::Info, Some("ABANDON"));
            self.challenge = None;
            if challenge == "watched" {
                self.schedule_inspector(false);
            }
        }
    }

    // --- Loop --------------------------------------------------------------------

    pub fn tick(&mut self, dt: f64) {
        self.time += dt;
        self.stats.total_time += dt;
        if !self.exploded {
            return;
        }
        self.run_time += dt;

        if self.time - self.last_click > COMBO_GRACE {
            let decay_rate = if self.has_syn("berserker") { 0.4 } else { 1.2 };
            self.combo = (self.combo - decay_rate * dt).max(0.0);
        }

        self.gain(self.production() * dt);
        self.auto_detonate(dt);

        self.autobuy_acc += dt;
        if self.autobuy_acc >= 1.0 {
            self.autobuy_acc = 0.0;
            self.autobuy();
            if self.sr("s_auto") > 0 {
                self.auto_talents();
            }
        }

        let now = self.time;
        self.buffs.retain(|b| b.until > now);
        self.fail_slow_challenge();

        events::tick(self, dt);

        self.ach_acc += dt;
        if self.ach_acc >= 1.0 {
            self.ach_acc = 0.0;
            self.check_achievements();
        }
    }

    fn auto_detonate(&mut self, dt: f64) {
        if self.tr("t_auto") > 0 && self.challenge != Some("onearm") {
            self.auto_acc += dt * self.tr("t_auto") as f64;
            while self.auto_acc >= 1.0 {
                self.auto_acc -= 1.0;
                self.click(false);
            }
        }
    }

    fn fail_slow_challenge(&mut self) {
        if let Some(ch) = self.challenge.map(data::challenge)
            && ch.time.is_some_and(|limit| self.run_time > limit)
        {
            self.challenge = None;
            self.log(
                format!("Défi {} échoué : trop lent !", ch.name),
                MsgKind::Bad,
                Some("ÉCHEC"),
            );
        }
    }

    pub fn autobuy(&mut self) {
        let rank = self.tr("l_autobuy") as usize;
        if rank == 0 {
            return;
        }
        let allowed: Vec<&'static str> = UNITS.iter().take(3 * rank).map(|u| u.id).collect();
        for _ in 0..50 {
            // The cheapest one (the first of them on a tie).
            let mut cheapest: Option<(&str, f64)> = None;
            for &u in allowed.iter().filter(|u| self.can_buy_unit(u)) {
                let cost = self.unit_cost(u, None);
                if cheapest.is_none_or(|(_, c)| cost < c) {
                    cheapest = Some((u, cost));
                }
            }
            let Some((u, _)) = cheapest else { break };
            self.buy_unit(u);
        }
    }

    pub fn check_achievements(&mut self) {
        for a in &events::ACHIEVEMENTS {
            if !self.achievements.contains(a.id) && (a.check)(self) {
                self.achievements.insert(a.id);
                self.sfx.push_back("achievement");
                self.log(
                    format!("★ Succès : {} (+2 % de production)", a.name),
                    MsgKind::Star,
                    Some("SUCCÈS"),
                );
            }
        }
    }

    // --- Saving ------------------------------------------------------------------

    /// Current threats and deadlines, in relative time (g.time restarts from 0 on load).
    pub fn threats_data(&self) -> ThreatsData {
        let left = |t: f64| (t - self.time).max(0.0);
        let walker = |start: f64, dur: f64, hits: u32| WalkerData {
            elapsed: self.time - start,
            dur,
            hits,
        };
        let enemies = self
            .enemies
            .iter()
            .map(|e| EnemyData {
                kind: e.kind.id().to_string(),
                walker: walker(e.start, e.dur, e.hits),
            })
            .map(|e| serde_json::to_value(e).expect("enemy serializes"))
            .collect();
        ThreatsData {
            inspector: self.inspector.map(|i| walker(i.start, i.dur, i.hits)),
            enemies: Some(enemies),
            wave: Some(self.wave),
            next_inspector: Some(left(self.next_inspector)),
            next_enemy: Some(left(self.next_enemy)),
            bribe: Some(left(self.bribe_until)),
            freeze: Some(left(self.freeze_until)),
        }
    }

    /// Restores threats_data; each invalid value is ignored (the one from reset_run is kept).
    pub fn load_threats(&mut self, t: &ThreatsData) {
        if let Some(v) = is_delay(t.next_inspector) {
            self.next_inspector = self.time + v;
        }
        if let Some(v) = is_delay(t.next_enemy) {
            self.next_enemy = self.time + v;
        }
        if let Some(v) = is_delay(t.bribe) {
            self.bribe_until = self.time + v;
        }
        if let Some(v) = is_delay(t.freeze) {
            self.freeze_until = self.time + v;
        }
        if let Some((elapsed, dur, hits)) = t
            .inspector
            .as_ref()
            .and_then(|w| load_walker(w, events::inspector_hits(self)))
        {
            self.inspector = Some(Inspector {
                start: self.time - elapsed,
                dur,
                hits,
            });
        }
        for e in t.enemies.iter().flatten() {
            let Ok(e) = serde_json::from_value::<EnemyData>(e.clone()) else {
                continue;
            };
            let Some(kind) = EnemyKind::from_id(&e.kind) else {
                continue;
            };
            if let Some((elapsed, dur, hits)) =
                load_walker(&e.walker, events::enemy_hits(self, kind))
                && self.enemies.len() < events::WAVE_MAX
            {
                self.enemies.push(Enemy {
                    kind,
                    start: self.time - elapsed,
                    dur,
                    hits,
                });
            }
        }
        if let Some(wave) = t.wave.filter(|&w| w <= 10_000) {
            self.wave = wave;
        }
    }

    /// Content of the save file.
    pub fn save_data(&self) -> SaveData {
        let names = |m: &BTreeMap<&'static str, u32>| {
            Some(m.iter().map(|(k, v)| (k.to_string(), *v)).collect())
        };
        let list = |ids: &mut dyn Iterator<Item = &&'static str>| {
            Some(ids.map(|s| s.to_string()).collect())
        };
        SaveData {
            version: Some(saves::SAVE_VERSION),
            isotopes: Some(self.isotopes),
            ptalents: names(&self.ptalents),
            prestige_count: Some(self.prestige_count),
            skin: Some(self.skin.to_string()),
            challenge: Some(self.challenge.map(str::to_string)),
            stats: Some(self.stats.clone()),
            exploded: Some(self.exploded),
            level: Some(self.level),
            progress: Some(self.progress),
            radiation: Some(self.radiation),
            pt: Some(self.pt),
            units: Some(self.units.iter().map(|(k, v)| (k.to_string(), v)).collect()),
            talents: names(&self.talents),
            mutations: list(&mut self.mutations.iter()),
            pending_mutation: self
                .pending_mutation
                .as_ref()
                .and_then(|m| list(&mut m.iter())),
            pending_chest: self
                .pending_chest
                .as_ref()
                .and_then(|c| list(&mut c.iter())),
            mut_index: Some(self.mut_index),
            mut_queue: Some(self.mut_queue),
            run_time: Some(self.run_time),
            crit_gauge: Some(self.crit_gauge),
            next_pt_level: Some(self.next_pt_level),
            audio: Some(self.audio.to_string()),
            volume: Some(self.volume as f64),
            stars: Some(self.stars),
            stalents: names(&self.stalents),
            supernova_count: Some(self.supernova_count),
            run_stats: Some(self.run_stats.clone()),
            last_run: self.last_run.clone(),
            threats: Some(self.threats_data()),
            achievements: list(&mut self.achievements.iter()),
            challenges_done: list(&mut self.challenges_done.iter()),
            synergies_found: list(&mut self.synergies_found.iter()),
            skins: list(&mut self.skins.iter()),
            wall_time: Some(wall_time()),
        }
    }

    /// The save as a JSON object.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self.save_data()).expect("save data serializes")
    }

    /// Atomic write of the save.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        saves::write(path, &self.to_json())
    }

    /// Game read from a save file; None if missing, unreadable or of an unsupported version.
    pub fn load(path: &Path) -> Option<Game> {
        let d = saves::read(path)?;
        if !saves::supported(&d) {
            return None;
        }
        let data: SaveData = serde_json::from_value(Value::Object(d)).ok()?;
        Some(Game::from_save(data, wall_time()))
    }

    /// Game rebuilt from the content of a save (`wall` = current wall-clock time, for offline progress).
    pub fn from_save(d: SaveData, wall: f64) -> Game {
        let mut g = Game::default();
        let known_ids = |m: Option<BTreeMap<String, u32>>,
                         find: fn(&str) -> Option<&'static str>| {
            m.map(|m| {
                m.iter()
                    .filter_map(|(k, &v)| Some((find(k)?, v)))
                    .collect::<BTreeMap<_, _>>()
            })
        };
        let ids = |v: Option<Vec<String>>, find: fn(&str) -> Option<&'static str>| {
            v.map(|v| v.iter().filter_map(|s| find(s)).collect::<Vec<_>>())
        };
        let find_mutation: fn(&str) -> Option<&'static str> =
            |s| events::find_mutation(s).map(|m| m.id);
        let find_challenge: fn(&str) -> Option<&'static str> =
            |s| data::find_challenge(s).map(|c| c.id);

        if let Some(v) = d.isotopes {
            g.isotopes = v;
        }
        if let Some(m) = known_ids(d.ptalents, |s| data::find_ptalent(s).map(|p| p.id)) {
            g.ptalents = m;
        }
        if let Some(v) = d.prestige_count {
            g.prestige_count = v;
        }
        g.skin = d
            .skin
            .as_deref()
            .and_then(data::find_skin)
            .unwrap_or("classic");
        if let Some(c) = d.challenge {
            g.challenge = c.as_deref().and_then(find_challenge);
        }
        if let Some(v) = d.stats {
            g.stats = v;
        }
        if let Some(v) = d.exploded {
            g.exploded = v;
        }
        if let Some(v) = d.level {
            g.level = v;
        }
        if let Some(v) = d.progress {
            g.progress = v;
        }
        if let Some(v) = d.radiation {
            g.radiation = v;
        }
        if let Some(v) = d.pt {
            g.pt = v;
        }
        for (uid, n) in d.units.iter().flatten() {
            if let Some(i) = data::unit_index(uid) {
                g.units.0[i] = *n;
            }
        }
        if let Some(m) = known_ids(d.talents, |s| data::find_talent(s).map(|t| t.id)) {
            g.talents = m;
        }
        if let Some(v) = d.mut_index {
            g.mut_index = v;
        }
        if let Some(v) = d.mut_queue {
            g.mut_queue = v;
        }
        if let Some(v) = d.run_time {
            g.run_time = v;
        }
        if let Some(v) = d.crit_gauge {
            g.crit_gauge = v;
        }
        if let Some(v) = d.next_pt_level {
            g.next_pt_level = v;
        }
        if let Some(m) = known_ids(d.stalents, |s| data::find_stalent(s).map(|p| p.id)) {
            g.stalents = m;
        }
        if let Some(v) = d.stars {
            g.stars = v;
        }
        if let Some(v) = d.supernova_count {
            g.supernova_count = v;
        }
        g.audio = d
            .audio
            .as_deref()
            .and_then(|a| AUDIO_MODES.into_iter().find(|&m| m == a))
            .unwrap_or("all");
        g.volume = match d.volume {
            Some(v) if (0.0..=100.0).contains(&v) => v as u32 / VOLUME_STEP * VOLUME_STEP,
            _ => 100,
        };
        g.achievements = ids(d.achievements, |s| {
            events::find_achievement(s).map(|a| a.id)
        })
        .map(BTreeSet::from_iter)
        .unwrap_or_default();
        g.challenges_done = ids(d.challenges_done, find_challenge)
            .map(BTreeSet::from_iter)
            .unwrap_or_default();
        g.synergies_found = ids(d.synergies_found, |s| events::find_synergy(s).map(|s| s.id))
            .map(BTreeSet::from_iter)
            .unwrap_or_default();
        g.last_run = d.last_run;
        g.run_stats = d.run_stats.unwrap_or_default();
        g.mutations = ids(d.mutations, find_mutation).unwrap_or_default();
        g.pending_mutation = ids(d.pending_mutation, find_mutation).filter(|m| !m.is_empty());
        if let Some(skins) = ids(d.skins, data::find_skin) {
            g.skins = skins.into_iter().collect();
        }
        let find_chest: fn(&str) -> Option<&'static str> =
            |s| events::CHEST_REWARDS.into_iter().find(|&k| k == s);
        g.pending_chest = ids(d.pending_chest, find_chest).filter(|c| !c.is_empty());
        if g.challenge == Some("watched") && g.inspector.is_none() {
            g.schedule_inspector(true); // reset_run ran before the challenge was loaded
        }
        if g.challenge == Some("invasion") {
            g.schedule_enemy(false);
        }
        if let Some(t) = &d.threats {
            g.load_threats(t); // after the default deadlines: the save wins
        }
        g.apply_offline(d.wall_time.map_or(0.0, |w| wall - w));
        g
    }

    pub fn apply_offline(&mut self, elapsed: f64) {
        let rank = self.pr("p_offline");
        if rank == 0 || elapsed < OFFLINE_MIN || !self.exploded {
            return;
        }
        let secs = elapsed.min(OFFLINE_MAX);
        let amount = self.production() * secs * OFFLINE_SHARE * rank as f64;
        if amount > 0.0 {
            self.gain(amount);
            let text = format!(
                "Pendant ton absence ({} min) : +{} ☢",
                (secs / 60.0) as u64,
                self.fmt(amount)
            );
            self.log(text, MsgKind::Good, Some("RETOUR"));
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::rng::FixedRandom;

    /// Game already exploded (level 1), sounds and messages cleared.
    pub(crate) fn exploded_game_with(rng: Box<dyn Rng>) -> Game {
        let mut g = Game::new(rng);
        g.click(true);
        g.sfx.clear();
        g.messages.clear();
        g
    }

    pub(crate) fn exploded_game() -> Game {
        exploded_game_with(Box::new(StdRandom::seeded(0)))
    }

    pub(crate) fn fixed(value: f64) -> Box<dyn Rng> {
        Box::new(FixedRandom::new(value))
    }

    pub(crate) fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-7 * b.abs().max(1.0)
    }

    pub(crate) fn has(g: &Game, sound: &str) -> bool {
        g.sfx.contains(&sound)
    }

    fn last_kind(g: &Game) -> MsgKind {
        g.messages.back().unwrap().kind
    }

    use crate::saves::TempDir;
    use serde_json::json;
    use std::fs;

    /// Writes a raw save then loads it.
    fn load_raw(path: &Path, d: &Value) -> Option<Game> {
        fs::write(path, serde_json::to_string(d).unwrap()).unwrap();
        Game::load(path)
    }

    fn without(mut d: Value, key: &str) -> Value {
        d.as_object_mut().unwrap().remove(key);
        d
    }

    // --- Format ---

    #[test]
    fn small_numbers() {
        assert_eq!(fmt(5.0), "5");
        assert_eq!(fmt(2.5), "2.5");
        assert_eq!(fmt(150.0), "150");
        assert_eq!(fmt(99.95), "100");
        assert_eq!(fmt(0.04), "0");
        assert_eq!(fmt(f64::INFINITY), "∞");
    }

    #[test]
    fn suffixes() {
        assert_eq!(fmt(1500.0), "1.50K");
        assert_eq!(fmt(2e6), "2.00M");
        assert_eq!(fmt(999_999.0), "1000.00K");
        assert_eq!(fmt(1e40), "1.00e+40");
    }

    #[test]
    fn need_grows() {
        assert_eq!(need(1), 25.0);
        assert!(need(10) < need(11));
    }

    // --- Click ---

    #[test]
    fn first_click_explodes() {
        let mut g = Game::new(Box::new(StdRandom::seeded(0)));
        g.click(true);
        assert!(g.exploded);
        assert_eq!(g.level, 1);
        assert!(has(&g, "boom"));
        assert_eq!(last_kind(&g), MsgKind::Epic);
    }

    #[test]
    fn manual_click_gains_and_sounds() {
        let mut g = exploded_game();
        g.click(true);
        assert_eq!(g.stats.clicks, 1);
        assert!(approx(g.radiation, 1.08));
        assert_eq!(g.sfx.iter().copied().collect::<Vec<_>>(), ["click0"]);
    }

    #[test]
    fn click_pitch_follows_combo() {
        let mut g = exploded_game();
        g.combo = (g.max_combo() - 1) as f64;
        g.click(true);
        assert_eq!(*g.sfx.back().unwrap(), "click7");
    }

    #[test]
    fn auto_click_is_silent() {
        let mut g = exploded_game();
        g.click(false);
        assert!(g.radiation > 0.0);
        assert_eq!(g.stats.clicks, 0);
        assert!(g.sfx.is_empty());
    }

    #[test]
    fn overheat() {
        let mut g = exploded_game();
        for _ in 0..=g.overheat_threshold() {
            g.click(true);
        }
        assert!(g.overheated());
        assert!(has(&g, "overheat"));
        let before = g.radiation;
        g.sfx.clear();
        g.click(true);
        assert_eq!(g.radiation, before);
        assert_eq!(g.sfx.iter().copied().collect::<Vec<_>>(), ["jam"]); // press refused but audible
    }

    #[test]
    fn crit_sound_only_when_manual() {
        let mut g = exploded_game_with(fixed(0.0));
        g.talents.insert("t_crit", 1);
        g.click(true);
        assert!(has(&g, "crit"));
        assert!(approx(g.radiation, 10.0 * 1.08)); // base 1 × combo × crit
        g.sfx.clear();
        g.click(false);
        assert!(!has(&g, "crit"));
    }

    #[test]
    fn onearm_forbids_space() {
        let mut g = exploded_game();
        g.challenge = Some("onearm");
        g.click(true);
        g.click(true);
        assert_eq!(g.stats.clicks, 0);
        assert!(has(&g, "nope"));
        assert_eq!(g.messages.len(), 1); // repeated message → deduplicated
    }

    // --- Levels ---

    #[test]
    fn level_up_gives_pt_and_sound() {
        let mut g = exploded_game();
        let pt = g.pt;
        g.gain(need(1));
        assert_eq!(g.level, 2);
        assert_eq!(g.pt, pt + 1);
        assert!(has(&g, "level"));
    }

    #[test]
    fn level_sound_only_when_pt_awarded() {
        let mut g = exploded_game();
        g.ptalents.insert("p_cap", 1);
        (g.level, g.next_pt_level, g.mut_index) = (200, 300, 10);
        g.gain(need(200));
        assert_eq!(g.level, 201);
        assert!(!has(&g, "level"));
        g.next_pt_level = 202;
        g.gain(need(201));
        assert!(has(&g, "level"));
    }

    #[test]
    fn level_capped() {
        let mut g = exploded_game();
        g.gain(1e30);
        assert_eq!(g.level, g.cap());
        assert_eq!(g.progress, 0.0);
    }

    // --- Shop ---

    #[test]
    fn unit_needs_unlock() {
        let mut g = exploded_game();
        g.radiation = 1000.0;
        assert!(!g.buy_unit("soldier"));
        g.talents.insert("u_soldier", 1);
        assert!(g.buy_unit("soldier"));
        assert_eq!(g.units["soldier"], 1);
        assert!(approx(g.radiation, 1000.0 - 15.0));
        assert!(approx(
            g.unit_cost("soldier", None),
            15.0 * data::UNIT_COST_GROWTH
        ));
    }

    #[test]
    fn buy_unit_max() {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        g.radiation = g.unit_cost_n("soldier", 2);
        assert_eq!(g.buy_unit_max("soldier"), 2);
        assert!(!g.can_buy_unit("soldier"));
    }

    #[test]
    fn cost_n_and_affordable() {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        assert!(approx(
            g.unit_cost_n("soldier", 2),
            15.0 + 15.0 * data::UNIT_COST_GROWTH
        ));
        g.radiation = g.unit_cost_n("soldier", 2) + 1.0;
        assert_eq!(g.affordable("soldier"), 2);
        g.talents.remove("u_soldier");
        assert_eq!(g.affordable("soldier"), 0);
    }

    #[test]
    fn buy_n_all_or_nothing() {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        g.radiation = g.unit_cost_n("soldier", 10) - 1.0;
        assert_eq!(g.buy_unit_n("soldier", 10), 0);
        assert_eq!(g.units["soldier"], 0);
        g.radiation += 1.0;
        assert_eq!(g.buy_unit_n("soldier", 10), 10);
        assert_eq!(g.units["soldier"], 10);
        assert!(g.radiation.abs() < 1e-6);
    }

    #[test]
    fn buy_n_exact_budget_rounding() {
        use crate::rng::RandomExt;
        let mut rng = StdRandom::seeded(0);
        for _ in 0..500 {
            let mut g = exploded_game();
            g.talents.insert("u_soldier", 1);
            g.units["soldier"] = rng.randint(0, 300) as u64;
            let k = *rng.choice(&[10, 100]);
            g.radiation = g.unit_cost_n("soldier", k);
            let before = g.units["soldier"];
            assert_eq!(g.buy_unit_n("soldier", k), k);
            assert_eq!(g.units["soldier"], before + k);
            assert!(g.radiation >= 0.0);
        }
    }

    #[test]
    fn bulk_crosses_milestones() {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        g.units["soldier"] = 45;
        g.radiation = 1e30;
        g.buy_unit_n("soldier", 60);
        assert_eq!(g.sfx.iter().filter(|s| **s == "milestone").count(), 2); // 50 and 100
    }

    #[test]
    fn affordable_bounded() {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        g.radiation = f64::INFINITY;
        assert_eq!(g.affordable("soldier"), MAX_BULK);
        assert_eq!(g.buy_unit_max("soldier"), MAX_BULK);
    }

    #[test]
    fn talent_cost_and_requirements() {
        let mut g = exploded_game();
        g.pt = 10;
        assert!(!g.buy_talent("t_crit")); // requires t_click
        assert!(g.buy_talent("t_click"));
        assert_eq!(g.pt, 9);
        assert_eq!(g.talent_cost("t_click"), 2);
        assert!(g.buy_talent("t_crit"));
    }

    #[test]
    fn production() {
        let mut g = exploded_game();
        g.units["soldier"] = 10;
        assert!(approx(g.production(), 10.0 * 0.5));
        g.freeze_until = g.time + 5.0;
        assert_eq!(g.production(), 0.0);
    }

    // --- Milestones ---

    fn milestone_game() -> Game {
        let mut g = exploded_game();
        g.talents.insert("u_soldier", 1);
        g
    }

    #[test]
    fn milestone_production_multiplied() {
        let mut g = milestone_game();
        let first = data::UNIT_MILESTONES[0];
        g.units["soldier"] = first - 1;
        let base = g.unit_prod("soldier");
        assert_eq!(g.next_milestone("soldier"), Some(first));
        g.units["soldier"] = first;
        assert_eq!(g.unit_milestones("soldier"), 1);
        assert!(approx(g.unit_prod("soldier"), base * data::MILESTONE_MULT));
    }

    #[test]
    fn milestone_crossing_announced() {
        let mut g = milestone_game();
        g.units["soldier"] = data::UNIT_MILESTONES[0] - 1;
        g.radiation = 1e12;
        g.buy_unit("soldier");
        assert!(has(&g, "milestone"));
        assert_eq!(last_kind(&g), MsgKind::Good);
        g.sfx.clear();
        g.buy_unit("soldier");
        assert!(!has(&g, "milestone"));
    }

    #[test]
    fn milestone_all_reached() {
        let mut g = milestone_game();
        g.units["soldier"] = *data::UNIT_MILESTONES.last().unwrap();
        assert_eq!(g.next_milestone("soldier"), None);
        assert_eq!(
            g.unit_milestones("soldier") as usize,
            data::UNIT_MILESTONES.len()
        );
    }

    // --- Prestige ---

    #[test]
    fn prestige_resets_run() {
        let mut g = exploded_game();
        g.level = g.cap();
        let gain = g.prestige_gain();
        assert!(g.prestige(None));
        assert_eq!(g.isotopes, gain);
        assert_eq!(g.prestige_count, 1);
        assert_eq!(g.level, 0);
        assert!(!g.exploded);
        assert!(has(&g, "prestige"));
    }

    #[test]
    fn prestige_requires_max_level() {
        let mut g = exploded_game();
        assert!(!g.prestige(None));
    }

    #[test]
    fn challenge_success() {
        let mut g = exploded_game();
        let ch = &data::CHALLENGES[0];
        g.challenge = Some(ch.id);
        g.level = ch.target - 1;
        g.ptalents.insert("p_cap", 4);
        g.mut_index = 20;
        g.level += 1;
        g.on_level_up();
        assert!(g.challenges_done.contains(ch.id));
        assert!(has(&g, "fanfare"));
        assert_eq!(g.pending_story, Some((ch.id, "end")));
    }

    #[test]
    fn challenge_start_requests_story() {
        let mut g = exploded_game();
        g.prestige_count = 1;
        g.prestige(Some("nanar"));
        assert_eq!(g.pending_story, Some(("nanar", "start")));
    }

    // --- Supernova ---

    fn supernova_ready() -> Game {
        let mut g = exploded_game();
        g.ptalents.insert("p_cap", data::ptalent("p_cap").max);
        g.level = g.cap();
        g
    }

    #[test]
    fn supernova_locked_before_max_core() {
        let mut g = exploded_game();
        g.level = g.cap();
        assert!(!g.supernova_unlocked());
        assert!(!g.can_supernova());
        assert!(!g.supernova());
    }

    #[test]
    fn supernova_gain_counts_isotopes_earned() {
        let mut g = supernova_ready();
        g.stats.iso_earned = 50_000;
        let expected =
            (((50_000 + g.prestige_gain()) as f64 / data::SUPERNOVA_DIVISOR).sqrt()) as i64;
        assert_eq!(g.supernova_gain(), expected);
    }

    #[test]
    fn supernova_resets_and_keeps() {
        let mut g = supernova_ready();
        (g.isotopes, g.prestige_count, g.stats.iso_earned) = (999, 7, 100_000);
        g.ptalents.insert("p_prod", 5);
        g.achievements.insert("boom");
        g.skins.insert("plasma");
        g.challenges_done.insert("race");
        let gain = g.supernova_gain();
        assert!(g.supernova());
        assert_eq!((g.stars, g.supernova_count, g.isotopes), (gain, 1, 0));
        assert!(g.ptalents.is_empty());
        assert_eq!(g.cap(), data::BASE_CAP);
        assert_eq!(g.prestige_count, 7);
        assert_eq!(g.stats.iso_earned, 0);
        assert!(g.achievements.contains("boom"));
        assert!(g.challenges_done.contains("race"));
        assert!(!g.exploded);
        assert!(has(&g, "supernova"));
        assert!(g.supernova_unlocked());
    }

    #[test]
    fn prestige_tracks_isotopes() {
        let mut g = exploded_game();
        g.level = g.cap();
        let gain = g.prestige_gain();
        g.prestige(None);
        assert_eq!(g.stats.iso_earned, gain);
    }

    #[test]
    fn star_talents() {
        let mut g = exploded_game();
        g.units["soldier"] = 10;
        g.level = g.cap();
        let (prod, iso) = (g.production(), g.prestige_gain());
        g.stars = 100;
        assert!(g.buy_stalent("s_prod"));
        assert!(g.buy_stalent("s_iso"));
        assert!(approx(g.production(), prod * 2.0));
        assert_eq!(g.prestige_gain(), iso * 2);
        assert_eq!(g.stars, 100 - 1 - 1);
    }

    #[test]
    fn dense_core_discounts_expansion() {
        let mut g = exploded_game();
        let full = g.ptalent_cost("p_cap");
        g.stars = 10;
        g.buy_stalent("s_core");
        assert_eq!(g.ptalent_cost("p_cap"), (full * 0.8).round());
        g.supernova_count = 1;
        assert_eq!(g.cap(), data::BASE_CAP); // leaves the starting max level alone
    }

    #[test]
    fn supernova_auto_talents() {
        let mut g = exploded_game();
        g.stalents.insert("s_auto", 1);
        g.pt = 3;
        g.tick(1.0);
        assert_eq!(g.tr("u_soldier"), 1); // unlock bought first
        // then everything affordable
        assert!(
            !data::TALENTS
                .iter()
                .any(|t| g.talent_available(t.id) && g.pt >= g.talent_cost(t.id))
        );
    }

    #[test]
    fn star_cost_and_max() {
        let mut g = exploded_game();
        g.stars = 1000;
        while g.buy_stalent("s_auto") {}
        assert_eq!(g.sr("s_auto"), 1);
        assert_eq!(g.stalent_cost("s_auto"), f64::INFINITY);
    }

    #[test]
    fn supernova_saved() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        (g.stars, g.supernova_count) = (4, 2);
        g.stalents = BTreeMap::from([("s_prod", 3)]);
        g.save(&path).unwrap();
        let h = Game::load(&path).unwrap();
        assert_eq!(
            (h.stars, h.supernova_count, h.stalents),
            (4, 2, BTreeMap::from([("s_prod", 3)]))
        );
    }

    #[test]
    fn rank_costs() {
        let mut g = exploded_game();
        // Costs round half to even: 5 × 1.7 = 8.5 → 8.
        let costs: Vec<f64> = (0..4)
            .map(|r| {
                g.ptalents.insert("p_iso", r);
                g.ptalent_cost("p_iso")
            })
            .collect();
        assert_eq!(costs, [5.0, 8.0, 14.0, 25.0]);
        g.ptalents = BTreeMap::from([("p_cap", 2), ("p_prod", 1)]);
        g.isotopes = 100;
        assert_eq!(g.isotopes_invested(), 100 + 20 + 150 + 2);
    }

    // --- Run report ---

    #[test]
    fn run_stats_tracked() {
        let mut g = exploded_game_with(fixed(0.0));
        g.talents.insert("t_crit", 1);
        g.click(true);
        g.click(false);
        let rs = &g.run_stats;
        assert_eq!((rs.clicks, rs.crits), (1, 1)); // the automatic press doesn't count
        assert!(approx(rs.best_click, 10.0 * 1.08));
        assert!(approx(rs.earned, g.radiation));
    }

    #[test]
    fn prestige_report() {
        let mut g = exploded_game();
        g.mutations = vec!["spores", "mitosis"];
        g.run_stats.chests = vec!["Frénésie".to_string()];
        g.units["soldier"] = 12;
        (g.level, g.run_time) = (g.cap(), 300.0);
        let gain = g.prestige_gain();
        g.prestige(None);
        let r = g.last_run.as_ref().unwrap();
        assert!(g.report_ready);
        assert_eq!(
            (r.kind, r.gain, r.level, r.time, r.units, r.number),
            (ReportKind::Prestige, gain, data::BASE_CAP, 300.0, 12, 1)
        );
        assert_eq!(r.synergies, ["Prolifération"]);
        assert_eq!(r.mutations, ["Spores radioactives", "Mitose"]);
        assert_eq!(r.chests, ["Frénésie"]);
        assert!(g.run_stats.chests.is_empty()); // reset for the next run
    }

    #[test]
    fn challenge_start_without_gain_has_no_report() {
        let mut g = exploded_game();
        g.prestige_count = 1;
        g.prestige(Some("race"));
        assert!(!g.report_ready);
        assert!(g.last_run.is_none());
    }

    #[test]
    fn supernova_report() {
        let mut g = exploded_game();
        g.ptalents.insert("p_cap", 4);
        g.level = g.cap();
        let gain = g.supernova_gain();
        g.supernova();
        let r = g.last_run.as_ref().unwrap();
        assert_eq!((r.kind, r.gain), (ReportKind::Supernova, gain));
    }

    #[test]
    fn corrupt_last_run_dropped() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        for bad in [
            json!({"kind": "prestige"}),
            json!({"kind": "x"}),
            json!("texte"),
            json!(null),
        ] {
            let mut d = exploded_game().to_json();
            d["last_run"] = bad.clone();
            assert!(load_raw(&path, &d).unwrap().last_run.is_none(), "{bad}");
        }
        let mut g = exploded_game();
        g.level = g.cap();
        g.prestige(None);
        let mut d = g.to_json();
        assert!(load_raw(&path, &d).unwrap().last_run.is_some());
        d["last_run"]["mutations"] = json!(null);
        assert!(load_raw(&path, &d).unwrap().last_run.is_none());
    }

    #[test]
    fn corrupt_run_stats_repaired() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut clicks = Vec::new();
        for bad in [
            json!([1, 2]),
            json!(null),
            json!({"best_click": "x", "chests": "abc", "clicks": 4}),
        ] {
            let mut d = exploded_game().to_json();
            d["run_stats"] = bad;
            let mut g = load_raw(&path, &d).unwrap();
            g.click(true);
            g.pending_chest = Some(vec!["frenzy"]);
            events::choose_chest(&mut g, 0); // no longer crashes
            assert_eq!(g.run_stats.chests, ["Frénésie"]);
            clicks.push(g.run_stats.clicks);
        }
        assert_eq!(clicks.last(), Some(&5)); // the valid value (4) is kept
    }

    #[test]
    fn unknown_mutations_filtered() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut d = exploded_game().to_json();
        d["mutations"] = json!(["spores", "zzz", 3]);
        d["pending_mutation"] = json!(["zzz"]);
        let mut g = load_raw(&path, &d).unwrap();
        assert_eq!(
            (g.mutations.clone(), g.pending_mutation.clone()),
            (vec!["spores"], None)
        );
        g.level = g.cap();
        assert!(g.prestige(None)); // the summary finds every mutation
    }

    #[test]
    fn report_and_synergies_saved() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        g.level = g.cap();
        g.prestige(None);
        let mut d = g.to_json();
        d["synergies_found"] = json!(["storm", "bogus"]);
        let h = load_raw(&path, &d).unwrap();
        assert_eq!(h.last_run.unwrap().kind, ReportKind::Prestige);
        assert!(!h.report_ready); // the screen doesn't come back on reload
        assert_eq!(h.synergies_found, BTreeSet::from(["storm"]));
    }

    // --- Achievements ---

    #[test]
    fn achievement_unlock_once() {
        let mut g = exploded_game();
        g.check_achievements();
        assert!(g.achievements.contains("boom"));
        assert!(has(&g, "achievement"));
        assert_eq!(last_kind(&g), MsgKind::Star);
        let n = g.messages.len();
        let last = g.messages.back().cloned();
        g.check_achievements();
        assert_eq!((g.messages.len(), g.messages.back().cloned()), (n, last));
    }

    // --- Watched timer ---

    #[test]
    fn abandon_restores_normal_timer() {
        let mut g = exploded_game();
        g.challenge = Some("watched");
        g.reset_run();
        g.abandon_challenge();
        assert!(g.next_inspector - g.time >= 150.0);
    }

    #[test]
    fn completion_restores_normal_timer() {
        let mut g = exploded_game();
        (g.challenge, g.level, g.mut_index) = (Some("watched"), 99, 20);
        g.level = 100;
        g.on_level_up();
        assert!(g.challenge.is_none());
        assert!(g.next_inspector - g.time >= 150.0);
    }

    // --- Blind messages ---

    #[test]
    fn game_fmt_masks() {
        let mut g = exploded_game();
        assert_eq!(g.fmt(1500.0), "1.50K");
        g.challenge = Some("blind");
        assert_eq!(g.fmt(1500.0), "???");
    }

    #[test]
    fn drop_message_masked() {
        let mut g = exploded_game();
        g.challenge = Some("blind");
        g.plane = Some(Plane {
            letter: 'a',
            start: 0.0,
            dur: 10.0,
        });
        g.rng = fixed(0.5);
        events::catch(&mut g, 'a');
        g.units["soldier"] = 100;
        assert!(events::chest_text(&g, "lump").1.contains("???"));
        g.pending_chest = Some(vec!["lump"]);
        events::choose_chest(&mut g, 0);
        assert!(g.messages.back().unwrap().text.contains("???"));
    }

    // --- Log ---

    #[test]
    fn log_kind_and_dedupe() {
        let mut g = Game::new(Box::new(StdRandom::seeded(0)));
        g.log("salut", MsgKind::Info, None);
        g.time = 3.0;
        g.log("salut", MsgKind::Bad, None);
        let expected = Message {
            text: "salut".to_string(),
            time: 3.0,
            kind: MsgKind::Bad,
            title: None,
        };
        assert_eq!(g.messages.iter().cloned().collect::<Vec<_>>(), [expected]);
        g.log("autre", MsgKind::Info, Some("TITRE"));
        assert_eq!(g.messages.len(), 2);
        assert_eq!(g.messages[1].title.as_deref(), Some("TITRE"));
    }

    #[test]
    fn log_bounded() {
        let mut g = exploded_game();
        for i in 0..20 {
            g.log(format!("message {i}"), MsgKind::Info, None);
        }
        assert_eq!(g.messages.len(), MESSAGES_MAX);
        assert_eq!(g.messages.back().unwrap().text, "message 19");
    }

    #[test]
    fn sfx_queue_bounded() {
        let mut g = exploded_game();
        for _ in 0..100 {
            g.click(false);
            g.sfx.push_back("tick");
        }
        assert!(g.sfx.len() <= SFX_MAX);
    }

    // --- Save ---

    #[test]
    fn save_roundtrip() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        (g.level, g.pt, g.audio) = (12, 7, "sfx");
        g.units["soldier"] = 3;
        g.achievements.insert("boom");
        g.talents.insert("t_click", 2);
        g.ptalents.insert("p_prod", 3);
        g.challenges_done.insert("race");
        g.skins.insert("mario");
        g.skin = "mario";
        g.challenge = Some("nanar");
        g.mutations = vec!["fog"];
        g.save(&path).unwrap();
        let h = Game::load(&path).unwrap();
        assert_eq!(
            (h.level, h.pt, h.audio, h.units["soldier"]),
            (12, 7, "sfx", 3)
        );
        assert_eq!(h.achievements, BTreeSet::from(["boom"]));
        assert_eq!((h.tr("t_click"), h.pr("p_prod")), (2, 3));
        assert_eq!(
            (h.skin, h.challenge, h.mutations),
            ("mario", Some("nanar"), vec!["fog"])
        );
        assert!(h.challenges_done.contains("race") && h.skins.contains("mario"));
        assert_eq!(h.stats, g.stats);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn save_without_some_fields() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let d = without(without(exploded_game().to_json(), "audio"), "skins");
        let g = load_raw(&path, &d).unwrap();
        assert_eq!(g.audio, "all");
        assert_eq!(g.skins, BTreeSet::from(["classic"]));
        assert!(g.exploded);
    }

    #[test]
    fn invalid_values_ignored_one_by_one() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        (g.level, g.pt) = (12, 7);
        let mut d = g.to_json();
        d["audio"] = json!("fort");
        d["skin"] = json!("bogus");
        d["radiation"] = json!("beaucoup");
        d["units"] = json!({"soldier": 4, "mortar": "x", "zzz": 3});
        d["stats"] = json!({"clicks": 9, "drops": -1});
        d["challenge"] = json!("zzz");
        let h = load_raw(&path, &d).unwrap();
        assert_eq!((h.audio, h.skin, h.challenge), ("all", "classic", None));
        assert_eq!((h.level, h.pt, h.radiation), (12, 7, 0.0));
        assert_eq!((h.units["soldier"], h.units["mortar"]), (4, 0));
        assert_eq!((h.stats.clicks, h.stats.drops), (9, 0));
    }

    #[test]
    fn missing_or_corrupt() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        assert!(Game::load(&path).is_none());
        for content in ["{pas du json", "[1, 2]", "42", "{}"] {
            fs::write(&path, content).unwrap();
            assert!(Game::load(&path).is_none(), "{content}");
        }
    }

    #[test]
    fn null_wall_time() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut d = exploded_game().to_json();
        d["wall_time"] = json!(null);
        assert!(load_raw(&path, &d).unwrap().exploded);
    }

    #[test]
    fn corrupt_pending_chest_sanitized() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let cases = [
            (json!(["bogus", "pt"]), Some(vec!["pt"])),
            (json!("xyz"), None),
            (json!([1, 2]), None),
            (json!(["bogus"]), None),
        ];
        for (bad, expected) in cases {
            let mut d = exploded_game().to_json();
            d["pending_chest"] = bad.clone();
            assert_eq!(
                load_raw(&path, &d).unwrap().pending_chest,
                expected,
                "{bad}"
            );
        }
    }

    #[test]
    fn watched_save_reloads_with_short_timer() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        g.challenge = Some("watched");
        let d = without(g.to_json(), "threats");
        let h = load_raw(&path, &d).unwrap();
        assert!(h.next_inspector - h.time <= 60.0);
    }

    #[test]
    fn volume_saved_and_validated() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        assert_eq!(g.volume, 100);
        assert!(!g.change_volume(1)); // already at max
        assert!(g.change_volume(-1));
        g.save(&path).unwrap();
        assert_eq!(Game::load(&path).unwrap().volume, 95);
        let cases = [
            (json!(-20), 100),
            (json!(250), 100),
            (json!("fort"), 100),
            (json!(true), 100),
            (json!(57), 55),
            (json!(60.0), 60),
        ];
        for (bad, expected) in cases {
            let mut d = g.to_json();
            d["volume"] = bad.clone();
            assert_eq!(load_raw(&path, &d).unwrap().volume, expected, "{bad}");
        }
        for _ in 0..30 {
            g.change_volume(-1);
        }
        assert_eq!(g.volume, 0);
    }

    #[test]
    fn threats_survive_reload() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        g.time += 500.0; // game time restarts at 0 on load: everything must be relative
        g.inspector = Some(Inspector {
            start: g.time - 5.0,
            dur: 20.0,
            hits: 2,
        });
        g.enemies = vec![
            Enemy {
                kind: EnemyKind::Commando,
                start: g.time - 3.0,
                dur: 20.0,
                hits: 4,
            },
            Enemy {
                kind: EnemyKind::Drone,
                start: g.time + 1.5,
                dur: 15.0,
                hits: 0,
            }, // not entered yet
        ];
        g.wave = 7;
        (g.next_inspector, g.next_enemy) = (g.time + 40.0, g.time + 70.0);
        (g.bribe_until, g.freeze_until) = (g.time + 100.0, g.time + 8.0);
        g.save(&path).unwrap();
        let h = Game::load(&path).unwrap();
        let inspector = h.inspector.unwrap();
        assert!(approx(h.time - inspector.start, 5.0));
        assert_eq!((inspector.dur, inspector.hits), (20.0, 2));
        let kinds: Vec<(EnemyKind, u32)> = h.enemies.iter().map(|e| (e.kind, e.hits)).collect();
        assert_eq!(kinds, [(EnemyKind::Commando, 4), (EnemyKind::Drone, 0)]);
        assert!(approx(h.time - h.enemies[0].start, 3.0));
        assert!(approx(h.enemies[1].start - h.time, 1.5));
        assert_eq!(h.wave, 7);
        let lefts = [
            (h.next_inspector, 40.0),
            (h.next_enemy, 70.0),
            (h.bribe_until, 100.0),
            (h.freeze_until, 8.0),
        ];
        for (deadline, left) in lefts {
            assert!(approx(deadline - h.time, left), "{left}");
        }
        assert!(h.frozen());
    }

    #[test]
    fn late_threat_hits_survive_reload() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut g = exploded_game();
        g.ptalents.insert("p_cap", 2);
        g.level = 10_000;
        g.inspector = Some(Inspector {
            start: g.time,
            dur: 20.0,
            hits: 7,
        }); // above the base INSPECTOR_HITS
        g.save(&path).unwrap();
        assert_eq!(Game::load(&path).unwrap().inspector.unwrap().hits, 7);
    }

    #[test]
    fn bad_threats_ignored() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let g = exploded_game();
        let cases = [
            json!(null),
            json!([1]),
            json!({"inspector": {"elapsed": 30, "dur": 20, "hits": 0}}),
            json!({"inspector": {"elapsed": 1, "dur": 20, "hits": 99}}),
            json!({"inspector": {"elapsed": 1, "dur": 5000, "hits": 0}}),
            json!({"enemies": [{"kind": "zzz", "elapsed": 1, "dur": 10, "hits": 0}, "x", null]}),
            json!({"enemies": {"kind": "drone"}}),
            json!({"next_inspector": -5, "freeze": null, "bribe": 1e9}),
            json!({"freeze": "long"}),
            json!({"enemies": [{"kind": "drone", "elapsed": 1, "dur": 10, "hits": true}]}),
        ];
        for bad in cases {
            let mut d = g.to_json();
            d["threats"] = bad.clone();
            let h = load_raw(&path, &d).unwrap_or_else(|| panic!("{bad}"));
            assert!(h.inspector.is_none(), "{bad}");
            assert!(h.enemies.is_empty(), "{bad}");
            assert!(!h.frozen(), "{bad}");
            assert!(h.next_inspector > h.time, "{bad}");
        }
    }

    #[test]
    fn too_many_enemies_ignored() {
        let tmp = TempDir::new();
        let path = tmp.path("s.json");
        let mut d = exploded_game().to_json();
        let drone = json!({"kind": "drone", "elapsed": 1, "dur": 10, "hits": 0});
        d["threats"] = json!({"enemies": [drone, drone, drone, drone], "wave": 20_000});
        let h = load_raw(&path, &d).unwrap();
        assert_eq!((h.enemies.len(), h.wave), (events::WAVE_MAX, 0));
    }

    #[test]
    fn offline_gain() {
        let mut g = exploded_game();
        g.ptalents.insert("p_offline", 1);
        g.units["soldier"] = 10;
        g.apply_offline(3600.0);
        assert!(approx(g.radiation, 5.0 * 3600.0 * 0.25));
        assert_eq!(last_kind(&g), MsgKind::Good);
    }

    #[test]
    fn offline_from_wall_time() {
        let mut g = exploded_game();
        g.ptalents.insert("p_offline", 1);
        g.units["soldier"] = 10;
        let mut d = g.save_data();
        d.wall_time = Some(1000.0);
        let h = Game::from_save(d, 1000.0 + 3600.0);
        assert!(approx(h.radiation, 5.0 * 3600.0 * 0.25));
    }
}
