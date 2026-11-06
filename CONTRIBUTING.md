# Contributing

Thanks for helping with Tack.

1. Build with `make`, run with `cargo run -- shot.png`.
2. Before sending a change, run `make check` (rustfmt, clippy with warnings
   as errors, and the tests).
3. Keep modules small and focused: see the layout in [AGENTS.md](AGENTS.md).
4. Run external programs with an argument list, never through a shell.
5. Add a line under a new `Unreleased` heading in [CHANGELOG.md](CHANGELOG.md).

Bugs and ideas are welcome as issues. Please include your compositor, GTK
version, and the command you ran.
