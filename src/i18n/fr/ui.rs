//! French texts of the game screen.

use crate::i18n::fixed;
use crate::i18n::ui::{ReportLabels, StatLabels, UiTexts};

pub(super) const UI: UiTexts = UiTexts {
    too_small: |w, h, min_w, min_h| format!("Terminal trop petit ({w}×{h}, min {min_w}×{min_h})."),
    too_small_state: |level, radiation| {
        format!("Niveau {level}  ☢ {radiation}  — ESPACE marche quand même.")
    },
    term_title: |level| format!("☢ Niv. {level} — Champignon atomique"),
    tabs: ["Unités", "Talents", "Prestige", "Succès", "Infos"],
    audio_all: "musique + effets",
    audio_sfx: "effets seuls",
    audio_off: "muet",
    key_help: |modes, volume_step| {
        vec![
            (
                "ESPACE",
                "Détonateur : gagne de la radiation (attention à la surchauffe).".into(),
            ),
            (
                "F",
                "Masse critique : lance la jauge, puis F au bon moment.".into(),
            ),
            (
                "X",
                "Chasse l'inspecteur de l'ONU et les ennemis (à marteler).".into(),
            ),
            (
                "lettre",
                "Attrape le coffre de l'avion en tapant sa lettre.".into(),
            ),
            (
                "1-5",
                "Change d'onglet (ou choisit un bonus de coffre / une mutation).".into(),
            ),
            ("↑↓ / j k", "Sélectionne une ligne.".into()),
            ("Entrée", "Achète ou active la ligne sélectionnée.".into()),
            (
                "Onglet 3",
                "Défis à 3 rangs ; « Double défi : oui », puis Entrée sur deux défis.".into(),
            ),
            (
                "M",
                "Quantité achetée par Entrée : ×1, ×10, ×100 ou max.".into(),
            ),
            ("Tab ←→", "Onglet suivant / précédent.".into()),
            ("0", format!("Son : {}.", modes.join(" → "))),
            (
                "+ / -",
                format!("Volume général (par paliers de {volume_step} %)."),
            ),
            ("Q", "Sauvegarde et quitte.".into()),
        ]
    },
    qty_max: "max",
    mushroom_title: "Champignon atomique",
    press_space: "Appuie sur ESPACE !",
    inspector_alert: "⚠ Un inspecteur ! Martèle X ! ⚠",
    enemies: |n| format!("{n} ennemis"),
    enemy_alert: |who| format!("⚠ {who} ! Martèle X ! ⚠"),
    frozen_banner: |left| format!("❄ PRODUCTION GELÉE {left:.0} s ❄"),
    inspector_sign: "[ONU]",
    stats_title: "Statistiques",
    level: |level, cap| format!("NIVEAU {level} / {cap}"),
    challenge_tag: |names| format!("Défi {names}"),
    prestige_tag: |count| format!("Prestige ×{count}"),
    level_max: "MAX ! Prestige disponible → onglet 3",
    production: |prod| format!("+{prod}/s"),
    frozen: "GELÉ",
    click_stats: |click, crit| format!("Détonation {click}   Crit {crit}%"),
    auto: |per_second| format!("Auto {per_second}/s"),
    pt: |pt| format!("PT {pt}"),
    overheat: |left| format!("SURCHAUFFE ! {} s", fixed(left, 1)),
    combo: |mult| format!("Combo ×{}", fixed(mult, 1)),
    gauge_running: "MASSE CRITIQUE EN COURS !",
    gauge_ready: "Masse critique PRÊTE → appuie sur F",
    gauge: "Masse critique",
    buff: |name, left| format!("{name} {left:.0} s"),
    mutations: |names| format!("Mutations : {names}"),
    tab_help: |q, sound| {
        vec![
            format!(" Entrée acheter · M ×{q} · 5 aide · 0 son {sound} · Q "),
            format!(" M ×{q} · 5 aide · 0 son {sound} · Q "),
            format!(" M ×{q} · 5 aide · Q "),
            " 5 aide ".to_string(),
        ]
    },
    sound_on: "on",
    sound_off: "off",
    sound_unavailable: "indispo",
    confirm: "Appuie encore sur Entrée pour confirmer !",
    fold: |open, what| {
        let verb = if open { "masquer" } else { "afficher" };
        format!("Entrée : {verb} {what}.")
    },
    per_second: "/s",
    max_rank: "MAX",
    qty_all: " Entrée : tous les rangs possibles !",
    qty_up_to: |k| format!(" Entrée : jusqu'à {k} rangs."),
    moussaka: |size| format!("Moussaka géante : production ÷{size}"),
    pacifist: "Défi Pacifiste : unités interdites !",
    mono: "Défi Monoculture : une seule unité !",
    strike: "Grève : appuie sur ESPACE !",
    savior: "Le héros frappe, l'armée suit !",
    devil: "Cheval noir : attends un peu...",
    unit_desc: |name, prod, k| {
        format!("{name} : +{prod} ☢/s chacun. Entrée : acheter ×{k} (M pour changer).")
    },
    milestone: |count, mult| format!(" Palier à {count} : production ×{mult}."),
    wait: |time| format!(" Encore {time} d'attente."),
    missing: |amount| format!(" Il manque {amount} ☢."),
    teaser_challenge: "??? — réussis un défi",
    teaser_challenge_desc: |name| format!("Se débloque après le défi {name}."),
    teaser_talent: |talent| format!("??? — talent « {talent} »"),
    teaser_talent_desc: "Débloque de nouvelles unités dans l'onglet Talents (2), branche Infanterie.",
    talent_points: |pt| format!("Points de talent : {pt} PT"),
    pt_cost: |pt| format!("{pt} PT"),
    ascete: |desc| format!("Défi Ascète : talent interdit. {desc}"),
    requires: |talents| format!(" Requiert : {talents}."),
    locked: "VERROUILLÉ",
    locked_reach: |cap| format!("Atteins le niveau {cap} pour débloquer le prestige."),
    locked_lines: &[
        "Tu y gagneras des isotopes (⚛) à investir",
        "dans un arbre qui rend les runs suivantes",
        "plus efficaces.",
    ],
    reset_desc: "Réinitialise niveau, ☢, unités, talents et mutations. Garde ⚛, prestige, succès et skins.",
    prestige_button: |gain| format!("☢ PRESTIGE : +{gain} ⚛ ☢"),
    prestige_desc: |reset, surplus, bonus| {
        format!(
            "{reset} Surplus : ×{surplus} (+{bonus} par ×10 de la radiation qu'il fallait pour le niveau max : \
             rester au niveau max rapporte plus)."
        )
    },
    prestige_reach: |cap| format!("Prestige : atteins le niveau {cap}"),
    challenges_rule: "DÉFIS",
    monsters_rule: "MONSTRES",
    double_mode: |on| format!("Double défi : {}", if on { "oui" } else { "non" }),
    double_desc: "Deux défis à la fois, chacun à son rang suivant : atteins la plus haute des deux cibles et les \
                  deux gagnent un rang. Entrée sur un premier défi (◆), puis sur le second.",
    abandon: "✘ Abandonner le défi en cours",
    abandon_desc: "Retire la contrainte du défi sans réinitialiser la run.",
    done_fold: |done| format!("✔ {done} défi(s) terminé(s) au rang III"),
    done_fold_what: "les défis réussis à tous les rangs",
    in_progress: " (en cours)",
    challenge: |name, done, all| format!("Défi {name} {done}/{all}"),
    reward: |reward| format!("Récompense : {reward}."),
    all_ranks_done: |reward| format!("Tous les rangs réussis. {reward}"),
    solo_only: |reward| format!("Ne se joue pas en double défi. {reward}"),
    rank_locked: |rank, needed, core, reward| {
        format!("Rang {rank} : niveau max {needed} requis ({core}). {reward}")
    },
    note_prestige: " Lance une nouvelle run (et fait le prestige).",
    note_no_gain: " Lance une nouvelle run sans gain d'isotopes.",
    rank_desc: |rank, desc, reward, note| format!("Rang {rank} : {desc} {reward}{note}"),
    skin: |name, owned, all| format!("Skin : {name}  ({owned}/{all})"),
    skin_desc: |prestiges| {
        format!(
            "Entrée : changer de skin. Les défis et {prestiges} prestiges en débloquent d'autres."
        )
    },
    prestige_tree: |isotopes| format!("ARBRE DE PRESTIGE ({isotopes} ⚛)"),
    max_cap: |cap| format!(" Niveau max actuel : {cap}."),
    supernova_tree: |stars| format!("SUPERNOVA ({stars} ✦)"),
    supernova_desc: |divisor, core, keep| {
        format!(
            "Réinitialise aussi les isotopes et l'arbre de prestige (sauf les rangs d'{core} gardés par {keep}). \
             Garde ✦, succès, skins et défis. \
             Gain : racine des isotopes gagnés depuis la dernière supernova / {divisor}."
        )
    },
    supernova_button: |gain| format!("✦ SUPERNOVA : +{gain} ✦ ✦"),
    supernova_reach: |level| format!("Supernova : atteins le niveau {level}"),
    achievements: |done, all, bonus| format!("{done}/{all} succès · +{bonus} % de production"),
    synergies_rule: |found, all| format!("SYNERGIES ({found}/{all})"),
    synergy_active: " (active)",
    synergy_desc: |pair, effect| format!("{pair} : {effect}"),
    synergy_unknown_desc: "Synergie inconnue : choisis la bonne paire de mutations pendant une partie.",
    keys_rule: "TOUCHES",
    stats_rule: "STATISTIQUES",
    last_run_rule: "DERNIÈRE PARTIE",
    stats: StatLabels {
        total_time: "Temps de jeu total",
        run_time: "Partie en cours",
        best_prestige: "Meilleur prestige",
        prestiges: "Prestiges",
        supernovas: "Supernovas",
        max_level: "Niveau max atteint",
        clicks: "Appuis sur ESPACE",
        overheats: "Surchauffes",
        perfects: "Fusions parfaites",
        drops: "Colis attrapés",
        inspectors: "Inspecteurs chassés",
        enemies: "Ennemis repoussés",
        frozen: "Scellés subis",
        mutations: "Mutations choisies",
        challenge_ranks: "Rangs de défis réussis",
        skins: "Skins débloqués",
        sound: "Son",
        synergies: "Synergies découvertes",
    },
    label_value: |label, value| format!("{label} : {value}"),
    intro: "Revoir l'introduction",
    intro_desc: "Entrée : relire l'histoire du professeur Morille et de son champignon.",
    stories_fold: |count| format!("Histoires des défis ({count})"),
    stories_fold_what: "les histoires des défis réussis",
    story: |name| format!("Revoir le défi {name}"),
    story_desc: |name| format!("Entrée : relire l'histoire du défi {name}."),
    qte_title: "MASSE CRITIQUE",
    qte_prompt: "Appuie sur F au bon moment !",
    report_supernova: |n| format!("FIN DE PARTIE — SUPERNOVA n°{n}"),
    report_prestige: |n| format!("FIN DE PARTIE — PRESTIGE n°{n}"),
    report_prompt: "Appuie sur une touche pour recommencer",
    report: ReportLabels {
        duration: "Durée",
        level: "Niveau atteint",
        gain: "Gain",
        radiation: "Radiation produite",
        clicks: "Appuis sur ESPACE",
        clicks_value: |clicks, crits| format!("{clicks} (critiques : {crits})"),
        best_click: "Meilleure détonation",
        perfects: "Fusions parfaites",
        inspectors: "Inspecteurs chassés",
        units: "Unités",
        mutations: "Mutations",
        synergies: "Synergies",
        chests: "Coffres",
        none: "aucune",
        no_chest: "aucun",
    },
    chest_title: "COFFRE DE RAVITAILLEMENT",
    chest_choose: |n| format!("Choisis ton bonus avec 1-{n} :"),
    chest_useless: "Plus utile pour l'instant.",
    mutation_title: "MUTATION !",
    mutation_choose: |n| format!("Ton champignon mute. Choisis avec 1-{n} :"),
    synergy_unknown: "synergie inconnue !",
    news: "FLASH INFO",
    crit: |gain| format!("CRIT ! +{gain}"),
    fizzle: "pschitt…",
    level_up: |level| format!("NIVEAU {level} !"),
    chest_caught: "COFFRE !",
    sound: |mode, muted| {
        let hint = if muted { " (0 pour le remettre)" } else { "" };
        format!("Son : {mode}{hint}")
    },
    volume: |volume| format!("Volume : {volume} %"),
};
