//! French messages of the game.

use crate::i18n::msg::{MsgTexts, Titles};

pub(super) const MSG: MsgTexts = MsgTexts {
    boom: "BOOM ! Le champignon est né.",
    onearm_forbidden: "Défi Manchot : ESPACE est interdit !",
    brawl: |lost| format!("Baston homérique ! -{lost} ☢"),
    overheat: "SURCHAUFFE ! Doucement sur le détonateur...",
    max_level: "NIVEAU MAX ! Le prestige est disponible (onglet 3).",
    milestone: |count, unit, mult| format!("Palier : {count} {unit}s, production ×{mult} !"),
    core_grows: |cap| format!("Le cœur s'étend : niveau max {cap} !"),
    achievement: |name, bonus| format!("★ Succès : {name} (+{bonus} % de production)"),
    offline: |minutes, amount| format!("Pendant ton absence ({minutes} min) : +{amount} ☢"),
    prestige: |gain| format!("PRESTIGE ! +{gain} ⚛. Tout recommence..."),
    supernova: |gain| format!("SUPERNOVA ! +{gain} ✦. L'univers recommence..."),
    challenge_rank: |name, rank| format!("{name} {rank}"),
    challenge_join: " + ",
    challenge_started: |names, double| {
        let kind = if double { "Double défi" } else { "Défi" };
        format!("{kind} {names} lancé !")
    },
    challenge_done: |name, rank, reward| format!("Défi {name} {rank} réussi ! {reward}"),
    challenge_failed: |name| format!("Défi {name} échoué : trop lent !"),
    challenge_abandoned: "Défi abandonné.",
    not_together: |a, b| format!("{a} et {b} ne se jouent pas ensemble."),
    crocodile: |eaten, unit| format!("Croco ! -{eaten} {unit}"),
    qte_perfect: |reward| format!("FUSION PARFAITE ! +{reward} ☢"),
    qte_partial: |reward| format!("Fusion partielle : +{reward} ☢"),
    qte_fizzle: "Pétard mouillé...",
    mutation: |name| format!("Mutation : {name} !"),
    synergy: |name, effect, discovered| {
        let found = if discovered { " découverte" } else { "" };
        format!("Synergie{found} : {name} ! {effect}")
    },
    chest: |what| format!("Coffre : {what} !"),
    chest_radiation: |amount| format!(" (+{amount} ☢)"),
    chest_pt: |pts| format!(" (+{pts} PT)"),
    chest_frenzy: |mult, secs| format!("Production ×{mult} pendant {secs:.0} s."),
    chest_click_frenzy: |mult, secs| format!("Détonation ×{mult} pendant {secs:.0} s."),
    chest_lump: |amount| format!("+{amount} ☢ tout de suite."),
    chest_pt_desc: |pts| format!("+{pts} PT."),
    chest_gauge: "Masse critique chargée à 100 % (appuie sur F).",
    chest_army: |percent| format!("+{percent:.0} % de chaque type d'unité possédé (au moins 1)."),
    chest_shield: |minutes| format!("Aucun inspecteur pendant {minutes:.0} min (l'actuel repart)."),
    buff_prod: |mult| format!("Frénésie ×{mult}"),
    buff_click: |mult| format!("Détonation ×{mult}"),
    inspector_chased: |loot| {
        format!("L'inspecteur s'enfuit en courant ! (+{loot} ☢ de matériel abandonné)")
    },
    inspector_guards: "Tes gardes escortent l'inspecteur vers la sortie.",
    inspector_freeze: |secs| {
        format!("L'inspecteur a mis les scellés : production gelée {secs} s !")
    },
    enemy_alert: |name, effect| format!("{name} en approche : {effect} ! Martèle X."),
    wave: |n, names| format!("Vague {n} : {names} ! Martèle X."),
    wave_join: ", ",
    enemy_chased: |name, loot| format!("{name} repoussé ! (+{loot} ☢ de butin)"),
    saboteur: |stolen| format!("Le saboteur s'enfuit avec {stolen} ☢ !"),
    drone: |calls_inspector| {
        let outcome = if calls_inspector {
            " : un inspecteur arrive bientôt !"
        } else {
            " et vidé ta masse critique !"
        };
        format!("Le drone a photographié le site{outcome}")
    },
    commando: |percent| format!("Le commando a saboté {percent} % de ton armée !"),
    hippies_leave: "Les manifestants rentrent chez eux.",
    titles: Titles {
        boom: "BOOM !",
        no: "NON !",
        brawl: "BASTON",
        overheat: "CHAUD !",
        max: "MAX !",
        milestone: "PALIER",
        core: "CŒUR +",
        achievement: "SUCCÈS",
        back: "RETOUR",
        prestige: "PRESTIGE",
        supernova: "SUPERNOVA",
        challenge: "DÉFI",
        challenge_done: "DÉFI OK",
        failed: "ÉCHEC",
        abandon: "ABANDON",
        crocodile: "CROCO !",
        mutation: "MUTATION",
        synergy: "SYNERGIE",
        chest: "COFFRE",
        chased: "OUSTE !",
        guards: "GARDES",
        frozen: "GELÉ !",
        alert: "ALERTE",
        wave: |n| format!("VAGUE {n}"),
        repelled: "REPOUSSÉ",
        stolen: "VOLÉ !",
        spotted: "REPÉRÉ",
        sabotage: "SABOTAGE",
        sit_in_over: "FIN DU SIT-IN",
    },
};
