#!/usr/bin/env bash
# Tack installer: checks dependencies, builds, and installs.
#
#   ./install.sh               install to ~/.local (no root)
#   ./install.sh --system      install to /usr (asks for sudo)
#   ./install.sh --prefix DIR  install under DIR
#   ./install.sh --uninstall   remove an install (same prefix options)
#   ./install.sh --yes         don't ask before installing anything
#
# Missing packages are installed after asking: with pacman on Arch and
# CachyOS, with apt on Debian, Ubuntu, Pop!_OS, and Mint. On apt systems,
# Rust comes from rustup (apt's is often too old) and the icon font is
# downloaded from the Nerd Fonts releases. Elsewhere the script lists what
# to install and stops.

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")"

PREFIX="$HOME/.local"
SUDO=()
UNINSTALL=0
ASSUME_YES=0

RUST_MIN=1.85
GTK_MIN=4.14
FONT_URL=https://github.com/ryanoasis/nerd-fonts/releases/latest/download/NerdFontsSymbolsOnly.tar.xz
FONT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/NerdFontsSymbolsOnly"

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
die() { echo "install.sh: $*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

# ask "Question?": yes unless the answer starts with n. --yes skips it.
ask() {
  ((ASSUME_YES)) && return 0
  local answer
  read -r -p "    $1 [Y/n] " answer || answer=n
  [[ ${answer:-y} == [Yy]* ]]
}

# version_ge A B: A >= B, comparing like version numbers.
version_ge() { printf '%s\n%s\n' "$2" "$1" | sort -V -C; }

if ((UNINSTALL)); then
  say "Removing Tack from $PREFIX"
  "${SUDO[@]}" make uninstall PREFIX="$PREFIX"
  exit 0
fi

# rustup installs cargo here without always adding it to PATH.
cargo_path() {
  if [[ -x $HOME/.cargo/bin/cargo && ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
}
cargo_path

if have pacman; then
  FAMILY=arch
elif have apt-get; then
  FAMILY=apt
else
  FAMILY=other
fi

rust_ok() {
  have cargo && have rustc || return 1
  version_ge "$(rustc --version | awk '{print $2}')" "$RUST_MIN"
}
gtk_ok() { have pkg-config && pkg-config --atleast-version="$GTK_MIN" gtk4 2>/dev/null; }
font_ok() {
  have fc-list || return 1
  # Read the list first: grep -q in a pipeline would trip pipefail.
  local fonts
  fonts=$(fc-list : family)
  grep -qi "Symbols Nerd Font" <<<"$fonts"
}

# What is missing, by need. Each family maps a need to its package below.
needs=()      # required to build or run
wants=()      # nice to have
say "Checking dependencies"
rust_ok || needs+=(rust)
have cc || needs+=(cc)
have make || needs+=(make)
have pkg-config || needs+=(pkgconf)
gtk_ok || needs+=(gtk4)
font_ok || needs+=(font)
have wl-copy || wants+=(wl-clipboard)
have notify-send || wants+=(notify)

# The package for a need, or nothing when it doesn't come from the
# package manager on this system.
package() {
  case $FAMILY:$1 in
    arch:rust) have rustup || echo rust ;;   # rustup and rust conflict
    arch:cc) echo gcc ;;
    arch:pkgconf) echo pkgconf ;;
    arch:gtk4) echo gtk4 ;;
    arch:font) echo ttf-nerd-fonts-symbols ;;
    arch:notify) echo libnotify ;;
    apt:rust) have curl || echo curl ;;      # for rustup
    apt:cc) echo build-essential ;;
    apt:pkgconf) echo pkg-config ;;
    apt:gtk4) echo libgtk-4-dev ;;
    apt:font)
      have curl || echo curl
      have xz || echo xz-utils
      have fc-list || echo fontconfig ;;
    apt:notify) echo libnotify-bin ;;
    *:make | *:wl-clipboard) echo "$1" ;;
  esac
}

if ((${#wants[@]})); then
  warn "Optional, not installed: ${wants[*]} (copy that outlives the app, notifications)"
fi

if ((${#needs[@]} == 0)); then
  :
elif [[ $FAMILY == other ]]; then
  warn "Missing: ${needs[*]}"
  cat >&2 <<EOF
install.sh: install these, then run this again:
  Rust $RUST_MIN+ and Cargo   https://rustup.rs
  GTK $GTK_MIN+ with headers  gtk4-devel (Fedora, openSUSE)
  C compiler, make, pkg-config
  Symbols Nerd Font        https://www.nerdfonts.com/font-downloads
EOF
  exit 1
else
  warn "Missing: ${needs[*]}"

  # An old release can't get a new enough GTK from its own archive. Say
  # so now, before installing anything.
  if [[ $FAMILY == apt && " ${needs[*]} " == *" gtk4 "* ]]; then
    candidate=$(apt-cache policy libgtk-4-dev 2>/dev/null | awk '/Candidate:/ {print $2}')
    candidate=${candidate#*:}
    if [[ -z $candidate || $candidate == "(none)" ]] || ! version_ge "$candidate" "$GTK_MIN"; then
      die "this release offers GTK ${candidate:-none}; Tack needs GTK $GTK_MIN or newer
  (Ubuntu 24.04, Pop!_OS 24.04, Linux Mint 22, Debian 13, or newer)."
    fi
  fi

  packages=()
  for need in "${needs[@]}" "${wants[@]}"; do
    while read -r p; do
      [[ -n $p && " ${packages[*]} " != *" $p "* ]] && packages+=("$p")
    done < <(package "$need")
  done

  if ((${#packages[@]})); then
    if [[ $FAMILY == arch ]]; then
      ask "Install with pacman: ${packages[*]}?" ||
        die "install the packages above, then run this again."
      sudo pacman -S --needed "${packages[@]}"
    else
      ask "Install with apt: ${packages[*]}?" ||
        die "install the packages above, then run this again."
      sudo apt-get update
      sudo apt-get install -y "${packages[@]}"
    fi
  fi

  # Rust: pacman's is current; apt's is often too old, so use rustup.
  if ! rust_ok; then
    if have rustup; then
      ask "Update Rust with rustup (needs $RUST_MIN or newer)?" ||
        die "install Rust $RUST_MIN or newer, then run this again."
      rustup update stable
    elif [[ $FAMILY == apt ]]; then
      ask "Install Rust with rustup (https://rustup.rs) for your user?" ||
        die "install Rust $RUST_MIN or newer (https://rustup.rs), then run this again."
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
        sh -s -- -y --profile minimal
    fi
    cargo_path
    rust_ok || die "Rust $RUST_MIN or newer is still missing (found: $(rustc --version 2>/dev/null || echo none))."
  fi

  # The icon font isn't packaged on Debian or Ubuntu: fetch it for this user.
  if ! font_ok && [[ $FAMILY == apt ]]; then
    ask "Download the Symbols Nerd Font (toolbar icons) to $FONT_DIR?" ||
      die "install the Symbols Nerd Font, then run this again."
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    curl -fsSL -o "$tmp/font.tar.xz" "$FONT_URL"
    mkdir -p "$FONT_DIR"
    tar -xJf "$tmp/font.tar.xz" -C "$FONT_DIR"
    fc-cache -f "$FONT_DIR" >/dev/null
  fi

  gtk_ok || die "GTK $GTK_MIN or newer (with headers) is still missing."
  font_ok || die "the Symbols Nerd Font is still missing."
  have cc || die "a C compiler (cc) is still missing."
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

On Hyprland, add the window rules from docs/configuration.md so the editor
and pins float. On GNOME or COSMIC, see docs/install.md for capturing.
EOF
