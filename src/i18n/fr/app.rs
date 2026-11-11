//! French texts of the program outside the game screens.

use crate::i18n::app::AppTexts;

pub(super) const APP: AppTexts = AppTexts {
    main_slot: "partie principale",
    slot: |n| format!("partie {n}"),
    purged: |names, several| {
        if several {
            format!("Sauvegardes trop anciennes supprimées : {names}")
        } else {
            format!("Sauvegarde trop ancienne supprimée : {names}")
        }
    },
    reset_nothing: "Aucune partie à effacer.",
    reset_list: |files| format!("Parties à effacer :\n  {}", files.join("\n  ")),
    reset_confirm: "Effacer définitivement ? [o/N] ",
    yes: &["o", "oui", "y", "yes"],
    reset_cancelled: "Rien n'a été effacé.",
    reset_failed: |files| format!("Impossible d'effacer :\n  {}", files.join("\n  ")),
    reset_done: "Parties effacées.",
    no_slot: |slots| {
        format!(
            "Aucune partie utilisable et aucun emplacement libre ({slots} fichiers) : supprime un fichier de \
             sauvegarde ou lance champignon --reset."
        )
    },
    saved_in: |path| format!("Partie sauvegardée dans {path}"),
    error: |e| format!("Erreur : {e}"),
};
