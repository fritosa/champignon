#!/bin/sh
# Installe la commande « champignon » : un lien vers main.py dans ~/.local/bin (ou le dossier donné).
# Usage : ./install.sh [dossier]   |   ./install.sh --desinstaller [dossier]
set -e

if [ "$1" = "--desinstaller" ]; then
    shift
    lien="${1:-$HOME/.local/bin}/champignon"
    if [ -L "$lien" ]; then
        rm "$lien"
        echo "Commande supprimée : $lien (les parties sauvegardées sont conservées)."
    else
        echo "Pas de lien à supprimer : $lien"
    fi
    exit 0
fi

projet=$(cd "$(dirname "$0")" && pwd)
dossier="${1:-$HOME/.local/bin}"
lien="$dossier/champignon"

if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 est introuvable : installez-le d'abord (par exemple : sudo apt install python3)." >&2
    exit 1
fi
if [ -e "$lien" ] && [ ! -L "$lien" ]; then
    echo "$lien existe déjà et n'est pas un lien : je n'y touche pas." >&2
    exit 1
fi

chmod +x "$projet/main.py"
mkdir -p "$dossier"
ln -sfn "$projet/main.py" "$lien"
echo "Installé : $lien -> $projet/main.py"

case ":$PATH:" in
    *":$dossier:"*) echo "Lancez le jeu avec : champignon" ;;
    *) echo "$dossier n'est pas dans le PATH : ajoutez la ligne suivante à ~/.bashrc puis rouvrez le terminal"
       echo "  export PATH=\"$dossier:\$PATH\"" ;;
esac
