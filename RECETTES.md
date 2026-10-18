# 🧪 Recettes pour les évolutions

Pas à pas pour ajouter du contenu au jeu. L'architecture, les règles et les pièges sont dans
[ARCHITECTURE.md](ARCHITECTURE.md).

Chaque recette se termine par : `python3 -m unittest`, et si l'équilibrage change, `python3 sim.py`. Puis
revue automatique du commit et correction des problèmes trouvés (voir `CLAUDE.md`).

**Nouvelle unité** : ajouter une entrée dans `data.UNITS` (le talent de déblocage et le multiplicateur sont
générés) + un sprite dans `art.UNIT_ART` (2 images, lignes alignées en bas, tourné vers la droite ; le miroir
est automatique pour le côté droit) ; le même sprite s'affiche dans l'onglet Unités à côté de la
description, en silhouette grise tant que l'unité n'est pas débloquée.

**Nouvel ennemi** : entrée dans `events.ENEMIES` (`level`, `hits`, `dur`, `sky`, `desc`) + effet dans
`_enemy_arrives` (ou un effet continu là où il s'applique) + sprite dans `art.ENEMY_ART` + phrase dans
`events.NEWS_ENEMY`. Le test `test_every_enemy_has_art_and_news` le vérifie.

**Nouveau défi** : entrée dans `data.CHALLENGES` (avec `skin`) + skin dans `data.SKINS` + palette dans
`art.PALETTES` + la contrainte et la récompense dans le code. Si le défi change une échéance de début de run,
penser à la fin de `Game.load`. Le succès « Maître des défis » suit automatiquement `len(CHALLENGES)`. Optionnel : une histoire de
lancement et de réussite dans `ui.STORIES`.

**Nouveau talent** : entrée dans `data.TALENTS` / `PTALENTS` / `STALENTS` + effet dans le multiplicateur
concerné de `Game`. Si un talent ne doit jamais être acheté par l'auto-achat, adapter `auto_talents`.

**Nouvelle mutation / synergie** : `events.MUTATIONS` / `events.SYNERGIES` (chaque mutation n'appartient qu'à
une paire) + effet via `g.has_mut("id")` / `g.has_syn("id")`.

**Nouveau succès** : `events.ACHIEVEMENTS` (chaque succès donne +2 % de production). Un nouveau compteur va
dans `Game.stats` (absent d'une sauvegarde, il vaut 0).

**Nouveau son** : l'ajouter dans `sound._build()`, lui donner une priorité dans `PRIORITY` si c'est un jingle,
une crête plus basse dans `sound.LEVEL` s'il est fréquent, éventuellement un espacement dans `ui.SFX_GAP`.
Le test `test_every_emitted_sound_exists` vérifie que tout `sfx("x")` émis existe.

**Changement du format de sauvegarde** : incrémenter `saves.SAVE_VERSION`. Clé ajoutée : rien d'autre si la
valeur par défaut de `Game` convient. Clé renommée ou sens changé : conversion dans `Game.load`, ou, si elle
est trop lourde, monter `MIN_SAVE_VERSION` (les parties plus anciennes seront supprimées) et retirer le code
de compatibilité devenu inutile.

**Nouvelle touche** : `UI.handle` + `ui.KEY_HELP` + le tableau « Le mode d'emploi du détonateur » du
[README](README.md#-le-mode-demploi-du-détonateur). Touches déjà prises : toutes les lettres de
`events.DROP_LETTERS` (coffres), plus F, X, M, Q, J, K, chiffres, + = -, Espace, Tab, Maj+Tab, flèches,
Page préc./suiv., Entrée.

**Nouvel écran plein (overlay)** : dessiné en fin de `UI.draw` ; s'il doit capturer les touches, le traiter
en tête de `UI.handle` (voir l'écran de fin de partie). Écrans bloquants hors jeu (menu, intro) : fonctions
à part dans `ui.py` qui pilotent `scr` elles-mêmes et remettent `scr.nodelay(True)` en sortant.
