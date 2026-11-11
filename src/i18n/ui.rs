//! Texts of the game screen (ui.rs): panels, tabs, overlays, help bars, floating texts. Numbers arrive
//! already formatted (`game::fmt`, `i18n::num`, `UI::num`), except counts and the seconds of the timers;
//! padding and columns stay in ui.rs.

/// Tabs of the game screen (keys 1 to 5).
pub const TAB_COUNT: usize = 5;

/// Key help: (keys, what they do).
pub type KeyHelp = Vec<(&'static str, String)>;

pub struct UiTexts {
    // --- Whole screen
    /// Terminal too small: its width and height, then the minimum ones.
    pub too_small: fn(i32, i32, i32, i32) -> String,
    /// Under `too_small`: level and radiation.
    pub too_small_state: fn(u32, &str) -> String,
    /// Terminal title: the level.
    pub term_title: fn(&str) -> String,
    pub tabs: [&'static str; TAB_COUNT],
    /// Names of the audio modes (game::AUDIO_MODES).
    pub audio_all: &'static str,
    pub audio_sfx: &'static str,
    pub audio_off: &'static str,
    /// Key help of the Infos tab: (keys, what they do), given the names of the audio modes in their
    /// order and the volume step in %.
    pub key_help: fn(&[&str], u32) -> KeyHelp,
    /// "Max" quantity bought by Enter (key M).
    pub qty_max: &'static str,
    // --- Mushroom panel
    pub mushroom_title: &'static str,
    pub press_space: &'static str,
    pub inspector_alert: &'static str,
    /// Several enemies at once: their count.
    pub enemies: fn(usize) -> String,
    /// Enemy name, or `enemies`.
    pub enemy_alert: fn(&str) -> String,
    /// Seconds of freeze left.
    pub frozen_banner: fn(f64) -> String,
    /// Sign carried by the inspector (its width sets the room he takes).
    pub inspector_sign: &'static str,
    // --- Statistics panel
    pub stats_title: &'static str,
    /// Level and max level.
    pub level: fn(&str, &str) -> String,
    /// Name(s) of the current challenge(s) (the time left follows, if it is timed).
    pub challenge_tag: fn(&str) -> String,
    /// Prestige count.
    pub prestige_tag: fn(u64) -> String,
    pub level_max: &'static str,
    /// Production per second.
    pub production: fn(&str) -> String,
    pub frozen: &'static str,
    /// Detonation power and critical chance in %.
    pub click_stats: fn(&str, i64) -> String,
    /// Auto-detonator presses per second.
    pub auto: fn(u32) -> String,
    /// Talent points owned.
    pub pt: fn(i64) -> String,
    /// Seconds of overheat left.
    pub overheat: fn(f64) -> String,
    /// Combo multiplier.
    pub combo: fn(f64) -> String,
    pub gauge_running: &'static str,
    pub gauge_ready: &'static str,
    pub gauge: &'static str,
    /// Active buff: its name and seconds left.
    pub buff: fn(&str, f64) -> String,
    /// Names of the mutations of the run.
    pub mutations: fn(&str) -> String,
    // --- Tab bars
    /// Bottom edge, from the most detailed to the shortest: quantity bought, sound state.
    pub tab_help: fn(&str, &str) -> Vec<String>,
    pub sound_on: &'static str,
    pub sound_off: &'static str,
    pub sound_unavailable: &'static str,
    pub confirm: &'static str,
    /// Description of a row that folds a list: unfolded or not, what it shows.
    pub fold: fn(bool, &str) -> String,
    /// After the cost of each unit (production per second).
    pub per_second: &'static str,
    /// Rank or level reached, instead of a cost.
    pub max_rank: &'static str,
    /// Enter buys several ranks (key M): all, or up to that many.
    pub qty_all: &'static str,
    pub qty_up_to: fn(u64) -> String,
    // --- Units tab
    /// Moussaka challenge: production divisor.
    pub moussaka: fn(f64) -> String,
    pub pacifist: &'static str,
    pub mono: &'static str,
    pub strike: &'static str,
    pub savior: &'static str,
    pub devil: &'static str,
    /// Unit name, production per unit, quantity bought by Enter.
    pub unit_desc: fn(&str, &str, u64) -> String,
    /// Count of the next milestone, production multiplier.
    pub milestone: fn(u64, &str) -> String,
    /// Time to wait (formatted duration) or radiation missing.
    pub wait: fn(&str) -> String,
    pub missing: fn(&str) -> String,
    /// Unit unlocked by a challenge: row, then description with the challenge name.
    pub teaser_challenge: &'static str,
    pub teaser_challenge_desc: fn(&str) -> String,
    /// Unit unlocked by a talent: row with the talent name, then description.
    pub teaser_talent: fn(&str) -> String,
    pub teaser_talent_desc: &'static str,
    // --- Talents tab
    /// Talent points owned.
    pub talent_points: fn(i64) -> String,
    /// Cost in talent points.
    pub pt_cost: fn(i64) -> String,
    /// Description of the talent.
    pub ascete: fn(&str) -> String,
    /// Talents required, with their rank.
    pub requires: fn(&str) -> String,
    // --- Prestige tab
    pub locked: &'static str,
    /// Max level to reach.
    pub locked_reach: fn(&str) -> String,
    /// Lines under `locked_reach`.
    pub locked_lines: &'static [&'static str],
    pub reset_desc: &'static str,
    /// Isotopes earned.
    pub prestige_button: fn(i64) -> String,
    /// `reset_desc`, surplus multiplier, bonus per ×10.
    pub prestige_desc: fn(&str, &str, &str) -> String,
    /// Max level to reach.
    pub prestige_reach: fn(&str) -> String,
    pub challenges_rule: &'static str,
    pub monsters_rule: &'static str,
    pub double_mode: fn(bool) -> String,
    pub double_desc: &'static str,
    pub abandon: &'static str,
    pub abandon_desc: &'static str,
    /// Count of the challenges done at every rank.
    pub done_fold: fn(usize) -> String,
    pub done_fold_what: &'static str,
    pub in_progress: &'static str,
    /// Challenge name, ranks done, ranks.
    pub challenge: fn(&str, u32, u32) -> String,
    /// Reward of the next rank.
    pub reward: fn(&str) -> String,
    /// Then `reward`.
    pub all_ranks_done: fn(&str) -> String,
    pub solo_only: fn(&str) -> String,
    /// Rank, max level required, name of the talent that raises it, `reward`.
    pub rank_locked: fn(&str, &str, &str, &str) -> String,
    /// After the description of a challenge that can be started, with or without a prestige.
    pub note_prestige: &'static str,
    pub note_no_gain: &'static str,
    /// Rank, description of the challenge at that rank, `reward`, then a note.
    pub rank_desc: fn(&str, &str, &str, &str) -> String,
    /// Skin name, skins owned, skins.
    pub skin: fn(&str, usize, usize) -> String,
    /// Prestiges for the plasma skin.
    pub skin_desc: fn(u64) -> String,
    /// Isotopes owned.
    pub prestige_tree: fn(&str) -> String,
    /// Current max level, after the description of the core expansion.
    pub max_cap: fn(&str) -> String,
    /// Stars owned.
    pub supernova_tree: fn(&str) -> String,
    /// Divisor of the supernova gain, names of the talents "p_cap" and "s_keep".
    pub supernova_desc: fn(f64, &str, &str) -> String,
    /// Stars earned.
    pub supernova_button: fn(i64) -> String,
    /// Level to reach.
    pub supernova_reach: fn(&str) -> String,
    // --- Achievements tab
    /// Achievements done, achievements, production bonus in %.
    pub achievements: fn(usize, usize, &str) -> String,
    /// Synergies found, synergies.
    pub synergies_rule: fn(usize, usize) -> String,
    pub synergy_active: &'static str,
    /// Mutation pair and effect.
    pub synergy_desc: fn(&str, &str) -> String,
    pub synergy_unknown_desc: &'static str,
    // --- Infos tab
    pub keys_rule: &'static str,
    pub stats_rule: &'static str,
    pub last_run_rule: &'static str,
    pub stats: StatLabels,
    /// A label and its value, in a sentence.
    pub label_value: fn(&str, &str) -> String,
    pub intro: &'static str,
    pub intro_desc: &'static str,
    /// Count of the stories to read again.
    pub stories_fold: fn(usize) -> String,
    pub stories_fold_what: &'static str,
    /// Challenge name.
    pub story: fn(&str) -> String,
    pub story_desc: fn(&str) -> String,
    // --- Overlays
    pub qte_title: &'static str,
    pub qte_prompt: &'static str,
    /// End of game report: supernova or prestige number.
    pub report_supernova: fn(u64) -> String,
    pub report_prestige: fn(u64) -> String,
    pub report_prompt: &'static str,
    pub report: ReportLabels,
    pub chest_title: &'static str,
    /// Number of choices.
    pub chest_choose: fn(usize) -> String,
    pub chest_useless: &'static str,
    pub mutation_title: &'static str,
    /// Number of choices.
    pub mutation_choose: fn(usize) -> String,
    pub synergy_unknown: &'static str,
    pub news: &'static str,
    // --- Floating texts and messages of the keys
    /// Radiation of a critical detonation.
    pub crit: fn(&str) -> String,
    pub fizzle: &'static str,
    /// New level.
    pub level_up: fn(&str) -> String,
    pub chest_caught: &'static str,
    /// Audio mode name, muted or not.
    pub sound: fn(&str, bool) -> String,
    /// Volume in %.
    pub volume: fn(u32) -> String,
}

/// Labels of the statistics of the Infos tab.
pub struct StatLabels {
    pub total_time: &'static str,
    pub run_time: &'static str,
    pub best_prestige: &'static str,
    pub prestiges: &'static str,
    pub supernovas: &'static str,
    pub max_level: &'static str,
    pub clicks: &'static str,
    pub overheats: &'static str,
    pub perfects: &'static str,
    pub drops: &'static str,
    pub inspectors: &'static str,
    pub enemies: &'static str,
    pub frozen: &'static str,
    pub mutations: &'static str,
    pub challenge_ranks: &'static str,
    pub skins: &'static str,
    pub sound: &'static str,
    pub synergies: &'static str,
}

/// Labels of the end of game report (also in the Infos tab).
pub struct ReportLabels {
    pub duration: &'static str,
    pub level: &'static str,
    pub gain: &'static str,
    pub radiation: &'static str,
    pub clicks: &'static str,
    /// Presses and critical ones.
    pub clicks_value: fn(&str, &str) -> String,
    pub best_click: &'static str,
    pub perfects: &'static str,
    pub inspectors: &'static str,
    pub units: &'static str,
    pub mutations: &'static str,
    pub synergies: &'static str,
    pub chests: &'static str,
    /// No mutation or synergy, no chest.
    pub none: &'static str,
    pub no_chest: &'static str,
}
