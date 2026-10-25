# Champignon atomique — architecture

Référence technique pour faire évoluer le jeu : fichiers, règles, systèmes, sauvegarde, son, pièges. Les pas à
pas (nouvelle unité, ennemi, défi…) sont dans [RECETTES.md](RECETTES.md), la présentation du jeu et les
commandes dans le [README](README.md).

Ce fichier et RECETTES.md décrivent l'état actuel du code, sans historique (ce qui a changé et pourquoi va
dans les messages de commit) : les tenir à jour à chaque changement structurel (nouveau système, nouveau
fichier, nouvelle convention).

## 🏗️ Fichiers

Un crate Rust (`Cargo.toml`) : la bibliothèque `champignon` (`src/lib.rs` déclare les modules) et trois
binaires, le jeu (`src/main.rs`), `sim` et `apercu`.

| Fichier | Rôle | Dépend de |
|---|---|---|
| `src/data.rs` | Données statiques : unités (`UNITS`), talents de run (`TALENTS`, ceux des unités générés), talents de prestige (⚛, `PTALENTS`) et de supernova (✦, `STALENTS`), défis (`CHALLENGES`), skins (`SKINS`). | — |
| `src/rng.rs` | Trait `Rng` (`random`, `below`, `uniform`, `randint`) et son extension `RandomExt` (`choice`, `sample`) ; `StdRandom` (graine système ou `seeded`), `FixedRandom` pour les tests. | `rand` |
| `src/events.rs` | Logique pure des systèmes annexes : ravitaillements/coffres, inspecteur de l'ONU, autres ennemis, masse critique (QTE), mutations et synergies, succès, flash info. | `data`, `game`, `rng` |
| `src/saves.rs` | Fichiers de sauvegarde : dossier et emplacements des parties, version du format, lecture/écriture atomique, purge. Ne connaît que des objets JSON (`RawSave`). | `serde_json` |
| `src/game.rs` | `Game` : état complet, multiplicateurs, actions, prestige/supernova, `tick`, file de sons `Sfx`, messages ; format de sauvegarde typé (`SaveData`, serde) et conversion dans les deux sens (`save_data` / `from_save`). | `data`, `events`, `rng`, `saves`, `serde` |
| `src/term.rs` | Couche écran : couleurs (`ColorName`, `Color`), `Style`, touches (`Key`), trait `Screen`, terminal réel `TermScreen` (crossterm, double tampon) et grille en mémoire `BufferScreen` (tests, capture). Drapeau des signaux (`interrupt_flag`), restauration du terminal après une panique. | `game` (`MsgKind`), `crossterm` |
| `src/art.rs` | Génération ASCII : champignon procédural, palettes des skins, sprites des unités et ennemis, grosse police. Renvoie des cellules/lignes, n'écrit pas à l'écran. | `term` (`ColorName`) |
| `src/sound.rs` | Synthèse des effets et de la musique, mixage dans un thread, sortie PulseAudio (`libpulse-simple` chargée à l'exécution par `libloading`). Silencieux sans PulseAudio. Trait `SoundSink` (ce que l'UI utilise). | `libloading`, `rand` |
| `src/screens.rs` | Écrans plein écran hors partie, qui pilotent le `Screen` eux-mêmes : introduction et histoires des défis (texte tapé à la machine), menu des parties ; aides de texte (`wrap`, `center`, `head`, `slice`). | `game`, `saves`, `sound`, `term` |
| `src/ui.rs` | `UI<S: Screen>` : rendu des panneaux, onglets et overlays de la partie, bandeaux, particules, sons de l'interface, touches (`handle` → `Flow`). | tous les modules de la bibliothèque |
| `src/main.rs` | Binaire `champignon` : choix de la partie, intro des nouvelles parties, boucle principale (20 images/s), autosauvegarde toutes les 30 s, sauvegarde même après une panique, signaux, `--reset`, `--mute`. | `game`, `saves`, `screens`, `sound`, `term`, `ui`, `signal-hook` |
| `src/bin/sim.rs` | Bot qui joue sans affichage, pour vérifier l'équilibrage (`cargo run --release --bin sim -- [runs] [appuis/s]`, 8 runs et 6 appuis/s par défaut). | `game`, `events`, `data`, `rng` |
| `src/bin/apercu.rs` | Génère `apercu.svg`, la capture en couleurs du README : le vrai rendu de `UI` sur un `BufferScreen` de 100×30. À relancer (`cargo run --release --bin apercu`) quand l'écran change. | `game`, `ui`, `events`, `data`, `term`, `rng`, `sound` |
| `install.sh` | Compile (`cargo build --release`) et crée (ou `--desinstaller` retire) la commande `champignon`, lien vers `target/release/champignon` dans `~/.local/bin` (ou le dossier passé en argument). Refuse d'écraser un fichier qui n'est pas un lien, et de retirer un lien qui pointe ailleurs. | `cargo` |
| `Cargo.toml` | Crate `champignon` (édition 2024) ; dépendances : `crossterm`, `libloading`, `rand`, `serde`, `serde_json`, `signal-hook`. | |

Les tests sont dans un module `#[cfg(test)] mod tests` en fin de chaque fichier ; `cargo test` les lance tous.

### 📏 Règles d'architecture

- **La logique ne connaît pas l'écran.** `game.rs` et `events.rs` sont purs et testables. L'UI lit l'état et
  appelle des méthodes de `Game` ou des fonctions d'`events` ; elle ne calcule pas de règles du jeu.
- **Les sons passent par `g.sfx`** (`Sfx`, file de `&'static str` bornée à `SFX_MAX`). La logique fait
  `g.sfx.push_back("nom")`, l'UI les vide dans `UI::play_sfx` (dédoublonnage par image, espacement minimal
  `SFX_GAP`, secousse d'écran `SHAKE`).
- **Les messages passent par `g.log(msg, kind, title)`.** `kind` (`MsgKind::Info`, `Good`, `Bad`, `Loot`,
  `Star`, `Epic`) donne la couleur du bandeau, `title` (`Some("TITRE")` ou `None`) un titre court en grosses
  lettres.
- **Le temps** : `g.time` (temps de jeu total), `g.run_time` (partie en cours). Les échéances sont stockées en
  temps absolu (`next_plane`, `next_inspector`, `next_enemy`, `freeze_until`, `bribe_until`...). Inspecteur et
  ennemis sont programmés par `Game::schedule_inspector` / `schedule_enemy` (délais selon le défi en cours).
- **Hasard** : toujours `g.rng` (`Box<dyn Rng>`, injectable : `Game::new(rng)` ; les tests passent un
  `StdRandom::seeded(0)` ou `game::tests::fixed(valeur)`, un `FixedRandom`). L'UI a son propre `UI::rng`
  (effets visuels : flash de l'explosion, secousse, décalage des nombres flottants).
- **Défi Brouillard de guerre** : tout nombre affiché passe par `g.fmt()` (messages) ou `UI::num()` (écran)
  pour être masqué en `???`.

## 🔁 Boucle de jeu

`main::play` : lit les touches (`TermScreen::read_key` sans attente) → `UI::handle`, puis `Game::tick(dt)`,
puis `UI::draw(dt)`, puis `UI::show_story()`. `dt` est plafonné à `MAX_TICK` (1 s) : après une mise en veille,
le jeu ne rattrape pas tout d'un coup.

`Game::tick` : combo qui retombe → production → auto-détonateur → achat automatique (1/s) → buffs →
échec des défis chronométrés → `events::tick` (masse critique, avion, inspecteur, ennemis) → succès (1/s).

## ⚙️ Systèmes du jeu

- **Niveaux** : radiation gagnée → `progress` ; `need(level) = 25 * level^2.4`. Niveau max = `g.cap()`
  (`BASE_CAP` × 10^rang de « Expansion du cœur », jusqu'à `MAX_CAP` = 1 000 000).
- **Appui (ESPACE)** : `Game::click(manual)`. Combo, surchauffe (`overheat_threshold`,
  les appuis refusés jouent `jam`), critiques ×10 (`crit_mult`).
  Dans le code les noms restent `click*`, mais **les textes affichés disent « appui » / « détonation »,
  jamais « clic »** (on joue au clavier).
- **Unités** (`data::UNITS`) : coût ×1,15 par unité (`UNIT_COST_GROWTH`), paliers ×2 à
  50/100/200... (`UNIT_MILESTONES` ; les unités achetées ou offertes en jeu passent par `Game::add_units`, qui
  annonce les paliers franchis), débloquées par les talents `u_<id>` et boostées par `m_<id>` (générés par
  `unit_talents` dans `data.rs`). `g.units` s'indexe par identifiant (`g.units["soldier"]`).
- **Talents de run** (PT, branches DÉTONATION / INFANTERIE / LOGISTIQUE, `data::BRANCHES`), **prestige**
  (⚛, arbre `PTALENTS`), **supernova** (✦, arbre `STALENTS`, remet aussi à zéro le prestige). Rangs lus par
  `g.tr(id)`, `g.pr(id)`, `g.sr(id)`.
- **Mutations** aux niveaux 25, 50, 75, 100, 200, 400... (`events::mutation_milestone`) ; **synergies** quand
  une paire précise est réunie.
- **Masse critique** : jauge qui se remplit, F lance une QTE, F au bon moment (vert ×300, jaune ×60).
- **Ravitaillements** : un avion passe, taper sa lettre (`events::DROP_LETTERS`) ouvre un coffre (3 bonus au
  choix, `CHEST_REWARDS`).
- **Inspecteur de l'ONU** (dès le niveau 10) : gèle la production s'il arrive ; X pour le chasser.
- **Autres ennemis** (`events::ENEMIES`, un `EnemySpec` par variante d'`EnemyKind`, liste `g.enemies`) :
  saboteur, drone, manifestants (`Hippies`), commando. Un seul à la fois en temps normal ; pendant le défi
  Invasion, des vagues (`g.wave`, taille `wave_size`) dont les membres entrent à `WAVE_GAP` s d'écart (leur
  `start` peut être dans le futur). Le suivant n'est programmé que quand le terrain est dégagé. X les chasse
  aussi (`events::chase`) : l'inspecteur en priorité, puis l'ennemi le plus avancé (`enemy_progress`). Effet à
  l'arrivée dans `enemy_arrives`, sauf les manifestants (production divisée par `protest_divisor` dans
  `Game::production` tant qu'il y en a, sans cumul).
- **Escalade des menaces** (`events::threat_pressure`) : 1 jusqu'au niveau 100, +0,5 par ×10 au-delà (3 au
  niveau 1 000 000). Hors défis Sous surveillance et Invasion, les délais de l'inspecteur et des ennemis sont
  divisés par cette pression ; les appuis sur X nécessaires (`inspector_hits`, `enemy_hits`) sont multipliés
  par elle, au plus ×`MAX_HITS_MULT`. Les effets aussi, sans plafond : gel de l'inspecteur
  (`inspector_freeze`), vol du saboteur (`stolen_percent`), sabotage du commando (`sabotaged_percent`),
  diviseur des manifestants (`protest_divisor`) ; les alertes annoncent la force du moment (`enemy_desc`,
  `{}` dans le `desc` de l'`EnemySpec`, valeur fournie par `effect_strength`). Calculé sur le niveau actuel,
  donc rien à sauvegarder.
- **Défis** (`data::CHALLENGES`) : contraintes codées en dur là où elles s'appliquent (chercher
  `challenge == Some("<id>")`, dans `Game` comme dans `ui.rs`), récompense via
  `challenges_done.contains("<id>")`. `g.challenge` vaut `Option<&'static str>`.
- **Histoires de défis** (`screens::STORIES`, clés `(id, "start" | "end")`, lues par `screens::story`) : `Game`
  pose `pending_story` au lancement et à la réussite d'un défi ; `UI::show_story` (appelé par la boucle de
  `main::play`, pas par `draw`) l'affiche en plein écran (même écran tapé à la machine que l'introduction,
  `screens::intro`, touches ignorées pendant `STORY_GUARD` s) puis l'efface. Un défi sans histoire n'affiche
  rien ; une fois réussi, l'onglet Infos propose de la relire. Le lore : le professeur Amédée Morille
  (`screens::INTRO`), Gérard Zédé (Nanarland : un nanar, mauvais film sympathique, jamais un navet).
- **Skins** : palettes dans `art::PALETTES` (sauf `rainbow`, géré dans `art::color`, et `mario`, géré à part
  dans `art::cap` et `art::stem`) ; `art::has_skin` connaît les deux.

## 💾 Sauvegarde

- JSON dans `~/.champignon/save_<n>.json` (parties 1 à `MAX_SLOTS` = 99, la 1 est la « partie principale »).
  Variable d'environnement `CHAMPIGNON_SAVE_DIR` (`saves::DIR_ENV`) pour changer de dossier.
- `saves.rs` gère les fichiers (`Saves` : `slot_path`, `save_slots`, `free_slot`, `purge_obsolete`,
  `all_files` ; écriture atomique `.tmp` puis `fs::rename`, dossier créé au besoin) ; `game.rs` gère le
  contenu : `Game::save_data` remplit un `SaveData` (struct serde, un champ par clé JSON),
  `Game::save` / `Game::load` l'écrivent et le relisent, `Game::from_save` reconstruit la partie.
- **Version du format** : clé `version` (`saves::SAVE_VERSION`, actuellement 1), à incrémenter quand le format
  change. Une sauvegarde est lisible si sa version est entre `MIN_SAVE_VERSION` et `SAVE_VERSION`
  (`saves::supported`). Au lancement, `purge_obsolete` efface celles qui sont plus anciennes (message en
  jeu). Celles d'une version plus récente sont ignorées sans être touchées : réécrites ici, elles prendraient
  un numéro plus bas. Un fichier illisible n'est jamais supprimé ni réutilisé (s'il ne reste ni partie lisible
  ni emplacement libre, le jeu refuse de démarrer).
- **Compatibilité** : `Game::from_save` part d'une partie neuve et accepte les fichiers abîmés. Chaque champ
  de `SaveData` est un `Option` lu avec `#[serde(deserialize_with = "lenient")]` (ou `lenient_map` /
  `lenient_strings`, qui écartent les entrées invalides une à une) : absent ou mal typé → `None`, la valeur de
  la partie neuve est gardée. Les compteurs de `Stats` / `RunStats` utilisent `or_default` (une clé absente de
  `stats` vaut 0). Puis validation des valeurs et filtrage des identifiants inconnus (talents, mutations,
  synergies, coffres, skins, défis). Quand ce code de compatibilité devient trop lourd, monter
  `MIN_SAVE_VERSION` et le supprimer.
- Menaces en cours (clé `threats`, `ThreatsData`, `Game::threats_data` / `load_threats`) : inspecteur,
  ennemis, échéances `next_inspector` / `next_enemy`, pots-de-vin et gel. Toujours en **temps relatif**
  (écoulé / restant), car `g.time` repart de 0 au chargement. Chaque valeur invalide est ignorée
  individuellement (durées bornées par `THREAT_MAX`).
- Non sauvegardé (recréé par `reset_run`) : avion, QTE, buffs ; `pending_story` (histoire non vue si on quitte
  avant). Attention : `reset_run` tourne (dans `Game::new`) *avant* que le défi chargé soit appliqué ; les
  échéances qui dépendent du défi sont recalculées en fin de `from_save` (voir `watched` et `invasion`), puis
  `load_threats` les remplace si la sauvegarde en contient.
- Gains hors ligne : `wall_time` est sauvegardé, `Game::apply_offline` crédite l'absence en fin de `from_save`.

## 🔊 Son

- Réglages du joueur, sauvegardés dans la partie : `g.audio` (touche 0 : `AUDIO_MODES`, musique + effets /
  effets seuls / muet) et `g.volume` (touches + et -, de 0 à 100 % par paliers de `VOLUME_STEP`).
  `UI::play_sfx` les recopie dans le `SoundSink` à chaque image ; le volume s'applique au mixage final.
- Tous les sons sont synthétisés dans `sound::build()` (banque `Bank` : nom → échantillons) ; la musique
  (3 pistes : `calm`, `tension`, `epic`) dans `build_music()`, choisie par `UI::music_track()`. Style radio
  rétro à la Fallout : swing (`SWING`), accords de jazz (`PROGRESSIONS`), arrangement par piste (`STYLES` :
  temps de la contrebasse, du piano, de la grosse caisse, fréquence du thème), puis `radio` (bande étroite,
  souffle, craquements). Les pistes ont toutes la même longueur et bouclent sans clic (`add_at` replie la fin
  au début).
- Chaque effet est normalisé à sa crête visée (`sound::LEVEL`, `DEFAULT_LEVEL` = 0.8 sinon ; les variantes
  numérotées `click0`…`click7` partagent le niveau de leur famille). Le mixage final passe par un limiteur à
  gain (`limiter`) : si un bloc dépasse `CEIL`, le gain descend en rampe jusqu'au premier échantillon trop
  fort, puis remonte de `RELEASE` par bloc. Ne jamais saturer ou écrêter échantillon par échantillon : la
  musique craque dès qu'un effet fort s'y ajoute.
- La sortie ajoute un souffle inaudible (`keep_alive`, ±`KEEPALIVE_LSB`) : sous WSLg, un flux de silence pur
  est coupé puis relancé en retard, et les effets seuls (sans musique) arrivaient hachés.
- **Priorités** (`sound::PRIORITY`) : à partir de `JINGLE` (2), ce sont des jingles exclusifs (un seul à la
  fois, un plus important coupe le précédent en fondu, un moins important est ignoré). Les autres (1 par
  défaut) sont des bruitages, limités à `MAX_MINOR` voix et atténués (`DUCK`) pendant un jingle. Tout nouveau
  son long ou mélodique doit recevoir une priorité ≥ 2, sinon il se superposera.

## ⚠️ Contraintes et pièges

- Terminal minimum 60×20 (`ui::MIN_W`, `MIN_H`) : tout rendu doit rester dans l'écran à cette taille
  (`UI::put` coupe, mais `BufferScreen::put`, utilisé par les tests, panique sur tout débordement ; `sized`
  dans les tests de `ui.rs` change la taille de l'écran de test).
- **1 colonne par caractère** : `term::width` compte les `char`. Les textes affichés ne doivent contenir que
  des caractères d'une colonne (pas d'emoji ni de caractère double largeur ou combinant), sinon la mise en
  page se décale. Couper une chaîne par caractère (`screens::slice`, `head`, `ui::clip`), jamais par octet.
- **Noms de sons littéraux** : le test `every_emitted_sound_exists` (`sound.rs`) cherche dans tout `src/`
  (tests compris, sauf `sound.rs`) les `sfx.push_back("…")`, `.play("…")` et `self.sfx("…")` et vérifie que chaque nom existe
  dans `build()`. Dans `game.rs` et `events.rs`, toute émission doit donc être un littéral
  `g.sfx.push_back("nom")` (seule exception : `CLICK_SOUNDS[…]`), ce que vérifie `sounds_are_pushed_literally`.
  Même règle pour `self.sfx("nom")` dans `ui.rs` (pas de `self.sfx(if …)`). Les tests ne doivent pousser que
  des noms de sons qui existent.
- **Signaux** : SIGHUP, SIGTERM et SIGINT lèvent `term::interrupt_flag()` (enregistré par `main`) ; Ctrl+C
  arrive comme une touche (mode brut). Dans les deux cas `TermScreen::read_key` renvoie `Key::Interrupt` :
  `UI::handle` renvoie `Flow::Quit`, les écrans de `screens.rs` renvoient `Err(Interrupted)`, et la partie est
  sauvegardée. Toute attente doit donc passer par `read_key` (jamais `thread::sleep` en boucle sans lire les
  touches). Après SIGHUP, la sortie standard n'existe plus : `let _ = writeln!(…)`, pas `println!`.
- **Panique** : `term::install_panic_hook` restaure le terminal avant le message ; `main::run` sauvegarde la
  partie puis relance la panique.
- Les tables de données sont des tableaux de taille fixe (`[Unit; 9]`, `[Challenge; 12]`…) : ajouter une
  entrée oblige à changer la taille (le compilateur le signale). Elles sont sous `#[rustfmt::skip]`.
- Les tests de `art.rs` listent en dur les unités et ennemis (`art` ne dépend pas de `data`) : les mettre à
  jour avec les tables.
- Tests : construire l'UI par `bare_ui()` / `bare_ui_with(g, sound)` (`UI::new` sur un `BufferScreen` de
  30×100, hasard à graine fixe) et le son factice `FakeSound` (`ui::tests`) ; une partie déjà lancée par
  `game::tests::exploded_game()` ; les fichiers dans un `saves::TempDir`, jamais dans `~/.champignon`.
- `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` doivent passer.
- Code en anglais (identifiants, commentaires, doc-comments) ; textes affichés au joueur en français.

## 🔮 Pistes d'évolution

Aucune en attente : noter ici les prochaines idées retenues.
