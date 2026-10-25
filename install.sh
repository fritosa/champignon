#!/bin/sh
# Builds the game (cargo build --release) and installs the "champignon" command: a link to the binary in
# ~/.local/bin (or the given directory).
# Usage: ./install.sh [dir]   |   ./install.sh --desinstaller [dir]
set -e

usage() {
    echo "Usage : $0 [dossier]   |   $0 --desinstaller [dossier]" >&2
    exit 2
}

project=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
target="$project/target/release/champignon"

uninstall=
if [ "$1" = "--desinstaller" ]; then
    uninstall=1
    shift
fi
case "$1" in -*) usage ;; esac
[ $# -le 1 ] || usage
dir="${1:-$HOME/.local/bin}"

if [ -n "$uninstall" ]; then
    link="${dir%/}/champignon"
    # main.py: link left by an install of the former Python version.
    if [ -L "$link" ] && { [ "$(readlink "$link")" = "$target" ] || [ "$(readlink "$link")" = "$project/main.py" ]; }; then
        rm "$link"
        echo "Commande supprimée : $link (les parties sauvegardées sont conservées)."
    elif [ -L "$link" ]; then
        echo "$link pointe vers $(readlink "$link"), pas vers $target : je n'y touche pas." >&2
        exit 1
    else
        echo "Pas de lien à supprimer : $link"
    fi
    exit 0
fi

cargo=$(command -v cargo || echo "$HOME/.cargo/bin/cargo")
if [ ! -x "$cargo" ]; then
    echo "cargo est introuvable : installez Rust d'abord (https://rustup.rs)." >&2
    exit 1
fi

mkdir -p -- "$dir"
dir=$(CDPATH= cd -- "$dir" && pwd)   # absolute, no trailing "/", for PATH
link="$dir/champignon"
if [ -e "$link" ] && [ ! -L "$link" ]; then
    echo "$link existe déjà et n'est pas un lien : je n'y touche pas." >&2
    exit 1
fi

# --target-dir: the link points to $target even if CARGO_TARGET_DIR or build.target-dir says otherwise.
"$cargo" build --release --quiet --manifest-path "$project/Cargo.toml" --target-dir "$project/target"
ln -sfn "$target" "$link"
echo "Installé : $link -> $target"

case ":$PATH:" in
    *":$dir:"*) echo "Lancez le jeu avec : champignon" ;;
    *) echo "$dir n'est pas dans le PATH : ajoutez la ligne suivante à ~/.bashrc puis rouvrez le terminal"
       echo "  export PATH=\"$dir:\$PATH\"" ;;
esac
