//! Texts of the full screens outside a game (screens.rs): introduction, challenge stories, start menu.

/// Unit of the time elapsed since a game was saved, in the start menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgoUnit {
    Days,
    Hours,
    Minutes,
}

/// Pages of a story: the first one is shown in bold.
pub type Pages = &'static [&'static str];

pub struct ScreenTexts {
    /// Name of the game, on top of the full screens.
    pub title: &'static str,
    // --- Typewriter screens
    /// Introduction of a new game.
    pub intro: Pages,
    /// Stories of the challenges: ((challenge id, "start" or "end"), pages).
    pub stories: &'static [((&'static str, &'static str), Pages)],
    /// End of the prompt of the introduction, then of a story (see `press_key`).
    pub start: &'static str,
    pub resume: &'static str,
    /// Prompt once the text is fully shown: what the key does (`start` or `resume`).
    pub press_key: fn(&str) -> String,
    pub show_all: &'static str,
    // --- Start menu
    pub question: &'static str,
    /// Name of the main game, then of the other slots.
    pub main_slot: &'static str,
    pub slot: fn(u32) -> String,
    /// Name of the game.
    pub continue_game: fn(&str) -> String,
    /// Level and isotopes (formatted), prestige count.
    pub game_detail: fn(&str, &str, i64) -> String,
    /// Count of units elapsed since the last save, then less than a minute.
    pub ago: fn(i64, AgoUnit) -> String,
    pub just_now: &'static str,
    pub new_game: &'static str,
    pub new_game_desc: &'static str,
    pub no_free_slot: &'static str,
    pub menu_help: &'static str,
}
