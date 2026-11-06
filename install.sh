#!/usr/bin/env bash
# Tack installer: checks dependencies, builds, and installs.
#
#   ./install.sh               install to ~/.local (no root)
#   ./install.sh --system      install to /usr (asks for sudo)
#   ./install.sh --prefix DIR  install under DIR
#   ./install.sh --uninstall   remove an install (same prefix options)
#   ./install.sh --yes         don't ask before installing packages
#
# On Arch and CachyOS, missing packages can be installed with pacman after
# asking. Elsewhere the script lists what to install and stops.

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")"

PREFIX="$HOME/.local"
SUDO=()
UNINSTALL=0
ASSUME_YES=0

usage() {
  # The comment block at the top of this file.
  sed -n '2,/^$/p' "$0" | sed -e 's/^# \{0,1\}//' -e '/^$/d'
}

while (($#)); do
  case $1 in
    --system) PREFIX=/usr; SUDO=(sudo) ;;
    --prefix)
      [[ $# -ge 2 && -n $2 ]] || { echo "install.sh: --prefix needs a folder" >&2; exit 2; }
      PREFIX=$2; shift ;;
    --uninstall) UNINSTALL=1 ;;
    -y | --yes) ASSUME_YES=1 ;;
    -h | --help) usage; exit 0 ;;
    *) echo "install.sh: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[33m  !\033[0m %s\n' "$*"; }

if ((UNINSTALL)); then
  say "Removing Tack from $PREFIX"
  "${SUDO[@]}" make uninstall PREFIX="$PREFIX"
  exit 0
fi

# rustup installs cargo here without always adding it to PATH.
if ! command -v cargo >/dev/null 2>&1 && [[ -x $HOME/.cargo/bin/cargo ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

missing=()     # package names to install
optional=()    # nice-to-have packages that are missing

have() { command -v "$1" >/dev/null 2>&1; }

say "Checking dependencies"
have cargo || missing+=(rust)
have make || missing+=(make)
have pkg-config || missing+=(pkgconf)
if have pkg-config && ! pkg-config --atleast-version=4.14 gtk4 2>/dev/null; then
  missing+=(gtk4)
fi
# Read the font list first: grep -q in a pipeline would trip pipefail.
if have fc-list; then
  fonts=$(fc-list : family)
  grep -qi "Symbols Nerd Font" <<<"$fonts" || missing+=(ttf-nerd-fonts-symbols)
fi
have wl-copy || optional+=(wl-clipboard)
have notify-send || optional+=(libnotify)

if ((${#optional[@]})); then
  warn "Optional, not installed: ${optional[*]} (copy that outlives the app, notifications)"
fi

if ((${#missing[@]})); then
  warn "Missing: ${missing[*]}"
  if have pacman; then
    packages=("${missing[@]}" "${optional[@]}")
    answer=y
    if ((!ASSUME_YES)); then
      read -r -p "    Install with pacman: ${packages[*]}? [Y/n] " answer || answer=n
    fi
    case ${answer:-y} in
      [Yy]*) sudo pacman -S --needed "${packages[@]}" ;;
      *) echo "install.sh: install the packages above, then run this again." >&2; exit 1 ;;
    esac
    have cargo || [[ ! -x $HOME/.cargo/bin/cargo ]] || export PATH="$HOME/.cargo/bin:$PATH"
  else
    cat >&2 <<'EOF'
install.sh: install these, then run this again:
  Rust and Cargo     https://rustup.rs
  GTK 4.14+          libgtk-4-dev (Debian/Ubuntu), gtk4-devel (Fedora)
  pkg-config, make
  Symbols Nerd Font  https://www.nerdfonts.com/font-downloads
EOF
    exit 1
  fi
fi

say "Building (release)"
make build

say "Installing to $PREFIX"
"${SUDO[@]}" make install PREFIX="$PREFIX"

case ":$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *) warn "$PREFIX/bin is not on your PATH; add it to use 'tack' from a terminal." ;;
esac

cat <<EOF

Done. Try it:
  tack some-screenshot.png
  Right-click an image in your file manager > Open With > Tack

On Hyprland, add the window rules from the README so the editor and pins float.
EOF
