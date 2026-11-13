# Tack — build and install.
#
#   make                         build the release binary
#   make install                 install to ~/.local (no root needed)
#   sudo make install PREFIX=/usr
#   make uninstall               remove what install put in place
#   make check                   format, lint, and test

APP_ID  := io.github.crome1394.Tack
PREFIX  ?= $(HOME)/.local
BINDIR  ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
DESTDIR ?=
CARGO   ?= cargo

BIN     := target/release/tack
SOURCES := Cargo.toml Cargo.lock $(shell find src -name '*.rs')
APPS    := $(DESTDIR)$(DATADIR)/applications
ICONS   := $(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps

.PHONY: all build install uninstall check clean

all: build

build:
	$(CARGO) build --release --locked

# Rebuilt when any source is newer, so `make install` never installs a
# stale binary. Under sudo it refuses to build: cargo as root would leave
# root-owned files in target/. (DESTDIR installs, as in the PKGBUILD, run
# under fakeroot after their own build.)
$(BIN): $(SOURCES)
	@if [ "$$(id -u)" = 0 ] && [ -z "$(DESTDIR)" ]; then \
		echo "tack: the build is out of date. Run 'make' as yourself first." >&2; \
		exit 1; \
	fi
	$(CARGO) build --release --locked

install: $(BIN)
	install -Dm755 $(BIN) "$(DESTDIR)$(BINDIR)/tack"
	install -d "$(APPS)" "$(ICONS)"
	sed 's|@BINDIR@|$(BINDIR)|' data/$(APP_ID).desktop.in > "$(APPS)/$(APP_ID).desktop"
	chmod 644 "$(APPS)/$(APP_ID).desktop"
	install -Dm644 data/$(APP_ID).svg "$(ICONS)/$(APP_ID).svg"
	@if [ -z "$(DESTDIR)" ]; then \
		update-desktop-database "$(APPS)" 2>/dev/null || true; \
		gtk-update-icon-cache -qtf "$(DESTDIR)$(DATADIR)/icons/hicolor" 2>/dev/null || true; \
	fi
	@echo "Installed tack to $(BINDIR). Right-click an image > Open With > Tack."

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/tack" "$(APPS)/$(APP_ID).desktop" "$(ICONS)/$(APP_ID).svg"
	@if [ -z "$(DESTDIR)" ]; then update-desktop-database "$(APPS)" 2>/dev/null || true; fi

check:
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets --locked -- -D warnings
	$(CARGO) test --locked

clean:
	$(CARGO) clean
