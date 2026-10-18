#!/bin/sh
# Installe la commande « champignon » : un lien vers main.py dans ~/.local/bin (ou le dossier donné).
# Usage : ./install.sh [dossier]   |   ./install.sh --desinstaller [dossier]
set -e

usage() {
    echo "Usage : $0 [dossier]   |   $0 --desinstaller [dossier]" >&2
    exit 2
}

projet=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cible="$projet/main.py"

desinstaller=
if [ "$1" = "--desinstaller" ]; then
    desinstaller=1
    shift
fi
case "$1" in -*) usage ;; esac
[ $# -le 1 ] || usage
dossier="${1:-$HOME/.local/bin}"

if [ -n "$desinstaller" ]; then
    lien="${dossier%/}/champignon"
    if [ -L "$lien" ] && [ "$(readlink "$lien")" = "$cible" ]; then
        rm "$lien"
        echo "Commande supprimée : $lien (les parties sauvegardées sont conservées)."
    elif [ -L "$lien" ]; then
        echo "$lien pointe vers $(readlink "$lien"), pas vers $cible : je n'y touche pas." >&2
        exit 1
    else
        echo "Pas de lien à supprimer : $lien"
    fi
    exit 0
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 est introuvable : installez-le d'abord (par exemple : sudo apt install python3)." >&2
    exit 1
fi

mkdir -p -- "$dossier"
dossier=$(CDPATH= cd -- "$dossier" && pwd)   # absolu et sans « / » final, pour le PATH
lien="$dossier/champignon"
if [ -e "$lien" ] && [ ! -L "$lien" ]; then
    echo "$lien existe déjà et n'est pas un lien : je n'y touche pas." >&2
    exit 1
fi

chmod +x "$cible"
ln -sfn "$cible" "$lien"
echo "Installé : $lien -> $cible"

case ":$PATH:" in
    *":$dossier:"*) echo "Lancez le jeu avec : champignon" ;;
    *) echo "$dossier n'est pas dans le PATH : ajoutez la ligne suivante à ~/.bashrc puis rouvrez le terminal"
       echo "  export PATH=\"$dossier:\$PATH\"" ;;
esac
