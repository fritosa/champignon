//! French texts of the full screens outside a game.

use super::stories::{INTRO, STORIES};
use crate::i18n::screens::{AgoUnit, ScreenTexts};

pub(super) const SCREENS: ScreenTexts = ScreenTexts {
    title: "☢  CHAMPIGNON ATOMIQUE  ☢",
    intro: INTRO,
    stories: STORIES,
    start: "commencer",
    resume: "continuer",
    press_key: |then| format!("Appuie sur une touche pour {then}"),
    show_all: "Une touche pour tout afficher",
    question: "Quelle partie lancer ?",
    main_slot: "Partie principale",
    slot: |n| format!("Partie {n}"),
    continue_game: |name| format!("Continuer : {name}"),
    game_detail: |level, isotopes, prestiges| {
        format!("niveau {level} · ⚛ {isotopes} · prestige ×{prestiges}")
    },
    ago: |n, unit| {
        let unit = match unit {
            AgoUnit::Days => "j",
            AgoUnit::Hours => "h",
            AgoUnit::Minutes => "min",
        };
        format!("il y a {n} {unit}")
    },
    just_now: "à l'instant",
    new_game: "+ Nouvelle partie",
    new_game_desc: "Recommence de zéro dans une sauvegarde séparée.",
    no_free_slot: "Plus d'emplacement libre : supprime un fichier de sauvegarde.",
    menu_help: "↑↓ choisir · Entrée valider · Q quitter",
};
