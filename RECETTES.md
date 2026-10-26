# 🧪 Recettes pour les évolutions

Pas à pas pour ajouter du contenu au jeu. L'architecture, les règles et les pièges sont dans
[ARCHITECTURE.md](ARCHITECTURE.md).

Chaque recette se termine par : `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
et si l'équilibrage change, `cargo run --release --bin sim`. Puis revue automatique du commit et correction des
problèmes trouvés (voir `CLAUDE.md`). Les tables sont des tableaux de taille fixe : ajuster la taille déclarée
(`[Unit; 9]`…) à chaque ajout. Toute valeur d'effet ou de réglage est une constante nommée près de sa table,
et les textes la citent (`{}` dans un `data::Desc`, ou `format!` + `data::fr_num`) au lieu de recopier le
nombre (voir « Pas de nombre magique » dans [ARCHITECTURE.md](ARCHITECTURE.md#-règles-darchitecture)).

**Nouvelle unité** : ajouter un `unit_def(…)` dans `data::UNITS` (le talent de déblocage `u_<id>` et le
multiplicateur `m_<id>` sont générés par `unit_talents`) + un sprite dans `art::UNIT_ART` (2 images, lignes
alignées en bas, tourné vers la droite ; le miroir est automatique pour le côté droit) + l'identifiant dans la
liste `UNITS` des tests de `art.rs`. Le même sprite s'affiche dans l'onglet Unités à côté de la description, en
silhouette grise tant que l'unité n'est pas débloquée.

**Nouvel ennemi** : variante dans `events::EnemyKind` (et dans `EnemyKind::ALL`) + `EnemySpec` dans
`events::ENEMIES`, **dans le même ordre que l'enum** (`spec()` indexe par la variante) : `id`, `name`, `level`,
`hits`, `dur`, `sky`, `desc` (avec `{}` pour la force de l'effet si elle grandit avec la pression des menaces :
la fournir dans `effect_strength`) et `news` (titre du flash info pendant sa présence) + effet dans le `match`
d'`enemy_arrives` (ou un effet continu là où il s'applique) + sprite dans `art::ENEMY_ART` et identifiant dans
la liste `ENEMIES` des tests de `art.rs`. Les tests `every_enemy_has_spec_and_news` (`events.rs`) et
`every_unit_and_enemy_has_art` (`art.rs`) le vérifient.

**Nouveau défi** : un `challenge_def(…)` dans `data::CHALLENGES` (avec `skin`) + skin dans `data::SKINS` +
palette dans `art::PALETTES` (et dans la liste du test `known_skins`) + la contrainte
(`challenge == Some("<id>")`) et la récompense (`challenges_done.contains("<id>")`) dans le code, leurs valeurs en constantes
`<DÉFI>_…` dans `data.rs` citées par `desc` et `reward`. Si le défi
change une échéance de début de run, penser à la fin de `Game::from_save`. Le succès « Maître des défis » suit
automatiquement `CHALLENGES.len()`. Optionnel : une histoire de lancement et de réussite dans
`screens::STORIES` (clés `("<id>", "start")` / `("<id>", "end")`).

**Nouveau talent** : un `talent_def(…)` dans `data::build_talents` (talents de run, `TALENTS`), ou une entrée
dans `data::PTALENTS` / `STALENTS` + la constante de son effet à côté (`T_…`, `P_…`, `S_…`), citée par sa
description (`Desc`) + effet dans le multiplicateur concerné de `Game` (rang lu par `g.tr`, `g.pr` ou
`g.sr`). Si un talent ne doit jamais être acheté par l'auto-achat, adapter `Game::auto_talents`.

**Nouvelle mutation / synergie** : `events::MUTATIONS` / `events::SYNERGIES` (chaque mutation n'appartient
qu'à une paire) + la constante de son effet dans `events.rs`, citée par son texte + effet via
`g.has_mut("id")` / `g.has_syn("id")`.

**Nouveau succès** : un `achievement(id, nom, condition, |g| …)` dans `events::ACHIEVEMENTS` (chaque succès
donne `events::ACHIEVEMENT_BONUS` de production). Un nouveau compteur va dans `game::Stats`, avec
`#[serde(deserialize_with = "or_default")]` (absent d'une sauvegarde, il vaut 0).

**Nouveau son** : l'ajouter dans `sound::build()` (`add("nom", …)`, avec un `Timbre` ou un `Noise` et les
notes nommées), lui donner une priorité dans
`sound::PRIORITY` si c'est un jingle, une crête plus basse dans `sound::LEVEL` s'il est fréquent,
éventuellement un espacement dans `ui::SFX_GAP`. L'émettre par un littéral `g.sfx.push_back("nom")` (`self.sfx("nom")`
dans `ui.rs`) : le test `every_emitted_sound_exists` vérifie que tout nom émis ainsi (ou par `.play("nom")`)
existe, `sounds_are_pushed_literally` que toute émission est littérale, et
`level_keys_exist`, `priority_keys_exist`, `throttled_sounds_exist` vérifient les tables.

**Changement du format de sauvegarde** : incrémenter `saves::SAVE_VERSION`. Clé ajoutée : un champ `Option`
dans `game::SaveData` (avec `#[serde(deserialize_with = "lenient")]`), rempli dans `Game::save_data` et relu
dans `Game::from_save` ; rien d'autre si la valeur de la partie neuve convient quand il manque. Clé renommée
ou sens changé : conversion dans `Game::from_save`, ou, si elle est trop lourde, monter
`saves::MIN_SAVE_VERSION` (les parties plus anciennes seront supprimées) et retirer le code de
compatibilité devenu inutile.

**Nouvelle touche** : `UI::handle` + `ui::key_help()` + le tableau « Le mode d'emploi du détonateur » du
[README](README.md#-le-mode-demploi-du-détonateur). Touches déjà prises : toutes les lettres de
`events::DROP_LETTERS` (coffres), plus F, X, M, Q, J, K, chiffres, + = -, Espace, Tab, Maj+Tab, flèches,
Page préc./suiv., Entrée. Une touche spéciale nouvelle demande une variante de `term::Key` et sa traduction
dans `key_from_key_event`.

**Nouvel écran plein (overlay)** : dessiné en fin de `UI::draw` ; s'il doit capturer les touches, le traiter
en tête de `UI::handle` (voir l'écran de fin de partie). Écrans bloquants hors jeu (menu, intro) : fonctions
de `screens.rs` qui reçoivent un `&mut dyn Screen`, lisent les touches par `read_key` et renvoient
`Err(Interrupted)` (ou `None` pour le menu) sur `Key::Interrupt`.
