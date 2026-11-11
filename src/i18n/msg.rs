//! Messages of the game (`Game::log` in game.rs and events.rs), their big titles and the texts of the
//! chests and buffs. Numbers arrive already formatted (`g.fmt`, `i18n::num`), except counts and durations.

pub struct MsgTexts {
    // --- Detonator and levels
    pub boom: &'static str,
    pub onearm_forbidden: &'static str,
    /// Radiation lost.
    pub brawl: fn(&str) -> String,
    pub overheat: &'static str,
    pub max_level: &'static str,
    /// Count reached, unit name, new production multiplier.
    pub milestone: fn(u64, &str, &str) -> String,
    /// New max level.
    pub core_grows: fn(&str) -> String,
    /// Achievement name, production bonus in %.
    pub achievement: fn(&str, &str) -> String,
    /// Minutes away, radiation earned.
    pub offline: fn(u64, &str) -> String,
    // --- Prestige and challenges
    /// Isotopes earned.
    pub prestige: fn(i64) -> String,
    /// Stars earned.
    pub supernova: fn(i64) -> String,
    /// Challenge name and rank number (I, II, III).
    pub challenge_rank: fn(&str, &str) -> String,
    /// Between the two names of a double challenge.
    pub challenge_join: &'static str,
    /// Names with their ranks, double challenge or not.
    pub challenge_started: fn(&str, bool) -> String,
    /// Challenge name, rank number, reward of the rank.
    pub challenge_done: fn(&str, &str, &str) -> String,
    /// Names of the current challenge(s).
    pub challenge_failed: fn(&str) -> String,
    pub challenge_abandoned: &'static str,
    /// The two challenge names.
    pub not_together: fn(&str, &str) -> String,
    /// Units eaten, unit name.
    pub crocodile: fn(u64, &str) -> String,
    // --- Critical mass
    /// Result of the QTE in the green zone: radiation earned.
    pub qte_perfect: fn(&str) -> String,
    /// Result of the QTE in the yellow zone: radiation earned.
    pub qte_partial: fn(&str) -> String,
    /// QTE missed.
    pub qte_fizzle: &'static str,
    // --- Mutations
    pub mutation: fn(&str) -> String,
    /// Synergy name, its effect, first discovery or not.
    pub synergy: fn(&str, &str, bool) -> String,
    // --- Chests and buffs
    /// Chest bonus name, then what it gave (if it shows a number).
    pub chest: fn(&str) -> String,
    /// Radiation of the uranium lump, appended to the bonus name.
    pub chest_radiation: fn(&str) -> String,
    /// PT of the tactical manual, appended to the bonus name.
    pub chest_pt: fn(&str) -> String,
    /// Descriptions of the chest bonuses: multiplier and seconds (frenzies), radiation (lump), PT,
    /// percentage (army), minutes (bribes).
    pub chest_frenzy: fn(&str, f64) -> String,
    pub chest_click_frenzy: fn(&str, f64) -> String,
    pub chest_lump: fn(&str) -> String,
    pub chest_pt_desc: fn(&str) -> String,
    pub chest_gauge: &'static str,
    pub chest_army: fn(f64) -> String,
    pub chest_shield: fn(f64) -> String,
    /// Active buffs: multiplier.
    pub buff_prod: fn(&str) -> String,
    pub buff_click: fn(&str) -> String,
    // --- Inspector and enemies
    /// Radiation of the abandoned equipment.
    pub inspector_chased: fn(&str) -> String,
    pub inspector_guards: &'static str,
    /// Seconds of freeze.
    pub inspector_freeze: fn(u32) -> String,
    /// Enemy name, announced effect.
    pub enemy_alert: fn(&str, &str) -> String,
    /// Wave number, enemy names.
    pub wave: fn(u32, &str) -> String,
    /// Between the enemy names of a wave.
    pub wave_join: &'static str,
    /// Enemy name, loot.
    pub enemy_chased: fn(&str, &str) -> String,
    /// Radiation stolen.
    pub saboteur: fn(&str) -> String,
    /// The drone calls an inspector, or not.
    pub drone: fn(bool) -> String,
    /// Percentage of the army destroyed.
    pub commando: fn(u64) -> String,
    pub hippies_leave: &'static str,
    pub titles: Titles,
}

/// Big titles of the messages (art::BIG_FONT letters only: test `titles_use_the_big_font`).
pub struct Titles {
    pub boom: &'static str,
    pub no: &'static str,
    pub brawl: &'static str,
    pub overheat: &'static str,
    pub max: &'static str,
    pub milestone: &'static str,
    pub core: &'static str,
    pub achievement: &'static str,
    pub back: &'static str,
    pub prestige: &'static str,
    pub supernova: &'static str,
    pub challenge: &'static str,
    pub challenge_done: &'static str,
    pub failed: &'static str,
    pub abandon: &'static str,
    pub crocodile: &'static str,
    pub mutation: &'static str,
    pub synergy: &'static str,
    pub chest: &'static str,
    pub chased: &'static str,
    pub guards: &'static str,
    pub frozen: &'static str,
    pub alert: &'static str,
    /// Wave number.
    pub wave: fn(u32) -> String,
    pub repelled: &'static str,
    pub stolen: &'static str,
    pub spotted: &'static str,
    pub sabotage: &'static str,
    pub sit_in_over: &'static str,
}

impl Titles {
    /// Every fixed title (the test of the big font also checks `wave`). Destructured: a new title
    /// that is not listed here does not compile.
    pub fn all(&self) -> [&'static str; 28] {
        let Titles {
            boom,
            no,
            brawl,
            overheat,
            max,
            milestone,
            core,
            achievement,
            back,
            prestige,
            supernova,
            challenge,
            challenge_done,
            failed,
            abandon,
            crocodile,
            mutation,
            synergy,
            chest,
            chased,
            guards,
            frozen,
            alert,
            repelled,
            stolen,
            spotted,
            sabotage,
            sit_in_over,
            wave: _,
        } = *self;
        [
            boom,
            no,
            brawl,
            overheat,
            max,
            milestone,
            core,
            achievement,
            back,
            prestige,
            supernova,
            challenge,
            challenge_done,
            failed,
            abandon,
            crocodile,
            mutation,
            synergy,
            chest,
            chased,
            guards,
            frozen,
            alert,
            repelled,
            stolen,
            spotted,
            sabotage,
            sit_in_over,
        ]
    }
}
