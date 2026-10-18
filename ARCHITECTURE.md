# Champignon atomique — architecture

Référence technique pour faire évoluer le jeu : fichiers, règles, systèmes, sauvegarde, son, pièges. Les pas à
pas (nouvelle unité, ennemi, défi…) sont dans [RECETTES.md](RECETTES.md), la présentation du jeu et les
commandes dans le [README](README.md).

Ce fichier et RECETTES.md décrivent l'état actuel du code, sans historique (ce qui a changé et pourquoi va
dans les messages de commit) : les tenir à jour à chaque changement structurel (nouveau système, nouveau
fichier, nouvelle convention).

## 🏗️ Fichiers

| Fichier | Rôle | Dépend de |
|---|---|---|
| `data.py` | Données statiques : unités, talents de run, talents de prestige (⚛) et de supernova (✦), défis, skins. | — |
| `events.py` | Logique pure des systèmes annexes : ravitaillements/coffres, inspecteur de l'ONU, autres ennemis, masse critique (QTE), mutations et synergies, succès, flash info. | `data` |
| `saves.py` | Fichiers de sauvegarde : emplacements des parties, version du format, lecture/écriture atomique, purge. Ne connaît que des dicts JSON. | — |
| `game.py` | Classe `Game` : état complet, multiplicateurs, actions, prestige/supernova, `tick`, conversion de l'état en dict JSON et retour (`to_dict` / `load`). | `data`, `events`, `saves` |
| `art.py` | Génération ASCII : champignon procédural, palettes des skins, sprites des unités et ennemis, grosse police. Renvoie des cellules/lignes, ne touche pas à curses. | — |
| `sound.py` | Synthèse des effets et de la musique (stdlib), mixage dans un thread, sortie PulseAudio via ctypes. Silencieux sans PulseAudio. | — |
| `ui.py` | Tout ce qui est curses : rendu des panneaux et overlays, touches, menu des parties, introduction. | tous |
| `main.py` | Boucle principale (20 images/s), choix de la partie, intro des nouvelles parties, autosauvegarde toutes les 30 s, `--reset`. | `game`, `saves`, `ui`, `sound` |
| `sim.py` | Bot qui joue sans affichage, pour vérifier l'équilibrage. | `game`, `events`, `data` |
| `apercu.py` | Génère `apercu.svg`, la capture en couleurs du README : le vrai rendu de `UI` sur un écran factice. À relancer quand l'écran change. | `game`, `ui`, `events` |
| `tests/` | `unittest`. `tests/__init__.py` redirige `CHAMPIGNON_SAVE` vers un dossier temporaire. | |

### 📏 Règles d'architecture

- **La logique ne connaît pas curses.** `game.py` et `events.py` sont purs et testables. L'UI lit l'état et
  appelle des méthodes de `Game` ou des fonctions d'`events` ; elle ne calcule pas de règles du jeu.
- **Les sons passent par `g.sfx`** (deque de noms). La logique fait `g.sfx.append("nom")`, l'UI les vide dans
  `UI.play_sfx` (dédoublonnage par image, espacement minimal `SFX_GAP`, secousse d'écran `SHAKE`).
- **Les messages passent par `g.log(msg, kind, title)`.** `kind` donne la couleur du bandeau
  (`info`, `good`, `bad`, `loot`, `star`, `epic`), `title` un titre court en grosses lettres.
- **Le temps** : `g.time` (temps de jeu total), `g.run_time` (partie en cours). Les échéances sont stockées en
  temps absolu (`next_plane`, `next_inspector`, `next_enemy`, `freeze_until`, `bribe_until`...).
- **Hasard** : toujours `g.rng` (injectable, les tests passent un `random.Random(0)` ou `FixedRandom`).
- **Défi Brouillard de guerre** : tout nombre affiché passe par `g.fmt()` (messages) ou `UI.num()` (écran)
  pour être masqué en `???`.

## 🔁 Boucle de jeu

`main.run` : lit les touches → `UI.handle`, puis `Game.tick(dt)`, puis `UI.draw(dt)`, puis `UI.show_story()`.

`Game.tick` : combo qui retombe → production → auto-détonateur → achat automatique (1/s) → buffs →
échec des défis chronométrés → `events.tick` (masse critique, avion, inspecteur, ennemis) → succès (1/s).

## ⚙️ Systèmes du jeu

- **Niveaux** : radiation gagnée → `progress` ; `need(level) = 25 * level**2.4`. Niveau max = `g.cap`
  (100 × 10^rang de « Expansion du cœur », jusqu'à 1 000 000).
- **Appui (ESPACE)** : `Game.click(manual)`. Combo, surchauffe (`overheat_threshold`,
  les appuis refusés jouent `nope`), critiques ×10.
  Dans le code les noms restent `click*`, mais **les textes affichés disent « appui » / « détonation »,
  jamais « clic »** (on joue au clavier).
- **Unités** (`data.UNITS`) : coût ×1,15 par unité, paliers ×2 à 50/100/200..., débloquées par les talents `u_<id>`
  et boostées par `m_<id>` (générés automatiquement dans `data.py`).
- **Talents de run** (PT, branches DÉTONATION / INFANTERIE / LOGISTIQUE), **prestige** (⚛, arbre `PTALENTS`),
  **supernova** (✦, arbre `STALENTS`, remet aussi à zéro le prestige).
- **Mutations** aux niveaux 25, 50, 75, 100, 200, 400... ; **synergies** quand une paire précise est réunie.
- **Masse critique** : jauge qui se remplit, F lance une QTE, F au bon moment (vert ×300, jaune ×60).
- **Ravitaillements** : un avion passe, taper sa lettre ouvre un coffre (3 bonus au choix).
- **Inspecteur de l'ONU** (dès le niveau 10) : gèle la production s'il arrive ; X pour le chasser.
- **Autres ennemis** (`events.ENEMIES`, liste `g.enemies`) : saboteur, drone, manifestants, commando. Un seul
  à la fois en temps normal ; pendant le défi Invasion, des vagues (`g.wave`, taille `wave_size`) dont les
  membres entrent à `WAVE_GAP` s d'écart (leur `start` peut être dans le futur). Le suivant n'est programmé
  que quand le terrain est dégagé. X les chasse aussi : l'inspecteur en priorité, puis l'ennemi le plus
  avancé (`enemy_progress`). Effet à l'arrivée dans `_enemy_arrives`, sauf les manifestants (production ÷2
  dans `Game.production` tant qu'il y en a, sans cumul).
- **Défis** (`data.CHALLENGES`) : contraintes codées en dur là où elles s'appliquent (chercher
  `self.challenge == "<id>"` / `g.challenge == "<id>"`), récompense via `"<id>" in challenges_done`.
- **Histoires de défis** (`ui.STORIES`, clés `(id, "start" | "end")`) : `Game` pose `pending_story` au lancement
  et à la réussite d'un défi ; `UI.show_story` (appelé par la boucle de `main.run`, pas par `draw`) l'affiche
  en plein écran (même écran tapé à la machine que l'introduction, touches ignorées pendant `STORY_GUARD` s)
  puis l'efface. Un défi sans histoire n'affiche rien ; une fois réussi, l'onglet Infos propose de la relire.
  Le lore : le professeur Amédée Morille (`ui.INTRO`), Gérard Zédé (Nanarland : un nanar, mauvais film
  sympathique, jamais un navet).
- **Skins** : palettes dans `art.PALETTES` (sauf `rainbow`, géré dans `art._color`, et `mario`, géré à part dans `art._cap` et `art._stem`).

## 💾 Sauvegarde

- JSON dans `~/.champignon_save.json` (partie 1), `~/.champignon_save_<n>.json` (parties 2 à 99).
  Variable d'environnement `CHAMPIGNON_SAVE` pour changer le chemin (utilisée par les tests).
- `saves.py` gère les fichiers (écriture atomique `.tmp` puis `os.replace`) ; `Game` gère le contenu : attributs
  listés dans `Game.SAVED`, plus les ensembles (`achievements`, `skins`...) dans `to_dict`.
- **Version du format** : clé `version` (`saves.SAVE_VERSION`), à incrémenter quand le format change. Une
  sauvegarde est lisible si sa version est entre `MIN_SAVE_VERSION` et `SAVE_VERSION`. Au lancement,
  `purge_obsolete` efface celles qui sont plus anciennes (message en jeu). Celles d'une version plus récente
  sont ignorées sans être touchées : réécrites ici, elles prendraient un numéro plus bas. Un fichier illisible
  n'est jamais supprimé ni réutilisé.
- **Compatibilité** : `Game.load` accepte toutes les versions de `MIN_SAVE_VERSION` à `SAVE_VERSION` et les
  fichiers abîmés : valeurs par défaut pour les clés absentes (les dicts sont fusionnés avec `update`, donc
  une clé absente de `stats` vaut 0), validation des types, filtrage des identifiants inconnus (mutations,
  synergies, coffres). Quand ce code de compatibilité devient trop lourd, monter `MIN_SAVE_VERSION` et le
  supprimer.
- Menaces en cours (clé `threats`, `threats_dict` / `load_threats`) : inspecteur, ennemis, échéances
  `next_inspector` / `next_enemy`, pots-de-vin et gel. Toujours en **temps relatif** (écoulé / restant), car
  `g.time` repart de 0 au chargement. Chaque valeur invalide est ignorée individuellement.
- Non sauvegardé (recréé par `reset_run`) : avion, QTE, buffs ; `pending_story` (histoire non vue si on quitte avant). Attention : `reset_run` tourne *avant* que le
  défi chargé soit appliqué ; les échéances qui dépendent du défi sont recalculées en fin de `load` (voir
  `watched` et `invasion`), puis `load_threats` les remplace si la sauvegarde en contient.

## 🔊 Son

- Réglages du joueur, sauvegardés dans la partie : `g.audio` (touche 0 : musique + effets / effets seuls /
  muet) et `g.volume` (touches + et -, de 0 à 100 % par paliers de `VOLUME_STEP`). `UI.play_sfx` les recopie
  dans `Sound` à chaque image ; le volume s'applique au mixage final.
- Tous les sons sont synthétisés dans `sound._build()` ; la musique (3 pistes : `calm`, `tension`, `epic`)
  dans `_build_music()`, choisie par `UI.music_track()`.
- Chaque effet est normalisé à sa crête visée (`sound.LEVEL`, 0.8 par défaut). Le mixage final passe par un
  limiteur à gain (`_limiter`) : si un bloc dépasse `CEIL`, tout le bloc est baissé, puis le gain remonte de
  `RELEASE` par bloc. Ne jamais saturer ou écrêter échantillon par échantillon : la musique craque dès
  qu'un effet fort s'y ajoute.
- **Priorités** (`sound.PRIORITY`) : à partir de 2, ce sont des jingles exclusifs (un seul à la fois, un plus
  important coupe le précédent en fondu, un moins important est ignoré). Les autres (1 par défaut) sont des
  bruitages, limités à `MAX_MINOR` voix et atténués (`DUCK`) pendant un jingle. Tout nouveau son long ou
  mélodique doit recevoir une priorité ≥ 2, sinon il se superposera.

## ⚠️ Contraintes et pièges

- Terminal minimum 60×20 (`ui.MIN_W`, `MIN_H`) : tout rendu doit rester dans l'écran à cette taille
  (`UI.put` coupe, mais les tests utilisent un `PutScreen` qui refuse les débordements).
- Pas de dépendance externe : stdlib seulement (le son passe par ctypes).
- Les tests UI construisent `UI` sans `__init__` (`bare_ui` dans `tests/test_ui.py`) : un nouvel attribut
  utilisé au rendu doit y être ajouté, ou initialisé paresseusement.
- Noms de classes de test uniques par fichier (un doublon masque silencieusement l'autre).
- Textes, commentaires et messages en français.

## 🔮 Pistes d'évolution

Aucune en attente : noter ici les prochaines idées retenues.
