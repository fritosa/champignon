//! Texts of the program outside the game screens (main.rs): saves purged, `--reset`, messages on exit.

pub struct AppTexts {
    /// Name of the main game, then of the other slots in the lists of games.
    pub main_slot: &'static str,
    pub slot: fn(u32) -> String,
    /// Games deleted at launch: their names, more than one or not.
    pub purged: fn(&str, bool) -> String,
    pub reset_nothing: &'static str,
    /// Files to erase.
    pub reset_list: fn(&[String]) -> String,
    pub reset_confirm: &'static str,
    /// Answers that confirm (compared in lower case).
    pub yes: &'static [&'static str],
    pub reset_cancelled: &'static str,
    /// Files that could not be erased.
    pub reset_failed: fn(&[String]) -> String,
    pub reset_done: &'static str,
    /// No usable game and no free slot: the number of slots.
    pub no_slot: fn(u32) -> String,
    /// Path of the saved game.
    pub saved_in: fn(&str) -> String,
    pub error: fn(&str) -> String,
}
