# Champignon atomique

`ARCHITECTURE.md` est ta référence technique (fichiers, règles, systèmes, sauvegarde, son, pièges) et
`RECETTES.md` le pas à pas des évolutions courantes : les lire avant de modifier le code, et les tenir à jour.
Le `README.md` est la vitrine pour les joueurs (présentation, lancement, commandes) : le mettre à jour quand
ce qu'il montre change (touches, commandes, capture `apercu.svg`). Tous décrivent l'état actuel, sans
historique : ce qui a changé et pourquoi va dans les messages de commit.

## Process après chaque changement

1. `python3 -m unittest` (et `python3 sim.py` si l'équilibrage change).
2. Commit du changement.
3. Revue automatique du commit puis correction : `/code-review HEAD~1..HEAD --fix`, sans attendre qu'on la
   demande.
4. Relancer les tests, puis commit séparé des corrections issues de la revue (ou rien si la revue ne trouve rien).
5. Ne jamais pousser sans accord explicite.
